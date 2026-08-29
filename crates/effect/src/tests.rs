use super::*;
use ora_domain::{Namespace, WorkspaceId};
use pretty_assertions::{assert_eq, assert_ne};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Builds one Workspace scope used by pure domain fixtures.
fn scope() -> EffectScopeId {
    EffectScopeId::Workspace(WorkspaceId::new("workspace-1"))
}

/// Builds the exact capabilities of the first Agent plugin Consumer Revision.
fn capabilities() -> CapabilitySet {
    CapabilitySet {
        effect_protocols: BTreeMap::from([(EffectKind::skill(), 1)]),
        materialization_contracts: BTreeSet::from([
            MaterializationContract::skill_directory_v1().capability_key()
        ]),
        coordination_contracts: BTreeSet::new(),
        readiness_contracts: BTreeSet::new(),
    }
}

/// Builds one stable Agent Consumer identity without exposing connection identity.
fn consumer() -> ConsumerIdentity {
    ConsumerIdentity::new(ConsumerKind::agent_plugin(), "official/codex")
        .unwrap_or_else(|error| panic!("consumer identity: {error}"))
}

/// Builds one filesystem Resource and its normalized physical key.
fn resource() -> EffectResource {
    EffectResource {
        identity: EffectResourceId::new("resource-1"),
        scope: scope(),
        resource_key: ResourceKey::parse("filesystem-directory:.agents/skills")
            .unwrap_or_else(|error| panic!("resource key: {error}")),
        adapter: ResourceAdapterIdentity::parse("ora/filesystem-directory")
            .unwrap_or_else(|error| panic!("adapter identity: {error}")),
        descriptor: VersionedResourceDescriptor::FilesystemDirectoryV1(
            FilesystemDirectoryDescriptor {
                workspace_root: PathBuf::from("/workspace"),
                relative_path: ResourcePath::parse(".agents/skills")
                    .unwrap_or_else(|error| panic!("resource path: {error}")),
            },
        ),
        format: MaterializationFormat::skill_directory_v1(),
        lifecycle: ResourceLifecycle::Active,
    }
}

/// Builds one immutable Skill revision and its selected Desired Effect.
fn desired_skill() -> (DesiredEffect, EffectRevision) {
    let source = SkillSourceKey {
        source_kind: SkillSourceKind::Local,
        namespace: Namespace::local(),
        name: SkillName::parse("grilling").unwrap_or_else(|error| panic!("skill name: {error}")),
    };
    let revision_id = EffectRevisionId::new("revision-1");
    let desired = DesiredEffect {
        identity: DesiredEffectIdentity::new("desired-1"),
        revision: revision_id.clone(),
        parameters: ValidatedEffectParameters::Skill(SkillParameters {}),
        audience: TargetSelector::default(),
    };
    let revision = EffectRevision {
        identity: revision_id,
        source: EffectSourceIdentity::new("source-1"),
        revision_key: SourceRevisionKey::parse("1")
            .unwrap_or_else(|error| panic!("revision key: {error}")),
        definition: ValidatedEffectDefinition::Skill(SkillDefinition {
            source,
            skill_md_digest: Digest::sha256(b"manifest"),
            package_fingerprint: Fingerprint::sha256(b"package"),
            package_root: PathBuf::from("/catalog/grilling"),
        }),
        digest: Digest::sha256(b"revision"),
        availability: RevisionAvailability::Available,
    };
    (desired, revision)
}

/// Builds one Target and its immutable capability/declaration snapshots.
fn target_facts(
    lifecycle: TargetLifecycle,
) -> (
    EffectTarget,
    ConsumerRevision,
    TargetDeclaration,
    EffectResource,
) {
    let resource = resource();
    let consumer = consumer();
    let revision_id = ConsumerRevisionId::new("consumer-revision-1");
    let target = EffectTarget {
        identity: EffectTargetId::new("target-1"),
        scope: scope(),
        consumer: consumer.clone(),
        consumer_revision: revision_id.clone(),
        lifecycle,
    };
    let consumer_revision = ConsumerRevision {
        identity: revision_id.clone(),
        consumer,
        capabilities: capabilities(),
        declaration_digest: Digest::sha256(b"declaration"),
    };
    let binding = TargetResourceBinding {
        target: target.identity.clone(),
        resource: resource.identity.clone(),
        accepts: CapabilityRequirement::default(),
        coordination: CoordinationRequirement::Uninterrupted,
    };
    let declaration = TargetDeclaration {
        target: target.identity.clone(),
        consumer_revision: revision_id,
        bindings: BTreeMap::from([(resource.identity.clone(), binding)]),
        digest: Digest::sha256(b"target-declaration"),
    };
    (target, consumer_revision, declaration, resource)
}

#[test]
fn target_watermarks_reject_unproven_readiness() {
    assert_eq!(
        TargetProgress::restore(
            Generation::new(3),
            Generation::new(2),
            Generation::new(1),
            Generation::new(2),
        ),
        Err(StatusTransitionError::InvalidTargetWatermarks)
    );
}

#[test]
fn operation_intent_rejects_a_mutation_label_that_disagrees_with_exact_states() {
    let native_identity = NativeResourceIdentity::parse("skill")
        .unwrap_or_else(|error| panic!("native identity: {error}"));
    let managed_identity = ManagedIdentity::new("managed-1");
    let result = EffectOperation::prepare(
        EffectOperationId::new("operation-1"),
        EffectOperationIntent {
            attempt: ReconcileAttemptId::new("attempt-1"),
            resource: EffectResourceId::new("resource-1"),
            generation: Generation::new(1),
            sequence: 0,
            mutation: EffectMutation::Delete,
            expected: ExactPreviousState::Present {
                native_identity: native_identity.clone(),
                fingerprint: Fingerprint::sha256(b"previous"),
                managed_identity: managed_identity.clone(),
            },
            planned: ExactPlannedState::Present {
                native_identity,
                fingerprint: Fingerprint::sha256(b"planned"),
                managed_identity,
            },
            payload: VersionedAdapterPlan::FilesystemDirectoryV1(FilesystemOperationPlan {
                workspace_root: PathBuf::from("/workspace"),
                resource_relative_path: ResourcePath::parse(".agents/skills")
                    .unwrap_or_else(|error| panic!("resource path: {error}")),
                resource_root: PathBuf::from("/workspace/.agents/skills"),
                source_root: None,
                staging_path: PathBuf::from("/workspace/.agents/staging"),
                backup_path: PathBuf::from("/workspace/.agents/backup"),
            }),
        },
        LocalTimestamp::from_millis(1),
    );

    assert_eq!(result, Err(OperationTransitionError::InvalidMutationStates));
}

#[test]
fn retiring_target_projects_no_desired_contribution_but_keeps_cleanup_binding() {
    let (target, consumer_revision, declaration, resource) =
        target_facts(TargetLifecycle::Retiring);
    let (desired, revision) = desired_skill();
    let desired_state = DesiredState::normalized(scope(), Generation::new(4), [desired])
        .unwrap_or_else(|error| panic!("desired state: {error}"));
    let result = SkillPlanner
        .project(TargetPlanningInput {
            desired: &desired_state,
            target: &target,
            consumer_revision: &consumer_revision,
            declaration: &declaration,
            resources: &BTreeMap::from([(resource.identity.clone(), resource.clone())]),
            revisions: &BTreeMap::from([(revision.identity.clone(), revision)]),
        })
        .unwrap_or_else(|error| panic!("target projection: {error}"));
    let PlanningResult::Projected(projection) = result else {
        panic!("retiring Target should produce a cleanup projection");
    };

    assert_eq!(projection.desired_effects, BTreeSet::new());
    assert_eq!(
        projection
            .resource_requirements
            .get(&resource.identity)
            .map(|requirement| requirement.desired_effects.clone()),
        Some(BTreeSet::new())
    );
}

#[test]
fn unowned_observed_item_is_preserved_and_never_becomes_a_mutation() {
    let resource = resource();
    let native_identity = NativeResourceIdentity::parse("foreign")
        .unwrap_or_else(|error| panic!("native identity: {error}"));
    let fingerprint = Fingerprint::sha256(b"foreign bytes");
    let observed = ResourceObservation {
        resource: resource.identity.clone(),
        items: BTreeMap::from([(
            native_identity.clone(),
            ObservedItem {
                native_identity: native_identity.clone(),
                fingerprint: fingerprint.clone(),
                ownership_evidence: OwnershipEvidence::NoOwnershipEvidence,
            },
        )]),
        fingerprint: fingerprint.clone(),
    };
    let result = SkillPlanner
        .merge(ResourcePlanningInput {
            resource: &resource,
            generation: Generation::new(2),
            requirements: &[],
            desired_effects: &BTreeMap::new(),
            revisions: &BTreeMap::new(),
            managed: &[],
            observed: &observed,
        })
        .unwrap_or_else(|error| panic!("resource projection: {error}"));
    let PlanningResult::Projected(plan) = result else {
        panic!("unrelated external state should not block an empty projection");
    };

    assert_eq!(
        plan.preserved,
        vec![PreservedItem {
            resource: resource.identity,
            native_identity,
            fingerprint,
        }]
    );
    assert_eq!(plan.changes, Vec::new());
}

#[test]
fn shared_resource_merges_target_contributors_before_planning_one_create() {
    let resource = resource();
    let (desired, revision) = desired_skill();
    let desired_ids = BTreeSet::from([desired.identity.clone()]);
    let requirements = vec![
        ResourceRequirement {
            target: EffectTargetId::new("target-a"),
            resource: resource.identity.clone(),
            desired_effects: desired_ids.clone(),
            materialization_contract: MaterializationContract::skill_directory_v1(),
            digest: Digest::sha256(b"requirement-a"),
        },
        ResourceRequirement {
            target: EffectTargetId::new("target-b"),
            resource: resource.identity.clone(),
            desired_effects: desired_ids,
            materialization_contract: MaterializationContract::skill_directory_v1(),
            digest: Digest::sha256(b"requirement-b"),
        },
    ];
    let observed = ResourceObservation {
        resource: resource.identity.clone(),
        items: BTreeMap::new(),
        fingerprint: Fingerprint::sha256(&[]),
    };
    let result = SkillPlanner
        .merge(ResourcePlanningInput {
            resource: &resource,
            generation: Generation::new(1),
            requirements: &requirements,
            desired_effects: &BTreeMap::from([(desired.identity.clone(), desired)]),
            revisions: &BTreeMap::from([(revision.identity.clone(), revision)]),
            managed: &[],
            observed: &observed,
        })
        .unwrap_or_else(|error| panic!("resource projection: {error}"));
    let PlanningResult::Projected(plan) = result else {
        panic!("compatible Target requirements should merge");
    };

    assert_eq!(
        plan.projection.contributors,
        BTreeSet::from([
            EffectTargetId::new("target-a"),
            EffectTargetId::new("target-b"),
        ])
    );
    assert_eq!(plan.projection.items.len(), 1);
    assert_eq!(plan.changes.len(), 1);
}

#[test]
fn identical_physical_declarations_share_a_resource_key_without_sharing_targets() {
    let template = FilesystemResourceTemplate {
        relative_path: ResourcePath::parse(".agents/skills")
            .unwrap_or_else(|error| panic!("resource path: {error}")),
        materialization_format: MaterializationFormat::skill_directory_v1(),
        accepts: CapabilityRequirement::default(),
        coordination: CoordinationRequirement::Uninterrupted,
    };

    assert_eq!(template.resource_key(), template.resource_key());
    assert_ne!(
        EffectTargetId::new("target-a"),
        EffectTargetId::new("target-b")
    );
}
