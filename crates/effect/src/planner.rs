use crate::{
    ConditionProposal, DesiredEffect, DesiredEffectIdentity, DesiredState, EffectMutation,
    EffectResource, EffectResourceId, EffectRevision, ExactPlannedState, ExactPreviousState,
    Generation, ManagedIdentity, ManagedItem, PreservedItem, ResourceObservation,
    ResourceProjection, ResourceRequirement, TargetDeclaration, TargetProjection,
    VersionedMaterializationInput,
};
use std::collections::BTreeMap;
use thiserror::Error;

/// Input snapshot for deterministic projection of one complete Target.
pub struct TargetPlanningInput<'a> {
    pub desired: &'a DesiredState,
    pub target: &'a crate::EffectTarget,
    pub consumer_revision: &'a crate::ConsumerRevision,
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
