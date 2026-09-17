//! PNG header parsing: the eight-byte signature followed by the mandatory `IHDR` chunk.

use super::{ImageDimensions, ImageFormat, ImageHeaderError, be_u32};

/// The fixed eight-byte signature every PNG datastream opens with.
const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Byte offset of the first chunk's four-character type, which the spec fixes as `IHDR`.
const FIRST_CHUNK_TYPE_OFFSET: usize = 12;

/// Byte offset of the `IHDR` payload, which opens with the width and height fields.
const IHDR_PAYLOAD_OFFSET: usize = 16;

/// Returns whether `bytes` opens with the PNG signature.
pub(super) fn has_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(SIGNATURE)
}

/// Reads the canvas size the mandatory `IHDR` chunk declares.
///
/// `IHDR` is required by the specification to be the first chunk, so its position is fixed and
/// no chunk walk is needed; a stream that puts something else there is malformed rather than an
/// image whose size lives elsewhere.
pub(super) fn read_dimensions(bytes: &[u8]) -> Result<ImageDimensions, ImageHeaderError> {
    const FORMAT: ImageFormat = ImageFormat::Png;
    let chunk_type = bytes
        .get(FIRST_CHUNK_TYPE_OFFSET..FIRST_CHUNK_TYPE_OFFSET + 4)
        .ok_or(ImageHeaderError::Truncated { format: FORMAT })?;
    if chunk_type != b"IHDR" {
        return Err(ImageHeaderError::Malformed {
            format: FORMAT,
            detail: "first chunk is not IHDR",
        });
    }
    Ok(ImageDimensions {
        width: be_u32(bytes, IHDR_PAYLOAD_OFFSET, FORMAT)?,
        height: be_u32(bytes, IHDR_PAYLOAD_OFFSET + 4, FORMAT)?,
    })
}
