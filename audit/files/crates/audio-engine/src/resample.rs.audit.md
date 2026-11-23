# AUDIT: crates/audio-engine/src/resample.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-23 |
| Lines / functions | 698 / 6 prod fns + 1 macro; 12 tests + 4-fn in-test reference impl |
| File verdict | REAL (code implemented, unit + zero-alloc tests executed, pass) — but `process()` is UNREACHABLE from the shipped build (see F-001) |
| Reachable from shipped entry point? | PARTIAL: constructor YES (apps/cli/src/pipeline_host.rs:1600 `AudioPipeline::new` -> crates/audio-engine/src/pipeline.rs:130 `MicroResampler::new`); `process()` NO — no shipped path ever sets a non-zero drift (proof in F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `cargo test -p audio-engine --lib resample --offline` -> 12 passed / 0 failed; `cargo test -p audio-engine --test zero_alloc_resample --offline` -> 1 passed; sed verifications (all quotes below) |

## 1. One-paragraph verdict
This is a genuine, carefully engineered *linear* micro-resampler restricted to drift correction (±1000 ppm, i.e. ratio ∈ [0.999, 1.001]), with exact i128/i64 rational phase arithmetic, no floats on the control path, no unsafe, and an honest in-file differential test against the former single-loop kernel — all 12 unit tests and the zero-alloc test pass (I ran them). It is NOT a general resampler and does not pretend to be: it cannot do 44.1→48 (ratio 1.088) and its own doc says so (lines 47-54); the feature-matrix `afeat-resampler` row is honestly `not_started` (docs/feature-matrix.toml:1730) and PERF-VS-VLC makes no Lumen-side resample-quality claim (its only "resampler" mention is characterizing VLC's float aout chain, PERF-VS-VLC.md:116-118). The two real problems: (a) the shipped CLI never arms the drift governor, so `MicroResampler::process` is dead code in the release lane — the "consumer #1 of DriftCorrection / audit M12 closed" story is type-level wiring only; (b) the module doc's "no panics … a geometry mismatch produces no output" claim is false for a frame-count mismatch (plane shorter than `in_frames` panics at lines 255/292).

## 2. What actually works (verified)
— `cargo test -p audio-engine --lib resample --offline` -> `test result: ok. 12 passed; 0 failed; 0 ignored` (incl. `restructured_kernel_is_bit_identical_to_the_former_single_loop`, `ratio_is_exact_over_long_streams`, `out_never_exceeds_the_design_bound`).
— `cargo test -p audio-engine --test zero_alloc_resample --offline` -> `drift_corrected_quantum_path_is_zero_allocation_after_warm_up ... ok`.
— Phase math verified by analysis: `out = in × (1e6+ppm)/1e6` exactly; at ppm=−1000 each `advance!` adds ≤2 to `cursor` (frac < 999000, +1e6 < 2×999000), so the exit cursor ∈ {last_abs, last_abs+1} = next {base−1, base} — the prefix/body/exact-tail split is equivalent to the reference loop; `DriftCorrection::new` clamps ±1000 (drift.rs:60-62) so `step_den ∈ [999000, 1001000]`: no division by zero, no renormalize infinite loop.

## 3. Findings
F-001 [S2]
Where: crates/audio-engine/src/resample.rs:1-3, 191-296; apps/cli/src/pipeline_host.rs:634-638
Code (resample.rs:1-3):
```
//! The bounded micro-resampler (card 3.B1; doc 03 §4.7 e / §4.8 e-f) —
//! **consumer #1 of [`DriftCorrection`]**, the piece audit M12 named
//! absent ("DriftCorrection ±1000ppm limiter: ZERO consumers").
```
Code (apps/cli/src/pipeline_host.rs:634-638):
```
    /// Arm the drift governor (the late-arming composition path —
    /// [`AudioLimb::observe_drift`]'s regime note): the production
    /// open path leaves the limb unarmed; a composition that paces
```
What is wrong: `MicroResampler::process` (resample.rs:191) is never called in the shipped build. Call-chain proof: the only `mix()` call is pipeline.rs:348-360, which routes through the resampler only `if self.drift.ppm() != 0`; `drift` becomes non-zero only via `AudioPipeline::set_drift`, called only from `observe_drift` (pipeline.rs:185-191) — which returns `DriftCorrection::ZERO` before touching `set_drift` unless a governor is armed; `arm_drift_governor` is called nowhere in apps/cli/src (only apps/cli/tests/sync_scenarios.rs:93 and audio-engine's own tests). The code itself admits it ("the production open path leaves the limb unarmed"). Why it matters: the headline "audit M12 closed — consumer #1 of DriftCorrection" is dormant wiring; in the shipped CLI the resampler never resamples anything, and device-clock drift is never corrected. Blast radius is capped by the fact the shipped CLI's only sink is the null device (apps/cli/src/host.rs:79), where real device drift does not exist. What correct looks like: either arm the governor on the production pump path or state the dormant status in this module's doc instead of the M12-closed framing. Confidence: HIGH — Verification: read (call-chain rg + sed, quoted).
F-002 [S3]
Where: crates/audio-engine/src/resample.rs:42-45 vs 255, 292
Code (42-45):
```
//! - No `unsafe`, no floats on the control path, no panics: a
//!   zero-frame quantum is a no-op; a geometry mismatch produces no
//!   output (the pipeline is the only caller and guarantees the
//!   shape — the guard is defensive, not reachable).
```
Code (291-293):
```
        for (c, slot) in self.tail.iter_mut().enumerate() {
            *slot = input[c][in_frames - 1];
        }
```
What is wrong: "a geometry mismatch produces no output" is true only for the channel-count mismatch (`input.len() != self.channels`, line 192). A frame-count mismatch — a plane shorter than `in_frames` — panics: line 292 `input[c][in_frames - 1]` (index out of bounds) or line 255 `&in_plane[idx..idx + 2]` (slice out of range) or prefix line 242 `input[c][0]` on an empty plane. The doc's "no panics" claim is therefore false as stated. Latent today: the sole caller (pipeline.rs:358) passes mixer-built planes of exactly `frames` length. What correct looks like: guard `input[c].len() >= in_frames` in the line-192 check, or delete the "no panics / geometry mismatch" sentence. Confidence: HIGH — Verification: read (index semantics; no run — would need a mismatched-plane call no caller makes).
F-003 [S3]
Where: crates/audio-engine/src/resample.rs:213, 238-247; crates/media-clock/src/drift.rs:31-34
Code (resample.rs:213):
```
        let cap = in_frames + in_frames / 1000 + 1;
```
Code (drift.rs:31-34):
```
/// The maximum drift-correction magnitude, in parts per million of the
/// nominal rate (doc 03 §4.7 e). **The limit is the invariant; the
/// ±1000 ppm value is `[PROPOSAL]`** — the final number comes from
/// Phase 3 measurements. Changing it is a one-line, one-test change.
```
What is wrong: the capacity formula hardcodes `/1000` and the prefix's `b = input[c][0]` lerp (lines 238-247) silently assumes the entry cursor is `base−1` or `base`, which holds only while the step stays < 2 input samples per output (ppm > −500 000) and out ≤ in×(1+ppm/1e6) ≤ cap (ppm ≤ 1000). Raising `MAX_CORRECTION_PPM` — which drift.rs:33 advertises as "a one-line, one-test change" and says will happen via "Phase 3 measurements" — to any value > 1000 makes output exceed `cap` and panics at `plane[emitted] = ...` (line 243/258/276); below −500 000 the prefix mis-lerps silently. Nothing in resample.rs asserts or documents this coupling to the foreign constant. What correct looks like: derive cap from `correction.ppm()` (`in + in*ppm/1e6 + 2`) and `debug_assert!` the step bound. Confidence: HIGH — Verification: read (arithmetic; panics follow from plain indexing once cap is under-reserved).
F-004 [S4]
Where: crates/audio-engine/src/resample.rs:38-39
Code:
```
//!   (no allocation after warm-up — measured by the counting
//!   allocator in `tests/zero_alloc_quantum.rs`).
```
What is wrong: the resampler's zero-allocation measurement lives in `tests/zero_alloc_resample.rs` (which I ran: pass); `tests/zero_alloc_quantum.rs` measures the non-resample quantum path. The doc cites the wrong file for this claim. Confidence: HIGH — Verification: read + ran both paths (`ls crates/audio-engine/tests/` shows both files).
F-005 [S4]
Where: crates/audio-engine/src/resample.rs:119-123
Code:
```
    /// Set the correction (the governor's actuator input). The phase
    /// carry is renormalized against the new denominator so the
    /// `0 ≤ frac < step_den` invariant survives ratio changes
    /// mid-stream (the cursor absorbs the whole-sample part — no
    /// discontinuity, no reset).
```
What is wrong: "no discontinuity" is overstated. Renormalization subtracts whole `step_den` units but keeps `frac_num` verbatim, so the *fractional* position is reinterpreted against the new denominator: e.g. frac 500000 goes from 0.500000 (den 1e6) to ≈0.500500 (den 999000) — a sub-sample phase jump of up to ~0.002 samples at the ±1000 ppm bound. Inaudible and pinned by the differential test, but the comment asserts exactness the arithmetic does not have. Confidence: HIGH — Verification: read (rational arithmetic).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MicroResampler::new` | 99-111 | REAL | — | Fixed-size state; `out` starts empty (alloc on first process — fine) |
| `correction` | 114-117 | REAL | — | Getter |
| `set_correction` | 124-134 | REAL | — | Renorm loop correct for den ∈ [999000,1001000]; F-005 doc nit; dead in shipped build (F-001) |
| `reset` | 140-145 | REAL | — | Clears phase + tail; correction intentionally kept |
| `planes` | 151-154 | REAL | — | Borrows reused planes |
| `process` | 191-296 | REAL (tests pass) but UNREACHABLE in shipped build | S2/S3 | Exact rational phase; prefix/body/exact-tail split proven equivalent to reference (differential test + my analysis); panics on plane-shorter-than-in_frames (F-002); cap hardcodes /1000 (F-003) |
| `advance!` macro | 223-231 | REAL | — | frac i64, no overflow (frac < 1_001_000 + 1e6) |
| tests `planes` helper + 12 tests | 304-697 | REAL | — | See §5 |
| `ReferenceResampler` (test-only) | 498-592 | REAL | — | Pre-restructure single-loop kernel, differential oracle |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| zero_correction_emits_the_input_count_exactly | YES | Exact count + exact sample equality at ppm=0 |
| ratio_is_exact_over_long_streams | YES | 1000 quanta × ±1000 ppm vs closed form, ±1 tolerance — catches ratio drift |
| phase_carries_exactly_across_quanta | YES | 1-sample quanta stress cross-quantum carry; monotone + hull bounds |
| linear_interpolation_values_are_exact | YES | Closed-form lerp weights vs f64 reference at 1e-5 |
| channel_coherence_by_construction | YES | Cross-channel weight equality (scale-invariance check) |
| correction_change_midstream_renormalizes_the_phase | WEAK | Only asserts output count ranges (2..=5), not values — a value-corrupting renorm bug would pass |
| reset_starts_a_fresh_phase | YES | First sample exact equality post-reset |
| zero_frame_quantum_is_a_no_op | YES | Count + first-sample equality |
| out_never_exceeds_the_design_bound | YES (partial) | Bound check only; 7 lengths × ppm=+1000; doesn't test negative-ppm underflow of the ±1 |
| the_stream_never_fabricates_samples_past_the_kernel_window | YES | Convex-hull bound over 100 quanta at −777 ppm |
| restructured_kernel_is_bit_identical_… | YES | Full differential vs reference: 3 channel counts × 9 ppm values × 24 quanta, midstream correction change + reset — strongest test in file |
| special_values_survive_the_exact_tail_identically | YES | Bit-level (to_bits) equality on ±0.0/inf/NaN |
| (tests/zero_alloc_resample.rs, external) | YES | Counting allocator around the armed pipeline — ran, passes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "consumer #1 of DriftCorrection … audit M12 named absent" | resample.rs:1-3 | SHALLOW | Wiring exists (pipeline.rs:36,86,130,358) but dormant: shipped CLI never arms the governor (F-001) |
| "Linear … inaudible at ≤1000 ppm [PROPOSAL]" | resample.rs:15-17 | REAL-UNPROVEN | Linear kernel verified; "inaudible" is an untested psychoacoustic proposal — at ≤0.1% ratio change, aliasing is bounded to the top 0.1% of spectrum; no test measures audio quality |
| "no cumulative rounding drift, no floats on the control path" | resample.rs:18-23 | REAL | i128/i64 phase; only float is the per-sample weight division (data path), both operands < 2^24 |
| "out_len ≤ in_len + in_len/1000 + 1 for any |ppm| ≤ 1000" | resample.rs:40-41 | REAL | Test `out_never_exceeds_the_design_bound` + arithmetic (F-003 notes fragility) |
| "No … panics: … a geometry mismatch produces no output" | resample.rs:42-45 | FALSE (for frame-count mismatch) | F-002: line 292/255 panic on short planes |
| "Track-rate conversion (48 kHz in 44.1 kHz session) is a different, later card" | resample.rs:47-54 | REAL | No ratio beyond ±1000 ppm is expressible; `afeat-resampler` matrix row `not_started` (feature-matrix.toml:1730-1731) |
| PERF-VS-VLC resample-quality claim | docs/PERF-VS-VLC.md:116-118 | N/A (no Lumen claim) | The doc only records characterizing *VLC's* float aout resampler as an open follow-up; it claims nothing about Lumen's resampler — no unsupported quality claim exists |

## 7. Dead / duplicated / unreachable code
- `process`/`set_correction`/`reset`/`planes` on the non-zero-drift path: unreachable from the shipped CLI (F-001) — reachable only from audio-engine tests and apps/cli/tests/sync_scenarios.rs.
- `ReferenceResampler` (lines 498-592): test-only duplicate of the pre-restructure kernel — intentional differential oracle, not shipping duplication.
- No other dead code in the module.

## 8. Dependencies & flags
- Depends only on `media_clock::DriftCorrection` (workspace crate). No features gate this module; `audio-engine` is built in the release lane (lumen-cli depends on it via pipeline_host). No unsafe, no external crates.

## 9. Open questions for the next agent
- Who was supposed to call `AudioLimb::arm_drift_governor` in production ("the production pump once 3.B3's backpressure lands steady-state pacing")? Check whether 3.B3 landed and the call was forgotten (pipeline_host.rs:605-609 promises it).
- The shipped CLI's only audio sink is `NullDevice` (host.rs:79): confirm with the audio-output auditor whether any real backend (pulse/wasapi/coreaudio) is reachable in the release lane at all — if not, the entire drift-correction card is unobservable in production.
