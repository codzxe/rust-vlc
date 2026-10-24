//! The ADTS frame header parse (card 5.C2a's decode-entry framing
//! half; ISO 14496-3 §1.A.6.2 — the 56-bit fixed header + the 16-bit
//! optional CRC).
//!
//! This module is deliberately byte-level and standalone: the codec's
//! decode entry needs profile / sampling index / channel config to
//! configure the engine per stream, and the resync walker needs the
//! same validation to find frame boundaries in garbage. The
//! **demuxer-side** ADTS surface (probe, packetization, seeking) is
//! the parallel 5.C2b card's — this is the codec-side mirror of
//! `mp3::header`.
//!
//! Bit layout (MSB-first, 7 bytes when `protection_absent` = 1):
//!
//! ```text
//!  0..12  syncword                 0xFFF
//! 12      ID                       0 = MPEG-4, 1 = MPEG-2
//! 13..15  layer                    0b00 (always — anything else is damage)
//! 15      protection_absent        1 = no CRC (header 7 B), 0 = CRC (9 B)
//! 16..18  profile                  0 = Main, 1 = LC, 2 = SSR, 3 = reserved
//! 18..22  sampling_frequency_index 0..=12 valid, 13..15 reserved
//! 22      private_bit
//! 23..26  channel_configuration    0 = PCE in band, 1..=6 = count, 7 = 8ch
//! 26..30  original/home/copyright bits (informational)
//! 30..43  aac_frame_length         full frame incl. header
//! 43..54  adts_buffer_fullness     0x7FF = VBR
//! 54..56  number_of_raw_data_blocks_in_frame − 1
//! ```

use super::error::{AacError, corrupt};
use media_types::error::CorruptDetail;

/// The ADTS sampling-frequency table (ISO 14496-3 Table 1.16; indices
/// 13–15 are reserved and refused typedly).
pub(crate) const SAMPLING_FREQUENCIES: [u32; 13] = [
    96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
    7_350,
];

/// The ADTS `profile` field (already the "profile − 1" encoding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdtsProfile {
    /// profile 0 — AAC Main (engine: not implemented).
    Main,
    /// profile 1 — AAC LC (the supported subset).
    Lc,
    /// profile 2 — AAC SSR (engine: not implemented).
    Ssr,
    /// profile 3 — reserved.
    Reserved,
}

impl AdtsProfile {
    /// The stable dotted-code fragment for the typed refusal.
    pub(crate) fn name(self) -> &'static str {
        match self {
            AdtsProfile::Main => "main",
            AdtsProfile::Lc => "lc",
            AdtsProfile::Ssr => "ssr",
            AdtsProfile::Reserved => "reserved",
        }
    }
}

/// Channel-count resolution for `channel_configuration` (ISO 14496-3
/// Table 1.17): 0 is PCE-in-band (refused typedly upstream — the
/// engine requires the count up front), 7 means 8 channels.
#[must_use]
pub(crate) fn channel_count(config: u8) -> u8 {
    match config {
        0 => 0,
        1..=6 => config,
        _ => 8,
    }
}

/// One parsed ADTS fixed header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdtsHeader {
    /// ID bit: `true` = MPEG-2, `false` = MPEG-4.
    pub mpeg2: bool,
    /// `protection_absent`: `true` = 7-byte header, `false` = 9 (+CRC).
    pub protection_absent: bool,
    /// The profile field.
    pub profile: AdtsProfile,
    /// The raw sampling-frequency index.
    pub sampling_index: u8,
    /// The resolved sample rate in Hz.
    pub sample_rate: u32,
    /// The raw channel-configuration field.
    pub channel_config: u8,
    /// The resolved channel count (0 = PCE in band).
    pub channels: u8,
    /// The full frame length in bytes, header included.
    pub frame_len: usize,
    /// The header length (7 or 9).
    pub header_len: usize,
    /// Raw data blocks in this frame (`field + 1`).
    pub raw_blocks: usize,
}

/// Parse one ADTS fixed header at the start of `data`.
///
/// # Errors
/// Typed [`AacError`]s: fewer than 7 bytes (`aac.adts.truncated`), a
/// bad syncword (`aac.adts.sync`), a non-zero layer field
/// (`aac.adts.layer`), a reserved sampling index
/// (`aac.adts.sampling_index`), a frame length below its own header
/// (`aac.adts.frame_len`).
pub(crate) fn parse_adts_header(data: &[u8]) -> Result<AdtsHeader, AacError> {
    if data.len() < 7 {
        return Err(corrupt(
            "aac.adts.truncated",
            CorruptDetail::Truncated {
                expected: 7,
                found: data.len() as u64,
            },
            "an ADTS fixed header needs 7 bytes",
        ));
    }
    if data[0] != 0xFF || data[1] >> 4 != 0xF {
        return Err(corrupt(
            "aac.adts.sync",
            CorruptDetail::InvalidStructure { offset: Some(0) },
            "syncword is not 0xFFF",
        ));
    }
    let mpeg2 = data[1] & 0x08 != 0;
    let layer = (data[1] >> 1) & 0x03;
    if layer != 0 {
        return Err(corrupt(
            "aac.adts.layer",
            CorruptDetail::InvalidStructure { offset: Some(1) },
            format!("layer field is {layer}, must be 0"),
        ));
    }
    let protection_absent = data[1] & 0x01 != 0;
    let profile = match data[2] >> 6 {
        0 => AdtsProfile::Main,
        1 => AdtsProfile::Lc,
        2 => AdtsProfile::Ssr,
        _ => AdtsProfile::Reserved,
    };
    let sampling_index = (data[2] >> 2) & 0x0F;
    let Some(&sample_rate) = SAMPLING_FREQUENCIES.get(sampling_index as usize) else {
        return Err(corrupt(
            "aac.adts.sampling_index",
            CorruptDetail::InvalidStructure { offset: Some(2) },
            format!("sampling-frequency index {sampling_index} is reserved"),
        ));
    };
    let channel_config = ((data[2] & 0x01) << 2) | (data[3] >> 6);
    let frame_len =
        (((data[3] & 0x03) as usize) << 11) | (data[4] as usize) << 3 | ((data[5] >> 5) as usize);
    let header_len = if protection_absent { 7 } else { 9 };
    if frame_len < header_len {
        return Err(corrupt(
            "aac.adts.frame_len",
            CorruptDetail::InvalidStructure { offset: Some(3) },
            format!("frame length {frame_len} is below the {header_len}-byte header"),
        ));
    }
    Ok(AdtsHeader {
        mpeg2,
        protection_absent,
        profile,
        sampling_index,
        sample_rate,
        channel_config,
        channels: channel_count(channel_config),
        frame_len,
        header_len,
        raw_blocks: (data[6] & 0x03) as usize + 1,
    })
}

/// One walked ADTS frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AdtsFrame {
    /// Byte offset of the frame's first byte.
    pub offset: usize,
    /// The parsed fixed header.
    pub header: AdtsHeader,
}

/// Walk complete ADTS frames from `start`, resyncing past garbage the
/// way `mp3::header::walk_frames` does: a position advances to a
/// candidate sync only if the full header validates AND the frame
/// fits in the remaining bytes (a trailing partial frame is left out,
/// matching ffmpeg's end-of-stream behavior — trailing garbage never
/// fails the walk).
///
/// The walk is bounded by the input length; no allocation beyond the
/// returned frame list.
#[must_use]
pub(crate) fn walk_adts_frames(data: &[u8], start: usize) -> Vec<AdtsFrame> {
    let mut frames = Vec::new();
    let mut pos = start.min(data.len());
    while pos + 7 <= data.len() {
        // Cheap sync scan: 0xFF then high nibble 0xF.
        if data[pos] != 0xFF || data[pos + 1] >> 4 != 0xF {
            pos += 1;
            continue;
        }
        let header = match parse_adts_header(&data[pos..]) {
            Ok(h) => h,
            // A validated-then-failing header (bad layer/index/length)
            // is garbage at this position — resync one byte forward.
            Err(_) => {
                pos += 1;
                continue;
            }
        };
        let Some(end) = pos.checked_add(header.frame_len) else {
            break;
        };
        if end > data.len() {
            // Trailing partial frame: stop here (honest, like ffmpeg).
            break;
        }
        frames.push(AdtsFrame {
            offset: pos,
            header,
        });
        pos = end;
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One hand-built ADTS header frame body for parse tests: MPEG-4,
    /// no CRC, LC, 44.1 kHz, stereo, frame 64 B, 1 raw block.
    fn lc_stereo_441(len: usize) -> Vec<u8> {
        let mut h = vec![0xFF, 0xF1]; // sync | MPEG-4, layer 0, no CRC
        h.push((1 << 6) | (4 << 2)); // profile=LC, sf_idx=4 (44100)
        // channel_config=2 (stereo: b2 bit0 = 0, b3 top 2 bits = 2)
        // + the frame-length high 2 bits.
        h.push((2 << 6) | ((len >> 11) & 0x03) as u8);
        h.push(((len >> 3) & 0xFF) as u8);
        h.push((((len & 0x07) as u8) << 5) | 0x1F); // len lo + fullness hi
        // fullness low 6 bits (VBR 0x7FF) | raw blocks field = 0 (1 block).
        h.push(0x3F_u8 << 2);
        h.resize(len, 0);
        h
    }

    #[test]
    fn header_fields_parse_exactly() {
        let data = lc_stereo_441(64);
        let h = match parse_adts_header(&data) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(h.profile, AdtsProfile::Lc);
        assert_eq!(h.sampling_index, 4);
        assert_eq!(h.sample_rate, 44_100);
        assert_eq!(h.channel_config, 2);
        assert_eq!(h.channels, 2);
        assert_eq!(h.frame_len, 64);
        assert_eq!(h.header_len, 7);
        assert_eq!(h.raw_blocks, 1);
        assert!(!h.mpeg2);
        assert!(h.protection_absent);
    }

    #[test]
    fn channel_config_table_resolves_the_documented_counts() {
        assert_eq!(channel_count(0), 0);
        assert_eq!(channel_count(1), 1);
        assert_eq!(channel_count(2), 2);
        assert_eq!(channel_count(6), 6);
        assert_eq!(channel_count(7), 8);
    }

    #[test]
    fn bad_sync_layer_index_and_short_buffers_are_typed() {
        let e = match parse_adts_header(&[0x00; 7]) {
            Err(e) => e,
            Ok(_) => unreachable!("must be a bad syncword"),
        };
        assert_eq!(
            e,
            corrupt(
                "aac.adts.sync",
                CorruptDetail::InvalidStructure { offset: Some(0) },
                "syncword is not 0xFFF"
            )
        );
        let e = match parse_adts_header(&[0xFF, 0xF3, 0x10, 0x80, 0, 0, 0]) {
            Err(e) => e,
            Ok(_) => unreachable!("layer 1 must be refused"),
        };
        assert_eq!(
            e,
            corrupt(
                "aac.adts.layer",
                CorruptDetail::InvalidStructure { offset: Some(1) },
                "layer field is 1, must be 0"
            )
        );
        let e = match parse_adts_header(&[0xFF, 0xF1, 0x34, 0x80, 0, 0, 0]) {
            Err(e) => e,
            Ok(_) => unreachable!("sf index 13 must be refused"),
        };
        assert_eq!(
            e,
            corrupt(
                "aac.adts.sampling_index",
                CorruptDetail::InvalidStructure { offset: Some(2) },
                "sampling-frequency index 13 is reserved"
            )
        );
        let e = match parse_adts_header(&[0xFF, 0xF1]) {
            Err(e) => e,
            Ok(_) => unreachable!("2 bytes must be refused"),
        };
        assert_eq!(
            e,
            corrupt(
                "aac.adts.truncated",
                CorruptDetail::Truncated {
                    expected: 7,
                    found: 2
                },
                "an ADTS fixed header needs 7 bytes"
            )
        );
    }

    #[test]
    fn walk_resyncs_past_garbage_and_drops_the_trailing_partial() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0xDE, 0xAD]); // leading garbage
        data.extend_from_slice(&lc_stereo_441(64));
        data.extend_from_slice(&[0x00; 5]); // inter-frame garbage
        data.extend_from_slice(&lc_stereo_441(64));
        data.extend_from_slice(&lc_stereo_441(64)[..30]); // partial tail
        let frames = walk_adts_frames(&data, 0);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].offset, 2);
        assert_eq!(frames[1].offset, 2 + 64 + 5);
    }
}
