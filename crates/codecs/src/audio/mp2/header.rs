//! MP2 (MPEG audio Layer II) frame headers (card 5.J3): the Layer II
//! gate, the bitrate tables, the mode-extension semantics, and the
//! walk that turns a byte stream into exact one-frame packets plus
//! the CBR geometry the seek arithmetic consumes.
//!
//! This is a **thin sibling of the family module** ([`super`] is
//! `audio::mp3`, card 5.A3 — landed): the 4-byte header parse, the
//! sync scan and the resync-on-garbage walker are the MPEG-audio
//! family's shared truth and are NOT duplicated here. What this
//! module owns is everything Layer II-specific:
//!
//! - the **Layer II gate** ([`parse_mp2_header`]): a header that
//!   parses as the family but is not Layer II is a typed corruption
//!   (`mp2.header.not_layer_ii`), never silently decoded;
//! - the **Layer II bitrate tables** (ISO/IEC 11172-3 §B.2 for
//!   MPEG-1, 13818-3 §B.1 for MPEG-2/2.5 LSF) exposed for probe /
//!   registry use, pinned against the family parser by unit test;
//! - the **mode extension** under Layer II semantics — unlike Layer
//!   III's 2+2 intensity/ms flags, the Layer II bits encode the
//!   intensity-stereo *bound subband* (4/8/12/16);
//! - the **CBR geometry** ([`Mp2Geometry`]): Layer II streams are CBR
//!   by construction (fixed table bitrate per frame; there is no
//!   Xing/Info/VBRI construction in Layer II), so the walk also
//!   verifies that every frame agrees on version/bitrate/rate/
//!   channels — a disagreement is a typed refusal, because the seek
//!   arithmetic below would otherwise lie.
//!
//! # Frame length arithmetic (the spec formula)
//!
//! Layer II carries 1152 samples/frame in every MPEG version, so
//! `frame_size = 144 × bitrate / sample_rate + padding` — the family
//! module computes it; the unit tests here pin it on real corpus
//! headers.

use super::error::corrupt;
use crate::audio::mp3::{
    ChannelMode, FrameAt, FrameHeader, Layer, MpegVersion, channel_count, parse_frame_header,
    walk_frames as family_walk,
};
use media_types::EngineError;
use media_types::error::CorruptDetail;

/// Layer II bitrate table, MPEG-1 (ISO/IEC 11172-3 §B.2), kbps, index
/// 1..=14 (index 0 = free format, index 15 = bad — both refused by
/// the family parser before this table is consulted).
pub const LAYER_II_BITRATES_MPEG1: [u32; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
];

/// Layer II bitrate table, MPEG-2/2.5 LSF (ISO/IEC 13818-3 §B.1),
/// kbps — shared by MPEG-2 and MPEG-2.5.
pub const LAYER_II_BITRATES_MPEG2_LSF: [u32; 14] =
    [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

/// The Layer II bitrate table for `version`, or `None` for the
/// reserved version bits (never reachable through the family parser).
#[must_use]
pub fn layer_ii_bitrate_table(version: MpegVersion) -> Option<&'static [u32; 14]> {
    match version {
        MpegVersion::V1 => Some(&LAYER_II_BITRATES_MPEG1),
        MpegVersion::V2 | MpegVersion::V2_5 => Some(&LAYER_II_BITRATES_MPEG2_LSF),
        MpegVersion::Reserved => None,
    }
}

/// The Layer II bitrate for a header's (version, index-1), kbps.
/// `None` when the index is free-format (0) or invalid (15).
#[must_use]
pub fn layer_ii_bitrate(version: MpegVersion, index: u8) -> Option<u32> {
    if index == 0 || index > 14 {
        return None;
    }
    layer_ii_bitrate_table(version).and_then(|row| row.get(usize::from(index) - 1).copied())
}

/// The Layer II mode extension (ISO/IEC 11172-3 §2.4.2.3, "mode
/// extension"): the subband above which the joint-stereo frames
/// switch to intensity stereo. Bits `00`→4, `01`→8, `10`→12, `11`→16.
///
/// Outside joint stereo the 2 bits carry no meaning (the spec marks
/// them N/A) — [`intensity_bound`] makes that explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerIiModeExtension {
    /// Intensity stereo above subband 4 (bits `00`).
    Bound4,
    /// Intensity stereo above subband 8 (bits `01`).
    Bound8,
    /// Intensity stereo above subband 12 (bits `10`).
    Bound12,
    /// Intensity stereo above subband 16 (bits `11`).
    Bound16,
}

impl LayerIiModeExtension {
    /// Decode the raw 2 header bits.
    #[must_use]
    pub fn from_bits(bits: u8) -> LayerIiModeExtension {
        match bits & 0x3 {
            0 => LayerIiModeExtension::Bound4,
            1 => LayerIiModeExtension::Bound8,
            2 => LayerIiModeExtension::Bound12,
            _ => LayerIiModeExtension::Bound16,
        }
    }

    /// The bound subband: subbands `bound..32` run intensity stereo,
    /// `0..bound` run plain stereo.
    #[must_use]
    pub fn bound_subband(self) -> u8 {
        match self {
            LayerIiModeExtension::Bound4 => 4,
            LayerIiModeExtension::Bound8 => 8,
            LayerIiModeExtension::Bound12 => 12,
            LayerIiModeExtension::Bound16 => 16,
        }
    }
}

/// The intensity-stereo bound for a header, or `None` when the mode
/// is not joint stereo (the bits are meaningless there, and honest
/// callers must not read a bound out of them).
#[must_use]
pub fn intensity_bound(mode: ChannelMode, ext: LayerIiModeExtension) -> Option<u8> {
    match mode {
        ChannelMode::JointStereo => Some(ext.bound_subband()),
        _ => None,
    }
}

/// One parsed MP2 (Layer II) frame header: the validated family
/// header plus the Layer II mode-extension decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp2FrameHeader {
    /// The validated MPEG-audio family header (bitrate/samplerate/
    /// channel-mode/framing arithmetic — [`crate::audio::mp3`]).
    pub family: FrameHeader,
    /// The mode extension under Layer II semantics.
    pub mode_extension: LayerIiModeExtension,
}

/// Parse the 4-byte frame header at the front of `data` and apply the
/// Layer II gate.
///
/// `Err` carries the exact rule that failed (typed): every family
/// failure (bad sync, reserved version/layer, free-format or invalid
/// bitrate, reserved samplerate, reserved emphasis, short header)
/// passes through with its family code, and a non-Layer II layer adds
/// `mp2.header.not_layer_ii`.
pub fn parse_mp2_header(data: &[u8]) -> Result<Mp2FrameHeader, EngineError> {
    let family = parse_frame_header(data)?;
    if family.layer != Layer::L2 {
        let which = match family.layer {
            Layer::L1 => "Layer I",
            Layer::L3 => "Layer III",
            Layer::L2 | Layer::Reserved => "reserved",
        };
        return Err(corrupt(
            "mp2.header.not_layer_ii",
            CorruptDetail::InvalidStructure { offset: Some(1) },
            format!("layer bits decode to {which} — this decoder is Layer II (mp2) only"),
        )
        .into_engine_error());
    }
    Ok(Mp2FrameHeader {
        mode_extension: LayerIiModeExtension::from_bits(family.mode_extension),
        family,
    })
}

/// One walked MP2 frame: its absolute byte offset and header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp2FrameAt {
    /// Absolute byte offset of the frame's first byte.
    pub offset: usize,
    /// The validated Layer II header.
    pub header: Mp2FrameHeader,
}

/// Walk the Layer II frames in `data` starting at byte `start`.
///
/// Delegates framing and resync to the family walker (each step
/// consumes exactly `header.frame_size` bytes; garbage is scanned
/// past; truncated tails yield no frame), then applies the Layer II
/// gate to every walked frame: the first non-Layer II frame is a
/// typed `mp2.stream.layer_mismatch` with its offset. Zero frames is
/// `Ok([])` — the caller decides whether that is damage.
pub fn walk_mp2_frames(data: &[u8], start: usize) -> Result<Vec<Mp2FrameAt>, EngineError> {
    let family: Vec<FrameAt> = family_walk(data, start);
    let mut out = Vec::with_capacity(family.len());
    for frame in family {
        if frame.header.layer != Layer::L2 {
            return Err(corrupt(
                "mp2.stream.layer_mismatch",
                CorruptDetail::InvalidStructure {
                    offset: Some(frame.offset as u64),
                },
                format!(
                    "frame at byte {} decodes as a non-Layer II header — \
                     an mp2 stream must be uniformly Layer II",
                    frame.offset
                ),
            )
            .into_engine_error());
        }
        out.push(Mp2FrameAt {
            offset: frame.offset,
            header: Mp2FrameHeader {
                mode_extension: LayerIiModeExtension::from_bits(frame.header.mode_extension),
                family: frame.header,
            },
        });
    }
    Ok(out)
}

/// The CBR geometry of a walked Layer II frame list.
///
/// Layer II streams are CBR by construction: every frame reads its
/// bitrate from the same (version, layer) table row and there is no
/// Xing/Info/VBRI header construction in Layer II (those are Layer
/// III encoder side-channels; VBRI is Fraunhofer's Layer III thing).
/// [`geometry`] verifies exactly that: version, bitrate, sample rate
/// and channel count must agree across all frames, else the typed
/// `mp2.stream.cbr_mismatch` — the seek arithmetic in [`super::seek`]
/// divides by these numbers and must never divide by a guess.
///
/// The **padding bit** is the one permitted per-frame variation: at
/// 44.1/32/22.05/16 kHz (and LSF halves) the nominal frame size is
/// fractional, and the encoder distributes the remainder as +1-slot
/// padded frames. `frame_size` here is the *unpadded nominal* size;
/// byte-exact positions need [`super::seek::refine_to_frame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp2Geometry {
    /// MPEG version (V1, V2 or V2.5).
    pub version: MpegVersion,
    /// Bitrate in bits/second (constant across the stream).
    pub bitrate_bps: u32,
    /// Sample rate in Hz (constant).
    pub sample_rate: u32,
    /// Channel count (constant; dual channel counts 2).
    pub channels: u32,
    /// Always 1152 for Layer II.
    pub samples_per_frame: u32,
    /// Nominal (unpadded) frame size in bytes, header included.
    pub frame_size: u32,
    /// Byte offset of the first frame.
    pub first_offset: u64,
    /// Walked frame count.
    pub total_frames: u64,
    /// `total_frames × 1152` per-channel samples.
    pub total_samples: u64,
}

/// Verify the CBR consistency of `frames` and distill the geometry.
///
/// # Errors
/// Typed `mp2.stream.cbr_mismatch` when any frame disagrees with the
/// first on version/bitrate/rate/channels (the offset of the offender
/// rides in the detail). An empty list yields `None` (the caller owns
/// the "no frames" damage report).
pub fn geometry(frames: &[Mp2FrameAt]) -> Result<Option<Mp2Geometry>, EngineError> {
    let Some(first) = frames.first() else {
        return Ok(None);
    };
    let h0 = first.header.family;
    for frame in frames {
        let h = frame.header.family;
        if (h.version, h.bitrate_bps, h.sample_rate, h.channel_mode)
            != (h0.version, h0.bitrate_bps, h0.sample_rate, h0.channel_mode)
        {
            return Err(corrupt(
                "mp2.stream.cbr_mismatch",
                CorruptDetail::InvalidStructure {
                    offset: Some(frame.offset as u64),
                },
                format!(
                    "frame at byte {} disagrees with the first frame's geometry \
                     (Layer II is CBR by construction; the seek arithmetic refuses a guess)",
                    frame.offset
                ),
            )
            .into_engine_error());
        }
    }
    let nominal = h0.frame_size - u32::from(h0.padding);
    Ok(Some(Mp2Geometry {
        version: h0.version,
        bitrate_bps: h0.bitrate_bps,
        sample_rate: h0.sample_rate,
        channels: channel_count(h0.channel_mode),
        samples_per_frame: h0.samples_per_frame,
        frame_size: nominal,
        first_offset: first.offset as u64,
        total_frames: frames.len() as u64,
        total_samples: frames.len() as u64 * u64::from(h0.samples_per_frame),
    }))
}

/// Whether `data` begins like a raw MP2 stream: an ID3v2 tag or a
/// parseable Layer II frame header at offset 0 (the probe the future
/// MP2 demuxer / registry row will call).
#[must_use]
pub fn is_mp2_stream(data: &[u8]) -> bool {
    if data.get(..3) == Some(b"ID3") {
        // An ID3 tag could hide anything; the probe stays honest by
        // demanding a Layer II header right after the tag.
        return match crate::audio::mp3::id3v2_len(data) {
            Ok(Some(len)) => {
                let start = len.min(data.len());
                parse_mp2_header(data.get(start..).unwrap_or(&[])).is_ok()
            }
            _ => false,
        };
    }
    parse_mp2_header(data).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The corpus `0119` geometry: MPEG-1 L2 384 kbps 44.1 kHz stereo.
    const MP2_384K_44100: [u8; 4] = [0xFF, 0xFD, 0xE0, 0x00];
    /// The corpus `0005` geometry: MPEG-2 LSF L2 80 kbps 16 kHz mono.
    const MP2_LSF_80K_16000: [u8; 4] = [0xFF, 0xF5, 0x98, 0xC0];

    #[test]
    fn parses_the_canonical_layer_ii_headers() {
        let h = match parse_mp2_header(&MP2_384K_44100) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(h.family.version, MpegVersion::V1);
        assert_eq!(h.family.layer, Layer::L2);
        assert_eq!(h.family.bitrate_bps, 384_000);
        assert_eq!(h.family.sample_rate, 44_100);
        assert_eq!(h.family.channel_mode, ChannelMode::Stereo);
        assert_eq!(h.family.samples_per_frame, 1152);
        // 144 × 384000 / 44100 = 1253 (floor).
        assert_eq!(h.family.frame_size, 1253);

        let h = match parse_mp2_header(&MP2_LSF_80K_16000) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(h.family.version, MpegVersion::V2);
        assert_eq!(h.family.bitrate_bps, 80_000);
        assert_eq!(h.family.sample_rate, 16_000);
        assert_eq!(h.family.channel_mode, ChannelMode::Mono);
        assert_eq!(channel_count(h.family.channel_mode), 1);
        // 144 × 80000 / 16000 = 720 exactly.
        assert_eq!(h.family.frame_size, 720);
    }

    #[test]
    fn non_layer_ii_is_a_typed_refusal() {
        // Layer III (0xFFFB): the family parses, the gate refuses.
        let e = match parse_mp2_header(&[0xFF, 0xFB, 0x90, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("Layer III must be refused by the mp2 gate"),
        };
        assert_eq!(e.context.code, "mp2.header.not_layer_ii");
        assert!(matches!(e.kind, media_types::ErrorKind::Corrupt { .. }));
        // Layer I (0xFFFE) also.
        let e = match parse_mp2_header(&[0xFF, 0xFE, 0xE0, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("Layer I must be refused by the mp2 gate"),
        };
        assert_eq!(e.context.code, "mp2.header.not_layer_ii");
    }

    #[test]
    fn family_header_failures_pass_through_typed() {
        // Bad sync passes through with the family code.
        let e = match parse_mp2_header(&[0x00, 0xFD, 0xE0, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("bad sync must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.sync");
        // Free-format Layer II: the family's typed refusal.
        let e = match parse_mp2_header(&[0xFF, 0xFD, 0x00, 0x00]) {
            Err(e) => e,
            Ok(_) => unreachable!("free format must be refused"),
        };
        assert_eq!(e.context.code, "mp3.free_format");
        // Short header.
        let e = match parse_mp2_header(&[0xFF, 0xFD]) {
            Err(e) => e,
            Ok(_) => unreachable!("short header must fail"),
        };
        assert_eq!(e.context.code, "mp3.header.short");
    }

    #[test]
    fn bitrate_tables_match_the_family_parser() {
        // Every MPEG-1 Layer II index the family accepts must equal
        // our table row (a cross-module pin: the two tables cannot
        // drift apart silently).
        for index in 1..=14u8 {
            let mut hdr = MP2_384K_44100;
            hdr[2] = (hdr[2] & 0x0F) | (index << 4);
            let h = match parse_mp2_header(&hdr) {
                Ok(h) => h,
                Err(_) => unreachable!("index {index} is a valid MPEG-1 L2 bitrate"),
            };
            assert_eq!(
                h.family.bitrate_bps,
                layer_ii_bitrate(MpegVersion::V1, index).unwrap_or(0) * 1000,
                "MPEG-1 L2 index {index}"
            );
        }
        for index in 1..=14u8 {
            let mut hdr = MP2_LSF_80K_16000;
            hdr[2] = (hdr[2] & 0x0F) | (index << 4);
            let h = match parse_mp2_header(&hdr) {
                Ok(h) => h,
                Err(_) => unreachable!("index {index} is a valid MPEG-2 L2 bitrate"),
            };
            assert_eq!(
                h.family.bitrate_bps,
                layer_ii_bitrate(MpegVersion::V2, index).unwrap_or(0) * 1000,
                "MPEG-2 L2 index {index}"
            );
        }
        // The refusal edges.
        assert_eq!(layer_ii_bitrate(MpegVersion::V1, 0), None);
        assert_eq!(layer_ii_bitrate(MpegVersion::V2, 15), None);
        assert_eq!(layer_ii_bitrate_table(MpegVersion::Reserved), None);
    }

    #[test]
    fn mode_extension_decodes_the_layer_ii_bound() {
        // Joint stereo + each of the four bound encodings.
        for (bits, bound) in [(0u8, 4u8), (1, 8), (2, 12), (3, 16)] {
            let mut hdr = [0xFF, 0xFD, 0xE0, 0x00];
            hdr[3] = 0x40 | (bits << 4);
            let h = match parse_mp2_header(&hdr) {
                Ok(h) => h,
                Err(e) => unreachable!("{e:?}"),
            };
            assert_eq!(h.family.channel_mode, ChannelMode::JointStereo);
            assert_eq!(h.mode_extension.bound_subband(), bound);
            assert_eq!(
                intensity_bound(h.family.channel_mode, h.mode_extension),
                Some(bound)
            );
        }
        // Outside joint stereo the bits read as a bound but the
        // honest accessor returns None (spec: N/A).
        let h = match parse_mp2_header(&[0xFF, 0xFD, 0xE0, 0x20]) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(h.family.channel_mode, ChannelMode::Stereo);
        assert_eq!(h.mode_extension.bound_subband(), 12);
        assert_eq!(
            intensity_bound(h.family.channel_mode, h.mode_extension),
            None
        );
    }

    #[test]
    fn walk_gates_layer_and_reports_geometry() {
        // Two Layer II frames + garbage between them (resync), then
        // geometry consistency.
        let mut data = Vec::new();
        data.extend_from_slice(&MP2_LSF_80K_16000);
        data.resize(data.len() + 720 - 4, 0xA5);
        data.extend_from_slice(&[0u8; 6]); // inter-frame garbage
        data.extend_from_slice(&MP2_LSF_80K_16000);
        data.resize(data.len() + 720 - 4, 0x5A);
        let frames = match walk_mp2_frames(&data, 0) {
            Ok(f) => f,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].offset, 720 + 6);
        let geo = match geometry(&frames) {
            Ok(Some(g)) => g,
            Ok(None) => unreachable!("two frames must distill a geometry"),
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(
            (
                geo.bitrate_bps,
                geo.sample_rate,
                geo.channels,
                geo.samples_per_frame,
                geo.frame_size
            ),
            (80_000, 16_000, 1, 1152, 720)
        );
        assert_eq!(geo.total_samples, 2 * 1152);
        match geometry(&[]) {
            Ok(None) => {}
            other => unreachable!("an empty walk has no geometry: {other:?}"),
        }
    }

    #[test]
    fn walk_refuses_a_midstream_layer_switch() {
        // A valid Layer III frame after a Layer II frame: typed.
        let mut data = Vec::new();
        data.extend_from_slice(&MP2_LSF_80K_16000);
        data.resize(data.len() + 720 - 4, 0);
        data.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        data.resize(data.len() + 417 - 4, 0);
        let e = match walk_mp2_frames(&data, 0) {
            Err(e) => e,
            Ok(_) => unreachable!("a Layer III frame inside an mp2 stream is a mismatch"),
        };
        assert_eq!(e.context.code, "mp2.stream.layer_mismatch");
    }

    #[test]
    fn geometry_refuses_a_bitrate_switch() {
        // Same version/rate, different bitrate index mid-stream: the
        // CBR gate (seek arithmetic must never divide by a guess).
        let mut data = Vec::new();
        let mut second = MP2_LSF_80K_16000;
        second[2] = (second[2] & 0x0F) | (11 << 4); // 112 kbps
        data.extend_from_slice(&MP2_LSF_80K_16000);
        data.resize(data.len() + 720 - 4, 0);
        data.extend_from_slice(&second);
        data.resize(data.len() + (144 * 112_000 / 16_000) - 4, 0);
        let frames = match walk_mp2_frames(&data, 0) {
            Ok(f) => f,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(frames.len(), 2);
        let e = match geometry(&frames) {
            Err(e) => e,
            Ok(_) => unreachable!("a mid-stream bitrate switch must be refused"),
        };
        assert_eq!(e.context.code, "mp2.stream.cbr_mismatch");
    }

    #[test]
    fn padding_bit_varies_frame_size_but_not_the_nominal_geometry() {
        // Padded vs unpadded first frame: nominal size strips padding.
        let mut padded = MP2_384K_44100;
        padded[2] |= 0x02;
        let h = match parse_mp2_header(&padded) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        assert!(h.family.padding);
        assert_eq!(h.family.frame_size, 1254);
        let mut data = Vec::new();
        data.extend_from_slice(&padded);
        data.resize(data.len() + 1254 - 4, 0);
        data.extend_from_slice(&MP2_384K_44100);
        data.resize(data.len() + 1253 - 4, 0);
        let frames = match walk_mp2_frames(&data, 0) {
            Ok(f) => f,
            Err(e) => unreachable!("{e:?}"),
        };
        let geo = match geometry(&frames) {
            Ok(Some(g)) => g,
            other => unreachable!("mixed padding is legal CBR: {other:?}"),
        };
        assert_eq!(geo.frame_size, 1253);
    }

    #[test]
    fn probe_accepts_layer_ii_only() {
        assert!(is_mp2_stream(&MP2_384K_44100));
        assert!(is_mp2_stream(&MP2_LSF_80K_16000));
        // Layer III is the mp3 family's, not ours.
        assert!(!is_mp2_stream(&[0xFF, 0xFB, 0x90, 0x00]));
        assert!(!is_mp2_stream(&[0xFF, 0x00, 0x00, 0x00]));
        assert!(!is_mp2_stream(b"RIFF"));
        assert!(!is_mp2_stream(&[]));
        // A truncated 3-byte tail is not a stream.
        assert!(!is_mp2_stream(&MP2_384K_44100[..3]));
    }
}
