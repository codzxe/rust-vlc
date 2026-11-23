# AUDIT: crates/audio-engine/src/governor.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-16 |
| Lines / functions | 632 / 6 (new, correction, target_ns, stats, observe, reset) + 9 tests + Plant model |
| File verdict | UNREACHABLE (real, correct, tested code — but no shipped entry point ever arms it) |
| Reachable from shipped entry point? | NO — the only constructors are `AudioPipeline::with_drift_governor`/`arm_drift_governor` (pipeline.rs:144/155); in non-test code both are *definitions only* (rg over apps/ crates/ excluding tests: pipeline.rs:144,155 + passthrough apps/cli/src/pipeline_host.rs:639-640, zero call sites). The shipped CLI's per-quantum `observe_audio_drift` (pipeline_host.rs:1056-1061) → `AudioPipeline::observe_drift` (pipeline.rs:185-192) returns `DriftCorrection::ZERO` while unarmed. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n "with_drift_governor\|arm_drift_governor\|observe_drift" crates apps` (exit 0 — caller census); `rg -n "governor::tests" audit/runs/baseline/cargo-test-workspace.txt` (exit 0 — 9/9 ok, lines 63-78); `rg -n "audio_drift_stats\|drift_stats" apps crates` (exit 0 — test-only readers); sed quote verification |

## 1. One-paragraph verdict
The control law itself is genuinely good: a proportional-on-fill (integral-in-the-loop) correction derived via `DriftCorrection::for_divergence` (sign/clamp verified in media-clock/src/drift.rs:59-92), a ±250 ppm step limiter, frozen-time determinism, and nine serious closed-loop tests (S1/S2/S3/S5/S6 scenarios with convergence horizons and closed-form slope checks) that all pass in the baseline. But the hostile-focus claim from Wave 3 is **confirmed from this side: the governor is never armed in any production path**. `DriftGovernor::new` is only reachable through arming methods whose sole non-definition callers are tests (`audio-engine/tests/zero_alloc_resample.rs:73`, `apps/cli/tests/sync_scenarios.rs:93`); the shipped CLI pumps `observe_drift` every quantum into an unarmed pipeline that discards it (ZERO), and even the observability (`drift_stats`) is test-only. The module's own docs admit the production open path stays unarmed; docs/07-config-driven-system.md:58 nonetheless lists "A/V sync drift correction" as implemented **Code**. This file is a well-built actuator wired to nothing in the shipped product.

## 2. What actually works (verified)
— verified by `rg -n "governor::tests" audit/runs/baseline/cargo-test-workspace.txt` -> 9/9 ok (frozen_time_source, reset_bounds, s1, s2, s5, s3, step_limiter, unmeasurable, s6) — the closed-loop tests exercise `observe()` with a plant model and assert convergence bounds, limiter invariants, and closed-form residual slope; these would fail if the control law regressed.
— verified by `sed -n '138,157p' + '185,192p' crates/audio-engine/src/pipeline.rs` and `sed -n '613,641p' + '1053,1062p' apps/cli/src/pipeline_host.rs` -> the arming/observe plumbing is exactly as described (unarmed = ZERO no-op).

## 3. Findings
F-001 [S2]
Where: crates/audio-engine/src/governor.rs:176-218 (victim), crates/audio-engine/src/pipeline.rs:185-192, apps/cli/src/pipeline_host.rs:634-640
Code (pipeline.rs:185-190, verbatim):
```
    pub fn observe_drift(&mut self, now_ns: u64, fill_ns: Option<u64>) -> DriftCorrection {
        let Some(governor) = self.governor.as_mut() else {
            return DriftCorrection::ZERO;
        };
        let correction = governor.observe(now_ns, fill_ns);
```
and (pipeline_host.rs:634-640, verbatim):
```
    /// Arm the drift governor (the late-arming composition path —
    /// [`AudioLimb::observe_drift`]'s regime note): the production
    /// open path leaves the limb unarmed; a composition that paces
```
What is wrong: the entire 632-line control loop — the thing this module IS — has no shipped caller. `rg` over non-test code shows `arm_drift_governor`/`with_drift_governor` exist only as definitions/passthroughs; the production loop (`observe_audio_drift`, pipeline_host.rs:1056-1061) feeds an unarmed pipeline whose `observe_drift` short-circuits to ZERO. So in the shipped CLI/desktop build: no fill is ever measured into the estimator, no correction is ever derived, the micro-resampler never receives a non-zero `set_correction` from this path, and `DriftStats` stays all-zero. Why it matters: docs/07-config-driven-system.md:58 classifies "A/V sync drift correction" as **Code** (mechanism, implemented) — a reader concludes the shipped player actively corrects device drift; it does not (the null-sink first vertical cannot drift, and no real-device lane has landed). This independently re-confirms Wave 3's finding (audit/runs/W03-A09.log.md:47, W03-A10.log.md:102) from the audio-engine side. What correct looks like: either a composition arms the governor on real-device sessions, or the docs carry the honest "open-loop in the shipped build" caveat at the claim site. Confidence: HIGH — Verification: read + rg caller census (ran).
Note: governor.rs itself is honest — its header never claims production arming; the defect is the dead wiring + the config doc's classification.

F-002 [S3]
Where: crates/audio-engine/src/governor.rs:177-181
Code:
```
        let Some(fill) = fill_ns else {
            self.stats.unmeasurable = self.stats.unmeasurable.saturating_add(1);
            self.correction = DriftCorrection::ZERO;
            return self.correction;
        };
```
What is wrong: one unmeasurable fill sample slams the in-force correction from ANY value (±1000 ppm) to 0 in a single call — bypassing the ±250 ppm `STEP_LIMIT_PPM` that every other transition respects (reset()'s post-regime transient is bounded to one step, per the S7 criterion this module touts). A sink whose `latency()` intermittently returns None (a real cpal/ALSA hazard) would oscillate the correction hard (e.g. 600→0→600 each miss), driving audible pitch/length steps through the micro-resampler. The test `unmeasurable_sink_parks_at_zero` pins this deliberate-but-unbounded behavior. What correct looks like: hold the last correction (or decay it step-limited) while unmeasurable; park at zero only when measurement never returns. Confidence: HIGH — Verification: read; behavior pinned by baseline-passing test. Blast radius currently zero because of F-001 (never armed), which is why S3 not S2.

F-003 [S3]
Where: crates/audio-engine/src/governor.rs:60-71
Code:
```
/// The correction re-evaluation window `[PROPOSAL per W1-18 §3.2]`:
/// 1 s of injected device time — the ≤1 Hz bound.
pub const CORRECTION_WINDOW_NS: u64 = 1_000_000_000;
```
What is wrong: all three tuning constants (1 s window, 20 s `INTEGRAL_TIME_NS`, 250 ppm step) are `[PROPOSAL]` placeholders whose own doc says "the time constant is a Phase-3-measurable number" (governor.rs:28-29). Phase 3 has exited (audit/docs/gates/logs/phase3-exit-test.log exists) and no measurement against a real audio device ever replaced the guesses — the constants are validated only against the tests' own plant model, which bakes in the same numbers' plausibility. Untested control parameters for a loop that F-001 says never runs. What correct looks like: a measured calibration run on real hardware, or a doc downgrade from "[PROPOSAL]" to "unmeasured, first real-device session pending". Confidence: HIGH — Verification: read + rg (no calibration code/tests touching these constants outside this file).

F-004 [S4]
Where: crates/audio-engine/src/governor.rs:120, 190
Code:
```
        self.stats.slope_ppm_last = self.estimator.slope_ppm();
```
What is wrong: `slope_ppm_last` (and the whole `DriftStats` surface) has no production reader — `drift_stats`/`audio_drift_stats` passthroughs (pipeline_host.rs:671-672, 1068-1071) are called only by tests (rg census). Observability that nothing observes. What correct looks like: surface it in the CLI's engine-info/diagnostics output or drop it. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DriftGovernor::new` | 129-138 | REAL | — | fixed-size state, zero alloc |
| `correction` | 143-145 | REAL | — | actuator accessor |
| `target_ns` | 149-151 | REAL | — | |
| `stats` | 155-157 | REAL | — | test-only readers (F-004) |
| `observe` | 176-218 | REAL | S3 (F-002), S2 wiring (F-001) | control law verified: e→for_divergence(±1000 clamp)→step-limit; frozen window; i128-safe error; EMA fallback to raw on first window |
| `reset` | 226-232 | REAL | — | epoch restart, counted |
| test Plant/step | 252-284 | REAL | — | 30 Hz quantum model; closed-form slope checks |
| 9 tests | 286-631 | REAL | — | all assert convergence/limiter invariants; 9/9 baseline ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| s1/s2 (±100 ppm converge) | YES | 45 s convergence horizon, fill bounds (0 < fill ≤ 150 ms), clamped==0 pre-disturbance, correction band asserts |
| s3 (at/beyond bound) | YES | step-limit per window asserted every quantum; residual slope equals closed-form 4 ms/s ±5% |
| s5 jitter | YES | xorshift ±5 ms noise, correction must stay ≤150 ppm — fails if EMA leaks noise into the law |
| s6 (other geometry) | YES | 48 kHz/1024-frame quantum; convergence not assumed (asserts converged ≥ 1 s) |
| frozen_time_source | YES | 1000 observes at constant now → updates unchanged |
| reset_bounds | YES | post-reset first observe ≤ +250 ppm |
| step_limiter | YES | −1000 reached exactly on the 4th window and held |
| unmeasurable | YES | pins the park-at-zero behavior (including its unboundedness — F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A/V sync drift correction | **Code**" | docs/07-config-driven-system.md:58 | SHALLOW (mechanism exists, never activated in shipped builds) | caller census above: no production arm site; observe path returns ZERO unarmed (pipeline.rs:186-188) |
| "the production open path leaves the limb unarmed" | pipeline_host.rs:635-637 doc comment | REAL (honest admission) | same census — the comment is accurate |
| "consumer #2 of DriftCorrection ... the component that measures and derives" | governor.rs:1-5 | REAL as code, UNREACHABLE in product | baseline tests prove behavior; no armed production path |
| Wave 3: "the drift governor is never armed in production" | audit/runs/W03-A09.log.md:47 | CONFIRMED from audio-engine side | this audit, F-001 |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (F-001): reachable only from tests. Within the module, no dead code; `slope_ppm_last` is write-only in practice (F-004).

## 8. Dependencies & flags
`media_clock::sync::{DriftEstimator, RateWindow}` + `media_clock::{DriftCorrection, MAX_CORRECTION_PPM}` — all used. No feature gates. No unsafe. No allocation on the observe path (D-1 honored — fixed-size state).

## 9. Open questions for the next agent
- Who owns the composition that should arm the governor when a real (non-null) audio device lands? If nobody, F-001 stays a permanent S2 doc-vs-product gap.
- Consider whether F-002's unbounded park-at-zero should be step-limited before that arming lands — cheap to fix now, audible later.
