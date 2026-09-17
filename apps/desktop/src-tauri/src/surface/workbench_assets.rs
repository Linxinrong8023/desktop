//! The `ora-plugin://` protocol: serves a workbench page's package-shipped files to the one
//! webview instance that owns them. Authorization is the caller label resolved through the
//! registry, never the URL: the URL names only the instance, and the file root comes from that
//! instance's registry record.
//!
//! The scheme carries a second, separately authorised branch for plugin icons, in
//! `logo_assets.rs`. The two are told apart by the first path segment before either
//! authorisation chain runs: an instance is a number, so the icon prefix can never name one.

use crate::surface::gateway::SurfacePluginGateway;
use crate::surface::logo_assets::resolve_logo_asset;
use crate::surface::service::SurfaceService;
use ora_logging::{ora_debug, ora_info};
use ora_plugin_asset::{ASSET_SCHEME, AssetUrlForm, LOGO_URL_PREFIX, asset_content_type};
use ora_surface::{AssetRequest, SurfaceRegistry, SurfaceSource, asset_base, workbench_csp};
use ora_utils::path::{CanonicalPathRoot, PortableRelativePath};
use std::borrow::Cow;
use tauri::http::{Response, StatusCode, header};
use tauri::{Manager, Runtime};
use url::Url;

/// Outcome of resolving one asset request, kept separate from the HTTP shape for testing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetOutcome {
    /// The file may be served; `csp_base` is present only for a workbench entry document, and
    /// carries the instance base its policy pins every resource to.
    Serve {
        content_type: &'static str,
        body: Vec<u8>,
        csp_base: Option<Url>,
    },
    /// Every refusal is a 404: the page learns nothing about why.
    NotFound(&'static str),
}

/// Resolves one asset request for the webview `label`.
///
/// Refusals are deliberately indistinguishable to the page; the reason only goes to the log.
/// The chain is: label must be a live workbench instance, the URL must name that instance, the
/// file must resolve canonically below the instance's asset root, and its extension decides the
/// content type.
pub fn resolve_asset(registry: &SurfaceRegistry, label: &str, request_path: &str) -> AssetOutcome {
    let Some(record) = registry.resolve_label(label) else {
        return AssetOutcome::NotFound("label is not a live surface");
    };
    let SurfaceSource::Workbench(workbench) = &record.definition.source else {
        return AssetOutcome::NotFound("surface is not a workbench page");
    };
    let Some(request) = AssetRequest::parse(request_path) else {
        return AssetOutcome::NotFound("path lacks an instance segment");
    };
    if request.instance != record.instance {
        return AssetOutcome::NotFound("path names another instance");
    }
    let decoded = match urlencoding::decode(&request.path) {
        Ok(decoded) => decoded,
        Err(_) => return AssetOutcome::NotFound("path is not valid UTF-8"),
    };
    let relative = if decoded.is_empty() {
        Cow::Borrowed(workbench.page_entry.as_str())
    } else {
        decoded
    };
    let Ok(relative) = PortableRelativePath::parse(&relative) else {
        return AssetOutcome::NotFound("path is not a safe relative path");
    };
    if relative.is_root() {
        return AssetOutcome::NotFound("path names the asset root");
    }
    let Ok(root) = CanonicalPathRoot::new(&workbench.asset_root) else {
        return AssetOutcome::NotFound("asset root is unavailable");
    };
    let Ok(resolved) = root.resolve_existing(&relative) else {
        return AssetOutcome::NotFound("file does not resolve inside the asset root");
    };
    if !resolved.is_file() {
        return AssetOutcome::NotFound("path is not a regular file");
    }
    let extension = resolved
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match std::fs::read(&resolved) {
        Ok(body) => AssetOutcome::Serve {
            content_type: asset_content_type(&extension),
            body,
            // The instance was resolved from the caller's label above, so an entry document's
            // policy is pinned to that instance's own base and to no other.
            csp_base: (extension == "html")
                .then(|| asset_base(AssetUrlForm::CURRENT, record.instance).ok())
                .flatten(),
        },
        Err(_) => AssetOutcome::NotFound("file could not be read"),
    }
}

/// Turns an outcome into the HTTP response handed to the webview runtime.
pub fn asset_response(outcome: AssetOutcome, label: &str, request_path: &str) -> Response<Vec<u8>> {
    match outcome {
        AssetOutcome::Serve {
            content_type,
            body,
            csp_base,
        } => {
            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
            if let Some(base) = csp_base {
                // Only a workbench entry document carries a policy, and it is that instance's own
                // asset base; documents are never cached so a package update after reopen always
                // reloads the policy.
                builder = builder.header(header::CONTENT_SECURITY_POLICY, workbench_csp(&base));
                builder = builder.header(header::CACHE_CONTROL, "no-store");
            } else {
                // Asset URLs carry no content hash and the profile is persistent, so an
                // `immutable` grant would keep a package's old scripts alive after an update
                // while the uncached HTML already references the new ones. `no-cache` forces
                // revalidation on every load instead.
                builder = builder.header(header::CACHE_CONTROL, "no-cache");
            }
            builder.body(body).expect("static headers are valid")
        }
        AssetOutcome::NotFound(reason) => {
            ora_info!(
                message = "workbench asset refused",
                label,
                path = request_path,
                reason
            );
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(b"not found".to_vec())
                .expect("static headers are valid")
        }
    }
}

impl<G: SurfacePluginGateway, R: Runtime> SurfaceService<G, R> {
    /// Resolves one instance-scoped `ora-plugin://` request issued by the webview `label`.
    pub fn resolve_workbench_asset(&self, label: &str, path: &str) -> AssetOutcome {
        ora_debug!(message = "workbench asset requested", label, path);
        resolve_asset(&self.registry, label, path)
    }
}

/// Registers the `ora-plugin` scheme on the application builder.
///
/// The scheme carries two branches with two different authorisation chains, and the first path
/// segment picks between them before either chain runs: a surface instance is addressed by a
/// number, so the icon prefix can never be mistaken for one. The handler looks the state up per
/// request because it is managed only after `setup`; a request arriving before that (impossible
/// for a page the service creates) is refused like any other unknown label.
pub fn register_protocol(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.register_uri_scheme_protocol(ASSET_SCHEME, |context, request| {
        let label = context.webview_label();
        let path = request.uri().path();
        let Some(state) = context
            .app_handle()
            .try_state::<crate::state::DesktopState>()
        else {
            return asset_response(
                AssetOutcome::NotFound("host state is not ready"),
                label,
                path,
            );
        };
        let outcome = if path.trim_start_matches('/').starts_with(LOGO_URL_PREFIX) {
            resolve_logo_asset(&state.backend.plugins(), label, path)
        } else {
            state.surfaces.resolve_workbench_asset(label, path)
        };
        asset_response(outcome, label, path)
    })
}
