//! Reading one icon candidate and deciding whether it may be served.
//!
//! Every rejection here is equivalent to the file not existing. Modelling a failure as a third
//! state would push the variant decision table from three boolean dimensions to three ternary
//! ones, and none of the extra rows could do anything but fall back to ignoring the file — so
//! the normalisation happens at the read, and the table never learns that failure exists.

use super::candidate::LogoExtension;
use ora_utils::image::{ImageHeaderError, read_header};
use ora_utils::svg::{SvgValidationError, validate};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;

/// The byte ceiling shared by all five extensions.
///
/// A single ceiling rather than a per-format one, because the byte count can be judged *before*
/// the format is known: the read stops one byte past the limit, so an oversized file is refused
/// without ever asking what it is. Per-format ceilings would order the cheapest gate after the
/// most expensive step for no gain, since 50 KiB is ample at icon scale.
pub const MAX_LOGO_BYTES: usize = ora_utils::svg::DEFAULT_MAX_SVG_BYTES;

/// The pixel ceiling each raster candidate must stay within on both axes.
///
/// It is independent of, and not implied by, the byte ceiling: a highly compressible image
/// passes the byte gate easily and still expands into a decompression bomb inside the webview's
/// decoder, which matters because a marketplace list decodes dozens of icons at once.
pub const MAX_LOGO_PIXELS: u32 = 1024;

/// Reports why one candidate file cannot be served as an icon.
///
/// Callers turn every variant into "this candidate is not there"; the distinctions exist only so
/// the warning says which packaging mistake was made.
#[derive(Debug, Error)]
pub(crate) enum LogoRejection {
    #[error("icon file could not be read: {0}")]
    Unreadable(#[source] io::Error),
    #[error("icon exceeds the {MAX_LOGO_BYTES} byte limit")]
    TooLarge,
    #[error("icon is not valid UTF-8 text")]
    NotUtf8,
    #[error(transparent)]
    UnsafeSvg(#[from] SvgValidationError),
    #[error(transparent)]
    UnusableRaster(#[from] ImageHeaderError),
    #[error("icon bytes are {actual} but the `.{expected}` extension promises another format")]
    FormatMismatch {
        expected: &'static str,
        actual: &'static str,
    },
    #[error("icon is {width}x{height}, past the {MAX_LOGO_PIXELS}x{MAX_LOGO_PIXELS} limit")]
    TooManyPixels { width: u32, height: u32 },
}

impl LogoRejection {
    /// Returns whether this rejection is the ordinary case of a candidate simply being absent.
    ///
    /// Absence is what most of the fifteen candidates are for any given plugin, so it is the one
    /// outcome that must stay silent instead of filling the log on every scan.
    pub(crate) fn is_absent(&self) -> bool {
        matches!(self, Self::Unreadable(error) if error.kind() == io::ErrorKind::NotFound)
    }
}

/// Checks whether the candidate at `path` may be served under `extension`.
///
/// The bytes are read once here and are not returned: the resolver's product is the mapping from
/// role to extension, and the protocol handler reads the file again when it actually serves it.
/// Validating up front is what lets the list-rendering path stay free of any byte-level check
/// and keeps every URL that reaches the contract a URL that has already passed this gate.
pub(crate) fn accepts_candidate(
    path: &Path,
    extension: LogoExtension,
) -> Result<(), LogoRejection> {
    let bytes = read_bounded(path)?;
    match extension.promised_raster_format() {
        None => {
            validate(&bytes)?;
            // The bytes are served verbatim as `image/svg+xml`, and the existing icon policy
            // admits text only, so a document that is not UTF-8 is refused rather than served.
            std::str::from_utf8(&bytes).map_err(|_| LogoRejection::NotUtf8)?;
            Ok(())
        }
        Some(promised) => {
            let header = read_header(&bytes)?;
            if header.format != promised {
                return Err(LogoRejection::FormatMismatch {
                    expected: extension.as_str(),
                    actual: header.format.as_str(),
                });
            }
            let (width, height) = (header.dimensions.width, header.dimensions.height);
            if width > MAX_LOGO_PIXELS || height > MAX_LOGO_PIXELS {
                return Err(LogoRejection::TooManyPixels { width, height });
            }
            Ok(())
        }
    }
}

/// Reads at most one byte past the ceiling so an oversized file is detected, never loaded.
fn read_bounded(path: &Path) -> Result<Vec<u8>, LogoRejection> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(LogoRejection::Unreadable)?
        .take(MAX_LOGO_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(LogoRejection::Unreadable)?;
    if bytes.len() > MAX_LOGO_BYTES {
        return Err(LogoRejection::TooLarge);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::super::candidate::LogoExtension;
    use super::super::fixtures::{
        SAFE_SVG, UNSAFE_SVG, animated_webp, gif, jpeg, png, webp, write,
    };
    use super::{LogoRejection, MAX_LOGO_BYTES, MAX_LOGO_PIXELS, accepts_candidate};
    use pretty_assertions::assert_eq;
    use std::path::Path;
    use tempfile::TempDir;

    /// Writes one candidate and reports how the read judged it, as a short label.
    fn verdict(directory: &Path, file_name: &str, extension: LogoExtension) -> &'static str {
        match accepts_candidate(&directory.join(file_name), extension) {
            Ok(()) => "accepted",
            Err(LogoRejection::Unreadable(_)) => "unreadable",
            Err(LogoRejection::TooLarge) => "too large",
            Err(LogoRejection::NotUtf8) => "not utf-8",
            Err(LogoRejection::UnsafeSvg(_)) => "unsafe svg",
            Err(LogoRejection::UnusableRaster(_)) => "unusable raster",
            Err(LogoRejection::FormatMismatch { .. }) => "format mismatch",
            Err(LogoRejection::TooManyPixels { .. }) => "too many pixels",
        }
    }

    /// Every whitelisted extension is accepted when its bytes agree with its name.
    #[test]
    fn accepts_each_whitelisted_extension() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.svg", SAFE_SVG);
        write(root.path(), "logo.png", png(64, 64));
        write(root.path(), "logo.webp", webp(64, 64));
        write(root.path(), "logo.jpg", jpeg(64, 64));
        write(root.path(), "logo.jpeg", jpeg(64, 64));

        assert_eq!(
            [
                verdict(root.path(), "logo.svg", LogoExtension::Svg),
                verdict(root.path(), "logo.png", LogoExtension::Png),
                verdict(root.path(), "logo.webp", LogoExtension::Webp),
                verdict(root.path(), "logo.jpg", LogoExtension::Jpg),
                verdict(root.path(), "logo.jpeg", LogoExtension::Jpeg),
            ],
            ["accepted"; 5]
        );
        Ok(())
    }

    /// The format comes from the bytes, so a file whose name disagrees with them is refused.
    ///
    /// Trusting the extension instead would let a `logo.png` full of SVG source walk past every
    /// SVG security check, and would leave the served content type describing the wrong bytes.
    #[test]
    fn refuses_a_candidate_whose_bytes_contradict_its_extension()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.png", UNSAFE_SVG);
        write(root.path(), "logo.webp", png(64, 64));
        // Raster bytes under an `.svg` name fail the SVG branch rather than being re-typed:
        // the policy admits text only, so the binary payload never reaches a response.
        write(root.path(), "logo.svg", png(64, 64));

        assert_eq!(
            [
                verdict(root.path(), "logo.png", LogoExtension::Png),
                verdict(root.path(), "logo.webp", LogoExtension::Webp),
                verdict(root.path(), "logo.svg", LogoExtension::Svg),
            ],
            ["unusable raster", "format mismatch", "not utf-8"]
        );
        Ok(())
    }

    /// Formats outside the whitelist are refused however they are named.
    #[test]
    fn refuses_formats_outside_the_whitelist() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.png", gif());
        write(root.path(), "logo.webp", animated_webp(64, 64));

        assert_eq!(
            [
                verdict(root.path(), "logo.png", LogoExtension::Png),
                verdict(root.path(), "logo.webp", LogoExtension::Webp),
            ],
            ["unusable raster", "unusable raster"]
        );
        Ok(())
    }

    /// An unsafe SVG is refused by the existing icon security policy, unchanged.
    #[test]
    fn refuses_an_unsafe_svg() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.svg", UNSAFE_SVG);

        assert_eq!(
            verdict(root.path(), "logo.svg", LogoExtension::Svg),
            "unsafe svg"
        );
        Ok(())
    }

    /// The byte ceiling applies to every extension and is judged before the format is known.
    #[test]
    fn refuses_an_oversized_candidate_of_any_extension() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        // Neither file is a valid image, yet both are refused for size: the byte gate closes
        // before anything asks what the bytes are.
        write(root.path(), "logo.svg", "a".repeat(MAX_LOGO_BYTES + 1));
        write(root.path(), "logo.png", vec![0u8; MAX_LOGO_BYTES + 1]);

        assert_eq!(
            [
                verdict(root.path(), "logo.svg", LogoExtension::Svg),
                verdict(root.path(), "logo.png", LogoExtension::Png),
            ],
            ["too large", "too large"]
        );
        Ok(())
    }

    /// The pixel ceiling holds independently of the byte ceiling.
    ///
    /// A highly compressible image sails through the byte gate and still expands into a
    /// decompression bomb in the webview's decoder, which a marketplace list would hit dozens of
    /// times at once.
    #[test]
    fn refuses_an_oversized_canvas_that_passes_the_byte_ceiling()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let bomb = png(8000, 8000);
        write(root.path(), "logo.png", &bomb);
        write(
            root.path(),
            "logo.webp",
            webp(MAX_LOGO_PIXELS, MAX_LOGO_PIXELS),
        );

        assert_eq!(
            (
                bomb.len() < MAX_LOGO_BYTES,
                verdict(root.path(), "logo.png", LogoExtension::Png),
                verdict(root.path(), "logo.webp", LogoExtension::Webp),
            ),
            (true, "too many pixels", "accepted")
        );
        Ok(())
    }

    /// A candidate that is not there is reported as absent, which callers keep silent.
    #[test]
    fn reports_an_absent_candidate_separately_from_a_bad_one()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        write(root.path(), "logo.png", gif());

        let absent = accepts_candidate(&root.path().join("logo.svg"), LogoExtension::Svg)
            .expect_err("a missing file is rejected");
        let present = accepts_candidate(&root.path().join("logo.png"), LogoExtension::Png)
            .expect_err("a GIF is rejected");

        assert_eq!((absent.is_absent(), present.is_absent()), (true, false));
        Ok(())
    }
}
