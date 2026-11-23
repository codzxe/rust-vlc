# AUDIT: crates/subtitles/tests/timing_clock.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| Lines / functions | 223 / 7 (3 helpers + 4 tests) |
| File verdict | REAL (acceptance suite over the real MediaClock + compositor; executed, all green) |
| Reachable from shipped entry point? | N/A (test file) — runs in the workspace test lane; baseline cargo-test-workspace.txt `PACKAGE subtitles exit_code=0` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p subtitles --test timing_clock` → 4 passed / 0 failed (exit 0); rg on media-clock for the hold property; sed quote verification |

## 1. One-paragraph verdict
This is a genuine acceptance suite, not a mock parade: it drives the real `MediaClock::audio_master` over a counting `SampleCounterSource` at 48 kHz and asserts the half-open `[t0,t1)` cue rule at exact sample boundaries (47_999 off / 48_000 on / 95_999 on / 96_000 off), the adjacent-cue handover at exactly 2 s with no double-draw, nanosecond-granularity boundaries through the same compose path, and zero-duration/open-ended semantics — with pixel-level verification (`target.pixel(32,50)`), not just draw counts. All 4 tests pass when run. The single weakness is documentation-shaped: the suite's internal validity silently depends on the clock's hold-on-regression property (which it never regresses the counter to test), and that property is only asserted in prose in another crate.

## 2. What actually works (verified)
— verified by `cargo test -p subtitles --test timing_clock` → 4 passed: `cue_duration_is_sample_exact_on_the_real_clock`, `adjacent_cues_hand_over_without_gap_or_double_draw`, `cue_duration_is_nanosecond_exact_through_compose`, `zero_duration_and_open_ended_cues`.

## 3. Findings
F-001 [S4]
Where: crates/subtitles/tests/timing_clock.rs:102-106
Code (verbatim):
```
    // The interior midpoint (72_000 samples = 1.5 s): ON — checked
    // BEFORE the end boundary because the real clock is monotonic
    // and holds its position across a counter regression (doc 03
    // §4.7: never backwards).
```
What is wrong: The comment invokes a load-bearing property — that `PlaybackClock::now` holds its last position across a source regression (true per media-clock/src/clock.rs:28 "PlaybackClock::now then *holds* its last position", verified by read) — to justify the test's strictly-increasing counter sequence, but no case in this suite ever regresses the counter, so the property is never exercised. If the compositor were fed through a clock whose `now()` did NOT hold (or if someone "simplifies" `drawn_at` to reorder probes), the sample-exact claims could silently stop being reproducible. A one-line negative test (regress to 47_999 after 48_000 and assert the clock held ≥1 s) would pin it. Confidence: HIGH — Verification: read of both files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Counter + impl SampleCounterSource | 19-28 | REAL | — | 48 kHz stand-in, Relaxed loads fine here |
| ns / cue_from / drawn_at helpers | 30-61 | REAL | — | fresh-frame fill before each compose ✓ |
| cue_duration_is_sample_exact_on_the_real_clock | 63-142 | REAL | — | 5 boundary probes + pixel asserts; would fail on off-by-one anywhere |
| adjacent_cues_hand_over_without_gap_or_double_draw | 144-167 | REAL | — | 1-drawn invariant across the swap instant |
| cue_duration_is_nanosecond_exact_through_compose | 169-194 | REAL | — | 6 ns probes incl. ±1 ns around both bounds |
| zero_duration_and_open_ended_cues | 196-222 | REAL | — | zero never draws; Missing end draws forever |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| cue_duration_is_sample_exact_on_the_real_clock | YES | inclusive-start/exclusive-end each probed at ±1 sample with pixel verification |
| adjacent_cues_hand_over_without_gap_or_double_draw | YES | a gap or double-draw at 96_000 breaks the exact `1` |
| cue_duration_is_nanosecond_exact_through_compose | YES | ±1 ns probes at both boundaries |
| zero_duration_and_open_ended_cues | YES | zero-draw and forever-draw both pinned |
No `#[ignore]`, no tautologies, no mocks of the thing tested (the clock is the real media-clock crate; only the audio device is a counter stub — that is the documented seam).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the half-open `[t0,t1)` interval flips on/off at exact SAMPLE boundaries (48 kHz), and at exact NANOSECOND boundaries through the same compose path" | timing_clock.rs:5-7 | REAL | 4/4 tests pass (run) |
| "exactly the production shape (doc 03 §4.7)" | timing_clock.rs:3 | REAL-UNPROVEN | shape matches compositor+clock APIs (read); no production render loop in-tree to compare against (crate unreachable — eia608 audit F-001) |
| "a stale frame would keep old cue pixels" (fresh-frame rationale) | timing_clock.rs:48-49 | REAL | `target.fill(Rgba::BLACK)` before every compose (read) |

## 7. Dead / duplicated / unreachable code
None. The suite is wired into the package's integration tests and ran in the orchestrator baseline.

## 8. Dependencies & flags
dev-deps: media-clock (the real clock — the declared 2.D1-style dev edge), media-types, testkit. No feature gates. Deterministic (no sleeps, no threads beyond Arc counter).

## 9. Open questions for the next agent
- compositing/compose.rs and compositing/timing.rs are where the half-open rule actually lives — unaudited by me; this suite pins its behavior from the outside only.
- media-clock's hold-on-regression (clock.rs:28) deserves its own acceptance probe if any wave touches clock backends.
