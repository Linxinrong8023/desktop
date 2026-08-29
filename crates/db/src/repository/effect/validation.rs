use crate::DatabaseError;
use ora_effect::{
    ConditionOwner, ConditionProposal, EffectResourceId, EffectTargetId, Generation, ManagedItem,
    ResourceProjection, ResourceStatus, TargetProjection, TargetStatus,
};
use rusqlite::{OptionalExtension, Transaction, params};
use std::collections::BTreeSet;

/// Validates immutable projection inputs against the claimed Target's current Scope and topology.
///
/// This check lives at the persistence boundary because projection identities intentionally retain
/// history without foreign keys to current Targets and Resources.
pub(super) fn validate_projection_scope(
    transaction: &Transaction<'_>,
    claimed_target: &EffectTargetId,
    generation: Generation,
    target_projections: &[TargetProjection],
    resource_projections: &[ResourceProjection],
) -> Result<BTreeSet<EffectResourceId>, DatabaseError> {
    let claimed_scope = target_scope(transaction, claimed_target)?;
    let mut projected_targets = BTreeSet::new();
    for projection in target_projections {
        if !projected_targets.insert(projection.target.clone()) {
            return Err(DatabaseError::CorruptEffectState(format!(
                "duplicate Effect Target projection for {}",
                projection.target
            )));
        }
        let current = transaction
            .query_row(
                "SELECT scope_id, consumer_revision_id FROM effect_targets WHERE id = ?1",
                params![projection.target.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if current
            .as_ref()
            .map(|(scope, revision)| (scope.as_str(), revision.as_str()))
            != Some((
                claimed_scope.as_str(),
                projection.consumer_revision.as_str(),
            ))
            || projection.generation != generation
        {
            return Err(DatabaseError::CorruptEffectState(format!(
                "Effect Target projection {} crosses its claimed Scope or current Revision",
                projection.target
            )));
        }
        for (resource_id, requirement) in &projection.resource_requirements {
            if requirement.target != projection.target
                || requirement.resource != *resource_id
                || resource_scope(transaction, resource_id)?.as_deref()
                    != Some(claimed_scope.as_str())
            {
                return Err(DatabaseError::CorruptEffectState(format!(
                    "Effect Resource requirement {} crosses its Target projection Scope",
                    requirement.digest
                )));
            }
        }
    }

    let mut projected_resources = BTreeSet::new();
    for projection in resource_projections {
        if !projected_resources.insert(projection.resource.clone())
            || resource_scope(transaction, &projection.resource)?.as_deref()
                != Some(claimed_scope.as_str())
            || projection.generation != generation
        {
            return Err(DatabaseError::CorruptEffectState(format!(
                "Effect Resource projection {} crosses its claimed Scope",
                projection.resource
            )));
        }
        if projection
            .contributors
            .iter()
            .any(|target| !projected_targets.contains(target))
        {
            return Err(DatabaseError::CorruptEffectState(format!(
                "Effect Resource projection {} lacks a persisted contributor projection",
                projection.resource
            )));
        }
    }
    Ok(projected_resources)
}

/// Validates every mutable current-state fragment before a claimed Target can commit it.
#[allow(clippy::too_many_arguments)]
pub(super) fn validate_current_state_scope(
    transaction: &Transaction<'_>,
    claimed_target: &EffectTargetId,
    generation: Generation,
    authorized_resources: &BTreeSet<EffectResourceId>,
    target_statuses: &[TargetStatus],
    resource_statuses: &[ResourceStatus],
    managed: &[ManagedItem],
    removed_managed: &[ora_effect::ManagedIdentity],
    conditions: &[ConditionProposal],
) -> Result<(), DatabaseError> {
    let claimed_scope = target_scope(transaction, claimed_target)?;
    let mut targets = BTreeSet::new();
    for status in target_statuses {
        if !targets.insert(status.target().clone()) || status.target() != claimed_target {
            return Err(DatabaseError::CorruptEffectState(format!(
                "Effect Target status {} crosses its claimed Scope",
                status.target()
            )));
        }
    }

    let mut resources = BTreeSet::new();
    for status in resource_statuses {
        if !resources.insert(status.resource().clone())
            || !authorized_resources.contains(status.resource())
            || resource_scope(transaction, status.resource())?.as_deref()
                != Some(claimed_scope.as_str())
        {
            return Err(DatabaseError::CorruptEffectState(format!(
                "Effect Resource status {} crosses its claimed Scope",
                status.resource()
            )));
        }
    }

    let mut managed_ids = BTreeSet::new();
    for item in managed {
        let existing = managed_owner(transaction, &item.identity)?;
        if !managed_ids.insert(item.identity.clone())
            || item.applied_generation != generation
            || !authorized_resources.contains(&item.resource)
            || resource_scope(transaction, &item.resource)?.as_deref()
                != Some(claimed_scope.as_str())
            || existing.as_ref().is_some_and(|(scope, resource)| {
                scope != &claimed_scope || resource != item.resource.as_str()
            })
        {
            return Err(DatabaseError::CorruptEffectState(format!(
                "Managed Item {} crosses its claimed Scope or generation",
                item.identity
            )));
        }
    }
    for identity in removed_managed {
        let owner = managed_owner(transaction, identity)?;
        if !managed_ids.insert(identity.clone())
            || owner.as_ref().map(|(scope, _)| scope.as_str()) != Some(claimed_scope.as_str())
            || owner.as_ref().is_none_or(|(_, resource)| {
                !authorized_resources.contains(&EffectResourceId::new(resource))
            })
        {
            return Err(DatabaseError::CorruptEffectState(format!(
                "removed Managed Item {identity} crosses its claimed Scope"
            )));
        }
    }

    for condition in conditions {
        let owner_scope = match &condition.owner {
            ConditionOwner::Target(target) if target == claimed_target => {
                Some(target_scope(transaction, target)?)
            }
            ConditionOwner::Resource(resource) if authorized_resources.contains(resource) => {
                resource_scope(transaction, resource)?
            }
            ConditionOwner::Target(_) | ConditionOwner::Resource(_) => None,
        };
        if owner_scope.as_deref() != Some(claimed_scope.as_str()) {
            return Err(DatabaseError::CorruptEffectState(
                "Effect Condition crosses its claimed Scope".to_string(),
            ));
        }
    }
    Ok(())
}

/// Loads the required Scope for a current Target.
fn target_scope(
    transaction: &Transaction<'_>,
    target: &EffectTargetId,
) -> Result<String, DatabaseError> {
    transaction
        .query_row(
            "SELECT scope_id FROM effect_targets WHERE id = ?1",
            params![target.as_str()],
            |row| row.get::<_, String>(0),
        )
        .map_err(DatabaseError::from)
}

/// Loads a Resource Scope without treating a missing current Resource as historical authority.
fn resource_scope(
    transaction: &Transaction<'_>,
    resource: &EffectResourceId,
) -> Result<Option<String>, DatabaseError> {
    transaction
        .query_row(
            "SELECT scope_id FROM effect_resources WHERE id = ?1",
            params![resource.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(DatabaseError::from)
}

/// Loads the durable ownership Scope used to fence ledger removal and replacement.
fn managed_owner(
    transaction: &Transaction<'_>,
    identity: &ora_effect::ManagedIdentity,
) -> Result<Option<(String, String)>, DatabaseError> {
    transaction
        .query_row(
            "SELECT scope_id, resource_id FROM effect_managed_items WHERE id = ?1",
            params![identity.as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(DatabaseError::from)
}
