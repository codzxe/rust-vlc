# AUDIT: crates/video-engine/tests/sync_scenarios.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 421 / 5 tests + 3 helpers |
| File verdict | REAL — card 3.B1's video convergence scenarios (S4 PTS-gap snap, S5 jitter no-flap, S10 ladder grids ×3) on the real `PresentationScheduler` over virtual clocks, with exact counter/band/threshold assertions |
| Reachable from shipped entry point? | N/A (test binary). The subject (PresentationScheduler/PresentationQueue/SyncPolicy) is the shipped video presentation path driven by the CLI host's pump |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (not executed — no cargo) |

## 1. One-paragraph verdict

Five convergence scenarios with production-shaped wiring
(tick → enqueue → drain-to-presenter per step, :35-52) and exact
assertions. S4: a +500 ms PTS step snaps exactly once
(`snap_presents == 1`, `sync_pts_gaps == 1`), drops nothing, resets the
estimator (observable as `error_ema_ns == None` on the post-gap tick),
and every post-snap error stays inside one frame with the band never
leaving Hold (:63-163). S5: seeded ±½-frame jitter over 150 ticks —
zero threshold changes, zero correction bands, zero drops, ≥ 140
presents, in-order delivery (:169-234). S10a: sustained one-frame
lateness widens the threshold to exactly 2 frames (clamp arithmetic
pinned) with a monotonicity/cap grid over the pure policy
(:254-307). S10b: a spent drop budget freezes the threshold at the
configured value while drops continue bounded — the firehose never
widens the definition of late (:313-363). S10c: sustained earliness
extends the repeat budget to exactly 5 (3 + 2) before background, with
a runaway guard inside the loop (:369-421).

## 2. What actually works (verified)

— All assertions are exact or floor-bounded with named counters
(`snap_presents`, `sync_pts_gaps`, `dropped_late`, `presented`,
`late_threshold_ns`, `band`) — the scheduler's observability surface is
real, not events (the no-event-storm claim, :8-11).
— The S4 phase-3 comment explains the push-before-due pacing choice
and why the alternative would measure the wrong thing (:104-109) —
the test author understood the scheduler's late policy.
— S10a's policy grid iterates −⅛F..−6F and asserts monotone widening
under the 3-frame cap — a property check inside a scenario test.
— Mutation reasoning (UNVERIFIED): a second snap fails the == 1; a
jitter-driven threshold move fails S5's == 0; budget-ignoring widening
fails S10b's == FRAME; a repeat-budget runaway trips the in-loop
assert (:416).

## 3. Findings

### F-001 [S4] S5's seed is a hardcoded constant; the "no flapping" property is proven for one stream, not a seed sweep
Where: sync_scenarios.rs:178-184

Code:
```
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        seed ^= seed << 13;
```
What is wrong: the sibling suites (model_walk, queue_policy) expose a
seed hook (`LUMEN_PROP_SEEDS`) and walk hundreds of seeds; this
scenario runs one fixed seed. The property ("±½-frame jitter never
flaps") is strong and the mechanism (hold band + EMA) makes it
seed-robust in principle, but a single-seed proof is weaker evidence
than the repo's own convention.
What correct looks like: loop the scenario over the `LUMEN_PROP_SEEDS`
convention (or 8 fixed seeds) with the same assertions.
Confidence: HIGH — Verification: read (convention comparison).

## 4. Per-function table

| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `step`/`sync_step`/`wide_queue` helpers | 29-56, 242-248 | REAL | — | production-shaped wiring |
| `s4_pts_step_discontinuity_snaps_exactly_once` | 63-163 | REAL | — | snap==1, gap==1, drops==0, EMA reset observable, band never leaves Hold |
| `s5_pts_jitter_never_flaps_the_threshold` | 169-234 | REAL | S4 | exact zeros; single seed (F-001) |
| `s10_sustained_latness_widens_the_threshold_in_band` | 254-307 | REAL | — | exact 2-frame widening + monotone/cap policy grid |
| `s10_spent_drop_budget_freezes_the_widening` | 313-363 | REAL | — | threshold frozen at FRAME while drops ≥ 120 |
| `s10_sustained_earliness_extends_the_repeat_budget` | 369-421 | REAL | — | exactly 5 repeats; in-loop runaway guard |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 5 | YES | exact counters/bands/thresholds; in-order delivery asserted; mutation analysis §2 |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the PTS-gap snap, the jitter no-flap property, and the hysteresis-ladder grids" | :1-3 | REAL | the five tests |
| "counters and per-tick SyncSamples only — never events" | :8-10 | REAL | the suite consumes only stats/SyncSample surfaces |
| "The audio half … lives in audio-engine's governor/resample suites; S7–S9 in apps/cli" | :4-6 | REAL (cross-file) | audio half verified in the audio-pipeline audit; S7–S9 in apps/cli/tests/sync_scenarios.rs (Grep confirmed the harness exists) |

## 7. Dead / duplicated / unreachable code

None (uses tests/common — audited).

## 8. Dependencies & flags

`video_engine`, `media_clock` (SyncBand), `media_types`, `testkit`,
`mod common`. No feature gates.

## 9. Open questions for the next agent

- Wave 6 (video-engine sink/policy owners): the S4 comment names the
  backward re-anchor exemption as S7's (composition root) — confirm
  apps/cli/tests/sync_scenarios.rs covers it (that file is not in my
  list; my files end here).
