//! Header-only parsing for the raster image formats a host may accept as an asset.
//!
//! Every parser here reads only what the container declares about itself: the magic bytes that
//! identify the format and the fields that state the canvas size. No pixel data is decoded, so
//! bounding an image by its dimensions never costs more than a handful of byte reads and the
//! crate pulls in no image-decoding dependency.

mod jpeg;
mod png;
mod webp;

use thiserror::Error;

/// One raster image format whose dimensions can be read straight from its header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Webp,
    Jpeg,
}

impl ImageFormat {
    /// Returns the lowercase spelling used in diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Jpeg => "jpeg",
        }
    }
}

/// The canvas size an image header declares, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

/// One image's format and declared canvas size, as stated by its own header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageHeader {
    pub format: ImageFormat,
    pub dimensions: ImageDimensions,
}

/// Reports why a byte slice could not be read as a bounded still image.
///
/// The variants are deliberately coarse: a caller that treats an unreadable image as an absent
/// one only needs to know that the bytes are unusable, while diagnostics still name which format
/// was recognised before the header stopped making sense.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ImageHeaderError {
    #[error("bytes do not begin with the magic bytes of a supported image format")]
    UnrecognizedFormat,
    #[error("{format} header ends before the declared canvas size")]
    Truncated { format: ImageFormat },
    #[error("{format} header is malformed: {detail}")]
    Malformed {
        format: ImageFormat,
        detail: &'static str,
    },
    #[error("{format} image is animated")]
    Animated { format: ImageFormat },
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Identifies the format of `bytes` from its magic bytes alone, ignoring any filename.
///
/// Returning `None` is the whitelist in action: a format the host cannot bound without decoding
/// — an animated GIF among them — is indistinguishable here from arbitrary bytes, which is
/// exactly how callers are expected to treat it.
pub fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    if png::has_magic(bytes) {
        Some(ImageFormat::Png)
    } else if webp::has_magic(bytes) {
        Some(ImageFormat::Webp)
    } else if jpeg::has_magic(bytes) {
        Some(ImageFormat::Jpeg)
    } else {
        None
    }
}

/// Reads the format and declared canvas size of a still raster image.
///
/// The format is decided by [`detect_format`] before any field is interpreted, so a file whose
/// name disagrees with its contents is reported against what the bytes actually are. Animated
/// containers are rejected rather than measured: an icon that plays on its own is a different
/// kind of asset than the one callers asked to bound.
pub fn read_header(bytes: &[u8]) -> Result<ImageHeader, ImageHeaderError> {
    let format = detect_format(bytes).ok_or(ImageHeaderError::UnrecognizedFormat)?;
    let dimensions = match format {
        ImageFormat::Png => png::read_dimensions(bytes),
        ImageFormat::Webp => webp::read_dimensions(bytes),
        ImageFormat::Jpeg => jpeg::read_dimensions(bytes),
    }?;
    Ok(ImageHeader { format, dimensions })
}

/// Reads a big-endian `u32` at `offset`, or reports the header as truncated.
fn be_u32(bytes: &[u8], offset: usize, format: ImageFormat) -> Result<u32, ImageHeaderError> {
    let field = bytes
        .get(offset..offset + 4)
        .ok_or(ImageHeaderError::Truncated { format })?;
    Ok(u32::from_be_bytes([field[0], field[1], field[2], field[3]]))
}

/// Reads a big-endian `u16` at `offset`, or reports the header as truncated.
fn be_u16(bytes: &[u8], offset: usize, format: ImageFormat) -> Result<u16, ImageHeaderError> {
    let field = bytes
        .get(offset..offset + 2)
        .ok_or(ImageHeaderError::Truncated { format })?;
    Ok(u16::from_be_bytes([field[0], field[1]]))
}

#[cfg(test)]
mod tests {
    use super::{
        ImageDimensions, ImageFormat, ImageHeader, ImageHeaderError, detect_format, read_header,
    };
    use pretty_assertions::assert_eq;

    /// Builds a PNG whose signature and `IHDR` chunk declare `width` by `height`.
    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    /// Wraps one sub-format chunk in the `RIFF`/`WEBP` container it must arrive in.
    fn webp(chunk_type: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&((payload.len() + 12) as u32).to_le_bytes());
        bytes.extend_from_slice(b"WEBP");
        bytes.extend_from_slice(chunk_type);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    /// Builds a lossy `VP8 ` key-frame payload declaring `width` by `height`.
    fn lossy_payload(width: u16, height: u16) -> Vec<u8> {
        let mut payload = vec![0x10, 0x00, 0x00, 0x9d, 0x01, 0x2a];
        payload.extend_from_slice(&width.to_le_bytes());
        payload.extend_from_slice(&height.to_le_bytes());
        payload
    }

    /// Builds a lossless `VP8L` payload declaring `width` by `height`.
    fn lossless_payload(width: u32, height: u32) -> Vec<u8> {
        let packed = (width - 1) | ((height - 1) << 14);
        let mut payload = vec![0x2f];
        payload.extend_from_slice(&packed.to_le_bytes());
        payload
    }

    /// Builds an extended `VP8X` payload declaring `width` by `height` with the given flags.
    fn extended_payload(flags: u8, width: u32, height: u32) -> Vec<u8> {
        let mut payload = vec![flags, 0, 0, 0];
        payload.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
        payload.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
        payload
    }

    /// Builds a JPEG whose start-of-frame follows `leading` unrelated marker segments.
    fn jpeg(width: u16, height: u16, leading: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        for marker in leading {
            bytes.extend_from_slice(&[0xff, *marker, 0x00, 0x06, 1, 2, 3, 4]);
        }
        bytes.extend_from_slice(&[0xff, 0xc0, 0x00, 0x11, 8]);
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes
    }

    /// Identifies each whitelisted format from its magic bytes and nothing else.
    #[test]
    fn detects_every_supported_format_by_magic_bytes() {
        assert_eq!(
            (
                detect_format(&png(1, 1)),
                detect_format(&webp(b"VP8 ", &lossy_payload(1, 1))),
                detect_format(&jpeg(1, 1, &[])),
            ),
            (
                Some(ImageFormat::Png),
                Some(ImageFormat::Webp),
                Some(ImageFormat::Jpeg),
            )
        );
    }

    /// Refuses formats outside the whitelist, GIF and SVG source among them.
    #[test]
    fn refuses_formats_outside_the_whitelist() {
        assert_eq!(
            (
                detect_format(b"GIF89a\x01\x00\x01\x00"),
                detect_format(br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#),
                detect_format(b"RIFF\x00\x00\x00\x00WAVEfmt "),
                detect_format(b""),
            ),
            (None, None, None, None)
        );
        assert_eq!(
            read_header(b"GIF89a\x01\x00\x01\x00").unwrap_err(),
            ImageHeaderError::UnrecognizedFormat
        );
    }

    /// Reads the PNG canvas size from the mandatory `IHDR` chunk.
    #[test]
    fn reads_png_dimensions_from_ihdr() {
        assert_eq!(
            read_header(&png(320, 240)).unwrap(),
            ImageHeader {
                format: ImageFormat::Png,
                dimensions: ImageDimensions {
                    width: 320,
                    height: 240,
                },
            }
        );
    }

    /// Reports a PNG whose first chunk is not `IHDR` as malformed rather than guessing a size.
    #[test]
    fn refuses_a_png_without_a_leading_ihdr() {
        let mut bytes = png(8, 8);
        bytes[12..16].copy_from_slice(b"tEXt");

        assert_eq!(
            read_header(&bytes).unwrap_err(),
            ImageHeaderError::Malformed {
                format: ImageFormat::Png,
                detail: "first chunk is not IHDR",
            }
        );
    }

    /// Reads the canvas size of all three WebP sub-formats, not only the extended one.
    #[test]
    fn reads_dimensions_of_every_webp_sub_format() {
        let dimensions = |bytes: &[u8]| read_header(bytes).map(|header| header.dimensions);

        assert_eq!(
            (
                dimensions(&webp(b"VP8 ", &lossy_payload(64, 32))),
                dimensions(&webp(b"VP8L", &lossless_payload(64, 32))),
                dimensions(&webp(b"VP8X", &extended_payload(0, 64, 32))),
            ),
            (
                Ok(ImageDimensions {
                    width: 64,
                    height: 32,
                }),
                Ok(ImageDimensions {
                    width: 64,
                    height: 32,
                }),
                Ok(ImageDimensions {
                    width: 64,
                    height: 32,
                }),
            )
        );
    }

    /// Refuses an extended WebP that sets the animation flag.
    #[test]
    fn refuses_an_animated_webp() {
        assert_eq!(
            read_header(&webp(b"VP8X", &extended_payload(0b0000_0010, 64, 32))).unwrap_err(),
            ImageHeaderError::Animated {
                format: ImageFormat::Webp
            }
        );
    }

    /// Refuses a `RIFF`/`WEBP` container holding no recognised bitstream chunk.
    #[test]
    fn refuses_a_webp_without_a_bitstream_chunk() {
        assert_eq!(
            read_header(&webp(b"ICCP", &[0; 16])).unwrap_err(),
            ImageHeaderError::Malformed {
                format: ImageFormat::Webp,
                detail: "container holds no VP8, VP8L or VP8X chunk",
            }
        );
    }

    /// Walks past unrelated marker segments to read the size from the start-of-frame segment.
    #[test]
    fn reads_jpeg_dimensions_past_leading_segments() {
        assert_eq!(
            read_header(&jpeg(800, 600, &[0xe0, 0xdb, 0xc4])).unwrap(),
            ImageHeader {
                format: ImageFormat::Jpeg,
                dimensions: ImageDimensions {
                    width: 800,
                    height: 600,
                },
            }
        );
    }

    /// Reports a JPEG whose scan begins before any frame header as malformed.
    #[test]
    fn refuses_a_jpeg_without_a_start_of_frame() {
        let bytes = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 1, 2, 0xff, 0xda];

        assert_eq!(
            read_header(&bytes).unwrap_err(),
            ImageHeaderError::Malformed {
                format: ImageFormat::Jpeg,
                detail: "no start-of-frame segment precedes the scan",
            }
        );
    }

    /// Reports a header that ends inside its size fields as truncated for every format.
    #[test]
    fn reports_truncated_headers() {
        let png_bytes = png(8, 8);
        let webp_bytes = webp(b"VP8L", &lossless_payload(8, 8));
        let jpeg_bytes = jpeg(8, 8, &[]);

        assert_eq!(
            (
                read_header(&png_bytes[..18]).unwrap_err(),
                read_header(&webp_bytes[..22]).unwrap_err(),
                read_header(&jpeg_bytes[..6]).unwrap_err(),
            ),
            (
                ImageHeaderError::Truncated {
                    format: ImageFormat::Png
                },
                ImageHeaderError::Truncated {
                    format: ImageFormat::Webp
                },
                ImageHeaderError::Truncated {
                    format: ImageFormat::Jpeg
                },
            )
        );
    }
}
