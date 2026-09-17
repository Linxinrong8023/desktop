//! Host-served workbench assets: URL shape, request parsing, and the CSP handed to workbench
//! documents. Everything here is pure so the desktop protocol handler only does I/O.
//!
//! The scheme itself and the extension-to-content-type table live in `ora-plugin-asset`, because
//! plugin icons are served over the same protocol and must answer with the same content types.

use crate::definition::WorkbenchDefinition;
use crate::ids::SurfaceInstanceId;
use ora_plugin_asset::AssetUrlForm;
use url::{ParseError, Url};

/// Returns `<scheme>://localhost/<instance>/`, the base every asset of one instance lives under.
///
/// The URL names the instance and nothing else: the protocol handler resolves the instance to
/// its registry record (and from there to the package root) and refuses a page that asks for
/// another instance's files. No plugin id or disk path ever appears in a URL.
pub fn asset_base(form: AssetUrlForm, instance: SurfaceInstanceId) -> Result<Url, ParseError> {
    Url::parse(&format!("{}{}/", form.origin(), instance.value()))
}

/// Returns the URL of the instance's entry document below its asset base.
pub fn entry_url(
    form: AssetUrlForm,
    instance: SurfaceInstanceId,
    definition: &WorkbenchDefinition,
) -> Result<Url, ParseError> {
    asset_base(form, instance)?.join(definition.page_entry.as_str())
}

/// One asset request as addressed by its URL path: which instance it claims to belong to and the
/// file below that instance's asset root (empty for the entry document).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRequest {
    pub instance: SurfaceInstanceId,
    pub path: String,
}

impl AssetRequest {
    /// Splits a request path of the form `/<instance>/<path>`.
    ///
    /// Percent-decoding is left to the caller's path parser: an instance number never needs
    /// it, and an encoded traversal in the file part must reach the portable-path check
    /// undisturbed so it is decoded exactly once, there.
    pub fn parse(path: &str) -> Option<Self> {
        let mut segments = path.trim_start_matches('/').splitn(2, '/');
        let instance = segments
            .next()
            .filter(|segment| !segment.is_empty())?
            .parse::<u64>()
            .ok()?;
        let path = segments.next().unwrap_or_default();
        Some(Self {
            instance: SurfaceInstanceId::new(instance),
            path: path.to_owned(),
        })
    }
}

/// Builds the Content-Security-Policy of a workbench document.
///
/// Inline script and style are forbidden (no nonce can reach a static page), every resource must
/// come from this instance's own asset base, and the page cannot talk to the network; the two
/// `connect-src` entries are the transports Tauri's IPC itself uses on platforms where it goes
/// through `fetch`, so the bridge keeps working without opening anything else. A plugin cannot
/// relax this policy.
pub fn workbench_csp(base: &Url) -> String {
    format!(
        "default-src 'none'; base-uri 'none'; object-src 'none'; frame-src 'none'; \
         frame-ancestors 'none'; form-action 'none'; worker-src 'none'; \
         connect-src ipc: http://ipc.localhost; \
         script-src {base}; style-src {base}; img-src {base} data:; font-src {base}"
    )
}

#[cfg(test)]
mod tests {
    use super::{AssetRequest, AssetUrlForm, asset_base, entry_url, workbench_csp};
    use crate::definition::WorkbenchDefinition;
    use crate::ids::SurfaceInstanceId;
    use ora_utils::path::PortableRelativePath;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    /// Both URL forms name the instance only, and the entry joins below the base.
    #[test]
    fn asset_urls_name_the_instance_only() {
        let definition = WorkbenchDefinition {
            asset_root: PathBuf::from("/plugins/hello/assets"),
            page_entry: PortableRelativePath::parse("index.html").expect("entry"),
            declared_methods: Vec::new(),
        };
        let instance = SurfaceInstanceId::new(7);
        assert_eq!(
            (
                asset_base(AssetUrlForm::CustomScheme, instance)
                    .expect("base")
                    .to_string(),
                asset_base(AssetUrlForm::HttpLocalhost, instance)
                    .expect("base")
                    .to_string(),
                entry_url(AssetUrlForm::CustomScheme, instance, &definition)
                    .expect("entry")
                    .to_string(),
            ),
            (
                "ora-plugin://localhost/7/".to_owned(),
                "http://ora-plugin.localhost/7/".to_owned(),
                "ora-plugin://localhost/7/index.html".to_owned(),
            )
        );
    }

    /// Requests split into instance and undecoded file path; a non-numeric instance is refused.
    #[test]
    fn parses_asset_requests() {
        assert_eq!(
            (
                AssetRequest::parse("/7/app/%2e%2e/secret.js"),
                AssetRequest::parse("/7/"),
                AssetRequest::parse("/7"),
                AssetRequest::parse("/seven/index.html"),
                AssetRequest::parse("/"),
            ),
            (
                Some(AssetRequest {
                    instance: SurfaceInstanceId::new(7),
                    path: "app/%2e%2e/secret.js".to_owned(),
                }),
                Some(AssetRequest {
                    instance: SurfaceInstanceId::new(7),
                    path: String::new(),
                }),
                Some(AssetRequest {
                    instance: SurfaceInstanceId::new(7),
                    path: String::new(),
                }),
                None,
                None,
            )
        );
    }

    /// The CSP pins every resource to the instance base and never allows inline or remote.
    #[test]
    fn csp_pins_resources_to_the_instance_base() {
        let base = asset_base(AssetUrlForm::CustomScheme, SurfaceInstanceId::new(7)).expect("base");
        let csp = workbench_csp(&base);
        assert_eq!(
            (
                csp.contains("default-src 'none'"),
                csp.contains("script-src ora-plugin://localhost/7/"),
                csp.contains("unsafe-inline"),
                csp.contains("unsafe-eval"),
                csp.contains("frame-ancestors 'none'"),
            ),
            (true, true, false, false, true)
        );
    }
}
