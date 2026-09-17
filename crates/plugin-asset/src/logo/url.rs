//! The asset URLs an icon is served under, and the request shape the protocol handler parses.
//!
//! An icon request has no surface instance to be authorised against: the main window issues it
//! for an arbitrary plugin, and a marketplace listing that is not installed has no instance at
//! all. The URL therefore names four things and nothing else — which root, plugin id, theme
//! role, file extension — so that the handler can rebuild the candidate filename from closed
//! sets rather than trusting any path the caller supplies.

use super::candidate::{LogoExtension, LogoRole};
use super::variants::{LogoCandidate, PluginLogoVariants};
use crate::scheme::AssetUrlForm;
use ora_contracts::PluginLogo;
use ora_domain::PluginId;
use url::{ParseError, Url};

/// First path segment of every icon request, which separates the branch from instance assets.
///
/// A surface instance is addressed by a number, so a reserved word can never collide with one
/// and the handler can pick the authorisation chain from the first segment alone.
pub const LOGO_URL_PREFIX: &str = "logo";

/// Which of the two directories an icon request reads from.
///
/// The root has to be part of the address because the two directories resolve independently and
/// can disagree: a plugin whose marketplace entry already publishes `logo.light.svg` while the
/// installed package still ships only `logo.svg` yields two different compositions for one id.
/// Whoever resolved a directory mints the URL naming that directory, and the handler reads back
/// the same one — so a request can never be answered from a directory nobody resolved, and a
/// disagreement between the two shows up as each surface drawing its own icon rather than as a
/// URL that resolves to a filename no one produced.
///
/// It is a closed two-value set, so it adds no more surface to path construction than the theme
/// role does; the handler still builds the filename itself and never sees a caller-supplied path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoAssetRoot {
    /// The installed package directory, which is the icon of the code that actually runs.
    Installed,
    /// The entry directory of the marketplace source that publishes the id.
    Registry,
}

/// Both roots, in the order a parser matches them.
const LOGO_ASSET_ROOTS: [LogoAssetRoot; 2] = [LogoAssetRoot::Installed, LogoAssetRoot::Registry];

impl LogoAssetRoot {
    /// Returns the spelling used in asset URLs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Registry => "registry",
        }
    }

    /// Parses one URL segment into a root, refusing anything outside the closed set.
    pub fn parse(value: &str) -> Option<Self> {
        LOGO_ASSET_ROOTS
            .into_iter()
            .find(|root| root.as_str() == value)
    }
}

/// One icon request as addressed by its URL path.
///
/// Holding a parsed [`PluginId`] and the three closed-set values is the whole point of the type:
/// once a request exists, every part of the filename it will resolve to has already been checked
/// against a fixed set, and no caller-supplied string survives into path construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogoAssetRequest {
    pub root: LogoAssetRoot,
    pub plugin_id: PluginId,
    pub role: LogoRole,
    pub extension: LogoExtension,
}

impl LogoAssetRequest {
    /// Parses `/logo/<root>/<namespace>/<name>/<role>.<extension>` into its closed-set parts.
    ///
    /// `path` must already be percent-decoded, exactly once, by the caller. Decoding here as
    /// well would let an encoded separator survive the split and reappear afterwards; leaving it
    /// undone would instead make a legitimately encoded segment fail the id grammar. Neither
    /// mistake can produce a traversal, because every segment is then checked against a closed
    /// set: the id grammar admits only lowercase letters, digits, `-` and `.` and refuses `.`
    /// and `..`, while the root, role and extension must each equal one of the fixed spellings.
    pub fn parse(path: &str) -> Option<Self> {
        let mut segments = path.trim_start_matches('/').split('/');
        if segments.next()? != LOGO_URL_PREFIX {
            return None;
        }
        let root = segments.next()?;
        let namespace = segments.next()?;
        let name = segments.next()?;
        let file_name = segments.next()?;
        if segments.next().is_some() {
            return None;
        }
        let (role, extension) = file_name.split_once('.')?;
        Some(Self {
            root: LogoAssetRoot::parse(root)?,
            plugin_id: PluginId::new(namespace, name).ok()?,
            role: LogoRole::parse(role)?,
            extension: LogoExtension::parse(extension)?,
        })
    }
}

/// Returns the URL one plugin's icon candidate is served from on this host.
pub fn logo_asset_url(
    form: AssetUrlForm,
    root: LogoAssetRoot,
    plugin_id: &PluginId,
    role: LogoRole,
    extension: LogoExtension,
) -> Result<Url, ParseError> {
    Url::parse(&format!(
        "{origin}{LOGO_URL_PREFIX}/{root}/{namespace}/{name}/{role}.{extension}",
        origin = form.origin(),
        root = root.as_str(),
        namespace = plugin_id.namespace(),
        name = plugin_id.name(),
        role = role.as_str(),
        extension = extension.as_str(),
    ))
}

/// Turns a resolved icon composition into the contract shape the frontend renders.
///
/// The contract carries URLs rather than content: the bytes are fetched only for the icons that
/// are actually drawn, they belong to the webview's image cache instead of a JS string that
/// lives as long as the list, and the format stops mattering to the contract entirely, which is
/// what makes bitmap icons possible at all.
///
/// `root` must be the directory `variants` was resolved from: it is what the response will read
/// back, and naming the other one would address a candidate this composition never saw.
pub fn plugin_logo(
    form: AssetUrlForm,
    root: LogoAssetRoot,
    plugin_id: &PluginId,
    variants: &PluginLogoVariants,
) -> Option<PluginLogo> {
    let url = |candidate: LogoCandidate| {
        logo_asset_url(form, root, plugin_id, candidate.role, candidate.extension)
            .ok()
            .map(String::from)
    };
    match *variants {
        PluginLogoVariants::Universal { universal } => Some(PluginLogo::Universal {
            url: url(universal)?,
        }),
        PluginLogoVariants::Themed { light, dark } => Some(PluginLogo::Themed {
            light: url(light)?,
            dark: url(dark)?,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::super::variants::PluginLogoVariants;
    use super::{
        LogoAssetRequest, LogoAssetRoot, LogoExtension, LogoRole, logo_asset_url, plugin_logo,
    };
    use crate::scheme::AssetUrlForm;
    use ora_contracts::PluginLogo;
    use ora_domain::PluginId;
    use pretty_assertions::assert_eq;

    /// The plugin every URL in this module is built for.
    fn plugin() -> PluginId {
        PluginId::new("official", "acme.hub").expect("plugin id")
    }

    /// A URL names the root, the plugin id, the role and the extension, in both spellings.
    #[test]
    fn spells_the_icon_url_in_both_platform_forms() {
        let url = |form| {
            logo_asset_url(
                form,
                LogoAssetRoot::Installed,
                &plugin(),
                LogoRole::Dark,
                LogoExtension::Png,
            )
            .expect("icon url")
            .to_string()
        };

        assert_eq!(
            (
                url(AssetUrlForm::CustomScheme),
                url(AssetUrlForm::HttpLocalhost)
            ),
            (
                "ora-plugin://localhost/logo/installed/official/acme.hub/dark.png".to_owned(),
                "http://ora-plugin.localhost/logo/installed/official/acme.hub/dark.png".to_owned(),
            )
        );
    }

    /// The two roots are distinct addresses for one id, which is the point of naming them.
    ///
    /// A plugin whose marketplace entry already publishes a theme pair while its installed
    /// package still ships a single file resolves to two different compositions; without the
    /// root in the URL, one surface's URL would be answered from the other's directory and the
    /// icon would resolve to a filename nobody produced.
    #[test]
    fn addresses_the_two_roots_separately() {
        let url = |root| {
            logo_asset_url(
                AssetUrlForm::CustomScheme,
                root,
                &plugin(),
                LogoRole::Light,
                LogoExtension::Svg,
            )
            .expect("icon url")
            .to_string()
        };

        assert_eq!(
            (url(LogoAssetRoot::Installed), url(LogoAssetRoot::Registry)),
            (
                "ora-plugin://localhost/logo/installed/official/acme.hub/light.svg".to_owned(),
                "ora-plugin://localhost/logo/registry/official/acme.hub/light.svg".to_owned(),
            )
        );
    }

    /// A well-formed request parses back into the closed-set parts the URL carries.
    #[test]
    fn parses_a_well_formed_icon_request() {
        assert_eq!(
            (
                LogoAssetRequest::parse("/logo/registry/official/acme.hub/light.jpeg"),
                LogoAssetRequest::parse("/logo/installed/official/acme.hub/light.jpeg"),
            ),
            (
                Some(LogoAssetRequest {
                    root: LogoAssetRoot::Registry,
                    plugin_id: plugin(),
                    role: LogoRole::Light,
                    extension: LogoExtension::Jpeg,
                }),
                Some(LogoAssetRequest {
                    root: LogoAssetRoot::Installed,
                    plugin_id: plugin(),
                    role: LogoRole::Light,
                    extension: LogoExtension::Jpeg,
                }),
            )
        );
    }

    /// Every URL a request round-trips from is one the parser accepts again.
    #[test]
    fn round_trips_every_url_it_builds() {
        let url = logo_asset_url(
            AssetUrlForm::CustomScheme,
            LogoAssetRoot::Registry,
            &plugin(),
            LogoRole::Universal,
            LogoExtension::Webp,
        )
        .expect("icon url");

        assert_eq!(
            LogoAssetRequest::parse(url.path()),
            Some(LogoAssetRequest {
                root: LogoAssetRoot::Registry,
                plugin_id: plugin(),
                role: LogoRole::Universal,
                extension: LogoExtension::Webp,
            })
        );
    }

    /// Anything outside the closed sets is refused before a filename is ever built.
    ///
    /// The refusals matter more than the acceptances here: this branch serves an installed
    /// package root and an untrusted checkout, so a request that could smuggle a path segment,
    /// a traversal, or an unlisted extension past the parser would be an arbitrary file read.
    #[test]
    fn refuses_everything_outside_the_closed_sets() {
        let refused = [
            // A root that is not one of the two fixed spellings, including the shape the URL
            // had before the root was part of it.
            "/logo/cache/official/acme.hub/dark.svg",
            "/logo/Installed/official/acme.hub/dark.svg",
            "/logo/official/acme.hub/dark.svg",
            // A role or extension that is not one of the fixed spellings.
            "/logo/registry/official/acme.hub/themed.svg",
            "/logo/registry/official/acme.hub/dark.gif",
            "/logo/registry/official/acme.hub/dark.exe",
            // A path segment where the filename belongs, and a deeper path below it.
            "/logo/registry/official/acme.hub/nested/dark.svg",
            "/logo/registry/official/acme.hub/dark.svg/extra",
            // Traversal spelled directly, and spelled through the id segments.
            "/logo/registry/official/../../secret/dark.svg",
            "/logo/registry/../acme.hub/dark.svg",
            "/logo/registry/official/./dark.svg",
            // An id segment outside the id grammar, uppercase and separators included.
            "/logo/registry/Official/acme.hub/dark.svg",
            "/logo/registry/official/acme hub/dark.svg",
            "/logo/registry//acme.hub/dark.svg",
            // A request that is not an icon request at all.
            "/7/index.html",
            "/logo/registry/official/acme.hub",
            "/logo/registry",
            "/logo",
            "",
        ];

        assert_eq!(
            refused.map(|path| LogoAssetRequest::parse(path).is_some()),
            [false; 19]
        );
    }

    /// A percent-encoded traversal does not survive, because it is never decoded a second time.
    ///
    /// The caller decodes exactly once before parsing; these paths arrive still encoded, and the
    /// id grammar refuses the `%` outright instead of letting a decode turn it into a separator.
    #[test]
    fn refuses_percent_encoded_traversal_without_decoding_it() {
        assert_eq!(
            (
                LogoAssetRequest::parse("/logo/registry/official/%2e%2e%2f%2e%2e/dark.svg"),
                LogoAssetRequest::parse("/logo/registry/official/acme.hub/dark%2e%2e%2fsvg"),
                LogoAssetRequest::parse("/logo/registry/%2e%2e/acme.hub/dark.svg"),
                LogoAssetRequest::parse("/logo/%2e%2e/official/acme.hub/dark.svg"),
            ),
            (None, None, None, None)
        );
    }

    /// Each composition maps onto the contract shape the frontend branches on.
    #[test]
    fn maps_both_compositions_onto_the_contract() {
        let universal = PluginLogoVariants::from_roles(None, None, Some(LogoExtension::Svg))
            .expect("a universal candidate resolves");
        let themed = PluginLogoVariants::from_roles(
            Some(LogoExtension::Svg),
            None,
            Some(LogoExtension::Png),
        )
        .expect("a themed pair resolves");

        let logo = |variants| {
            plugin_logo(
                AssetUrlForm::CustomScheme,
                LogoAssetRoot::Registry,
                &plugin(),
                variants,
            )
        };

        assert_eq!(
            (logo(&universal), logo(&themed)),
            (
                Some(PluginLogo::Universal {
                    url: "ora-plugin://localhost/logo/registry/official/acme.hub/universal.svg"
                        .to_owned(),
                }),
                // The dark half is backed by `logo.png`, so it addresses the universal role.
                Some(PluginLogo::Themed {
                    light: "ora-plugin://localhost/logo/registry/official/acme.hub/light.svg"
                        .to_owned(),
                    dark: "ora-plugin://localhost/logo/registry/official/acme.hub/universal.png"
                        .to_owned(),
                }),
            )
        );
    }
}
