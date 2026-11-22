# AUDIT: crates/audio-engine/src/governor.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 633 / 3 consts, DriftStats (11 fields), DriftGovernor::{new, correction, target_ns, stats, observe, reset} + 9 tests (235-633, 63% of file) |
| File verdict | **REAL code, UNREACHABLE in the shipped build** — nothing in any shipped entry point constructs a `DriftGovernor`; the correction is permanently `ZERO` in the product |
| Reachable from shipped entry point? | NO. Chain that exists: `lumen` pump → `apps/cli/src/host.rs:618/735` `guard.observe_audio_drift()` → `pipeline_host.rs:613` `AudioLimb::observe_drift` → `audio-engine/src/pipeline.rs:185-189` `observe_drift` → **`governor: None` early-return → `DriftCorrection::ZERO`**. The only constructor path is `AudioPipeline::arm_drift_governor` (pipeline.rs:155-157), whose only callers are tests (apps/cli/tests/sync_scenarios.rs:93, pipeline.rs's own tests, zero_alloc_resample.rs:73). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | rg (`DriftGovernor|arm_drift_governor|with_drift_governor|observe_drift|reset_drift|observe_audio_drift` tree-wide); sed reads of pipeline.rs:60-230, pipeline_host.rs:595-645/1050-1115, host.rs:600-640; docs/gates grep for 3.B3. No cargo (static). |

## 1. One-paragraph verdict

The control law is genuine and well-tested: a proportional-on-fill (integral-in-the-loop) controller with a ±1000 ppm saturation limiter, a ±250 ppm/window step limiter, EMA-filtered error input, injected-time windows with the frozen-source property, and scenario tests (S1/S2/S3/S5/S6 + closed-form residual-slope checks) that would fail under real mutations of the law. But it governs nothing in the shipped product: the only production caller chain (`observe_audio_drift` per rendered quantum, called in the real pump at host.rs:618/735) terminates in `AudioPipeline::observe_drift`'s `governor: None` early-return — no shipped code ever calls `arm_drift_governor`. The CLI's own comment admits it ("an unarmed governor ignores it", host.rs:732-733), and the arming deferral ("the production pump once 3.B3's backpressure lands steady-state pacing", pipeline_host.rs:635-637) has expired: docs/gates/phase3-exit.md:107 records 3.B3 queue backpressure **COMPLETE**, and the wall-clock paced pump (`pump_wallclock`, host.rs:523) exists — yet the arming never landed. 632 lines of dead-in-product control engineering, plus a per-quantum no-op call burn in the hot loop.

## 2. What actually works (verified)

- The math, by reading: error = fill − target in clamped i128→i64 (governor.rs:186-188); desired = `DriftCorrection::for_divergence(filtered_ema, 20 s)`; step-limited to ±250 ppm around the correction in force (207-214); unmeasurable sinks park at ZERO and count (177-181); frozen injected time ⇒ zero updates (197 `window.permits` gate + test at 571-589).
- The 9 tests are scenario simulations with a plant model (252-284) that integrates `(correction − drift)` per quantum — mutation-sensitive (see §5).
- W03-A01's independent audit of pipeline.rs and W03-A02's audit of sync.rs reach the same reachability conclusion (audit/files/crates/audio-engine/src/pipeline.rs.audit.md:22, audit/files/crates/media-clock/src/sync.rs.audit.md F-001) — three-agent consensus.
- Historical gate logs (docs/gates/logs/phase3-exit-workspace-test.log:138-146) show all 9 governor tests passing in a real recorded run.

## 3. Findings

### F-001 [S2] — The governor is never constructed in any shipped build; audio-drift absorption is permanently ZERO
Where: crates/audio-engine/src/governor.rs:1-5 (module premise), pipeline.rs:155-157 + pipeline_host.rs:639-641 (the un-called arming), host.rs:731-733 (the admission)
Code (governor.rs:1-5):
```rust
//! The audio-side drift governor (card 3.B1; W1-18 §3.2) —
//! **consumer #2 of [`DriftCorrection`]**, the derivation site audit
//! M12 named absent: the component that *measures* the device-vs-
//! nominal divergence and *derives* a bounded correction from it via
//! [`DriftCorrection::for_divergence`].
```
Code (apps/cli/src/host.rs:728-733):
```rust
                    // Card 3.B1's loop closure (W1-18 §7 step 8): one
                    // drift observation per rendered quantum — the
                    // device just consumed, so the fill this sample
                    // sees is the steady-state residue. Counters only,
                    // never events; an unarmed governor ignores it.
                    guard.observe_audio_drift();
```
What is wrong: rg across the tree: `arm_drift_governor`/`with_drift_governor` callers are exclusively tests (apps/cli/tests/sync_scenarios.rs:93; pipeline.rs:813 internal test; zero_alloc_resample.rs:73). The shipped pump dutifully calls `observe_audio_drift` every rendered quantum, which flows into `pipeline.rs:186-189`:
```rust
        let Some(governor) = self.governor.as_mut() else {
            return DriftCorrection::ZERO;
        };
```
so the per-quantum call is a no-op and `DriftGovernor::observe` never executes in the product. The doc's stated precondition for arming ("the production pump once 3.B3's backpressure lands steady-state pacing", pipeline_host.rs:635-637) is satisfied on paper — docs/gates/phase3-exit.md:107: "3.B3 | queue backpressure | **COMPLETE**" and the wall-clock pump exists (host.rs:523 `pump_wallclock`) — but the arming never followed.
Why it matters: this is the audio axis of A/V drift absorption (doc 03 §4.7 e, card 3.B1). In the shipped product the micro-resampler ratio is permanently 1.0 and device-vs-nominal divergence is never corrected. It also burns a per-quantum call to a function whose meaningful branch is dead. The module's self-description ("consumer #2 of DriftCorrection") is true of the type graph and false of the runtime.
What correct looks like: the wall-clock pump's composition arms the governor where it paces (one `arm_drift_governor(DRIFT_TARGET_FILL_NS)` call in the production open path), or the module doc states plainly "no shipped composition arms this yet".
Confidence: HIGH — Verification: read (full caller trace above; three-agent consensus).
Severity: S2 not S3 because it is missing product behavior in the realistic case (real device clock divergence is the normal condition), matching W03-A02's calibration of the twin finding.

### F-002 [S3] — A single unmeasurable fill sample hard-zeros the correction instead of holding it
Where: crates/audio-engine/src/governor.rs:176-181
Code:
```rust
    pub fn observe(&mut self, now_ns: u64, fill_ns: Option<u64>) -> DriftCorrection {
        let Some(fill) = fill_ns else {
            self.stats.unmeasurable = self.stats.unmeasurable.saturating_add(1);
            self.correction = DriftCorrection::ZERO;
            return self.correction;
        };
```
What is wrong: one `latency() == None` answer — a transient measurement glitch — discards the in-force correction and returns to ZERO. On the next measurable sample the walk-back is step-limited (±250 ppm/window), so a flapping sink (measurable/unmeasurable alternating) oscillates the resampler ratio between ZERO and the partially-walked value instead of holding the last good correction. The module doc frames this as "honest degradation" against fabricated estimates, but holding the last derived value is equally honest and non-perturbing.
Why it matters: dormant today (F-001), but it is a latent control-quality defect on the exact path F-001's fix would activate.
What correct looks like: hold the last correction on `None`; zero it only after N consecutive unmeasurable samples (and count that).
Confidence: MEDIUM — Verification: read; behavior under flap is reasoned, not executed.

### F-003 [S4] — Module doc's acceptance-band arithmetic is presented as tested but the band is a plant assumption
Where: crates/audio-engine/src/governor.rs:17-20
Code:
```rust
//! ring at `δ` µs/s until underrun or `audio.ring_full`. Because the
//! plant *integrates* rate error, a proportional-on-position
//! controller is integral-in-the-loop: the correction that holds the
//! fill steady is exactly the device's rate error, and the fill error
//! settles at the small proportional offset `−W_c · δ` (2 ms at
```
What is wrong: nothing in the code — the note is that the "design's acceptance band `[10 ms, 150 ms]`" is asserted in prose and hardcoded in test bounds (368-373: `fill_ns_max <= 150_000_000`), with no named design source beyond W1-18 §3.2, and no cross-check against the actual ring's capacity (the "3 s ring", pipeline_host.rs:97). The band and the ring never meet in one test.
Why it matters: documentation-grade parameter provenance; if the ring size changes the band silently stops meaning anything.
What correct looks like: a test tying band to ring capacity, or a doc pointer to the ring geometry.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts CORRECTION_WINDOW_NS / INTEGRAL_TIME_NS / STEP_LIMIT_PPM | 62-71 | REAL-UNPROVEN | — | `[PROPOSAL]`-tagged honestly; invariants (±1000 limiter) live in drift.rs |
| DriftStats | 93-121 | REAL-UNPROVEN | — | 11 counters, polled-only; Copy/Default; used by pipeline_host for CLI stats |
| DriftGovernor::new | 129-138 | UNREACHABLE | S2 | no production caller (F-001) |
| DriftGovernor::correction / target_ns / stats | 143-157 | UNREACHABLE | — | accessors; stats surfaced through a real chain (`audio_drift_stats`, pipeline_host.rs:1068) but always all-zero in product |
| DriftGovernor::observe | 176-218 | UNREACHABLE | S2/S3 | the control law; clamped i128 error; EMA input with raw-error fallback; step limiter correct; F-002 zero-on-glitch |
| DriftGovernor::reset | 226-232 | UNREACHABLE | — | zero + window/estimator reset + counter; called via reset_drift at every seek boundary in production (host.rs:1106) — but resetting a governor that never existed (governor None ⇒ only `set_drift(ZERO)`) |

## 5. Test honesty (9 tests, lines 235-633)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unmeasurable_sink_parks_at_zero (287) | YES | primes nonzero correction first (90 quantas @600 ppm), then asserts ZERO + counter |
| s1_fixed_plus_100ppm_device_converges_within_45s (302) | YES | 120 s plant sim; disturbance at 60 s; return-band, stay-in-band, underrun/flood, clamp-before-disturbance, correction range 50..350 — a real scenario test |
| s2_fixed_minus_100ppm_device_converges_without_flooding (377) | YES | symmetric flooding direction; ±25 ppm convergence |
| s3_at_the_bound_and_adversarially_beyond (407) | YES | at-limit stability + monotone step + adversarial 5000 ppm with a closed-form residual slope equality (±200k ns/s of 4 ms/s) — the strongest test |
| s5_fill_jitter_does_not_amplify (496) | YES | xorshift ±5 ms noise; correction ≤150 ppm — kills EMA-removal mutations |
| s6_rate_composition_converges_at_other_geometry (525) | YES | 48 kHz/1024 geometry; displacement walked back (converged ≥1 s) |
| frozen_time_source_never_re_evaluates (571) | YES | 1000 frozen observes, zero updates, correction held |
| reset_bounds_the_post_regime_transient (592) | YES | post-reset first observe ≤ +250 ppm |
| step_limiter_is_counted_and_enforced (609) | YES | −7500 ppm desire walked −250/window to the bound and held; step_limited ≥3 |

All UNVERIFIED by execution here; docs/gates/logs/phase3-exit-workspace-test.log:138-146 records all 9 passing in a historical real run. The tests exercise the plant model, not a real device — appropriate for a control loop; the plant's integrator is the standard first-order model.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "consumer #2 of DriftCorrection … the derivation site" | governor.rs:1-5 | UNREACHABLE | no shipped constructor (F-001) |
| "the wiring applies it via set_correction on the pipeline's resampler" | governor.rs:160-161 | UNREACHABLE | pipeline.rs:185-192 does apply it — but only when armed; never in product |
| arming deferred "until 3.B3's backpressure lands steady-state pacing" | pipeline_host.rs:635-637 | FALSE (stale) | 3.B3 COMPLETE per docs/gates/phase3-exit.md:107 + wall-clock pump exists; arming still absent |
| "Everything here is counters, polled through DriftStats … nothing touches playback.error" | governor.rs:51-55 | REAL | read: no event emission anywhere in the file |

## 7. Dead / duplicated / unreachable code

- The entire `DriftGovernor` runtime (75-233) is unreachable from shipped entry points (F-001). Not duplicated — the sibling video-side sync ladder (media-clock/sync.rs → video-engine scheduler) IS wired (per W03-A02's audit), so only the audio half is dead.

## 8. Dependencies & flags

- Depends on `media_clock::sync::{DriftEstimator, RateWindow}` and `media_clock::{DriftCorrection, MAX_CORRECTION_PPM}` — both audited by W03-A02 (sync.rs REAL; drift.rs is this agent's next-but-one file).
- No feature flags; no unsafe; no allocation in the observe path (D-1 honored — integer state only).

## 9. Open questions for the next agent

- drift.rs (mine, next files): confirm `DriftCorrection::new` saturates to ±MAX (the governor's "by construction" premise) and `for_divergence`'s sign convention matches the tests' plant model.
- Whoever owns the CLI composition root: F-001's fix is a one-line arming; the wave-19 claim ledger should mark card 3.B1's audio half UNREACHABLE-in-product.
