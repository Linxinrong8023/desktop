//! The icon branch of the `ora-plugin://` protocol: serves one plugin's brand mark to the main
//! window, for any plugin the host knows — installed or only listed in a marketplace checkout.
//!
//! It shares the protocol shell with workbench assets and nothing else. A workbench request is
//! authorised by resolving the caller's label to a live surface instance, and an icon request has
//! no instance to resolve: it is issued by the main window on behalf of an arbitrary plugin, and
//! a marketplace listing that is not installed never had an instance at all. So the two branches
//! are separate chains, and this one is deliberately the narrower of the two — one caller label,
//! two possible roots, and a filename the host builds itself out of closed sets.

use crate::surface::MAIN_WINDOW_LABEL;
use crate::surface::workbench_assets::AssetOutcome;
use ora_backend::Plugins;
use ora_plugin_asset::{LogoAssetRequest, asset_content_type, candidate_file_name};
use ora_utils::path::{CanonicalPathRoot, PortableRelativePath};

/// Resolves one icon request issued by the webview `label`.
///
/// Refusals are indistinguishable to the caller, as on the workbench branch; the reason only
/// reaches the log. The chain is: the caller must be the main window, the URL must parse into a
/// root, plugin id, theme role and extension that all come from closed sets, that root must hold
/// that plugin, and the candidate filename the host builds from the role and extension must
/// resolve inside it.
pub fn resolve_logo_asset(plugins: &Plugins, label: &str, request_path: &str) -> AssetOutcome {
    // The only caller with a legitimate reason to ask for an arbitrary plugin's icon is the
    // trusted shell; a plugin's own workbench or webview has no business reaching this branch.
    if label != MAIN_WINDOW_LABEL {
        return AssetOutcome::NotFound("icons are served to the main window only");
    }
    // Decoded exactly once, before parsing. Doing it again afterwards would let an encoded
    // separator reappear inside a segment that has already been checked.
    let Ok(decoded) = urlencoding::decode(request_path) else {
        return AssetOutcome::NotFound("path is not valid UTF-8");
    };
    let Some(request) = LogoAssetRequest::parse(&decoded) else {
        return AssetOutcome::NotFound("path is not a root, plugin id, role and extension");
    };
    // The URL names the root the composition was resolved from, and this reads back that same
    // one. Falling through to the other root would hide a disagreement between them behind an
    // icon drawn from a directory nobody resolved.
    let Some(directory) = plugins.logo_directory(request.root, &request.plugin_id) else {
        return AssetOutcome::NotFound("the named root holds no such plugin");
    };
    // The filename never comes from the URL: it is rebuilt from two closed sets, so the only
    // thing the request contributes to the path is a plugin id that passed the id grammar.
    let file_name = candidate_file_name(request.role, request.extension);
    let Ok(relative) = PortableRelativePath::parse(&file_name) else {
        return AssetOutcome::NotFound("candidate name is not a safe relative path");
    };
    let Ok(root) = CanonicalPathRoot::new(&directory) else {
        return AssetOutcome::NotFound("plugin icon root is unavailable");
    };
    let Ok(resolved) = root.resolve_existing(&relative) else {
        return AssetOutcome::NotFound("icon does not resolve inside the plugin root");
    };
    if !resolved.is_file() {
        return AssetOutcome::NotFound("icon path is not a regular file");
    }
    match std::fs::read(&resolved) {
        // The content type comes from the extension alone. It can, because the extension was
        // checked against the file's magic bytes when the icon was resolved, so the type always
        // describes the bytes without the response ever sniffing them or consulting the index.
        Ok(body) => AssetOutcome::Serve {
            content_type: asset_content_type(request.extension.as_str()),
            body,
            csp_base: None,
        },
        Err(_) => AssetOutcome::NotFound("icon could not be read"),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_logo_asset;
    use crate::surface::MAIN_WINDOW_LABEL;
    use crate::surface::workbench_assets::AssetOutcome;
    use ora_backend::{Backend, BackendPaths, Plugins};
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    const PLUGIN: &str = "acme.hub";
    const SAFE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="8"/></svg>"#;

    /// Builds a PNG whose `IHDR` declares a 64 by 64 canvas.
    fn png() -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&64u32.to_be_bytes());
        bytes.extend_from_slice(&64u32.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    /// Writes one installed agent package carrying a mixed-format theme pair.
    fn write_installed_package(home: &Path) -> PathBuf {
        let package_root = home
            .join("plugins")
            .join("installed")
            .join("official")
            .join(PLUGIN)
            .join("1.0.0");
        fs::create_dir_all(&package_root).expect("create the package directory");
        fs::write(
            package_root.join("orax.toml"),
            format!(
                "resolver = 1\n\
                 identifier = \"{PLUGIN}\"\n\
                 kind = \"agent\"\n\
                 version = \"1.0.0\"\n\
                 description = \"Example plugin\"\n"
            ),
        )
        .expect("write the package manifest");
        fs::write(package_root.join("main.js"), "export default {};").expect("write entrypoint");
        fs::write(package_root.join("logo.light.svg"), SAFE_SVG).expect("write the light icon");
        fs::write(package_root.join("logo.dark.png"), png()).expect("write the dark icon");
        // A package file that is not an icon candidate, used to show the URL cannot name one.
        fs::write(package_root.join("secret.txt"), "private").expect("write a non-icon file");
        package_root
    }

    /// Opens a backend over a temporary home that already holds the fixture package.
    fn backend_with_installed_plugin(temporary: &TempDir) -> (Backend, Plugins) {
        ora_logging::initialize_test_clock();
        let home = temporary.path().join("home");
        write_installed_package(&home);
        let backend = Backend::open(BackendPaths {
            app_data_directory: temporary.path().join("data"),
            home_directory: home,
            deno_path: PathBuf::from("deno"),
            relative_path_base: temporary.path().to_path_buf(),
            timezone: "Asia/Shanghai".parse().expect("local timezone"),
        })
        .expect("open backend");
        let plugins = backend.plugins();
        (backend, plugins)
    }

    /// Reduces one outcome to what a caller can observe: the content type, or the refusal.
    fn served(outcome: &AssetOutcome) -> Result<(&'static str, usize), &'static str> {
        match outcome {
            AssetOutcome::Serve {
                content_type, body, ..
            } => Ok((content_type, body.len())),
            AssetOutcome::NotFound(reason) => Err(reason),
        }
    }

    /// Each half of a theme pair is served with the content type its own extension maps to.
    ///
    /// The type comes from the extension alone, which is sound because the extension was already
    /// checked against the file's magic bytes when the icon was resolved; the response therefore
    /// sniffs nothing and reads no index.
    #[tokio::test]
    async fn serves_each_half_of_a_pair_with_the_type_its_extension_maps_to() {
        let temporary = tempfile::tempdir().expect("temporary backend root");
        let (_backend, plugins) = backend_with_installed_plugin(&temporary);

        let light = resolve_logo_asset(
            &plugins,
            MAIN_WINDOW_LABEL,
            &format!("/logo/installed/official/{PLUGIN}/light.svg"),
        );
        let dark = resolve_logo_asset(
            &plugins,
            MAIN_WINDOW_LABEL,
            &format!("/logo/installed/official/{PLUGIN}/dark.png"),
        );

        assert_eq!(
            (served(&light), served(&dark)),
            (
                Ok(("image/svg+xml", SAFE_SVG.len())),
                Ok(("image/png", png().len())),
            )
        );
    }

    /// Only the main window may ask for an arbitrary plugin's icon.
    ///
    /// A plugin's own workbench or webview has no legitimate reason to reach this branch, and it
    /// serves an installed package root and an untrusted checkout, so the caller check is the
    /// first thing in the chain rather than a later refinement of it.
    #[tokio::test]
    async fn refuses_every_caller_but_the_main_window() {
        let temporary = tempfile::tempdir().expect("temporary backend root");
        let (_backend, plugins) = backend_with_installed_plugin(&temporary);
        let path = format!("/logo/installed/official/{PLUGIN}/light.svg");

        let refusals = ["surface-7", "", "Main", "main "]
            .map(|label| served(&resolve_logo_asset(&plugins, label, &path)));

        assert_eq!(
            refusals,
            [Err("icons are served to the main window only"); 4]
        );
    }

    /// Everything outside the three closed sets is refused before any path is built.
    #[tokio::test]
    async fn refuses_requests_outside_the_closed_sets() {
        let temporary = tempfile::tempdir().expect("temporary backend root");
        let (_backend, plugins) = backend_with_installed_plugin(&temporary);

        let refusals = [
            // A root outside the two fixed spellings, and the shape the URL had before the
            // root was part of it.
            format!("/logo/cache/official/{PLUGIN}/light.svg"),
            format!("/logo/official/{PLUGIN}/light.svg"),
            // A theme role and an extension that are not among the fixed spellings.
            format!("/logo/installed/official/{PLUGIN}/themed.svg"),
            format!("/logo/installed/official/{PLUGIN}/dark.gif"),
            // A path segment or a traversal in place of the candidate name.
            format!("/logo/installed/official/{PLUGIN}/nested/dark.svg"),
            format!("/logo/installed/official/{PLUGIN}/../secret.txt"),
            "/logo/installed/official/../../secret/dark.svg".to_owned(),
            // The same traversal percent-encoded: it is decoded exactly once, before parsing,
            // so the decoded separators face the id grammar rather than slipping past it.
            "/logo/installed/official/%2e%2e%2f%2e%2e/dark.svg".to_owned(),
            format!("/logo/installed/official/{PLUGIN}/%64ark.svg%2f%2e%2e"),
            // An id segment outside the id grammar.
            format!("/logo/installed/Official/{PLUGIN}/dark.svg"),
        ]
        .map(|path| served(&resolve_logo_asset(&plugins, MAIN_WINDOW_LABEL, &path)));

        assert_eq!(
            refusals,
            [Err("path is not a root, plugin id, role and extension"); 10]
        );
    }

    /// A non-icon file in the package cannot be reached, because the URL never names a file.
    ///
    /// The candidate filename is rebuilt from the role and the extension, so the only thing a
    /// request contributes to the resolved path is a plugin id that passed the id grammar.
    #[tokio::test]
    async fn cannot_reach_a_package_file_that_is_not_an_icon_candidate() {
        let temporary = tempfile::tempdir().expect("temporary backend root");
        let (_backend, plugins) = backend_with_installed_plugin(&temporary);

        let outcome = served(&resolve_logo_asset(
            &plugins,
            MAIN_WINDOW_LABEL,
            &format!("/logo/installed/official/{PLUGIN}/secret.txt"),
        ));

        assert_eq!(
            outcome,
            Err("path is not a root, plugin id, role and extension")
        );
    }

    /// A candidate the plugin does not ship, and an id no root owns, are both plain refusals.
    #[tokio::test]
    async fn refuses_an_absent_candidate_and_an_unknown_plugin() {
        let temporary = tempfile::tempdir().expect("temporary backend root");
        let (_backend, plugins) = backend_with_installed_plugin(&temporary);

        let absent = served(&resolve_logo_asset(
            &plugins,
            MAIN_WINDOW_LABEL,
            &format!("/logo/installed/official/{PLUGIN}/universal.webp"),
        ));
        // No installed package and no marketplace checkout owns this id, so there is no root to
        // resolve against — not a different root, and not the requesting plugin's own.
        let unknown = served(&resolve_logo_asset(
            &plugins,
            MAIN_WINDOW_LABEL,
            "/logo/installed/official/absent.plugin/universal.svg",
        ));

        assert_eq!(
            (absent, unknown),
            (
                Err("icon does not resolve inside the plugin root"),
                Err("the named root holds no such plugin"),
            )
        );
    }

    /// The root named by the URL is the root that answers, with no fallback to the other one.
    ///
    /// This is the case the two roots exist to keep apart: the marketplace entry publishes a
    /// theme pair while the installed package still ships a single `logo.svg`, so one id has two
    /// different compositions at once. Were the handler to pick a root by precedence instead of
    /// reading back the one the URL names, the marketplace card's `light.svg` request would be
    /// answered from the installed package — which has no `logo.light.svg` — and the card would
    /// draw a broken image even though both directories hold a perfectly good icon.
    #[tokio::test]
    async fn answers_from_the_root_the_url_names_rather_than_by_precedence() {
        let temporary = tempfile::tempdir().expect("temporary backend root");
        let (_backend, plugins) = backend_with_installed_plugin(&temporary);
        // The installed package holds `logo.light.svg`; the fixture ships no registry checkout,
        // so the registry root holds nothing for this id at all.
        let installed = served(&resolve_logo_asset(
            &plugins,
            MAIN_WINDOW_LABEL,
            &format!("/logo/installed/official/{PLUGIN}/light.svg"),
        ));
        let registry = served(&resolve_logo_asset(
            &plugins,
            MAIN_WINDOW_LABEL,
            &format!("/logo/registry/official/{PLUGIN}/light.svg"),
        ));

        assert_eq!(
            (installed, registry),
            (
                Ok(("image/svg+xml", SAFE_SVG.len())),
                Err("the named root holds no such plugin"),
            )
        );
    }
}
