//! Assets the host serves on a plugin's behalf over the `ora-plugin` protocol.
//!
//! The crate owns two things that must not drift apart: how an asset URL is spelled on each
//! platform together with the content type its extension maps to, and the icon convention that
//! decides which file on disk a plugin's logo request resolves to. Both the marketplace index
//! build and installed-package discovery consume the icon half, so the candidate filenames and
//! the variant decision table exist exactly once and the same directory therefore renders the
//! same icon before and after an install.

mod logo;
mod scheme;

pub use logo::{
    LOGO_EXTENSION_PRIORITY, LOGO_ROLES, LOGO_URL_PREFIX, LogoAssetRequest, LogoAssetRoot,
    LogoCandidate, LogoExtension, LogoRole, MAX_LOGO_BYTES, MAX_LOGO_PIXELS, PluginLogoVariants,
    candidate_file_name, logo_asset_url, plugin_logo, resolve_logo,
};
pub use scheme::{ASSET_SCHEME, AssetUrlForm, asset_content_type};
