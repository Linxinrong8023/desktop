//! Covers how a marketplace rebuild composes per-source outcomes into one cached index.
//!
//! The cases exercise the composition directly rather than through Git: what each source did is
//! the input here, and the behavior worth pinning down is which listings survive and which sync
//! time the result may claim.

use super::compose_rebuild;
use gitlancer::BranchName;
use ora_domain::PluginNamespace;
use ora_plugin_registry::{RegistryIndex, RegistrySource, RegistrySourceFailure};
use pretty_assertions::assert_eq;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const REFRESHED_AT: i64 = 1_776_244_428;
const OFFICIAL_URL: &str = "https://github.com/ora-space/marketplace";
const THIRD_PARTY_URL: &str = "https://github.com/acme/plugins";

/// Binds one temp checkout to a source identity so index building has a namespace to use.
fn source_at(checkout: &Path, url: &str) -> RegistrySource {
    let namespace = if url == OFFICIAL_URL {
        PluginNamespace::official()
    } else {
        PluginNamespace::derive_from_canonical_url(&ora_utils::url::canonical_repository_url(url))
    };
    RegistrySource::new(url, namespace, BranchName::new("main"), checkout)
}

/// Writes one listing under `root` the way a marketplace repository publishes it.
fn write_listing(
    root: &Path,
    identifier: &str,
    description: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = root
        .join("registry")
        .join(&identifier[..1])
        .join(identifier)
        .join("orax.toml");
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(
        &path,
        format!(
            "resolver = 1\n\
             identifier = \"{identifier}\"\n\
             kind = \"workbench\"\n\
             version = \"1.0.0\"\n\
             description = \"{description}\"\n\
             url = \"https://example.com/{identifier}.orax\"\n\
             sha256 = \"{}\"\n",
            "ab".repeat(32)
        ),
    )?;
    Ok(())
}

/// Builds the cached index one successful refresh of both sources would have produced.
fn cached_index(official: &RegistrySource, third_party: &RegistrySource) -> RegistryIndex {
    RegistryIndex::build_all(&[official, third_party], REFRESHED_AT)
        .index()
        .clone()
}

/// Returns every listed `identifier/description` pair in index order.
fn listed(index: &RegistryIndex) -> Vec<(String, String)> {
    index
        .plugins()
        .iter()
        .map(|entry| (entry.id().canonical(), entry.description().to_owned()))
        .collect()
}

/// Verifies a source that failed keeps its previous listings while the source that answered
/// publishes its new ones.
#[test]
fn failed_sources_keep_their_previous_listings() -> Result<(), Box<dyn std::error::Error>> {
    let official_root = TempDir::new()?;
    let third_party_root = TempDir::new()?;
    write_listing(official_root.path(), "weather", "Weather plugin")?;
    write_listing(third_party_root.path(), "retired", "Retired plugin")?;
    let official = source_at(official_root.path(), OFFICIAL_URL);
    let third_party = source_at(third_party_root.path(), THIRD_PARTY_URL);
    let previous = cached_index(&official, &third_party);

    // The refreshed source republished its listing; the failed source's checkout lost it entirely.
    write_listing(
        official_root.path(),
        "weather",
        "Weather plugin, republished",
    )?;
    fs::remove_dir_all(third_party_root.path().join("registry"))?;
    let failure = RegistrySourceFailure::new(third_party.canonical_url(), "git fetch failed");

    let build = compose_rebuild(
        &[&official, &third_party],
        std::slice::from_ref(&failure),
        Some(&previous),
        REFRESHED_AT + 60,
    );

    assert_eq!(
        listed(build.index()),
        vec![
            (
                "official/weather".to_string(),
                "Weather plugin, republished".to_string(),
            ),
            (
                format!("{}/retired", third_party.namespace()),
                "Retired plugin".to_string(),
            ),
        ],
    );
    assert_eq!(build.index().updated_at(), REFRESHED_AT + 60);
    assert_eq!(
        build.index().source_failures(),
        std::slice::from_ref(&failure)
    );
    Ok(())
}

/// Verifies a rebuild that reached no source at all keeps the previous sync time instead of
/// claiming a refresh that never happened.
#[test]
fn a_rebuild_without_any_refreshed_source_keeps_the_previous_sync_time()
-> Result<(), Box<dyn std::error::Error>> {
    let official_root = TempDir::new()?;
    let third_party_root = TempDir::new()?;
    write_listing(official_root.path(), "weather", "Weather plugin")?;
    write_listing(third_party_root.path(), "retired", "Retired plugin")?;
    let official = source_at(official_root.path(), OFFICIAL_URL);
    let third_party = source_at(third_party_root.path(), THIRD_PARTY_URL);
    let previous = cached_index(&official, &third_party);

    let failures = [
        RegistrySourceFailure::new(official.canonical_url(), "git fetch failed"),
        RegistrySourceFailure::new(third_party.canonical_url(), "git fetch failed"),
    ];
    let build = compose_rebuild(
        &[&official, &third_party],
        &failures,
        Some(&previous),
        REFRESHED_AT + 60,
    );

    assert_eq!(listed(build.index()), listed(&previous));
    assert_eq!(build.index().updated_at(), REFRESHED_AT);
    assert_eq!(build.index().source_failures(), failures);
    Ok(())
}

/// Verifies a first refresh that reaches no source reports "never synced" rather than a sync time
/// for listings that do not exist.
#[test]
fn a_first_rebuild_without_any_refreshed_source_reports_never_synced()
-> Result<(), Box<dyn std::error::Error>> {
    let official_root = TempDir::new()?;
    let official = source_at(official_root.path(), OFFICIAL_URL);
    let failure = RegistrySourceFailure::new(official.canonical_url(), "git fetch failed");

    let build = compose_rebuild(
        &[&official],
        std::slice::from_ref(&failure),
        None,
        REFRESHED_AT,
    );

    assert_eq!(listed(build.index()), Vec::new());
    assert_eq!(build.index().updated_at(), 0);
    assert_eq!(
        build.index().source_failures(),
        std::slice::from_ref(&failure)
    );
    Ok(())
}

/// Verifies a rebuild with no enabled source advances the sync time: nothing failed, and the empty
/// catalog it writes is accurate as of that moment.
#[test]
fn a_rebuild_without_any_enabled_source_is_synced_now() -> Result<(), Box<dyn std::error::Error>> {
    let official_root = TempDir::new()?;
    let third_party_root = TempDir::new()?;
    write_listing(official_root.path(), "weather", "Weather plugin")?;
    let official = source_at(official_root.path(), OFFICIAL_URL);
    let third_party = source_at(third_party_root.path(), THIRD_PARTY_URL);
    let previous = cached_index(&official, &third_party);

    let build = compose_rebuild(&[], &[], Some(&previous), REFRESHED_AT + 60);

    assert_eq!(
        (
            listed(build.index()),
            build.index().updated_at(),
            build.index().source_failures().to_vec(),
        ),
        (Vec::new(), REFRESHED_AT + 60, Vec::new()),
    );
    Ok(())
}
