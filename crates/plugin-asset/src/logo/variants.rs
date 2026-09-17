//! What a directory's icon files resolve to: either one icon for both themes, or a pair.

use super::candidate::{LogoExtension, LogoRole};
use serde::{Deserialize, Serialize};

/// The one candidate file a resolved half of an icon composition is served from.
///
/// Both halves of the pair are addressed this way rather than by theme alone, because the role
/// that *labels* a half and the role whose file *backs* it are not always the same: a plugin
/// that ships `logo.dark.svg` beside `logo.svg` has its light half backed by the universal file.
/// The URL has to name the file that exists, so the role travels with the extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogoCandidate {
    pub role: LogoRole,
    pub extension: LogoExtension,
}

/// The icon composition one plugin publishes.
///
/// Modelling the two shapes as an enum rather than a struct of optional fields is what keeps a
/// half-built pair — a light icon with no dark one — unrepresentable, so the renderer branches
/// on the variant and never has to decide anything itself.
///
/// Nothing here records a filename or a path: a candidate is a role plus an extension, and the
/// host rebuilds `logo` + optional role + extension from those two closed sets whenever it needs
/// the name. Storing whole filenames would make values outside the fifteen candidates
/// expressible, and storing paths would put back exactly what the convention exists to keep out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum PluginLogoVariants {
    /// One file serves both themes.
    Universal { universal: LogoCandidate },
    /// A theme pair, whose halves may come from different files and different formats.
    Themed {
        light: LogoCandidate,
        dark: LogoCandidate,
    },
}

impl PluginLogoVariants {
    /// Folds the three roles that may exist on disk into the composition they describe.
    ///
    /// All eight combinations are defined, and none of them is an error: an author who ships one
    /// file too few gets a sensible icon rather than a diagnostic they cannot act on.
    ///
    /// A theme pair wins whenever both halves exist, which incidentally keeps an author who
    /// ships all three files readable by an older host that only knows `logo.svg`. A single
    /// themed file is completed by the universal one when there is one to complete it with —
    /// shipping a dark icon says the unmarked file was drawn for light backgrounds — and falls
    /// back to serving that lone file under both themes when there is not.
    pub fn from_roles(
        light: Option<LogoExtension>,
        dark: Option<LogoExtension>,
        universal: Option<LogoExtension>,
    ) -> Option<Self> {
        let candidate =
            |role: LogoRole, extension: LogoExtension| LogoCandidate { role, extension };
        let light = light.map(|extension| candidate(LogoRole::Light, extension));
        let dark = dark.map(|extension| candidate(LogoRole::Dark, extension));
        let universal = universal.map(|extension| candidate(LogoRole::Universal, extension));
        match (light, dark, universal) {
            (Some(light), Some(dark), _) => Some(Self::Themed { light, dark }),
            (Some(light), None, Some(universal)) => Some(Self::Themed {
                light,
                dark: universal,
            }),
            (None, Some(dark), Some(universal)) => Some(Self::Themed {
                light: universal,
                dark,
            }),
            (Some(only), None, None) | (None, Some(only), None) => {
                Some(Self::Universal { universal: only })
            }
            (None, None, Some(universal)) => Some(Self::Universal { universal }),
            (None, None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LogoCandidate, LogoExtension, LogoRole, PluginLogoVariants};
    use pretty_assertions::assert_eq;

    /// Names the candidate a role and extension address.
    fn candidate(role: LogoRole, extension: LogoExtension) -> LogoCandidate {
        LogoCandidate { role, extension }
    }

    /// All eight existence combinations produce a defined result, and none of them is an error.
    ///
    /// This is the whole decision table in one assertion: an author who ships any subset of the
    /// three roles gets an icon composition rather than a diagnostic, so no combination can ever
    /// remove a plugin from a listing or leave it with an undefined variant.
    #[test]
    fn every_role_combination_resolves_to_a_defined_variant() {
        let svg = Some(LogoExtension::Svg);
        let png = Some(LogoExtension::Png);
        let webp = Some(LogoExtension::Webp);
        let resolved = [
            PluginLogoVariants::from_roles(svg, png, webp),
            PluginLogoVariants::from_roles(svg, png, None),
            PluginLogoVariants::from_roles(svg, None, webp),
            PluginLogoVariants::from_roles(svg, None, None),
            PluginLogoVariants::from_roles(None, png, webp),
            PluginLogoVariants::from_roles(None, png, None),
            PluginLogoVariants::from_roles(None, None, webp),
            PluginLogoVariants::from_roles(None, None, None),
        ];

        assert_eq!(
            resolved,
            [
                // A theme pair wins whenever both halves exist; the universal file is ignored,
                // which is what leaves it in place for an older host that only reads `logo.svg`.
                Some(PluginLogoVariants::Themed {
                    light: candidate(LogoRole::Light, LogoExtension::Svg),
                    dark: candidate(LogoRole::Dark, LogoExtension::Png),
                }),
                Some(PluginLogoVariants::Themed {
                    light: candidate(LogoRole::Light, LogoExtension::Svg),
                    dark: candidate(LogoRole::Dark, LogoExtension::Png),
                }),
                // One themed file plus the universal one is a pair: shipping a themed icon says
                // the unmarked file was drawn for the other theme.
                Some(PluginLogoVariants::Themed {
                    light: candidate(LogoRole::Light, LogoExtension::Svg),
                    dark: candidate(LogoRole::Universal, LogoExtension::Webp),
                }),
                // With nothing to complete the pair with, the lone file serves both themes.
                Some(PluginLogoVariants::Universal {
                    universal: candidate(LogoRole::Light, LogoExtension::Svg),
                }),
                Some(PluginLogoVariants::Themed {
                    light: candidate(LogoRole::Universal, LogoExtension::Webp),
                    dark: candidate(LogoRole::Dark, LogoExtension::Png),
                }),
                Some(PluginLogoVariants::Universal {
                    universal: candidate(LogoRole::Dark, LogoExtension::Png),
                }),
                Some(PluginLogoVariants::Universal {
                    universal: candidate(LogoRole::Universal, LogoExtension::Webp),
                }),
                None,
            ]
        );
    }

    /// A half completed by the universal file keeps naming the file that exists on disk.
    ///
    /// Without this the dark half of `logo.light.svg` + `logo.svg` would address
    /// `logo.dark.svg`, a filename the author never shipped, and the icon would 404.
    #[test]
    fn a_completed_half_addresses_the_universal_file_it_came_from() {
        assert_eq!(
            PluginLogoVariants::from_roles(
                Some(LogoExtension::Svg),
                None,
                Some(LogoExtension::Svg)
            ),
            Some(PluginLogoVariants::Themed {
                light: candidate(LogoRole::Light, LogoExtension::Svg),
                dark: candidate(LogoRole::Universal, LogoExtension::Svg),
            })
        );
    }

    /// The two halves of a pair may use different extensions.
    #[test]
    fn allows_a_pair_whose_halves_use_different_extensions() {
        assert_eq!(
            PluginLogoVariants::from_roles(
                Some(LogoExtension::Svg),
                Some(LogoExtension::Jpeg),
                None
            ),
            Some(PluginLogoVariants::Themed {
                light: candidate(LogoRole::Light, LogoExtension::Svg),
                dark: candidate(LogoRole::Dark, LogoExtension::Jpeg),
            })
        );
    }

    /// The persisted shape records extensions, so `.jpg` and `.jpeg` survive a round trip.
    #[test]
    fn round_trips_through_json_without_losing_the_extension_spelling() {
        let jpg = PluginLogoVariants::from_roles(None, None, Some(LogoExtension::Jpg))
            .expect("a universal candidate resolves");
        let pair = PluginLogoVariants::from_roles(
            Some(LogoExtension::Svg),
            Some(LogoExtension::Jpeg),
            None,
        )
        .expect("a themed pair resolves");

        let jpg_json = serde_json::to_value(jpg).expect("serialize");
        let pair_json = serde_json::to_value(pair).expect("serialize");

        assert_eq!(
            (
                jpg_json.clone(),
                pair_json.clone(),
                serde_json::from_value::<PluginLogoVariants>(jpg_json).expect("deserialize"),
                serde_json::from_value::<PluginLogoVariants>(pair_json).expect("deserialize"),
            ),
            (
                serde_json::json!({ "universal": { "role": "universal", "extension": "jpg" } }),
                serde_json::json!({
                    "light": { "role": "light", "extension": "svg" },
                    "dark": { "role": "dark", "extension": "jpeg" },
                }),
                jpg,
                pair,
            )
        );
    }
}
