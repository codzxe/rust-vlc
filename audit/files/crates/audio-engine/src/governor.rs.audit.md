# AUDIT: crates/audio-engine/src/governor.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 633 / 21 (6 prod fns, 3 consts, 2 structs, 10 test fns incl. Plant) |
| File verdict | UNREACHABLE (real, correct-looking control code; no shipped entry point ever constructs it) |
| Reachable from shipped entry point? | NO — only `AudioPipeline::arm_drift_governor`/`with_drift_governor` construct a `DriftGovernor`; their only callers are tests (`apps/cli/tests/sync_scenarios.rs:93`, `crates/audio-engine/src/pipeline.rs:813` test, `crates/audio-engine/tests/zero_alloc_resample.rs:73`). The shipped CLI calls `observe_audio_drift` (apps/cli/src/host.rs:618,735) on an **unarmed** pipeline → no-op. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "DriftGovernor\|governor" --glob *.rs` (callers) · `rg -n "with_drift_governor\|arm_drift_governor"` (arming sites: tests only) · `sed -n` quotes below · read media-clock/src/drift.rs:30-109 + sync.rs:60-180,400-457 (math verification) · `rg -n "drift\|resampl" docs/feature-matrix.toml` (row afeat-resampler = not_started on all 4 platforms) |

## 1. One-paragraph verdict
The control law itself is genuine and carefully built: a proportional-on-fill (integral-in-the-loop) controller with a ±1000 ppm limiter (inherited from `DriftCorrection::new`), a ±250 ppm/step limiter, an EMA-prefiltered error, saturating integer arithmetic throughout, and a 9-test suite that models the plant and asserts convergence, bounds, frozen-source determinism, and adversarial behavior beyond the limiter. I verified the sign algebra (`for_divergence(e,W_c) = −e/W_c·1e6` ppm → steady-state offset −W_c·δ) and the test Plant's integrator (`delta = ppm·quantum/1e6`) — both correct. But none of it runs in the shipped build: no production code arms the governor, the shipped audio device is the null device, and the module's own sibling doc admits the production pump does not arm it yet. 633 lines of dead-in-production control code with a 380-line test suite pinning behavior nothing user-visible exercises.

## 2. What actually works (verified)
— Control-law math verified by hand against media-clock/src/drift.rs:78-102 (`ppm = −(divergence/window)·1e6`, saturating, zero-window → ZERO) and sync.rs:430-445 (`RateWindow::permits`: first call opens+permits, frozen source never re-permits, backwards jumps hold). Step limiter lines 207-214 correctly bounds approach; `observe`'s i128 subtraction (186-187) cannot overflow.
— Unit tests ran in baseline: `cargo test --workspace --locked` 1681 passed / 1 failed / 24 ignored (audit/runs/baseline/INDEX.md #3); the one failure is unrelated (desktop zero-alloc). These governor tests are part of the 1681. Not re-run per §HARD-8.

## 3. Findings
### F-001 [S3] Governor never armed in any shipped build — dead code in production
Where: crates/audio-engine/src/governor.rs:75-90 (whole type), with proof at apps/cli/src/pipeline_host.rs:600-609
Code:
```
    /// makes the fill a *decode-ahead depth* metric instead; arming
    /// the governor there would "correct" buffering as if it were
    /// clock drift. The governor is therefore armed by compositions
    /// that pace (the S8 convergence harness, and the production pump
    /// once 3.B3's backpressure lands steady-state pacing).
```
What is wrong: `DriftGovernor::new` is reachable only via `AudioPipeline::arm_drift_governor`/`with_drift_governor` (crates/audio-engine/src/pipeline.rs:144-156), whose only callers are three test files (rg output above). The shipped `lumen` open/play path constructs no governor; `host.rs:618/735` `observe_audio_drift()` feeds an unarmed pipeline (`pipeline.rs:186-189`: `let Some(governor) = self.governor.as_mut() else { … ZERO }`).
Why it matters: the entire drift-correction actuator chain (governor → `MicroResampler::set_correction`, resample.rs:124) is inert in every shipped binary; A/V drift on real audio output is uncorrected, and 633 lines of code + tests advertise a capability the product does not exercise. Docs are internally honest (feature-matrix row `afeat-resampler` = not_started ×4), so this is dead code, not fraud.
What correct looks like: arm the governor on the production pump once 3.B3 pacing lands (as the doc itself promises), or gate the module behind the not-yet-existing pacing composition so the dead state is explicit.
Confidence: HIGH — Verification: read + rg caller trace (all callers listed above); baseline INDEX (audio is null-device only, `audio.null-device` feature).

### F-002 [S3] Test plant masks negative fill via `unsigned_abs` — "zero underruns" assertions cannot see underruns
Where: crates/audio-engine/src/governor.rs:274
Code:
```
            let correction = governor.observe(self.now_ns, Some(self.fill_ns.unsigned_abs()));
```
What is wrong: `fill_ns` is `i64`; a control-law regression that drives the ring negative passes `|fill|` to the governor, and the s1/s6 underrun asserts (`g.stats().fill_ns_min.must() > 0`, lines 368/565) inspect the same masked value — a −25 ms excursion reads as +25 ms and passes both the underrun and (≤150 ms) flood asserts. Only s3's second half asserts the raw `plant.fill_ns > 0` (line 463).
Why it matters: the suite's headline claim "Zero underruns (fill > 0 always)" (line 308) is not actually enforced for s1/s2/s6; a sign-flip or gain regression that briefly empties the modeled ring could still converge inside the error band later and pass.
What correct looks like: pass `Some(self.fill_ns.max(0) as u64)` and assert `plant.fill_ns >= 0` directly in every scenario (or model fill as u64 with an explicit underflow flag).
Confidence: HIGH — Verification: read (mutation reasoning: flip `for_divergence` sign → fill runs away negative; s1's min/max asserts see positive values until |fill| > 150 ms, which takes ~30 s at −5 ms/s — inside the 120 s window it would eventually trip the flood assert, but a brief dip near t≈0 would not).

### F-003 [S4] `clamped` counter cannot distinguish saturation from a legitimate exactly-±1000 ppm desire
Where: crates/audio-engine/src/governor.rs:204-206
Code:
```
            let desired = DriftCorrection::for_divergence(i128::from(filtered), INTEGRAL_TIME_NS);
            if desired.ppm().abs() >= MAX_CORRECTION_PPM {
                self.stats.clamped = self.stats.clamped.saturating_add(1);
```
What is wrong: `for_divergence` already saturates (drift.rs:59-63), so `desired.ppm()` is always ≤1000 in magnitude; `>= 1000` fires both for a true desire of 5000 ppm and for a legitimate desire of exactly 1000 ppm (e.g. steady state at δ = 1000: e = −20 ms exactly → desired = +1000 exactly). The counter's doc ("Times the desired correction hit the ±1000 ppm limiter", line 102-105) over-promises.
Why it matters: observability only (stats are polled, never control inputs), but the number this exports is ambiguous at exactly the boundary it exists to report.
What correct looks like: compute the unclamped ppm (i128 division) before `DriftCorrection::new`, and count `raw > MAX_CORRECTION_PPM`.
Confidence: HIGH — Verification: read; the s3 test (line 445: `assert_eq!(g.stats().clamped, 0, "δ at the bound never trips the clamp")`) only passes because convergence is asymptotic and never lands exactly on −20 ms.
### F-004 [S4] Plant comment misstates the unit (ppm vs per-mille)
Where: crates/audio-engine/src/governor.rs:276-277
Code:
```
            // ns of fill change per ns of time = rate_delta × 1e-3.
            let delta = (i128::from(rate_delta_ppm) * i128::from(self.quantum_ns)) / 1_000_000;
```
What is wrong: comment says ×1e-3 (per-mille); the code divides by 1e6 (ppm) — the code is right, the comment is wrong by 1000×.
Why it matters: comment-only, but this is the one line a future maintainer will trust when re-deriving the plant; a wrong unit note in a control-loop test invites a "fix" that breaks the model.
What correct looks like: `ns per ns = rate_delta_ppm × 1e-6`.
Confidence: HIGH — Verification: read (arithmetic re-derived by hand).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const CORRECTION_WINDOW_NS | 60-62 | REAL (test-only use) | — | 1 s; used by RateWindow::new |
| const INTEGRAL_TIME_NS | 64-67 | REAL (test-only use) | — | 20 s W_c |
| const STEP_LIMIT_PPM | 69-71 | REAL | — | 250 ppm/update |
| struct DriftGovernor (+fields) | 74-90 | REAL / UNREACHABLE-prod | F-001 | fixed-size, no alloc |
| struct DriftStats (+11 fields) | 92-121 | REAL | — | polled counters, all saturating |
| DriftGovernor::new | 128-138 | REAL | — | trivial ctor |
| DriftGovernor::correction | 142-145 | REAL | — | copy out |
| DriftGovernor::target_ns | 147-151 | REAL | — | getter; no production reader of value |
| DriftGovernor::stats | 153-157 | REAL | — | copy out |
| DriftGovernor::observe | 176-218 | REAL | F-003 | None→park ZERO (179-180, does not touch window/estimator — resume evaluates on stale EMA, bounded by step limiter); i128 error math safe (186-187); window/step logic correct (192-216) |
| DriftGovernor::reset | 226-232 | REAL | — | zero + window/estimator reset; `opened=false` redundant with `window.reset()` but consistent |
| tests::governor (helper) | 243-245 | REAL | — | |
| tests::Plant::new | 262-269 | REAL | — | |
| tests::Plant::step | 273-283 | REAL | F-002, F-004 | integrator math correct; unsigned_abs masks sign |
| unmeasurable_sink_parks_at_zero | 286-299 | REAL | — | would fail if park-at-zero regressed (asserts ZERO + counter) |
| s1_fixed_plus_100ppm… | 301-374 | REAL | F-002 caveat | strong: convergence band, ±1000 bound per-quantum, clamp-free pre-disturbance, flood bound; underrun assert weakened by F-002 |
| s2_fixed_minus_100ppm… | 376-404 | REAL | — | symmetric; correction within ±25 ppm of −100 |
| s3_at_the_bound_and_adversarially_beyond | 406-493 | REAL | — | strongest test: at-limit stability, step bound per update, adversarial 5000 ppm → pinned at +1000, residual slope closed-form check 4 ms/s ± 200 µs/s |
| s5_fill_jitter_does_not_amplify | 495-522 | REAL | — | ±5 ms noise → correction ≤ ±150 ppm; xorshift PRNG deterministic |
| s6_rate_composition… | 524-568 | REAL | — | 48 kHz/1024-frame geometry; asserts convergence is walked back, not assumed (≥1 s) |
| frozen_time_source_never_re_evaluates | 570-589 | REAL | — | pins the frozen-source property (1000 frozen observes, zero updates) |
| reset_bounds_the_post_regime_transient | 591-606 | REAL | — | post-reset ≤ +250 ppm |
| step_limiter_is_counted_and_enforced | 608-631 | REAL | — | −1000 reached in 4 steps, held; step_limited ≥ 3 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unmeasurable_sink_parks_at_zero | YES | asserts ZERO + unmeasurable==1 after a primed non-zero correction |
| s1 | PARTIALLY | sign-flip in for_divergence → runaway → flood assert (150 ms) trips within the 120 s window; but a brief negative dip early is masked by F-002 |
| s2 | YES | correction within ±25 ppm of −100 requires correct sign+gain |
| s3 | YES | closed-form residual slope (4 ms/s) + pinned-at-+1000 are quantitative; raw `plant.fill_ns > 0` checked directly |
| s5 | YES | jitter amplification would exceed ±150 ppm if EMA were bypassed or gain too high |
| s6 | YES | different geometry converging to same band catches quantum/rate coupling errors |
| frozen_time_source… | YES | any wind-down or time-driven update breaks updates==before |
| reset_bounds… | YES | >250 ppm on first post-reset observe fails |
| step_limiter… | YES | requires exact −250/window cadence to bound and hold |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "afeat-resampler … status not_started ×4 platforms" | docs/feature-matrix.toml:1723-1732 | REAL (honest: matrix admits not started) | rg callers: no production arming |
| Module doc: "Unmeasurable latency parks at ZERO — honest degradation" | governor.rs:47-49 | REAL | code 179-180; test 286-299 |
| Module doc: consumer #2 of DriftCorrection, "derives a bounded correction" | governor.rs:1-5 | REAL-code / UNREACHABLE-prod | F-001 |
| Module doc: "no wind-up state exists" | governor.rs:31-33 | REAL | observe is pure fn of filtered error + step clamp (192-216); verified no accumulator |
| W1-18 constants are [PROPOSAL] | governor.rs:60-71 | REAL (honest labeling) | consts marked PROPOSAL |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (F-001): constructed only under `#[cfg(test)]` callers. `DriftStats.slope_ppm_last` (line 120) is written every measurable observe but has no production reader (polled only via `PipelineHost::drift_stats`, itself called only from tests/host diagnostics — rg `drift_stats` shows pipeline_host.rs:1068 pub fn, no CLI command reads it).

## 8. Dependencies & flags
Depends only on media-clock (workspace path) + testkit (dev-only). No feature flags gate the module — it compiles into every build including the shipped one, inflating binary with dead code. No unsafe, no allocation, no panics (all indexing-free, saturating arithmetic — verified line by line).

## 9. Open questions for the next agent
- crates/audio-engine/src/pipeline.rs (other agent): does `observe_drift` on an unarmed pipeline still tick the sink sample counter read (u128 division per quantum — cost on the render hot path when unarmed)?
- resample.rs (other agent): `MicroResampler::set_correction` — is the ppm→ratio math exact-rational (claim at drift.rs:104-109) and does the zero-correction path short-circuit cleanly?
- Who eventually reads `drift_stats()` — if nothing, the DriftStats plumbing through pipeline/pipeline_host (pipeline_host.rs:1068) is also dead.
