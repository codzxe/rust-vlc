# AUDIT: crates/demux/src/mp4/mod.rs#part2 (lines 557–1112; function-integrity overlap 542–570)
| Field | Value |
|---|---|
| Auditor | W05-A07 |
| Date | 2026-11-21 |
| Lines / functions | 556 audited (assigned ≈557–1112; start pulled back to 542 to keep `oti_of_esds` whole — 15-line overlap with A08's part1, not re-audited; `parse_trak` (1045–1426) is split at 1112: my portion = header boxes through `minf`, A06's part3 owns 1113+ incl. stsd/stbl) / 18 items in table |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open <mp4/m4a/mov>` → media-probe `mp4` signature → apps/cli/src/containers/mod.rs `"mp4" => mp4::open` → Mp4Demuxer::new + `start()` (mod.rs:2009) → `open()` (697, this chunk) → `parse_moov`/`parse_trak`/`track_record` (this chunk). EXECUTED in baseline: mp4-smoke-open-session-420.txt:4-9 (`event: tracks 2 ready`, `MediaInfoReady { ... tracks: [Video h264, Audio aac] }`, then Playing — the file PLAYS end-to-end); mp4-smoke-open-session.txt:5-9 (moov-at-end yuv444p sample: demux yields 2 tracks + duration, decoder refuses later — demux side works); 38 `mp4::tests` ok (nofailfast :1764+), mp4_fixtures + matrix_mp4_* suites ok. |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:4 |
| Commands run | rg "major_brand()/compatible_brands()/is_fragmented()/.chapters()/sample_entry_bytes(/pool_bytes(" over crates+apps (callers found: sample_entry_bytes → containers/mp4.rs:190, chapters → containers/mp4.rs:640; compatible_brands/pool_bytes → none); rg "voe" in baseline smoke logs (both MP4 sessions, all tracks); python arithmetic decode of packed language 0x55C4; sed -n / python-repl verification of every quote (mind the tool display artifact: literal `[m` renders truncated in Bash output — byte-checked via the Read tool); rg "chpl" over mp4 tests/fixtures/CLI (no coverage); rg "multi_edit" (consumed at 1208, part3); sed of avi/riff.rs:538-568 (PayloadSlots::new starts with empty arenas — demand-driven, no eager 32 MiB); sed 183-196 + 636-645 of containers/mp4.rs (harvest_facts uses sample_entry_bytes; chapters used in the chapters CLI test). |

## 1. One-paragraph verdict
This chunk is the MP4 open path the owner's bug report is about, and it is genuinely implemented and proven working by baseline runs: the top-level box walk (ftyp/moov, moov-at-end tolerated, bounded to 65 536 boxes), the 16 MiB-capped whole-moov read, correct mvhd/tkhd/edts-elst/mdhd/hdlr/minf field offsets (verified against ISO 14496-12 layouts), damage-noted per-trak failures that never kill the file, and the sample-entry handoff the decoder config depends on. The one real defect found is a +1 alphabet shift in the mdhd language unpacker: ISO packs each letter as `char − 0x60`, the code adds `b'a'` (0x61), so the canonical "und" (0x55C4) decodes as `"voe"` — exactly the garbage `language: Some("voe")` the baseline smoke logs show on every MP4 track, with zero test coverage of language anywhere in the mp4 suites. The rest is small-bore: an untested self-admitted-guess `chpl` chapter layout with a silent 64-chapter cap, a swallowed I/O error in the moof probe, wrapping u64→i64 casts in `track_record`, and two dead observables.

## 2. What actually works (verified)
- The whole open path for real MP4s — verified by baseline `mp4-smoke-open-session-420.txt` (tracks 2 ready → Buffering → Playing → Stopped, EXIT=0) and `mp4-smoke-open-session.txt` (moov-at-end sample: tracks + duration + MediaInfoReady with h264/aac, refusal happens later at the decoder) — both named in `audit/runs/baseline/INDEX.md` CORRECTION v2.
- mvhd/trex/tkhd/elst(v0+v1)/mdhd(v0+v1)/hdlr field offsets — verified line-by-line against the ISO 14496-12 box layouts (mod.rs:917-925, 936-946, 1064-1096, 1102-1125); elst v0's signed media_time handled via `as i32` (1087) ✓.
- Damage-noted track isolation: a trak that fails to parse is noted ("trak-unparseable", 976) and the file continues — verified by design read + baseline `mp4::tests` matrix suites (trunc*/stszlie* all typed, nofailfast :1764+).
- Bounded everything: scan budget 65 536 boxes (717-719), moov ≤ 16 MiB (767-769), anti-spin guard (755-757), pool arenas demand-driven (riff.rs:542-549), all `body.get`-bounded reads in chpl/esds parsing (no index panics possible in this chunk).
- `sample_entry_bytes` → `containers/mp4.rs:190` harvest_facts: the verbatim stsd entry is handed to the codec-config blob — the avcC/hvcC/esds decoder-config handoff actually wired (grep-verified call site).
- `chapters()` → `containers/mp4.rs:640` — the chapter surface is consumed by the CLI host test path.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mp4/mod.rs:535-538 (root cause — 15-line overlap into A08's part1; defect traced to this wave per INDEX standing-defect list) and 880-883 (this chunk — propagation into `Track.language`)
Code (535-538, byte-verified via the Read tool):
```
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
```
and (880-883, this chunk):
```
        let language = t
            .language
            .as_ref()
            .and_then(|tag| media_types::ids::LanguageTag::parse(tag).ok());
```
What is wrong: ISO 14496-12 §12.1.3 packs each mdhd language letter as `letter − 0x60` in 5 bits (0x60 base, not 'a' = 0x61). With the 0x61 base here, every decoded letter is shifted +1: the canonical "undefined" code 0x55C4 (20/14/4) decodes as 'a'+21='v', 'a'+14='o', 'a'+4='e' → `"voe"`, which fails the `== "und"` check and is returned as a real language. Arithmetic proof: 0x55C4>>10 & 0x1F = 21 → 'v'; >>5 & 0x1F = 14 → 'o'; & 0x1F = 4 → 'e'. Every genuine tag is equally wrong ('eng' 0x15C7 → "foh").
Why it matters: This is the exact standing defect `audit/runs/baseline/INDEX.md` line 141 assigns to the demux wave: "TrackInfo `language: Some(\"voe\")` on tracks with no language metadata — garbage default (demux wave to trace)". Confirmed live in BOTH baseline MP4 sessions (`mp4-smoke-open-session-420.txt:9` and `mp4-smoke-open-session.txt:9`: `language: Some("voe")` on every track of the file that PLAYS — so this fires on the happy path of every MP4 ever opened, not just oddities). `LanguageTag::parse("voe")` accepts the garbage (it is a 3-letter tag), so it surfaces to users. No mp4 test asserts any language value (rg over mp4/tests.rs and mp4_fixtures.rs: zero matches for language/"und"/"voe") — 38 passing mp4::tests never noticed.
What correct looks like: `(0x60u8 + ((packed >> shift) & 0x1F)) as char` (ffmpeg's mov_parse_iso_language does exactly this), keep "und" → None, and add a fixture test pinning mdhd "und"/"eng" round-trips.
Confidence: HIGH — Verification: read + arithmetic + baseline run output (both smoke logs, quoted verbatim above from the files).

### F-002 [S4]
Where: crates/demux/src/mp4/mod.rs:1013-1021
Code:
```
        // version/flags(4) + reserved(4)? Nero layout: 1+3+4 then
        // count(u8?) — ffmpeg reads: 4 skip, 4 skip, u8 count... The
        // widely-interop shape: v/f(4) reserved(4) count(u8) then
        // entries { u64 start(100ns), u8 len, bytes }.
        let mut at = 9usize;
```
(plus 1021 `for _ in 0..count.min(64) {`)
What is wrong: The `chpl` layout is a self-admitted guess ("? Nero layout… ffmpeg reads…" with question marks in the code's own comment), it has zero test coverage (rg "chpl" over mp4 tests, mp4_fixtures, and the CLI host: no matches), the chapter count is silently capped at 64 with no damage note (`count.min(64)`), and its `bad_box(0)` error (1019) discards the box offset.
Why it matters: QuickTime-authored `chpl` chapters are the fallback chapter source; a wrong layout guess yields garbage or zero chapters with no diagnostic, and files with >64 chapters lose the tail silently. Metadata-only blast radius, hence S4.
What correct looks like: pin the layout with a Nero/ffmpeg-authored fixture test; note the cap (`self.note(0, "chpl-capped")`); carry the real offset in the error.
Confidence: MEDIUM — Verification: read + rg (no coverage); the true Nero layout itself UNVERIFIED (no spec text or chpl sample in sandbox).

### F-003 [S4]
Where: crates/demux/src/mp4/mod.rs:861-862
Code:
```
                Ok(None) => return false,
                Err(_) => return false,
```
What is wrong: `any_top_box` — the moof probe used to decide fragmented-vs-progressive — swallows every `read_top` error and reports "no moof". A mid-file I/O or structure error during the probe then sends `open()` down the `!any_samples → error::no_sample_table` branch: the user sees "no sample table" instead of the real error.
Why it matters: Error-kind drift masks the actual failure cause on damaged files (diagnosis quality, not correctness of the refusal itself — the file is refused either way).
What correct looks like: propagate the error (`any_top_box` returning `Result<bool>`), or at least note the swallowed error as damage.
Confidence: HIGH — Verification: read; both call contexts traced (782 → 794-795).

### F-004 [S4]
Where: crates/demux/src/mp4/mod.rs:870-875
Code:
```
        let start = t.empty_edit_delay as i64 - t.media_time as i64;
        let end = if t.media_duration == 0 {
            Stamp::Missing
        } else {
            Stamp::Present(Timestamp::new(
                start.saturating_add(t.media_duration as i64),
```
What is wrong: `as i64` casts of file-controlled u64s (`empty_edit_delay` from an elst v1 u64 segment_duration; `media_duration` from mdhd v1 u64) wrap to negative on values > i64::MAX instead of saturating — `u64::MAX as i64 == -1`. A malformed elst/mdhd pair can then produce an inverted or negative track span (start > end) rather than a clamped/typed one. The `saturating_add` on line 875 shows the intent; the casts defeat it.
Why it matters: Wrong-but-bounded timing facts from malformed input; no panic. Blast radius: track span display and downstream seek clamps for damaged files.
What correct looks like: `t.empty_edit_delay.min(i64::MAX as u64) as i64` (or `i64::try_from().unwrap_or(i64::MAX)`) for both casts.
Confidence: HIGH — Verification: read; elst v1 u64 origin confirmed at 1082.

### F-005 [S3]
Where: crates/demux/src/mp4/mod.rs:641-643 and 660-662
Code:
```
    pub fn compatible_brands(&self) -> &[[u8; 4]] {
        &self.compatible_brands
    }
```
and
```
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }
```
What is wrong: `compatible_brands` and `pool_bytes` have zero callers anywhere in crates/ or apps/ (rg over both trees; contrast `sample_entry_bytes` → containers/mp4.rs:190 and `chapters` → containers/mp4.rs:640, which are live). `major_brand` (635) and `is_fragmented` (647) are called only from mp4 tests.
Why it matters: Dead public observables inflate the tested-looking API surface; `pool_bytes` is the D-1 memory-budget instrument for the MP4 lane and nothing observes it, so the "bounded pool" claim has no runtime witness outside unit tests of other demuxers.
What correct looks like: delete them, or wire the D-1 observable into the CLI host/tests the way AVI's is.
Confidence: HIGH — Verification: rg over crates/*.rs + apps/*.rs.

### F-006 [S4]
Where: crates/demux/src/mp4/mod.rs:767-769
Code:
```
        let moov_len = moov_end - moov_start;
        if moov_len > MAX_MOOV_BYTES {
            return Err(error::oversized_box(b"moov", MAX_MOOV_BYTES, moov_len));
```
What is wrong: the 16 MiB moov ceiling (MAX_MOOV_BYTES, 113) can typed-refuse a legitimate long-form movie whose per-sample tables (stsz/stco/ctts for 3h+ content with per-sample ctts) exceed 16 MiB. The refusal is typed and honest, but it is a false negative on valid input for a media player.
Why it matters: Defensive bound vs. real-file interoperability tradeoff; no evidence a real file hits it in the corpus (UNVERIFIED — no such fixture), so S4 rather than higher.
What correct looks like: raise the ceiling (VLC has no such limit) or stream-parse stbl children instead of reading moov wholesale, keeping the cap as a damage-note-and-skip for the offending track.
Confidence: LOW on the realistic-file claim — Verification: read; no corpus sample near the bound (UNVERIFIED).

## 4. Per-function table (chunk lines 542–1112)
| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `oti_of_esds` | 542-570 | REAL | — | correct ES_Descriptor/DecoderConfig walk (tag 0x03, expandable len ≤4, ES_ID+flags, tag 0x04, expandable len, OTI); all `body.get`-bounded, no panic; dependentES_IDs unsupported → None (benign fallback) |
| `esds_oti` | 576-599 | REAL | — | entry-start candidates [28, 76, 78] verified: audio 28 (8+20), visual 78 (8+70) per §12.1.3/§12.4.3; ffmpeg pad 76 covered; wrong candidate falls through harmlessly; drives the mp4a→mp3 / mp4v→mjpeg quirks (corpus-pinned per comment) |
| `Mp4Demuxer::new` | 610-631 | REAL | — | rejects unseekable/unknown/empty via SourceReader; pool 2×16 MiB CAP, arenas demand-driven (riff.rs:542-549) — no eager allocation |
| `major_brand` | 635-637 | REAL-UNPROVEN | S3 | test-only caller (tests.rs:714) — see F-005 class |
| `compatible_brands` | 641-643 | UNREACHABLE | S3 | F-005: zero callers |
| `is_fragmented` | 647-649 | REAL-UNPROVEN | — | test-only callers; fine as an observable |
| `chapters` | 654-656 | REAL | — | used at containers/mp4.rs:640 ✓ |
| `pool_bytes` | 660-662 | UNREACHABLE | S3 | F-005: zero callers |
| `sample_entry_bytes` | 669-672 | REAL | — | used at containers/mp4.rs:190 (decoder-config handoff) ✓; `track.handle() as usize` — handle is the exposed ordinal, tracks indexed by construction order — consistent with `exposed` build at 818-829 |
| `note` / `surface_damage` | 675-687 | REAL | — | damage ledger → Recovered events; surfaced once at open end (840) |
| `fail` | 690-693 | REAL | — | sticky shape; replayed by next_event (part4) |
| `open` | 697-842 | REAL | — | THE mp4 open path; executed in baseline (smoke sessions); moov-at-end works; bounded scan (65 536), 16 MiB moov cap (F-006), anti-spin guard 755-757; unknown-top-box damage note 763-765; fragmented detection 781-796; progressive file-offset merge order 798-813 (sort_unstable, ~24 B/sample, transitively bounded by moov cap ≈4M entries → ≈96 MB pathological peak — no global sample cap, noted) |
| `any_top_box` | 845-866 | REAL | S4 | bounded re-scan; F-003 Err swallow |
| `track_record` | 869-910 | REAL | S2/S4 | F-001 propagation point (language 880-883); F-004 wrapping casts; `bad_box(0)` at 879 is dead-defensive (codec strings come from codec_of's static set, CodecDescriptor::new can't fail) |
| `parse_moov` | 914-999 | REAL | — | mvhd offsets verified (v1@20, v0@12, zero-guard); trex offsets (4/16/20/24) verified; chpl/cover/trak failures damage-noted, never fatal; cover absolute offset math (989) correct (payload-relative + payload_start) |
| `parse_udta_chpl` | 1003-1041 | REAL-UNPROVEN | S4 | F-002: untested layout guess, silent 64-cap, offset-less error; entry walk itself get()-bounded, `start/100` 100ns→ns correct |
| `parse_trak` (this chunk's portion) | 1045-1112 | REAL | — | tkhd (v1@20/v0@12, flags mask ✓), edts/elst (v1 8/16, v0 8/12, signed v0 media_time ✓, media==-1 → empty-edit delay ✓, multi_edit flag consumed at 1208), mdhd (v1 20/24/28, v0 12/16/20 ✓), hdlr@8-12 ✓, minf→stbl range capture; language captured here feeds F-001; the `track_id==0 || timescale==0` typed refusal is at 1139-1141 (part3/A06); rest of the function (stsd/stbl/stts/stsc/stsz, through its close at 1426) is part3/A06 |

## 5. Test honesty (tests in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 38 `mp4::tests` (nofailfast :1764+) | YES for covered lanes | `elst_empty_edit_delays_and_media_time_trims` covers my 1071-1096; `chap_tref_chapters_surface_with_titles` covers tref chapters (not chpl); matrix_mp4_trunc*/stszlie* pin typed refusals |
| mp4_fixtures / matrix suites | YES | bit-digest extraction pins vs writer-declared truth (`mp4_extraction_matches_writer_declared_frame_digests ... ok`) |
| language handling | NO TEST EXISTS | zero language assertions in mp4/tests.rs + mp4_fixtures.rs — F-001 shipped invisibly through 38 green tests |
| chpl parsing | NO TEST EXISTS | F-002: no chpl fixture anywhere |
| baseline smoke sessions | n/a (evidence, not tests) | they are what EXPOSED F-001 (`language: Some("voe")`) — the unit suites never looked |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the bytes the engine hands the decoder as `DecoderConfig::extra_data`" (sample_entry_bytes) | mod.rs:664-667 | REAL | containers/mp4.rs:190 `config_blob(codec, entry)` consumes it |
| mdhd language unpacked, "None for 'und'" | mod.rs:534-535 | FALSE | F-001: "und" (0x55C4) decodes "voe" and is returned as Some — live in both baseline smoke logs |
| "The widely-interop shape: v/f(4) reserved(4) count(u8)" (chpl) | mod.rs:1015-1016 | REAL-UNPROVEN | no fixture pins it; comment itself hedges with "?" |
| MP4 demux works / "tracks 2 ready" | baseline INDEX CORRECTION v2 | REAL | mp4-smoke-open-session-420.txt (plays end-to-end); my chunk built those Track records |
| (context, other files' defect) bench "no decoder in this build" for MP4 | INDEX standing defects | not this file | apps/cli/src/bench.rs hard-codes the first-vertical lane — recorded for the bench owner, not a demux defect |

## 7. Dead / duplicated / unreachable code
- `compatible_brands` (641-643), `pool_bytes` (660-662): dead (F-005).
- `major_brand`/`is_fragmented`: test-only.
- `bad_box(0)` at 879: dead-defensive error mapping (codec set is closed).
- No duplication found in this chunk (esds/stsd walking reuses `iter_boxes` from part1).

## 8. Dependencies & flags
- No [features] in crates/demux/Cargo.toml — mp4 module compiled unconditionally; shipped CLI reaches it (containers/mod.rs "mp4" arm).
- Reuses `crate::avi::riff::{PayloadSlots, SourceReader}` — same-crate, same pattern as PS (documented).
- `MAX_SAMPLE_BYTES` 16 MiB pool slot cap: demand-driven arenas verified at riff.rs:538-549.
- std-only; no unsafe; no unwrap/expect in lines 542-1112 (all `unwrap_or` defaults on file-controlled fields).

## 9. Open questions for the next agents
- A08 (part1): F-001's root-cause line 536 (`b'a' +` → `0x60 +`) lives in your chunk; the fix is one byte. My part2 records the propagation and the baseline proof.
- A06 (part3): `parse_trak` continuation — verify stsz/stco/co64/stsc counts are capped by MAX_SAMPLES (the consts at 109-122) and that ctts/stts arithmetic can't overflow i64 on u32-heavy inputs; `multi_edit` note at 1208; tkhd_flags consumption (track-enabled bit).
- A10 (part4): `read_chapter_titles`/`queue_cover_packets`/`push_cover_track` — cover offset math consumed my 989 abs computation; verify the packet read clamps `size` (a lying covr size must not read past EOF).
- Wave-owner: the `language: Some("voe")` INDEX standing defect is now TRACED to mp4/mod.rs:536 — request a mutation test (change 0x60→b'a' style fix) plus a fixture mdhd language pin in the follow-up.
