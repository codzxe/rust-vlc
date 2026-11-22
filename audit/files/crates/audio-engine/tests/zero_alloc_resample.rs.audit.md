# AUDIT: crates/audio-engine/tests/zero_alloc_resample.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 124 / 1 test fn + 2 helper fns/consts blocks |
| File verdict | REAL (a test file that tests something real) |
| Reachable from shipped entry point? | N/A (test) — runs in the workspace suite: plain `#[test]`, not `#[ignore]`, not cfg-gated; part of W00's 1751-passing baseline. Execution UNVERIFIED here (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read + cross-check of referenced symbols (`with_drift_governor`, `observe_drift`, `drift_stats` exist in audio-engine/src/pipeline.rs — grep in §2); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

A zero-allocation proof for the drift-corrected audio quantum path, and it is the honest kind: a counting **global allocator** measures the bytes, an explicit warm-up phase stabilizes the pooled slot rotation before the measured window opens, the measured loop drives the full per-quantum path (`observe_drift → process → mix → take_output`) over 512 quanta, and the guard asserts exactly zero allocated bytes — with post-conditions proving the loop really ran (governor sample count, +600 ppm correction live). The one scope limit is disclosed in the file itself: the steady state only; a correction *change* is a cold-path geometry change excluded from the measurement.

## 2. What actually works (verified)

- The measurement mechanism is real, not argued: `#[global_allocator] CountingGlobalAlloc` (14-16) + `ALLOC.state().guard()` (108) + `guard.assert_zero(...)` (117-119). Any per-quantum `Vec`/`clone` in the resampler planes, phase carry, or governor window state fails this test.
- Warm-up necessity is reasoned quantitatively, not hand-waved: +600 ppm ⇒ 1470.88 device samples per quantum ⇒ two slot lengths (1470/1471) must both be resident in the rotation before measurement (77-83); `drift_stats().samples == 128` asserts the governor window actually opened (96).
- The subject is the real `AudioPipeline` (`audio_engine::AudioPipeline::new(...).with_drift_governor(...)` 67-73) with content-varying frames (a stale-slot leak changes the bytes, 34-35) — not a mock of the subject.
- Post-conditions (122-123) pin sample count (640) and live correction (600 ppm), so the zero-alloc window cannot be vacuously empty.
- Referenced API exists on the production path: `with_drift_governor`/`observe_drift`/`drift_stats`/`set_drift` are audio-engine/src/pipeline.rs items (the same methods exercised at pipeline.rs:840-962 in its unit tests).

## 3. Findings

### F-001 [S4] Zero-alloc claim is steady-state-only; the cold path (correction change) is excluded by design and unmeasured
Where: crates/audio-engine/tests/zero_alloc_resample.rs:100-107
Code:
```rust
    /// (a correction CHANGE is a cold-path geometry change by design:
    /// new lengths enter fresh pooled slots, bounded by the rotation).
```
What is wrong: Nothing in the measured window exercises a correction change; the fill script deliberately holds the equilibrium so the ratio stays constant. A regression that allocates unboundedly on every correction change (e.g., growing the rotation instead of reusing) would pass this test. The comment discloses the exclusion and asserts boundedness "by the rotation" — an argument from code reading, exactly what this file's header (line 8) says it refuses to rely on.
Why it matters: The file's own standard ("measured by the counting global allocator, not argued from code reading") is met for the steady state and not met for the cold path. Hygiene/scope, not a product bug.
What correct looks like: A second test (or phase in this one) that steps the correction across values inside the guard and asserts a bounded allocation count per change.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `quantum_frame` | 36-55 | REAL | — | Content varies per quantum (index*37+i*11) — leak-sensitive payload; s16 stereo, sample-accurate pts. |
| `drift_corrected_quantum_path_is_zero_allocation_after_warm_up` | 62-124 | REAL | S4 | The proof itself; see F-001 for scope. |

Consts `RATE/QUANTUM_FRAMES/QUANTA/TARGET_FILL_NS/QUANTUM_NS` (27-32, 59): consistent arithmetic (QUANTUM_NS truncation 33,333,333 vs exact 33,333,333.33 — sub-ns-per-quantum skew, immaterial to the fill script).

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| drift_corrected_quantum_path_is_zero_allocation_after_warm_up | YES | Any steady-state per-quantum allocation trips `assert_zero`; vacuous-pass is blocked by the 640-sample/600-ppm post-conditions. Mutation example: make the resampler re-`collect()` its planes per quantum → guard fires. |

Single-test-per-binary design is documented (10-12) with the correct rationale (process-global allocator; sibling threads would pollute counts); `serialize_tests()` enforces it.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "allocates **zero bytes** … measured by the counting global allocator, not argued from code reading" (4-8) | module doc | REAL (steady state) | Guard + assert_zero mechanism verified; scope limit F-001. |
| "One `#[test]` per binary by design" (10-11) | module doc | REAL | Single test fn + serialize_tests. |
| Warm-up math (77-83) | comments | REAL-UNPROVEN | Arithmetic checked by hand (1470×1.0006=1470.88 ⇒ both slot lengths; 128 quanta ≈ 15 long-slot hits — plausible); actual pool behavior is audio-engine's internals (W07's files). |

## 7. Dead / duplicated / unreachable code

None. Companion file `zero_alloc_quantum.rs` (referenced line 2) is a separate assignment (not mine).

## 8. Dependencies & flags

dev-deps: testkit (allocator + Must), media-types, media-clock. No feature gates. Runs in the default test profile.

## 9. Open questions for the next agent

- W07 (audio-engine pipeline/resampler): confirm the pooled rotation's bound on cold-path growth — F-001's excluded path. If the rotation is unbounded across ratio changes, that is a real (slow) leak this file cannot see.
