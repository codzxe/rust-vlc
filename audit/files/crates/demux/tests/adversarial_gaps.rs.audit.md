# AUDIT: crates/demux/tests/adversarial_gaps.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 1201 / 35 (22 helpers + 13 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test); it exercises `demux::avi::*` which is reachable from lumen-cli open path (demux crate is a release-lane dependency — see audit/runs/baseline/cargo-build-release-lumen-cli.txt) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p demux --test adversarial_gaps` -> exit 0, 13 passed / 0 failed in 0.14s (compiled 5.15s); `sed -n` quote verification (all quotes); fixture existence checks via ls |

## 1. One-paragraph verdict
This is a genuine adversarial suite, not an echo chamber. It attacks the AVI parser with real malformed bytes: shuffled/reversed/mid-run-reversed/duplicated idx1 tables built by an in-file byte-level AVI builder, hand-built time-backwards `IndexTable`s fed straight to `search_index`, a dribbling `SourceStream` (seeded 1..=16 and 1-byte reads) driven over the committed parent fixtures to EOS, and wrong-container hybrids (real MP4 bytes into the AVI path, ftyp-prefixed AVI bodies, RIFF+garbage, WAVE/RIFX forms). Every family asserts differential equality against a canonical walk (per-packet SHA-256, ladder ticks, keyframe flags), damage accounting, and typed-refusal codes — the assertions are pinned to absolute recorded values (`&[0, 1, 1, 3, 5, 6]` landing ticks), not self-generated goldens. All 13 tests pass under my own run. The one disclosed hole: monotone-stored duplicate offsets are admitted to surface no damage note and are not tested. The four families genuinely exercise parser defense, but the corpus is AVI-only — no adversarial byte tests for MKV/MP4/TS live in this file (they live in other suites; see mkv/ts audits).

## 2. What actually works (verified)
— verified by `cargo test -p demux --test adversarial_gaps` -> `test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s` (my run, exit 0). Also matches baseline audit/runs/baseline/cargo-test-workspace.txt:2180-2198 ("running 13 tests ... test result: ok. 13 passed").
— verified by reading crates/demux/src/avi/index.rs:431 (`raw.sort_by_key(|entry| entry.chunk_pos);`) and index.rs:472 (`damage.note(out_of_order * 16, "idx1-out-of-order-entries");`): the parser really sorts and damage-counts, so the test's `misplaced_entries` running-max mirror (adversarial_gaps.rs:254-265) is pinned to real parser behavior, not a tautology.

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/adversarial_gaps.rs:590-594
Code:
```
    // properties, not exactness. (Monotone-stored duplicates —
    // [0, 1, 1, 2, 3, 3] — currently surface NO damage note at all;
    // registered as a hardening candidate in the W1-15 worklog.)
```
What is wrong: The suite's own comment admits an adversarial shape (monotone-stored duplicate `dwOffset`s) that surfaces zero damage — silently lying tables — and no test covers it. Why it matters: the file header (lines 1-4) claims "four behaviors the clean-path suites never exercise"; a fifth known-silent corruption shape is disclosed but left unpinned, so a regression there is invisible. What correct looks like: a `monotone_duplicates_surface_damage` test, or the parser damage note it implies. Confidence: HIGH — Verification: read (parser behavior cross-checked at index.rs:55 "Duplicate `dwOffset`s keep their relative order" — no damage path for them).

F-002 [S4]
Where: crates/demux/tests/adversarial_gaps.rs:1063
Code:
```
    std::fs::remove_file(&path).ok();
```
What is wrong: Swallowed result on cleanup. Why it matters: test hygiene only; a failed unlink leaves stale temp files. What correct looks like: `.must()` or assert. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/tests/adversarial_gaps.rs:1045-1047
Code:
```
    let path = std::env::temp_dir().join("lumen-w1-15-wrong-name.mp4");
    std::fs::write(&path, &bytes).must();
    let stream = open_via_input(&path).must();
```
What is wrong: Fixed temp filename — two concurrent runs of this suite (shared TMPDIR) interleave write/open. Why it matters: content is deterministic so corruption is unlikely, but the TOCTOU is real in parallel CI. What correct looks like: `stdext::tempfile`-style unique name or process-id suffix. Confidence: MEDIUM — Verification: read (no concurrency failure observed; would need parallel-process run to prove).

F-004 [S4]
Where: crates/demux/tests/adversarial_gaps.rs:32-38
Code:
```
//! handoff lives in `media-probe`, which is not in this crate's
//! dev-dependency allowlist (boundaries.toml — a root-config change is
//! Lead-only). The end-to-end refusal is pinned by the CLI suites
```
What is wrong: The end-to-end B5 probe-vs-demux race is NOT tested here — only the demux-side backstop. Why it matters: documented deviation, honestly disclosed; the cross-crate claim (CLI suites pin it) checks out (apps/cli/tests/unsupported_path.rs exists). Not a defect, recorded scope limit. Confidence: HIGH — Verification: read + `ls apps/cli/tests/`.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| chunk | 74-83 | REAL | — | WORD-aligned RIFF chunk writer, correct pad rule |
| end_sized | 85-88 | REAL | — | size patch, correct -4 |
| avih | 90-108 | REAL | — | 56-byte avih, LE, flags/streams/frames |
| strh_video | 110-124 | REAL | — | 56-byte strh, scale=1/rate=30 |
| strf_video | 126-136 | REAL | — | 40-byte BITMAPINFOHEADER |
| frame_payload | 140-142 | REAL | — | distinct per-frame payload → distinct digests |
| build_avi | 150-217 | REAL | — | byte-truth builder; idx1 in caller order; JUNK injection |
| SplitMix64::next | 229-235 | REAL | — | correct public-domain constants |
| shuffle | 239-249 | REAL | — | Fisher-Yates, fixed seed (deterministic, documented) |
| misplaced_entries | 254-265 | REAL | — | mirrors parser running-max rule (index.rs:396-401) |
| demuxer_over_bytes | 271-280 | REAL | — | mem_stream + AviDemuxer::new |
| demuxer_over_stream | 282-287 | REAL | — | box<dyn SourceStream> |
| drain | 305-329 | REAL | — | full event loop incl. Recovered/Discontinuity |
| span_at_nanos | 331-340 | REAL | — | 1/1e9 base |
| landing_ticks | 343-348 | REAL | — | |
| assert_order_invariant | 360-450 | REAL | — | differential spine: walk, damage, 6-point seek grid, reanchor tail |
| DribbleStream::new | 699-713 | REAL | — | |
| SourceStream::read (Dribble) | 728-752 | REAL | — | 1..=bound, no div-by-zero (bound>0 guarded), exact EOF |
| SourceStream::seek (Dribble) | 754-776 | REAL | — | i128 overflow-safe, typed refusal |
| out_of_range | 784-792 | REAL | — | typed EngineError |
| stats_of | 794-805 | REAL | — | |
| assert_no_read_loop_pathology | 811-829 | REAL | — | linear-ceiling 4×file+64KiB, empty-read cap 64 |
| shuffled_idx1_walks_and_seeks_like_the_canonical_file | 452-482 | REAL | — | asserts shuffle actually disordered (moved>=4, misplaced>0) |
| fully_reversed_idx1_walks_and_seeks_like_the_canonical_file | 484-499 | REAL | — | misplaced = FRAMES-1 pinned |
| search_index_on_time_backwards_entries_stays_keyframe_safe | 505-560 | REAL | — | degraded early-break pinned exactly (target 3 → pts 0) — matches search_index code trace |
| mid_table_reversed_offset_run_seeks_and_reanchors_typed | 562-581 | REAL | — | misplaced == 2 pinned |
| duplicate_offset_entries_seek_typed_and_never_loop | 583-666 | REAL | — | safety properties (no loop, monotone time, real frames) |
| dribbled_parent_walk_matches_the_plain_walk | 831-881 | REAL | — | REAL fixture, 2 dribble caps, sha256 differential |
| dribbled_parent_reanchor_revives_and_resumes_the_canonical_tail | 883-924 | REAL | — | tail equality plain_rows[60..] |
| dribbled_no_index_scan_seeks_and_lands_within_tolerance | 926-985 | REAL | — | SlowLinear scan through dribble, recorded tolerance |
| mp4_signatured_bytes_refuse_typed_in_the_avi_path | 991-1035 | REAL | — | sticky typed refusal, sniffed prefix pinned |
| avi_bytes_behind_a_lying_mp4_name_demux_fully | 1037-1064 | REAL | — | through input::FileAccess (real access stack) |
| ftyp_magic_with_an_embedded_avi_body_is_refused_not_rescued | 1066-1111 | REAL | — | well-formed ftyp + valid AVI → refused |
| avi_with_mp4_bytes_as_junk_payload_demuxes_as_avi | 1113-1157 | REAL | — | JUNK skip is silent, walk identical |
| probe_hit_demux_refuse_disagreement_is_typed | 1159-1201 | REAL | — | RIFF+0xFF×512 → no-header; WAVE/RIFX → not-avi |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 13 | YES | Mutation reasoning: un-sort idx1 in parser → `reasons == vec![(misplaced*16, ...)]` and walk-equality asserts fail; return landing above target in search_index → `landing.pts_ticks <= target` fails; mis-walk on short reads → sha256 row differential fails; panic on any input → test fails. Assertions compare against independent expectations (hand-computed tick grids, committed fixtures), not outputs of the code under test. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "13 tests, all pass" | my run + baseline:2180-2198 | REAL | command output above |
| Fixtures exist (PARENT, MP4_PARENT, NOIDX0, expectation TOML) | ls tests/fixtures/... | REAL | all 4 present, 425516/424452/423588 bytes, TOML has real sha256 + seek grid |
| "pinned against the real fixture corpus" (header line 3) | file header | REAL | PARENT/NOIDX0 loaded via testkit::fixture_bytes with existence check |

## 7. Dead / duplicated / unreachable code
None. All helpers are used. `PARENT_EXPECTATION` reused across 3 tests.

## 8. Dependencies & flags
Uses `demux::avi::test_support` — a `pub mod` compiled into the release library (avi/mod.rs:121, documented as intentional doctests/fuzz seam). Test-only surface shipped in the prod crate: pre-existing design decision outside this file's scope; noted for the avi/mod.rs auditor.

## 9. Open questions for the next agent
- Who owns the W1-15 worklog "hardening candidate" for monotone duplicates (F-001)? Check it was ever actioned.
- The `expectation/1` parser in tests/common/mod.rs:495-498 (`parse_u64` strips non-digits, `unwrap_or(0)`) silently zeroes malformed numbers — fixture corruption would read as 0, not fail. Owned by whoever audits tests/common.
