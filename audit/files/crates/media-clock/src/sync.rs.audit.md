# AUDIT: crates/media-clock/src/sync.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 697 / 27 (3 consts, 3 types + 20 methods/fns, 14 #[test] in-file) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, video side: `SyncBands`/`DriftEstimator` are consumed by crates/video-engine/src/scheduler.rs (lines 78, 456-458, 932 apply_sync_ladder) + policy.rs:434, and the `PresentationScheduler` is constructed in the shipped CLI (apps/cli/src/pipeline_host.rs:704-769). AUDIO SIDE REACHABLE-BUT-DISARMED: the `DriftEstimator`+`RateWindow` consumer (audio-engine/governor.rs:57,81,87) is never armed in production — see F-002. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | python3 raw-byte reads (see note); rg for consumers (governor.rs, scheduler.rs, policy.rs, pipeline_host.rs); rg `with_drift_governor|arm_drift_governor` repo-src (test-only callers); baseline cargo-test-workspace.txt:3537-3552 (14 sync tests pass). |

## 1. One-paragraph verdict
Hostile question answered: sync.rs does NOT implement A/V sync — by its own documentation it is "the measurement half of the 3.B1 audio-master drift loop," and that is exactly what the code is: a correct, integer-only, well-tested EMA (DriftEstimator), a Schmitt-trigger band classifier (SyncBands), and a coalescing window (RateWindow). All three are pure functions of injected values; no clocks, no audio, no actuators. The 14 in-file tests are precise (hand-checked dyadic ladder, dwell/exit symmetry, frozen-source, backwards-jump, i64-extreme boundedness with a documented 2^k−1 stall point) and pass in baseline. The real sync story given NULL audio: the shipped CLI builds an audio-master clock over the null sink's sample counter (pipeline_host.rs:1605 `MediaClock::audio_master(sink.clock_source(), …)`), wall-paces the null device (host.rs:519-536 `pump_wallclock` renders "only the quanta the device is owed by wall time"), and the video scheduler's SyncBands ladder adapts presents against that clock — that much is real. But the audio-side drift-correction loop that sync.rs's windowed slope exists for is NEVER ARMED in any production path: `arm_drift_governor` has zero production callers (only a test harness), so the shipped "audio-master sync" is open-loop — honest about it in comments, but a full A/V sync loop it is not.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:3537-3552` -> all 14 `sync::tests::*` pass ("92 passed" crate-wide, 0 failed).
— verified by rg over crates/*/src: consumers are audio-engine/src/governor.rs:57 (`use media_clock::sync::{DriftEstimator, RateWindow};`), video-engine/src/scheduler.rs:78 (`use media_clock::sync::{DriftEstimator, SyncBand, SyncBands};`), video-engine/src/policy.rs:435 (`SyncBands::new(...)`).
— verified by rg `with_drift_governor|arm_drift_governor` over crates/*/src apps/*/src -> only definitions (audio-engine/pipeline.rs:138-156, apps/cli/pipeline_host.rs:639-640) and ONE test caller (audio-engine/tests/zero_alloc_resample.rs:73). No production arming.
— NOTE (environment): this sandbox's output pipeline strips the two-byte sequence `[m` from all displayed tool output (proven with `echo 'aa[mbb'` → `aabb`). The file on disk genuinely contains `#[must_use]` (python regex `#\[must_use\]` matched 12 times); displays showing `#ust_use]` are an artifact, NOT file corruption. Quotes below avoid `[m` sequences.

## 3. Findings
F-001 [S3]
Where: crates/media-clock/src/sync.rs:226-229
Code:
```
        let weight = (1u128 << self.shift.min(126)).saturating_sub(1) as i128;
        let combined =
            i128::from(error_ns).saturating_add(i128::from(self.ema).saturating_mul(weight));
        self.ema = saturate_i64(combined >> self.shift);
```
What is wrong: `for_shift_and_window(shift: u32, …)` accepts any u32 but the EMA math is only correct for shift ≤ 126. The weight is clamped (`min(126)`) but the final `combined >> self.shift` is not: shift = 127 silently uses weight 2^126−1 instead of 2^127−1 (wrong filter); shift ≥ 128 panics in debug ("attempt to shift right with overflow") and in release the shift amount is masked to shift%128 (silent garbage). The `min(126)` clamp proves the author considered large shifts but clamped the wrong operand. Why it matters: public L2 API with unvalidated input; a downstream engine tuning k would get a panic or a silently mistuned filter. No shipped caller is affected (`DriftEstimator::new()` = shift 3; tests use 2-3). What correct looks like: clamp `shift` itself in the constructor (or return a typed error). Confidence: HIGH — Verification: read.
F-002 [S2]
Where: crates/media-clock/src/sync.rs:1-4,20-25 (module claims) + apps/cli/src/pipeline_host.rs:604-612
Code (pipeline_host.rs:604-608):
```
    /// regime (16-packet bursts into a 3 s ring, drained per pump)
    /// makes the fill a *decode-ahead depth* metric instead; arming
    /// the governor there would "correct" buffering as if it were
    /// clock drift. The governor is therefore armed by compositions
```
What is wrong: The audio-master drift loop this module exists for ("The measurement half of the 3.B1 audio-master drift loop") is not closed in any shipped build. `DriftGovernor` (the audio-side consumer of `DriftEstimator::observe_windowed` + `RateWindow`) is constructed only via `with_drift_governor`/`arm_drift_governor`, whose only caller is a test (audio-engine/tests/zero_alloc_resample.rs:73); `AudioLimb::observe_drift` (pipeline_host.rs:613-623) feeds samples into an unarmed pipeline that "ignores the sample (correction stays ZERO)". `DRIFT_TARGET_FILL_NS` (pipeline_host.rs:101) is a pub const no production code reads. Net: with null audio (the shipped first vertical), rate-matching is impossible anyway (a null sink cannot drift), so the open loop is defensible — but the shipped "A/V sync" is: wall-paced null counter → clock → video ladder. The slope half of DriftEstimator (`slope_mppm`) has zero live readers in production. Why it matters: anyone reading "audio-master drift loop" into the shipped product is misled; the sync quality of the product rests entirely on the video-side ladder. What correct looks like: arm the governor when 3.B3 pacing lands (the code's own stated plan) or mark the audio half UNREACHABLE-in-production in this module's docs. Confidence: HIGH — Verification: read + rg (callers enumerated).
F-003 [S4]
Where: crates/media-clock/src/sync.rs:154-156
Code:
```
                    let slope = de
                        .checked_mul(1_000_000_000)
                        .map(|scaled| scaled / dt)
                        .unwrap_or(i64::MAX as i128);
```
What is wrong: The overflow arm is dead and sign-blind: |de| ≤ 2^64 and 2^64 × 1e9 ≈ 1.8e28 < i128::MAX ≈ 1.7e38, so `checked_mul` can never fail for i64-derived inputs; and had it failed on a negative de, the fallback saturates POSITIVE (i64::MAX). Harmless today, misleading as a pattern. Confidence: HIGH — Verification: read + arithmetic.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DEFAULT_EMA_SHIFT / DEFAULT_SLOPE_WINDOW_NS / NS | 44-51, 54 | REAL | — | k=3, 2 s window, [PROPOSAL] tagged. |
| DriftEstimator::new / for_shift_and_window | 100-119 | REAL | S3 | F-001: shift unvalidated. |
| DriftEstimator::observe_sample | 122-124 | REAL | — | EMA only (video cadence). |
| DriftEstimator::observe_windowed | 129-167 | REAL | — | Windowed slope, exact i128, frozen-source property; F-003 dead arm. |
| error_ema / slope_mppm / slope_ppm / samples | 170-211 | REAL | — | slope_ppm hand-rolled ceil-div, correct for negatives. |
| DriftEstimator::reset | 214-221 | REAL | — | Full regime reset. |
| ema_step | 218-231 (body 224-230) | REAL | S3 | F-001 shift > 126 broken. |
| Default for DriftEstimator | 234-238 | REAL | — | Delegates to new(). |
| saturate_i64 | 243-245 | REAL | — | Clamp, never wrap. |
| SyncBand (enum) | 262-281 | REAL | — | Hold/WidenLate/ExtendEarly; sign convention documented. |
| SyncBands::new / band / half_band_ns | 316-336 | REAL | — | dwell.max(1); half from u64 saturated. |
| SyncBands::classify | 338-388 | REAL | — | Schmitt: enter |e|>H sustained D, exit |e|≤H/2 sustained D; None decays streaks. |
| SyncBands::reset / clear_streaks | 390-400 | REAL | — | Regime reset. |
| RateWindow::new / permits / reset / window_ns | 424-470 | REAL | — | First-call opens+permits; saturating backwards-jump hold. |
| 14 in-file tests | 478-697 | REAL | — | Exact-value pins incl. i64-extreme boundedness with documented dyadic stall (MAX−7 for k=3). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ema_is_primed…converges | YES | Fixed-point equality. |
| ema_is_bounded_by_the_input_range… | YES | 4096 random samples + i64 extremes with monotonicity asserts. |
| ema_step_weight_matches_the_dyadic_formula | YES | Hand-checked ladder (200/350/262). |
| windowed_slope_measures_an_exact_ramp… | YES | Exact −1000 mppm / −1 ppm. |
| frozen_source_never_elapses… | YES | 100 frozen samples → None. |
| reset_restarts… | YES | Post-reset None then exact new value. |
| hold_band / entry_requires_dwell / exit_requires… / none_samples / reset_returns | YES | Streak/dwell/hysteresis pins with off-by-one symmetry noted in comments. |
| rate_window_* (3 tests) | YES | Open/permit-once/frozen/backwards/reset pins. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The measurement half of the 3.B1 audio-master drift loop" | lines 3-4 | REAL (as scoped) | File contains measurement only; actuators in video-engine scheduler.rs:932. |
| "Integer arithmetic throughout on the control path … no floats" | lines 29-31 | REAL | All math i64/i128; no float in file. |
| "Bounded for any input … an i64 extreme sample can never overflow the EMA" | lines 40-42 | REAL (for shift ≤ 126) | Test passes; F-001 is the shift-parameter exception. |
| (hostile) "real A/V sync given null audio" | — | SHALLOW as a product claim | F-002: audio-side loop unarmed in production; shipped sync = null-counter clock + video ladder only. |

## 7. Dead / duplicated / unreachable code
- `DriftEstimator::slope_mppm/slope_ppm` + `observe_windowed`'s window half: unreachable in production (governor never armed) — F-002.
- `RateWindow`: same — its only consumer (governor.rs) is production-disarmed.
- `DRIFT_TARGET_FILL_NS` const (pipeline_host.rs:101): no production reader.
- Deliberate duplication of ProgressWindow semantics (RateWindow) is documented at lines 21-25 as a layering necessity (L2 vs L4) — accepted.

## 8. Dependencies & flags
media-types only (L2 per boundaries.toml); no feature gates; no unsafe; zero allocation (Copy types). In-file test module uses testkit::Must.

## 9. Open questions for the next agent
- video-engine/src/scheduler.rs (another agent): does `apply_sync_ladder` actually change present behavior in the shipped CLI, or does the hold band dominate in practice (null-audio wall pacing never drifts)?
- When 3.B3 backpressure lands (per the pipeline_host comment), verify the governor arming call appears in the production open path — until then F-002 stands.
