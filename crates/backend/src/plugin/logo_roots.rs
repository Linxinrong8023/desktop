//! Resolves one plugin id, under one named root, to the directory its icon may be read from.
//!
//! An icon exists in two independent places: an installed package has a package root, and a
//! marketplace listing has an entry directory in the checkout of the source that publishes it.
//! The two resolve separately and can disagree, so the caller says which one it means rather
//! than this deciding for it — a precedence rule here would serve one surface's URL out of the
//! other surface's directory, and the mismatch would surface as a missing icon.

use super::PluginApi;
use ora_domain::PluginId;
use ora_logging::ora_warn;
use ora_plugin_asset::LogoAssetRoot;
use ora_plugin_registry::RegistryIndex;
use std::path::PathBuf;

impl PluginApi {
    /// Returns the directory `plugin_id`'s icon candidates live in under `root`, if it exists.
    ///
    /// There is no fallback between the two roots. A locally imported plugin has no marketplace
    /// entry at all, and an installed package may lag whatever its source publishes today, so a
    /// root that has nothing for this id means the request was for an icon that is not there —
    /// not that the other root should answer in its place.
    pub(crate) fn logo_directory(
        &self,
        root: LogoAssetRoot,
        plugin_id: &PluginId,
    ) -> Option<PathBuf> {
        match root {
            LogoAssetRoot::Installed => self
                .lifecycle
                .installed_plugin(plugin_id)
                .map(|installed| installed.package_root),
            LogoAssetRoot::Registry => self.registry_entry_directory(plugin_id),
        }
    }

    /// Finds the entry directory `plugin_id` is published from, across the configured sources.
    ///
    /// A source whose namespace differs from the id's cannot answer, so an id never resolves
    /// into another source's checkout. A source that cannot be read is skipped with a warning:
    /// an icon must never be the reason a marketplace listing fails.
    fn registry_entry_directory(&self, plugin_id: &PluginId) -> Option<PathBuf> {
        let sources = self
            .registry_sources()
            .inspect_err(|error| {
                ora_warn!(%error, "cannot resolve a plugin icon without marketplace sources");
            })
            .ok()?;
        for source in &sources {
            match RegistryIndex::resolve_entry_directory(source, plugin_id) {
                Ok(Some(entry_directory)) => return Some(entry_directory),
                Ok(None) => {}
                Err(error) => {
                    ora_warn!(%error, "skipping a marketplace source while resolving a plugin icon");
                }
            }
        }
        None
    }
}
