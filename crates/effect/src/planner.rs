use crate::{
    ConditionGeneration, ConditionImpact, ConditionOwner, ConditionProposal, ConditionRetry,
    ConditionSubject, ConsumerRevision, DesiredEffect, DesiredEffectIdentity, DesiredState, Digest,
    EffectMutation, EffectResource, EffectResourceId, EffectRevision, EffectTarget,
    ExactPlannedState, ExactPreviousState, Generation, ManagedIdentity, ManagedItem,
    MaterializationContract, NativeResourceIdentity, OwnershipEvidence, PreservedItem,
    ProjectionDigest, ResolvedMaterialization, ResourceObservation, ResourceProjection,
    ResourceRequirement, RevisionAvailability, SafeConditionDetails, SkillMaterializationInput,
    StableConditionCode, TargetDeclaration, TargetProjection, VersionedMaterializationInput,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Input snapshot for deterministic projection of one complete Target.
pub struct TargetPlanningInput<'a> {
    pub desired: &'a DesiredState,
    pub target: &'a EffectTarget,
    pub consumer_revision: &'a ConsumerRevision,
    pub declaration: &'a TargetDeclaration,
    pub resources: &'a BTreeMap<EffectResourceId, EffectResource>,
    pub revisions: &'a BTreeMap<crate::EffectRevisionId, EffectRevision>,
}

/// Input snapshot for merging every active Target contribution to one Resource.
pub struct ResourcePlanningInput<'a> {
    pub resource: &'a EffectResource,
    pub generation: Generation,
    pub requirements: &'a [ResourceRequirement],
    pub desired_effects: &'a BTreeMap<DesiredEffectIdentity, DesiredEffect>,
    pub revisions: &'a BTreeMap<crate::EffectRevisionId, EffectRevision>,
    pub managed: &'a [ManagedItem],
    pub observed: &'a ResourceObservation,
}

/// Planner result distinguishes a usable complete projection from a structured blocked state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanningResult<T> {
    Projected(T),
    Blocked(Vec<ConditionProposal>),
}

/// One exact external mutation proposed for durable journaling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedMutation {
    pub managed_identity: ManagedIdentity,
    pub desired_effect: Option<DesiredEffectIdentity>,
    pub mutation: EffectMutation,
    pub expected: ExactPreviousState,
    pub planned: ExactPlannedState,
    pub input: Option<VersionedMaterializationInput>,
}

/// Ledger-only cleanup is separate from mutation so absence never creates a fake Operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannedResourceChange {
    Mutate(Box<PlannedMutation>),
    ForgetMissing(ManagedIdentity),
}

/// Complete Resource projection plus all changes required to reach it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePlan {
    pub projection: ResourceProjection,
    pub preserved: Vec<PreservedItem>,
    pub changes: Vec<PlannedResourceChange>,
}

/// Projects one Effect kind onto a complete Generic Target.
///
/// Implementations must be deterministic pure logic and report unsupported or invalid input as
/// structured Conditions rather than silently dropping it.
pub trait EffectKindPlanner {
    /// Produces the complete Target snapshot for one Desired generation and Consumer Revision.
    fn project(
        &self,
        input: TargetPlanningInput<'_>,
    ) -> Result<PlanningResult<TargetProjection>, PlannerError>;
}

/// Merges all Target requirements and plans one independently locked Resource.
///
/// Implementations must preserve external items without exact ledger evidence and may only plan
/// updates/deletes for matching Managed Items.
pub trait ResourcePlanner {
    /// Produces the unique Resource projection and exact mutation plan for a generation.
    fn merge(
        &self,
        input: ResourcePlanningInput<'_>,
    ) -> Result<PlanningResult<ResourcePlan>, PlannerError>;
}

/// First Effect-kind planner for Skill definitions and filesystem directory Resources.
#[derive(Clone, Copy, Debug, Default)]
pub struct SkillPlanner;

impl EffectKindPlanner for SkillPlanner {
    fn project(
        &self,
        input: TargetPlanningInput<'_>,
    ) -> Result<PlanningResult<TargetProjection>, PlannerError> {
        validate_target_input(&input)?;
        let owner = ConditionOwner::Target(input.target.identity.clone());
        let mut conditions = Vec::new();
        let mut selected = BTreeSet::new();

        for desired in input.desired.effects.values() {
            if input.target.lifecycle == crate::TargetLifecycle::Retiring {
                continue;
            }
            if !desired.audience.selects(
                &input.target.consumer,
                &input.consumer_revision.capabilities,
            ) {
                continue;
            }
            let Some(revision) = input.revisions.get(&desired.revision) else {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::DesiredEffect(desired.identity.clone()),
                    "revision_missing",
                    input.desired.generation,
                    "The selected immutable Effect revision is unavailable.",
                    ConditionRetry::OnChange,
                ));
                continue;
            };
            if matches!(revision.availability, RevisionAvailability::Unavailable(_)) {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::DesiredEffect(desired.identity.clone()),
                    "revision_unavailable",
                    input.desired.generation,
                    "The selected immutable Effect revision is unavailable.",
                    ConditionRetry::OnChange,
                ));
                continue;
            }
            if revision.definition.kind() != desired.parameters.kind() {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::DesiredEffect(desired.identity.clone()),
                    "effect_kind_mismatch",
                    input.desired.generation,
                    "The Desired parameters do not match the selected definition kind.",
                    ConditionRetry::OnChange,
                ));
                continue;
            }
            if input
                .consumer_revision
                .capabilities
                .effect_protocols
                .get(&revision.definition.kind())
                != Some(&1)
            {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::DesiredEffect(desired.identity.clone()),
                    "unsupported_effect",
                    input.desired.generation,
                    "The Consumer Revision does not support this Effect protocol.",
                    ConditionRetry::OnChange,
                ));
                continue;
            }
            selected.insert(desired.identity.clone());
        }

        let mut requirements = BTreeMap::new();
        for binding in input.declaration.bindings.values() {
            if !input.resources.contains_key(&binding.resource) {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::Resource(binding.resource.clone()),
                    "resource_declaration_missing",
                    input.desired.generation,
                    "The Target binding refers to a Resource outside its declaration.",
                    ConditionRetry::OnChange,
                ));
                continue;
            }
            if !binding
                .accepts
                .is_satisfied_by(&input.consumer_revision.capabilities)
            {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::Resource(binding.resource.clone()),
                    "invalid_resource_binding",
                    input.desired.generation,
                    "The Target binding exceeds the Consumer Revision capabilities.",
                    ConditionRetry::OnChange,
                ));
                continue;
            }
            let contract = MaterializationContract::skill_directory_v1();
            let draft = ResourceRequirementDigest {
                target: input.target.identity.as_str(),
                resource: binding.resource.as_str(),
                generation: input.desired.generation.value(),
                consumer_revision: input.consumer_revision.identity.as_str(),
                desired_effects: &selected,
                contract: &contract,
            };
            let requirement_digest = digest_serializable(&draft)?;
            requirements.insert(
                binding.resource.clone(),
                ResourceRequirement {
                    target: input.target.identity.clone(),
                    resource: binding.resource.clone(),
                    desired_effects: selected.clone(),
                    materialization_contract: contract,
                    digest: requirement_digest,
                },
            );
        }

        if !conditions.is_empty() {
            return Ok(PlanningResult::Blocked(conditions));
        }
        let draft = TargetProjectionDigest {
            target: input.target.identity.as_str(),
            generation: input.desired.generation.value(),
            consumer_revision: input.consumer_revision.identity.as_str(),
            desired_effects: &selected,
            requirements: &requirements,
        };
        let projection_digest = ProjectionDigest::new(digest_serializable(&draft)?);
        Ok(PlanningResult::Projected(TargetProjection {
            target: input.target.identity.clone(),
            generation: input.desired.generation,
            consumer_revision: input.consumer_revision.identity.clone(),
            desired_effects: selected,
            resource_requirements: requirements,
            digest: projection_digest,
        }))
    }
}

impl ResourcePlanner for SkillPlanner {
    fn merge(
        &self,
        input: ResourcePlanningInput<'_>,
    ) -> Result<PlanningResult<ResourcePlan>, PlannerError> {
        if input.observed.resource != input.resource.identity {
            return Err(PlannerError::ObservationResourceMismatch);
        }
        let owner = ConditionOwner::Resource(input.resource.identity.clone());
        let (preserved, observed_managed) = classify_observation(input.managed, input.observed);
        let mut conditions = Vec::new();
        let mut contributors = BTreeSet::new();
        let mut desired_ids = BTreeSet::new();
        for requirement in input.requirements {
            if requirement.resource != input.resource.identity {
                return Err(PlannerError::RequirementResourceMismatch);
            }
            contributors.insert(requirement.target.clone());
            desired_ids.extend(requirement.desired_effects.iter().cloned());
        }

        let managed_by_desired = input
            .managed
            .iter()
            .map(|managed| (managed.desired_effect.clone(), managed))
            .collect::<BTreeMap<_, _>>();
        let mut native_owners = BTreeMap::new();
        let mut items = BTreeMap::new();
        for desired_id in &desired_ids {
            let Some(desired) = input.desired_effects.get(desired_id) else {
                return Err(PlannerError::DesiredEffectMissing(desired_id.clone()));
            };
            let Some(revision) = input.revisions.get(&desired.revision) else {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::DesiredEffect(desired_id.clone()),
                    "revision_missing",
                    input.generation,
                    "The selected immutable Effect revision is unavailable.",
                    ConditionRetry::OnChange,
                ));
                continue;
            };
            let crate::ValidatedEffectDefinition::Skill(definition) = &revision.definition;
            let native_identity =
                NativeResourceIdentity::parse(definition.source.name.canonical())?;
            if let Some(previous) =
                native_owners.insert(native_identity.clone(), desired_id.clone())
                && previous != *desired_id
            {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::DesiredEffect(desired_id.clone()),
                    "native_identity_conflict",
                    input.generation,
                    "Multiple Desired Effects resolve to the same native Resource identity.",
                    ConditionRetry::OnChange,
                ));
                continue;
            }
            let managed_identity = managed_by_desired
                .get(desired_id)
                .map(|managed| managed.identity.clone())
                .unwrap_or_else(|| {
                    ManagedIdentity::for_intent(&input.resource.identity, desired_id)
                });
            let materialization_input =
                VersionedMaterializationInput::SkillDirectoryV1(SkillMaterializationInput {
                    name: definition.source.name.clone(),
                    source: definition.source.clone(),
                    package_root: definition.package_root.clone(),
                    skill_md_digest: definition.skill_md_digest.clone(),
                    package_fingerprint: definition.package_fingerprint.clone(),
                });
            items.insert(
                managed_identity.clone(),
                ResolvedMaterialization {
                    managed_identity,
                    desired_effect: desired_id.clone(),
                    revision: revision.identity.clone(),
                    native_identity,
                    contract: MaterializationContract::skill_directory_v1(),
                    input_digest: digest_serializable(&materialization_input)?,
                    input: materialization_input,
                },
            );
        }

        let preserved_by_native = preserved
            .iter()
            .map(|item| (item.native_identity.clone(), item))
            .collect::<BTreeMap<_, _>>();
        for item in items.values() {
            if preserved_by_native.contains_key(&item.native_identity) {
                conditions.push(blocking_condition(
                    owner.clone(),
                    ConditionSubject::DesiredEffect(item.desired_effect.clone()),
                    "preserved_item_conflict",
                    input.generation,
                    "A Preserved Item already occupies the required native identity.",
                    ConditionRetry::OnChange,
                ));
            }
        }
        if !conditions.is_empty() {
            return Ok(PlanningResult::Blocked(conditions));
        }

        let projection_draft = ResourceProjectionDigest {
            resource: input.resource.identity.as_str(),
            generation: input.generation.value(),
            contributors: &contributors,
            items: &items,
        };
        let projection_digest = ProjectionDigest::new(digest_serializable(&projection_draft)?);
        let projection = ResourceProjection {
            resource: input.resource.identity.clone(),
            generation: input.generation,
            contributors,
            items,
            digest: projection_digest,
        };
        let changes = plan_changes(
            input.generation,
            input.managed,
            &observed_managed,
            &projection,
            &owner,
            &mut conditions,
        );
        if !conditions.is_empty() {
            return Ok(PlanningResult::Blocked(conditions));
        }
        Ok(PlanningResult::Projected(ResourcePlan {
            projection,
            preserved,
            changes,
        }))
    }
}

/// Verifies that all Target projection inputs describe the same Target and capability snapshot.
fn validate_target_input(input: &TargetPlanningInput<'_>) -> Result<(), PlannerError> {
    if input.desired.scope != input.target.scope {
        return Err(PlannerError::ScopeMismatch);
    }
    if input.target.consumer != input.consumer_revision.consumer {
        return Err(PlannerError::ConsumerMismatch);
    }
    if input.target.consumer_revision != input.consumer_revision.identity
        || input.declaration.consumer_revision != input.consumer_revision.identity
        || input.declaration.target != input.target.identity
    {
        return Err(PlannerError::ConsumerRevisionMismatch);
    }
    Ok(())
}

/// Separates exact ledger matches from Preserved Items without treating marker claims as ownership.
fn classify_observation<'a>(
    managed: &'a [ManagedItem],
    observation: &'a ResourceObservation,
) -> (
    Vec<PreservedItem>,
    BTreeMap<ManagedIdentity, &'a crate::ObservedItem>,
) {
    let ledger = managed
        .iter()
        .map(|item| (item.identity.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let mut preserved = Vec::new();
    let mut matched = BTreeMap::new();
    for observed in observation.items.values() {
        let exact = match &observed.ownership_evidence {
            OwnershipEvidence::Claims(identity) => ledger.get(identity).filter(|managed| {
                managed.resource == observation.resource
                    && managed.native_identity == observed.native_identity
            }),
            OwnershipEvidence::NoOwnershipEvidence => None,
        };
        if let Some(managed) = exact {
            matched.insert(managed.identity.clone(), observed);
        } else {
            preserved.push(PreservedItem {
                resource: observation.resource.clone(),
                native_identity: observed.native_identity.clone(),
                fingerprint: observed.fingerprint.clone(),
            });
        }
    }
    (preserved, matched)
}

/// Plans only ledger-authorized mutations and reports drift instead of guessing external state.
fn plan_changes(
    generation: Generation,
    managed: &[ManagedItem],
    observed: &BTreeMap<ManagedIdentity, &crate::ObservedItem>,
    projection: &ResourceProjection,
    owner: &ConditionOwner,
    conditions: &mut Vec<ConditionProposal>,
) -> Vec<PlannedResourceChange> {
    let desired_by_identity = &projection.items;
    let mut changes = Vec::new();
    for managed_item in managed {
        let current = observed.get(&managed_item.identity).copied();
        if let Some(current) = current
            && current.fingerprint != managed_item.fingerprint
        {
            conditions.push(blocking_condition(
                owner.clone(),
                ConditionSubject::ManagedItem(managed_item.identity.clone()),
                "managed_item_drift",
                generation,
                "A Managed Item changed outside Ora and cannot be overwritten safely.",
                ConditionRetry::Manual,
            ));
            continue;
        }
        let desired = desired_by_identity.get(&managed_item.identity);
        match (current, desired) {
            (None, None) => {
                changes.push(PlannedResourceChange::ForgetMissing(
                    managed_item.identity.clone(),
                ));
            }
            (Some(current), None) => {
                changes.push(PlannedResourceChange::Mutate(Box::new(PlannedMutation {
                    managed_identity: managed_item.identity.clone(),
                    desired_effect: None,
                    mutation: EffectMutation::Delete,
                    expected: ExactPreviousState::Present {
                        native_identity: current.native_identity.clone(),
                        fingerprint: current.fingerprint.clone(),
                        managed_identity: managed_item.identity.clone(),
                    },
                    planned: ExactPlannedState::Missing,
                    input: None,
                })));
            }
            (None, Some(desired)) => {
                changes.push(materialize_change(
                    managed_item,
                    desired,
                    EffectMutation::Create,
                    ExactPreviousState::Missing,
                ));
            }
            (Some(current), Some(desired)) => {
                let crate::VersionedMaterializationInput::SkillDirectoryV1(input) = &desired.input;
                if current.fingerprint == input.package_fingerprint
                    && managed_item.applied_revision == desired.revision
                    && managed_item.native_identity == desired.native_identity
                {
                    continue;
                }
                let mutation = if managed_item.native_identity == desired.native_identity {
                    EffectMutation::Update
                } else {
                    EffectMutation::Replace
                };
                changes.push(materialize_change(
                    managed_item,
                    desired,
                    mutation,
                    ExactPreviousState::Present {
                        native_identity: current.native_identity.clone(),
                        fingerprint: current.fingerprint.clone(),
                        managed_identity: managed_item.identity.clone(),
                    },
                ));
            }
        }
    }

    let existing = managed
        .iter()
        .map(|item| item.identity.clone())
        .collect::<BTreeSet<_>>();
    for desired in desired_by_identity.values() {
        if existing.contains(&desired.managed_identity) {
            continue;
        }
        let crate::VersionedMaterializationInput::SkillDirectoryV1(input) = &desired.input;
        changes.push(PlannedResourceChange::Mutate(Box::new(PlannedMutation {
            managed_identity: desired.managed_identity.clone(),
            desired_effect: Some(desired.desired_effect.clone()),
            mutation: EffectMutation::Create,
            expected: ExactPreviousState::Missing,
            planned: ExactPlannedState::Present {
                native_identity: desired.native_identity.clone(),
                fingerprint: input.package_fingerprint.clone(),
                managed_identity: desired.managed_identity.clone(),
            },
            input: Some(desired.input.clone()),
        })));
    }
    changes
}

/// Builds an update/replace/create proposal while retaining the stable ownership identity.
fn materialize_change(
    managed: &ManagedItem,
    desired: &ResolvedMaterialization,
    mutation: EffectMutation,
    expected: ExactPreviousState,
) -> PlannedResourceChange {
    let crate::VersionedMaterializationInput::SkillDirectoryV1(input) = &desired.input;
    PlannedResourceChange::Mutate(Box::new(PlannedMutation {
        managed_identity: managed.identity.clone(),
        desired_effect: Some(desired.desired_effect.clone()),
        mutation,
        expected,
        planned: ExactPlannedState::Present {
            native_identity: desired.native_identity.clone(),
            fingerprint: input.package_fingerprint.clone(),
            managed_identity: managed.identity.clone(),
        },
        input: Some(desired.input.clone()),
    }))
}

/// Constructs one deterministic blocking fact without leaking adapter-specific details.
fn blocking_condition(
    owner: ConditionOwner,
    subject: ConditionSubject,
    code: &'static str,
    generation: Generation,
    message: &'static str,
    retry: ConditionRetry,
) -> ConditionProposal {
    ConditionProposal {
        owner,
        subject,
        code: StableConditionCode::built_in(code),
        impact: ConditionImpact::Blocking,
        retry,
        generation: ConditionGeneration::At(generation),
        safe_details: SafeConditionDetails {
            message: message.to_string(),
            parameters: BTreeMap::new(),
        },
    }
}

/// Hashes deterministic serialized planner state into a projection content identity.
fn digest_serializable(value: &impl Serialize) -> Result<Digest, PlannerError> {
    serde_json::to_vec(value)
        .map(|bytes| Digest::sha256(&bytes))
        .map_err(PlannerError::Serialize)
}

#[derive(Serialize)]
struct ResourceRequirementDigest<'a> {
    target: &'a str,
    resource: &'a str,
    generation: u64,
    consumer_revision: &'a str,
    desired_effects: &'a BTreeSet<DesiredEffectIdentity>,
    contract: &'a MaterializationContract,
}

#[derive(Serialize)]
struct TargetProjectionDigest<'a> {
    target: &'a str,
    generation: u64,
    consumer_revision: &'a str,
    desired_effects: &'a BTreeSet<DesiredEffectIdentity>,
    requirements: &'a BTreeMap<EffectResourceId, ResourceRequirement>,
}

#[derive(Serialize)]
struct ResourceProjectionDigest<'a> {
    resource: &'a str,
    generation: u64,
    contributors: &'a BTreeSet<crate::EffectTargetId>,
    items: &'a BTreeMap<ManagedIdentity, ResolvedMaterialization>,
}

/// Reports contradictory snapshots or serialization failure before any mutation is authorized.
#[derive(Debug, Error)]
pub enum PlannerError {
    #[error("Target and Desired State belong to different Effect Scopes")]
    ScopeMismatch,
    #[error("Target and Consumer Revision refer to different Consumers")]
    ConsumerMismatch,
    #[error("Target declaration does not match the exact Consumer Revision")]
    ConsumerRevisionMismatch,
    #[error("Resource observation belongs to a different Resource")]
    ObservationResourceMismatch,
    #[error("Target requirement belongs to a different Resource")]
    RequirementResourceMismatch,
    #[error("Desired Effect {0} is missing from the complete Desired State")]
    DesiredEffectMissing(DesiredEffectIdentity),
    #[error("failed to serialize deterministic planner state")]
    Serialize(#[source] serde_json::Error),
    #[error(transparent)]
    Identity(#[from] crate::IdentityError),
}
