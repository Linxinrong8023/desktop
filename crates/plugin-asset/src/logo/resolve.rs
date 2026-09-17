//! The one implementation of the candidate scan, shared by index building and package discovery.
//!
//! Both consumers face the same directory shape and the same rules, and keeping a single scan is
//! what stops the extension priority or the pair-completion rules from drifting apart between
//! them — a drift whose only symptom would be a plugin showing one icon in the marketplace and a
//! different one after install, which nobody would think to test for.

use super::candidate::{LOGO_EXTENSION_PRIORITY, LogoExtension, LogoRole, candidate_file_name};
use super::read::accepts_candidate;
use super::variants::PluginLogoVariants;
use ora_logging::ora_warn;
use std::path::Path;

/// Resolves the icon published in `directory` into its theme variants.
///
/// The scan is read-only: a package directory is byte-for-byte the unpacked archive and a
/// registry entry directory is an untrusted checkout, so neither is ever written to, not even to
/// cache what was found.
pub fn resolve_logo(directory: &Path) -> Option<PluginLogoVariants> {
    PluginLogoVariants::from_roles(
        resolve_role(directory, LogoRole::Light),
        resolve_role(directory, LogoRole::Dark),
        resolve_role(directory, LogoRole::Universal),
    )
}

/// Returns the winning extension for one role, or `None` when the role has no usable file.
///
/// The extensions are tried in a fixed order rather than in whatever order the directory lists
/// them, so the result of a scan depends only on which files exist. A candidate that exists but
/// cannot be served is skipped exactly like one that is absent, which is what lets a damaged
/// `logo.dark.webp` fall through to a sound `logo.dark.png` instead of costing the plugin its
/// dark icon — or its icon altogether.
fn resolve_role(directory: &Path, role: LogoRole) -> Option<LogoExtension> {
    for extension in LOGO_EXTENSION_PRIORITY {
        let path = directory.join(candidate_file_name(role, extension));
        match accepts_candidate(&path, extension) {
            Ok(()) => return Some(extension),
            Err(rejection) if rejection.is_absent() => {}
            Err(rejection) => {
                ora_warn!(
                    path = %path.display(),
                    error = %rejection,
                    "ignoring unusable plugin logo candidate"
                );
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::candidate::{LogoExtension, LogoRole};
    use super::super::fixtures::{
        SAFE_SVG, UNSAFE_SVG, animated_webp, gif, jpeg, png, webp, write,
    };
    use super::super::variants::{LogoCandidate, PluginLogoVariants};
    use super::resolve_logo;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    use tracing::Level;
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::registry::Registry;

    /// Names the candidate a role and extension address.
    fn candidate(role: LogoRole, extension: LogoExtension) -> LogoCandidate {
        LogoCandidate { role, extension }
    }

    /// Lists every entry in `directory`, so a scan can be shown to have written nothing.
    fn entries(directory: &Path) -> BTreeSet<String> {
        std::fs::read_dir(directory)
            .expect("list the icon directory")
            .map(|entry| {
                entry
                    .expect("read entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    /// Counts the `WARN` events a scan emits, which is how a skipped candidate stays observable.
    #[derive(Clone, Default)]
    struct WarningRecorder(Arc<Mutex<Vec<String>>>);

    impl WarningRecorder {
        fn layer(&self) -> impl Layer<Registry> + Send + Sync + 'static {
            self.clone()
        }

        fn warnings(&self) -> Vec<String> {
            self.0.lock().expect("warning recorder").clone()
        }
    }

    impl Layer<Registry> for WarningRecorder {
        fn on_event(&self, event: &tracing::Event<'_>, _context: Context<'_, Registry>) {
            if *event.metadata().level() == Level::WARN {
                self.0
                    .lock()
                    .expect("warning recorder")
                    .push(event.metadata().name().to_owned());
            }
        }
    }

    /// A directory with no candidate at all resolves to no icon rather than an error.
    #[test]
    fn resolves_an_empty_directory_to_no_icon() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;

        assert_eq!(resolve_logo(root.path()), None);
        Ok(())
    }

    /// Within one role the extension order is fixed, not the order the directory lists files in.
    ///
    /// Writing every extension for the same role and then removing the winner one at a time
    /// shows the whole priority chain, and shows it does not depend on creation order.
    #[test]
    fn picks_extensions_in_a_fixed_priority_order() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        // Deliberately written lowest-priority first, so file order contradicts the expectation.
        write(root.path(), "logo.jpeg", jpeg(16, 16));
        write(root.path(), "logo.jpg", jpeg(16, 16));
        write(root.path(), "logo.webp", webp(16, 16));
        write(root.path(), "logo.png", png(16, 16));
        write(root.path(), "logo.svg", SAFE_SVG);

        let mut winners = Vec::new();
        for removed in ["logo.svg", "logo.png", "logo.webp", "logo.jpg"] {
            winners.push(resolve_logo(root.path()));
            std::fs::remove_file(root.path().join(removed))?;
        }
        winners.push(resolve_logo(root.path()));

        let expected = |extension| {
            Some(PluginLogoVariants::Universal {
                universal: candidate(LogoRole::Universal, extension),
            })
        };
        assert_eq!(
            winners,
            vec![
                expected(LogoExtension::Svg),
                expected(LogoExtension::Png),
                expected(LogoExtension::Webp),
                expected(LogoExtension::Jpg),
                expected(LogoExtension::Jpeg),
            ]
        );
        Ok(())
    }

    /// A theme pair is resolved even when its halves ship in different formats.
    #[test]
    fn resolves_a_pair_whose_halves_use_different_formats() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = TempDir::new()?;
        write(root.path(), "logo.light.svg", SAFE_SVG);
        write(root.path(), "logo.dark.png", png(32, 32));

        assert_eq!(
            resolve_logo(root.path()),
            Some(PluginLogoVariants::Themed {
                light: candidate(LogoRole::Light, LogoExtension::Svg),
                dark: candidate(LogoRole::Dark, LogoExtension::Png),
            })
        );
        Ok(())
    }

    /// A damaged candidate is skipped and the next extension for the same role wins.
    ///
    /// The failure granularity is one file, not one role and not one plugin: a broken
    /// `logo.dark.svg` beside a sound `logo.dark.png` costs the author nothing. The damaged file
    /// deliberately outranks the sound one, so the scan has to reach past it rather than never
    /// looking at it.
    #[test]
    fn falls_through_a_damaged_candidate_to_the_next_extension()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.light.svg", SAFE_SVG);
        write(root.path(), "logo.dark.svg", UNSAFE_SVG);
        write(root.path(), "logo.dark.png", png(32, 32));

        let recorder = WarningRecorder::default();
        let resolved = ora_logging::with_recorded_trace_logging(recorder.layer(), || {
            resolve_logo(root.path())
        });

        assert_eq!(
            (resolved, recorder.warnings().len()),
            (
                Some(PluginLogoVariants::Themed {
                    light: candidate(LogoRole::Light, LogoExtension::Svg),
                    dark: candidate(LogoRole::Dark, LogoExtension::Png),
                }),
                1,
            )
        );
        Ok(())
    }

    /// One role's failure leaves the other roles, and the composition itself, intact.
    ///
    /// A damaged `logo.light.svg` beside a sound `logo.svg` still yields an icon, because the
    /// failure is indistinguishable from the themed file never having been shipped.
    #[test]
    fn a_failed_role_does_not_cost_the_plugin_its_icon() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.light.svg", UNSAFE_SVG);
        write(root.path(), "logo.svg", SAFE_SVG);

        let recorder = WarningRecorder::default();
        let resolved = ora_logging::with_recorded_trace_logging(recorder.layer(), || {
            resolve_logo(root.path())
        });

        assert_eq!(
            (resolved, recorder.warnings().len()),
            (
                Some(PluginLogoVariants::Universal {
                    universal: candidate(LogoRole::Universal, LogoExtension::Svg),
                }),
                1,
            )
        );
        Ok(())
    }

    /// Each of the six failure classes is equivalent to the candidate not existing.
    ///
    /// Every listed file is unusable for a different reason, and none of the reasons produces a
    /// state the decision table has to know about: the whole directory resolves to no icon, and
    /// each failure leaves one warning behind.
    #[test]
    fn every_failure_class_is_equivalent_to_an_absent_candidate()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        // Unsafe SVG, a format outside the whitelist, bytes contradicting the extension, an
        // oversized file, an oversized canvas, and an animated WebP.
        write(root.path(), "logo.svg", UNSAFE_SVG);
        write(root.path(), "logo.png", gif());
        write(root.path(), "logo.jpg", png(16, 16));
        write(
            root.path(),
            "logo.jpeg",
            "a".repeat(super::super::read::MAX_LOGO_BYTES + 1),
        );
        write(root.path(), "logo.light.png", png(8000, 8000));
        write(root.path(), "logo.dark.webp", animated_webp(32, 32));
        let before = entries(root.path());

        let recorder = WarningRecorder::default();
        let resolved = ora_logging::with_recorded_trace_logging(recorder.layer(), || {
            resolve_logo(root.path())
        });

        assert_eq!(
            (resolved, recorder.warnings().len(), entries(root.path())),
            (None, 6, before)
        );
        Ok(())
    }

    /// Resolution never writes to the directory it scans, and stays silent about absent files.
    ///
    /// A package directory is byte-for-byte the unpacked archive and a registry entry directory
    /// is an untrusted checkout, so the scan is a pure read — and the fourteen candidates a
    /// normal plugin does not ship must not each cost a log line.
    #[test]
    fn scanning_writes_nothing_and_stays_silent_about_absent_candidates()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.svg", SAFE_SVG);
        let before = entries(root.path());

        let recorder = WarningRecorder::default();
        let resolved = ora_logging::with_recorded_trace_logging(recorder.layer(), || {
            resolve_logo(root.path())
        });

        assert_eq!(
            (
                resolved.is_some(),
                entries(root.path()),
                recorder.warnings()
            ),
            (true, before, Vec::new())
        );
        Ok(())
    }
}
