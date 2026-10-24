//! In-crate byte builders for the Ogg tests (card 5.B1).
//!
//! Public like [`crate::avi::test_support`] on purpose: rustdoc
//! examples and the integration/fuzz suites need deterministic page
//! shapes (including deliberately malformed ones). Test-support
//! surface, not player surface — nothing in the demuxer uses it.

use std::sync::Arc;

use plugin_api::SourceStream;

use super::page;

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

/// Page header-type bits.
pub const CONTINUED: u8 = 0x01;
/// The BOS flag.
pub const BOS: u8 = 0x02;
/// The EOS flag.
pub const EOS: u8 = 0x04;

/// Build one CRC-correct Ogg page carrying `packets` (each packet is
/// laced into 255-byte runs; a final run of exactly 255 leaves the
/// packet open into the next page — the caller's tool for continued
/// packets). `granule` of `-1` writes the "no position" value.
#[must_use]
pub fn build_page(flags: u8, granule: i64, serial: u32, seq: u32, packets: &[&[u8]]) -> Vec<u8> {
    let mut segments: Vec<u8> = Vec::new();
    let mut body: Vec<u8> = Vec::new();
    for packet in packets {
        let mut left = packet.len();
        let mut first = true;
        while left > 0 {
            let take = left.min(255);
            segments.push(take as u8);
            let start = packet.len() - left;
            body.extend_from_slice(&packet[start..start + take]);
            left -= take;
            first = false;
            if take == 255 && left == 0 {
                // A run of exactly 255 continues (packet stays open).
                break;
            }
        }
        if first {
            // Zero-length packet: one zero lacing value.
            segments.push(0);
        }
    }
    if segments.is_empty() {
        segments.push(0);
    }
    let nsegs = segments.len().min(255);
    segments.truncate(nsegs);
    let mut page = Vec::with_capacity(page::HEADER_BYTES + nsegs + body.len());
    page.extend_from_slice(page::CAPTURE);
    page.push(0); // version
    page.push(flags);
    page.extend_from_slice(&granule.to_le_bytes());
    page.extend_from_slice(&serial.to_le_bytes());
    page.extend_from_slice(&seq.to_le_bytes());
    page.extend_from_slice(&0u32.to_le_bytes()); // CRC patched below
    page.push(nsegs as u8);
    page.extend_from_slice(&segments);
    page.extend_from_slice(&body);
    let crc = page::crc32(&page);
    // The CRC field lives at byte 22: total − 5 − nsegs − body.
    let at = page.len() - body.len() - nsegs - 5;
    page[at..at + 4].copy_from_slice(&crc.to_le_bytes());
    page
}

/// A minimal Vorbis identification header packet (30 bytes, the
/// canonical BOS body for a 8 kHz mono stream).
#[must_use]
pub fn vorbis_id_packet(rate: u32) -> Vec<u8> {
    let mut v = vec![0x01];
    v.extend_from_slice(b"vorbis");
    v.extend_from_slice(&0u32.to_le_bytes());
    v.push(1); // channels
    v.extend_from_slice(&rate.to_le_bytes()[..3]); // LE24
    v.extend_from_slice(&[0; 16]); // bitrates + block sizes + framing
    v
}

/// A minimal OpusHead packet (19 bytes, 48 kHz family).
#[must_use]
pub fn opus_head_packet() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"OpusHead");
    v.push(1); // version
    v.push(1); // channels
    v.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
    v.extend_from_slice(&48_000u32.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes()); // gain
    v.push(0); // mapping family
    v
}

/// The doctest's demo: one Vorbis-identified stream — BOS (ID
/// header), one data page (two packets), EOS.
#[must_use]
pub fn demo_stream() -> Arc<[u8]> {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0xAA11, 0, &[&id]));
    bytes.extend_from_slice(&build_page(
        0,
        512,
        0xAA11,
        1,
        &[b"\x00audio-a", b"\x00audio-b"],
    ));
    bytes.extend_from_slice(&build_page(EOS, 1024, 0xAA11, 2, &[b"\x00audio-c"]));
    Arc::from(bytes)
}
