use ora_application::{
    AgentDefinitionRepository, AgentSkillDelivery, AgentSkillDeliveryProvider,
    FilesystemSkillStorage, MaterializedSkillBinding, NodeType, RepositoryError,
    SkillDiscoveryRoots, SkillMaterializationReceipt, SkillRepository, StartPrerequisitesError,
    WorkflowGraph, WorkflowRunWorkspaceInitializer, has_usable_package, skill_package_is_usable,
};
use ora_db::{
    RepositoryPool, SqliteAgentDefinitionRepository, SqliteSkillRepository, TimestampSource,
};
use ora_domain::{AgentDefinitionId, AgentRef, Namespace};
use ora_utils::path::StrictRelativePath;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Current host capability used until Agent plugins publish their own discovery roots.
///
/// Keeping the default behind [`AgentSkillDeliveryProvider`] means workflow validation and prompt
/// rendering consume the same placements that the Effect subsystem materializes.
#[derive(Clone)]
pub struct SharedAgentSkillDeliveryProvider {
    discovery_roots: SkillDiscoveryRoots,
}

impl SharedAgentSkillDeliveryProvider {
    /// Builds the current shared capability after validating its worktree-relative root.
    fn new() -> Result<Self, ora_application::AgentSkillDeliveryError> {
        let root = StrictRelativePath::parse(".agents/skills").map_err(|error| {
            ora_application::AgentSkillDeliveryError::Invalid {
                message: format!("invalid built-in shared skill root: {error:?}"),
            }
        })?;
        Ok(Self {
            discovery_roots: SkillDiscoveryRoots::new(root, Vec::new()),
        })
    }
}

impl AgentSkillDeliveryProvider for SharedAgentSkillDeliveryProvider {
    fn skill_delivery(
        &self,
        _agent_ref: &AgentRef,
    ) -> Result<AgentSkillDelivery, ora_application::AgentSkillDeliveryError> {
        Ok(AgentSkillDelivery::Filesystem {
            discovery_roots: self.discovery_roots.clone(),
        })
    }
}

/// Validates a run workspace's roles and skill bindings at deploy time.
///
/// Roles and skills are deploy hard-dependencies: every agent's role must resolve in the agents
/// catalog and every enabled skill must exist in the catalog with a loadable package — the formal
/// directory for a local skill, the immutable plugin package for a plugin-imported skill. The
/// Effect subsystem owns physical package materialization; this initializer only freezes the
/// invocation names and Effect-owned discovery paths that execution uses to build the prompt.
#[derive(Clone)]
pub struct SkillRoleWorkspaceInitializer<DeliveryProvider = SharedAgentSkillDeliveryProvider> {
    skills_root: PathBuf,
    pool: RepositoryPool,
    delivery_provider: DeliveryProvider,
}

impl SkillRoleWorkspaceInitializer<SharedAgentSkillDeliveryProvider> {
    /// Builds an initializer from the skill catalog root and the shared repository pool.
    pub fn new(
        skills_root: PathBuf,
        pool: RepositoryPool,
    ) -> Result<Self, ora_application::AgentSkillDeliveryError> {
        Ok(Self::with_delivery_provider(
            skills_root,
            pool,
            SharedAgentSkillDeliveryProvider::new()?,
        ))
    }
}

impl<DeliveryProvider> SkillRoleWorkspaceInitializer<DeliveryProvider> {
    /// Builds an initializer with an injected Agent capability provider.
    pub fn with_delivery_provider(
        skills_root: PathBuf,
        pool: RepositoryPool,
        delivery_provider: DeliveryProvider,
    ) -> Self {
        Self {
            skills_root,
            pool,
            delivery_provider,
        }
    }
}

impl<DeliveryProvider> WorkflowRunWorkspaceInitializer
    for SkillRoleWorkspaceInitializer<DeliveryProvider>
where
    DeliveryProvider: AgentSkillDeliveryProvider,
{
    fn initialize_workspace(
        &self,
        graph: &WorkflowGraph,
        _workspace_root: &Path,
    ) -> Result<SkillMaterializationReceipt, StartPrerequisitesError> {
        let roles = collect_roles(graph);

        let agent_repository = SqliteAgentDefinitionRepository::new(self.pool.clone());
        for role_id in &roles {
            if resolve_role(&agent_repository, role_id)?.is_none() {
                return Err(StartPrerequisitesError::WorkflowRoleNotFound {
                    role_id: role_id.clone(),
                });
            }
        }

        let storage = FilesystemSkillStorage::new(self.skills_root.clone());
        let skill_repository = SqliteSkillRepository::new(self.pool.clone());
        resolve_graph_skill_bindings(&storage, &skill_repository, &self.delivery_provider, graph)
    }
}

/// Resolves a role by name first, falling back to the agent definition id for graphs that stored
/// the id as `roleId` (the pre-empty-role editor did).
fn resolve_role(
    agent_repository: &SqliteAgentDefinitionRepository,
    role_id: &str,
) -> Result<Option<ora_domain::AgentDefinition>, RepositoryError> {
    let by_name = agent_repository.find_agent_definition_by_name(&Namespace::local(), role_id)?;
    if by_name.is_some() {
        return Ok(by_name);
    }
    agent_repository.find_agent_definition(&AgentDefinitionId::new(role_id))
}

/// Collects the distinct role ids declared across all agent nodes.
fn collect_roles(graph: &WorkflowGraph) -> Vec<String> {
    let mut roles = Vec::new();
    for node in graph.nodes() {
        if node.node_type != NodeType::Agent {
            continue;
        }
        let Some(config) = &node.agent_config else {
            continue;
        };
        if let Some(role_id) = &config.role_id
            && !role_id.trim().is_empty()
            && !roles.contains(role_id)
        {
            roles.push(role_id.clone());
        }
    }
    roles
}

/// Resolves every node's enabled skills to the Effect-owned paths later consumed by execution.
fn resolve_graph_skill_bindings<DeliveryProvider>(
    storage: &FilesystemSkillStorage,
    skill_repository: &SqliteSkillRepository<impl TimestampSource>,
    delivery_provider: &DeliveryProvider,
    graph: &WorkflowGraph,
) -> Result<SkillMaterializationReceipt, StartPrerequisitesError>
where
    DeliveryProvider: AgentSkillDeliveryProvider,
{
    let mut receipt = SkillMaterializationReceipt::default();
    let mut resolved_packages = HashMap::<StrictRelativePath, String>::new();
    for node in graph
        .nodes()
        .filter(|node| node.node_type == NodeType::Agent)
    {
        let Some(config) = &node.agent_config else {
            continue;
        };
        let enabled_skills = config
            .skills
            .iter()
            .filter(|skill| skill.enabled)
            .collect::<Vec<_>>();
        if enabled_skills.is_empty() {
            continue;
        }
        let agent_ref = AgentRef::parse(&config.executor.agent_cli).map_err(|error| {
            StartPrerequisitesError::AgentSkillDeliveryError {
                agent_ref: config.executor.agent_cli.clone(),
                message: error.to_string(),
            }
        })?;
        let delivery = delivery_provider
            .skill_delivery(&agent_ref)
            .map_err(|error| StartPrerequisitesError::AgentSkillDeliveryError {
                agent_ref: config.executor.agent_cli.clone(),
                message: error.to_string(),
            })?;
        let AgentSkillDelivery::Filesystem { discovery_roots } = delivery else {
            return Err(StartPrerequisitesError::AgentSkillDeliveryUnsupported {
                agent_ref: config.executor.agent_cli.clone(),
            });
        };
        let mut seen_skill_ids = HashSet::new();
        for skill in enabled_skills {
            if !seen_skill_ids.insert(skill.skill_id.clone()) {
                continue;
            }
            let catalog_name =
                resolve_skill_catalog_name(storage, skill_repository, &skill.skill_id)?;
            let invocation_name = normalize_skill_name(&catalog_name);
            let package_paths = discovery_roots
                .iter()
                .map(|root| root.append_segment(&invocation_name))
                .collect::<Vec<_>>();
            for package_path in &package_paths {
                if let Some(existing_catalog_name) = resolved_packages.get(package_path) {
                    if existing_catalog_name != &catalog_name {
                        return Err(StartPrerequisitesError::SkillMaterializationError {
                            message: format!(
                                "skills {existing_catalog_name} and {catalog_name} resolve to the same worktree path {package_path}"
                            ),
                        });
                    }
                    continue;
                }
                resolved_packages.insert(package_path.clone(), catalog_name.clone());
            }
            receipt.bindings.push(MaterializedSkillBinding {
                node_id: node.id.clone(),
                skill_id: skill.skill_id.clone(),
                invocation_name,
                package_paths,
            });
        }
    }
    Ok(receipt)
}

/// Resolves one enabled skill id to its catalog name.
///
/// Graphs store the catalog name in `skillId`, so the candidate name first resolves through the
/// local formal package directory. When that directory is absent — the shape of every
/// plugin-imported skill, whose package stays inside the plugin installation — the catalog row is
/// matched and usability follows the row's owning package: the formal directory for a Local skill
/// and the immutable plugin package for a Plugin skill, mirroring the availability that the
/// settings and editor surfaces display for the same skill. A legacy namespaced id like
/// `cdase:sfmea_review` resolves by the suffix after the colon, and a full catalog id such as
/// `plugin:<plugin_id>:<name>` claims its row before the name match.
fn resolve_skill_catalog_name(
    storage: &FilesystemSkillStorage,
    skill_repository: &SqliteSkillRepository<impl TimestampSource>,
    skill_id: &str,
) -> Result<String, StartPrerequisitesError> {
    let candidate = skill_id.rsplit(':').next().unwrap_or(skill_id);
    if skill_package_usable(storage, candidate)? {
        return Ok(candidate.to_string());
    }
    let skills = skill_repository
        .list_skills()
        .map_err(StartPrerequisitesError::Repository)?;
    // An exact catalog id match wins over name matches so a stored row id is honored even when
    // its suffix is not the catalog name; name matching is ASCII-case-insensitive like the
    // repository's COLLATE NOCASE lookups.
    let mut matched = Vec::new();
    if let Some(by_id) = skills.iter().find(|skill| skill.id.as_ref() == skill_id) {
        matched.push(by_id);
    }
    matched.extend(skills.iter().filter(|skill| {
        skill.name.eq_ignore_ascii_case(candidate) && skill.id.as_ref() != skill_id
    }));
    for skill in matched {
        let usable = skill_package_is_usable(storage, skill).map_err(|error| {
            StartPrerequisitesError::SkillMaterializationError {
                message: error.to_string(),
            }
        })?;
        if usable {
            return Ok(skill.name.clone());
        }
    }
    Err(StartPrerequisitesError::WorkflowSkillNotFound {
        skill_id: skill_id.to_string(),
    })
}

/// Returns whether the local catalog still has a formal package that parses as a skill manifest.
fn skill_package_usable(
    storage: &FilesystemSkillStorage,
    name: &str,
) -> Result<bool, StartPrerequisitesError> {
    has_usable_package(storage, name).map_err(|error| {
        StartPrerequisitesError::SkillMaterializationError {
            message: error.to_string(),
        }
    })
}

/// Resolves an enabled skill id to the executable `/name` the agent CLI uses to invoke it: the
/// normalized catalog name, matching the directory materialized by the Effect subsystem.
#[cfg(test)]
fn resolve_executable_skill_name(
    storage: &FilesystemSkillStorage,
    skill_repository: &SqliteSkillRepository<impl TimestampSource>,
    skill_id: &str,
) -> Result<String, StartPrerequisitesError> {
    Ok(normalize_skill_name(&resolve_skill_catalog_name(
        storage,
        skill_repository,
        skill_id,
    )?))
}

/// Normalizes a catalog name for an Agent discovery directory: lowercase, `_` becomes `-`.
fn normalize_skill_name(name: &str) -> String {
    name.to_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ora_db::{
        DatabaseBootstrapper, DatabaseLocation, PluginSkillProjection, default_migration_catalog,
    };
    use ora_domain::PluginId;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[derive(Clone)]
    struct FixedDeliveryProvider {
        delivery: AgentSkillDelivery,
    }

    impl AgentSkillDeliveryProvider for FixedDeliveryProvider {
        fn skill_delivery(
            &self,
            _agent_ref: &AgentRef,
        ) -> Result<AgentSkillDelivery, ora_application::AgentSkillDeliveryError> {
            Ok(self.delivery.clone())
        }
    }

    /// Opens an isolated repository pool used by capability-driven binding tests.
    fn test_pool(temp: &TempDir) -> RepositoryPool {
        DatabaseBootstrapper::new(crate::test_clock::TestClock)
            .bootstrap_repository_pool(
                &DatabaseLocation::path(&temp.path().join("ora.sqlite3")),
                &default_migration_catalog().expect("create migration catalog"),
            )
            .expect("bootstrap repository pool")
    }

    /// Projects one installed plugin skill into the catalog with its package kept outside the
    /// local skills root, mirroring how a plugin import publishes its immutable skills.
    fn install_plugin_skill(pool: &RepositoryPool, package_root: &Path, name: &str) {
        std::fs::create_dir_all(package_root).unwrap();
        std::fs::write(
            package_root.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: review\n---\n"),
        )
        .unwrap();
        SqliteSkillRepository::with_clock(pool.clone(), crate::test_clock::TestClock)
            .replace_plugin_skills(
                &PluginId::new("official", "review-pack").unwrap(),
                "1.2.3",
                &[PluginSkillProjection {
                    name: name.to_string(),
                    description: "Reviews changes".to_string(),
                    package_root: package_root.to_path_buf(),
                    skill_md_digest: ora_effect::Digest::sha256(b"manifest"),
                    package_fingerprint: ora_effect::Fingerprint::from(
                        ora_utils::directory::fingerprint_directory(package_root, &[]).unwrap(),
                    ),
                }],
                10,
            )
            .unwrap();
    }

    #[test]
    fn normalizes_skill_names_to_lowercase_dashes() {
        assert_eq!(normalize_skill_name("sfmea_review"), "sfmea-review");
        assert_eq!(normalize_skill_name("OpenSpec_Explore"), "openspec-explore");
    }

    #[test]
    fn resolves_the_executable_skill_name_from_a_namespaced_id() {
        let temp = TempDir::new().unwrap();
        let skills_root = temp.path().join("skills");
        std::fs::create_dir_all(skills_root.join("sfmea_review")).unwrap();
        std::fs::write(
            skills_root.join("sfmea_review").join("SKILL.md"),
            "---\nname: sfmea_review\ndescription: review\n---\n",
        )
        .unwrap();
        let storage = FilesystemSkillStorage::new(skills_root);
        // An empty catalog keeps this legacy-id resolution on the local package fast path.
        let catalog_temp = TempDir::new().unwrap();
        let repository = SqliteSkillRepository::with_clock(
            test_pool(&catalog_temp),
            crate::test_clock::TestClock,
        );
        assert_eq!(
            resolve_executable_skill_name(&storage, &repository, "cdase:sfmea_review").unwrap(),
            "sfmea-review"
        );
    }

    /// A plugin-imported skill resolves through its immutable plugin package — by the catalog
    /// name the editor stores in `skillId` and by its full `plugin:` catalog id — even though no
    /// formal directory exists in the local skills root.
    #[test]
    fn resolves_a_plugin_skill_through_its_plugin_package() {
        let temp = TempDir::new().unwrap();
        let skills_root = temp.path().join("skills");
        let package_root = temp.path().join("plugins/review-pack/assets/review");
        let pool = test_pool(&temp);
        install_plugin_skill(&pool, &package_root, "review");
        let storage = FilesystemSkillStorage::new(skills_root);
        let repository = SqliteSkillRepository::with_clock(pool, crate::test_clock::TestClock);

        assert_eq!(
            resolve_executable_skill_name(&storage, &repository, "review").unwrap(),
            "review"
        );
        assert_eq!(
            resolve_executable_skill_name(
                &storage,
                &repository,
                "plugin:official/review-pack:review",
            )
            .unwrap(),
            "review"
        );
    }

    /// A plugin skill whose package no longer loads must still block deployment instead of
    /// resolving, matching the unavailable state the settings surface shows for it.
    #[test]
    fn rejects_a_plugin_skill_whose_package_is_unavailable() {
        let temp = TempDir::new().unwrap();
        let package_root = temp.path().join("plugins/review-pack/assets/review");
        let pool = test_pool(&temp);
        install_plugin_skill(&pool, &package_root, "review");
        std::fs::remove_dir_all(&package_root).unwrap();
        let storage = FilesystemSkillStorage::new(temp.path().join("skills"));
        let repository = SqliteSkillRepository::with_clock(pool, crate::test_clock::TestClock);

        assert!(matches!(
            resolve_executable_skill_name(&storage, &repository, "review"),
            Err(StartPrerequisitesError::WorkflowSkillNotFound { skill_id })
                if skill_id == "review"
        ));
    }

    /// A workflow node bound to a plugin-imported skill deploys: the receipt freezes the plugin
    /// skill's invocation name and discovery paths like a local skill's.
    #[test]
    fn initialize_workspace_binds_a_plugin_skill_through_its_plugin_package() {
        let temp = TempDir::new().unwrap();
        let skills_root = temp.path().join("skills");
        let package_root = temp.path().join("plugins/review-pack/assets/review");
        let pool = test_pool(&temp);
        install_plugin_skill(&pool, &package_root, "review");
        let initializer = SkillRoleWorkspaceInitializer::new(skills_root, pool).unwrap();
        let graph = WorkflowGraph::parse(
            r#"{"nodes":[{"id":"a","data":{"kind":"agent","agentConfig":{"executor":{"agentCli":"ora-space.codex","modelId":"m"},"skills":[{"skillId":"review","enabled":true}]}}}],"edges":[]}"#,
        )
        .unwrap();
        let worktree = temp.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();

        let receipt = initializer.initialize_workspace(&graph, &worktree).unwrap();

        assert!(!worktree.join(".agents").exists());
        assert_eq!(
            receipt,
            SkillMaterializationReceipt {
                bindings: vec![MaterializedSkillBinding {
                    node_id: "a".to_string(),
                    skill_id: "review".to_string(),
                    invocation_name: "review".to_string(),
                    package_paths: vec![
                        StrictRelativePath::parse(".agents/skills/review").unwrap()
                    ],
                }],
            }
        );
    }

    #[test]
    fn initialize_workspace_records_skill_bindings_without_copying_packages() {
        let temp = TempDir::new().unwrap();
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("sfmea_review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: sfmea_review\ndescription: review\n---\n\nbody\n",
        )
        .unwrap();
        let database_path = temp.path().join("ora.sqlite3");
        let pool = DatabaseBootstrapper::new(crate::test_clock::TestClock)
            .bootstrap_repository_pool(
                &DatabaseLocation::path(&database_path),
                &default_migration_catalog().expect("create migration catalog"),
            )
            .expect("bootstrap repository pool");
        let initializer = SkillRoleWorkspaceInitializer::new(skills_root, pool).unwrap();
        let graph = WorkflowGraph::parse(
            r#"{"nodes":[{"id":"a","data":{"kind":"agent","agentConfig":{"executor":{"agentCli":"ora-space.codex","modelId":"m"},"skills":[{"skillId":"sfmea_review","enabled":true}]}}}],"edges":[]}"#,
        )
        .unwrap();
        let worktree = temp.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();

        let receipt = initializer.initialize_workspace(&graph, &worktree).unwrap();

        assert!(!worktree.join(".agents").exists());
        assert_eq!(
            receipt,
            SkillMaterializationReceipt {
                bindings: vec![MaterializedSkillBinding {
                    node_id: "a".to_string(),
                    skill_id: "sfmea_review".to_string(),
                    invocation_name: "sfmea-review".to_string(),
                    package_paths: vec![
                        StrictRelativePath::parse(".agents/skills/sfmea-review").unwrap()
                    ],
                }],
            }
        );
    }

    /// An injected Agent capability controls the persisted placement receipt without any
    /// prompt-layer directory convention or workflow-owned filesystem writes.
    #[test]
    fn injected_agent_capability_controls_placements_and_the_frozen_receipt() {
        let temp = TempDir::new().unwrap();
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: review\ndescription: review\n---\n",
        )
        .unwrap();
        let first_root = StrictRelativePath::parse(".claude/skills").unwrap();
        let second_root = StrictRelativePath::parse(".vendor/agent-skills").unwrap();
        let initializer = SkillRoleWorkspaceInitializer::with_delivery_provider(
            skills_root,
            test_pool(&temp),
            FixedDeliveryProvider {
                delivery: AgentSkillDelivery::Filesystem {
                    discovery_roots: SkillDiscoveryRoots::new(
                        first_root.clone(),
                        vec![second_root.clone()],
                    ),
                },
            },
        );
        let graph = WorkflowGraph::parse(
            r#"{"nodes":[{"id":"review-node","data":{"kind":"agent","agentConfig":{"executor":{"agentCli":"acme.agent","modelId":"m"},"skills":[{"skillId":"review","enabled":true}]}}}],"edges":[]}"#,
        )
        .unwrap();
        let worktree = temp.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();

        let receipt = initializer.initialize_workspace(&graph, &worktree).unwrap();

        let package_paths = vec![
            first_root.append_segment("review"),
            second_root.append_segment("review"),
        ];
        assert_eq!(
            receipt,
            SkillMaterializationReceipt {
                bindings: vec![MaterializedSkillBinding {
                    node_id: "review-node".to_string(),
                    skill_id: "review".to_string(),
                    invocation_name: "review".to_string(),
                    package_paths: package_paths.clone(),
                }],
            }
        );
        assert_eq!(
            package_paths
                .iter()
                .map(|path| path.to_path(&worktree).exists())
                .collect::<Vec<_>>(),
            vec![false, false]
        );
    }

    /// Enabled skills fail deployment when the selected Agent explicitly cannot consume them.
    #[test]
    fn enabled_skills_reject_an_agent_without_delivery_support() {
        let temp = TempDir::new().unwrap();
        let initializer = SkillRoleWorkspaceInitializer::with_delivery_provider(
            temp.path().join("skills"),
            test_pool(&temp),
            FixedDeliveryProvider {
                delivery: AgentSkillDelivery::Unsupported,
            },
        );
        let graph = WorkflowGraph::parse(
            r#"{"nodes":[{"id":"a","data":{"kind":"agent","agentConfig":{"executor":{"agentCli":"acme.agent","modelId":"m"},"skills":[{"skillId":"review","enabled":true}]}}}],"edges":[]}"#,
        )
        .unwrap();

        assert!(matches!(
            initializer.initialize_workspace(&graph, temp.path()),
            Err(StartPrerequisitesError::AgentSkillDeliveryUnsupported { agent_ref })
                if agent_ref == "acme.agent"
        ));
    }
}
