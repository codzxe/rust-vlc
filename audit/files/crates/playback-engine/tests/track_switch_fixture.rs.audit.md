# AUDIT: crates/playback-engine/tests/track_switch_fixture.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 895 / 24 (21 helpers + 3 tests) |
| File verdict | REAL (real FileAccess → real AviDemuxer → real MjpegDecoder/PcmDecoder → real Engine; recorded-truth plan parsed from the committed expectation TOML; airtight drain assertions) |
| Reachable from shipped entry point? | The composition under test is the shipped stack: `input::FileAccess` (line 338), `demux::avi::AviDemuxer` (line 339), `codecs::video::mjpeg::MjpegDecoder` (line 258), `codecs::audio::pcm::PcmDecoder` (line 266), `playback_engine::Engine` (line 381-382) — the same crates the lumen-cli release lane links (baseline engine-info: decoders mjpeg, pcm_s16le). The test binary itself is CI-only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `sed -n` quotes (exit 0); `sha256sum tests/fixtures/generated/valid/avi_mjpeg_2pcm.avi` -> `14d3a0e9...868f3` (matches expectation TOML sha256, size 497160 matches `size_bytes = 497_160`); baseline cargo-test-workspace.txt:4604-4610 — 3 passed / 0 failed in 0.03 s |

## 1. One-paragraph verdict
This is the real-container proof the fake-harness suite (track_switch.rs) defers to, and it delivers: the committed multi-track AVI (sha256-verified against its expectation TOML) is opened through the real byte-source, real demuxer, and real MJPEG/PCM decoders, driven through a mid-play A→B→A audio switch with the full ordered event-sequence assertions, and — unlike track_switch.rs case 1 — the no-stale-frames proof here is airtight: assertion (g) checks BOTH that every old-track frame carries a pre-mint generation stamp AND that the old track's max pts never advances past its pre-switch value. `poll_until_terminal` (lines 494-514) accumulates the output stream across polls, fixing the batch-scoping weakness of the sibling suite. Only hygiene-level debt: a hand-rolled 80-line TOML-subset parser duplicating machinery that exists in tests/soak/scenario.rs, a hard-coded `>= 18` video-frame floor where the recorded truth says 20, and implicit (not index-compared) frame-before-terminal ordering.

## 2. What actually works (verified)
— Baseline: cargo-test-workspace.txt:4604-4610 — all 3 tests pass in 0.03 s (real decode of a 497 KB fixture that fast is plausible: 20 MJPEG frames at 320x240).
— Fixture integrity: `sha256sum` -> `14d3a0e9a842e66710524dcbd6ccd17caf462541a2d8335327854698479868f3` == expectations/valid/avi_mjpeg_2pcm.avi.toml:10; size 497160 == toml:11.
— Plan values pinned: the_recorded_switch_plan_parses (lines 852-860) asserts switch_at_ns=1_000_000_000, from=1, to=2, U8/44_100/1ch — all match the committed [switch_plan] section (toml:78-81), verified by read.
— Decoder-cache round-trip on REAL decoders: build counts stay 1 per track, `decoders_reused == 1` (line 795); cross-session death proven by the second test (line 893-894).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/track_switch_fixture.rs:105-183
Code:
```
fn toml_int(value: &str) -> i64 {
    value
        .trim()
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
```
What is wrong: a third hand-rolled TOML-subset scanner in the crate's test tree (this one ~80 lines; tests/soak/scenario.rs has its own `parse_expectation` scanner for the same files). The same expectation format is parsed by two independent implementations that can drift (e.g. this one silently ignores unknown keys in [switch_plan] and unknown fields in `post_switch`). For a committed, hash-pinned fixture the risk is contained — hence S4. What correct looks like: one shared expectation parser in testkit. Confidence: HIGH — Verification: read both parsers.

F-002 [S4]
Where: crates/playback-engine/tests/track_switch_fixture.rs:834-837
Code:
```
    assert!(
        frames_of(&frames, video).len() >= 18,
        "the video limb decoded (nearly) the whole stream"
    );
```
What is wrong: the recorded truth (expectation toml:43 `video_sample_count = 20`) is not what the assertion reads — 18 is a hard-coded floor allowing a 10% silent drop. The docstring is honest ("nearly"), and a gross stall is still caught, but the number is transcribed rather than read, violating this very file's own "never transcribed" doctrine (header line 23-25). Confidence: HIGH — Verification: read + toml:43.

F-003 [S4]
Where: crates/playback-engine/tests/track_switch_fixture.rs:686-693
Code:
```
    let first_new = to_frames.first().must();
    // (e) The re-prime: the new track's first frame follows the
    // switch generation (doc 03 §8's pass condition).
```
What is wrong: the frame-precedes-terminal ordering is proven only implicitly — `to_frames` must be non-empty when `poll_until_terminal` returns at the terminal's poll, so an engine emitting the terminal before ANY new-track decode fails via `.must()`. That is a genuine proof, but it lives in a panic-on-empty rather than an explicit ordering assertion; a future refactor of `poll_until_terminal` to "return after limit polls" would silently weaken it. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| toml_int / toml_quoted | 105-121 | REAL | S4 | hand-rolled scanner (F-001); loud panics on garbage |
| parse_switch_plan | 123-183 | REAL | S4 | reads [switch_plan] only; pinned by its own test |
| SharedNow::now_ns | 194-198 | REAL | — | frozen virtual clock |
| Collector::accept | 207-215 | REAL | — | unbounded Vec acceptable for a 2 s fixture |
| RealProvider::build_count | 229-238 | REAL | — | recycling oracle |
| RealProvider::decoder_for | 240-273 | REAL | — | REAL decoders; unreachable! guards are test assertions |
| fixture_queues / fixture_budget | 289-318 | REAL | — | sizing rationale documented and load-bearing |
| open_fixture | 335-391 | REAL | — | real FileAccess + AviDemuxer + pcm-fact harvest |
| stamp_ns / frames_snapshot / frames_of | 393-415 | REAL | — | accessors |
| video_track | 418-425 | REAL | — | id from container, not assumed |
| step / poll_until | 430-453 | REAL | — | drains outbox between windows (ordering hygiene) |
| poll_until_audio_at | 457-486 | REAL | — | loud failure with seen-frame count |
| poll_until_terminal | 494-514 | REAL | — | ACCUMULATES the stream — fixes track_switch.rs's batch scoping |
| selection_terms / receipt_for / index_of | 516-549 | REAL | — | |
| test real_fixture_audio_switch... | 558-842 | REAL | — | (a)-(j) all exact-value; (g) double-anchored (generation + pts max) |
| test the_recorded_switch_plan_parses | 851-860 | REAL | S4 | pins committed values (verified vs toml) |
| test the_decoder_cache_dies_with_the_session | 865-895 | REAL | — | stop→close→reopen; decoders_reused == 0 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| real_fixture_audio_switch_follows_the_recorded_plan | YES | exact generation stamps, shape equality, pts bounds, clock equality, build counts, stats counters — a stale-frame leak, a wrong-track decode, a rebuild-per-switch, or a wrong clock anchor all fail it |
| the_recorded_switch_plan_parses | YES | exact values, verified against the committed TOML |
| the_decoder_cache_dies_with_the_session | YES | decoders_reused==0 after reopen; a cross-session cache fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "opened through the real byte-source → demuxer → decoder stack" | header:4-6 | REAL | lines 338-339, 258, 266; sha256-verified fixture |
| "Every number this test switches on is read from the fixture's expectation TOML, never transcribed" | header:23-25 | SHALLOW | true for the switch plan; the video floor `>= 18` (line 835) IS transcribed (F-002) |
| "zero old-track frames past the boundary" (proof 4) | header:41-45 | REAL | lines 710-725: generation stamp AND pts-max equality |
| "the real PCM decoder instances round-trip through the per-session decoder cache" (proof 7) | header:49-53 | REAL | lines 775-795 |

## 7. Dead / duplicated / unreachable code
- TOML-subset scanner duplicated vs tests/soak/scenario.rs (F-001).
- No dead code found; no `#[ignore]`; no feature gates.

## 8. Dependencies & flags
Test-only; dev-dependency composition (demux, codecs, input) explicitly exempted from layer ordering per header:11-19 (boundaries dev-edge rule). Runs in the default lane (baseline).

## 9. Open questions for the next agent
- tests/track_switch_alloc.rs (the counting-allocator recycling proof, referenced in track_switch.rs:17-18) is unassigned to me — confirm it exercises the real MJPEG DecodeContext, not the fake provider.
- The `[audio_probe]` for stream 2 is an "honest omission" per toml:35-37 — if any doc claims per-stream audio probes exist for this fixture, that claim is FALSE.
