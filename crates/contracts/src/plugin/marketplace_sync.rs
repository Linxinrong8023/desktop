//! Status of an automatic marketplace refresh, independent of the host event transport.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Holds back manual synchronization while the host rebuilds the cached marketplace listing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export_to = "marketplace-sync.ts")]
pub enum MarketplaceAutoSyncEvent {
    Started,
    Finished,
}

/// Exports the payload while leaving event routing and grants with Desktop.
pub(super) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    MarketplaceAutoSyncEvent::export(config)
}
