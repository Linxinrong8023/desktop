//! The closed sets an icon candidate is addressed by: three theme roles and five extensions.

use ora_utils::image::ImageFormat;
use serde::{Deserialize, Serialize};

/// The stem every candidate filename is built from.
const LOGO_STEM: &str = "logo";

/// The theme role one icon file plays.
///
/// `Universal` is the role a file carries when its name states no theme at all (`logo.svg`); it
/// is not a fallback marker but a role in its own right, because a plugin that ships only that
/// file is asking for the same pixels under both themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogoRole {
    Universal,
    Light,
    Dark,
}

/// Every theme role, in the order the resolver examines them.
pub const LOGO_ROLES: [LogoRole; 3] = [LogoRole::Light, LogoRole::Dark, LogoRole::Universal];

impl LogoRole {
    /// Returns the spelling used in candidate filenames and in asset URLs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Universal => "universal",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Parses one URL segment into a role, refusing anything outside the closed set.
    pub fn parse(value: &str) -> Option<Self> {
        LOGO_ROLES.into_iter().find(|role| role.as_str() == value)
    }
}

/// The filename extension one icon file carries.
///
/// `Jpg` and `Jpeg` are two extensions of a single format and are kept apart on purpose: the
/// resolved variant has to name the file that is actually on disk, and a value that only
/// recorded the format could no longer say which of the two spellings to read back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogoExtension {
    Svg,
    Png,
    Webp,
    Jpg,
    Jpeg,
}

/// Every extension in the fixed order a role's candidates are tried in.
///
/// SVG wins over every bitmap because it stays sharp at both the list and the detail-page size.
/// PNG and WebP precede JPEG because they carry an alpha channel and can therefore sit on any
/// background. The order within each of those two groups only exists so that an author who
/// shipped both files gets a defined result instead of one that depends on directory order.
pub const LOGO_EXTENSION_PRIORITY: [LogoExtension; 5] = [
    LogoExtension::Svg,
    LogoExtension::Png,
    LogoExtension::Webp,
    LogoExtension::Jpg,
    LogoExtension::Jpeg,
];

impl LogoExtension {
    /// Returns the spelling used in candidate filenames and in asset URLs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Jpg => "jpg",
            Self::Jpeg => "jpeg",
        }
    }

    /// Parses one URL segment into an extension, refusing anything outside the closed set.
    pub fn parse(value: &str) -> Option<Self> {
        LOGO_EXTENSION_PRIORITY
            .into_iter()
            .find(|extension| extension.as_str() == value)
    }

    /// Returns the raster format this extension promises, or `None` for the vector extension.
    ///
    /// The promise is what a candidate's bytes are checked against: an extension states which
    /// format the file claims to be, and a file whose magic bytes say otherwise is a packaging
    /// mistake rather than a file to be served under a corrected content type.
    pub(crate) fn promised_raster_format(self) -> Option<ImageFormat> {
        match self {
            Self::Svg => None,
            Self::Png => Some(ImageFormat::Png),
            Self::Webp => Some(ImageFormat::Webp),
            Self::Jpg | Self::Jpeg => Some(ImageFormat::Jpeg),
        }
    }
}

/// Returns the one filename a role and extension address, such as `logo.dark.png`.
///
/// The host builds every candidate name itself out of two closed sets, so no part of the name
/// ever originates in author-supplied or request-supplied text.
pub fn candidate_file_name(role: LogoRole, extension: LogoExtension) -> String {
    match role {
        LogoRole::Universal => format!("{LOGO_STEM}.{}", extension.as_str()),
        LogoRole::Light | LogoRole::Dark => {
            format!("{LOGO_STEM}.{}.{}", role.as_str(), extension.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LOGO_EXTENSION_PRIORITY, LOGO_ROLES, LogoExtension, LogoRole, candidate_file_name,
    };
    use pretty_assertions::assert_eq;

    /// The fifteen candidate names are exactly three roles crossed with five extensions.
    #[test]
    fn spells_all_fifteen_candidate_file_names() {
        let names: Vec<String> = LOGO_ROLES
            .into_iter()
            .flat_map(|role| {
                LOGO_EXTENSION_PRIORITY
                    .into_iter()
                    .map(move |extension| candidate_file_name(role, extension))
            })
            .collect();

        assert_eq!(
            names,
            vec![
                "logo.light.svg",
                "logo.light.png",
                "logo.light.webp",
                "logo.light.jpg",
                "logo.light.jpeg",
                "logo.dark.svg",
                "logo.dark.png",
                "logo.dark.webp",
                "logo.dark.jpg",
                "logo.dark.jpeg",
                "logo.svg",
                "logo.png",
                "logo.webp",
                "logo.jpg",
                "logo.jpeg",
            ]
        );
    }

    /// Roles and extensions parse only from their own closed sets.
    #[test]
    fn parses_only_closed_set_spellings() {
        assert_eq!(
            (
                LogoRole::parse("dark"),
                LogoRole::parse("Dark"),
                LogoRole::parse("themed"),
                LogoRole::parse(""),
                LogoExtension::parse("jpeg"),
                LogoExtension::parse("gif"),
                LogoExtension::parse("SVG"),
            ),
            (
                Some(LogoRole::Dark),
                None,
                None,
                None,
                Some(LogoExtension::Jpeg),
                None,
                None,
            )
        );
    }

    /// The two JPEG extensions stay distinct values even though they name one format.
    #[test]
    fn keeps_the_two_jpeg_extensions_apart() {
        assert_eq!(
            (
                LogoExtension::Jpg == LogoExtension::Jpeg,
                candidate_file_name(LogoRole::Universal, LogoExtension::Jpg),
                candidate_file_name(LogoRole::Universal, LogoExtension::Jpeg),
                LogoExtension::Jpg.promised_raster_format(),
                LogoExtension::Jpeg.promised_raster_format(),
            ),
            (
                false,
                "logo.jpg".to_owned(),
                "logo.jpeg".to_owned(),
                Some(ora_utils::image::ImageFormat::Jpeg),
                Some(ora_utils::image::ImageFormat::Jpeg),
            )
        );
    }
}
