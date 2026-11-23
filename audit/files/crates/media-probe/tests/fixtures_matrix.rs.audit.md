# AUDIT: crates/media-probe/tests/fixtures_matrix.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 612 / 8 tests + 8 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test) — executed via prebuilt binary `target/debug/deps/fixtures_matrix-338ad049205a8e4b` (8 passed, exit 0). The code under test (probe signatures) is reachable from `lumen open` via demux_host.rs:444-458. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `target/debug/deps/fixtures_matrix-338ad049205a8e4b` → 8 passed, 0 failed (exit 0); `ls tests/fixtures/expectations/{valid,malformed}` (3 + 22 TOMLs); rg coverage checks for absidx1/negheight0/avi_mjpeg_2pcm |

## 1. One-paragraph verdict
This is a real differential matrix, not an echo chamber: 22 committed binary fixtures probed
through the real access stack (`input::FileAccess` → SourceStream), with expected verdicts
derived from the fixtures' own recorded expectation TOMLs (re-read at run time, not transcribed),
asserting format, exact score rung (70/90/95), ordered track hints, and exact `bytes_examined`.
Executed: 8/8 pass. The registry-count pin (`with_builtins().len() == 14`) independently
confirms this wave's cross-cutting finding: 14 builtin probe signatures, no ASF. Two doc-level
defects only: the header's "the whole committed probe corpus … all 20 class-B derivatives"
undercounts the committed tree (22 derivative + 3 valid expectation TOMLs exist; 3 fixtures are
exercised only by other crates' suites), and the summary table's "flip1 — like the parent" row
contradicts the MP4 flip1 verdict the test itself asserts (AUDIO_ONLY).

## 2. What actually works (verified)
- `target/debug/deps/fixtures_matrix-338ad049205a8e4b` → `test result: ok. 8 passed; 0 failed` (exit 0).
- Mutation sensitivity (reasoned): any change to signature scoring, hint harvesting, window
  accounting, or the unknown-refusal shape fails an exact `assert_eq!` (lines 171-200, 207-225).
- The expectation TOMLs are parsed live (`recorded_track_codecs` / `recorded_derivation`,
  lines 55-125) and their content asserted against the probe output (lines 247-256, 300-302,
  401-403, 486-487) — verdicts are pinned to recorded nature, not to the probe's current output.

## 3. Findings
F-001 [S4]
Where: crates/media-probe/tests/fixtures_matrix.rs:1-3
Code:
```rust
//! Card 2.A2's fixture-classification AC (doc 13 wave 2.A row 2.A2):
//! the whole committed probe corpus — both class-A parents and all 20
//! class-B derivatives — classified through the **real access stack**
```
What is wrong: "the whole committed probe corpus … all 20 class-B derivatives" is false as a
description of the committed tree. `tests/fixtures/expectations/malformed/` holds **22** class-B
derivative TOMLs — `avi_mjpeg_pcm__absidx1` and `avi_mjpeg_pcm__negheight0` are never run by
this suite (they are exercised only by crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:131,135
and tests/integration/first-vertical/tests/recovery_matrix.rs:650-664) — and
`expectations/valid/avi_mjpeg_2pcm.avi.toml` (a third parent) is covered only by
tools/bench-vs-vlc. The generated corpus tree is far larger (33 valid + 81 malformed files;
other suites cover other slices). The tests themselves are honest about their scope; the header
sentence overclaims. What correct looks like: "the 2.A2 avi/mp4 family (2 parents + the 20
derivatives this card's AC names)". Confidence: HIGH — Verification: ls + rg.
F-002 [S4]
Where: crates/media-probe/tests/fixtures_matrix.rs:18 (doc table) vs 441-450 (assertion)
Code (doc table row):
```rust
//! | `flip1` | like the parent | advisory byte, structure intact |
```
Code (the MP4 reality the test asserts):
```rust
    // flip1: the video trak's tkhd size field lies (0x5c → 0x40005c)
    // — the walk cannot reach that trak's mdia, so the video hint
    // degrades away honestly; the untouched audio trak still hints.
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__flip1.mp4",
        &Expected::Hit {
            format: "mp4",
            score: 95,
            hints: AUDIO_ONLY,
```
What is wrong: the summary table says flip1 stays "like the parent" (both hints); the MP4 flip1
expectation is AUDIO_ONLY — the video hint degrades. The table row is true only for the AVI
flip1. Internal doc-vs-test contradiction inside one file. Confidence: HIGH — Verification: read
+ sed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| recorded_track_codecs | 55-71 | REAL | — | live TOML scrape, fail-loud asserts |
| inline_string_field | 75-87 | REAL | — | quoted-value scrape |
| recorded_derivation | 97-125 | REAL | — | op + cut_offset scrape |
| kind_of | 149-155 | REAL | — | — |
| assert_verdict | 157-228 | REAL | — | exact format/score/hints/window; Unknown shape fully asserted |
| avi_parent / mp4_parent tests | 234-289 | REAL | — | executed, pass |
| avi_class_b / mp4_class_b tests | 295-496 | REAL | — | 20 verdicts executed, pass |
| a_small_window_degrades_the_verdicts_honestly | 502-536 | REAL | — | window as classification input, executed |
| every_fixture_is_window_bounded_whatever_the_verdict | 538-573 | REAL | — | Ok/Err both bounded; executed |
| the_ladder_rungs_are_what_the_matrix_assumes | 579-593 | REAL | — | registry len == 14 pin (cross-verifies no-ASF-signature) |
| parents_report_shape_matches_the_card | 595-612 | REAL | — | Display shapes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 8 | YES | exact assert_eq on format/score/hints/bytes_examined over committed bytes through the real FileAccess stack; any scoring or harvest regression fails (executed 8/8 green) |
| (note) | — | `every_fixture_is_window_bounded…` accepts either Ok or Err per file (weaker, but the bound claim is what it tests) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the whole committed probe corpus — both class-A parents and all 20 class-B derivatives" | fixtures_matrix.rs:1-3 | FALSE | 22 derivative + 3 valid expectation TOMLs committed; 3 fixtures not run here (F-001) |
| "`flip1` — like the parent" | fixtures_matrix.rs:18 | FALSE-for-MP4 | test asserts AUDIO_ONLY at 444-450 (F-002) |
| "Every expected verdict below is derived from the fixture's own recorded nature … not from the probe's current output" | fixtures_matrix.rs:6-10 | REAL | live TOML reads at 247/300/401/486 verified in source; executed |
| registry = "the 2.A2 pair + the eight Phase-5 R3 rows + the four raw-ES rows" = 14 | fixtures_matrix.rs:584-590 | REAL | my own read of signature/mod.rs:130-147 confirms 14 registrations (and no ASF) |

## 7. Dead / duplicated / unreachable code
None. All helpers are used; all referenced fixtures exist (the suite would have panicked on a
missing fixture via `testkit::Must`).

## 8. Dependencies & flags
- Dev-deps: `input` (real access stack), `testkit`, `media_types`, `media_probe`. No features.
- The suite depends on the committed corpus under `tests/fixtures/` (workspace root, resolved
  via `testkit::fixture_path`) — not on any generated-at-test-time bytes.

## 9. Open questions for the next agent
- `probe_io.rs` (10 tests) and `registry_roundtrip.rs` (3 tests) are this file's siblings —
  not in my assignment; the doc's two "honest deviations" (lines 25-32) cite probe_io.rs as the
  proof site for pure-garbage and sub-magic cuts — someone should verify those tests actually
  do that.
- absidx1/negheight0 run only in playback-engine + first-vertical suites with DIFFERENT
  expectations (clean Ended, full payout) — confirm no drift between the expectation TOMLs'
  recorded nature and those suites' assertions.
