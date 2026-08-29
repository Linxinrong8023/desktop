use super::mapping::{effect_json, generation_to_sql};
use crate::DatabaseError;
use ora_effect::{
    Digest, EffectResourceId, EffectTargetId, LocalTimestamp, ResourceProjection,
    ResourceRequirement, TargetProjection,
};
use rusqlite::{Transaction, params};
use std::collections::{BTreeMap, BTreeSet};

/// Saves complete immutable projections, accepting an existing digest only when every row matches.
pub(super) fn save_projections(
    transaction: &Transaction<'_>,
    targets: &[TargetProjection],
    resources: &[ResourceProjection],
    created_at: LocalTimestamp,
) -> Result<(), DatabaseError> {
    let mut requirements = BTreeMap::new();
    for projection in targets {
        save_target_projection(transaction, projection, created_at)?;
        for requirement in projection.resource_requirements.values() {
            requirements.insert(
                (requirement.resource.clone(), requirement.target.clone()),
                requirement.digest.clone(),
            );
        }
    }
    for projection in resources {
        save_resource_projection(transaction, projection, &requirements, created_at)?;
    }
    Ok(())
}

/// Inserts one Target projection as a unit or proves an idempotent replay is byte-exact.
fn save_target_projection(
    transaction: &Transaction<'_>,
    projection: &TargetProjection,
    created_at: LocalTimestamp,
) -> Result<(), DatabaseError> {
    let digest = projection.digest.digest().as_str();
    let generation = generation_to_sql(projection.generation)?;
    let inserted = transaction.execute(
        "INSERT INTO effect_target_projections (
             target_id, generation, consumer_revision_id, digest, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT DO NOTHING",
        params![
            projection.target.as_str(),
            generation,
            projection.consumer_revision.as_str(),
            digest,
            created_at.millis(),
        ],
    )?;
    if inserted == 0 {
        return validate_target_projection(transaction, projection);
    }

    for desired in &projection.desired_effects {
        transaction.execute(
            "INSERT INTO effect_target_projection_effects (
                 projection_digest, desired_effect_id
             ) VALUES (?1, ?2)",
            params![digest, desired.as_str()],
        )?;
    }
    for requirement in projection.resource_requirements.values() {
        insert_requirement(transaction, digest, generation, requirement)?;
    }
    Ok(())
}

/// Inserts one Target-owned Resource requirement after its parent projection is durable.
fn insert_requirement(
    transaction: &Transaction<'_>,
    projection_digest: &str,
    generation: i64,
    requirement: &ResourceRequirement,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO effect_resource_requirements (
             digest, target_projection_digest, target_id, generation, resource_id,
             materialization_contract_version, materialization_contract_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            requirement.digest.as_str(),
            projection_digest,
            requirement.target.as_str(),
            generation,
            requirement.resource.as_str(),
            i64::from(requirement.materialization_contract.version),
            effect_json(&requirement.materialization_contract)?,
        ],
    )?;
    for desired in &requirement.desired_effects {
        transaction.execute(
            "INSERT INTO effect_resource_requirement_effects (
                 requirement_digest, desired_effect_id
             ) VALUES (?1, ?2)",
            params![requirement.digest.as_str(), desired.as_str()],
        )?;
    }
    Ok(())
}

/// Validates every normalized row behind an existing Target projection digest.
fn validate_target_projection(
    transaction: &Transaction<'_>,
    projection: &TargetProjection,
) -> Result<(), DatabaseError> {
    let digest = projection.digest.digest().as_str();
    let generation = generation_to_sql(projection.generation)?;
    let exact_parent = transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM effect_target_projections
             WHERE digest = ?1 AND target_id = ?2 AND generation = ?3
               AND consumer_revision_id = ?4
         )",
        params![
            digest,
            projection.target.as_str(),
            generation,
            projection.consumer_revision.as_str(),
        ],
        |row| row.get::<_, i64>(0),
    )? != 0;
    if !exact_parent
        || load_string_set(
            transaction,
            "SELECT desired_effect_id FROM effect_target_projection_effects
             WHERE projection_digest = ?1",
            digest,
        )? != projection
            .desired_effects
            .iter()
            .map(|identity| identity.as_str().to_string())
            .collect()
    {
        return projection_conflict(digest);
    }

    let stored_requirements = load_string_set(
        transaction,
        "SELECT digest FROM effect_resource_requirements
         WHERE target_projection_digest = ?1",
        digest,
    )?;
    let expected_requirements = projection
        .resource_requirements
        .values()
        .map(|requirement| requirement.digest.as_str().to_string())
        .collect::<BTreeSet<_>>();
    if stored_requirements != expected_requirements {
        return projection_conflict(digest);
    }
    for requirement in projection.resource_requirements.values() {
        validate_requirement(transaction, digest, generation, requirement)?;
    }
    Ok(())
}

/// Validates an existing Requirement and its complete Desired Effect membership.
fn validate_requirement(
    transaction: &Transaction<'_>,
    projection_digest: &str,
    generation: i64,
    requirement: &ResourceRequirement,
) -> Result<(), DatabaseError> {
    let contract_json = effect_json(&requirement.materialization_contract)?;
    let exact = transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM effect_resource_requirements
             WHERE digest = ?1 AND target_projection_digest = ?2 AND target_id = ?3
               AND generation = ?4 AND resource_id = ?5
               AND materialization_contract_version = ?6
               AND materialization_contract_json = ?7
         )",
        params![
            requirement.digest.as_str(),
            projection_digest,
            requirement.target.as_str(),
            generation,
            requirement.resource.as_str(),
            i64::from(requirement.materialization_contract.version),
            contract_json,
        ],
        |row| row.get::<_, i64>(0),
    )? != 0;
    let stored_effects = load_string_set(
        transaction,
        "SELECT desired_effect_id FROM effect_resource_requirement_effects
         WHERE requirement_digest = ?1",
        requirement.digest.as_str(),
    )?;
    let expected_effects = requirement
        .desired_effects
        .iter()
        .map(|identity| identity.as_str().to_string())
        .collect::<BTreeSet<_>>();
    if !exact || stored_effects != expected_effects {
        return projection_conflict(requirement.digest.as_str());
    }
    Ok(())
}

/// Inserts one Resource projection as a unit or validates an exact digest replay.
fn save_resource_projection(
    transaction: &Transaction<'_>,
    projection: &ResourceProjection,
    requirements: &BTreeMap<(EffectResourceId, EffectTargetId), Digest>,
    created_at: LocalTimestamp,
) -> Result<(), DatabaseError> {
    let digest = projection.digest.digest().as_str();
    let generation = generation_to_sql(projection.generation)?;
    let inserted = transaction.execute(
        "INSERT INTO effect_resource_projections (resource_id, generation, digest, created_at)
         VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
        params![
            projection.resource.as_str(),
            generation,
            digest,
            created_at.millis(),
        ],
    )?;
    if inserted == 0 {
        return validate_resource_projection(transaction, projection, requirements);
    }

    for contributor in &projection.contributors {
        let requirement = contributor_requirement(projection, contributor, requirements)?;
        transaction.execute(
            "INSERT INTO effect_resource_projection_contributors (
                 projection_digest, target_id, requirement_digest
             ) VALUES (?1, ?2, ?3)",
            params![digest, contributor.as_str(), requirement.as_str()],
        )?;
    }
    for materialization in projection.items.values() {
        transaction.execute(
            "INSERT INTO effect_resolved_materializations (
                 projection_digest, resource_id, generation, managed_identity,
                 desired_effect_id, revision_id, native_identity, contract_version,
                 contract_json, input_digest, input_version, input_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11)",
            params![
                digest,
                projection.resource.as_str(),
                generation,
                materialization.managed_identity.as_str(),
                materialization.desired_effect.as_str(),
                materialization.revision.as_str(),
                materialization.native_identity.as_str(),
                i64::from(materialization.contract.version),
                effect_json(&materialization.contract)?,
                materialization.input_digest.as_str(),
                effect_json(&materialization.input)?,
            ],
        )?;
    }
    Ok(())
}

/// Validates every contributor and materialization behind an existing Resource projection.
fn validate_resource_projection(
    transaction: &Transaction<'_>,
    projection: &ResourceProjection,
    requirements: &BTreeMap<(EffectResourceId, EffectTargetId), Digest>,
) -> Result<(), DatabaseError> {
    let digest = projection.digest.digest().as_str();
    let exact_parent = transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM effect_resource_projections
             WHERE digest = ?1 AND resource_id = ?2 AND generation = ?3
         )",
        params![
            digest,
            projection.resource.as_str(),
            generation_to_sql(projection.generation)?,
        ],
        |row| row.get::<_, i64>(0),
    )? != 0;
    let mut expected_contributors = BTreeSet::new();
    for contributor in &projection.contributors {
        expected_contributors.insert((
            contributor.as_str().to_string(),
            contributor_requirement(projection, contributor, requirements)?
                .as_str()
                .to_string(),
        ));
    }
    if !exact_parent
        || load_contributors(transaction, digest)? != expected_contributors
        || load_string_set(
            transaction,
            "SELECT managed_identity FROM effect_resolved_materializations
             WHERE projection_digest = ?1",
            digest,
        )? != projection
            .items
            .keys()
            .map(|identity| identity.as_str().to_string())
            .collect()
    {
        return projection_conflict(digest);
    }
    for materialization in projection.items.values() {
        let exact = transaction.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM effect_resolved_materializations
                 WHERE projection_digest = ?1 AND resource_id = ?2 AND generation = ?3
                   AND managed_identity = ?4 AND desired_effect_id = ?5 AND revision_id = ?6
                   AND native_identity = ?7 AND contract_version = ?8 AND contract_json = ?9
                   AND input_digest = ?10 AND input_version = 1 AND input_json = ?11
             )",
            params![
                digest,
                projection.resource.as_str(),
                generation_to_sql(projection.generation)?,
                materialization.managed_identity.as_str(),
                materialization.desired_effect.as_str(),
                materialization.revision.as_str(),
                materialization.native_identity.as_str(),
                i64::from(materialization.contract.version),
                effect_json(&materialization.contract)?,
                materialization.input_digest.as_str(),
                effect_json(&materialization.input)?,
            ],
            |row| row.get::<_, i64>(0),
        )? != 0;
        if !exact {
            return projection_conflict(digest);
        }
    }
    Ok(())
}

/// Resolves the exact contributor Requirement used by a merged Resource projection.
fn contributor_requirement<'a>(
    projection: &ResourceProjection,
    contributor: &EffectTargetId,
    requirements: &'a BTreeMap<(EffectResourceId, EffectTargetId), Digest>,
) -> Result<&'a Digest, DatabaseError> {
    requirements
        .get(&(projection.resource.clone(), contributor.clone()))
        .ok_or_else(|| {
            DatabaseError::CorruptEffectState(format!(
                "Resource projection lacks contributor requirement {contributor}"
            ))
        })
}

/// Loads a one-column normalized membership set for exact snapshot comparison.
fn load_string_set(
    transaction: &Transaction<'_>,
    sql: &str,
    identity: &str,
) -> Result<BTreeSet<String>, DatabaseError> {
    let mut statement = transaction.prepare(sql)?;
    statement
        .query_map(params![identity], |row| row.get::<_, String>(0))?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(DatabaseError::from)
}

/// Loads contributor-to-Requirement edges because both identities define merge provenance.
fn load_contributors(
    transaction: &Transaction<'_>,
    projection: &str,
) -> Result<BTreeSet<(String, String)>, DatabaseError> {
    let mut statement = transaction.prepare(
        "SELECT target_id, requirement_digest
         FROM effect_resource_projection_contributors WHERE projection_digest = ?1",
    )?;
    statement
        .query_map(params![projection], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(DatabaseError::from)
}

/// Returns a stable corruption error for a digest collision or partial immutable snapshot.
fn projection_conflict<T>(digest: &str) -> Result<T, DatabaseError> {
    Err(DatabaseError::CorruptEffectState(format!(
        "immutable Effect projection digest {digest} conflicts with persisted content"
    )))
}
