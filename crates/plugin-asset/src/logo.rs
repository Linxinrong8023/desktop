//! The plugin icon convention: fifteen candidate filenames, the rules that turn whichever of
//! them exist into theme variants, and the URLs those variants are served under.
//!
//! Nothing about an icon comes from the manifest. Its presence, its theme role and its format
//! are decided entirely by the filename, so the host never resolves an author-supplied path and
//! an author can add or drop an icon without a schema change.

mod candidate;
#[cfg(test)]
mod fixtures;
mod read;
mod resolve;
mod url;
mod variants;

pub use candidate::{
    LOGO_EXTENSION_PRIORITY, LOGO_ROLES, LogoExtension, LogoRole, candidate_file_name,
};
pub use read::{MAX_LOGO_BYTES, MAX_LOGO_PIXELS};
pub use resolve::resolve_logo;
pub use url::{LOGO_URL_PREFIX, LogoAssetRequest, LogoAssetRoot, logo_asset_url, plugin_logo};
pub use variants::{LogoCandidate, PluginLogoVariants};
