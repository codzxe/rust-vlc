//! In-crate byte builders for the WAV tests (card 5.A2).
//!
//! Public like [`crate::avi::test_support`] on purpose: rustdoc
//! examples cannot see `#[cfg(test)]` items, and the integration
//! suites + fuzz harness need deterministic malformed shapes. This is
//! test-support surface, not player surface — nothing in the demuxer
//! itself uses it.

use std::sync::Arc;

use plugin_api::SourceStream;

/// Build a minimal valid little-endian PCM WAV: `RIFF <n> WAVE` +
/// 16-byte `fmt ` + `data`. No LIST, no fact, no padding — the
/// smallest shape every demuxer path must accept.
///
/// `samples` must be whole sample frames (`len % (channels × width)`
/// is the caller's business — the builder is a test tool, not a
/// validator).
#[must_use]
pub fn minimal_pcm_wav(
    fmt_tag: u16,
    channels: u16,
    rate: u32,
    bits: u16,
    samples: &[u8],
) -> Arc<[u8]> {
    let block_align = channels * (bits / 8);
    let mut b = Vec::with_capacity(44 + samples.len());
    b.extend_from_slice(b"RIFF");
    // Patched below.
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&fmt_tag.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    let avg = rate * u32::from(block_align);
    b.extend_from_slice(&avg.to_le_bytes());
    b.extend_from_slice(&block_align.to_le_bytes());
    b.extend_from_slice(&bits.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    b.extend_from_slice(samples);
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());
    Arc::from(b)
}

/// [`minimal_pcm_wav`] for the plain tag-1 PCM case the doctests use.
#[must_use]
pub fn minimal_pcm16_wav(samples: &[u8], rate: u32, channels: u16) -> Arc<[u8]> {
    minimal_pcm_wav(1, channels, rate, 16, samples)
}

/// A boxed in-memory [`SourceStream`] over `bytes` — the demux
/// crate's shared test seam (the AVI module's implementation, reused
/// verbatim: one `SourceStream`, not two).
///
/// # Errors
/// Propagates [`crate::avi::test_support::BytesStream::new`]'s
/// (never-failing) URI parse.
pub fn mem_stream(
    bytes: impl Into<Arc<[u8]>>,
) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
    crate::avi::test_support::mem_stream(bytes)
}
