//! In-crate byte builders for the MPEG-PS tests (card 5.I2).
//!
//! Public like [`crate::avi::test_support`] on purpose: rustdoc
//! examples and the fuzz/conformance suites need deterministic
//! container shapes (including deliberately malformed ones).
//! Test-support surface, not player surface.

use std::sync::Arc;

use plugin_api::SourceStream;

/// A boxed in-memory [`SourceStream`] over `bytes` (the demux
/// crate's shared test seam — the AVI module's implementation).
///
/// # Errors
/// Propagates the (never-failing) URI parse.
pub fn mem_stream(
    bytes: impl Into<Arc<[u8]>>,
) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
    crate::avi::test_support::mem_stream(bytes)
}

/// Encode one 5-byte PTS/DTS stamp field (the 3/15/15 split the
/// demuxer decodes — round-trips with its parse).
#[must_use]
pub fn stamp_bytes(prefix4: u8, ticks: u64) -> [u8; 5] {
    let t = ticks & 0x1_FFFF_FFFF;
    let mut out = [0u8; 5];
    out[0] = (prefix4 << 4) | ((t >> 30) as u8 & 0x07) << 1 | 1;
    out[1] = ((t >> 22) & 0xFF) as u8;
    out[2] = (((t >> 15) & 0x7F) as u8) << 1 | 1;
    out[3] = ((t >> 7) & 0xFF) as u8;
    out[4] = ((t & 0x7F) as u8) << 1 | 1;
    out
}

/// Build one MPEG-1 pack header (12 bytes, `'0010'` prefix — the
/// ffmpeg `mpeg` muxer's flavor).
#[must_use]
pub fn mpeg1_pack(scr: u64, mux_rate: u32) -> Vec<u8> {
    let s = scr & 0x1_FFFF_FFFF;
    let mut out = vec![0x00, 0x00, 0x01, 0xBA];
    out.push(0x20 | (((s >> 30) & 0x07) as u8) << 1 | 1);
    out.push(((s >> 22) & 0xFF) as u8);
    out.push((((s >> 15) & 0x7F) as u8) << 1 | 1);
    out.push(((s >> 7) & 0xFF) as u8);
    out.push(((s & 0x7F) as u8) << 1 | 1);
    let m = mux_rate & 0x3F_FFFF;
    out.push(((m >> 14) & 0xFF) as u8);
    out.push(((m >> 6) & 0xFF) as u8);
    // The trailing two bits: the spec says markers `11`, ffmpeg
    // writes `01` — the builder forges ffmpeg's (see tests).
    out.push((((m & 0x3F) as u8) << 2) | 0x01);
    out
}

/// Build one MPEG-2 pack header (14 bytes, `'01'` prefix — the
/// ffmpeg `dvd`/VOB muxer's flavor).
#[must_use]
pub fn mpeg2_pack(scr: u64, mux_rate: u32, stuffing: &[u8]) -> Vec<u8> {
    let s = scr & 0x1_FFFF_FFFF;
    let mut out = vec![0x00, 0x00, 0x01, 0xBA];
    out.push(0x40 | (((s >> 30) & 0x07) as u8) << 3 | 0x04 | (((s >> 28) & 0x03) as u8));
    out.push(((s >> 20) & 0xFF) as u8);
    out.push((((s >> 15) & 0x1F) as u8) << 3 | 0x04 | (((s >> 13) & 0x03) as u8));
    out.push(((s >> 5) & 0xFF) as u8);
    out.push((((s & 0x1F) as u8) << 3) | 0x04);
    // SCR extension bits 0 + trailing marker.
    out.push(0x01);
    let m = mux_rate & 0x3F_FFFF;
    out.push(((m >> 14) & 0xFF) as u8);
    out.push(((m >> 6) & 0xFF) as u8);
    out.push((((m & 0x3F) as u8) << 2) | 0x03);
    out.push(0xF8 | (stuffing.len() as u8 & 0x07));
    out.extend_from_slice(stuffing);
    out
}

/// Build one MPEG-2 PES packet with a PTS stamp (VOB flavor).
#[must_use]
pub fn mpeg2_pes(stream_id: u8, payload: &[u8], pts: Option<u64>) -> Vec<u8> {
    let header_len = if pts.is_some() { 5 } else { 1 };
    let total = 3 + header_len + payload.len();
    let mut out = vec![0x00, 0x00, 0x01, stream_id];
    let len = u16::try_from(total).unwrap_or(u16::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.push(0x80); // '10' flags
    out.push(if pts.is_some() { 0x80 } else { 0x00 }); // marker bit 0 — ffmpeg's convention
    out.push(header_len as u8);
    if let Some(pts) = pts {
        out.extend_from_slice(&stamp_bytes(0b0010, pts));
    } else {
        out.push(0xFF); // single stuffing byte (ffmpeg's no-stamp shape)
    }
    out.extend_from_slice(payload);
    out
}

/// Build one MPEG-1 PES packet (`0x21` PTS / `0x0F` no-stamp — the
/// two header conventions the demuxer pins).
#[must_use]
pub fn mpeg1_pes(stream_id: u8, payload: &[u8], pts: Option<u64>) -> Vec<u8> {
    let total = if pts.is_some() {
        5 + payload.len()
    } else {
        1 + payload.len()
    };
    let mut out = vec![0x00, 0x00, 0x01, stream_id];
    let len = u16::try_from(total).unwrap_or(u16::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    if let Some(pts) = pts {
        out.extend_from_slice(&stamp_bytes(0b0010, pts));
    } else {
        out.push(0x0F); // the ffmpeg no-stamp marker
    }
    out.extend_from_slice(payload);
    out
}

/// The program end code.
#[must_use]
pub fn program_end() -> [u8; 4] {
    [0x00, 0x00, 0x01, 0xB9]
}

/// A private-1 PES payload as ffmpeg's `dvd` muxer writes AC-3:
/// substream byte + the 3-byte filler + frame data.
#[must_use]
pub fn ac3_substream_payload(substream: u8, filler: [u8; 3], frame: &[u8]) -> Vec<u8> {
    let mut out = vec![substream];
    out.extend_from_slice(&filler);
    out.extend_from_slice(frame);
    out
}

/// The doctest's demo: an MPEG-1 system stream with one video PES
/// and one audio PES between two packs.
#[must_use]
pub fn demo_stream() -> Arc<[u8]> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&mpeg1_pack(0, 250_000));
    bytes.extend_from_slice(&mpeg1_pes(0xE0, &[0x00, 0x00, 0x01, 0xB3], Some(9_000)));
    bytes.extend_from_slice(&mpeg1_pes(0xC0, &[0xFF, 0xFD, 0x40, 0xC4], Some(9_000)));
    bytes.extend_from_slice(&program_end());
    Arc::from(bytes)
}

/// A tiny deterministic AC-3-ish frame body (not a real frame — the
/// builders only need bytes the demuxer must pass through).
#[must_use]
pub fn ramp(len: usize, seed: u8) -> Vec<u8> {
    (0..len).map(|i| seed.wrapping_add(i as u8)).collect()
}
