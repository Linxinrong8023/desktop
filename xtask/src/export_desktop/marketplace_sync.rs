//! Generates the existing native marketplace subscription without introducing an event catalog.

#[path = "../../../apps/desktop/src-tauri/bindings/marketplace_sync.rs"]
mod binding;

use crate::export_contracts::{GENERATED_FILE_HEADER, write_generated_file};
use ora_contracts::MarketplaceAutoSyncEvent;
use std::path::Path;

/// Derives the native listener and wire samples from the same declarations used by the emitter.
pub(super) fn export(staging: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let web = staging.join("apps").join("desktop").join("web");
    let route = serde_json::to_string(binding::AUTO_SYNC_EVENT)?;
    write_generated_file(
        &web.join("marketplace-sync.generated.ts"),
        &format!(
            "{GENERATED_FILE_HEADER}import {{ listen }} from \"@tauri-apps/api/event\";\nimport type {{ MarketplaceAutoSyncEvent }} from \"@ora/contracts\";\n\n/** Subscribes to the Desktop-owned automatic marketplace refresh route. */\nexport function onMarketplaceAutoSyncChanged(listener: (event: MarketplaceAutoSyncEvent) => void): Promise<() => void> {{\n  return listen<MarketplaceAutoSyncEvent>({route}, (event) => listener(event.payload));\n}}\n"
        ),
    )?;
    let payloads = serde_json::to_string(&[
        MarketplaceAutoSyncEvent::Started,
        MarketplaceAutoSyncEvent::Finished,
    ])?;
    write_generated_file(
        &web.join("marketplace-sync-fixtures.generated.ts"),
        &format!(
            "{GENERATED_FILE_HEADER}import type {{ MarketplaceAutoSyncEvent }} from \"@ora/contracts\";\n\n// Actual serde output, checked against the generated DTO and consumed by adapter tests.\nexport const marketplaceAutoSyncWire = {{ route: {route}, payloads: {payloads} }} as const satisfies {{ route: string; payloads: readonly MarketplaceAutoSyncEvent[] }};\n"
        ),
    )
}
