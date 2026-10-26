//! The raw elementary-stream signatures (card 6.OPW-es): the
//! headerless "container" family of `demux::es` — raw MPEG audio
//! (`.mp3`/`.mp2`), Annex-B H.264/HEVC (`.264`/`.265`) and native
//! FLAC (`.flac`). Until this card none of them had a probe
//! signature, so every raw-ES file probe-missed: the open path could
//! not route them even though the demuxers exist.
//!
//! # The design decision (Lead-approved, this card's review point)
//!
//! These are the only builtin signatures whose formats carry **no
//! anchored magic at all** in the container sense — a raw ES file is
//! one bare codec bitstream. The anchors are therefore the
//! bitstream's own framing grammar, and each signature walks only as
//! deep as it needs to *name the demuxer*, never to parse:
//!
//! * **MPEG audio** — the 11-bit frame sync (`0xFF` + `0xE0`-masked
//!   second byte) with **layer-bit discrimination**: Layer III is
//!   bits `01` (`mp3`), Layer II is bits `10` (`mp2`). The layer
//!   field is what keeps this disjoint from the ADTS signature
//!   (ISO 13818-7 §6.5.1: ADTS layer is always `00` — the
//!   `AdtsSignature` precedent's own refinement, phase5.rs): an
//!   ADTS frame can never anchor here and an MPEG-audio frame can
//!   never anchor there. Layer I (bits `11`) stays unprobed — the
//!   demuxer demuxes it but no corpus or fixture lane exercises a
//!   raw Layer-I file, and a name we cannot measure is a name we do
//!   not mint (recorded gap). The structural half of the demuxer's
//!   own `parse_head` gate rides along (reserved version, free/bad
//!   bitrate index, reserved sampling index) — the value tables stay
//!   demux-side.
//! * **Annex-B video** — the `00 00 01` start code with **whole-window
//!   NAL-header-shape discrimination** between H.264 and HEVC. The
//!   first NAL alone is NOT reliably discriminable (an H.264
//!   `0x41` slice and an HEVC type-32 `0x40`-band header overlap in
//!   the other codec's type read), so the signature classifies every
//!   NAL header byte the window shows and lets impossible types kill
//!   hypotheses: a byte is H.264-impossible when its `nal_unit_type`
//!   (`b & 0x1F`) is 0 or 16..=31, HEVC-impossible when its type
//!   (`b >> 1`) exceeds 40 (VCL 0..=31, parameter sets 32..=37,
//!   unspecified 38, SEI 39..=40), and impossible in both when the
//!   forbidden bit is set. The surviving hypothesis must ALSO have
//!   seen a **parameter-set NAL** (H.264 SPS/PPS/subset-SPS, types
//!   7/8/15; HEVC VPS/SPS/PPS, types 32/33/34) — the same positive
//!   evidence ffmpeg's own raw probes demand, and the guard that
//!   keeps chance `00 00 01 00` runs in container payloads (measured:
//!   the class-B `flip16` AVI derivative carries five of them, all
//!   zero heads) from claiming the family: a window with no
//!   parameter set names nothing. Real streams decide decisively
//!   because their parameter sets do the killing: H.264 SPS/PPS/IDR
//!   (`0x67`/`0x68`/`0x65`) read as HEVC types 50..=52, and the
//!   HEVC VPS (`0x40`) reads as H.264 type 0 — measured on the
//!   corpus lanes (`0019-raw-video.h264`, `0044-raw-video.265`) and
//!   the committed fixtures. When both hypotheses survive the
//!   honest verdict is the **family id `annexb` at a sub-threshold
//!   score** (the registry then refuses, naming the family) — a
//!   guess between two codecs is never minted as a fact, and the
//!   extension is not consulted to break the tie (content wins;
//!   the extension route is a recorded future decision, not this
//!   card's). A parameter-set-less window (mid-stream cut without
//!   SPS/PPS in range) probe-misses — recorded gap, mirroring the
//!   reference prober's own posture.
//! * **FLAC** — the `fLaC` stream marker with the spec-mandated
//!   first metadata block shape (type 0 STREAMINFO, 34-byte body):
//!   distinct from every other signature's magic, so this is the
//!   straightforward one. **No raw-FLAC demuxer exists** —
//!   `crates/demux/src/es` covers h264/hevc/mp3/aac only (card
//!   5.I3a's slice) — so the id routes to the open path's honest
//!   typed refusal naming that gap; the signature is landed now so
//!   the demuxer card that lands later flips one registry arm, and
//!   the FLAC decoder's contract (the 34-byte STREAMINFO body in
//!   `extra_data`, `codecs/src/audio/flac/decoder.rs`) is the
//!   harvest that card owes.
//!
//! # Known content-probe gaps (recorded, not smoothed)
//!
//! - An ID3-prefixed MPEG-audio file anchors **after** a validated
//!   ID3v2 tag only when the tag ends inside the probe window (the
//!   demuxer skips ID3 itself; refusing to route a file the demuxer
//!   plays would be the dishonesty — this supersedes the recorded
//!   gap note for raw MP3 in `phase5.rs`, which predates this
//!   signature). A tag larger than the window (measured: corpus
//!   `0071-cover-audio.flac` carries a 99,682-byte ID3v2 tag —
//!   MP3 bytes in a `.flac` name, ffprobe-verified) still
//!   probe-misses: the window never reaches a frame.
//! - MVC/3D-AVC H.264 NAL types (16..=20) kill the H.264 hypothesis
//!   (they read as HEVC types 8..=10) — such streams stay unprobed;
//!   the demuxer's own AU walk does not bound them either.
//! - A stream cut to begin inside an MPEG-1/2 picture (leading
//!   `00 00 01 00` picture / low slice codes with no `B3` sequence
//!   header) can satisfy the HEVC hypothesis — the demuxer's own
//!   plausibility gate then decides; recorded as the one known
//!   cross-family shape.

use crate::report::{FormatId, ProbeHit, SourceMeta};
use crate::signature::ProbePlugin;

use super::{MAX_WALK_STEPS, SCORE_FAMILY_ONLY, SCORE_MAGIC_ANCHOR};

/// A magic-anchored hit at the ladder's anchor rung (the phase5
/// helper's shape — kept local so this module reads standalone).
fn anchored(format: FormatId, consumed: usize) -> ProbeHit {
    ProbeHit {
        format,
        score: SCORE_MAGIC_ANCHOR,
        track_hints: Vec::new(),
        prefix_consumed: consumed,
    }
}

/// The MPEG-audio layer a frame header's bits name (the probe-side
/// half of the demuxer's `parse_head` layer field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MpegLayer {
    /// Bits `01` — Layer III (`.mp3`).
    L3,
    /// Bits `10` — Layer II (`.mp2`).
    L2,
}

/// Where an MPEG-audio frame header anchors in the window: byte 0,
/// or just past a head ID3v2 tag that ends inside the window.
///
/// Returns the anchor offset plus the layer the header's bits name.
/// The structural rejections mirror the demuxer's own
/// `parse_head` gate (reserved version, free-format/bad bitrate
/// index, reserved sampling index) — everything table-driven stays
/// demux-side; the layer-`00` (ADTS) and Layer-I (`11`) shapes
/// return `None` here (disjointness and the recorded gap).
fn mpeg_audio_anchor(prefix: &[u8]) -> Option<(usize, MpegLayer)> {
    let at = id3v2_end(prefix).unwrap_or(0);
    let h = prefix.get(at..at + 4)?;
    if h[0] != 0xFF || h[1] & 0xE0 != 0xE0 {
        return None; // the 11-bit frame sync
    }
    if h[1] >> 3 & 0x3 == 0b01 {
        return None; // reserved MPEG version
    }
    let layer = match h[1] >> 1 & 0x3 {
        0b01 => MpegLayer::L3,
        0b10 => MpegLayer::L2,
        _ => return None, // 00 = ADTS's layer; 11 = Layer I (gap)
    };
    if h[2] >> 4 == 0 || h[2] >> 4 == 0b1111 {
        return None; // free format or bad bitrate index
    }
    if h[2] >> 2 & 0x3 == 0b11 {
        return None; // reserved sampling index
    }
    Some((at, layer))
}

/// The end offset of a head ID3v2 tag (10-byte header + syncsafe
/// size + optional footer), when one starts the window AND ends
/// inside it — `None` otherwise (no tag, or a tag the window cannot
/// see past; a non-syncsafe size field is not an ID3v2 tag).
fn id3v2_end(prefix: &[u8]) -> Option<usize> {
    if prefix.len() < 10 || &prefix[0..3] != b"ID3" {
        return None;
    }
    let size_bytes = [prefix[6], prefix[7], prefix[8], prefix[9]];
    if size_bytes.iter().any(|b| b & 0x80 != 0) {
        return None; // not syncsafe — not a tag size
    }
    let size = (size_bytes[0] as usize) << 21
        | (size_bytes[1] as usize) << 14
        | (size_bytes[2] as usize) << 7
        | size_bytes[3] as usize;
    let footer = usize::from(prefix[5] & 0x10 != 0) * 10;
    let end = 10 + size + footer;
    (end <= prefix.len()).then_some(end)
}

/// The raw MP3 (MPEG-audio Layer III) signature: the frame sync
/// with layer bits `01`, anchored at the window start or just past
/// a head ID3v2 tag — disjoint from ADTS by the layer field (the
/// `AdtsSignature` refinement's other half).
pub struct Mp3Signature;

impl ProbePlugin for Mp3Signature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        let (at, layer) = mpeg_audio_anchor(prefix)?;
        (layer == MpegLayer::L3).then(|| anchored(FormatId::mp3(), at + 4))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mp3"]
    }
}

/// The raw MP2 (MPEG-audio Layer II) signature: the same frame sync
/// with layer bits `10` — the layer is the only bit that separates
/// the two ids, which is exactly the discrimination the demuxer's
/// own track descriptor makes (`es/mp3.rs`: L3 → `mp3`,
/// L1|L2 → `mp2`).
pub struct Mp2Signature;

impl ProbePlugin for Mp2Signature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        let (at, layer) = mpeg_audio_anchor(prefix)?;
        (layer == MpegLayer::L2).then(|| anchored(FormatId::mp2(), at + 4))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mp2"]
    }
}

/// The Annex-B raw video signature (H.264 `.264` / HEVC `.265`):
/// start-code anchored with whole-window NAL-header-shape
/// discrimination — see the module docs for the design decision and
/// its honesty tradeoffs.
pub struct AnnexbSignature;

impl ProbePlugin for AnnexbSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        // Walk every start code the window shows (bounded by the
        // house walk guard) and classify the NAL header byte that
        // follows under both codec hypotheses, tracking the
        // parameter-set evidence each needs to be named at all.
        let mut seen = 0usize;
        let mut h264_alive = true;
        let mut hevc_alive = true;
        let mut h264_param = false;
        let mut hevc_param = false;
        let mut i = 0usize;
        while i + 4 <= prefix.len() && seen < MAX_WALK_STEPS {
            if prefix[i] == 0 && prefix[i + 1] == 0 && prefix[i + 2] == 1 {
                let b = prefix[i + 3];
                seen += 1;
                if b & 0x80 != 0 {
                    // forbidden_zero_bit set: invalid in both codecs.
                    h264_alive = false;
                    hevc_alive = false;
                } else {
                    let h264_type = b & 0x1F;
                    let hevc_type = b >> 1;
                    if !matches!(h264_type, 1..=15) {
                        h264_alive = false; // type 0/16..=31: not H.264
                    } else if matches!(h264_type, 7 | 8 | 15) {
                        h264_param = true; // SPS / PPS / subset-SPS
                    }
                    if hevc_type > 40 {
                        hevc_alive = false; // past VCL+params+SEI: not HEVC
                    } else if matches!(hevc_type, 32..=34) {
                        hevc_param = true; // VPS / SPS / PPS
                    }
                }
                i += 4;
            } else {
                i += 1;
            }
        }
        if seen == 0 {
            return None; // no start code at all — not this family
        }
        match (h264_alive, hevc_alive) {
            // Decisive only with the surviving hypothesis's own
            // parameter-set evidence (the positive proof that keeps
            // payload-noise start codes unnamed).
            (true, false) if h264_param => Some(anchored(FormatId::h264(), i)),
            (false, true) if hevc_param => Some(anchored(FormatId::hevc(), i)),
            // Both survive (with either side's parameters seen): the
            // family is certain, the codec is not — a sub-threshold
            // family hit, so the registry refuses while still naming
            // `annexb` in the error (never a coin-flip id minted as
            // a fact).
            (true, true) if h264_param || hevc_param => Some(ProbeHit {
                format: FormatId::annexb(),
                score: SCORE_FAMILY_ONLY,
                track_hints: Vec::new(),
                prefix_consumed: i,
            }),
            // Alive without parameters, or dead: start codes alone
            // name nothing (a mid-stream cut without SPS/PPS in the
            // window is the recorded gap; payload noise is the
            // guarded case).
            _ => None,
        }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["264", "h264", "265", "h265", "hevc"]
    }
}

/// The native FLAC signature: the `fLaC` stream marker with the
/// spec-mandated first metadata block (type 0 STREAMINFO, 34-byte
/// body) — the marker alone names the family; the block shape keeps
/// a four-byte text collision from claiming FLAC.
pub struct FlacSignature;

impl ProbePlugin for FlacSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        (prefix.len() >= 8
            && &prefix[0..4] == b"fLaC"
            && prefix[4] & 0x7F == 0
            && prefix[5] == 0
            && prefix[6] == 0
            && prefix[7] == 34)
            .then(|| anchored(FormatId::flac(), 8))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["flac"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProbeConfig;
    use crate::report::Confidence;
    use crate::signature::ProbeRegistry;
    use media_types::PipelineStage;

    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    fn registry() -> ProbeRegistry {
        ProbeRegistry::with_builtins()
    }

    fn config() -> ProbeConfig {
        ok(ProbeConfig::new(4096, 60))
    }

    fn probe(prefix: &[u8]) -> Result<crate::report::ProbeReport, media_types::EngineError> {
        registry().probe(prefix, &SourceMeta::new(None, None), &config())
    }

    /// A Layer III frame header at the window start (MPEG-1, 128 kbps,
    /// 44.1 kHz stereo — the demuxer's own test vector shape).
    #[test]
    fn layer3_header_probes_mp3() {
        let report = ok(probe(&[0xFF, 0xFB, 0x90, 0x00]));
        assert_eq!(report.format.as_str(), "mp3");
        assert_eq!(report.confidence.as_u8(), 70);
    }

    /// A Layer II frame header (MPEG-1, 384 kbps, 48 kHz stereo — the
    /// committed `mp2_48k.mp2` fixture's own header) probes `mp2`.
    #[test]
    fn layer2_header_probes_mp2() {
        let report = ok(probe(&[0xFF, 0xFD, 0xA4, 0x04]));
        assert_eq!(report.format.as_str(), "mp2");
    }

    /// MPEG-2.5 low-sampling-frequency headers anchor too (the
    /// corpus's `0005-audio-only.mp2` is `0xFF 0xF5` — version bits
    /// `00`, Layer II).
    #[test]
    fn mpeg25_layer2_probes_mp2() {
        let report = ok(probe(&[0xFF, 0xF5, 0x98, 0xC0]));
        assert_eq!(report.format.as_str(), "mp2");
    }

    /// ADTS frames (layer `00`) never anchor here — the layer-bit
    /// disjointness the `AdtsSignature` precedent established, proven
    /// from this side too.
    #[test]
    fn adts_layer00_is_not_mpeg_audio() {
        for b1 in [0xF0u8, 0xF1] {
            assert!(
                mpeg_audio_anchor(&[0xFF, b1, 0x90, 0x00]).is_none(),
                "layer-00 must not anchor MPEG audio"
            );
        }
        // And the registry still routes those windows to ADTS.
        let report = ok(probe(&[0xFF, 0xF1, 0x50, 0x80, 0x00, 0x00]));
        assert_eq!(report.format.as_str(), "adts");
    }

    /// The structural rejections: free-format bitrate, bad bitrate
    /// index, reserved sampling index, reserved version, Layer I —
    /// windows with the sync bits but a header the demuxer itself
    /// would refuse stay unprobed (no name without a frame).
    #[test]
    fn structurally_invalid_headers_stay_unprobed() {
        for h in [
            [0xFFu8, 0xFB, 0x00, 0x00], // free-format bitrate index
            [0xFF, 0xFB, 0xF0, 0x00],   // bad bitrate index 15
            [0xFF, 0xFB, 0x9C, 0x00],   // reserved sampling index 3
            [0xFF, 0xEB, 0x90, 0x00],   // reserved MPEG version (01)
            [0xFF, 0xFE, 0x90, 0x00],   // Layer I (bits 11): the gap
        ] {
            match probe(&h) {
                Ok(report) => unreachable!("must not probe: {h:?} -> {}", report.format),
                Err(err) => assert_eq!(err.context.code, "probe.no-demuxer"),
            }
        }
    }

    /// The ID3-skip lane: a head tag that ends inside the window
    /// anchors the frame after it (the demuxer skips ID3 itself —
    /// routing the file is the probe's job). The tag bytes are the
    /// committed `mpeg2_24k.mp3` fixture's own 34-byte tag shape.
    #[test]
    fn id3_prefixed_frame_anchors_after_the_tag() {
        let mut window = vec![0x49, 0x44, 0x33, 0x04, 0x00, 0x00];
        window.extend_from_slice(&[0, 0, 0, 0x22]); // 34-byte tag body
        window.extend_from_slice(&[0u8; 34]);
        window.extend_from_slice(&[0xFF, 0xF3, 0x84, 0x00]); // MPEG-2 L3 24 kHz
        let report = ok(probe(&window));
        assert_eq!(report.format.as_str(), "mp3");
        // The consumed prefix includes the tag (44 bytes + 4 header).
        assert_eq!(report.bytes_examined, window.len());
    }

    /// A tag the window cannot see past stays a miss — measured
    /// shape: corpus `0071` carries a 99,682-byte tag against the
    /// 64 KiB default window.
    #[test]
    fn oversized_id3_tag_is_a_miss_not_a_guess() {
        let mut window = vec![0x49, 0x44, 0x33, 0x04, 0x00, 0x00];
        window.extend_from_slice(&[0x06, 0x0A, 0x62]); // 99,682-byte tag
        window.extend_from_slice(&[0xFFu8; 4000]); // window ends inside the tag
        match probe(&window) {
            Ok(report) => unreachable!("must not probe: {}", report.format),
            Err(err) => assert_eq!(err.context.code, "probe.no-demuxer"),
        }
    }

    /// H.264 decides: the SPS byte (`0x67`) reads as HEVC type 51,
    /// killing that hypothesis while every NAL stays H.264-plausible
    /// (the committed fixture's head: SPS `0x67`, then PPS/slices).
    #[test]
    fn h264_start_codes_probe_h264() {
        let window = [
            0x00, 0x00, 0x00, 0x01, 0x67, 0x64, 0x00, 0x0D, 0x00, 0x00, 0x00, 0x01, 0x68, 0xEA,
            0x00, 0x00, 0x00, 0x01, 0x06, 0x05, 0x00, 0x00, 0x00, 0x01, 0x41, 0x9A,
        ];
        let report = ok(probe(&window));
        assert_eq!(report.format.as_str(), "h264");
        assert_eq!(report.confidence.as_u8(), 70);
    }

    /// HEVC decides: the VPS byte (`0x40`) reads as H.264 type 0,
    /// killing that hypothesis while the NALs stay HEVC-plausible
    /// (the committed fixture's head: VPS `0x40 01`, then SPS).
    #[test]
    fn hevc_start_codes_probe_hevc() {
        let window = [
            0x00, 0x00, 0x00, 0x01, 0x40, 0x01, 0x0C, 0x01, 0xFF, 0xFF, 0x04, 0x08, 0x00, 0x00,
            0x03, 0x00, 0x9E, 0x08, 0x00, 0x00, 0x03, 0x00, 0x00, 0x1E, 0x00, 0x00, 0x01, 0x42,
            0x01,
        ];
        let report = ok(probe(&window));
        assert_eq!(report.format.as_str(), "hevc");
    }

    /// The ambiguous window: NAL bytes plausible in both codecs (the
    /// H.264 slice `0x41` is HEVC type 32; the HEVC SPS `0x42` is
    /// H.264 type 2) is a family-only, sub-threshold hit — the
    /// registry refuses it and names `annexb`, never a codec guess.
    #[test]
    fn ambiguous_nal_shapes_name_the_family_sub_threshold() {
        let window = [
            0x00, 0x00, 0x01, 0x41, 0x9A, 0x00, 0x00, 0x01, 0x02, 0x1D, 0x00, 0x00, 0x01, 0x06,
            0x05, 0x00, 0x00, 0x01, 0x42, 0x01,
        ];
        let err = match probe(&window) {
            Ok(report) => unreachable!("sub-threshold must not select: {}", report.format),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "probe.no-demuxer");
        assert_eq!(err.stage, PipelineStage::Probe);
        // The error's best-known format names the family.
        match &err.kind {
            media_types::ErrorKind::Unsupported { format } => {
                let info = match format {
                    Some(info) => info,
                    None => unreachable!("the family hit rides the error"),
                };
                assert_eq!(info.name.as_deref(), Some("annexb"));
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
        // The signature itself, asked directly, awards exactly the
        // family rung — the threshold math is the registry's.
        let hit = AnnexbSignature
            .probe(&window, &SourceMeta::new(None, None))
            .map(|hit| (hit.format, hit.score));
        match hit {
            Some((format, score)) => {
                assert_eq!(format.as_str(), "annexb");
                assert_eq!(score, SCORE_FAMILY_ONLY);
                let confidence = match Confidence::from_score(score) {
                    Some(c) => c,
                    None => unreachable!("literal score is in range"),
                };
                assert!(!confidence.meets(60), "50 must sit below the default 60");
            }
            None => unreachable!("the family anchored"),
        }
    }

    /// Start codes with no codec shape (MPEG-PS pack/system headers,
    /// an MPEG-1/2 sequence header) never claim the Annex-B family:
    /// the forbidden bit and impossible types kill both hypotheses.
    /// The PS pack header is honestly the MPEG-PS signature's own
    /// anchor — the assert is that the family ids are never minted,
    /// not that the window goes unnamed.
    #[test]
    fn mpeg_system_start_codes_stay_unprobed() {
        for window in [
            vec![0x00, 0x00, 0x01, 0xBA, 0x11], // PS pack header
            vec![0x00, 0x00, 0x01, 0xB3, 0x2C], // MPEG-1/2 sequence header
            vec![0x00, 0x00, 0x01, 0xE0, 0x00], // PES video
            vec![0x00, 0x00, 0x01, 0x80, 0x00], // forbidden bit set
        ] {
            match probe(&window) {
                Ok(report) => assert!(
                    !matches!(report.format.as_str(), "h264" | "hevc" | "annexb"),
                    "{} claimed the Annex-B family: {window:?}",
                    report.format
                ),
                Err(err) => assert_eq!(err.context.code, "probe.no-demuxer"),
            }
        }
        // And directly: the signature itself returns no decisive
        // hit for any of them (the PS claim above is another
        // signature's, not this one's).
        let meta = SourceMeta::new(None, None);
        for window in [
            vec![0x00, 0x00, 0x01, 0xBA, 0x11],
            vec![0x00, 0x00, 0x01, 0xB3, 0x2C],
            vec![0x00, 0x00, 0x01, 0xE0, 0x00],
            vec![0x00, 0x00, 0x01, 0x80, 0x00],
        ] {
            let hit = AnnexbSignature.probe(&window, &meta);
            assert!(hit.is_none(), "{window:?}: {hit:?}");
        }
    }

    /// The parameter-set guard: start codes whose heads are payload
    /// noise (the class-B `flip16` AVI derivative's five
    /// `00 00 01 00` runs — zero bytes read as HEVC TRAIL_N) carry
    /// no SPS/PPS/VPS evidence and name NOTHING, however many there
    /// are. This is the measured false-positive the guard exists for.
    #[test]
    fn start_codes_without_parameter_sets_name_nothing() {
        let window = [0x00u8, 0x00, 0x01, 0x00].repeat(64);
        match probe(&window) {
            Ok(report) => unreachable!("must not probe: {}", report.format),
            Err(err) => assert_eq!(err.context.code, "probe.no-demuxer"),
        }
        // And an all-plausible slice-only window (mid-stream cut,
        // no SPS/PPS in range) is the same honest miss.
        let slices = [0x00u8, 0x00, 0x01, 0x41, 0x9A].repeat(16);
        match probe(&slices) {
            Ok(report) => unreachable!("must not probe: {}", report.format),
            Err(err) => assert_eq!(err.context.code, "probe.no-demuxer"),
        }
    }

    /// The FLAC marker with the STREAMINFO-first block shape probes
    /// `flac` (the committed fixtures' and the corpus lanes' head).
    #[test]
    fn flac_marker_probes_flac() {
        let report = ok(probe(b"fLaC\x00\x00\x00\x22\x10\x00\x10\x00"));
        assert_eq!(report.format.as_str(), "flac");
        assert_eq!(report.confidence.as_u8(), 70);
    }

    /// A marker without the spec's first-block shape (non-STREAMINFO
    /// type, or a body length that is not 34) is not claimed.
    #[test]
    fn flac_near_misses_stay_unprobed() {
        for window in [
            &b"fLaC\x04\x00\x00\x22\x00\x00"[..], // type 4 first: not FLAC
            &b"fLaC\x00\x00\x00\x21\x00\x00"[..], // 33-byte "STREAMINFO"
            &b"fLaC"[..],                         // marker only
            &b"flaC\x00\x00\x00\x22\x00\x00"[..], // wrong case
        ] {
            match probe(window) {
                Ok(report) => unreachable!("must not probe: {window:?} -> {}", report.format),
                Err(err) => assert_eq!(err.context.code, "probe.no-demuxer"),
            }
        }
    }

    /// Disjointness from every other builtin, from this side: the
    /// MPEG-audio window does not claim ADTS, and none of the new
    /// signatures claims the committed families' magics.
    #[test]
    fn new_signatures_do_not_steal_other_families() {
        for (window, expected) in [
            (&b"RIFF\x00\x00\x00\x10AVI \x00\x00"[..], "avi"),
            (&b"OggS\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00"[..], "ogg"),
            (
                &[0x1A, 0x45, 0xDF, 0xA3, 0x93, 0x42, 0x86, 0x81, 0x01],
                "matroska",
            ),
        ] {
            let report = ok(probe(window));
            assert_eq!(report.format.as_str(), expected, "{window:?}");
        }
    }
}
