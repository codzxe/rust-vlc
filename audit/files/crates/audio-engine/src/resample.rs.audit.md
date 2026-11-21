# AUDIT: crates/audio-engine/src/resample.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Lines / functions | 698 / 22 (8 API items + 1 macro + 13 test fns + ReferenceResampler 4 fns) |
| File verdict | UNREACHABLE (real, tested code; the `process` kernel is never activated by any shipped entry point) |
| Reachable from shipped entry point? | NO for `process` — call chain: `apps/cli/src/pipeline_host.rs:546` builds `AudioPipeline` → `pipeline.rs:130` `MicroResampler::new` (construction reachable); `pipeline.rs:353` gates `process` behind `drift.ppm() != 0`; the only writers of a non-zero drift are `set_drift`/`observe_drift`+armed governor — `arm_drift_governor`/`with_drift_governor` are called ONLY from `apps/cli/tests/sync_scenarios.rs:93` (test code). Shipped CLI never arms, never calls `set_drift` (rg over `apps/`: only `.drift()` reads). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "MicroResampler" crates` (callers: pipeline.rs only) · `rg -n "arm_drift_governor|set_drift|apply_correction" apps` (tests only) · `rg drift.rs ppm clamp` (limiter verified ±1000) · `sed -n` quote verification (all quotes below) · baseline `test-workspace.log.txt:112-129,148-151` (all 13 resample tests + zero-alloc test PASS) |

## 1. One-paragraph verdict
The arithmetic core of this micro-resampler is genuinely correct: I re-derived the phase math by hand (ratio `out = in × (1e6+ppm)/1e6`, exact i128/i64 rational accumulator, prefix/body/exact-tail loop split), proved the capacity bound `cap = in + in/1000 + 1` can never be exceeded for any `|ppm| ≤ 1000` (emissions ≤ `floor(d/step)+1` with `d ≤ n`, `floor(n×1.001) = n + floor(n/1000)`), and confirmed `DriftCorrection::new` clamps to ±1000 ppm (drift.rs:59-62) so `step_den ∈ [999000, 1001000]` is guaranteed positive — no div-by-zero, no infinite `while frac >= step_den` loop. The test suite is unusually honest (bit-exact differential test vs the former kernel over ppm × quantum-length × channels × mid-stream changes). BUT: none of this executes in the shipped build — the drift governor is armed only by a test file, so `process` is dead code in production, and the module explicitly disclaims general sample-rate conversion (48 kHz→44.1 kHz is "a different, later card") — the wave question's answer for rate conversion is: it does not exist.

## 2. What actually works (verified)
— All 13 unit tests + `drift_corrected_quantum_path_is_zero_allocation_after_warm_up` PASSED in baseline `audit/runs/baseline/test-workspace.log.txt:112-129,148-151` (audio-engine compiled, 0 failures).
— Hand re-derivation of the emission-count bound and the entry-cursor invariant (`entry cursor ∈ {base−1, base}` — body exit leaves `cursor ∈ {last_abs, last_abs+1}`, next `base = last_abs+1`): no path writes past `cap`; prefix ≤ 2 iterations at ±1000 ppm exactly as the comment at resample.rs:174-175 claims.
— `DriftCorrection` limiter: `rg -n "clamp" crates/media-clock/src/drift.rs` → `ppm: ppm.clamp(-MAX_CORRECTION_PPM, MAX_CORRECTION_PPM)` with `MAX_CORRECTION_PPM = 1000` (drift.rs:35,59-62). The resampler's ratio-safety assumption holds by construction.

## 3. Findings
### F-001 [S3]
Where: crates/audio-engine/src/resample.rs:191-193 (guard), 255 (slice), 292 (index), contrad. doc 42-45
Code:
```
    pub fn process(&mut self, input: &[Vec<f32>], in_frames: usize) -> usize {
        if in_frames == 0 || input.len() != self.channels {
            return 0;
```
What is wrong: the "geometry mismatch" guard checks only the channel count, not per-plane length. A caller passing `in_frames` greater than any `input[c].len()` panics at `&in_plane[idx..idx + 2]` (line 255) or `input[c][in_frames - 1]` (line 292). The module doc explicitly promises the opposite: "No `unsafe`, no floats on the control path, no panics: … a geometry mismatch produces no output" (lines 42-45).
Why it matters: the documented contract is false for half the geometry-mismatch space; any future caller (the API is `pub`, exported at lib.rs:127) inherits a panic path the docs say cannot happen.
What correct looks like: either extend the guard to `input.iter().all(|p| p.len() >= in_frames)` (returning 0), or fix the doc to "channel-count mismatch only; plane length is the caller's invariant".
Confidence: HIGH — Verification: read (slice/index sites traced); the single production caller `pipeline.rs:358` passes `self.mixed`, whose plane lengths equal `frames` by mixer construction (verified in pipeline part2 audit), so not reachable today.

### F-002 [S3]
Where: crates/audio-engine/src/resample.rs:191 (`process` — whole kernel), activation gate at crates/audio-engine/src/pipeline.rs:353-360
Code:
```
        let (planes, out_frames) = if self.drift.ppm() == 0 {
            (self.mixed.as_slice(), frames)
        } else {
```
What is wrong: the entire resample kernel is dead code in the shipped build. The only route to `drift.ppm() != 0` is `AudioPipeline::set_drift` (pipeline.rs:162-165) or `observe_drift` with an armed governor (pipeline.rs:185-192). `rg -n "arm_drift_governor|with_drift_governor|set_drift" apps/` → only `apps/cli/tests/sync_scenarios.rs:93` (test) and observability reads (`pipeline_host.rs:647`). The shipped `lumen` CLI constructs the pipeline unarmed and never applies a correction, so `process` never runs outside tests.
Why it matters: 698 lines of real, tested DSP plus the `media-clock` drift-governor machinery it closes the loop on, are unreachable in the product; the doc-03 §4.7(e) "audio-side drift is absorbed by a bounded micro-resampler" behavior does not exist at runtime in the shipped binary.
What correct looks like: either wire the governor into the CLI composition (the `arm_drift_governor` doc at pipeline.rs:149-154 says compositions "arm the governor where production is paced at the consumption rate" — no shipped composition does), or mark the module test-only until the wiring card lands.
Confidence: HIGH — Verification: read + `rg` call-chain exhaustion (apps/cli, apps/web checked; apps/web/host.rs:74 uses `AudioPipeline` but the web build is not a shipped entry point and parity-report says web=0/252).

### F-003 [S4]
Where: crates/audio-engine/src/resample.rs:47-54 (module doc), docs/feature-matrix.toml:1723-1732 (row `afeat-resampler`)
Code:
```
//! Track-rate conversion (a 48 kHz track in a 44.1 kHz session) is a
//! different, later card: it changes the *negotiated* rate, not the
//! device-vs-nominal ratio. This type's whole contract is the
```
What is wrong: not a code bug — but the doc/matrix alignment is inverted. The matrix row `afeat-resampler` ("Bounded audio resampling and channel-map conversion between decode and output") reports `not_started` on all four platforms with empty evidence, while a bounded resampler + channel map demonstrably exist and pass tests; conversely the one thing users actually need (48 kHz track → 44.1 kHz session conversion) does not exist anywhere in the tree, and the pipeline answers a rate mismatch with a typed refusal (pipeline part2). The cell neither reflects the existing code nor flags the real gap.
Why it matters: the matrix is the project's self-report instrument; this row tells the owner "nothing begun" when the truth is "correction-resampler done but unwired; track-rate conversion absent".
What correct looks like: split the row (drift-correction resampler: implemented/unwired; rate conversion: not_started) or set the cell to `partial` with the resample.rs citation.
Confidence: HIGH — Verification: read of both files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `const PPM_SCALE` | 60 | REAL | — | restates drift.rs constant; consistent (verified against drift.rs PPM_SCALE use at drift.rs:118) |
| `struct MicroResampler` | 70-94 | REAL | — | state is O(channels); i128 cursor — no overflow (7-year headroom comment is conservative, i128 has ~3.8e38 range) |
| `MicroResampler::new` | 100-111 | REAL (reachable) | — | called by `AudioPipeline::new` (pipeline.rs:130) — reachable in shipped build; `channels: u32 as usize` — no file-controlled blowup (spec-bounded) |
| `correction` | 114-117 | REAL | — | trivial getter |
| `set_correction` | 124-134 | REAL | — | renormalization preserves position exactly (cursor+frac/den invariant); early-return on equal ppm avoids churn; only reachable via `set_drift` (unshipped) |
| `reset` | 140-145 | REAL (reachable) | — | called via `reset_drift` ← `pipeline_host.rs:1106` (shipped); deliberately keeps `correction` — documented |
| `planes` | 152-154 | REAL | — | returns last-process planes; `out[c].len()` == returned count (truncate at 282-284 guarantees) |
| `advance!` macro | 223-231 | REAL | — | exact integer phase advance; `step_den ≥ 999000` guaranteed by DriftCorrection clamp — loop terminates |
| `process` | 191-296 | UNREACHABLE (kernel) | S3 | math verified by hand: ratio exact, capacity bound provably safe, prefix ≤ 2 iters, body idx+2 ≤ in_frames, exact-tail weight-0 −0.0 flush documented as pinned bits; channel-count-only guard (F-001) |
| `tests::planes` (helper) | 304-306 | REAL | — | test helper |
| `zero_correction_emits_the_input_count_exactly` | 308-319 | REAL | — | pins step=1 at ppm 0 |
| `ratio_is_exact_over_long_streams` | 321-344 | REAL | — | 1000 quanta ±1 sample at ±1000 ppm — would fail on any ratio sign/scale error (mutation: `step_den = PPM_SCALE − ppm` fails here) |
| `phase_carries_exactly_across_quanta` | 346-371 | REAL | — | 1-sample quanta stress the tail path; monotonicity+hull asserts |
| `linear_interpolation_values_are_exact` | 373-395 | REAL | — | closed-form f64 cross-check |
| `channel_coherence_by_construction` | 397-414 | REAL | — | would fail if phases diverged per channel |
| `correction_change_midstream_renormalizes_the_phase` | 416-432 | REAL | — | range asserts are loose (2..=5) but the differential test covers renormalization bit-exactly |
| `reset_starts_a_fresh_phase` | 434-446 | REAL | — | pins post-reset phase 0 |
| `zero_frame_quantum_is_a_no_op` | 448-457 | REAL | — | pins the 0-frame early return |
| `out_never_exceeds_the_design_bound` | 459-474 | REAL | — | first-quantum only; multi-quantum cap is pinned indirectly (new kernel panics past cap; differential test covers 24-quantum streams) |
| `the_stream_never_fabricates_samples_past_the_kernel_window` | 476-491 | REAL | — | convex-hull property |
| `ReferenceResampler::{new,set_correction,reset,process}` | 498-592 | REAL | — | verbatim pre-W5-19 kernel kept as differential oracle — honest practice, not dead weight |
| `restructured_kernel_is_bit_identical_to_the_former_single_loop` | 594-657 | REAL | — | 3 channels × 9 ppm × 24 quanta incl. mid-stream correction change + reset; strongest test in the file |
| `special_values_survive_the_exact_tail_identically` | 659-697 | REAL | — | bit-level (to_bits) compare incl. NaN/±inf/−0.0 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 13 tests | YES | Mutation check (reasoned, not run): flipping `step_den` sign (line 196) breaks `ratio_is_exact_over_long_streams` (±1000 ppm arms it); breaking the capacity arithmetic (line 213 `cap` too small) panics `out_never_exceeds_the_design_bound` AND the differential test (index-write kernel cannot exceed cap without panicking — the reference's `push` would silently grow, so the panic itself is the discriminator); breaking prefix/body split order breaks bit-identity; all executed in baseline (test-workspace.log.txt:112-129) |
| `#[ignore]`d `pipeline_throughput_*` (tests/, not this file) | n/a | baseline log lines 136-137 show both ignored — W5-19 throughput convention, requires `--release --ignored`; numbers not in any run I can cite — UNVERIFIED |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Audio-side drift … absorbed by a bounded micro-resampler in audio-engine: correction rate limited to ±1000 ppm" | doc 03 §4.7(e) (03-engine-plan.md:561-564) | REAL-UNPROVEN | Code exists, unit-proven (baseline tests), but no shipped composition arms the governor (F-002) — the absorption never happens in the shipped binary |
| "D: device adaptation — rate/channel/format conversion, micro-resample" | doc 03 §4.8(b) MixPoint sketch (03-engine-plan.md:591) | SHALLOW | micro-resample implemented-unwired; rate conversion absent (resample.rs:47-54 disclaims it); format conversion = f32 only (pipeline device_format); channel map exists (mix.rs) |
| afeat-resampler row `not_started`/empty evidence | feature-matrix.toml:1723-1732 | FALSE (under-reports existing code) / TRUE (not in product) | resample.rs 698 lines + tests pass in baseline; but `process` unreachable in shipped build (F-002) |

## 7. Dead / duplicated / unreachable code
- `process` kernel + `set_correction`'s renormalization path: unreachable in shipped build (F-002) — reachable only from `apps/cli/tests/sync_scenarios.rs` and the crate's own tests.
- `ReferenceResampler` (498-592): deliberate test-oracle duplication of the production kernel — acceptable (differential testing), but it IS ~95 lines of copy-paste that must be maintained in lockstep; a comment break between the two would silently rot the oracle.
- `apps/web/src/audio/clock.rs:262` describes itself as "the twin of audio-engine's MicroResampler phase rule" — a second implementation of the same phase discipline in another crate (flagged for the web auditor; not this file's defect).

## 8. Dependencies & flags
- `media-clock` (path dep) — only for `DriftCorrection`; verified clamp (drift.rs:35,59-62).
- No feature flags gate this module; it compiles in the default workspace build (baseline build-release-workspace.log.txt exit 0; audio-engine compiled per test-workspace.log.txt:29).
- No `unsafe` in the file (rg verified: none). No external deps (std only) — matches the crate manifest's "Std-only by C5".

## 9. Open questions for the next agent
- pipeline part2 (W07-A02, same agent — resolved there): does `self.mixed` plane length always equal `frames` at the `process` call (F-001's reachability)? → RESOLVED: yes, mixer writes exactly `frames` per plane (see pipeline part2 audit §F-001 note).
- Who was supposed to arm the governor in the shipped CLI (which card), and is the null device the reason it never happens? The `observe_drift` doc (pipeline.rs:149-154) claims compositions exist that arm late — only tests do.
