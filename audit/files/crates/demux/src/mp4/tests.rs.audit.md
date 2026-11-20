# Audit — crates/demux/src/mp4/tests.rs

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W05-A02 |
| Date | 2026-11-16 |
| Lines | 1111 (verified `wc -l`) |
| Functions | 38 `#[test]` functions + 12 test-local helpers (`video_track`, `audio_track`, `open`, `drain`, `ErrOf`, `code_of`, `span_at`, `ftyp_bytes`, `pcmc_box`, `pcm_track`, `find_box`, `drain_noting`) |
| File verdict | **REAL** (35/38 tests verify real demuxer behavior through genuine byte streams; 1 test pins a spec-unit bug as expected output) |
| Reachable | YES — `mod tests;` gated `#[cfg(test)]` at mod.rs:103-104; part of the workspace suite the orchestrator ran 2026-11-16 (1001 passed / 2 failed(control-server) / 20 ignored — cited, not re-run per orders) |
| Findings | S0:0 S1:0 **S2:1** **S3:3** S4:2 (6 total) |
| Commands run | `wc -l`; `Read` (full file); `Grep '^fn |#\[test\]'` (38 tests counted); `sed -n '1,4p'`, `'57,70p'`, `'184,197p'`, `'213,221p'`, `'586,594p'`, `'644,650p'`, `'980,984p'`, `'1075,1096p'` (quote verification); cross-verified mod.rs quotes via `sed -n '1084,1095p'`, `'1863,1869p'`, `'916,924p'` |

## 1 One-paragraph verdict

This is a real byte-level unit suite, not a mock theater: every fixture is a genuine MP4 byte stream built by an independent muxer emulation (`test_support::progressive/fragmented/...`), every demuxer interaction goes through the real `SourceStream` seam, and expected values (DTS ladders, PTS shifts, codec strings, payload bytes, damage-note magnitudes) are hand-computed literals in the test file, not values re-derived from production code. 35 of 38 tests would fail if the corresponding demuxer logic were broken. Zero tautologies and zero circular proofs found. The three defects that matter: `drain()` silently swallows `Err` from `next_event()`, which makes one test (`absent_stss_means_all_sync`) vacuously pass on a broken demuxer; two error-path tests accept disjunctions, contradicting the file header's "every error path pinned to its typed code"; and `elst_empty_edit_delays_and_media_time_trims` **pins a unit-conversion bug as expected output** — the empty-edit dwell is applied in raw movie-timescale ticks as media-timescale ticks (mod.rs:1091 stores `seg_dur` unconverted; mod.rs:1869 adds it to media ticks), and this test certifies that wrong number (1000 instead of 9000 media ticks), so the suite is structurally incapable of catching a real elst correctness bug.

## 2 What actually works (verified)

- Fixtures are REAL MP4 bytes: `open()` (tests.rs:51-55) routes through `test_support::mem_stream` → `crate::avi::test_support::mem_stream` → `BytesStream` implementing the real `SourceStream` trait (avi/test_support.rs:121-126). No in-memory structure bypasses the parser; the demuxer parses actual bytes.
- Round-trip payload assertions (`&packets[0].data[..] == &[0x10; 4]`, tests.rs:147-148) are not tautologies: the parser must correctly walk ftyp→moov→stbl→stco/stsz to fetch those bytes. If stco offset patching or stsz sizes were broken, the payload or count asserts fail.
- The DTS ladder (tests.rs:131-140) is computed in-test (`i * 3000`), independent of both builder and parser arithmetic.
- Negative probe cases (tests.rs:111-113) pin real refusal behavior of `probe()`.
- Seek tests assert landing tick AND resumed packet count AND typed out-of-range refusal (tests.rs:416-445) — strong.
- Damage-note magnitudes are hand-derived constants (e.g. `Some(50)` at tests.rs:928 = 8+28+14 box arithmetic), not production echoes.
- `stss_unsorted_with_duplicates_matches_sorted_flags` (tests.rs:1038-1073) is a genuine property test: two different byte streams (unsorted/dup vs sorted canonical) must yield identical per-sample flags — catches a naive binary search over an unsorted array.
- Typed error pins for `not-mp4`, `no-moov`, `truncated` (x2), `not-started`, `seek.out-of-range` are exact string-equality asserts — these fail if the code paths regress.

## 3 Findings

### F-001 [S2] elst empty-edit test pins a movie→media timescale unit bug as expected output
Where: `crates/demux/src/mp4/tests.rs:184-211` (+ production refs mod.rs:1084-1091, 1863-1869, 917-923)
Code (tests.rs:186-197, verified `sed -n '184,197p'`):
```rust
    let mut track = video_track(2, 3000, &[true, true], &[0]);
    track.edit = Some((1_000, -1)); // empty edit: 1000-tick dwell
    ...
    assert_eq!(pts(&packets[0]), 1_000);
    assert_eq!(pts(&packets[1]), 4_000);
```
Code (mod.rs:1090-1094, verified `sed -n '1084,1095p'`):
```rust
                            if media == -1 {
                                empty_edit_delay = seg_dur;
                            } else if media > 0 {
                                media_time = media as u64;
                            }
```
Code (mod.rs:1866-1869, verified `sed -n '1863,1869p'`):
```rust
        let dts_ticks = (s.dts as i64).saturating_sub(t.media_time as i64);
        let pts_ticks = dts_ticks
            .saturating_add(s.cts)
            .saturating_add(t.empty_edit_delay as i64);
```
What's wrong: ISO/IEC 14496-12 §8.6.6 defines `elst` `segment_duration` in the **movie** timescale. The fixture writes mvhd timescale 1000 (test_support.rs:294 `mvhd(1000, 0)`) and media timescale 9000 (tests.rs:19). A 1000-movie-tick dwell is 1 second = **9000** media ticks; the demuxer adds the raw 1000 to media ticks (no rescale anywhere — `movie_timescale` is parsed at mod.rs:917-923 and used only for covers/chapters, never for elst), and the test asserts the unconverted 1000. The test would **FAIL if the demuxer were fixed** to spec. Same unconverted mix leaks into `track_record`'s timing span (mod.rs:870).
Why it matters: this is the exact class of bug a demux unit suite exists to catch (ffmpeg's mov demuxer rescales elst durations across timescales). The suite instead launders it as "expected."
What correct looks like: fixture with movie ts ≠ media ts (it already has 1000 vs 9000!) and expected value `9000` — i.e., rescale `seg_dur` from `movie_timescale` to the track timescale at parse time, then assert the converted value.
Confidence: HIGH on the mismatch (all three quotes sed-verified; `movie_timescale` usage grep-verified — no rescale call site exists). The ffmpeg-precedent phrasing in the repo's own docs ("the ffmpeg discard precedent", "ffmpeg attached-pic precedent") makes the un-ffmpeg-like elst handling stand out.

### F-002 [S3] `drain()` silently swallows typed errors → vacuous pass in `absent_stss_means_all_sync`
Where: `crates/demux/src/mp4/tests.rs:57-70` (drain) and 213-221 (the vacuous test)
Code (verified `sed -n '57,70p'`):
```rust
fn drain(demuxer: &mut Mp4Demuxer) -> (usize, Vec<media_types::Packet>) {
    let mut packets = Vec::new();
    let mut events = 0;
    while let Ok(event) = demuxer.next_event() {
        events += 1;
        match event {
            DemuxEvent::Packet(p) => packets.push(p),
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
        assert!(events < 10_000, "event budget");
    }
    (events, packets)
}
```
Code (tests.rs:213-221, verified `sed -n '213,221p'`):
```rust
fn absent_stss_means_all_sync() {
    let track = video_track(3, 100, &[true, true, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    assert!(packets.iter().all(|p| p.keyframe));
}
```
What's wrong: `while let Ok(...)` exits the loop on the FIRST `Err` with no assertion. A demuxer regression that returns `Err` immediately after a successful `start()` yields `packets == []`, and `iter().all(...)` over an empty vec is vacuously **true** — the test passes. No packet-count assert exists in this test to catch it.
Why it matters: "would it fail if broken?" — for this test the answer is NO for whole classes of breakage (any error-returning regression), which is the audit's core question.
What correct looks like: `assert_eq!(packets.len(), 3, "all three samples must flow")` before the `all()` — and `drain()` returning/panicking on unexpected `Err` (e.g. match on `Err(e) => unreachable!("unexpected typed error mid-walk: {}", e.context.code)`), reserving silent-exit for the tests that intentionally hit sticky errors.
Confidence: HIGH (Rust `Iterator::all` on empty iterator is true by definition; no other assert in the test body).

### F-003 [S3] File header claims "every error path pinned to its typed code" — two error tests accept disjunctions
Where: `crates/demux/src/mp4/tests.rs:1-4` (claim), 586-594 and 644-650 (disjunctions)
Code (header, verified `sed -n '1,4p'`):
```rust
//! Unit tests for the MP4/MOV demuxer (card 5.C1) — in-memory
//! fixtures from [`super::test_support`], every error path pinned to
//! its typed code (no panics, no unwrap-driven assertions on
//! demuxer output).
```
Code (tests.rs:588-594, verified `sed -n '586,594p'`):
```rust
    let err = mp4.start().err_of("overrun");
    let code = code_of(&err);
    assert!(
        code == "demux.mp4.box-overruns-parent"
            || code == "demux.mp4.truncated"
            || code == "demux.mp4.no-tracks",
        "got {code}"
    );
```
Code (tests.rs:646-649, verified `sed -n '644,650p'`):
```rust
    assert!(
        code == "demux.mp4.no-sample-table" || code == "demux.mp4.no-tracks",
        "got {code}"
    )
```
What's wrong: an OR of 3 (resp. 2) codes can absorb a regression that swaps one typed refusal for another — the pin is not a pin. The header's "every error path pinned" is false for exactly these two tests. Additionally the header's "no panics" is loose: the file leans on ~45 `unreachable!()` guards (e.g. tests.rs:53-54, 80) which are panics-on-precondition.
Why it matters: doc claims about test rigor are themselves evidence in this repo (matrix path-existence standard); an overstated header inflates the perceived pin strength.
What correct looks like: the second half of `child_beyond_parent_is_typed` (tests.rs:615-618) already shows the right pattern — one precise byte mutation, one exact `assert_eq!(code_of(&err), "demux.mp4.box-overruns-parent")`. The disjunctive halves should be replaced by the precise mutation that provably produces the exact code.
Confidence: HIGH (both quotes sed-verified; second half of the same test demonstrates the exact-pin pattern exists).

### F-004 [S4] Ceiling tests parameterize expectations with the production constant `MAX_SAMPLES`
Where: `crates/demux/src/mp4/tests.rs:982-984` (also 1013-1014, 1032-1033)
Code (verified `sed -n '980,984p'`):
```rust
    let max = super::MAX_SAMPLES as u64;
    assert_eq!(noted, Some(0x0800_0000u64 - max));
    assert_eq!(packets, 2, "the two real samples still demux");
```
What's wrong: the expected value is computed FROM the production constant. If someone changes `MAX_SAMPLES` (mod.rs:122), the tests follow silently — they verify the relation "note = declared − cap", not the cap's value or its rationale (1 Mi samples). Not circular (the cap arithmetic itself is verified against `0x0800_0000`, which would catch cap-removal or off-by-one), but value-drift-invisible.
What correct looks like: pin the literal cap in one dedicated test (`assert_eq!(super::MAX_SAMPLES, 1024 * 1024)`) and keep the relation asserts — the pair catches both classes.
Confidence: HIGH.

### F-005 [S4] Fixture timescale coupling blinds the chapter test to the scale used for marker conversion
Where: `crates/demux/src/mp4/tests.rs:276-300` (chapter test) + test_support.rs:294 (`mvhd(1000, 0)`) + chapter_track timescale 1000 (tests.rs:278)
Code (tests.rs:288-290):
```rust
    // DTS ladder: 0 ticks and 1000 ticks at 1000 timescale = 1 s.
    assert_eq!(chapters[0].start.0, 0);
    assert_eq!(chapters[1].start.0, 1_000_000_000);
```
What's wrong: movie timescale (1000) and the chapter trak's own timescale (1000) are IDENTICAL in the fixture, so the test cannot distinguish whether marker starts are converted with the chapter track's mdhd timescale or the movie timescale. A production swap of the scale source passes this test unchanged.
What correct looks like: chapter fixture whose trak timescale differs from mvhd timescale (e.g. 600 vs 1000) with the expected nanosecond starts computed from the trak's own scale.
Confidence: MEDIUM (the blindness is certain; whether production uses the right scale for chapter markers was not established in this audit's scope).

### F-006 [S3] The unit suite never exercises three of the taxonomy's limit guards
Where: `crates/demux/src/mp4/tests.rs` (whole file — absence)
What's wrong: none of the 38 tests fires `demux.mp4.oversized-box` (moov > 16 MiB), `demux.mp4.scan-budget-exhausted` (fragment scan budget, mod.rs:718/1856/2210), or `demux.mp4.depth-exceeded` (see error.rs audit E-001 — that one is unreachable in production too). The bounded-recovery narrative ("a pathological moof ladder is a typed failure, never a hang", error.rs:158-160) has no unit-level pin here; the malformed-derivative fixtures (mp4_fixtures.rs) mutate small files that cannot reach a 16 MiB moov.
What correct looks like: one test synthesizing a moof ladder beyond `MAX_FRAGMENT_SCAN_BOXES` and one declaring an oversized moov (patched size field over the ceiling) asserting the typed codes.
Confidence: HIGH for the absence (grep over the file for the three codes: zero hits); MEDIUM for integration coverage (mp4_fixtures.rs runs derivatives through the fuzz harness which swallows errors by contract — fuzz.rs:7-9).

## 4 Per-test table (all 38)

| # | Test | Lines | Would fail if broken | Classification |
|---|---|---|---|---|
| 1 | probe_accepts_known_brands_and_qt_heads | 91-114 | YES (pos+neg byte-level probe cases) | real |
| 2 | progressive_single_track_demuxes_samples_in_order | 118-149 | YES (count, codec, seekability, DTS ladder, keyframes, payload bytes) | real |
| 3 | progressive_two_tracks_interleave_by_file_offset | 151-166 | YES (6 packets, handle order 0→1) | real |
| 4 | ctts_shifts_pts_off_dts | 168-182 | YES (PTS 0/0/9000) | real |
| 5 | elst_empty_edit_delays_and_media_time_trims | 184-211 | YES — but pins WRONG values (F-001) | real (defective pin) |
| 6 | absent_stss_means_all_sync | 213-221 | **NO** (vacuous on Err-early demuxer; F-002) | weak |
| 7 | fragmented_demuxes_moofs_with_tfdt_ladder | 225-254 | YES (8 pkts, tfdt ladder, sync pattern, SlowLinear) | real |
| 8 | fragmented_two_tracks_per_moof | 256-271 | YES (8 pkts, per-moof handle alternation) | real |
| 9 | chap_tref_chapters_surface_with_titles | 275-300 | YES (titles, ns starts, hidden trak, no leak; scale-blind F-005) | real |
| 10 | cover_art_surfaces_as_a_still_image_video_track | 304-332 | YES (2 traks, mjpeg, still_image, 1 cover pkt, verbatim jpeg) | real |
| 11 | png_cover_and_unknown_cover_types_map_or_note | 334-374 | YES (png mapping + cover-type-unsupported note + 1 pkt) | real |
| 12 | cover_and_chapters_keep_packet_handles_aligned_with_tracks | 376-401 | YES (gap-alignment handles, cover bytes) | real |
| 13 | progressive_seek_lands_on_keyframe | 415-445 | YES (two landings, resume count 2, typed OOR) | real |
| 14 | fragmented_seek_scans_moofs | 447-470 | YES (landing 4000, 4 pkts, first sync) | real |
| 15 | not_mp4_is_typed_refusal | 474-479 | YES (exact code) | real |
| 16 | missing_moov_is_typed | 481-491 | YES (exact code) | real |
| 17 | truncated_box_header_is_typed_truncation | 504-513 | YES (exact code) | real |
| 18 | size_zero_convention_extends_to_stream_end | 515-534 | YES (2 pkts survive mdat size=0) | real |
| 19 | size_one_reads_64bit_largesize | 536-564 | YES (2 pkts survive 64-bit size rewrite) | real |
| 20 | child_beyond_parent_is_typed | 566-619 | YES overall (2nd half exact; 1st half 3-way OR — F-003) | real (half-weak) |
| 21 | zero_sample_tables_in_progressive_is_typed | 621-650 | YES (Ok would panic the guard) but 2-way OR (F-003) | real (half-weak) |
| 22 | unknown_boxes_between_ftyp_and_moov_are_tolerated | 652-670 | YES (1 pkt through spliced junk box) | real |
| 23 | truncated_sample_read_is_typed_after_start | 672-688 | YES (typed truncated + sticky repeat) | real |
| 24 | not_started_calls_are_typed | 690-699 | YES (both methods exact code) | real |
| 25 | mov_quirks_wide_and_qt_brand_open | 701-717 | YES (wide prefix, qt brand, 2 pkts) | real |
| 26 | sowt_entry_maps_to_pcm_s16le | 719-727 | YES (codec string map) | real |
| 27 | threegp_brand_and_samr_entry_open | 729-739 | YES (amr_nb map + 2 pkts) | real |
| 28 | seek_failure_leaves_state_usable_d4 | 741-763 | YES (2 pkts consumed, typed OOR, remaining 2 pkts) | real |
| 29 | ipcm_fpcm_pcmc_mapping_table | 800-854 | YES (10 mapping cases + 4 refusals + 2 defaults) | real (unit-level, direct fn call) |
| 30 | ipcm_fpcm_with_pcmc_demux_as_pcm_codecs | 856-880 | YES (codec + payload bytes through bytes) | real |
| 31 | ipcm_without_pcmc_defaults_to_entry_name_convention | 882-894 | YES (pcm_s16be default) | real |
| 32 | ipcm_pcmc_unmapped_width_notes_unknown_entry | 896-930 | YES (dropped trak, note magnitude 50, 1 pkt) | real |
| 33 | stco_count_over_ceiling_is_bounded_and_damage_noted | 966-985 | YES (surplus note + 2 pkts; MAX_SAMPLES param F-004) | real |
| 34 | co64_count_over_ceiling_is_bounded_and_damage_noted | 987-1016 | YES (co64 rewrite + note + 1 pkt) | real |
| 35 | stsc_count_over_ceiling_is_bounded_and_damage_noted | 1018-1035 | YES (sibling demuxes 2, inflated trak noted) | real |
| 36 | stss_unsorted_with_duplicates_matches_sorted_flags | 1037-1073 | YES (property: unsorted/dup ≡ sorted canonical) | real |
| 37 | fuzz_lite_mutations_never_panic | 1077-1096 | **NO** (only panics/hangs; contract is absence-of-panic) | weak (by design, labeled) |
| 38 | fuzz_lite_fragment_mutations_never_panic | 1098-1111 | **NO** (same) | weak (by design, labeled) |

Counts: **real 35 / weak 3 / tautology 0 / circular 0**.

## 5 Doc claims touching this file

- `docs/feature-matrix.toml:151` — evidence string `"crates/demux/src/mp4/** (card 5.C1; record docs/formats/container-mp4.md, 5.REC-1: 30-lane corpus_mp4 table); Linux-only evidence"` for row `container-mp4-mov-3gp` (status implemented win/linux/macos). VERDICT: SUPPORTED at the path-existence standard (real files with a real 38-test suite; this audit confirms the tests are genuine byte-stream tests, 35/38 real). The 30-lane corpus differential itself is environment-dependent (corpus not committed; suite skips loudly without it) — UNVERIFIED in this sandbox.
- `docs/formats/container-mp4.md:86` — `"cargo test -p demux --all-features 298/0/4 (unit 224/0; corpus_mp4 2/2; mp4_fixtures 5/5)"`. VERDICT: UNVERIFIED (historical gate; re-running is forbidden by this audit's orders; the 2026-11-16 orchestrator run cites 1001 passed / 2 failed(control-server) / 20 ignored workspace-wide, which is consistent with a suite that includes these tests passing).
- `docs/formats/container-mp4.md:68` — `"no panics (fuzz-lite + in-crate fuzz_mutations green)"`. VERDICT: SUPPORTED (tests #37/#38 exist and the fuzz harness contract is absence-of-panic, fuzz.rs:7-9; they passed in the orchestrator run cited above).
- `crates/demux/src/mp4/tests.rs:1-4` header — `"every error path pinned to its typed code"`. VERDICT: OVERSTATED (F-003: two disjunctive pins).
- `docs/13-agent-wave-program.md:517` — card 5.C1 AC `"frag-MP4 + MOV corpora demux + seek"` citing `crates/demux/src/mp4/**`. VERDICT: SUPPORTED at path-existence (tests #7/#8/#14 and #25 exist and are real); corpus-level portion rides corpus_mp4.rs (env-dependent).
- The matrix note (feature-matrix.toml:152) retains the stale sentence `"No MP4 demuxer code exists"` followed by later implemented-status updates in the same field — self-contradictory notes field; VERDICT: STALE TEXT (S4 doc hygiene), not evidence of falsity.

## 6 Dead/duplicated code

- None in this file. All 12 helpers have callers within the file; `drain_noting`/`find_box` serve the AUD-01 tests; `ftyp_bytes` serves two tests.
- Adjacent duplication worth recording: the `drain()` vs `drain_noting()` pair (tests.rs:57-70 vs 945-964) share the loop shape — acceptable, different contracts.

## 7 Open questions

1. Does any corpus lane (corpus-v4) contain an elst empty-edit with movie ts ≠ media ts? If yes, the corpus differential would have caught F-001's unit bug — check the ffprobe-measured first-PTS values in the worklog for such a lane; if all corpus lanes share ts or have no empty edits, the bug is invisible repo-wide.
2. Are chapter-marker starts converted with the chapter trak's mdhd timescale or the movie timescale in mod.rs (F-005)? Needs the chapters() code path read (mod.rs:2155+) — outside this file's scope, deferred to the mod.rs auditor.
3. `zero_sample_tables_in_progressive_is_typed` patches both stsz and stco counts to 0 — which of the two codes does the CURRENT build actually produce? The disjunction hides it (F-003); a debug run would pin it. UNVERIFIED (no cargo test permitted).
