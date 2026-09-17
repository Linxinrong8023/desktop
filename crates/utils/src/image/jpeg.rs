//! JPEG header parsing: a walk from `SOI` over marker segments to the frame header.
//!
//! JPEG states its size in a start-of-frame (`SOF`) segment whose position depends on how many
//! tables, comments and application segments precede it, so the only way to read the size
//! without decoding is to follow the segment lengths until that marker appears.

use super::{ImageDimensions, ImageFormat, ImageHeaderError, be_u16};

const FORMAT: ImageFormat = ImageFormat::Jpeg;

/// The two-byte start-of-image marker every JPEG datastream opens with.
const START_OF_IMAGE: &[u8] = &[0xff, 0xd8];

/// Prefix byte that introduces every marker.
const MARKER_PREFIX: u8 = 0xff;

/// Start-of-scan: entropy-coded data begins here, so a size not seen by now is absent.
const START_OF_SCAN: u8 = 0xda;

/// End-of-image marker, which likewise ends the search.
const END_OF_IMAGE: u8 = 0xd9;

/// Returns whether `bytes` opens with the JPEG start-of-image marker.
pub(super) fn has_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(START_OF_IMAGE)
}

/// Walks the marker segments and reads the size from the first start-of-frame segment.
pub(super) fn read_dimensions(bytes: &[u8]) -> Result<ImageDimensions, ImageHeaderError> {
    let mut offset = START_OF_IMAGE.len();
    loop {
        // Any number of `0xff` fill bytes may pad the space before a marker.
        while bytes.get(offset) == Some(&MARKER_PREFIX) {
            offset += 1;
        }
        let marker = *bytes
            .get(offset)
            .ok_or(ImageHeaderError::Truncated { format: FORMAT })?;
        offset += 1;
        if marker == START_OF_SCAN || marker == END_OF_IMAGE {
            return Err(ImageHeaderError::Malformed {
                format: FORMAT,
                detail: "no start-of-frame segment precedes the scan",
            });
        }
        let length = usize::from(be_u16(bytes, offset, FORMAT)?);
        if length < 2 {
            return Err(ImageHeaderError::Malformed {
                format: FORMAT,
                detail: "segment declares a length shorter than its own length field",
            });
        }
        if is_start_of_frame(marker) {
            // A frame header is one precision byte followed by height then width, both 16-bit.
            return Ok(ImageDimensions {
                height: u32::from(be_u16(bytes, offset + 3, FORMAT)?),
                width: u32::from(be_u16(bytes, offset + 5, FORMAT)?),
            });
        }
        offset += length;
    }
}

/// Returns whether a marker introduces a start-of-frame segment.
///
/// The `SOF` range is not contiguous: `0xc4`, `0xc8` and `0xcc` sit inside it but introduce
/// Huffman tables, a JPEG extension and arithmetic-coding conditioning instead of a frame.
fn is_start_of_frame(marker: u8) -> bool {
    (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc)
}
