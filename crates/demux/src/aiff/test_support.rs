//! In-crate byte builders for the AIFF tests (card 5.A2) — the WAV
//! twin of [`crate::wav::test_support`]: public test-support surface,
//! nothing in the demuxer itself uses it.

use std::sync::Arc;

use plugin_api::SourceStream;

/// The 80-bit extended encoding of an integer sample rate (the
/// ffmpeg-compatible normal form — see
/// [`chunks::f64_to_extended80`](crate::aiff::chunks::f64_to_extended80)).
///
/// # Errors
/// [`media_types::EngineError`] when `rate` is not representable
/// (non-finite, non-integral, out of range).
pub fn extended80(rate: f64) -> Result<[u8; 10], media_types::EngineError> {
    crate::aiff::chunks::f64_to_extended80(rate)
}

/// Build a minimal valid classic AIFF: `FORM <n> AIFF` + 18-byte
/// COMM + SSND(offset 0, blockSize 0) — the smallest shape every
/// demuxer path must accept. `samples` must be whole sample frames.
#[must_use]
pub fn minimal_aiff(channels: u16, rate: u32, bits: u16, samples: &[u8]) -> Arc<[u8]> {
    let frames = (samples.len() / (channels as usize * usize::from(bits).div_ceil(8))) as u32;
    let mut b = Vec::with_capacity(54 + samples.len());
    b.extend_from_slice(b"FORM");
    b.extend_from_slice(&0u32.to_be_bytes()); // patched below
    b.extend_from_slice(b"AIFF");
    b.extend_from_slice(b"COMM");
    b.extend_from_slice(&18u32.to_be_bytes());
    b.extend_from_slice(&channels.to_be_bytes());
    b.extend_from_slice(&frames.to_be_bytes());
    b.extend_from_slice(&bits.to_be_bytes());
    match crate::aiff::chunks::f64_to_extended80(f64::from(rate)) {
        Ok(ext) => b.extend_from_slice(&ext),
        Err(_) => b.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]), // 44100
    }
    b.extend_from_slice(b"SSND");
    b.extend_from_slice(&(samples.len() as u32 + 8).to_be_bytes());
    b.extend_from_slice(&0u32.to_be_bytes()); // offset
    b.extend_from_slice(&0u32.to_be_bytes()); // blockSize
    b.extend_from_slice(samples);
    let form_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&form_size.to_be_bytes());
    Arc::from(b)
}

/// A boxed in-memory [`SourceStream`] over `bytes` (the demux
/// crate's shared seam).
///
/// # Errors
/// Propagates the (never-failing) URI parse.
pub fn mem_stream(
    bytes: impl Into<Arc<[u8]>>,
) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
    crate::avi::test_support::mem_stream(bytes)
}
