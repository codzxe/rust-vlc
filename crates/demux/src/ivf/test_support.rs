//! In-crate byte builders for the IVF tests (card 5.I3b).
//!
//! Public like [`crate::avi::test_support`] on purpose: the
//! integration/fuzz suites need deterministic container shapes
//! (including deliberately malformed ones). Test-support surface, not
//! player surface — nothing in the demuxer uses it.

use std::sync::Arc;

use plugin_api::SourceStream;

/// A boxed in-memory [`SourceStream`] over `bytes` (the demux crate's
/// shared test seam — the AVI module's implementation).
///
/// # Errors
/// Propagates the (never-failing) URI parse.
pub fn mem_stream(
    bytes: impl Into<Arc<[u8]>>,
) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
    crate::avi::test_support::mem_stream(bytes)
}

/// Build a 32-byte IVF file header. `fourcc` is written verbatim;
/// `rate`/`scale` are the tick ratio (one tick = scale/rate seconds);
/// `frames` is the declared frame count (advisory and freely lied
/// about — the walk is the truth).
#[must_use]
pub fn ivf_header(
    fourcc: &[u8; 4],
    width: u16,
    height: u16,
    rate: u32,
    scale: u32,
    frames: u32,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(b"DKIF");
    out.extend_from_slice(&0u16.to_le_bytes()); // version
    out.extend_from_slice(&32u16.to_le_bytes()); // header length
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&scale.to_le_bytes());
    out.extend_from_slice(&frames.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // unused
    out
}

/// Build one IVF frame (`size`-byte payload of a deterministic ramp,
/// the given pts). The ramp is byte-labeled (verifiable, not
/// zero-padded — zero runs hide truncation bugs).
#[must_use]
pub fn ivf_frame(pts: u64, payload_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + payload_len);
    out.extend_from_slice(&u32::try_from(payload_len).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&pts.to_le_bytes());
    for i in 0..payload_len {
        out.push((i % 251) as u8);
    }
    out
}

/// An AV1-flavored demo stream: 30 fps, 64×48, three ramp frames at
/// pts 0/1/2 (the doctest's demo shape — the same field values
/// ffmpeg writes for a 30 fps delivery).
#[must_use]
pub fn demo_stream() -> Arc<[u8]> {
    let mut bytes = ivf_header(b"AV01", 64, 48, 30, 1, 3);
    for pts in 0..3u64 {
        bytes.extend_from_slice(&ivf_frame(pts, 8));
    }
    Arc::from(bytes)
}
