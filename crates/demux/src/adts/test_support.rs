//! In-crate byte builders for the ADTS/ADIF tests (card 5.C2b).
//!
//! Public like [`crate::avi::test_support`] on purpose: rustdoc
//! examples and the integration/fuzz suites need deterministic frame
//! shapes (including deliberately malformed ones). Test-support
//! surface, not player surface — nothing in the demuxer uses it.

use std::sync::Arc;

use plugin_api::SourceStream;

use super::{SAMPLE_RATES, SAMPLES_PER_BLOCK};

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

/// The CRC-16/MPEG-2 over the 56 header bits (the same check the
/// demuxer applies — exported so builders can forge protected frames).
#[must_use]
pub fn header_crc(head7: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in head7 {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Build one ADTS frame. `payload_len` counts the bytes after the
/// header; `blocks` is `number_of_raw_data_blocks_in_frame` (1–4).
/// `Some(bad)` forges the 9-byte protected header with a correct
/// (`bad == false`) or deliberately wrong (`bad == true`) `crc_check`;
/// `None` writes `protection_absent = 1`.
#[must_use]
pub fn adts_frame(
    profile: u8,
    sampling_index: u8,
    channel_config: u8,
    blocks: u8,
    payload_len: usize,
    protection: Option<bool>,
) -> Vec<u8> {
    let blocks = blocks.clamp(1, 4);
    let total = 7 + 2 * usize::from(protection.is_some()) + payload_len;
    let flen = u16::try_from(total).unwrap_or(u16::MAX);
    let mut head = [0u8; 7];
    head[0] = 0xFF; // syncword[11:4]
    head[1] = 0xF0 // syncword[3:0] | ID=0 (MPEG-4) | layer=0
        | u8::from(protection.is_none()); // protection_absent
    head[2] = (profile & 0x03) << 6 | (sampling_index & 0x0F) << 2 | (channel_config >> 2) & 0x01;
    head[3] = (channel_config & 0x03) << 6
        | 0x20 // original_copy
        | 0x10 // home
        | ((flen >> 11) & 0x03) as u8;
    head[4] = (flen >> 3) as u8;
    head[5] = ((flen as u8) & 0x07) << 5 | (0x7FF >> 6) as u8;
    head[6] = 0xFC | ((blocks - 1) & 0x03);
    let mut frame = head.to_vec();
    if let Some(bad) = protection {
        let crc = if bad { 0xBEEF } else { header_crc(&head) };
        frame.extend_from_slice(&crc.to_be_bytes());
    }
    // Deterministic payload: a byte-labeled ramp (verifiable, not
    // zero-padded — zero runs hide truncation bugs).
    for i in 0..payload_len {
        frame.push((i % 251) as u8);
    }
    frame
}

/// An ADTS stream of `frames` uniform frames (the demo shape).
#[must_use]
pub fn adts_stream(frames: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..frames {
        out.extend_from_slice(&adts_frame(1, 4, 2, 1, 100, None));
    }
    out
}

/// The doctest's demo: three LC stereo 44.1 kHz frames of 8 payload
/// bytes each.
#[must_use]
pub fn demo_stream() -> Arc<[u8]> {
    let mut bytes = Vec::new();
    for _ in 0..3 {
        bytes.extend_from_slice(&adts_frame(1, 4, 2, 1, 8, None));
    }
    Arc::from(bytes)
}

/// An ID3v2.3 tag carrying `payload` (syncsafe 28-bit size; the
/// demuxer skips the whole tag).
#[must_use]
pub fn id3v2_tag(payload: &[u8]) -> Vec<u8> {
    let size = payload.len() as u64;
    let mut tag = vec![b'I', b'D', b'3', 3, 0, 0];
    tag.push(((size >> 21) & 0x7F) as u8);
    tag.push(((size >> 14) & 0x7F) as u8);
    tag.push(((size >> 7) & 0x7F) as u8);
    tag.push((size & 0x7F) as u8);
    tag.extend_from_slice(payload);
    tag
}

/// Build a single-program **CBR ADIF** stream from `frames` uniform
/// raw blocks of `frame_len` bytes at `sampling_index` — the
/// container-agnostic mirror of [`adts_stream`] (the cross-container
/// differential's other half). The bitrate is derived so the demuxer's
/// `bitrate × 128 / rate` arithmetic reproduces `frame_len` exactly.
#[must_use]
pub fn adif_stream(sampling_index: u8, channels: u16, frames: usize, frame_len: usize) -> Vec<u8> {
    let rate = SAMPLE_RATES[usize::from(sampling_index.min(12))];
    // bitrate = frame_len × 8 × rate / 1024 = frame_len × rate / 128.
    let bitrate = u32::try_from(frame_len * rate as usize / 128).unwrap_or(u32::MAX);
    let mut bits: Vec<bool> = Vec::new();
    let push = |bits: &mut Vec<bool>, value: u64, n: usize| {
        for i in (0..n).rev() {
            bits.push(value >> i & 1 == 1);
        }
    };
    for byte in b"ADIF" {
        push(&mut bits, u64::from(*byte), 8);
    }
    bits.push(false); // copyright_id_present = 0
    bits.push(true); // original_copy
    bits.push(false); // home
    bits.push(false); // bitstream_type = 0 (CBR)
    push(&mut bits, u64::from(bitrate.min(0x7F_FFFF)), 23);
    push(&mut bits, 0, 4); // num_program_config_elements = 0
    // program_config_element: 1 front SCE per channel pair/mono.
    push(&mut bits, 0, 4); // element_instance_tag
    push(&mut bits, 1, 2); // profile = LC
    push(&mut bits, u64::from(sampling_index), 4);
    let front = channels.div_ceil(2);
    push(&mut bits, u64::from(front), 4); // num_front_channel_elements
    push(&mut bits, 0, 4); // side
    push(&mut bits, 0, 4); // back
    push(&mut bits, 0, 2); // lfe
    push(&mut bits, 0, 3); // assoc
    push(&mut bits, 0, 4); // valid_cc
    bits.push(false); // mono_mixdown_present
    bits.push(false); // stereo_mixdown_present
    bits.push(false); // matrix_mixdown_idx_present
    for i in 0..front {
        // Element 0 is a CPE only for even channel counts; the rest
        // pair up (1 → SCE, 2/4/6 → CPEs, 3/5 → SCE + CPEs).
        let cpe = channels.is_multiple_of(2) || i > 0;
        bits.push(cpe);
        push(&mut bits, 0, 4); // element_tag_select
    }
    while !bits.len().is_multiple_of(8) {
        bits.push(false); // byte_align
    }
    push(&mut bits, 0, 8); // comment_field_bytes = 0
    let mut out = Vec::new();
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for (i, bit) in chunk.iter().enumerate() {
            byte |= u8::from(*bit) << (7 - i);
        }
        out.push(byte);
    }
    for f in 0..frames {
        for i in 0..frame_len {
            out.push(((f * 31 + i) % 251) as u8);
        }
    }
    out
}

/// The samples one raw block carries (re-exported for arithmetic
/// assertions in the suites).
pub const BLOCK_SAMPLES: u64 = SAMPLES_PER_BLOCK;
