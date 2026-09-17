//! WebP header parsing across the three sub-formats a RIFF/WEBP container can hold.
//!
//! WebP is a container, not a single bitstream: the canvas size lives at a different offset and
//! in a different bit layout in each of the lossy (`VP8 `), lossless (`VP8L`) and extended
//! (`VP8X`) sub-formats. Export tools emit the two non-extended forms for small images, so a
//! parser that only understood `VP8X` would misread the most common files as damaged.

use super::{ImageDimensions, ImageFormat, ImageHeaderError};

const FORMAT: ImageFormat = ImageFormat::Webp;

/// Byte offset of the four-character code naming the container's first chunk.
const CHUNK_TYPE_OFFSET: usize = 12;

/// Byte offset at which the first chunk's payload begins, past its type and size fields.
const CHUNK_PAYLOAD_OFFSET: usize = 20;

/// Bit in the `VP8X` flags byte that marks the file as an animation.
const ANIMATION_FLAG: u8 = 0b0000_0010;

/// Three-byte start code that follows a lossy key frame's frame tag.
const VP8_KEY_FRAME_START_CODE: &[u8] = &[0x9d, 0x01, 0x2a];

/// Signature byte that opens a lossless bitstream.
const VP8L_SIGNATURE: u8 = 0x2f;

/// Mask of the 14-bit dimension fields used by both the lossy and lossless sub-formats.
const FOURTEEN_BITS: u32 = 0x3fff;

/// Returns whether `bytes` opens with a `RIFF` container whose form type is `WEBP`.
pub(super) fn has_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
}

/// Reads the canvas size declared by whichever sub-format the container holds.
///
/// An extended container that sets the animation flag is refused here rather than measured: the
/// flag is the only reliable statement a WebP makes about being animated, and it is cheaper to
/// honour it at the same moment the canvas size is read than to re-open the file later.
pub(super) fn read_dimensions(bytes: &[u8]) -> Result<ImageDimensions, ImageHeaderError> {
    let chunk_type = bytes
        .get(CHUNK_TYPE_OFFSET..CHUNK_TYPE_OFFSET + 4)
        .ok_or(ImageHeaderError::Truncated { format: FORMAT })?;
    let payload = bytes
        .get(CHUNK_PAYLOAD_OFFSET..)
        .ok_or(ImageHeaderError::Truncated { format: FORMAT })?;
    match chunk_type {
        b"VP8 " => lossy_dimensions(payload),
        b"VP8L" => lossless_dimensions(payload),
        b"VP8X" => extended_dimensions(payload),
        _ => Err(ImageHeaderError::Malformed {
            format: FORMAT,
            detail: "container holds no VP8, VP8L or VP8X chunk",
        }),
    }
}

/// Reads the frame size of a lossy (`VP8 `) key frame.
fn lossy_dimensions(payload: &[u8]) -> Result<ImageDimensions, ImageHeaderError> {
    let header = payload
        .get(..10)
        .ok_or(ImageHeaderError::Truncated { format: FORMAT })?;
    // Only a key frame carries the picture size; an inter frame at the start of a still image
    // means the bitstream is not what the container claims it is.
    if header[0] & 1 != 0 || &header[3..6] != VP8_KEY_FRAME_START_CODE {
        return Err(ImageHeaderError::Malformed {
            format: FORMAT,
            detail: "lossy bitstream does not open with a key frame",
        });
    }
    Ok(ImageDimensions {
        width: le_u16(&header[6..8]) & FOURTEEN_BITS,
        height: le_u16(&header[8..10]) & FOURTEEN_BITS,
    })
}

/// Reads the image size of a lossless (`VP8L`) bitstream.
fn lossless_dimensions(payload: &[u8]) -> Result<ImageDimensions, ImageHeaderError> {
    let header = payload
        .get(..5)
        .ok_or(ImageHeaderError::Truncated { format: FORMAT })?;
    if header[0] != VP8L_SIGNATURE {
        return Err(ImageHeaderError::Malformed {
            format: FORMAT,
            detail: "lossless bitstream lacks its signature byte",
        });
    }
    // The two dimensions are packed as consecutive 14-bit fields holding size minus one.
    let packed = u32::from_le_bytes([header[1], header[2], header[3], header[4]]);
    Ok(ImageDimensions {
        width: (packed & FOURTEEN_BITS) + 1,
        height: ((packed >> 14) & FOURTEEN_BITS) + 1,
    })
}

/// Reads the canvas size of an extended (`VP8X`) container, refusing animations.
fn extended_dimensions(payload: &[u8]) -> Result<ImageDimensions, ImageHeaderError> {
    let header = payload
        .get(..10)
        .ok_or(ImageHeaderError::Truncated { format: FORMAT })?;
    if header[0] & ANIMATION_FLAG != 0 {
        return Err(ImageHeaderError::Animated { format: FORMAT });
    }
    Ok(ImageDimensions {
        width: le_u24(&header[4..7]) + 1,
        height: le_u24(&header[7..10]) + 1,
    })
}

/// Reads a little-endian `u16` from a two-byte field.
fn le_u16(field: &[u8]) -> u32 {
    u32::from(u16::from_le_bytes([field[0], field[1]]))
}

/// Reads a little-endian 24-bit field, the width every `VP8X` canvas dimension uses.
fn le_u24(field: &[u8]) -> u32 {
    u32::from_le_bytes([field[0], field[1], field[2], 0])
}
