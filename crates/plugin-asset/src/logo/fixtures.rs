//! Byte builders for icon files, shared by the tests of every module in this directory.
//!
//! Real bytes rather than mocks, because the whole point of the read is that it looks at what a
//! file actually contains: a fixture that only pretended to be a PNG would pass a check that the
//! production path is supposed to fail.

use std::fs;
use std::path::Path;

/// A minimal well-formed SVG that passes the icon security policy.
pub(super) const SAFE_SVG: &str =
    r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="8" height="8"/></svg>"#;

/// An SVG carrying a `<script>` element, which the security policy refuses.
pub(super) const UNSAFE_SVG: &str = "<svg><script>evil()</script></svg>";

/// Builds a PNG datastream whose `IHDR` declares `width` by `height`.
pub(super) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes
}

/// Builds a lossless (`VP8L`) WebP declaring `width` by `height`.
pub(super) fn webp(width: u32, height: u32) -> Vec<u8> {
    let packed = (width - 1) | ((height - 1) << 14);
    let mut payload = vec![0x2f];
    payload.extend_from_slice(&packed.to_le_bytes());
    riff(b"VP8L", &payload)
}

/// Builds an extended (`VP8X`) WebP that sets the animation flag.
pub(super) fn animated_webp(width: u32, height: u32) -> Vec<u8> {
    let mut payload = vec![0b0000_0010, 0, 0, 0];
    payload.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    payload.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    riff(b"VP8X", &payload)
}

/// Wraps one WebP bitstream chunk in its `RIFF`/`WEBP` container.
fn riff(chunk_type: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&((payload.len() + 12) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(chunk_type);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

/// Builds a baseline JPEG whose start-of-frame declares `width` by `height`.
pub(super) fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 8];
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes
}

/// Builds a GIF, the animation-capable format the whitelist deliberately excludes.
pub(super) fn gif() -> Vec<u8> {
    b"GIF89a\x08\x00\x08\x00\x00\x00\x00".to_vec()
}

/// Writes one icon candidate into `directory` under the exact filename given.
pub(super) fn write(directory: &Path, file_name: &str, bytes: impl AsRef<[u8]>) {
    fs::write(directory.join(file_name), bytes).expect("write icon candidate");
}
