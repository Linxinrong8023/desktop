use rusqlite::{OptionalExtension, params};

use crate::DatabaseError;
use crate::repository::RepositoryPool;

/// Whether the pack installation created this member or found it already installed.
///
/// Ownership records the install action, not membership alone: a member the pack named but
/// never touched stays `PreExisting` forever, which is what makes a later pack uninstall safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackMemberOwnership {
    /// A pack installation created this member.
    ManagedByPack,
    /// The member was already installed when the pack named it; the pack never touched it.
    PreExisting,
}

impl PackMemberOwnership {
    /// Returns the persisted spelling of this ownership.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ManagedByPack => "managed_by_pack",
            Self::PreExisting => "pre_existing",
        }
    }

    /// Parses the persisted spelling back into the closed ownership set.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "managed_by_pack" => Some(Self::ManagedByPack),
            "pre_existing" => Some(Self::PreExisting),
            _ => None,
        }
    }
}

/// One member of a recorded pack installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackInstallationMemberRecord {
    /// Canonical `namespace/identifier` of the member package.
    pub member_id: String,
    /// The member version at the moment the pack relationship was established. An
    /// independently upgraded member no longer matches, which later reconciliation reports.
    pub version_at_install: String,
    /// Whether the pack installation created the member or found it already installed.
    pub ownership: PackMemberOwnership,
}

/// One recorded pack installation with its member relationships.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackInstallationRecord {
    /// Canonical `namespace/identifier` of the pack listing.
    pub pack_id: String,
    /// Canonical URL of the marketplace source the pack was installed from.
    pub source_url: String,
    /// The recorded members in stable identifier order.
    pub members: Vec<PackInstallationMemberRecord>,
}

/// Persists which packs were installed, which members each installation created, and which
/// members were already present when the pack named them.
#[derive(Clone, Debug)]
pub struct SqlitePackInstallationRepository {
    pool: RepositoryPool,
}

impl SqlitePackInstallationRepository {
    pub fn new(pool: RepositoryPool) -> Self {
        Self { pool }
    }

    /// Inserts the pack row or refreshes its source attribution.
    pub fn upsert_pack(
        &self,
        pack_id: &str,
        source_url: &str,
        now_ms: i64,
    ) -> Result<(), DatabaseError> {
        self.pool.with_connection(|connection| {
            connection.execute(
                "INSERT INTO pack_installation (pack_id, source_url, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)
                 ON CONFLICT(pack_id) DO UPDATE
                 SET source_url = excluded.source_url, updated_at = excluded.updated_at",
                params![pack_id, source_url, now_ms],
            )?;
            Ok(())
        })
    }

    /// Inserts one member relationship or refreshes its recorded version and ownership.
    pub fn upsert_member(
        &self,
        pack_id: &str,
        member: &PackInstallationMemberRecord,
        now_ms: i64,
    ) -> Result<(), DatabaseError> {
        self.pool.with_connection(|connection| {
            connection.execute(
                "INSERT INTO pack_installation_member (
                     pack_id, member_id, version_at_install, ownership, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                 ON CONFLICT(pack_id, member_id) DO UPDATE
                 SET version_at_install = excluded.version_at_install,
                     ownership = excluded.ownership,
                     updated_at = excluded.updated_at",
                params![
                    pack_id,
                    member.member_id.as_str(),
                    member.version_at_install.as_str(),
                    member.ownership.as_str(),
                    now_ms,
                ],
            )?;
            Ok(())
        })
    }

    /// Returns one recorded member relationship, absent when the pack never recorded it.
    pub fn load_member(
        &self,
        pack_id: &str,
        member_id: &str,
    ) -> Result<Option<PackInstallationMemberRecord>, DatabaseError> {
        self.pool.with_connection(|connection| {
            let row: Option<(String, String)> = connection
                .query_row(
                    "SELECT version_at_install, ownership
                     FROM pack_installation_member
                     WHERE pack_id = ?1 AND member_id = ?2",
                    params![pack_id, member_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let Some((version_at_install, ownership)) = row else {
                return Ok(None);
            };
            let ownership = PackMemberOwnership::parse(&ownership).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    format!("unknown pack member ownership {ownership:?}").into(),
                )
            })?;
            Ok(Some(PackInstallationMemberRecord {
                member_id: member_id.to_owned(),
                version_at_install,
                ownership,
            }))
        })
    }

    /// Returns one recorded pack installation with its members in identifier order, absent when
    /// the pack was never installed.
    pub fn load(&self, pack_id: &str) -> Result<Option<PackInstallationRecord>, DatabaseError> {
        self.pool.with_connection(|connection| {
            let source_url = match connection
                .query_row(
                    "SELECT source_url FROM pack_installation WHERE pack_id = ?1",
                    params![pack_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                Some(source_url) => source_url,
                None => return Ok(None),
            };
            let mut statement = connection.prepare(
                "SELECT member_id, version_at_install, ownership
                 FROM pack_installation_member
                 WHERE pack_id = ?1
                 ORDER BY member_id",
            )?;
            let members = statement
                .query_map(params![pack_id], map_member_row)?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(
                    |(member_id, version_at_install, ownership)| PackInstallationMemberRecord {
                        member_id,
                        version_at_install,
                        ownership,
                    },
                )
                .collect::<Vec<_>>();
            Ok(Some(PackInstallationRecord {
                pack_id: pack_id.to_owned(),
                source_url,
                members,
            }))
        })
    }

    /// Returns every recorded pack installation with its members, ordered by pack id.
    pub fn list(&self) -> Result<Vec<PackInstallationRecord>, DatabaseError> {
        self.pool.with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT pack_id FROM pack_installation ORDER BY pack_id")?;
            let pack_ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            let mut records = Vec::with_capacity(pack_ids.len());
            for pack_id in pack_ids {
                match self.load(&pack_id)? {
                    Some(record) => records.push(record),
                    // A pack row without any member rows is still a recorded installation.
                    None => {
                        let source_url: String = connection.query_row(
                            "SELECT source_url FROM pack_installation WHERE pack_id = ?1",
                            params![pack_id],
                            |row| row.get(0),
                        )?;
                        records.push(PackInstallationRecord {
                            pack_id,
                            source_url,
                            members: Vec::new(),
                        });
                    }
                }
            }
            Ok(records)
        })
    }

    /// Removes one pack installation together with its member relationships, returning whether
    /// a record existed.
    pub fn remove(&self, pack_id: &str) -> Result<bool, DatabaseError> {
        self.pool.with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            let deleted_members = transaction.execute(
                "DELETE FROM pack_installation_member WHERE pack_id = ?1",
                params![pack_id],
            )?;
            let deleted_pack = transaction.execute(
                "DELETE FROM pack_installation WHERE pack_id = ?1",
                params![pack_id],
            )?;
            transaction.commit()?;
            Ok(deleted_pack > 0 || deleted_members > 0)
        })
    }

    /// Removes one member relationship, returning whether a row existed.
    pub fn remove_member(&self, pack_id: &str, member_id: &str) -> Result<bool, DatabaseError> {
        self.pool.with_connection(|connection| {
            let deleted = connection.execute(
                "DELETE FROM pack_installation_member WHERE pack_id = ?1 AND member_id = ?2",
                params![pack_id, member_id],
            )?;
            Ok(deleted > 0)
        })
    }
}

/// Maps one member row onto its record parts.
fn map_member_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, PackMemberOwnership)> {
    let member_id: String = row.get(0)?;
    let version_at_install: String = row.get(1)?;
    let ownership: String = row.get(2)?;
    let ownership = PackMemberOwnership::parse(&ownership).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            format!("unknown pack member ownership {ownership:?}").into(),
        )
    })?;
    Ok((member_id, version_at_install, ownership))
}
