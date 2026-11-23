# AUDIT: crates/demux/tests/mp4_fixtures.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 284 / 6 (3 helpers + 5 #[test]) |
| File verdict | REAL (with SHALLOW spots in the valid-path and fuzz tests) |
| Reachable from shipped entry point? | NO — integration test. Code under test reachable: Mp4Demuxer::new <- apps/cli/src/containers/mp4.rs:100 <- CLI open path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | ls over fixtures dirs (all present), rg for derivative usage repo-wide, sed quote checks, baseline citation. No cargo runs. |

## 1. One-paragraph verdict
Real ffmpeg-authored fixtures on disk (7 valid incl. fMP4±sidx, QuickTime, chapters, elst, 3GP H.263 — all verified present under tests/fixtures/generated/valid/) driven through the REAL access stack (`open_via_input` -> input::FileAccess), plus 14 malformed derivatives through both the fuzz harness and a typed-pass. All 5 tests pass (baseline lines 2368-2376: `5 passed; 0 failed`). The weak spots: `valid_mp4_fixtures_demux_fully` asserts only `total > 0` — the name promises full demux but a demuxer dropping 90% of samples after the first packet passes; the edts test checks only `pts != 0`; and both the fuzz-smoke and the walk-error branch of the typed-pass assert less than their names promise. The deeper differential (exact per-track packet counts) lives in corpus_mp4.rs, which depends on the corpus-v4 directory that is ABSENT from this sandbox (skip-when-absent design) — so in-repo, the committed fixtures never get content-level verification.

## 2. What actually works (verified)
— verified by ls: all 7 valid fixtures exist (mp4_h264_aac.mp4 13,703 B ... mp4_qt.mov 13,754 B) and all 14 MALFORMED derivatives exist under tests/fixtures/generated/malformed/.
— verified by baseline: `Running tests/mp4_fixtures.rs` -> `test result: ok. 5 passed; 0 failed` (cargo-test-workspace.txt 2368-2376); corpus_mp4 (exact-count differential, baseline 2 passed) ran against corpus-v4 at baseline time — corpus dir ABSENT here (ls exit 2), so corpus results are cited, not re-run.
— read: `drain` has a 500k event budget and a non-empty-payload assert (129) — a fabricated empty-packet demuxer fails.

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/mp4_fixtures.rs:166-173
Code (verbatim):
```
        let (packets, _key, total) = drain(&mut mp4);
        assert!(total > 0, "{}: no packets", spec.fixture);
        assert_eq!(
            packets.len(),
            spec.codecs.len(),
            "{}: packet tracks",
```
What is wrong: the valid-path acceptance test verifies track table, codec strings, fragmented/chapters flags, and that at least one non-empty packet per track exists — nothing more. No per-sample hashes (the AVI parent suite does per-frame sha256), no exact packet counts, no PTS ladder, no duration check. Why it matters: a demuxer that emits the first sample of each track then silently skips the rest passes `total > 0`; the test name says "demux_fully". The exact-count oracle for real MP4 files (corpus_mp4.rs:272 `assert_eq!(counts[i], *want...)`) runs only when the corpus-v4 dir exists — absent in this sandbox and in any fresh clone, so the committed fixtures are the only always-present truth and they get count>0 only. What correct looks like: record per-track packet counts (or digests) for the 7 committed fixtures, as the AVI expectation TOMLs do. Confidence: HIGH — Verification: read + ls (corpus absent).
F-002 [S4]
Where: crates/demux/tests/mp4_fixtures.rs:277-279
Code (verbatim):
```
            match mp4.next_event() {
                Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                Ok(DemuxEvent::Packet(p)) => assert!(!p.data.is_empty(), "{fixture}: fake packet"),
```
What is wrong: the typed-pass test verifies stage (`Demux`) and code prefix (`demux.mp4.`) for START refusals (263-271) but accepts ANY `Err` from the walk with zero verification — the "or demux typed" half of the test's name (line 255) is unenforced for walk errors. Mitigated by mp4/tests.rs unit pins of specific codes (444/478/490/512/590-591), so blast radius is small. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/demux/tests/mp4_fixtures.rs:244-249
Code (verbatim):
```
    for fixture in MALFORMED {
        let bytes = testkit::fixture_bytes(fixture).must();
        fuzz::fuzz_demux_mp4(&bytes);
    }
```
What is wrong: the fuzz smoke test asserts nothing (absence-of-panic only); a gutted `fuzz_demux_mp4` body passes it. Same pattern as fixtures_parent.rs F-001. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/demux/tests/mp4_fixtures.rs:218-221
Code (verbatim):
```
    assert!(
        pts != 0,
        "the edts fixture's first video PTS must be shifted, got {pts}"
    );
```
What is wrong: the elst test asserts only non-zero-ness — any wrong-but-nonzero shift (e.g., double the media-time, or a sign error) passes. The elst media-time the fixture carries is a fixed recorded value; pinning it (or its µs conversion) would make the test honest about magnitude. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VALID table | 34-97 | REAL | — | 7 fixtures, codecs/kinds/fragmented/chapters declared |
| started | 100-105 | REAL | — | real FileAccess + typed unwrap_or_else unreachable |
| drain | 109-137 | REAL | — | 500k budget, non-empty payload assert, per-track counts |
| valid_mp4_fixtures_demux_fully | 140-175 | SHALLOW | S3 | total > 0 only (F-001) |
| chapters_fixture_surfaces_titled_markers | 179-187 | REAL | — | 3 chapters, non-empty titles, strictly monotone starts |
| edts_fixture_shifts_the_first_pts | 192-222 | SHALLOW | S4 | pts != 0 only (F-004) |
| malformed_mp4_fixtures_survive_the_fuzz_harness | 244-249 | SHALLOW | S4 | no assertions (F-003) |
| malformed_mp4_fixtures_only_ever_refuse_or_demux_typed | 255-284 | REAL | S4 | start-refusals typed-verified; walk Err(_) unverified (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| valid_mp4_fixtures_demux_fully | PARTIAL | catches track-table/codec/flag breakage; misses partial sample loss (F-001) |
| chapters_fixture_surfaces_titled_markers | YES | count + titles + strict monotonicity |
| edts_fixture_shifts_the_first_pts | PARTIAL | catches a zero PTS; not a wrong shift (F-004) |
| malformed_..._survive_the_fuzz_harness | NO (panic-only) | gutted harness passes (F-003) |
| malformed_..._refuse_or_demux_typed | PARTIAL | start refusals fully checked; walk errors unchecked (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ffmpeg-authored valid fixtures ... demux through the real access stack" | mp4_fixtures.rs:1-4 | REAL | open_via_input -> FileAccess (common/mod.rs:546-550); fixtures present |
| "no panic, no hang, no unbounded allocation, nothing fake" (malformed) | mp4_fixtures.rs:5-7 | REAL for panic/empty-packet (budget assert + non-empty assert); hang bounded only by the 500k event budget; UNVERIFIED for allocation (nothing measures memory) |
| "fixtures on disk are only half the contract" | mp4_fixtures.rs:12-13 | REAL | this file is the other half — present and passing |

## 7. Dead / duplicated / unreachable code
None dead. Note: the 11 `mp4_mjpeg_pcm__*` malformed derivatives on disk are NOT in this file's MALFORMED array — verified they are exercised by codecs/tests/fuzz_survival.rs, media-probe/tests/fixtures_matrix.rs, and the xtask generator itself (rg), so they are not orphans; the split is deliberate but undocumented in this file.

## 8. Dependencies & flags
demux (Mp4Demuxer, fuzz), common (open_via_input), testkit (fixture_path/bytes/Must), plugin_api, media-types. No feature gates. No external deps.

## 9. Open questions for the next agent
- corpus_mp4.rs auditor: confirm the corpus-v4 exact-count differential covers the same codec/shape families as these committed fixtures (if corpus is missing at CI, F-001's gap is total in-repo).
- mp4/src/tests.rs auditor: does any unit test pin sample CONTENT (not just counts) for progressive + fragmented shapes?
