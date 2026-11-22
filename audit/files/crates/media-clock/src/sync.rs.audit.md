# AUDIT: crates/media-clock/src/sync.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 697 / 20 functions + 15 tests |
| File verdict | REAL-UNPROVEN (video-side reachable; audio-side consumers UNREACHABLE in the shipped build) |
| Reachable from shipped entry point? | SPLIT — YES for `SyncBands`/`DriftEstimator` (video): `lumen` bin → apps/cli/src/pipeline_host.rs:1324 `PresentationScheduler::new(cfg, clock)` → crates/video-engine/src/scheduler.rs:456-458 (`estimator: DriftEstimator`, `bands: SyncBands`) → scheduler.rs:869 `.classify(...)` → scheduler.rs:932 `apply_sync_ladder`. NO for the audio-side loop (`observe_windowed`/`RateWindow` via audio-engine governor): the governor is armed only by the test harness (apps/cli/tests/sync_scenarios.rs:93); no production caller of `arm_drift_governor` exists (rg over apps/ + crates/: only pipeline_host.rs:639 definition, audio-engine/src/pipeline.rs:145/155 definition, and the test call). |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:2 |
| Commands run | `sed -n` quote verifications (sync.rs 112-124, 226-230; scheduler.rs 860-880, 930-980; pipeline_host.rs 620-660, 1290-1330) — exit 0; `rg` consumer searches over crates/ and apps/. No cargo (adaptation). |

## 1. One-paragraph verdict

The math is real. `DriftEstimator` is a correct dyadic integer EMA (`ema = (x + (2^k−1)·ema) >> k`, convex-combination boundedness genuinely holds), the slope computation is exact i128 milli-ppm arithmetic, `SyncBands` is a correctly implemented Schmitt trigger (enter |ê|>H sustained D, exit |ê|≤H/2 sustained D, `None` samples decay streaks), and `RateWindow` has the documented frozen-source/backwards-jump properties. The unit tests are specific and would catch mutations. The hostile question resolves as: **half real, half dead in the shipped build**. The video-side ladder is genuinely wired into the production CLI path (PresentationScheduler), but the audio-side drift loop — the estimator's `observe_windowed`/`RateWindow` consumers in audio-engine's `DriftGovernor` — is never armed in any shipped entry point; the production open path leaves the limb unarmed by its own admission, so the micro-resampler's correction is permanently ZERO in the product and A/V drift is never absorbed on the audio axis. Plus one latent shift-overflow panic on the pub constructor. Runtime verification UNVERIFIED (no cargo); W00 baseline cited where relevant.

## 2. What actually works (verified)

- Dyadic EMA correctness read-verified (sync.rs:220-230): first sample primes; constant input is an exact fixed point; the i128 intermediate plus the convexity argument make the i64 saturation unreachable except at i64 extremes. The test at 481-528 (`ema_is_bounded_by_the_input_range_for_any_sequence`, xorshift, 4096 samples + i64 extremes) would fail on any overflow/runaway mutation — reasoned, UNVERIFIED by run.
- Slope math (sync.rs:145-160): `de·1e9/dt` in i128 with exact-integer ramp test (`windowed_slope_measures_an_exact_ramp_in_milli_ppm`, −1000 ns/s ⇒ −1000 mppm ⇒ −1 ppm) — the units and the rounding are pinned by hand-checked assertions.
- `slope_ppm` ceiling division (185-195): truncation-toward-zero plus remainder>0 bump equals round-toward-+∞ for both signs — hand-checked (−1500→−1, −500→0).
- Schmitt trigger (334-381): entry/exit asymmetric thresholds and dwell; the hysteresis-gap persistence (classify at H/2<|e|≤H keeps band) is pinned by `exit_requires_half_amplitude_sustained_for_dwell` (620-640).
- Video-side reachability chain verified by rg + read: pipeline_host.rs:1324 → scheduler.rs:456/458/869/932.

## 3. Findings

### F-001 [S2] The audio-side drift loop (estimator + RateWindow) is never armed in the shipped build — A/V drift correction on the audio axis is dead code in the product
Where: crates/media-clock/src/sync.rs:132-163 (`observe_windowed`), 415-457 (`RateWindow`); consumer chain crates/audio-engine/src/governor.rs:57,81,87; arming gap apps/cli/src/pipeline_host.rs:633-640
Code (pipeline_host.rs:633-640, verbatim):
```rust
    /// Arm the drift governor (the late-arming composition path —
    /// [`AudioLimb::observe_drift`]'s regime note): the production
    /// open path leaves the limb unarmed; a composition that paces
    /// production at the consumption rate arms with
    /// [`crate::pipeline_host::DRIFT_TARGET_FILL_NS`] (or its own
    /// target).
    pub fn arm_drift_governor(&mut self, target_ns: u64) {
        self.pipeline.arm_drift_governor(target_ns);
    }
```
What is wrong: `DriftEstimator::observe_windowed` and `RateWindow` exist solely for the audio governor (module doc sync.rs:12-16: "the audio governor feeds it per-quantum fill errors"). The governor is armed only by `apps/cli/tests/sync_scenarios.rs:93`. No shipped entry point (`lumen` CLI, desktop shell, web host) calls `arm_drift_governor`; in production `AudioPipeline::observe_drift` (audio-engine/src/pipeline.rs:186-189) hits the `governor: None` early-return and the correction stays `DriftCorrection::ZERO` forever.
Why it matters: doc 03 §4.7(e) (cited in the module header, sync.rs:5-6) assigns drift to two absorbers; in the shipped build only the video ladder operates. Long-run A/V skew on the audio axis is never measured or corrected — the "A/V sync" story is half real. The code is honest about the arming gap, but the feature as shipped does not exist.
What correct looks like: the production open path arms the governor with `DRIFT_TARGET_FILL_NS` (the constant already exists in pipeline_host.rs), or the docs claim only the video ladder.
Confidence: HIGH — Verification: read + rg caller search (definition sites only; sole call is the test).

### F-002 [S3] `ema_step` shift-overflow: `combined >> self.shift` with raw shift panics (debug) / masks (release) for shift ≥ 128
Where: crates/media-clock/src/sync.rs:226-229
Code (verbatim):
```rust
        let weight = (1u128 << self.shift.min(126)).saturating_sub(1) as i128;
        let combined =
            i128::from(error_ns).saturating_add(i128::from(self.ema).saturating_mul(weight));
        self.ema = saturate_i64(combined >> self.shift);
```
What is wrong: the weight computation clamps `self.shift` to 126, but the final shift uses the raw `self.shift` (u32). `DriftEstimator::for_shift_and_window(u32::MAX, ..)` stores it unvalidated (112-124); the first `observe_sample` then evaluates `i128 >> huge` — panic "attempt to shift right with overflow" in debug, masked (shift % 128) garbage in release. The constructor doc (108-110) blesses zero shift but says nothing about the upper bound.
Why it matters: a public constructor accepts an input that crashes the library; the clamped weight shows the author considered the range but fixed only half the expression.
What correct looks like: clamp/validate `shift` once in `for_shift_and_window` (e.g. `shift.min(63)`), or `debug_assert!` + documented bound.
Confidence: HIGH — Verification: read (Rust shift semantics); not executed.
Production exposure: none found — all production constructors pass k=3 (`DriftEstimator::new()`, DEFAULT_EMA_SHIFT).

### F-003 [S4] Dead saturation branch with wrong sign on the impossible path
Where: crates/media-clock/src/sync.rs:152-155
Code (verbatim):
```rust
                    let slope = de
                        .checked_mul(1_000_000_000)
                        .map(|scaled| scaled / dt)
                        .unwrap_or(i64::MAX as i128);
```
What is wrong: `de` is the difference of two i64s (|de| ≤ 2^64), so `de·1e9` ≤ ~1.8e28, far below i128::MAX (~1.7e38) — `checked_mul` can never fail; the `unwrap_or(i64::MAX)` is dead. If it ever did fail with negative `de`, the fallback saturates to +MAX (wrong sign). Harmless today, misleading armor.
Confidence: HIGH — Verification: read + arithmetic.

### F-004 [S4] `samples` counter counts only windowed observations
Where: crates/media-clock/src/sync.rs:136-138 vs 128-130
Code (verbatim):
```rust
    pub fn observe_sample(&mut self, error_ns: i64) {
        self.ema_step(error_ns);
    }
```
vs
```rust
    pub fn observe_windowed(&mut self, now_ns: u64, error_ns: i64) {
        self.ema_step(error_ns);
        self.samples = self.samples.saturating_add(1);
```
What is wrong: the doc (197-199) says "Total windowed samples" — accurate — but a caller mixing cadences gets a counter that ignores every `observe_sample`; the video scheduler's per-present observations are invisible to it. Naming/doc could say "windowed-cadence observations only".
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DriftEstimator::new` | 103-106 | REAL | — | k=3, 2 s window defaults |
| `DriftEstimator::for_shift_and_window` | 111-124 | REAL | S3 | no shift bound validation (F-002) |
| `DriftEstimator::observe_sample` | 126-130 | REAL | — | video cadence; no samples++ (F-004) |
| `DriftEstimator::observe_windowed` | 132-163 | REAL | S2 | correct math; production consumer never armed (F-001) |
| `DriftEstimator::error_ema` | 165-169 | REAL | — | Option gating on primed |
| `DriftEstimator::slope_mppm` | 171-176 | REAL | — | |
| `DriftEstimator::slope_ppm` | 178-195 | REAL | — | hand-rolled ceiling div, correct for both signs |
| `DriftEstimator::samples` | 197-202 | REAL | S4 | windowed-only count (F-004) |
| `DriftEstimator::reset` | 204-214 | REAL | — | does not reset `samples` (documented as monotonic) |
| `DriftEstimator::ema_step` | 216-230 | REAL | S3 | shift-overflow corner (F-002) |
| `Default for DriftEstimator` | 233-237 | REAL | — | |
| `saturate_i64` | 239-243 | REAL | — | clamp, never wrap |
| `SyncBands::new` | 302-315 | REAL | — | u64→i64 saturate; dwell max(1) |
| `SyncBands::band`/`half_band_ns` | 317-327 | REAL | — | |
| `SyncBands::classify` | 329-381 | REAL | — | Schmitt + dwell; None decays streaks; i128 compares; reachable via PresentationScheduler (chain in header) |
| `SyncBands::reset` | 383-388 | REAL | — | |
| `SyncBands::clear_streaks` | 390-394 | REAL | — | |
| `RateWindow::new` | 415-423 | REAL | — | |
| `RateWindow::permits` | 425-445 | REAL | S2 | correct frozen/backwards semantics; production consumer never armed (F-001) |
| `RateWindow::reset`/`window_ns` | 447-456 | REAL | — | |
| tests (15) | 466-697 | REAL | — | see §5 |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ema_is_primed_by_the_first_sample_and_converges | YES (reasoned) | exact fixed-point assert |
| ema_is_bounded_by_the_input_range_for_any_sequence | YES (reasoned) | 4096 random samples + i64 extremes with in-[min,max] assert; would catch overflow/wrap/runaway; documents the +7 dyadic stall — unusually honest |
| ema_step_weight_matches_the_dyadic_formula | YES (reasoned) | hand-checked k=2 ladder (200/350/262) |
| windowed_slope_measures_an_exact_ramp_in_milli_ppm | YES (reasoned) | exact −1000 mppm / −1 ppm / 11 samples |
| frozen_source_never_elapses_the_slope_window | YES (reasoned) | 100 frozen samples ⇒ None |
| reset_restarts_ema_and_slope_regime | YES (reasoned) | |
| hold_band_absorbs_errors_within_the_half_band | YES (reasoned) | boundary value ±H itself held |
| entry_requires_dwell_sustained_beyond_the_half_band | YES (reasoned) | 7-vs-8 tick boundary + interruption reset + symmetric early side |
| exit_requires_half_amplitude_sustained_for_dwell | YES (reasoned) | hysteresis-gap persistence + 7-vs-8 exit boundary |
| none_samples_hold_the_band_and_decay_streaks | YES (reasoned) | |
| reset_returns_to_hold | YES (reasoned) | |
| rate_window_first_call_opens_and_permits_once | YES (reasoned) | exact boundary (999_999_999 vs 1_000_000_000) |
| rate_window_frozen_source_permits_exactly_one | YES (reasoned) | |
| rate_window_backwards_jump_holds_and_reset_reopens | YES (reasoned) | seek-jump semantics |

No tautologies, no mock-of-subject, none `#[ignore]` (verified by reading the file). Mutation run not possible (no cargo): conclusions reasoned; mark UNVERIFIED for runtime.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The measurement half of the 3.B1 audio-master drift loop" | sync.rs:4 | REAL-UNPROVEN | estimator/governor math exists and is correct; the loop's arming is absent in production (F-001) |
| Doc 03 §4.7(e): two drift absorbers, neither is the clock | sync.rs:4-8 | HALF-FALSE for the shipped build | video absorber (scheduler ladder) wired (pipeline_host.rs:1324); audio absorber never armed (F-001) |
| "Integer arithmetic throughout … no floats" | sync.rs:29-31 | REAL | verified: no float types in the module |
| "Bounded for any input … an i64 extreme sample can never overflow the EMA" | sync.rs:43-46 | REAL (except shift input, F-002) | convexity argument read-verified; shift ≥ 128 breaks a different bound |

## 7. Dead / duplicated / unreachable code

- `RateWindow` + `DriftEstimator::observe_windowed`: no shipped caller arms their consumer (governor). UNREACHABLE from shipped entry points (F-001).
- `DriftEstimator::for_shift_and_window`'s shift ≥ 128 region: unreachable in production (all call sites pass 3 or test values), panic if reached.

## 8. Dependencies & flags

- media-clock is L2 (media-types only, per module doc and boundaries.toml note at sync.rs:6-8). No feature gates on this file; it compiles into every build that links video-engine or audio-engine (W00: release build PASS).

## 9. Open questions for the next agent

- Does the desktop/web host arm the audio governor? rg says no caller of `arm_drift_governor` outside tests — the desktop-tauri and web hosts should be re-checked by their agents (apps/desktop/src/*, apps/web/src/host.rs).
- video-engine/scheduler.rs `decide_every` cadence and `SchedulerCfg::for_frame_duration` defaults (scheduler.rs:1300, pipeline_host.rs:1300) decide whether the video ladder ever fires in practice at 30 fps with a null-clock — if `snapshot.state != Running` it classifies `None` forever; with the F-001-class frozen clock in the CLI REPL (session.rs F-001), the ladder is fed `None` and stays Hold. Cross-check with W03's session.rs finding.
- The `2 s` slope window and `D = 8` dwell are `[PROPOSAL]` numbers (sync.rs:48-56, 304) — no measurement backs them; PERF-VS-VLC claims should be checked against that (wave 19).
