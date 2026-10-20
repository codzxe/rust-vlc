//! The MP4 / ISO-BMFF signature (card 2.A2): anchored `ftyp` brand
//! scoring plus a bounded shallow `moov` walk for track hints — the
//! cheap pre-stage to a future MP4 demuxer (2.B3 was closed
//! not-selected for the first vertical, D-018; the signature stays per
//! the card: "both container variants").
//!
//! Byte-level ground truth is the 2.A3 fixture writer's emission
//! (`build_mp4`): `ftyp` box at 0 (u32be size, type, major brand,
//! version, compatible brands), `moov` at 28, `mvhd` at 36, per-track
//! `trak > mdia > hdlr(vide/soun)` and `mdia > minf > stbl > stsd`
//! (`jpeg` visual sample entry, `sowt` audio sample entry). Boxes are
//! u32be-sized and every walk clamps to the window: an oversize lie
//! (the class-B `stszlie0` pattern — the lie sits at offset 827,
//! *inside* a 64 KiB window but in `stsz`, a box the signature never
//! opens) cannot move the scan.
//!
//! Scored ladder (see [`super`]): recognized major brand 70,
//! recognized compatible brand only 65, family-only (valid `ftyp`,
//! unknown brands) 50 — deliberately below the doc's default
//! `min_score = 60`: an unknown brand is not a selectable claim, but
//! the no-hit error still names the family. `+moov` 80, `+mvhd` 90,
//! `+stsd` track hints 95. A box whose fourcc header is visible
//! confirms its rung even when its size field is unusable, but
//! `size == 0` (ISO: "box extends to end of file") and `size < 8`
//! end a walk in v0 — the to-EOF extent is not followed (no fixture
//! or production file in scope needs it — recorded limitation); 64-bit
//! `largesize` boxes are likewise not followed in v0.

use media_types::ids::MediaKind;

use super::{
    MAX_WALK_STEPS, ProbePlugin, SCORE_COMPAT_BRAND, SCORE_CONTAINER_STRUCTURE, SCORE_FAMILY_ONLY,
    SCORE_MAGIC_ANCHOR, SCORE_PRIMARY_HEADER, SCORE_TRACK_HINTS,
};
use crate::report::{FormatId, ProbeHit, SourceMeta, TrackHint};

/// The MP4 / ISO-BMFF family signature.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mp4Signature;

impl ProbePlugin for Mp4Signature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        // Anchor: a well-formed `ftyp` box at the window start —
        // size + type + major brand + version need 16 bytes minimum.
        if prefix.len() < 16 || &prefix[4..8] != b"ftyp" {
            return None;
        }
        let ftyp_size = u32_be(prefix, 0) as usize;
        if ftyp_size < 16 {
            return None;
        }
        let ftyp_end = ftyp_size.min(prefix.len());
        let brand = &prefix[8..12];
        let mut score = if is_known_brand(brand) {
            SCORE_MAGIC_ANCHOR
        } else if compat_brands(prefix, 16, ftyp_end).any(is_known_brand) {
            SCORE_COMPAT_BRAND
        } else {
            SCORE_FAMILY_ONLY
        };
        let mut hints = Vec::new();
        let mut consumed = ftyp_end;

        // Shallow top-level walk: confirm `moov` (and only `moov`).
        // A visible box **header** (pos+8 inside the window) confirms
        // its rung even when its size is unusable — the fourcc is
        // literally in the window, and "there is a moov here" is
        // honest; but a sub-header or zero size (ISO: "extends to end
        // of file") ends the walk in v0 — the extent beyond the
        // window is unknown, so children are not followed (recorded
        // limitation, `[PROPOSAL]` refinement: treat size == 0 as
        // window-end and descend).
        let mut pos = ftyp_size;
        let mut steps = 0;
        while pos + 8 <= prefix.len() && steps < MAX_WALK_STEPS {
            steps += 1;
            let size = u32_be(prefix, pos) as usize;
            let kind = &prefix[pos + 4..pos + 8];
            if kind == b"moov" {
                score = score.max(SCORE_CONTAINER_STRUCTURE);
                if size >= 8 {
                    let box_end = pos.saturating_add(size).min(prefix.len());
                    let (saw_mvhd, mut moov_hints, moov_consumed) =
                        walk_moov(prefix, pos + 8, box_end);
                    if saw_mvhd {
                        score = score.max(SCORE_PRIMARY_HEADER);
                    }
                    if !moov_hints.is_empty() {
                        score = score.max(SCORE_TRACK_HINTS);
                        hints.append(&mut moov_hints);
                    }
                    consumed = consumed.max(moov_consumed);
                }
            }
            if size < 8 {
                break;
            }
            pos = pos.saturating_add(size);
        }

        Some(ProbeHit {
            format: FormatId::mp4(),
            score,
            track_hints: hints,
            prefix_consumed: consumed,
        })
    }
}

/// Major/compatible brands this signature recognizes as MP4 family
/// (ISO/IEC 14496-12 brand registry, common subset). `[PROPOSAL]` —
/// additive rows are safe; an absent brand falls back to the
/// family-only score rather than a guess.
const KNOWN_BRANDS: [&[u8; 4]; 18] = [
    b"isom", b"iso2", b"iso3", b"iso4", b"iso5", b"iso6", b"mp41", b"mp42", b"mp71", b"avc1",
    b"avc3", b"dash", b"m4v ", b"m4a ", b"3gp ", b"3g2 ", b"3g2a", b"qt  ",
];

/// Whether a fourcc is a recognized brand.
fn is_known_brand(fourcc: &[u8]) -> bool {
    KNOWN_BRANDS.iter().any(|brand| *brand == fourcc)
}

/// Iterate the compatible-brand fourccs inside an `ftyp` payload
/// (after the major brand at 8..12 and the minor version at 12..16,
/// 4-byte aligned).
fn compat_brands(prefix: &[u8], start: usize, end: usize) -> impl Iterator<Item = &[u8]> {
    (start..end.saturating_sub(3))
        .step_by(4)
        .map(move |at| &prefix[at..at + 4])
}

/// Walk `moov`'s children (start..end, already clamped): confirm
/// `mvhd`, sniff each `trak`. Best-effort — a lying size ends the walk
/// with what was collected. As above, a visible `mvhd` fourcc
/// confirms the primary-header rung even when its own size is broken
/// (the flag is set before the sub-header break); descent past a
/// broken box never happens.
fn walk_moov(prefix: &[u8], start: usize, end: usize) -> (bool, Vec<TrackHint>, usize) {
    let mut saw_mvhd = false;
    let mut hints = Vec::new();
    let mut pos = start;
    let mut steps = 0;
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let size = u32_be(prefix, pos) as usize;
        let kind = &prefix[pos + 4..pos + 8];
        if kind == b"mvhd" {
            saw_mvhd = true;
        }
        if size < 8 {
            break;
        }
        let box_end = pos.saturating_add(size).min(end);
        if kind == b"trak" {
            hints.extend(walk_trak(prefix, pos + 8, box_end));
        }
        pos = pos.saturating_add(size);
    }
    (saw_mvhd, hints, pos.min(end))
}

/// Walk one `trak` (children start..end): read `mdia > hdlr` for the
/// handler type and `mdia > minf > stbl > stsd` for the first sample
/// entry, then map the pair to at most one hint.
fn walk_trak(prefix: &[u8], start: usize, end: usize) -> Vec<TrackHint> {
    let mut handler: Option<&[u8]> = None;
    let mut entry: Option<&[u8]> = None;
    let mut pos = start;
    let mut steps = 0;
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let size = u32_be(prefix, pos) as usize;
        if size < 8 {
            break;
        }
        let box_end = pos.saturating_add(size).min(end);
        if &prefix[pos + 4..pos + 8] == b"mdia" {
            let (h, e) = walk_mdia(prefix, pos + 8, box_end);
            if h.is_some() {
                handler = h;
            }
            if e.is_some() {
                entry = e;
            }
        }
        pos = pos.saturating_add(size);
    }
    classify_track(handler, entry)
}

/// Walk `mdia`: the `hdlr` handler_type and the `stsd` first entry
/// (via `minf > stbl`).
fn walk_mdia(prefix: &[u8], start: usize, end: usize) -> (Option<&[u8]>, Option<&[u8]>) {
    let mut handler = None;
    let mut entry = None;
    let mut pos = start;
    let mut steps = 0;
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let size = u32_be(prefix, pos) as usize;
        if size < 8 {
            break;
        }
        let box_end = pos.saturating_add(size).min(end);
        let kind = &prefix[pos + 4..pos + 8];
        if kind == b"hdlr" && box_end >= pos + 8 + 12 {
            // FullBox version/flags (4) + pre_defined (4) + handler.
            handler = Some(&prefix[pos + 8 + 8..pos + 8 + 12]);
        }
        if kind == b"minf"
            && let Some(found) = find_stsd_entry(prefix, pos + 8, box_end)
        {
            entry = Some(found);
        }
        pos = pos.saturating_add(size);
    }
    (handler, entry)
}

/// Find the first `stsd` sample-entry fourcc inside a `minf` (via its
/// `stbl`), if the boxes and the entry are fully inside the window.
fn find_stsd_entry(prefix: &[u8], start: usize, end: usize) -> Option<&[u8]> {
    let mut pos = start;
    let mut steps = 0;
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let size = u32_be(prefix, pos) as usize;
        if size < 8 {
            return None;
        }
        let box_end = pos.saturating_add(size).min(end);
        let kind = &prefix[pos + 4..pos + 8];
        if kind == b"stsd" {
            // FullBox version/flags (4) + entry_count (4) + entries;
            // one entry = u32be size + format fourcc. Sanity: the
            // count is small and the first entry's header is inside.
            if box_end < pos + 8 + 8 + 8 {
                return None;
            }
            let count = u32_be(prefix, pos + 8 + 4);
            if !(1..=16).contains(&count) {
                return None;
            }
            return Some(&prefix[pos + 8 + 8 + 4..pos + 8 + 8 + 8]);
        }
        if kind == b"stbl"
            && let Some(found) = find_stsd_entry(prefix, pos + 8, box_end)
        {
            return Some(found);
        }
        pos = pos.saturating_add(size);
    }
    None
}

/// Map a sniffed (`hdlr`, `stsd` entry) pair to a hint.
///
/// Only pairs with in-repo ground truth produce hints — the fixture
/// corpus's `vide`+`jpeg` and `soun`+`sowt`, named exactly as the AVI
/// side and the expectation TOMLs name them (`mjpeg`, `pcm_s16le`).
/// Other fourccs stay unhinted: their descriptor names are pinned when
/// their decoder cards land (additive rows).
fn classify_track(handler: Option<&[u8]>, entry: Option<&[u8]>) -> Vec<TrackHint> {
    let (Some(handler), Some(entry)) = (handler, entry) else {
        return Vec::new();
    };
    let descriptor = if handler == b"vide" && matches!(entry, b"jpeg" | b"mjpa" | b"mjpb") {
        Some((MediaKind::Video, "mjpeg"))
    } else if handler == b"soun" && matches!(entry, b"sowt" | b"twos") {
        Some((MediaKind::Audio, "pcm_s16le"))
    } else {
        None
    };
    match descriptor.and_then(|(kind, name)| TrackHint::new(kind, name).ok()) {
        Some(hint) => vec![hint],
        None => Vec::new(),
    }
}

/// Big-endian u32 at `at` (ISO boxes are big-endian throughout).
fn u32_be(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(prefix: &[u8]) -> Option<ProbeHit> {
        Mp4Signature.probe(prefix, &SourceMeta::new(None, None))
    }

    // ------------------------------------------------------------------
    // A tiny MP4 builder (mirrors the 2.A3 writer: u32be box sizes,
    // FullBox payloads, patched moov extent).
    // ------------------------------------------------------------------

    fn box_bytes(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + payload.len());
        out.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        out
    }

    fn fullbox(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut payload = vec![0u8; 4];
        payload.extend_from_slice(body);
        box_bytes(kind, &payload)
    }

    fn ftyp(major: &[u8; 4], compat: &[&[u8; 4]]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(major);
        payload.extend_from_slice(&0u32.to_be_bytes());
        for brand in compat {
            payload.extend_from_slice(*brand);
        }
        box_bytes(b"ftyp", &payload)
    }

    fn hdlr(handler: &[u8; 4]) -> Vec<u8> {
        let mut body = vec![0u8; 4]; // pre_defined
        body.extend_from_slice(handler);
        body.extend_from_slice(&[0u8; 12]); // reserved + name
        fullbox(b"hdlr", &body)
    }

    fn stsd(entry: &[u8; 4]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        let mut entry_box = Vec::new();
        entry_box.extend_from_slice(&16u32.to_be_bytes()); // entry size
        entry_box.extend_from_slice(entry);
        entry_box.extend_from_slice(&[0u8; 8]); // entry body (stub)
        body.extend_from_slice(&entry_box);
        fullbox(b"stsd", &body)
    }

    fn stbl(entry: &[u8; 4]) -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend(stsd(entry));
        inner.extend(fullbox(b"stts", &[0u8; 8]));
        inner.extend(fullbox(b"stsz", &[0u8; 12]));
        box_bytes(b"stbl", &inner)
    }

    fn trak(handler: &[u8; 4], entry: &[u8; 4]) -> Vec<u8> {
        let mut mdia = Vec::new();
        mdia.extend(fullbox(b"mdhd", &[0u8; 20]));
        mdia.extend(hdlr(handler));
        let mut minf = Vec::new();
        minf.extend(fullbox(b"vmhd", &[0u8; 8]));
        minf.extend(stbl(entry));
        mdia.extend(box_bytes(b"minf", &minf));
        let mut trak = Vec::new();
        trak.extend(fullbox(b"tkhd", &[0u8; 80]));
        trak.extend(box_bytes(b"mdia", &mdia));
        box_bytes(b"trak", &trak)
    }

    fn moov() -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend(fullbox(b"mvhd", &[0u8; 96]));
        inner.extend(trak(b"vide", b"jpeg"));
        inner.extend(trak(b"soun", b"sowt"));
        box_bytes(b"moov", &inner)
    }

    fn full_file() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(ftyp(b"isom", &[b"isom", b"iso2", b"mp41"]));
        out.extend(moov());
        out.extend(box_bytes(b"mdat", &[0xAA; 32]));
        out
    }

    fn hint_kinds(hit: &ProbeHit) -> Vec<(MediaKind, &str)> {
        hit.track_hints
            .iter()
            .map(|h| (h.kind, h.codec.as_str()))
            .collect()
    }

    #[test]
    fn recognized_major_brand_anchors_at_seventy() {
        // The class-B `trunc0` shape byte-for-byte: the writer's ftyp
        // is 28 bytes — major `isom`, version 0, three compatible
        // brands `isom`/`iso2`/`mp41` (fixtures.rs line 1951).
        let window = ftyp(b"isom", &[b"isom", b"iso2", b"mp41"]);
        assert_eq!(window.len(), 28);
        let hit = match probe(&window) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.format.as_str(), "mp4");
        assert_eq!(hit.score, 70);
        assert!(hit.track_hints.is_empty());
        assert_eq!(hit.prefix_consumed, 28);
    }

    #[test]
    fn short_or_foreign_windows_do_not_anchor() {
        for window in [
            &b""[..],
            &b"\0\0\0\x1cftyp"[..],
            &b"\0\0\0\x0cftypisom\0\0\0\0"[..], // size < 16
            &b"\0\0\0\x1cft9pisom\0\0\0\0"[..], // broken type
            &b"\0\0\0\x1cmoovisom\0\0\0\0"[..], // wrong box type
        ] {
            assert!(probe(window).is_none(), "must not anchor: {window:?}");
        }
    }

    #[test]
    fn brand_ladder_is_honest() {
        // Unknown major, recognized compatible → 65 (selectable).
        let compat_only = ftyp(b"lumo", &[b"mp41"]);
        let hit = match probe(&compat_only) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 65);
        // Unknown everywhere → family-only 50 (below the default
        // min_score of 60: not selectable, but the error names mp4).
        let unknown = ftyp(b"lumo", &[b"zumo"]);
        let hit = match probe(&unknown) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 50);
        // The minor-version field is never mistaken for a brand: a
        // version whose bytes spell a known brand stays family-only.
        let mut tricky = ftyp(b"lumo", &[b"zumo"]);
        tricky[12..16].copy_from_slice(b"isom");
        let hit = match probe(&tricky) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 50, "version field is not a compatible brand");
    }

    #[test]
    fn moov_and_mvhd_climb_the_ladder() {
        let file = full_file();
        let ftyp_len = 28;
        // ftyp + moov header + mvhd header: the `trunc1` shape (64
        // bytes of the real fixture — here 28 + 8 + 8 = 44 for the
        // synthetic minimal sizes, same rungs).
        let cut = ftyp_len + 8 + 8;
        let hit = match probe(&file[..cut]) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 90, "moov + mvhd confirmed");
        assert!(hit.track_hints.is_empty());
        // Just the moov box header (no mvhd visible yet): 80.
        let hit = match probe(&file[..ftyp_len + 8]) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 80);
    }

    #[test]
    fn full_moov_sniffs_both_tracks() {
        let file = full_file();
        let moov_len = file.len() - 28 - 40; // minus ftyp + mdat
        let hit = match probe(&file[..28 + moov_len]) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95);
        assert_eq!(
            hint_kinds(&hit),
            vec![(MediaKind::Video, "mjpeg"), (MediaKind::Audio, "pcm_s16le")]
        );
        // Over the whole file (mdat skipped by size, not parsed).
        let hit = match probe(&file) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95);
        assert_eq!(hit.track_hints.len(), 2);
    }

    #[test]
    fn moov_after_mdat_is_still_found() {
        // A large leading mdat does not hide a later moov when both
        // fit the window (real-world moov-at-end files, small enough).
        let mut file = Vec::new();
        file.extend(ftyp(b"isom", &[b"mp41"]));
        file.extend(box_bytes(b"mdat", &[0x55; 64]));
        file.extend(moov());
        let hit = match probe(&file) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95);
        assert_eq!(hit.track_hints.len(), 2);
        // But when mdat pushes moov past the window, the score stays
        // at the anchor rung — honest, not guessed (a later, larger
        // window would find it; the probe never re-probes deeper than
        // its config, doc 03 §4.2 one-shot probing).
        let mut big = Vec::new();
        big.extend(ftyp(b"isom", &[b"mp41"]));
        big.extend(box_bytes(b"mdat", &[0x55; 512]));
        big.extend(moov());
        let hit = match probe(&big[..200]) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 70);
        assert!(hit.track_hints.is_empty());
    }

    #[test]
    fn broken_box_sizes_end_the_walk() {
        let mut file = full_file();
        // Corrupt the moov size field (28..32) to a huge value: the
        // walk clamps to the window and still harvests both traks.
        file[28..32].copy_from_slice(&0xFFFF_0000u32.to_be_bytes());
        let hit = match probe(&file) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95);
        assert_eq!(hit.track_hints.len(), 2);
        // Corrupt the first trak's size to a sub-header value: the
        // moov walk stops there — mvhd (which precedes) still counts.
        let mut file = full_file();
        // moov starts at 28: header 8, mvhd 108, trak1 next.
        let trak1 = 28 + 8 + 108;
        file[trak1..trak1 + 4].copy_from_slice(&4u32.to_be_bytes());
        let hit = match probe(&file) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 90, "mvhd only — the walk stopped at the lie");
        assert!(hit.track_hints.is_empty());
    }

    #[test]
    fn zero_size_box_terminates_the_walk() {
        let mut file = full_file();
        // size == 0 means "extends to end of file" — treat as end.
        file[28..32].copy_from_slice(&0u32.to_be_bytes());
        let hit = match probe(&file) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 80, "moov header seen, children unreachable");
    }

    #[test]
    fn unmapped_sample_entries_stay_unhinted() {
        let mut file = Vec::new();
        file.extend(ftyp(b"isom", &[b"mp41"]));
        let mut inner = Vec::new();
        inner.extend(fullbox(b"mvhd", &[0u8; 96]));
        inner.extend(trak(b"vide", b"avc1")); // h264: no decoder card yet
        inner.extend(trak(b"soun", b"mp4a")); // aac: no decoder card yet
        file.extend(box_bytes(b"moov", &inner));
        let hit = match probe(&file) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 90, "structure confirmed, no hints invented");
        assert!(hit.track_hints.is_empty());
    }

    #[test]
    fn stsd_entry_count_lies_are_ignored() {
        // The class-B `stszlie0` cousin at stsd: an absurd entry_count
        // makes the entry unusable — hint dropped, format intact.
        let mut file = full_file();
        // Find the first stsd (the video trak's) and corrupt its
        // entry_count. The search lands on the TYPE fourcc; the box
        // layout from there is type+4 version/flags, then the count.
        let at = match find(&file, b"stsd") {
            Some(at) => at,
            None => unreachable!("stsd present"),
        };
        file[at + 8..at + 12].copy_from_slice(&0x7FFF_FFFFu32.to_be_bytes());
        let hit = match probe(&file) {
            Some(h) => h,
            None => unreachable!("anchors"),
        };
        assert_eq!(hit.score, 95, "the audio trak still hints");
        assert_eq!(hit.track_hints.len(), 1);
        assert_eq!(hint_kinds(&hit), vec![(MediaKind::Audio, "pcm_s16le")]);
    }

    #[test]
    fn every_truncation_is_a_sane_verdict() {
        let file = full_file();
        for cut in 0..=file.len() {
            if let Some(hit) = probe(&file[..cut]) {
                assert_eq!(hit.format.as_str(), "mp4");
                assert!(
                    matches!(hit.score, 50 | 65 | 70 | 80 | 90 | 95),
                    "ladder rung only, got {}",
                    hit.score
                );
                assert!(hit.prefix_consumed <= cut);
            }
        }
    }

    /// Find the first occurrence of a fourcc, byte-wise (no alignment
    /// assumption — payload bytes can mirror box types).
    fn find(haystack: &[u8], needle: &[u8; 4]) -> Option<usize> {
        (0..haystack.len().saturating_sub(3)).find(|&at| &haystack[at..at + 4] == needle)
    }
}
