use crate::DatabaseError;
use ora_effect::{
    DesiredEffectIdentity, EffectOperation, EffectResourceId, EffectRevisionId, ExactPlannedState,
    ExactPreviousState, ManagedIdentity, ManagedItem, NativeResourceIdentity, ReconcileAttemptId,
    ResourceProjection,
};
use rusqlite::{Transaction, params};
use std::collections::{BTreeMap, BTreeSet};

/// Scalar projection facts that every finalized ownership ledger row must preserve.
#[derive(Debug, Eq, PartialEq)]
struct ExpectedManagedItem {
    resource: EffectResourceId,
    desired_effect: DesiredEffectIdentity,
    revision: EffectRevisionId,
    native_identity: NativeResourceIdentity,
}

/// Validates a no-mutation commit against its complete in-memory Resource projections.
pub(super) fn validate_projection_managed_transition(
    transaction: &Transaction<'_>,
    projections: &[ResourceProjection],
    managed: &[ManagedItem],
    removed: &[ManagedIdentity],
) -> Result<(), DatabaseError> {
    let mut expected = BTreeMap::new();
    let resources = projections
        .iter()
        .map(|projection| projection.resource.clone())
        .collect::<BTreeSet<_>>();
    for projection in projections {
        for materialization in projection.items.values() {
            let duplicate = expected.insert(
                materialization.managed_identity.clone(),
                ExpectedManagedItem {
                    resource: projection.resource.clone(),
                    desired_effect: materialization.desired_effect.clone(),
                    revision: materialization.revision.clone(),
                    native_identity: materialization.native_identity.clone(),
                },
            );
            if duplicate.is_some() {
                return Err(DatabaseError::CorruptEffectState(format!(
                    "Managed identity {} appears in multiple Resource projections",
                    materialization.managed_identity
                )));
            }
        }
    }
    validate_managed_transition(transaction, &resources, &expected, managed, removed)
}

/// Validates finalization ledgers against the Resource projections sealed into an Attempt.
pub(super) fn validate_attempt_managed_transition(
    transaction: &Transaction<'_>,
    attempt: &ReconcileAttemptId,
    managed: &[ManagedItem],
    removed: &[ManagedIdentity],
) -> Result<(), DatabaseError> {
    let mut statement = transaction.prepare(
        "SELECT materialization.managed_identity, materialization.resource_id,
                materialization.desired_effect_id, materialization.revision_id,
                materialization.native_identity
         FROM effect_attempt_resource_projections attempt_projection
         JOIN effect_resolved_materializations materialization
           ON materialization.projection_digest = attempt_projection.resource_projection_digest
         WHERE attempt_projection.attempt_id = ?1",
    )?;
    let rows = statement
        .query_map(params![attempt.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut expected = BTreeMap::new();
    let mut resources = BTreeSet::new();
    for (identity, resource, desired, revision, native) in rows {
        let resource = EffectResourceId::new(resource);
        resources.insert(resource.clone());
        let identity = ManagedIdentity::new(identity);
        let item = ExpectedManagedItem {
            resource,
            desired_effect: DesiredEffectIdentity::new(desired),
            revision: EffectRevisionId::new(revision),
            native_identity: NativeResourceIdentity::parse(native)
                .map_err(|error| DatabaseError::CorruptEffectState(error.to_string()))?,
        };
        if expected.insert(identity.clone(), item).is_some() {
            return Err(DatabaseError::CorruptEffectState(format!(
                "Managed identity {identity} appears in multiple Attempt projections"
            )));
        }
    }
    let mut resource_statement = transaction.prepare(
        "SELECT projection.resource_id
         FROM effect_attempt_resource_projections attempt_projection
         JOIN effect_resource_projections projection
           ON projection.digest = attempt_projection.resource_projection_digest
         WHERE attempt_projection.attempt_id = ?1",
    )?;
    resources.extend(
        resource_statement
            .query_map(params![attempt.as_str()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(EffectResourceId::new),
    );
    validate_managed_transition(transaction, &resources, &expected, managed, removed)
}

/// Ties final ledger fingerprints and removals to each verified Operation's exact planned state.
pub(super) fn validate_operation_managed_evidence(
    operations: &[EffectOperation],
    managed: &[ManagedItem],
    removed: &[ManagedIdentity],
) -> Result<(), DatabaseError> {
    let managed = managed
        .iter()
        .map(|item| (item.identity.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let removed = removed.iter().collect::<BTreeSet<_>>();
    for operation in operations {
        match operation.planned() {
            ExactPlannedState::Present {
                native_identity,
                fingerprint,
                managed_identity,
            } => {
                if managed.get(managed_identity).is_none_or(|item| {
                    item.resource != *operation.resource()
                        || item.native_identity != *native_identity
                        || item.fingerprint != *fingerprint
                }) {
                    return Err(DatabaseError::CorruptEffectState(format!(
                        "Operation {} lacks its exact finalized Managed Item",
                        operation.identity()
                    )));
                }
            }
            ExactPlannedState::Missing => {
                let ExactPreviousState::Present {
                    managed_identity, ..
                } = operation.expected()
                else {
                    return Err(DatabaseError::CorruptEffectState(format!(
                        "delete Operation {} lacks previous ownership",
                        operation.identity()
                    )));
                };
                if !removed.contains(managed_identity) {
                    return Err(DatabaseError::CorruptEffectState(format!(
                        "delete Operation {} did not remove its Managed Item",
                        operation.identity()
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Compares the submitted ledger transition with both its projection and the current ledger.
fn validate_managed_transition(
    transaction: &Transaction<'_>,
    resources: &BTreeSet<EffectResourceId>,
    expected: &BTreeMap<ManagedIdentity, ExpectedManagedItem>,
    managed: &[ManagedItem],
    removed: &[ManagedIdentity],
) -> Result<(), DatabaseError> {
    let supplied = managed
        .iter()
        .map(|item| (item.identity.clone(), item))
        .collect::<BTreeMap<_, _>>();
    if supplied.len() != managed.len()
        || supplied.len() != expected.len()
        || expected.iter().any(|(identity, expected)| {
            supplied.get(identity).is_none_or(|item| {
                item.resource != expected.resource
                    || item.desired_effect != expected.desired_effect
                    || item.applied_revision != expected.revision
                    || item.native_identity != expected.native_identity
            })
        })
    {
        return Err(DatabaseError::CorruptEffectState(
            "Effect Managed ledger does not match its complete Resource projections".to_string(),
        ));
    }

    let mut current = BTreeSet::new();
    for resource in resources {
        let mut statement =
            transaction.prepare("SELECT id FROM effect_managed_items WHERE resource_id = ?1")?;
        current.extend(
            statement
                .query_map(params![resource.as_str()], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(ManagedIdentity::new),
        );
    }
    let expected_identities = expected.keys().cloned().collect::<BTreeSet<_>>();
    let expected_removed = current
        .difference(&expected_identities)
        .cloned()
        .collect::<BTreeSet<_>>();
    let supplied_removed = removed.iter().cloned().collect::<BTreeSet<_>>();
    if supplied_removed.len() != removed.len() || supplied_removed != expected_removed {
        return Err(DatabaseError::CorruptEffectState(
            "Effect Managed ledger removal is not the exact projection difference".to_string(),
        ));
    }
    Ok(())
}
