use crate::{
    DatabaseBootstrapper, DatabaseLocation, RepositoryPool, SourceMutationOutcome,
    SourcePublication, SqliteEffectRepository, TimestampSource, default_migration_catalog,
};
use ora_domain::{Namespace, WorkspaceId};
use ora_effect::{
    ConsumerCoordination, ConsumerId, DesiredSkillState, Digest, EffectRepository,
    FilesystemSkillSurface, Generation, MaterializationFormat, ReplaceEffectOutcome, SkillName,
    SkillSelectionKey, SkillSource, SkillState, SourceKind, SourceVersion, SurfaceDescriptorSet,
    SurfaceLifecycle, SurfacePath, WorkspaceEffectSpec,
};
use ora_logging::with_trace_logging;
use pretty_assertions::assert_eq;
use rusqlite::params;
use std::collections::BTreeMap;
use tempfile::TempDir;

#[derive(Clone, Copy, Debug)]
struct FixedTimestamp;

impl TimestampSource for FixedTimestamp {
    fn current_timestamp_millis(&self) -> i64 {
        1
    }
}

/// Creates a file-backed pool with one valid Workspace foreign-key target.
fn fixture() -> (TempDir, RepositoryPool, WorkspaceId) {
    let directory = TempDir::new().unwrap_or_else(|error| panic!("create database dir: {error}"));
    let location = DatabaseLocation::path(directory.path().join("ora.sqlite"));
    let pool = with_trace_logging(|| {
        DatabaseBootstrapper::new(FixedTimestamp)
            .bootstrap_repository_pool(
                &location,
                &default_migration_catalog()
                    .unwrap_or_else(|error| panic!("build catalog: {error}")),
            )
            .unwrap_or_else(|error| panic!("bootstrap pool: {error}"))
    });
    let workspace_id = WorkspaceId::new("workspace-1");
    pool.with_connection(|connection| {
        connection.execute(
            "INSERT INTO projects (
                 id, name, repository_kind, created_at, updated_at, is_deleted
             ) VALUES ('project-1', 'Demo', 'git', 1, 1, 0)",
            [],
        )?;
        connection.execute(
            "INSERT INTO workspace_locations (
                 id, location_kind, locator_version, locator_json, created_at, updated_at
             ) VALUES ('location-1', 'local_filesystem', 1, '{}', 1, 1)",
            [],
        )?;
        connection.execute(
            "INSERT INTO workspaces (
                 id, project_id, workspace_kind, location_id, lifecycle,
                 created_at, updated_at, is_deleted
             ) VALUES (?1, 'project-1', 'main', 'location-1', 'active', 1, 1, 0)",
            params![workspace_id.as_ref()],
        )?;
        Ok(())
    })
    .unwrap_or_else(|error| panic!("insert Workspace fixture: {error}"));
    (directory, pool, workspace_id)
}

/// Builds one active Local source revision.
fn local_source(version: &str, manifest: &[u8]) -> (SkillSelectionKey, DesiredSkillState) {
    let name =
        SkillName::parse("review").unwrap_or_else(|error| panic!("parse Skill name: {error}"));
    let key = SkillSelectionKey::new(SourceKind::Local, Namespace::local(), name.clone());
    let state = DesiredSkillState::try_new(SkillState {
        name,
        skill_md_digest: Digest::sha256(manifest),
        source: SkillSource::Local {
            namespace: Namespace::local(),
            version: SourceVersion::parse(version)
                .unwrap_or_else(|error| panic!("parse source version: {error}")),
        },
    })
    .unwrap_or_else(|error| panic!("build source: {error}"));
    (key, state)
}

/// Registers one consumer-declared physical surface for request-upsert assertions.
fn register_surface(repository: &SqliteEffectRepository, workspace_id: &WorkspaceId, now: i64) {
    let descriptors = SurfaceDescriptorSet::merge(
        workspace_id,
        [FilesystemSkillSurface {
            workspace_relative_path: SurfacePath::parse(".agents/skills")
                .unwrap_or_else(|error| panic!("parse surface path: {error}")),
            materialization_format: MaterializationFormat::skill_directory_v1(),
            consumer: ConsumerId::new("codex"),
            coordination: ConsumerCoordination::WaitForIdleAndRestart,
        }],
    )
    .unwrap_or_else(|error| panic!("merge surface: {error}"));
    repository
        .replace_surfaces(
            workspace_id,
            std::path::Path::new("/workspace"),
            &descriptors,
            now,
        )
        .unwrap_or_else(|error| panic!("register surface: {error}"));
}

#[test]
fn desired_replace_uses_cas_and_normalized_no_op_semantics() {
    let (_directory, pool, workspace_id) = fixture();
    let repository = SqliteEffectRepository::new(pool.clone());
    let source = local_source("1", b"manifest-v1");
    repository
        .publish_source(
            &source.1,
            std::path::Path::new("/catalog/review"),
            SourcePublication::Create,
            10,
        )
        .unwrap_or_else(|error| panic!("publish source: {error}"));
    register_surface(&repository, &workspace_id, 10);
    let spec = WorkspaceEffectSpec {
        skills: BTreeMap::from([(source.0, source.1)]),
    };

    assert_eq!(
        repository
            .replace_workspace_effect(&workspace_id, Generation::new(1), spec, 30)
            .unwrap_or_else(|error| panic!("replace no-op: {error}")),
        ReplaceEffectOutcome::Unchanged(
            repository
                .load_workspace_effect(&workspace_id)
                .unwrap_or_else(|error| panic!("load effect: {error}"))
        )
    );
    assert_eq!(
        repository
            .replace_workspace_effect(
                &workspace_id,
                Generation::default(),
                WorkspaceEffectSpec::default(),
                40,
            )
            .unwrap_or_else(|error| panic!("replace conflict: {error}")),
        ReplaceEffectOutcome::Conflict {
            expected_generation: Generation::default(),
            current_generation: Generation::new(1),
        }
    );

    let request_generation = pool
        .with_connection(|connection| {
            connection
                .query_row(
                    "SELECT requested_generation FROM effect_reconcile_requests
                     WHERE surface_id IN (
                         SELECT id FROM effect_surfaces WHERE workspace_id = ?1
                     )",
                    params![workspace_id.as_ref()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(Into::into)
        })
        .unwrap_or_else(|error| panic!("load request: {error}"));
    assert_eq!(request_generation, 1);
}

#[test]
fn source_updates_coalesce_and_delete_uninstalls_from_every_workspace() {
    let (_directory, pool, workspace_id) = fixture();
    let repository = SqliteEffectRepository::new(pool);
    let version_one = local_source("1", b"manifest-v1");
    repository
        .publish_source(
            &version_one.1,
            std::path::Path::new("/catalog/review"),
            SourcePublication::Create,
            10,
        )
        .unwrap_or_else(|error| panic!("publish v1: {error}"));
    let version_two = local_source("2", b"manifest-v2");
    let version_three = local_source("3", b"manifest-v3");
    repository
        .publish_source(
            &version_two.1,
            std::path::Path::new("/catalog/review"),
            SourcePublication::Update,
            30,
        )
        .unwrap_or_else(|error| panic!("publish v2: {error}"));
    repository
        .publish_source(
            &version_three.1,
            std::path::Path::new("/catalog/review"),
            SourcePublication::Update,
            40,
        )
        .unwrap_or_else(|error| panic!("publish v3: {error}"));
    assert_eq!(
        repository
            .list_propagation_requests()
            .unwrap_or_else(|error| panic!("list propagation: {error}")),
        vec![version_one.0.clone()]
    );
    assert_eq!(
        repository
            .propagate_source(&version_one.0, 50)
            .unwrap_or_else(|error| panic!("propagate latest: {error}")),
        vec![(workspace_id.clone(), Generation::new(2))]
    );
    let effect = repository
        .load_workspace_effect(&workspace_id)
        .unwrap_or_else(|error| panic!("load propagated effect: {error}"));
    assert_eq!(effect.generation, Generation::new(2));
    assert_eq!(
        effect.spec.skills[&version_one.0].state().source.version(),
        version_three.1.state().source.version()
    );
    assert!(
        repository
            .list_propagation_requests()
            .unwrap_or_else(|error| panic!("list completed propagation: {error}"))
            .is_empty()
    );
    assert_eq!(
        repository
            .delete_source(&version_one.0)
            .unwrap_or_else(|error| panic!("delete source: {error}")),
        SourceMutationOutcome::Deleted
    );
    let effect = repository
        .load_workspace_effect(&workspace_id)
        .unwrap_or_else(|error| panic!("load uninstalled effect: {error}"));
    assert_eq!(effect.generation, Generation::new(3));
    assert_eq!(effect.spec, WorkspaceEffectSpec::default());
}

#[test]
fn unavailable_source_cannot_enter_desired_state() {
    let (_directory, pool, workspace_id) = fixture();
    let repository = SqliteEffectRepository::new(pool);
    let source = local_source("1", b"manifest-v1");
    repository
        .publish_source(
            &source.1,
            std::path::Path::new("/catalog/review"),
            SourcePublication::Create,
            10,
        )
        .unwrap_or_else(|error| panic!("publish source: {error}"));
    repository
        .mark_source_unavailable(&source.0, "external drift", 20)
        .unwrap_or_else(|error| panic!("mark unavailable: {error}"));
    repository
        .replace_workspace_effect(
            &workspace_id,
            Generation::new(1),
            WorkspaceEffectSpec::default(),
            25,
        )
        .unwrap_or_else(|error| panic!("remove unavailable source: {error}"));

    assert_eq!(
        repository
            .replace_workspace_effect(
                &workspace_id,
                Generation::new(2),
                WorkspaceEffectSpec {
                    skills: BTreeMap::from([(source.0.clone(), source.1)]),
                },
                30,
            )
            .unwrap_or_else(|error| panic!("replace desired: {error}")),
        ReplaceEffectOutcome::SourceUnavailable {
            selection_key: source.0,
        }
    );
}

/// The worker reads a self-contained descriptor, so it never rebuilds one from a live declaration.
///
/// A request outlives the process that created it, and the plugin that declared the surface may be
/// gone by the time it is served, so everything the reconciler needs has to come back out of the
/// database rather than out of whatever happens to be running.
#[test]
fn due_requests_carry_the_locator_and_consumers_the_reconciler_needs() {
    let (_directory, pool, workspace_id) = fixture();
    let repository = SqliteEffectRepository::new(pool);
    register_surface(&repository, &workspace_id, 10);

    let due = repository
        .claim_due_reconcile_requests("worker-1", 10, 10_000, 8)
        .unwrap_or_else(|error| panic!("claim due requests: {error}"));

    assert_eq!(due.len(), 1);
    let entry = &due[0].due;
    assert_eq!(entry.workspace_id, workspace_id);
    assert_eq!(entry.workspace_root, std::path::Path::new("/workspace"));
    assert_eq!(entry.descriptor.path.as_str(), ".agents/skills");
    assert_eq!(
        entry.descriptor.format,
        MaterializationFormat::skill_directory_v1()
    );
    assert_eq!(entry.descriptor.lifecycle, SurfaceLifecycle::Active);
    assert_eq!(
        entry.descriptor.consumers,
        BTreeMap::from([(
            ConsumerId::new("codex"),
            ConsumerCoordination::WaitForIdleAndRestart
        )])
    );
}

/// Completing at a stale generation must not discard the wakeup a later edit already merged in.
///
/// Desired can advance while a reconcile is mid-flight. Nothing re-creates a deleted request, so
/// clearing one the reconcile never caught up with would strand that surface until the next
/// unrelated edit.
#[test]
fn completing_a_request_respects_a_generation_that_advanced_mid_reconcile() {
    let (_directory, pool, workspace_id) = fixture();
    let repository = SqliteEffectRepository::new(pool);
    register_surface(&repository, &workspace_id, 10);
    let claim = repository
        .claim_due_reconcile_requests("worker-1", 10, 10_000, 8)
        .unwrap_or_else(|error| panic!("claim due requests: {error}"))
        .remove(0)
        .claim;

    let source = local_source("1", b"manifest-v1");
    repository
        .publish_source(
            &source.1,
            std::path::Path::new("/catalog/review"),
            SourcePublication::Create,
            20,
        )
        .unwrap_or_else(|error| panic!("publish source: {error}"));

    // Publishing installed the source into every Workspace, so the request now asks for
    // generation 1 while the in-flight reconcile only ever observed the empty generation 0.
    assert!(
        !repository
            .complete_reconcile_request(&claim, Generation::default(), 30)
            .unwrap_or_else(|error| panic!("complete stale: {error}")),
    );
    // Falling back to pending rather than deleting is what keeps the newer generation scheduled.
    let reclaimed = repository
        .claim_due_reconcile_requests("worker-1", 30, 10_000, 8)
        .unwrap_or_else(|error| panic!("reclaim after stale: {error}"));
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(
        reclaimed[0].due.requested_generation,
        Generation::new(1),
        "the request must carry the generation that landed mid-reconcile",
    );

    assert!(
        repository
            .complete_reconcile_request(&reclaimed[0].claim, Generation::new(1), 40)
            .unwrap_or_else(|error| panic!("complete current: {error}")),
    );
    assert!(
        repository
            .claim_due_reconcile_requests("worker-1", 40, 10_000, 8)
            .unwrap_or_else(|error| panic!("claim after current: {error}"))
            .is_empty(),
    );
}

/// A retired surface may only be forgotten once nothing on disk is still owned through it.
#[test]
fn retired_surface_deletion_waits_for_an_empty_ownership_ledger() {
    let (_directory, pool, workspace_id) = fixture();
    let repository = SqliteEffectRepository::new(pool.clone());
    register_surface(&repository, &workspace_id, 10);
    let surface_key = repository
        .claim_due_reconcile_requests("worker-1", 10, 10_000, 8)
        .unwrap_or_else(|error| panic!("claim due requests: {error}"))
        .remove(0)
        .due
        .descriptor
        .surface_key;
    // Withdrawing every declaration retires the surface without deleting it.
    repository
        .replace_surfaces(&workspace_id, std::path::Path::new("/workspace"), &[], 20)
        .unwrap_or_else(|error| panic!("retire surface: {error}"));

    let source = local_source("1", b"manifest-v1");
    repository
        .publish_source(
            &source.1,
            std::path::Path::new("/catalog/review"),
            SourcePublication::Create,
            30,
        )
        .unwrap_or_else(|error| panic!("publish source: {error}"));
    pool.with_connection(|connection| {
        connection.execute(
            "INSERT INTO effect_managed_items (
                 id, surface_id, source_id, applied_revision_id, target_key, target_json,
                 applied_fingerprint, applied_generation, created_at, updated_at
             )
             SELECT 'managed-1', ?1, heads.source_id, heads.revision_id, 'review', '{}',
                    'sha256:0', 0, 40, 40
             FROM effect_source_heads heads",
            params![surface_key.as_str()],
        )?;
        Ok(())
    })
    .unwrap_or_else(|error| panic!("insert managed item: {error}"));

    assert!(
        !repository
            .delete_retired_surface(&surface_key)
            .unwrap_or_else(|error| panic!("delete owned surface: {error}")),
    );

    pool.with_connection(|connection| {
        connection.execute("DELETE FROM effect_managed_items", [])?;
        Ok(())
    })
    .unwrap_or_else(|error| panic!("clear ledger: {error}"));

    assert!(
        repository
            .delete_retired_surface(&surface_key)
            .unwrap_or_else(|error| panic!("delete cleaned surface: {error}")),
    );
}
