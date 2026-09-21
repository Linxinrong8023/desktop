//! Restart reconciliation for recorded pack installations: identification only.
//!
//! The ownership journal records what a pack install did; the installed tree is what exists
//! now. Reconciliation compares the two and classifies every recorded member, so a restart can
//! *see* drift (member deleted externally, member upgraded independently) without repairing
//! anything. Ownership always comes from the journal — the current marketplace manifest is never
//! consulted, because the pack listing may have moved on and cannot explain an old relationship
//! — and the recorded `version_at_install` is treated as an immutable historical fact.

use super::PluginApi;
use crate::error::BackendError;
use ora_plugin_manager::PluginManager;

/// How one recorded pack member compares against the currently installed tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum PackMemberReconciliation {
    /// The member is installed at exactly the version recorded when the pack installed it.
    ExpectedAndPresent {
        member_id: String,
        version_at_install: String,
        ownership: ora_db::PackMemberOwnership,
    },
    /// The member is installed, but at a different version than the journal recorded. The
    /// recorded version and ownership stay untouched: the journal is history, not live state.
    VersionChanged {
        member_id: String,
        version_at_install: String,
        current_version: String,
        ownership: ora_db::PackMemberOwnership,
    },
    /// The member is not installed at all — deleted externally or lost to a failed operation.
    Missing {
        member_id: String,
        version_at_install: String,
        ownership: ora_db::PackMemberOwnership,
    },
}

/// The reconciliation of one recorded pack installation against the installed tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PackReconciliation {
    pack_id: String,
    members: Vec<PackMemberReconciliation>,
}

impl PackReconciliation {
    /// Returns the recorded members for the plan builder, which shares this module.
    pub(super) fn members(&self) -> &[PackMemberReconciliation] {
        &self.members
    }
}

// The pack-id accessor exists for in-crate tests; the plan builder reads the field directly
// because it shares this struct's module.
#[cfg(test)]
impl PackReconciliation {
    /// Returns the canonical pack id this reconciliation belongs to.
    pub(super) fn pack_id(&self) -> &str {
        &self.pack_id
    }
}

impl PluginApi {
    /// Reconciles one recorded pack installation against the currently installed tree.
    ///
    /// Returns `None` when the pack has no ownership record — reconciliation never consults the
    /// marketplace to invent a relationship that no install created.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn reconcile_pack_installation(
        &self,
        pack_id: &str,
    ) -> Result<Option<PackReconciliation>, BackendError> {
        let Some(record) = self.pack_installation(pack_id)? else {
            return Ok(None);
        };
        // One discovery snapshot answers every member: the installed tree is the only live side
        // of the comparison, and the journal is never rewritten by this operation.
        let installed = PluginManager::discover(&self.home_directory);
        let members = record
            .members
            .iter()
            .map(|member| {
                let current = installed
                    .installed_plugins()
                    .iter()
                    .find(|plugin| plugin.id.canonical() == member.member_id);
                match current {
                    None => PackMemberReconciliation::Missing {
                        member_id: member.member_id.clone(),
                        version_at_install: member.version_at_install.clone(),
                        ownership: member.ownership,
                    },
                    Some(installed_member)
                        if installed_member.version.to_string() == member.version_at_install =>
                    {
                        PackMemberReconciliation::ExpectedAndPresent {
                            member_id: member.member_id.clone(),
                            version_at_install: member.version_at_install.clone(),
                            ownership: member.ownership,
                        }
                    }
                    Some(installed_member) => PackMemberReconciliation::VersionChanged {
                        member_id: member.member_id.clone(),
                        version_at_install: member.version_at_install.clone(),
                        current_version: installed_member.version.to_string(),
                        ownership: member.ownership,
                    },
                }
            })
            .collect::<Vec<_>>();
        Ok(Some(PackReconciliation {
            pack_id: record.pack_id,
            members,
        }))
    }
}
