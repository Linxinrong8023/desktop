//! The custom URI scheme plugin assets are served under, and the extension-to-content-type
//! table every response answers with.

/// Custom URI scheme under which the host serves plugin-owned assets.
pub const ASSET_SCHEME: &str = "ora-plugin";

/// How the webview runtime spells a custom scheme URL on this platform.
///
/// Tauri serves custom protocols as `<scheme>://localhost/...` except on Windows and Android,
/// where they become `http://<scheme>.localhost/...`; the policy and CSP must use the spelling
/// the page actually sees. Because the spelling differs per platform, an asset URL is never a
/// stable string to assert on or persist — code that must recognise one goes through this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetUrlForm {
    CustomScheme,
    HttpLocalhost,
}

impl AssetUrlForm {
    /// The form used by the running host.
    pub const CURRENT: Self = if cfg!(any(windows, target_os = "android")) {
        Self::HttpLocalhost
    } else {
        Self::CustomScheme
    };

    /// Returns the `<scheme>://localhost/` origin every asset of this host is served from.
    ///
    /// Both asset branches build their URLs on top of this one spelling, so a platform whose
    /// runtime rewrites the scheme can never end up serving one branch and not the other.
    pub fn origin(self) -> String {
        match self {
            Self::CustomScheme => format!("{ASSET_SCHEME}://localhost/"),
            Self::HttpLocalhost => format!("http://{ASSET_SCHEME}.localhost/"),
        }
    }
}

/// Maps a file extension to the content type the handler may serve; anything else is served as
/// `application/octet-stream` and never sniffed.
///
/// The list is the build-capability contract of workbench pages: a template that emits another
/// extension must extend this table (and the documentation) rather than relying on sniffing. It
/// is also the only place an icon response looks up its content type, which is what keeps that
/// response from having to re-sniff the bytes it is about to send.
pub fn asset_content_type(extension: &str) -> &'static str {
    match extension {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        "map" if cfg!(debug_assertions) => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetUrlForm, asset_content_type};
    use pretty_assertions::assert_eq;

    /// The origin differs per platform, which is why no code may match a single spelling.
    #[test]
    fn spells_the_origin_per_platform() {
        assert_eq!(
            (
                AssetUrlForm::CustomScheme.origin(),
                AssetUrlForm::HttpLocalhost.origin(),
            ),
            (
                "ora-plugin://localhost/".to_owned(),
                "http://ora-plugin.localhost/".to_owned(),
            )
        );
    }

    /// Known extensions get their type; unknown ones are octet-stream, never sniffed.
    ///
    /// The five icon extensions are part of the same closed table the workbench uses, which is
    /// what lets an icon response answer from its URL alone without re-reading the bytes.
    #[test]
    fn content_types_are_a_closed_table() {
        assert_eq!(
            [
                asset_content_type("html"),
                asset_content_type("mjs"),
                asset_content_type("wasm"),
                asset_content_type("svg"),
                asset_content_type("png"),
                asset_content_type("webp"),
                asset_content_type("jpg"),
                asset_content_type("jpeg"),
                asset_content_type("gif"),
                asset_content_type("exe"),
            ],
            [
                "text/html; charset=utf-8",
                "text/javascript; charset=utf-8",
                "application/wasm",
                "image/svg+xml",
                "image/png",
                "image/webp",
                "image/jpeg",
                "image/jpeg",
                "application/octet-stream",
                "application/octet-stream",
            ]
        );
    }
}
