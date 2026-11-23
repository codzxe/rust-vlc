# AUDIT: crates/demux/src/mkv/header.rs
| Field | Value |
|---|---|
| Auditor | W05-A07 |
| Date | 2025-05-24 |
| Lines / functions | 1033 / 13 fns + 4 tests (2 chunks, one file) |
| File verdict | REAL-UNPROVEN (core parse real + tests pass; two doc claims FALSE, one doctrine violation) |
| Reachable from shipped entry point? | YES: apps/cli main -> containers/mod.rs:57 `"matroska" => mkv::open` -> containers/mkv.rs:147 `MkvDemuxer::new` -> mkv/mod.rs:342/408/412 `header::parse_ebml_header`/`parse_info`/`parse_tracks` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 3 |
| Commands run | `cargo test -p demux --lib mkv::header` -> 4 passed, exit 0; sed quote verification (12 blocks, all byte-verbatim); rg reachability/damage-string searches |

## 1. One-paragraph verdict
The Info/Tracks parser is real, reachable, and its happy paths are test-proven (my run: 4/4). Hostile-focus answers: EBML Void/CRC-32/unknown elements ARE skipped by declared size at every level; DocType gating EXISTS but lives in mod.rs (header.rs only parses); `V_MPEG4/ISO/AVC` -> `h264` (yes), `V_VP9` -> `vp9` (yes, decode refusal is downstream by design). But the file's own robustness doc is partly false: a float wider than 8 bytes is accepted (not `bad_structure`), child reads are bounded by the child's declared end (not the parent's), and the per-track containment doctrine is broken at the `track()` seam — one viable entry with an empty/control-char CodecID aborts the whole file open mislabeled as `no_tracks`. The damage-row doc says "absolute offset" while the code stores byte counts.

## 2. What actually works (verified)
— `cargo test -p demux --lib mkv::header` -> `test result: ok. 4 passed; 0 failed` (exit 0). Covers the TrackType map, Info defaults/duration ticks, Info Title + TrackUID parse, and 30+ codec descriptor mappings. Baseline corroborates: audit/runs/baseline/cargo-test-workspace.txt:1967-1970 (4 header tests ok).

## 3. Findings
F-001 [S2]
Where: crates/demux/src/mkv/header.rs:664-671 + 227-228; crates/demux/src/mkv/mod.rs:446-450; crates/media-types/src/ids.rs:582-583
Code:
```
    let codec_id = codec_id.unwrap_or_default();
    // Mapped before the move into `TrackPrivate` (the private bytes
    // feed the VFW fourcc lane of the descriptor).
    let codec_descriptor = codec_descriptor_for(&codec_id, bit_depth, &codec_private);
```
```
        let codec = CodecDescriptor::new(&self.codec_descriptor).map_err(|_| error::no_tracks())?;
```
```
        self.tracks = self
            .entries
            .iter()
            .map(|e| e.track(self.base))
            .collect::<Result<Vec<_>, _>>()?;
```
What is wrong: A TrackEntry with a *missing or zero-length* CodecID (spec-mandatory element, but this parser's degenerate list at lines 13-16 only covers missing number/unknown type/over-ceiling CodecID) survives `parse_tracks` with `codec_id = ""`; `codec_descriptor_for("")` returns `""` (falls through every branch to `codec_id.to_string()`); `CodecDescriptor::new("")` rejects empty (ids.rs:583); `track()` maps that to `error::no_tracks()`; mod.rs:450 `?` aborts `open()` for the ENTIRE file. A CodecID containing an ASCII control char (valid UTF-8, passes `utf8_from`) hits the same wall. Why it matters: the module doc promises "a degenerate TrackEntry ... skips that track with damage — the other tracks and the file survive" — one unlabeled track in a 5-track file kills all 5, and the error lies ("no tracks" when the table has tracks). What correct looks like: treat an empty/control-char descriptor as a degenerate entry (skip with damage at parse time) or skip the failing entry in the mod.rs collect. Confidence: HIGH — Verification: read (full chain traced; not executed: crafting a CodecID-less MKV requires a fixture build, cargo-build-budget not spent).

F-002 [S3]
Where: crates/demux/src/mkv/header.rs:444-445 vs 474, 483
Code:
```
/// Track-table damage rows: (absolute offset, reason).
pub type TrackDamage = Vec<(u64, &'static str)>;
```
```
            if entries.len() >= MAX_TRACKS {
                damage.push((child_end - pos, "track-over-ceiling"));
```
What is wrong: The type doc says the first tuple field is an *absolute offset*; the code pushes `child_end - pos`, a byte count. mod.rs:414 confirms the consumer reads it as bytes (`self.damage_total += damage.iter().map(|(n, _)| *n).sum::<u64>()` feeding "Total damage bytes skipped"). Any future consumer following the type doc prints a length where an offset belongs. Why it matters: the two doc sentences in the same file disagree (line 445 "absolute offset" vs line 448 "byte count"); the public type doc is the false one. What correct looks like: fix the doc to "(byte count, reason)". Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/demux/src/mkv/header.rs:17-19 (doc) vs 430-435 (code)
Code:
```
//! - Fixed-width fields obey the EBML grammar: a uint wider than 8
//!   bytes or a float wider than 8 is corruption, not a big number
//!   (`bad_structure`, never an oversized allocation).
```
```
            ID_DURATION => {
                let bytes = reader.read_prefix_at(len.min(8), child.data_pos, child_end)?;
                match ebml::float_from_be(&bytes) {
                    Some(value) => parsed.duration_secs = Some(value),
```
What is wrong: For `Duration` (and `SamplingFrequency`, line 757, identical `len.min(8)` shape) a declared width > 8 is *accepted* by silently reading the first 8 bytes — the doc's promised `bad_structure` never fires on this path. (Widths 1-3 and 5-7 do fail via `float_from_be` -> None.) Why it matters: the module's headline robustness contract is quoted in reviews; this bullet is false for floats. No memory-safety impact (read capped at 8), so S3 not S2. What correct looks like: `if len > 8 { return Err(bad_structure(...)) }` before the read, matching the uint fields. Confidence: HIGH — Verification: read.

F-004 [S3]
Where: crates/demux/src/mkv/header.rs:6-9 (doc) vs 401-413 (code)
Code:
```
//! - Declared element sizes are hints: every child read is clamped to
//!   its parent's effective end and the stream's physical end; a
//!   truncated child is the typed truncation at its own boundary, a
//!   truncated parent walk stops at the bound.
```
```
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
```
What is wrong: `parse_header_at` only bounds the id+size vints by the parent end (ebml.rs:450 `let window = (pos + 12).min(bound);`); a child whose declared size exceeds the parent extent gets `child_end = data_pos + size` *beyond* the parent, and every value read uses `child_end` as its `until` bound (e.g. line 413 `read_exact_at(len, child.data_pos, child_end)`), so field bytes can be sourced from outside the parent element (up to the stream end). The walk then exits (`pos = child_end > end`). Contained by stream length + the 8 B / 4 KiB / 4 MiB ceilings, so no security hole — but the "clamped to its parent's effective end" bullet is not what the code does. What correct looks like: clamp `child_end` to the parent `end` (or reject the over-running child as `bad_structure`, which is what mkv/error.rs's own `bad_structure` doc claims: "a known element whose declared extent contradicts its parent"). Confidence: HIGH — Verification: read (ebml.rs:439-488 traced).

F-005 [S3]
Where: crates/demux/src/mkv/header.rs:40-43; crates/demux/src/mkv/mod.rs:343-345
Code:
```
/// `matroska` — the DocType of general MKV files.
pub const DOCTYPE_MATROSKA: &str = "matroska";
/// `webm` — the DocType of the browser-media subset.
pub const DOCTYPE_WEBM: &str = "webm";
```
```
        match ebml_header.doctype.as_str() {
            "matroska" | "webm" => {}
```
What is wrong: The gate hardcodes the literals; `rg` shows `DOCTYPE_MATROSKA`/`DOCTYPE_WEBM` referenced nowhere else in the repo — dead `pub` constants whose only value is drift risk (change the constant, nothing changes). Why it matters: dead code + a second source of truth for the accepted-DoType policy. What correct looks like: gate on the constants or delete them. Confidence: HIGH — Verification: read + `rg` (only the two definition lines match).

F-006 [S4]
Where: crates/demux/src/mkv/header.rs:530-535
Code:
```
            ID_TRACK_NUMBER => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                number =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
            }
```
What is wrong: A zero-length (or explicit 0) TrackNumber yields `Some(0)` (ebml.rs:491 "empty = 0"), which passes the viability check at line 664 — Matroska requires TrackNumber > 0. The entry occupies a table slot/ordinal and keys `by_number[0]`. Corrupt-file-only blast radius. What correct looks like: reject `number == 0` as degenerate. Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/demux/src/mkv/header.rs:851-855
Code:
```
/// Silence the `MAX_PARSE_DEPTH` import when the fixed-shape parsers
/// above carry the depth bound structurally (the constant documents the
/// bound the walk shape enforces; the cluster walker uses it at
/// runtime).
#[cfg(test)]
mod tests {
```
What is wrong: The comment refers to silencing a `MAX_PARSE_DEPTH` import that does not exist in this file (import list lines 29-38 has no such name; `rg MAX_PARSE_DEPTH` in this file matches only this comment). Orphaned leftover describing a removed `use`. Confidence: HIGH — Verification: read + rg.

F-008 [S4]
Where: crates/demux/src/mkv/header.rs:784-786, 823-824
Code:
```
    if codec_id.starts_with("V_MPEG4/ISO/AVC") {
        return "h264".to_string();
    }
```
```
    if codec_id.starts_with("A_AAC") {
        return "aac".to_string();
    }
```
What is wrong: Prefix matching collapses every AAC profile (LC/HE/Main/LTP/SBR...) to one `aac` descriptor and maps any hypothetical `V_MPEG4/ISO/AVC*` suffix id to `h264`. No real-world CodecID collides today (verified against the registry vocabulary test), so hygiene-grade. What correct looks like: exact matches plus the known profile suffixes, or an explicit profile carry-through. Confidence: MEDIUM — Verification: read (no collision demonstrated because none exists in the spec's id list).

F-009 [S3]
Where: crates/demux/src/mkv/header.rs:473-483 (production) + tests 894-1032 (absence)
Code:
```
                    None => damage.push((child_end - pos, "degenerate-track-entry")),
```
What is wrong: The module's headline doctrine — per-track damage containment (`track-over-ceiling`, `degenerate-track-entry`) — has ZERO test coverage: `rg` over the whole repo matches both strings only at their production sites (header.rs:474,483). The four in-file tests are happy-path (2 clean entries, `assert!(damage.is_empty())` at line 966); fixtures/corpus tests cover CodecID only as a builder input (mkv_fixtures.rs:250). If the containment regressed to "return Err on first bad track", no test would fail. What correct looks like: a test with a missing-TrackNumber entry next to a good one asserting damage=1/entries=1, and a 65-track file asserting the ceiling. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SegmentInfo::default` (impl Default) | 98-107 | REAL | — | spec default scale 1e6, correct |
| `SegmentInfo::time_base` | 117-120 | REAL | — | scale 0 -> TimeBase err -> bad_structure; verified vs time.rs:76-86 |
| `SegmentInfo::duration_ticks` | 124-135 | REAL | — | finite/neg/overflow guarded; tested |
| `media_kind_from_track_type` | 141-149 | REAL | — | 1/2/0x11/0x20-0x3F correct; tested |
| `TrackEntry::track` | 227-249 | REAL | S2 | F-001 poison seam + mislabeled no_tracks |
| `child_len` | 257-259 | REAL | — | try_from typed; only fires on 32-bit |
| `parse_ebml_header` | 273-378 | REAL | — | magic, DocType presence, vint caps enforced (0 or >4/>8 refused); Void/CRC/unknown skipped by size |
| `parse_info` | 389-442 | REAL | S3 | F-003 (wide Duration), F-004 (parent escape); scale<=u32::MAX, scale 0 stored-then-typed (documented) |
| `parse_tracks` | 456-491 | REAL | S3 | F-002 damage-doc lie; MAX_TRACKS=64 enforced; F-009 untested paths |
| `parse_track_entry` | 495-704 | REAL | S2/S4 | F-001 empty CodecID survives; F-006 TrackNumber 0; uint len>8 all typed; CodecPrivate 4 MiB ceiling never read past |
| `parse_video_master` | 707-734 | REAL | — | len>8 children silently ignored (lenient, consistent) |
| `parse_audio_master` | 738-773 | REAL | S3 | F-003 twin (SamplingFrequency min(8)) |
| `codec_descriptor_for` | 779-849 | REAL | S4 | F-008 prefix over-match; VFW biCompression offset 16 correct; 30+ mappings test-verified |
| tests: `id_bytes`/`size_bytes`/`el`/`uint_payload` | 860-892 | REAL | — | correct vint encoders (size_bytes caps at 8-byte form) |
| test `track_type_maps_to_the_l0_vocabulary` | 894-903 | REAL | — | would fail on map break |
| test `segment_info_defaults_and_duration_ticks` | 905-930 | REAL | — | covers NaN/negative; would fail on drift |
| test `info_title_and_track_uid_parse_into_the_models` | 932-973 | REAL-UNPROVEN | — | happy path only; no damage/ceiling cases (F-009) |
| test `codec_descriptors_follow_the_registry_vocabulary` | 975-1032 | REAL | — | 30+ asserts incl. raw pass-through; strong |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| track_type_maps_to_the_l0_vocabulary | YES | exhaustive match asserts, ran ok |
| segment_info_defaults_and_duration_ticks | YES | asserts exact tick values + NaN/neg guards, ran ok |
| info_title_and_track_uid_parse_into_the_models | PARTLY | would catch Title/UID/ordinal regressions; asserts `damage.is_empty()` so it does NOT cover the containment doctrine the module advertises |
| codec_descriptors_follow_the_registry_vocabulary | YES | 30+ exact mapping asserts incl. unknown-raw and VFW lanes, ran ok |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "float wider than 8 is corruption, bad_structure" | header.rs:17-19 | FALSE | lines 431/757 `len.min(8)` accept >8-byte floats (F-003) |
| "every child read is clamped to its parent's effective end" | header.rs:6-7 | FALSE | reads bound by child_end, only vints by parent (F-004, ebml.rs:450) |
| "degenerate TrackEntry skips that track ... other tracks survive" | header.rs:13-15 | FALSE at the track() seam | F-001: empty/control-char CodecID aborts whole open (mod.rs:446-450) |
| "TrackUID ... retained, no longer dropped (issue #34)" | header.rs:195-199 | REAL | parsed at 537-545, tested line 968-969 |
| "V_MPEG4/ISO/AVC -> h264, V_VP9 -> vp9 mapping" (hostile focus) | header.rs:784-786, 805 | REAL | test lines 977, 980; ran ok |
| "VP8/VP9/AV1 demuxed but NOT decodable (typed Unsupported at decode)" | mod.rs:19-23 | REAL-UNPROVEN | mapping real; decode refusal is downstream (out of this file; see baseline engine-info: decoders mjpeg, pcm_s16le) |
| "TrackDamage rows: (absolute offset, reason)" | header.rs:445 | FALSE | byte counts pushed (F-002) |

## 7. Dead / duplicated / unreachable code
- `DOCTYPE_MATROSKA`/`DOCTYPE_WEBM` (40-43): dead pub constants, gate uses literals (F-005).
- Orphaned `MAX_PARSE_DEPTH` comment (851-854) referencing a nonexistent import (F-007).
- `TrackPrivate.lacing_allowed` is parsed (593-599) but documented as informational-only; the block lacing bits rule — honest per doc, noted for the cluster auditor.

## 8. Dependencies & flags
Depends on: `media_types` (ids/time/error), `crate::mkv::ebml` (reader + 34 element IDs), `crate::mkv::error`, and test-only `crate::avi::test_support::mem_stream` + `testkit::Must` (cross-module test reach into avi — layering smell, hygiene). No feature gates in this file; demux crate is unconditionally compiled into the shipped CLI (containers/mod.rs:57). No unsafe.

## 9. Open questions for the next agent
- Does any real-world muxer emit a TrackEntry without CodecID? If yes, F-001 is user-visible file-refusal, not just doctrine violation — promote severity.
- `media-probe`/`playlists` also key off `mkv::probe`; confirm their DocType handling agrees with mod.rs:343-345 (they only sniff the EBML magic, so a `webm`-doctyped file with .mkv extension routes here — fine).
- The baseline "which registry lies" question (engine-info lists h264 landed; decode-bench says no decoder in first-vertical build) — the header-side mapping target string is exactly "h264"; the CLI host (containers/mkv.rs harvest_facts) should be audited for what it does with `TrackPrivate.codec_private` avcC.
