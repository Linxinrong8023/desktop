//! Pack installation status and uninstall plan presentation contracts (D4).
//!
//! These types project the backend ownership journal and reconciliation for the frontend's
//! installed-packs list and uninstall confirmation. The frontend never re-derives ownership:
//! every bucket it renders comes from these projections verbatim.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Exports every TypeScript binding declared in this module into the target directory.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    ListPackInstallationsRequest::export_all(config)?;
    ListPackInstallationsResponse::export_all(config)?;
    PackInstallationStatus::export_all(config)?;
    PackMemberStatus::export_all(config)?;
    PackMemberOwnership::export_all(config)?;
    PackMemberReconciliationState::export_all(config)?;
    PackUninstallPlanRequest::export_all(config)?;
    PackUninstallPlanResponse::export_all(config)?;
    PackUninstallPlan::export_all(config)?;
    PackUninstallPreservation::export_all(config)?;
    PackUninstallPreservationReason::export_all(config)?;
    Ok(())
}
/// Requests every recorded pack installation with its reconciled member states.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct ListPackInstallationsRequest {}

/// Returns every recorded pack installation, each member reconciled against the installed
/// tree. Read-only: reconciliation identifies drift, it never repairs the journal or the
/// installed tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct ListPackInstallationsResponse {
    pub packs: Vec<PackInstallationStatus>,
}

/// One recorded pack installation projected for the installed-packs presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct PackInstallationStatus {
    /// Canonical `namespace/identifier` of the pack listing.
    pub pack_id: String,
    /// Canonical URL of the marketplace source the pack was installed from.
    pub source_url: String,
    /// Every journaled member, reconciled against the installed tree.
    pub members: Vec<PackMemberStatus>,
}

/// One journaled pack member with its reconciled state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct PackMemberStatus {
    pub member_id: String,
    /// The member version at the moment the pack relationship was established.
    pub version_at_install: String,
    pub ownership: PackMemberOwnership,
    pub state: PackMemberReconciliationState,
}

/// Whether the pack installation created this member or found it already installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "plugin.ts")]
pub enum PackMemberOwnership {
    ManagedByPack,
    PreExisting,
}

/// The reconciled state of one journaled pack member against the installed tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export_to = "plugin.ts")]
pub enum PackMemberReconciliationState {
    /// Installed at exactly the recorded version.
    ExpectedAndPresent,
    /// Installed at a different version than recorded: the user took the member over.
    VersionChanged { current_version: String },
    /// Not installed: deleted externally or lost to a failed operation.
    Missing,
}

/// Requests the ownership-aware uninstall plan for one pack id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct PackUninstallPlanRequest {
    pub plugin_id: String,
}

/// Returns the ownership-aware uninstall plan, absent when the id has no journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct PackUninstallPlanResponse {
    pub plan: Option<PackUninstallPlan>,
}

/// The computed, not-yet-executed uninstall plan for one recorded pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct PackUninstallPlan {
    /// Members the pack created at their recorded versions: safe to remove.
    pub remove: Vec<String>,
    /// Members preserved with the structured reason the UI presents.
    pub preserve: Vec<PackUninstallPreservation>,
    /// Managed members that are already absent: released without filesystem work.
    pub already_missing: Vec<String>,
}

/// One preserved member and why the pack uninstall leaves it alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "plugin.ts")]
pub struct PackUninstallPreservation {
    pub member_id: String,
    pub reason: PackUninstallPreservationReason,
}

/// Why a pack uninstall preserves a member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "plugin.ts")]
pub enum PackUninstallPreservationReason {
    /// The pack never created this member; it was already installed when the pack named it.
    PreExisting,
    /// The pack created this member, but the user has independently changed it since.
    VersionChanged,
}
