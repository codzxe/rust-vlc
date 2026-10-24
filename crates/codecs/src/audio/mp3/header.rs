//! MPEG-audio frame headers (card 5.A3): the sync scan, the
//! version/layer/bitrate/samplerate tables, and the frame walker that
//! turns a byte stream into exact one-frame packets.
//!
//! This is the module's framing truth — the engine
//! (`symphonia-bundle-mp3`) hard-requires one packet to be exactly one
//! frame (`header.frame_size == packet bytes remaining`), so every
//! offset the decoder consumes is computed HERE, from the ISO/IEC
//! 11172-3 / 13818-3 tables below, not guessed anywhere else.
//!
//! # Frame length arithmetic (the spec formulas)
//!
//! - Layer I: `frame_size = (12 * bitrate / sample_rate + padding) * 4`
//!   (slots of 4 bytes), 384 samples/frame.
//! - Layer II: `frame_size = 144 * bitrate / sample_rate + padding`,
//!   1152 samples/frame.
//! - Layer III: same as II, but 1152 samples/frame at MPEG-1 and
//!   576 at MPEG-2/2.5 (so the coefficient is 72 there).
//!
//! Bitrates are tabulated per (version, layer); samplerates per
//! (version, index) with the reserved `11` index rejected. The
//! free-format index `0000` is a typed refusal (doc 04 §3.2's known
//! `[VERIFY-P0]` gap; guessing a free-format bitrate is never allowed).

use super::error::{corrupt, unsupported};
use media_types::EngineError;
use media_types::error::CorruptDetail;

/// MPEG audio version from the header's version bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpegVersion {
    /// MPEG-2.5 (extension, 00 bits).
    V2_5,
    /// Reserved (01 bits) — never a valid stream, rejected at parse.
    Reserved,
    /// MPEG-2 (10 bits).
    V2,
    /// MPEG-1 (11 bits).
    V1,
}

/// MPEG audio layer from the header's layer bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Layer I.
    L1,
    /// Layer II (MP2).
    L2,
    /// Layer III (MP3).
    L3,
    /// Reserved (00 bits) — rejected at parse.
    Reserved,
}

/// Channel mode from the header's mode bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    /// Stereo (00).
    Stereo,
    /// Joint stereo (01) — the mode extension field is meaningful.
    JointStereo,
    /// Dual channel (10).
    DualChannel,
    /// Mono (11).
    Mono,
}

/// De-emphasis indication from the header's emphasis bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    /// None (00).
    None,
    /// 50/15 ms (01).
    Ms50_15,
    /// Reserved (10) — rejected at parse.
    Reserved,
    /// CCITT J.17 (11).
    CcitJ17,
}

/// One parsed MPEG-audio frame header (the first 4 bytes of a frame,
/// fully validated).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// MPEG version.
    pub version: MpegVersion,
    /// Layer.
    pub layer: Layer,
    /// True when a CRC-16 word follows the header (protection bit 0).
    pub protected_by_crc: bool,
    /// Bitrate in bits/second (from the tables, never free-format).
    pub bitrate_bps: u32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Padding slot flag (adds one slot to the frame size).
    pub padding: bool,
    /// Private bit.
    pub private: bool,
    /// Channel mode.
    pub channel_mode: ChannelMode,
    /// Mode extension (2 bits; joint-stereo intensity/mode flags).
    pub mode_extension: u8,
    /// Copyright bit.
    pub copyright: bool,
    /// Original-media bit.
    pub original: bool,
    /// De-emphasis.
    pub emphasis: Emphasis,
    /// The frame's byte length INCLUDING the 4 header bytes.
    pub frame_size: u32,
    /// Samples per frame per channel (granule-combined).
    pub samples_per_frame: u32,
}

/// Bitrate tables: `[version_group][layer_group][index-1]`.
/// version groups: 0 = MPEG-1, 1 = MPEG-2/2.5; layer groups per table.
/// Index 0 (free) and 15 (bad) are excluded from the 14-entry rows.
const BITRATES_V1_L1: [u32; 14] = [
    32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
];
const BITRATES_V1_L2: [u32; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
];
const BITRATES_V1_L3: [u32; 14] = [
    32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];
const BITRATES_V2_L1: [u32; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
];
/// MPEG-2/2.5 Layer II and Layer III share the same table.
const BITRATES_V2_L23: [u32; 14] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

/// Sample-rate tables: `[version][index]`; index 3 is reserved.
const SAMPLE_RATES: [[u32; 3]; 4] = [
    [11_025, 12_000, 8_000],  // MPEG-2.5
    [0, 0, 0],                // reserved version — unreachable
    [22_050, 24_000, 16_000], // MPEG-2
    [44_100, 48_000, 32_000], // MPEG-1
];

/// The bitrate for a header's (version, layer, index), or `None` when
/// the index is free-format (0) or invalid (15).
fn table_bitrate(version: MpegVersion, layer: Layer, index: u8) -> Option<u32> {
    let row: &[u32; 14] = match (version, layer) {
        (MpegVersion::V1, Layer::L1) => &BITRATES_V1_L1,
        (MpegVersion::V1, Layer::L2) => &BITRATES_V1_L2,
        (MpegVersion::V1, Layer::L3) => &BITRATES_V1_L3,
        (MpegVersion::V2, Layer::L1) | (MpegVersion::V2_5, Layer::L1) => &BITRATES_V2_L1,
        (MpegVersion::V2, Layer::L2)
        | (MpegVersion::V2, Layer::L3)
        | (MpegVersion::V2_5, Layer::L2)
        | (MpegVersion::V2_5, Layer::L3) => &BITRATES_V2_L23,
        // Reserved version/layer never reach the table (rejected first).
        (MpegVersion::Reserved, _) | (_, Layer::Reserved) => return None,
    };
    if index == 0 || index > 14 {
        return None;
    }
    row.get(usize::from(index) - 1).copied()
}

/// The sample rate for a header's (version, index), or `None` when
/// the index is the reserved `11`.
fn table_sample_rate(version: MpegVersion, index: u8) -> Option<u32> {
    let row = match version {
        MpegVersion::V1 => &SAMPLE_RATES[3],
        MpegVersion::V2 => &SAMPLE_RATES[2],
        MpegVersion::V2_5 => &SAMPLE_RATES[0],
        MpegVersion::Reserved => return None,
    };
    if index > 2 {
        return None;
    }
    row.get(usize::from(index)).copied()
}

/// Channel count for a mode (dual channel = 2).
#[must_use]
pub fn channel_count(mode: ChannelMode) -> u32 {
    match mode {
        ChannelMode::Mono => 1,
        ChannelMode::Stereo | ChannelMode::JointStereo | ChannelMode::DualChannel => 2,
    }
}

/// Samples per frame per channel for (version, layer).
#[must_use]
pub fn samples_per_frame(version: MpegVersion, layer: Layer) -> u32 {
    match (version, layer) {
        (_, Layer::L1) => 384,
        (_, Layer::L2) => 1152,
        (MpegVersion::V1, Layer::L3) => 1152,
        // MPEG-2/2.5 Layer III halves the granule count.
        (MpegVersion::V2, Layer::L3) | (MpegVersion::V2_5, Layer::L3) => 576,
        (MpegVersion::Reserved, _) | (_, Layer::Reserved) => 0,
    }
}

/// The byte length of one frame from its validated fields (the spec
/// formulas in the module docs; `frame_size` includes the 4 header
/// bytes).
fn compute_frame_size(
    version: MpegVersion,
    layer: Layer,
    bitrate_bps: u32,
    sample_rate: u32,
    padding: bool,
) -> Option<u32> {
    if sample_rate == 0 || bitrate_bps == 0 {
        return None;
    }
    let padding_u32 = u32::from(padding);
    let size = match layer {
        Layer::L1 => {
            (12u64 * u64::from(bitrate_bps) / u64::from(sample_rate) + u64::from(padding_u32)) * 4
        }
        Layer::L2 => {
            // Layer II is 1152 samples/frame in every version: 144.
            144 * u64::from(bitrate_bps) / u64::from(sample_rate) + u64::from(padding_u32)
        }
        Layer::L3 => {
            // 1152 samples over 8 bits per byte at MPEG-1 = 144;
            // 576 at MPEG-2/2.5 = 72.
            let coeff = if matches!(version, MpegVersion::V1) {
                144
            } else {
                72
            };
            coeff * u64::from(bitrate_bps) / u64::from(sample_rate) + u64::from(padding_u32)
        }
        Layer::Reserved => return None,
    };
    u32::try_from(size).ok()
}

/// Parse the 4-byte MPEG-audio frame header at the front of `data`.
///
/// `Err` carries the exact rule that failed (typed, per the card's
/// error-path list): bad sync, reserved version, reserved layer,
/// bad bitrate index (free format is a distinct typed refusal),
/// bad samplerate index, reserved emphasis, or a header shorter
/// than 4 bytes.
pub fn parse_frame_header(data: &[u8]) -> Result<FrameHeader, EngineError> {
    let bytes: [u8; 4] = match data.get(..4) {
        Some(b) => {
            let mut out = [0u8; 4];
            out.copy_from_slice(b);
            out
        }
        None => {
            return Err(corrupt(
                "mp3.header.short",
                CorruptDetail::Truncated {
                    expected: 4,
                    found: data.len() as u64,
                },
                "frame header needs 4 bytes",
            )
            .into_engine_error());
        }
    };
    // Sync: 11 set bits (0xFFE).
    if bytes[0] != 0xFF || bytes[1] & 0xE0 != 0xE0 {
        return Err(corrupt(
            "mp3.header.sync",
            CorruptDetail::InvalidStructure { offset: Some(0) },
            "frame sync (11 set bits) not present",
        )
        .into_engine_error());
    }
    let version = match (bytes[1] >> 3) & 0x3 {
        0 => MpegVersion::V2_5,
        1 => MpegVersion::Reserved,
        2 => MpegVersion::V2,
        _ => MpegVersion::V1,
    };
    if version == MpegVersion::Reserved {
        return Err(corrupt(
            "mp3.header.version_reserved",
            CorruptDetail::InvalidStructure { offset: Some(1) },
            "version bits 01 are reserved",
        )
        .into_engine_error());
    }
    let layer = match (bytes[1] >> 1) & 0x3 {
        0 => Layer::Reserved,
        1 => Layer::L3,
        2 => Layer::L2,
        _ => Layer::L1,
    };
    if layer == Layer::Reserved {
        return Err(corrupt(
            "mp3.header.layer_reserved",
            CorruptDetail::InvalidStructure { offset: Some(1) },
            "layer bits 00 are reserved",
        )
        .into_engine_error());
    }
    let protected_by_crc = bytes[1] & 0x1 == 0;
    let bitrate_index = bytes[2] >> 4;
    let bitrate_bps = match table_bitrate(version, layer, bitrate_index) {
        Some(b) => b * 1000,
        None if bitrate_index == 0 => {
            // Free-format: doc 04 §3.2's known gap — refuse, never guess.
            return Err(unsupported(
                "mp3.free_format",
                "free-format bitrate (index 0) is not supported; the \
                 bitrate would have to be measured, not read",
            )
            .into_engine_error());
        }
        None => {
            return Err(corrupt(
                "mp3.header.bad_bitrate",
                CorruptDetail::InvalidStructure { offset: Some(2) },
                format!("bitrate index {bitrate_index} is invalid for {version:?}/{layer:?}"),
            )
            .into_engine_error());
        }
    };
    let sample_rate = match table_sample_rate(version, (bytes[2] >> 2) & 0x3) {
        Some(r) => r,
        None => {
            return Err(corrupt(
                "mp3.header.bad_samplerate",
                CorruptDetail::InvalidStructure { offset: Some(2) },
                "samplerate index 3 is reserved",
            )
            .into_engine_error());
        }
    };
    let emphasis = match bytes[3] & 0x3 {
        0 => Emphasis::None,
        1 => Emphasis::Ms50_15,
        2 => Emphasis::Reserved,
        _ => Emphasis::CcitJ17,
    };
    if emphasis == Emphasis::Reserved {
        return Err(corrupt(
            "mp3.header.emphasis_reserved",
            CorruptDetail::InvalidStructure { offset: Some(3) },
            "emphasis bits 10 are reserved",
        )
        .into_engine_error());
    }
    let channel_mode = match (bytes[3] >> 6) & 0x3 {
        0 => ChannelMode::Stereo,
        1 => ChannelMode::JointStereo,
        2 => ChannelMode::DualChannel,
        _ => ChannelMode::Mono,
    };
    let spf = samples_per_frame(version, layer);
    let padding = bytes[2] & 0x2 != 0;
    let frame_size = match compute_frame_size(version, layer, bitrate_bps, sample_rate, padding) {
        Some(s) if s >= 4 => s,
        _ => {
            return Err(corrupt(
                "mp3.header.frame_size",
                CorruptDetail::InvalidStructure { offset: Some(2) },
                "frame size arithmetic collapsed (bitrate or rate too small)",
            )
            .into_engine_error());
        }
    };
    Ok(FrameHeader {
        version,
        layer,
        protected_by_crc,
        bitrate_bps,
        sample_rate,
        padding,
        private: bytes[2] & 0x1 != 0,
        channel_mode,
        mode_extension: (bytes[3] >> 4) & 0x3,
        copyright: bytes[3] & 0x8 != 0,
        original: bytes[3] & 0x4 != 0,
        emphasis,
        frame_size,
        samples_per_frame: spf,
    })
}

/// Find the next offset `>= from` where a parseable frame header
/// starts (bad-sync recovery: the scan-and-verify loop that keeps the
/// walker alive across garbage).
///
/// Returns `None` when the remaining bytes hold no header. A candidate
/// that syncs but fails validation is skipped (that is the recovery);
/// free-format headers are NOT treated as sync points (they cannot be
/// framed by table arithmetic).
#[must_use]
pub fn scan_sync(data: &[u8], from: usize) -> Option<usize> {
    let mut pos = from;
    while pos + 4 <= data.len() {
        if data[pos] == 0xFF
            && data[pos + 1] & 0xE0 == 0xE0
            && parse_frame_header(&data[pos..]).is_ok()
        {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

/// One walked frame: its absolute byte offset and validated header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameAt {
    /// Absolute byte offset of the frame's first byte.
    pub offset: usize,
    /// The validated header.
    pub header: FrameHeader,
}

/// Walk the frames in `data` starting at byte `start`.
///
/// Pure table arithmetic: each step validates the header, consumes
/// exactly `header.frame_size` bytes, and on ANY parse failure scans
/// forward to the next parseable header (resync — the card's
/// bad-sync-recovery requirement; also how trailing ID3v1/garbage is
/// skipped). Truncated tails yield no frame. The walk is bounded by
/// `data.len()` — no loops on any input.
#[must_use]
pub fn walk_frames(data: &[u8], start: usize) -> Vec<FrameAt> {
    let mut frames = Vec::new();
    let mut pos = start;
    while pos + 4 <= data.len() {
        let header = match parse_frame_header(&data[pos..]) {
            Ok(h) => h,
            Err(_) => {
                // Resync: scan to the next parseable header.
                match scan_sync(data, pos + 1) {
                    Some(next) if next > pos => {
                        pos = next;
                        continue;
                    }
                    _ => break,
                }
            }
        };
        let end = pos.saturating_add(header.frame_size as usize);
        if end > data.len() {
            // Truncated final frame: not a frame, stop.
            break;
        }
        frames.push(FrameAt {
            offset: pos,
            header,
        });
        pos = end;
    }
    frames
}

/// The Layer III side-information byte size for (version, channels) —
/// where the Xing/Info/VBRI payload starts after `4 + crc`.
#[must_use]
pub fn side_info_size(version: MpegVersion, channels: u32) -> usize {
    let mono = channels == 1;
    match version {
        MpegVersion::V1 => {
            if mono {
                17
            } else {
                32
            }
        }
        MpegVersion::V2 | MpegVersion::V2_5 => {
            if mono {
                9
            } else {
                17
            }
        }
        MpegVersion::Reserved => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A canonical MPEG-1 Layer III 128 kbps 44.1 kHz stereo header.
    const MP3_128K_44100: [u8; 4] = [0xFF, 0xFB, 0x90, 0x00];

    #[test]
    fn parses_the_canonical_header() {
        let h = match parse_frame_header(&MP3_128K_44100) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(h.version, MpegVersion::V1);
        assert_eq!(h.layer, Layer::L3);
        assert_eq!(h.bitrate_bps, 128_000);
        assert_eq!(h.sample_rate, 44_100);
        assert_eq!(h.channel_mode, ChannelMode::Stereo);
        assert_eq!(h.samples_per_frame, 1152);
        assert!(!h.protected_by_crc);
        // 144 * 128000 / 44100 = 417 (floor).
        assert_eq!(h.frame_size, 417);
    }

    #[test]
    fn layer_ii_and_mpeg2_lengths_match_the_spec_formulas() {
        // MPEG-1 Layer II 384 kbps 48 kHz stereo: 144*384000/48000=1152.
        let mp2 = [0xFF, 0xFC, 0xE4, 0x00];
        let h = match parse_frame_header(&mp2) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(
            (h.layer, h.bitrate_bps, h.sample_rate),
            (Layer::L2, 384_000, 48_000)
        );
        assert_eq!(h.frame_size, 1152);
        // MPEG-2 Layer III 64 kbps 24 kHz: 72*64000/24000=192.
        let m2 = [0xFF, 0xF3, 0x84, 0x00];
        let h2 = match parse_frame_header(&m2) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!((h2.version, h2.samples_per_frame), (MpegVersion::V2, 576));
        assert_eq!(h2.frame_size, 192);
        // Layer I 448 kbps 44.1 kHz: (12*448000/44100)*4 = 484.
        let l1 = [0xFF, 0xFE, 0xE0, 0x00];
        let h1 = match parse_frame_header(&l1) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(h1.layer, Layer::L1);
        assert_eq!(h1.frame_size, 484);
        assert_eq!(h1.samples_per_frame, 384);
    }

    #[test]
    fn mode_extension_emphasis_and_crc_bits_round_trip() {
        // CRC on (protection bit 0), joint stereo, mode ext 01,
        // not copyright, original, 50/15ms emphasis.
        let hdr = [0xFF, 0xFA, 0x90, 0x55];
        let h = match parse_frame_header(&hdr) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert!(h.protected_by_crc);
        assert_eq!(h.channel_mode, ChannelMode::JointStereo);
        assert_eq!(h.mode_extension, 1);
        assert!(h.original);
        assert_eq!(h.emphasis, Emphasis::Ms50_15);
        assert!(!h.copyright);
    }

    #[test]
    fn error_paths_are_each_typed() {
        // Bad sync.
        let e = match parse_frame_header(&[0x00, 0xFB, 0x90, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("bad sync must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.sync");
        // Reserved version bits (01): 1110 1100.
        let e = match parse_frame_header(&[0xFF, 0xEC, 0x90, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("reserved version must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.version_reserved");
        // Reserved layer bits (00).
        let e = match parse_frame_header(&[0xFF, 0xF1, 0x90, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("reserved layer must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.layer_reserved");
        // Free-format bitrate index 0000 — typed refusal, not structure.
        let e = match parse_frame_header(&[0xFF, 0xFB, 0x00, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("free format must be refused"),
        };
        assert_eq!(e.context.code, "mp3.free_format");
        // Invalid bitrate index 1111.
        let e = match parse_frame_header(&[0xFF, 0xFB, 0xF0, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("bitrate 15 must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.bad_bitrate");
        // Reserved samplerate index 11.
        let e = match parse_frame_header(&[0xFF, 0xFB, 0x9C, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("samplerate 3 must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.bad_samplerate");
        // Reserved emphasis bits (10).
        let e = match parse_frame_header(&[0xFF, 0xFB, 0x90, 0x02]) {
            Err(e) => e,
            Ok(_) => unreachable!("emphasis 10 must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.emphasis_reserved");
        // Truncated header.
        let e = match parse_frame_header(&[0xFF, 0xFB]) {
            Err(e) => e,
            Ok(_) => unreachable!("short header must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.short");
    }

    #[test]
    fn scan_sync_recovers_past_garbage() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 3]);
        data.extend_from_slice(&MP3_128K_44100);
        data.extend_from_slice(&[0u8; 7]);
        let hit = match scan_sync(&data, 0) {
            Some(p) => p,
            None => unreachable!("sync must be found"),
        };
        assert_eq!(hit, 3);
        assert_eq!(scan_sync(&data, 3 + 417), None);
    }

    #[test]
    fn walk_frames_resyncs_and_drops_a_truncated_tail() {
        let mut data = Vec::new();
        let push = |data: &mut Vec<u8>| {
            let h = match parse_frame_header(&MP3_128K_44100) {
                Ok(h) => h,
                Err(e) => unreachable!("{e:?}"),
            };
            data.extend_from_slice(&MP3_128K_44100);
            data.resize(data.len() + h.frame_size as usize - 4, 0xA5);
        };
        push(&mut data);
        // Garbage that once fakes a sync (bad bitrate index).
        data.extend_from_slice(&[0xFF, 0xFB, 0xF0, 0x00, 0, 0, 0, 0]);
        push(&mut data);
        let frames = walk_frames(&data, 0);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].offset, 0);
        assert_eq!(frames[1].offset, 417 + 8);
        // A truncated final frame yields nothing (frame 1 needed the
        // removed tail bytes to reach its header-declared length).
        data.truncate(data.len() - 10);
        assert_eq!(walk_frames(&data, 0).len(), 1);
    }

    #[test]
    fn padding_bit_adds_one_slot() {
        let padded = [0xFF, 0xFB, 0x92, 0x00];
        let h = match parse_frame_header(&padded) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert!(h.padding);
        assert_eq!(h.frame_size, 418);
    }
}
