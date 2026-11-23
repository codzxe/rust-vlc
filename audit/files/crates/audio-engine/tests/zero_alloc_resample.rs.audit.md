# AUDIT: crates/audio-engine/tests/zero_alloc_resample.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-24 |
| Lines / functions | 124 / 2 (quantum_frame, drift_corrected_quantum_path_is_zero_allocation_after_warm_up) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test binary — not shipped by design; it exercises `AudioPipeline::with_drift_governor` + `MicroResampler` (audio-engine/src/pipeline.rs:353-359, resample.rs), which the shipped audio path uses |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `target/debug/deps/zero_alloc_resample-0eb6265d03fdd675` → 1 passed (0.33s); `zero_alloc_resample-7b3f68e4d3519472` → 1 passed (0.30s) (both prebuilt variants green); rg for quality-measurement tests across audio-engine (none found) |

## 1. One-paragraph verdict
REAL and honest about what it proves. This is a genuine zero-allocation proof, not an argument from code reading: a counting global allocator (`testkit::alloc_count::CountingGlobalAlloc`) wraps the process, a serialized single test warms the pooled rotation for 128 quanta, then a guard window runs the full per-quantum pump (`observe_drift → process → mix → take_output → drop`) over 512 quanta at a live +600 ppm correction and asserts zero allocation events and zero bytes — I ran both prebuilt binary variants and both pass in ~0.3 s. The warm-up arithmetic (1470 × 1.0006 = 1470.882 → ~7/8 long quanta, ~15 short-length hits in 128) is correct, and the test is cleverer than it looks: any ratio error producing an unexpected output length would miss the pooled rotation and allocate, so `assert_zero` indirectly pins the resampler's ±1-frame length behavior too. The one gap is inherited from the domain, not the file: the resampler's filter-quality claim ("linear is inaudible at ≤1000 ppm") is measured nowhere in the tree — this file proves D-1 allocation, resample.rs's own unit tests pin ratio/phase exactness, and the audibility claim rests on a `[PROPOSAL per W1-18 §3.2]` tag with zero spectral/THD evidence.

## 2. What actually works (verified)
— verified by running both prebuilt binaries: `drift_corrected_quantum_path_is_zero_allocation_after_warm_up ... ok` (1 passed, exit 0, twice).
— verified by reading testkit/src/alloc_count.rs:126-208: the guard snapshots `alloc_calls`/`total_bytes` and `assert_zero` panics on any allocation event inside the window ("the zero-allocation contract failing loudly, never silently"); the instrument itself has a meta-test (`assert_zero_fails_loudly_on_a_nonzero_window`, alloc_count.rs:309).
— verified arithmetic: 1470 × 1.0006 = 1470.882 (long-quanta fraction 0.882 ≈ 7/8, matching the doc's "roughly seven 1471-frame quanta per 1470-frame one"); 128 × 0.118 ≈ 15.1 short-length hits, matching "≈ 15".

## 3. Findings
F-001 [S4]
Where: crates/audio-engine/tests/zero_alloc_resample.rs:5 and 109-115
Code:
```rust
//! `drop → observe_drift` over N further quanta allocates **zero
//! bytes** —
```
What is wrong: the doc says "over N further quanta", but the measured window iterates `for frame in &frames` — ALL 512 frames, whose first 128 are the SAME frames the warm-up already processed (content and pts replay: pts restarts at 0 after the warm-up ended at pts 127×1470). For an allocation proof the replay is irrelevant (content does not drive allocation; the asserts confirm), but the description is inaccurate: the window is 512 quanta of which 128 are re-runs, not 512 fresh ones after the 128 warm-up. Cosmetic accuracy nit on an otherwise airtight proof. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `quantum_frame` | 36-55 | REAL | — | deterministic per-quantum varying content (stale-slot leaks would surface as wrong bytes — though nothing in this test compares content; that pin lives elsewhere) |
| `drift_corrected_quantum_path_is_zero_allocation_after_warm_up` | 61-124 | REAL | S4 | the proof itself; warm-up excluded by design and documented; F-001 wording nit |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `drift_corrected_quantum_path_is_zero_allocation_after_warm_up` | YES (allocation) | any per-quantum alloc (fresh slot, plane growth, governor window realloc, boxed error/closure) trips `assert_zero` with the exact byte/call delta |
| — ratio regression | YES (indirect) | a wrong ratio emitting an unpooled length allocates a fresh slot → assert_zero fails |
| — governor liveness | YES | `assert_eq!(pipeline.drift_stats().samples, 128)` then `QUANTA + 128` — the loop provably ran and observed every quantum |
| — correction sanity | WEAK | `assert!(correction.ppm().abs() <= 1000)` only bounds; the scripted equilibrium fill makes the desired correction equal the one in force (documented at 99-107) |
| quality (THD/SNR/spectral) | NOT TESTED | no test in audio-engine measures resample output quality (rg over src/ + tests/: only resample.rs docs mention it) — see §6 |

## 6. Doc claims touching this file (the resample hostile-focus ledger)
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "allocates zero bytes — … measured by the counting global allocator, not argued from code reading" | zero_alloc_resample.rs:5-8 | REAL | ran: 1 passed ×2 binaries; instrument verified in testkit/src/alloc_count.rs |
| warm-up "128 quanta ≈ 15 short-length hits; the rotation stabilizes" | zero_alloc_resample.rs:82-83 | REAL | arithmetic re-derived: 128 × (1 − 0.882) ≈ 15.1 |
| resample ratio `out = in × (1e6 + ppm)/1e6`, exact i128 rational phase, "no cumulative rounding drift, no floats on the control path" | resample.rs:10-24 (context, not this file) | REAL (unit-tested there: `ratio_is_exact_over_long_streams`, `phase_carries_exactly_across_quanta`) + indirectly pinned here via the pooled-length argument | ran this file's test; resample.rs tests are another agent's assignment |
| "[PROPOSAL per W1-18 §3.2: linear is inaudible at ≤1000 ppm; cubic is a later quality knob]" | resample.rs:15-17 (context) | PAPER-ONLY for the audibility half | rg `thd\|snr\|spectral\|quality\|inaudible` over audio-engine: matches only in resample.rs's own docs — no measurement test exists anywhere in the tree; the claim is honestly tagged [PROPOSAL] but is evidenced by nobody |
| "a correction CHANGE is a cold-path geometry change by design: new lengths enter fresh pooled slots, bounded by the rotation" | zero_alloc_resample.rs:105-107 | REAL-UNPROVEN | the steady window is measured; the change-path exclusion is documented, not measured here (no ratio-swing test asserts bounded alloc on cold path — UNVERIFIED) |

## 7. Dead / duplicated / unreachable code
None. (`QUANTUM_NS` truncation: 1470×1e9/44100 = 33,333,333.33 → 33,333,333 ns per step; the ~0.33 ns/quantum cumulative slip is irrelevant because the fill is injected, not derived from `now_ns` — verified the fill parameter is a scripted constant at line 93/111.)

## 8. Dependencies & flags
Test-only; depends on audio-engine, media-clock (DriftCorrection), testkit (alloc_count + Must). The `#[global_allocator]` swap makes this binary mutually exclusive with any other allocator user — handled by `serialize_tests()` + one test per binary (testkit/src/alloc_count.rs:64).

## 9. Open questions for the next agent
- audio-engine/src/resample.rs owner: the "inaudible at ≤1000 ppm" claim needs either a measurement test (band-limited sweep through the resampler, THD vs the ±1000 ppm bound) or a downgrade to explicitly untested design rationale — today it is a PROPOSAL with no evidence in the tree.
- The cold-path ratio-change allocation bound ("new lengths enter fresh pooled slots") has no test asserting it stays bounded across repeated ratio swings — a hostile governor oscillation test is owed.
