use std::fs;
use std::path::{Path, PathBuf};

use ora_domain::PluginId;
use ora_logging::ora_warn;
use ora_plugin_manifest::PluginManifest;

use crate::entry::{RegistryEntry, entry_id};
use crate::error::RegistryError;
use crate::source::RegistrySource;

/// The index schema version reported in every built index file.
///
/// It is bumped whenever a persisted field changes shape rather than merely appearing: a new
/// field with a serde default is readable from an older cache, but `logo` turning from SVG
/// source text into an object is a type mismatch that no default can absorb.
const INDEX_VERSION: &str = "2.0";

/// Holds one immutable registry index that lists every discoverable marketplace plugin.
///
/// The index is a lightweight derived artifact: callers read it instead of re-scanning and
/// parsing every `orax.toml`, so `updated_at` and the schema `version` are stable pointers
/// that help consumers detect staleness and schema changes.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RegistryIndex {
    updated_at: i64,
    version: String,
    plugins: Vec<RegistryEntry>,
}

impl RegistryIndex {
    /// Scans every configured source's registry directory for `orax.toml`, parses each valid
    /// manifest under that source's namespace, and merges the results into one deterministically
    /// ordered index built at the injected Unix `updated_at` instant.
    ///
    /// Malformed or unreadable manifests are skipped, logged as warnings, and reported through
    /// the returned [`RegistryBuild`] so a single bad file never blocks the whole build.
    ///
    /// Entries from different sources never collide: an entry's id is `<source namespace>/<identifier>`,
    /// so two repositories publishing the same `identifier` produce two ids and both stay listed.
    /// Deduplication therefore only collapses a repeated id *within* one source, where the first
    /// manifest in path order wins. Telling the two apart is the display layer's job, which is
    /// what [`RegistryEntry::source_url`] exists for.
    pub fn build_all(sources: &[&RegistrySource], updated_at: i64) -> RegistryBuild {
        let mut entries = Vec::new();
        let mut skipped = Vec::new();
        for source in sources {
            for path in orax_manifest_paths(&source.registry_dir()) {
                match parse_manifest(&path) {
                    Ok(manifest) => {
                        // The icon lives beside the manifest under one of the fixed candidate
                        // names; the same scan runs against an installed package root, so a
                        // listing and its install can never resolve to different icons.
                        let logo = path.parent().and_then(ora_plugin_asset::resolve_logo);
                        entries.push(RegistryEntry::from_manifest(
                            &manifest,
                            source.namespace(),
                            source.canonical_url(),
                            logo,
                        ));
                    }
                    Err(error) => {
                        ora_warn!(path = %path.display(), %error, "skipping invalid registry plugin manifest");
                        skipped.push(SkippedManifest {
                            path,
                            reason: error.to_string(),
                        });
                    }
                }
            }
        }
        entries.sort_by(|left, right| left.id().cmp(right.id()));
        entries.dedup_by(|left, right| left.id() == right.id());

        let index = Self {
            updated_at,
            version: INDEX_VERSION.to_owned(),
            plugins: entries,
        };
        RegistryBuild { index, skipped }
    }

    /// Resolves the full release manifest one source publishes for `id`.
    ///
    /// This is the install-time companion of [`Self::build_all`]: the cached index carries only
    /// the lightweight display fields, so consumers re-read the source `orax.toml` to obtain the
    /// release `url` and `sha256` needed to download and verify. A source whose namespace differs
    /// from the id's resolves nothing, because that id names a plugin this source does not
    /// publish. Unparseable manifests are skipped exactly as during the index build, so one bad
    /// file never blocks a lookup.
    pub fn resolve_manifest(
        source: &RegistrySource,
        id: &PluginId,
    ) -> Result<Option<PluginManifest>, RegistryError> {
        let Some(path) = Self::find_manifest_path(source, id)? else {
            return Ok(None);
        };
        Ok(Some(parse_manifest(&path)?))
    }

    /// Resolves the README text beside the manifest that matches `id` in `source`.
    ///
    /// This is the detail-page companion of [`Self::resolve_manifest`]: the cached index carries
    /// only display fields, so the UI reads the source README on demand. A listing without a
    /// README reads as `None`; an unreadable, non-UTF-8, or oversized document is reported.
    pub fn resolve_readme(
        source: &RegistrySource,
        id: &PluginId,
    ) -> Result<Option<String>, crate::readme::ReadmeReadError> {
        let Some(path) = Self::find_manifest_path(source, id)? else {
            return Ok(None);
        };
        crate::readme::read_beside_manifest(&path)
    }

    /// Resolves the entry directory `id` is published from in `source`.
    ///
    /// This is what the icon protocol resolves a plugin id against for a listing that is not
    /// installed: the entry directory is the only place a marketplace icon exists, and it is
    /// already the directory `resolve_readme` reads by id, so no new part of the checkout becomes
    /// reachable.
    pub fn resolve_entry_directory(
        source: &RegistrySource,
        id: &PluginId,
    ) -> Result<Option<PathBuf>, RegistryError> {
        Ok(Self::find_manifest_path(source, id)?
            .and_then(|path| path.parent().map(Path::to_path_buf)))
    }

    /// Locates the manifest in `source` whose identity under that source's namespace equals `id`.
    fn find_manifest_path(
        source: &RegistrySource,
        id: &PluginId,
    ) -> Result<Option<PathBuf>, RegistryError> {
        if source.namespace().as_str() != id.namespace() {
            return Ok(None);
        }
        for path in orax_manifest_paths(&source.registry_dir()) {
            let manifest = match parse_manifest(&path) {
                Ok(manifest) => manifest,
                Err(error) => {
                    ora_warn!(path = %path.display(), %error, "skipping invalid registry plugin manifest while resolving");
                    continue;
                }
            };
            if entry_id(&manifest, source.namespace()) == *id {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    /// Resolves the full release manifest for `id` across every configured source.
    ///
    /// An id carries the namespace of the source that published it, so this is not a
    /// first-source-wins search: only the source that owns that namespace can answer, and an
    /// install or update therefore always follows the entry's own source and proxy policy no
    /// matter how the source list is ordered or reordered afterwards.
    pub fn resolve_manifest_all(
        sources: &[&RegistrySource],
        id: &PluginId,
    ) -> Result<Option<PluginManifest>, RegistryError> {
        for source in sources {
            if let Some(manifest) = Self::resolve_manifest(source, id)? {
                return Ok(Some(manifest));
            }
        }
        Ok(None)
    }

    /// Loads an index from a previously written JSON file so consumers can read it without
    /// rescanning, refusing one written under a different schema version.
    ///
    /// The version check is what the field was always for. Without it a host upgraded past a
    /// shape change would try to deserialize a cache it cannot understand, and the failure would
    /// surface as a broken marketplace rather than as an index that simply has to be rebuilt.
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        let bytes = fs::read(path)?;
        let index: Self = serde_json::from_slice(&bytes)?;
        if index.version != INDEX_VERSION {
            return Err(RegistryError::UnsupportedIndexVersion {
                found: index.version,
                expected: INDEX_VERSION.to_owned(),
            });
        }
        Ok(index)
    }

    /// Returns whether `error` means the cached index cannot be read and must be rebuilt.
    ///
    /// A cache is a derived artifact, so "written by another schema", "corrupt" and "not there
    /// yet" are one situation with one remedy — sync again. Callers use this to fold all three
    /// into the same not-yet-synced response instead of turning two of them into an error the
    /// user cannot act on.
    pub fn is_unusable_cache(error: &RegistryError) -> bool {
        match error {
            RegistryError::Io(error) => error.kind() == std::io::ErrorKind::NotFound,
            RegistryError::Json(_) | RegistryError::UnsupportedIndexVersion { .. } => true,
            RegistryError::Git(_)
            | RegistryError::Manifest(_)
            | RegistryError::SourceUrl(_)
            | RegistryError::SourceBranch(_)
            | RegistryError::MissingCloneParent(_) => false,
        }
    }

    /// Atomically replaces `path` with this index's JSON serialization through a same-directory
    /// temporary file, so concurrent readers never observe a partially written index.
    pub fn write(&self, path: &Path) -> Result<(), RegistryError> {
        let bytes = serde_json::to_vec(self)?;
        ora_utils::atomic::write(path, &bytes)?;
        Ok(())
    }

    /// Returns the Unix timestamp (seconds) at which this index was built.
    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }

    /// Returns the schema version this index conforms to.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the listed plugins.
    pub fn plugins(&self) -> &[RegistryEntry] {
        &self.plugins
    }
}

/// Holds one built index together with every manifest that was skipped during the scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryBuild {
    index: RegistryIndex,
    skipped: Vec<SkippedManifest>,
}

impl RegistryBuild {
    /// Returns the completed, deterministically ordered index.
    pub fn index(&self) -> &RegistryIndex {
        &self.index
    }

    /// Returns the manifests that were skipped during the build, in path order.
    pub fn skipped(&self) -> &[SkippedManifest] {
        &self.skipped
    }
}

/// Describes one `orax.toml` that could not be parsed into the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedManifest {
    path: PathBuf,
    reason: String,
}

impl SkippedManifest {
    /// Returns the path of the manifest that was skipped.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the human-readable reason the manifest was skipped.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Collects every `orax.toml` beneath `root` in deterministic path order.
fn orax_manifest_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_orax_manifests(root, &mut paths);
    paths.sort();
    paths
}

/// Recursively accumulates `orax.toml` paths without following symlinks or reporting missing roots.
fn collect_orax_manifests(dir: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_orax_manifests(&path, paths);
        } else if path
            .file_name()
            .is_some_and(|name| name == std::ffi::OsStr::new("orax.toml"))
        {
            paths.push(path);
        }
    }
}

/// Reads and parses one manifest, mapping both I/O and semantic failures onto [`RegistryError`].
fn parse_manifest(path: &Path) -> Result<PluginManifest, RegistryError> {
    let source = fs::read_to_string(path)?;
    Ok(PluginManifest::parse(&source)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitlancer::BranchName;
    use ora_domain::PluginNamespace;
    use ora_plugin_asset::{LogoCandidate, LogoExtension, LogoRole, PluginLogoVariants};
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::TempDir;

    const UPDATED_AT: i64 = 1_776_244_428;
    const OFFICIAL_URL: &str = "https://github.com/ora-space/marketplace";
    const THIRD_PARTY_URL: &str = "https://github.com/acme/plugins";

    /// Binds one temp checkout to a source identity so index building has a namespace to use.
    fn source_at(checkout: &Path, url: &str, namespace: PluginNamespace) -> RegistrySource {
        RegistrySource::new(url, namespace, BranchName::new("main"), checkout)
    }

    /// Binds a temp checkout to the default marketplace identity.
    fn official_source(checkout: &Path) -> RegistrySource {
        source_at(checkout, OFFICIAL_URL, PluginNamespace::official())
    }

    /// Binds a temp checkout to a third-party source whose namespace is derived from its URL.
    fn third_party_source(checkout: &Path) -> RegistrySource {
        let canonical = ora_utils::url::canonical_repository_url(THIRD_PARTY_URL);
        source_at(
            checkout,
            THIRD_PARTY_URL,
            PluginNamespace::derive_from_canonical_url(&canonical),
        )
    }

    /// Builds a syntactically valid `orax.toml` string for a plugin identifier.
    fn valid_manifest(identifier: &str, description: &str) -> String {
        format!(
            "resolver = 1\n\
             identifier = \"{identifier}\"\n\
             kind = \"workbench\"\n\
             version = \"1.2.0\"\n\
             description = \"{description}\"\n\
             homepage = \"https://example.com\"\n\
             license = \"MIT\"\n\
             url = \"https://example.com/{identifier}.orax\"\n\
             sha256 = \"{}\"\n",
            "ab".repeat(32)
        )
    }

    /// Writes a manifest at a nesting level that mimics the two-tier marketplace layout.
    fn write_manifest(
        root: &Path,
        identifier: &str,
        source: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let path = root
            .join("registry")
            .join(&identifier[..1])
            .join(identifier)
            .join("orax.toml");
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("no parent"))?;
        fs::create_dir_all(parent)?;
        fs::write(&path, source)?;
        Ok(path)
    }

    /// Verifies entries are ordered by their `namespace/identifier` identifier regardless of scan
    /// order.
    #[test]
    fn builds_deterministically_ordered_index() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write_manifest(root.path(), "z", &valid_manifest("z", "Z plugin"))?;
        write_manifest(root.path(), "a", &valid_manifest("a", "A plugin"))?;
        let source = official_source(root.path());

        let build = RegistryIndex::build_all(&[&source], UPDATED_AT);

        let a_manifest = PluginManifest::parse(&valid_manifest("a", "A plugin"))?;
        let z_manifest = PluginManifest::parse(&valid_manifest("z", "Z plugin"))?;
        let expected_plugins = vec![
            RegistryEntry::from_manifest(
                &a_manifest,
                source.namespace(),
                source.canonical_url(),
                /*logo*/ None,
            ),
            RegistryEntry::from_manifest(
                &z_manifest,
                source.namespace(),
                source.canonical_url(),
                /*logo*/ None,
            ),
        ];

        assert_eq!(build.index().plugins().to_vec(), expected_plugins);
        assert_eq!(build.index().updated_at(), UPDATED_AT);
        assert_eq!(build.index().version(), INDEX_VERSION);
        assert_eq!(build.skipped().len(), 0);
        Ok(())
    }

    /// Verifies an entry's namespace and attribution come from the publishing source, and that a
    /// residual `namespace` key in the manifest cannot influence either.
    #[test]
    fn takes_identity_from_the_source_not_the_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let manifest = valid_manifest("weather", "Weather plugin")
            .replace("kind = ", "namespace = \"official\"\nkind = ");
        write_manifest(root.path(), "weather", &manifest)?;
        let source = third_party_source(root.path());

        let build = RegistryIndex::build_all(&[&source], UPDATED_AT);

        assert_eq!(build.skipped().len(), 0);
        assert_eq!(
            build
                .index()
                .plugins()
                .iter()
                .map(|entry| (
                    entry.id().canonical(),
                    entry.namespace().to_owned(),
                    entry.source_url().to_owned(),
                ))
                .collect::<Vec<_>>(),
            vec![(
                format!("{}/weather", source.namespace()),
                source.namespace().to_string(),
                "https://github.com/acme/plugins".to_string(),
            )],
        );
        Ok(())
    }

    /// Verifies `kind = "mcp"` is indexed rather than skipped as an unsupported kind.
    #[test]
    fn indexes_an_mcp_kind_marketplace_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let manifest = valid_manifest("ora-space.tavily", "Tavily MCP")
            .replace("kind = \"workbench\"", "kind = \"mcp\"");
        write_manifest(root.path(), "ora-space.tavily", &manifest)?;
        let source = official_source(root.path());

        let build = RegistryIndex::build_all(&[&source], UPDATED_AT);

        assert_eq!(build.skipped().len(), 0);
        assert_eq!(build.index().plugins().len(), 1);
        assert_eq!(build.index().plugins()[0].kind(), "mcp");
        assert_eq!(
            build.index().plugins()[0].id().canonical(),
            "official/ora-space.tavily"
        );
        Ok(())
    }

    /// Verifies install-time resolution re-reads a manifest from the source registry by its id.
    #[test]
    fn resolves_manifest_by_identifier() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write_manifest(
            root.path(),
            "weather",
            &valid_manifest("weather", "Weather plugin"),
        )?;
        let source = official_source(root.path());

        let manifest = RegistryIndex::resolve_manifest(
            &source,
            &PluginId::new("official", "weather").expect("plugin id"),
        )?
        .ok_or_else(|| std::io::Error::other("expected a resolved manifest"))?;

        assert_eq!(manifest.name().as_str(), "weather");

        let missing = RegistryIndex::resolve_manifest(
            &source,
            &PluginId::new("official", "absent").expect("plugin id"),
        )?;
        assert!(missing.is_none());
        Ok(())
    }

    /// Verifies an id resolves to an entry directory only in the source whose namespace owns it.
    ///
    /// This is what keeps the icon protocol's second root from becoming a way to address any
    /// directory in any checkout: the namespace is part of the id, so a source that does not
    /// publish that namespace answers nothing at all rather than searching its own tree.
    #[test]
    fn resolves_an_entry_directory_only_in_the_owning_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let manifest_path = write_manifest(
            root.path(),
            "weather",
            &valid_manifest("weather", "Weather plugin"),
        )?;
        let entry_dir = manifest_path
            .parent()
            .ok_or_else(|| std::io::Error::other("no parent"))?
            .to_path_buf();
        let official = official_source(root.path());
        let third_party = third_party_source(root.path());
        let id = PluginId::new("official", "weather").expect("plugin id");

        assert_eq!(
            (
                RegistryIndex::resolve_entry_directory(&official, &id)?,
                // The same checkout, read through a source publishing another namespace.
                RegistryIndex::resolve_entry_directory(&third_party, &id)?,
                RegistryIndex::resolve_entry_directory(
                    &official,
                    &PluginId::new("official", "absent").expect("plugin id"),
                )?,
            ),
            (Some(entry_dir), None, None)
        );
        Ok(())
    }

    /// Verifies detail-page resolution reads the README beside the matching manifest.
    #[test]
    fn resolves_readme_beside_a_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let manifest_path = write_manifest(
            root.path(),
            "weather",
            &valid_manifest("weather", "Weather plugin"),
        )?;
        let entry_dir = manifest_path
            .parent()
            .ok_or_else(|| std::io::Error::other("no parent"))?;
        fs::write(entry_dir.join("README.md"), "# Weather\n\nLive forecasts.")?;
        let source = official_source(root.path());

        let id = PluginId::new("official", "weather").expect("plugin id");
        assert_eq!(
            RegistryIndex::resolve_readme(&source, &id)?,
            Some("# Weather\n\nLive forecasts.".to_string())
        );

        // An identifier absent from the registry, or a listing without a README, reads as none.
        let absent = PluginId::new("official", "absent").expect("plugin id");
        assert_eq!(RegistryIndex::resolve_readme(&source, &absent)?, None);
        write_manifest(root.path(), "silent", &valid_manifest("silent", "No docs"))?;
        let silent = PluginId::new("official", "silent").expect("plugin id");
        assert_eq!(RegistryIndex::resolve_readme(&source, &silent)?, None);
        Ok(())
    }

    /// Verifies the icon beside a manifest is indexed as the composition it resolves to.
    #[test]
    fn indexes_the_logo_composition_beside_each_manifest() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = TempDir::new()?;
        let logo = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="8"/></svg>"#;
        let manifest_path = write_manifest(root.path(), "a", &valid_manifest("a", "A plugin"))?;
        let entry_dir = manifest_path
            .parent()
            .ok_or_else(|| std::io::Error::other("no parent"))?;
        fs::write(entry_dir.join("logo.svg"), logo)?;
        write_manifest(root.path(), "b", &valid_manifest("b", "B plugin"))?;
        let source = official_source(root.path());

        let build = RegistryIndex::build_all(&[&source], UPDATED_AT);

        assert_eq!(
            (
                build.index().plugins()[0].logo(),
                build.index().plugins()[1].logo(),
            ),
            (
                Some(PluginLogoVariants::Universal {
                    universal: LogoCandidate {
                        role: LogoRole::Universal,
                        extension: LogoExtension::Svg,
                    },
                }),
                None,
            )
        );
        Ok(())
    }

    /// Verifies an unsafe logo is dropped while its plugin still reaches the marketplace listing.
    #[test]
    fn indexes_a_plugin_whose_logo_is_unsafe() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let manifest_path = write_manifest(root.path(), "a", &valid_manifest("a", "A plugin"))?;
        let entry_dir = manifest_path
            .parent()
            .ok_or_else(|| std::io::Error::other("no parent"))?;
        fs::write(
            entry_dir.join("logo.svg"),
            "<svg><script>evil()</script></svg>",
        )?;
        let source = official_source(root.path());

        let build =
            ora_logging::with_trace_logging(|| RegistryIndex::build_all(&[&source], UPDATED_AT));

        assert_eq!(build.index().plugins().len(), 1);
        assert_eq!(build.index().plugins()[0].logo(), None);
        assert_eq!(build.skipped().len(), 0);
        Ok(())
    }

    /// Verifies a missing or empty marketplace registry directory builds a valid empty index.
    #[test]
    fn builds_an_empty_index_for_a_missing_registry_directory()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let source = official_source(&root.path().join("absent"));

        let build = RegistryIndex::build_all(&[&source], UPDATED_AT);

        assert_eq!(build.index().plugins().len(), 0);
        assert_eq!(build.skipped().len(), 0);
        Ok(())
    }

    /// Verifies a malformed manifest is skipped, logged, and reported without blocking the build.
    #[test]
    fn skips_invalid_manifest_and_reports_it() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write_manifest(root.path(), "good", &valid_manifest("good", "Good plugin"))?;
        let bad_path = write_manifest(root.path(), "bad", "this is not valid toml")?;
        let source = official_source(root.path());

        let build =
            ora_logging::with_trace_logging(|| RegistryIndex::build_all(&[&source], UPDATED_AT));

        assert_eq!(build.index().plugins().len(), 1);
        assert_eq!(build.index().plugins()[0].id().canonical(), "official/good");
        assert_eq!(build.skipped().len(), 1);
        assert_eq!(build.skipped()[0].path(), bad_path);
        assert!(!build.skipped()[0].reason().is_empty());
        Ok(())
    }

    /// Verifies a written index loads back into an equal in-memory value.
    #[test]
    fn load_round_trips_written_index() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write_manifest(root.path(), "a", &valid_manifest("a", "A plugin"))?;
        let source = official_source(root.path());

        let index = RegistryIndex::build_all(&[&source], UPDATED_AT)
            .index()
            .clone();
        let target = root.path().join("cache").join("registry_index.json");
        let parent = target
            .parent()
            .ok_or_else(|| std::io::Error::other("no parent"))?;
        fs::create_dir_all(parent)?;
        index.write(&target)?;

        let loaded = RegistryIndex::load(&target)?;
        assert_eq!(loaded, index);
        Ok(())
    }

    /// Verifies the persisted schema uses `identifier` and still accepts the previous cache key.
    #[test]
    fn serializes_identifier_and_reads_legacy_name() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write_manifest(
            root.path(),
            "weather",
            &valid_manifest("weather", "Weather plugin"),
        )?;
        let source = official_source(root.path());

        let index = RegistryIndex::build_all(&[&source], UPDATED_AT)
            .index()
            .clone();
        let serialized = serde_json::to_value(&index)?;
        assert_eq!(serialized["plugins"][0]["identifier"], "weather");
        assert!(serialized["plugins"][0].get("name").is_none());

        let mut legacy = serialized;
        let plugin = legacy["plugins"][0]
            .as_object_mut()
            .ok_or_else(|| std::io::Error::other("expected a serialized plugin object"))?;
        let identifier = plugin
            .remove("identifier")
            .ok_or_else(|| std::io::Error::other("expected an identifier field"))?;
        plugin.insert("name".to_owned(), identifier);
        assert_eq!(serde_json::from_value::<RegistryIndex>(legacy)?, index);
        Ok(())
    }

    /// Verifies write replaces prior content and leaves no same-directory temporary files behind.
    #[test]
    fn write_overwrites_atomically_without_leftovers() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write_manifest(root.path(), "a", &valid_manifest("a", "A plugin"))?;
        let source = official_source(root.path());

        let target = root.path().join("registry_index.json");
        RegistryIndex::build_all(&[&source], UPDATED_AT)
            .index()
            .write(&target)?;
        let first = fs::read_to_string(&target)?;

        let second_index = RegistryIndex::build_all(&[&source], UPDATED_AT + 1)
            .index()
            .clone();
        second_index.write(&target)?;
        let second = fs::read_to_string(&target)?;

        assert_ne!(first, second);
        assert_eq!(
            serde_json::from_str::<RegistryIndex>(&second)?,
            second_index
        );
        let leftover_temps = fs::read_dir(root.path())?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
            .count();
        assert_eq!(leftover_temps, 0);
        Ok(())
    }

    /// Verifies a cache written under another schema version is refused rather than reinterpreted.
    ///
    /// The version field has always been written and never read. Reading it is what makes a shape
    /// change survivable: `logo` turning from source text into an object is a type mismatch, so
    /// deserializing the old file under the new schema would fail somewhere less specific — or,
    /// for a field that happened to stay compatible, succeed and hand back a half-understood
    /// index.
    #[test]
    fn refuses_a_cache_written_under_another_schema_version()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write_manifest(root.path(), "a", &valid_manifest("a", "A plugin"))?;
        let source = official_source(root.path());
        let target = root.path().join("registry_index.json");
        RegistryIndex::build_all(&[&source], UPDATED_AT)
            .index()
            .write(&target)?;

        let mut stored: serde_json::Value = serde_json::from_str(&fs::read_to_string(&target)?)?;
        stored["version"] = serde_json::Value::String("1.0".to_owned());
        fs::write(&target, serde_json::to_string(&stored)?)?;

        assert!(matches!(
            RegistryIndex::load(&target),
            Err(RegistryError::UnsupportedIndexVersion { .. })
        ));
        Ok(())
    }

    /// Verifies the three ways a cache can be unreadable are one situation with one remedy.
    ///
    /// Absent, corrupt and written-by-another-schema all mean the derived file has to be rebuilt
    /// by a sync; a genuine failure such as an unparseable manifest does not, and must stay
    /// distinguishable so it can still surface as an error.
    #[test]
    fn classifies_every_unreadable_cache_as_one_that_must_be_rebuilt()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let corrupt = root.path().join("corrupt.json");
        fs::write(&corrupt, "{ not json")?;
        let stale = root.path().join("stale.json");
        fs::write(&stale, r#"{"updated_at":1,"version":"0.9","plugins":[]}"#)?;

        let missing = RegistryIndex::load(&root.path().join("absent.json"))
            .expect_err("a missing cache does not load");
        let corrupt = RegistryIndex::load(&corrupt).expect_err("a corrupt cache does not load");
        let stale = RegistryIndex::load(&stale).expect_err("a stale cache does not load");
        let unrelated = RegistryError::MissingCloneParent(root.path().to_path_buf());

        assert_eq!(
            [
                RegistryIndex::is_unusable_cache(&missing),
                RegistryIndex::is_unusable_cache(&corrupt),
                RegistryIndex::is_unusable_cache(&stale),
                RegistryIndex::is_unusable_cache(&unrelated),
            ],
            [true, true, true, false]
        );
        Ok(())
    }

    /// Verifies an index rewritten in the current shape reads back normally afterwards.
    #[test]
    fn a_rebuilt_index_reads_back_normally() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let manifest_path = write_manifest(root.path(), "a", &valid_manifest("a", "A plugin"))?;
        let entry_dir = manifest_path
            .parent()
            .ok_or_else(|| std::io::Error::other("no parent"))?;
        fs::write(
            entry_dir.join("logo.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="8"/></svg>"#,
        )?;
        let source = official_source(root.path());
        let target = root.path().join("registry_index.json");

        // A cache from before the shape change, then the rewrite one sync performs.
        fs::write(&target, r#"{"updated_at":1,"version":"1.0","plugins":[]}"#)?;
        assert!(RegistryIndex::load(&target).is_err());
        let rebuilt = RegistryIndex::build_all(&[&source], UPDATED_AT)
            .index()
            .clone();
        rebuilt.write(&target)?;

        assert_eq!(RegistryIndex::load(&target)?, rebuilt);
        Ok(())
    }

    /// Verifies loading a missing file surfaces an error instead of an empty index.
    #[test]
    fn load_missing_file_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;

        assert!(RegistryIndex::load(&root.path().join("missing.json")).is_err());
        Ok(())
    }

    /// Verifies two sources publishing the same `identifier` both stay listed, each under its own
    /// namespace and carrying its own attribution.
    ///
    /// This is the shadowing case the source-derived namespace exists to remove: when every entry
    /// shared one namespace, whichever source came first silently replaced the other's listing,
    /// and a third-party source ordered ahead of the default one could stand in for a first-party
    /// plugin without the user seeing which repository the card came from.
    #[test]
    fn lists_same_identifier_from_two_sources_without_shadowing()
    -> Result<(), Box<dyn std::error::Error>> {
        let first = TempDir::new()?;
        let second = TempDir::new()?;
        write_manifest(
            first.path(),
            "shared",
            &valid_manifest("shared", "Shared from the third party"),
        )?;
        write_manifest(
            first.path(),
            "a",
            &valid_manifest("a", "A from third party"),
        )?;
        write_manifest(
            second.path(),
            "shared",
            &valid_manifest("shared", "Shared from official"),
        )?;
        // The third-party source is deliberately ordered ahead of the default one.
        let third_party = third_party_source(first.path());
        let official = official_source(second.path());

        let build = RegistryIndex::build_all(&[&third_party, &official], UPDATED_AT);

        assert_eq!(
            build
                .index()
                .plugins()
                .iter()
                .map(|entry| (
                    entry.id().canonical(),
                    entry.description().to_owned(),
                    entry.source_url().to_owned(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "official/shared".to_string(),
                    "Shared from official".to_string(),
                    official.canonical_url().to_string(),
                ),
                (
                    format!("{}/a", third_party.namespace()),
                    "A from third party".to_string(),
                    third_party.canonical_url().to_string(),
                ),
                (
                    format!("{}/shared", third_party.namespace()),
                    "Shared from the third party".to_string(),
                    third_party.canonical_url().to_string(),
                ),
            ],
        );
        assert_eq!(build.skipped().len(), 0);
        Ok(())
    }

    /// Verifies install-time resolution follows the id's own source rather than the source order.
    ///
    /// An update must re-read the manifest published by the repository the plugin was installed
    /// from. Because the id names that source's namespace, reordering the source list — or adding
    /// a source ahead of it that publishes the same `identifier` — cannot redirect the lookup.
    #[test]
    fn resolves_manifest_from_the_source_that_owns_the_id() -> Result<(), Box<dyn std::error::Error>>
    {
        let first = TempDir::new()?;
        let second = TempDir::new()?;
        write_manifest(
            first.path(),
            "weather",
            &valid_manifest("weather", "Weather from the third party"),
        )?;
        write_manifest(
            second.path(),
            "weather",
            &valid_manifest("weather", "Weather from official"),
        )?;
        let third_party = third_party_source(first.path());
        let official = official_source(second.path());
        let ordered = [&third_party, &official];

        let official_id = PluginId::new("official", "weather").expect("plugin id");
        let third_party_id =
            PluginId::new(third_party.namespace().clone(), "weather").expect("plugin id");

        assert_eq!(
            (
                RegistryIndex::resolve_manifest_all(&ordered, &official_id)?
                    .map(|manifest| manifest.description().to_owned()),
                RegistryIndex::resolve_manifest_all(&ordered, &third_party_id)?
                    .map(|manifest| manifest.description().to_owned()),
                RegistryIndex::resolve_manifest_all(
                    &ordered,
                    &PluginId::new("official", "absent").expect("plugin id"),
                )?
                .is_none(),
            ),
            (
                Some("Weather from official".to_string()),
                Some("Weather from the third party".to_string()),
                true,
            ),
        );
        Ok(())
    }
}
