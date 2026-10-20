//! The AVI signature (card 2.A2): anchored RIFF/`AVI ` magic plus a
//! bounded `hdrl`/`strl` sniff — the cheap pre-stage to the demux
//! crate's full AVI 1.0 walk (doc 03 §4.2(a) "identify the container",
//! not parse it).
//!
//! Byte-level ground truth is the 2.A3 fixture writer's emission
//! (`tools/xtask/src/cmd/fixtures.rs`, `build_avi`): `RIFF` + u32le
//! size + form `AVI ` at 0..12, `LIST` + u32le size + `hdrl` at
//! 12..24, `avih` chunk at 24, one `LIST strl` per stream (`strh`
//! fccType/fccHandler, `strf` WAVEFORMATEX for audio). Sizes are
//! **never trusted into allocations** — every walk clamps to the
//! window and stops on sizes that stall or explode (a lying LIST size
//! merely extends the walk to the window edge; the hints found before
//! it survive, exactly the class-B `flip4` semantics).
//!
//! Scored ladder (see [`super`] for the rungs): magic 70, +`LIST hdrl`
//! 80, +`avih` 90, +track hints 95. RIFF forms other than `AVI `
//! (e.g. `WAVE`) do not anchor — a different format's signature owns
//! them.

use media_types::ids::MediaKind;

use super::{
    MAX_WALK_STEPS, ProbePlugin, SCORE_CONTAINER_STRUCTURE, SCORE_MAGIC_ANCHOR,
    SCORE_PRIMARY_HEADER, SCORE_TRACK_HINTS,
};
use crate::report::{FormatId, ProbeHit, SourceMeta, TrackHint};

/// The AVI (RIFF, form `AVI `) signature.
#[derive(Debug, Clone, Copy, Default)]
pub struct AviSignature;

impl ProbePlugin for AviSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        // Anchor: RIFF magic + AVI form. Short windows that cannot
        // even carry the form do not anchor.
        if prefix.len() < 12 || &prefix[0..4] != b"RIFF" || &prefix[8..12] != b"AVI " {
            return None;
        }
        let mut score = SCORE_MAGIC_ANCHOR;
        let mut hints = Vec::new();
        let mut consumed = 12usize;

        // hdrl LIST: `LIST` + u32le size + form `hdrl`.
        if prefix.len() >= 24 && &prefix[12..16] == b"LIST" && &prefix[20..24] == b"hdrl" {
            score = SCORE_CONTAINER_STRUCTURE;
            // The LIST size counts form + children; clamp to the
            // window so a lie cannot extend the scan past the bound.
            let declared = u32_le(prefix, 16) as usize;
            let end = 20usize.saturating_add(declared).min(prefix.len());
            let (saw_avih, walk_hints, walked_to) = walk_hdrl(prefix, 24, end);
            if saw_avih {
                score = SCORE_PRIMARY_HEADER;
            }
            hints = walk_hints;
            if !hints.is_empty() {
                score = SCORE_TRACK_HINTS;
            }
            consumed = walked_to;
        }

        Some(ProbeHit {
            format: FormatId::avi(),
            score,
            track_hints: hints,
            prefix_consumed: consumed,
        })
    }
}

/// Walk the `hdrl` LIST's children (start..end, already clamped):
/// `avih` confirms the primary header; each `LIST strl` is sniffed for
/// a track hint. Best-effort by design — a truncated or lying chunk
/// ends the walk with whatever was collected (the demuxer owns
/// hard-error semantics).
fn walk_hdrl(prefix: &[u8], start: usize, end: usize) -> (bool, Vec<TrackHint>, usize) {
    let mut saw_avih = false;
    let mut hints = Vec::new();
    let mut pos = start;
    let mut steps = 0;
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let fourcc = &prefix[pos..pos + 4];
        let size = u32_le(prefix, pos + 4) as usize;
        let payload_start = pos + 8;
        let payload_end = payload_start.saturating_add(size).min(end);
        if fourcc == b"avih" {
            saw_avih = true;
        }
        if fourcc == b"LIST"
            && payload_start + 4 <= payload_end
            && &prefix[payload_start..payload_start + 4] == b"strl"
        {
            hints.extend(walk_strl(prefix, payload_start + 4, payload_end));
        }
        // RIFF chunks are WORD-aligned: header + size + one pad byte
        // when the payload is odd. A zero-size chunk still advances by
        // its header, so the walk cannot stall.
        pos = pos.saturating_add(size + 8 + (size & 1));
    }
    (saw_avih, hints, pos.min(end))
}

/// Walk one `strl` LIST (children start..end): collect `strh`
/// (fccType/fccHandler) and `strf` (WAVEFORMATEX for audio), then map
/// the pair to at most one hint.
fn walk_strl(prefix: &[u8], start: usize, end: usize) -> Vec<TrackHint> {
    let mut stream_type: Option<(&[u8], &[u8])> = None;
    let mut wave: Option<(u16, u16)> = None;
    let mut pos = start;
    let mut steps = 0;
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let fourcc = &prefix[pos..pos + 4];
        let size = u32_le(prefix, pos + 4) as usize;
        let payload_start = pos + 8;
        let payload_end = payload_start.saturating_add(size).min(end);
        if fourcc == b"strh" && payload_end >= payload_start + 8 {
            stream_type = Some((
                &prefix[payload_start..payload_start + 4],
                &prefix[payload_start + 4..payload_start + 8],
            ));
        }
        if fourcc == b"strf" && payload_end >= payload_start + 16 {
            // WAVEFORMATEX: wFormatTag, nChannels, nSamplesPerSec,
            // nAvgBytesPerSec, nBlockAlign, wBitsPerSample.
            wave = Some((
                u16_le(prefix, payload_start),
                u16_le(prefix, payload_start + 14),
            ));
        }
        pos = pos.saturating_add(size + 8 + (size & 1));
    }
    classify_stream(stream_type, wave)
}

/// Map a sniffed (`strh`, `strf`) pair to a hint.
///
/// Only the pairs with in-repo ground truth produce hints — the
/// fixture corpus's `vids`+`MJPG` and `auds`+PCM, named exactly as the
/// demuxer names them (`mjpeg`, `pcm_s16le`, …). Anything else stays
/// unhinted: the probe never invents codec names (the decoder cards
/// pin new rows when they land; additive).
fn classify_stream(
    stream_type: Option<(&[u8], &[u8])>,
    wave: Option<(u16, u16)>,
) -> Vec<TrackHint> {
    let Some((fcc_type, handler)) = stream_type else {
        return Vec::new();
    };
    if fcc_type == b"vids"
        && matches!(handler, b"MJPG" | b"mjpa" | b"mjpb")
        && let Some(hint) = TrackHint::new(MediaKind::Video, "mjpeg").ok()
    {
        return vec![hint];
    }
    if fcc_type == b"auds"
        && let Some((tag, bits)) = wave
        && let Some(name) = pcm_descriptor(tag, bits)
        && let Some(hint) = TrackHint::new(MediaKind::Audio, name).ok()
    {
        return vec![hint];
    }
    Vec::new()
}

/// The PCM WAVEFORMATEX tag × bit-width → descriptor table (the
/// demuxer's own naming; PCM integer tags 0x0001, IEEE float 0x0003).
fn pcm_descriptor(tag: u16, bits: u16) -> Option<&'static str> {
    match (tag, bits) {
        (0x0001, 8) => Some("pcm_u8"),
        (0x0001, 16) => Some("pcm_s16le"),
        (0x0001, 24) => Some("pcm_s24le"),
        (0x0001, 32) => Some("pcm_s32le"),
        (0x0003, 32) => Some("pcm_f32le"),
        (0x0003, 64) => Some("pcm_f64le"),
        _ => None,
    }
}

/// Little-endian u32 at `at` (RIFF is little-endian throughout).
fn u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Little-endian u16 at `at`.
fn u16_le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    // ------------------------------------------------------------------
    // A tiny AVI builder (mirrors the 2.A3 writer / demux test rules:
    // WORD-aligned chunks, patched LIST sizes).
    // ------------------------------------------------------------------

    fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) {
        out.extend_from_slice(fourcc);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        if payload.len() & 1 == 1 {
            out.push(0);
        }
    }

    fn list(out: &mut Vec<u8>, form: &[u8; 4], children: &[u8]) -> usize {
        let size_pos = out.len() + 4;
        out.extend_from_slice(b"LIST");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(form);
        out.extend_from_slice(children);
        // A RIFF/LIST size counts everything AFTER the size field: the
        // form fourcc plus the children — the 2.A3 writer's own rule
        // ("a RIFF/LIST size includes its form FourCC"), pinned against
        // the committed parent's bytes (hdrl LIST at 12 declares 292 =
        // 4 + 64 avih + 124 strl + 100 strl, extent ending at 312).
        let size = (out.len() - (size_pos + 4)) as u32;
        out[size_pos..size_pos + 4].copy_from_slice(&size.to_le_bytes());
        size_pos - 4
    }

    fn strh(fcc_type: &[u8; 4], handler: &[u8; 4]) -> Vec<u8> {
        let mut payload = vec![0u8; 56];
        payload[0..4].copy_from_slice(fcc_type);
        payload[4..8].copy_from_slice(handler);
        let mut out = Vec::new();
        chunk(&mut out, b"strh", &payload);
        out
    }

    fn strf_wave(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&tag.to_le_bytes());
        p.extend_from_slice(&channels.to_le_bytes());
        p.extend_from_slice(&rate.to_le_bytes());
        p.extend_from_slice(&(rate * channels as u32 * (bits / 8) as u32).to_le_bytes());
        p.extend_from_slice(&(channels * (bits / 8)).to_le_bytes());
        p.extend_from_slice(&bits.to_le_bytes());
        let mut out = Vec::new();
        chunk(&mut out, b"strf", &p);
        out
    }

    fn strf_bitmap() -> Vec<u8> {
        let mut p = vec![0u8; 40];
        p[4..8].copy_from_slice(&320u32.to_le_bytes());
        p[8..12].copy_from_slice(&240u32.to_le_bytes());
        p[14..16].copy_from_slice(&24u16.to_le_bytes());
        p[16..20].copy_from_slice(b"MJPG");
        let mut out = Vec::new();
        chunk(&mut out, b"strf", &p);
        out
    }

    fn full_header() -> Vec<u8> {
        let mut strl_video = Vec::new();
        strl_video.extend(strh(b"vids", b"MJPG"));
        strl_video.extend(strf_bitmap());
        let mut strl_audio = Vec::new();
        strl_audio.extend(strh(b"auds", b"\0\0\0\0"));
        strl_audio.extend(strf_wave(0x0001, 2, 44100, 16));
        let mut hdrl = Vec::new();
        chunk(&mut hdrl, b"avih", &[0u8; 56]);
        list(&mut hdrl, b"strl", &strl_video);
        list(&mut hdrl, b"strl", &strl_audio);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"AVI ");
        list(&mut out, b"hdrl", &hdrl);
        let riff_size = (out.len() - 8) as u32;
        out[4..8].copy_from_slice(&riff_size.to_le_bytes());
        out
    }

    fn probe(prefix: &[u8]) -> Option<ProbeHit> {
        AviSignature.probe(prefix, &SourceMeta::new(None, None))
    }

    fn hint_kinds(hit: &ProbeHit) -> Vec<(MediaKind, &str)> {
        hit.track_hints
            .iter()
            .map(|h| (h.kind, h.codec.as_str()))
            .collect()
    }

    #[test]
    fn magic_only_anchors_at_seventy() {
        // Exactly the class-B `trunc0` shape: RIFF + size + form.
        let window = b"RIFF\x24\x7e\x06\x00AVI ";
        let hit = match probe(window) {
            Some(h) => h,
            None => unreachable!("magic anchors"),
        };
        assert_eq!(hit.format.as_str(), "avi");
        assert_eq!(hit.score, 70);
        assert!(hit.track_hints.is_empty());
        assert_eq!(hit.prefix_consumed, 12);
    }

    #[test]
    fn short_or_foreign_windows_do_not_anchor() {
        for window in [
            &b"RIFF"[..],
            &b"RIFF...."[..],
            &b"RIFF....AVI"[..],
            &b""[..],
            &b"RIFX....AVI "[..],
            &b"RIFF....WAVE"[..],
            &b"RIFF....AVIX"[..],
            &b"xxxx....AVI "[..],
        ] {
            assert!(probe(window).is_none(), "must not anchor: {window:?}");
        }
    }

    #[test]
    fn hdrl_list_confirms_structure() {
        let full = full_header();
        // 24 bytes: RIFF+form (12) + LIST header (8) + `hdrl` form (4)
        // — the avih chunk header is not yet visible.
        let hit = match probe(&full[..24]) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 80);
        assert!(hit.track_hints.is_empty());
    }

    #[test]
    fn avih_confirms_primary_header() {
        let full = full_header();
        // 32 bytes: through the avih chunk header (fourcc + size).
        let hit = match probe(&full[..32]) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 90);
        assert!(hit.track_hints.is_empty());
        // The class-B `trunc1` shape (52 bytes: 20 bytes of avih
        // payload) lands on the same rung.
        let hit = match probe(&full[..52]) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 90);
    }

    #[test]
    fn full_header_sniffs_both_tracks() {
        let full = full_header();
        let hit = match probe(&full) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95);
        assert_eq!(
            hint_kinds(&hit),
            vec![(MediaKind::Video, "mjpeg"), (MediaKind::Audio, "pcm_s16le")]
        );
        // The walk consumed exactly the hdrl extent.
        assert_eq!(hit.prefix_consumed, full.len());
        // A window larger than the header (the movi LIST follows)
        // consumes only the hdrl extent.
        let mut with_movi = full.clone();
        let mut movi = Vec::new();
        chunk(&mut movi, b"00dc", &[0xFF, 0xD8, 0xFF]);
        list(&mut with_movi, b"movi", &movi);
        let hit = match probe(&with_movi) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95);
        assert_eq!(hit.prefix_consumed, full.len());
        assert_eq!(hit.track_hints.len(), 2);
    }

    #[test]
    fn lying_list_size_clamps_to_the_window_but_keeps_earlier_hints() {
        // The class-B `flip4` semantics: a bit lands in the hdrl LIST
        // size field, the declared extent explodes past the window, and
        // the walk still harvests the strl pairs before it.
        let mut full = full_header();
        // hdrl LIST size field sits at 16..20 (LE) — corrupt the top
        // byte like a seeded flip would.
        full[19] = 0x10;
        let window = &full[..1024.min(full.len())];
        let hit = match probe(window) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95);
        assert_eq!(hit.track_hints.len(), 2);
        assert_eq!(hit.prefix_consumed, window.len(), "walk clamped to window");
    }

    #[test]
    fn strh_without_strf_yields_no_audio_hint() {
        let mut strl = Vec::new();
        strl.extend(strh(b"auds", b"\0\0\0\0"));
        let mut hdrl = Vec::new();
        chunk(&mut hdrl, b"avih", &[0u8; 56]);
        list(&mut hdrl, b"strl", &strl);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"AVI ");
        list(&mut out, b"hdrl", &hdrl);
        let hit = match probe(&out) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 90, "no usable hint, header confirmed");
        assert!(hit.track_hints.is_empty());
    }

    #[test]
    fn pcm_width_table_names_like_the_demuxer() {
        for (tag, bits, want) in [
            (0x0001u16, 8u16, "pcm_u8"),
            (0x0001, 16, "pcm_s16le"),
            (0x0001, 24, "pcm_s24le"),
            (0x0001, 32, "pcm_s32le"),
            (0x0003, 32, "pcm_f32le"),
            (0x0003, 64, "pcm_f64le"),
        ] {
            let mut strl = Vec::new();
            strl.extend(strh(b"auds", b"\0\0\0\0"));
            strl.extend(strf_wave(tag, 2, 48000, bits));
            let mut hdrl = Vec::new();
            chunk(&mut hdrl, b"avih", &[0u8; 56]);
            list(&mut hdrl, b"strl", &strl);
            let mut out = Vec::new();
            out.extend_from_slice(b"RIFF");
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(b"AVI ");
            list(&mut out, b"hdrl", &hdrl);
            let hit = match probe(&out) {
                Some(h) => h,
                None => unreachable!("anchors"),
            };
            assert_eq!(hit.score, 95, "{want}");
            assert_eq!(hint_kinds(&hit), vec![(MediaKind::Audio, want)]);
        }
        // Unknown tags stay unhinted, never invented.
        let mut strl = Vec::new();
        strl.extend(strh(b"auds", b"\0\0\0\0"));
        strl.extend(strf_wave(0x0055, 2, 44100, 16)); // MP3 tag
        let mut hdrl = Vec::new();
        chunk(&mut hdrl, b"avih", &[0u8; 56]);
        list(&mut hdrl, b"strl", &strl);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"AVI ");
        list(&mut out, b"hdrl", &hdrl);
        let hit = match probe(&out) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert!(hit.track_hints.is_empty());
        assert_eq!(hit.score, 90);
    }

    #[test]
    fn unknown_video_handlers_stay_unhinted() {
        let mut strl = Vec::new();
        strl.extend(strh(b"vids", b"XVID"));
        strl.extend(strf_bitmap());
        let mut hdrl = Vec::new();
        chunk(&mut hdrl, b"avih", &[0u8; 56]);
        list(&mut hdrl, b"strl", &strl);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"AVI ");
        list(&mut out, b"hdrl", &hdrl);
        let hit = match probe(&out) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert!(hit.track_hints.is_empty(), "no invented codec names");
    }

    #[test]
    fn zero_size_chunks_cannot_stall_the_walk() {
        // A size-0 chunk advances by its 8-byte header; the walk ends
        // at the extent without spinning.
        let mut hdrl = Vec::new();
        chunk(&mut hdrl, b"avih", &[0u8; 56]);
        hdrl.extend_from_slice(b"JUNK");
        hdrl.extend_from_slice(&0u32.to_le_bytes());
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"AVI ");
        list(&mut out, b"hdrl", &hdrl);
        let hit = match probe(&out) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 90);
    }

    #[test]
    fn every_truncation_is_a_sane_verdict() {
        // Doc 03 §4.2(f) robustness at unit level: for every prefix
        // length the signature either declines or returns a hit on the
        // documented ladder — no panic, no out-of-range score.
        let full = full_header();
        for cut in 0..=full.len() {
            if let Some(hit) = probe(&full[..cut]) {
                assert_eq!(hit.format.as_str(), "avi");
                assert!(
                    matches!(hit.score, 70 | 80 | 90 | 95),
                    "ladder rung only, got {}",
                    hit.score
                );
                assert!(hit.prefix_consumed <= cut);
            }
        }
    }

    #[test]
    fn all_descriptor_rows_are_valid_hints() {
        // The static tables are unit-pinned: every descriptor row
        // parses, so runtime hint construction cannot silently drop.
        for (tag, bits) in [
            (0x0001u16, 8u16),
            (0x0001, 16),
            (0x0001, 24),
            (0x0001, 32),
            (0x0003, 32),
            (0x0003, 64),
        ] {
            let name = match pcm_descriptor(tag, bits) {
                Some(n) => n,
                None => unreachable!("table row exists"),
            };
            let hint = ok(TrackHint::new(MediaKind::Audio, name));
            assert!(!hint.codec.as_str().is_empty());
        }
        let hint = ok(TrackHint::new(MediaKind::Video, "mjpeg"));
        assert_eq!(hint.codec.as_str(), "mjpeg");
    }
}
