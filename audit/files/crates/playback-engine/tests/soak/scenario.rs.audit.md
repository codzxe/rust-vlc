# AUDIT: crates/playback-engine/tests/soak/scenario.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 624 / 17 (1 `#[test]`) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test lane only — compiled into the `soak` test binary: tests/soak/main.rs:65 `use scenario::{CycleCounts, fixture_rotation, run_cycle};` and main.rs:215 `run_cycle(`. The engine it drives is shipped (see seek_flow.rs audit header). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `cargo test -p playback-engine --test soak scenario::tests::fixture_rotation` -> 1 passed (exit 0); baseline: `soak_loops_stay_memory_bounded ... ok` (120.04s, cargo-test-workspace.txt:4482-4497) |

## 1. One-paragraph verdict
The soak loop script is real and honest about its own fragility: the module docs admit that with ops placed after an advance, 100% of seeks were skipped on the ~2 s fixtures, and the fix (submitting seek/switch immediately off the open settle) is policed by the runner's D-6 asserts (skip rate ≤ 25%, ≥1 completed seek/switch — main.rs:260-286, verified by the passing 120 s baseline soak). Seek targets and switch plans are parsed from the committed expectation TOMLs, never transcribed, and the unit test pins the parsed values. Every completed cycle must decode frames (asserted in `finish_cycle`). Defects found are latent-lane contradictions and brittleness notes, not fakeness.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p playback-engine --test soak scenario::tests::fixture_rotation` -> `1 passed; 0 failed` (exit 0). Baseline cargo-test-workspace.txt:4482-4497: full soak `soak_loops_stay_memory_bounded ... ok` (120.04s) + `scenario::tests::fixture_rotation_parses_the_recorded_truth ... ok`.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/soak/scenario.rs:411-424 vs 570-573
Code:
```
    if exit == CycleExit::Error {
        // The dirty lane: no playback ops from `Error` (the table has
        // no edge) — close is the remaining exit.
        close_cycle(host, plan, cycle_index, phases, live)?;
        finish_cycle(host, counts, started, frames_before, exit);
        return Ok(());
    }
```
What is wrong: internal contradiction. The error-open lane counts `error_opens` and the runner (main.rs:251-254) subtracts error opens/closes from `playable_cycles` as if they were legitimate outcomes — but `finish_cycle`'s D-6 assert (`decoded > frames_before`) means any cycle that settles in `Error` at open with zero decoded frames (the only realistic error-open shape — see session_flow's `an_open_failure_lands_in_error_with_the_fatal_first`) PANICS the soak. The lane is a trap: a future fixture that legitimately errors at open can never pass. Currently harmless (all three rotation fixtures decode frames; error_opens == 0 in the passing baseline). What correct looks like: exempt `CycleExit::Error`-at-open cycles from the decoded-frames assert (or assert only for playable cycles). Confidence: HIGH — Verification: read (both branches traced). Confidence on "error opens decode zero frames": MEDIUM — no error-open fixture in the rotation to execute.

F-002 [S4]
Where: crates/playback-engine/tests/soak/scenario.rs:17-18 (module docs)
Code:
```
//! counted and bounded by the liveness asserts (D-6). Measured with
//! ops-after-advance: 1597/1597 cycles skipped the seek (100%).
```
What is wrong: this is a historical measurement of a previous op order that I cannot reproduce without editing the script (forbidden). The load-bearing consequence — that the CURRENT order keeps the seek skip rate low — is independently enforced by main.rs:260-268 (`seek_skip_rate <= MAX_SKIP_RATE` with `MAX_SKIP_RATE = 0.25`, main.rs:88) and the passing 120 s baseline, so the doc claim is not load-bearing for correctness. Confidence: HIGH that it is unverifiable; the current-order guarantee is REAL (baseline). Verification: UNVERIFIED (the 1597/1597 figure).

F-003 [S4]
Where: crates/playback-engine/tests/soak/scenario.rs:153-190
Code:
```
            ("seek", "targets_us") => {
                // targets_us = [0, 500_000, 1_000_000, 1_999_000]
                let inner = value.trim().trim_start_matches('[').trim_end_matches(']');
                for item in inner.split(',') {
                    let digits: String = item
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '-')
                        .collect();
```
What is wrong: hand-rolled TOML scanning by digit-filtering (also the `[tracks] audio` inline-table id scan at 166-184 and `switch_plan` at 185-190). Brittle against TOML shapes the committed corpus doesn't use (e.g. multi-line arrays, `id=-1` sign handling, arrays of tables). Accepted because "the parsing is part of the proof surface" (comment 75-76) and the unit test pins the parsed truth for the actual corpus; a corpus evolution (multiline arrays) would fail loudly rather than mis-parse, mostly. Confidence: MEDIUM — Verification: read; multiline-array behavior is asserted to refuse in the soak_scenarios manifest tests (different system) — not verified for this scanner (single-line arrays only in the committed expectations).

F-004 [S4]
Where: crates/playback-engine/tests/soak/scenario.rs:338-347
Code:
```
    let target = match cycle_index % 4 {
        0 => WireSeekTarget::Absolute {
            time: Timestamp(pick(0)),
        },
        1 => WireSeekTarget::Relative { delta_ms: 250 },
        2 => WireSeekTarget::Percent { fraction: 0.4 },
```
What is wrong: the Relative (+250 ms) and Percent (0.4) shapes are fixed constants across all cycles — no rotation, so the soak exercises exactly one relative delta and one fraction. Minor coverage note; the resolution paths themselves are unit-covered in src/seek.rs. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FixturePlan (struct) | 58-71 | REAL | — | targets/ids/switch from recorded TOML |
| FixturePlan::from_expectation | 86-114 | REAL | — | parent fallback for dirty class-B facts; typed errors |
| FixturePlan::path | 120-123 | REAL | — | existence-checked via testkit |
| parse_expectation | 127-201 | REAL | S4 | digit-filter scanner (F-003) |
| parent_of | 205-221 | REAL | — | maps `parent =` to expectations/valid/… |
| fixture_rotation | 230-248 | REAL | — | 2 valid parents + 1 dirty truncation |
| CycleCounts | 256-295 | REAL | — | 14 counters incl. skip lanes and sink totals |
| CycleExit | 298-306 | REAL | — | |
| open_request / seek_request / switch_request / close_request | 313-376 | REAL | S4 | fixed Relative/Percent constants (F-004) |
| mid_media_targets | 357-360 | REAL | — | drops end-adjacent target; single-target honest |
| run_cycle | 386-500 | REAL | S4 | the script; error lane contradiction (F-001) |
| in_playback | 503-508 | REAL | — | Playing \| Buffering — the table's seek-serving states |
| switch_selectors | 513-528 | REAL | — | multi-track: to+back; single: deselect+reselect |
| advance | 531-535 | REAL | — | bounded 3 polls |
| close_cycle | 538-555 | REAL | — | one dispatch, phase-attributed |
| finish_cycle | 562-586 | REAL | — | D-6 liveness assert (570-573); sink fold (survives close per dispatch.rs:402-407 — verified by read) |
| tests::fixture_rotation_parses_the_recorded_truth | 596-623 | REAL | — | pins parsed targets [0, 500_000, 1_000_000, 1_999_000], audio ids, switch_to Some(2), dirty-from-parent; executed |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixture_rotation_parses_the_recorded_truth | YES | exact parsed values (targets, ids, switch_to) — a fixture regeneration that changed the corpus or a parser regression fails it; executed. |
| run_cycle (via main.rs soak, baseline) | YES | any op erroring outside the state-skip lanes returns Err and fails the soak; a cycle decoding no frames trips the D-6 assert; a missing seek terminal trips "no seek terminal recorded" (445). The skip-rate and ≥1-completed asserts live in main.rs:246-286 (not this file). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seek targets and track ids are read from the fixtures' own expectation files, never transcribed" | module docs 3-6 | REAL | parse_expectation + unit test pins (602-621); executed |
| "Measured with ops-after-advance: 1597/1597 cycles skipped the seek (100%)" | module docs 17-18 | REAL-UNPROVEN (historical) | cannot rerun the old order (read-only); current order policed by main.rs MAX_SKIP_RATE=0.25 + passing baseline (F-002) |
| "A soak that skips its ops is a ritual, not a measurement — the scenario fails loudly on that" | module docs 34-35 | REAL (for this file's part) | finish_cycle assert 570-573; the skip-rate half lives in main.rs:260-278 |
| "the pipeline drains to `Ended` within a tick or two of priming" (~2 s fixtures) | module docs 10-13 | REAL-UNPROVEN | consistent with fixtures' ~2 s extent and the 100%-skip note; not directly measured by me |

## 7. Dead / duplicated / unreachable code
`CycleCounts::error_opens`/`error_closes` and the `CycleExit::Error`-at-open lane are currently-dead machinery (F-001: the lane cannot complete without tripping its own liveness assert). The scanner style is deliberately duplicated from tests/track_switch_fixture.rs (documented precedent).

## 8. Dependencies & flags
commands (wire types), playback_engine::state, testkit (fixture_path, Must, alloc_count::serialize_tests in the unit test), super::dispatch/memory (sibling modules). No feature gates, no unsafe. The unit test takes the allocation-serializer lock (`serialize_tests`) — correct discipline for the counting allocator.

## 9. Open questions for the next agent
- soak/dispatch.rs and soak/main.rs (not my files): confirm `MAX_SKIP_RATE`/`MIN_CYCLES` constants (read: 0.25 / 3 at main.rs:85-88) and that the runner prints per-op counts into the report (report.rs — my file 11).
- Would a multiline `targets_us` array (valid TOML) silently yield ZERO targets here (scanner reads only same-line content) → then from_expectation falls back to parent or errors? Trace: line-scanner sees `targets_us = [` with empty inner → no targets → parent fallback (dirty) or hard error (valid parents). For a valid parent that would be a loud setup error — acceptable, but confirm.
