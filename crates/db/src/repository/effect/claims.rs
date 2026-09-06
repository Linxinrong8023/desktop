use super::mapping::generation_from_sql;
use crate::DatabaseError;
use ora_effect::{
    EffectResourceId, EffectTargetId, Fingerprint, ManagedIdentity, ManagedItem, ReconcileClaim,
};
use rusqlite::{Connection, Transaction, params};
use std::collections::BTreeSet;

/// Checks every identifying field of the Target claim before a fenced transition.
pub(super) fn verify_claim(
    connection: &Connection,
    target: &EffectTargetId,
    claim: &ReconcileClaim,
) -> Result<(), DatabaseError> {
    if !claim_is_valid(connection, target, claim)? {
        return Err(DatabaseError::CorruptEffectState(
            "stale or mismatched Effect Target claim".to_string(),
        ));
    }
    Ok(())
}

/// Checks claim identity separately so optional retry transitions can reject stale authority.
pub(super) fn claim_is_valid(
    connection: &Connection,
    target: &EffectTargetId,
    claim: &ReconcileClaim,
) -> Result<bool, DatabaseError> {
    let token = i64::try_from(claim.token.value()).map_err(|_| {
        DatabaseError::CorruptEffectState("Target fencing token exceeds SQLite INTEGER".to_string())
    })?;
    let valid = connection.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM effect_reconcile_requests
             WHERE target_id = ?1 AND state = 'claimed' AND claim_token = ?2
               AND claim_worker = ?3 AND lease_until = ?4
         )",
        params![
            target.as_str(),
            token,
            claim.worker.as_str(),
            claim.lease_until.millis(),
        ],
        |row| row.get::<_, i64>(0),
    )? != 0;
    Ok(valid)
}

/// Releases only Resource leases derived from the exact fenced Target claim.
pub(super) fn release_resource_claims(
    transaction: &Transaction<'_>,
    target: &EffectTargetId,
    claim: &ReconcileClaim,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "DELETE FROM effect_resource_claims
         WHERE target_id = ?1 AND target_claim_token = ?2",
        params![
            target.as_str(),
            i64::try_from(claim.token.value()).map_err(|_| {
                DatabaseError::CorruptEffectState(
                    "Target fencing token exceeds SQLite INTEGER".to_string(),
                )
            })?,
        ],
    )?;
    Ok(())
}

/// Verifies that the current Target claim still owns every Resource transition it will commit.
pub(super) fn verify_resource_claims(
    transaction: &Transaction<'_>,
    target: &EffectTargetId,
    claim: &ReconcileClaim,
    resources: &BTreeSet<EffectResourceId>,
) -> Result<(), DatabaseError> {
    let token = i64::try_from(claim.token.value()).map_err(|_| {
        DatabaseError::CorruptEffectState("Target fencing token exceeds SQLite INTEGER".to_string())
    })?;
    for resource in resources {
        let valid = transaction.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM effect_resource_claims resource_claim
                 JOIN effect_resources resource ON resource.id = resource_claim.resource_id
                 WHERE resource_claim.resource_id = ?1 AND resource_claim.target_id = ?2
                   AND resource_claim.target_claim_token = ?3 AND resource_claim.worker = ?4
                   AND resource_claim.resource_fence = resource.claim_fence
             )",
            params![
                resource.as_str(),
                target.as_str(),
                token,
                claim.worker.as_str(),
            ],
            |row| row.get::<_, i64>(0),
        )? != 0;
        if !valid {
            return Err(DatabaseError::CorruptEffectState(format!(
                "stale or missing Effect Resource claim {resource}"
            )));
        }
    }
    Ok(())
}

/// Finds every Target whose current binding contributes to one claimed Resource.
pub(super) fn load_related_target_ids(
    connection: &Connection,
    resources: &BTreeSet<EffectResourceId>,
) -> Result<Vec<EffectTargetId>, DatabaseError> {
    let mut targets = BTreeSet::new();
    for resource in resources {
        let mut statement = connection.prepare(
            "SELECT target.id FROM effect_target_resource_bindings binding
             JOIN effect_targets target ON target.id = binding.target_id
             WHERE binding.resource_id = ?1 AND target.lifecycle IN ('active', 'retiring')
             ORDER BY target.id",
        )?;
        let rows = statement
            .query_map(params![resource.as_str()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        targets.extend(rows.into_iter().map(EffectTargetId::new));
    }
    Ok(targets.into_iter().collect())
}

/// Loads the exact ownership ledger for one Resource.
pub(super) fn load_managed(
    connection: &Connection,
    resource: &EffectResourceId,
) -> Result<Vec<ManagedItem>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT id, desired_effect_id, applied_revision_id, native_identity,
                fingerprint, applied_generation
         FROM effect_managed_items WHERE resource_id = ?1 ORDER BY native_identity, id",
    )?;
    let rows = statement.query_map(params![resource.as_str()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let mut managed = Vec::new();
    for row in rows {
        let (id, desired, revision, native, fingerprint, generation) = row?;
        managed.push(ManagedItem {
            identity: ManagedIdentity::new(id),
            resource: resource.clone(),
            desired_effect: ora_effect::DesiredEffectIdentity::new(desired),
            applied_revision: ora_effect::EffectRevisionId::new(revision),
            native_identity: ora_effect::NativeResourceIdentity::parse(native)
                .map_err(|error| DatabaseError::CorruptEffectState(error.to_string()))?,
            fingerprint: Fingerprint::parse(fingerprint)
                .map_err(|error| DatabaseError::CorruptEffectState(error.to_string()))?,
            applied_generation: generation_from_sql(generation)?,
        });
    }
    Ok(managed)
}
