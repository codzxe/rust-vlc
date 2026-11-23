# AUDIT: crates/renderer-wgpu/tests/zero_alloc_present.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 128 / 2 (request, present_path_is_zero_allocation_after_warm_up) + global allocator |
| File verdict | REAL (executed) |
| Reachable from shipped entry point? | N/A (test); the present path it measures is shipped (renderer-wgpu in both apps; software compositor is the v0 backend — asserted `backend_name() == "software"`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | executed prebuilt `zero_alloc_present-a1a58fac9a1f3d5b --nocapture` -> 1 passed / 0 failed, 0.04s (baseline identical, cargo-test-workspace.txt:5293-5297) |

## 1. One-paragraph verdict
A real zero-allocation proof with an anti-vacuity gate: before the measured window the test READS BACK the framebuffer and pins four pixels (band placement, 2x Fit resample values, both letterbox bars) — proving the present path actually composited; then 128 full presents run under a counting global allocator with `assert_zero`. The header is candid that this binary materializes evidence a prior audit found missing ("staging.rs claimed a counting-allocator proof binary that did not exist"). I executed it: 1/1 in 0.04s, matching baseline. The only limitation is inherent: it proves the HEADLESS software surface path (the only v0 backend); there is no hardware present path yet to measure.

## 2. What actually works (verified)
— verified by execution: `test result: ok. 1 passed; 0 failed` (0.04s); baseline L5293-5297 identical.
— verified by read: the pixel pins are arithmetically correct (frame(16,8) → (128,128,0x5A) under 2x scale; bars opaque black), so a present path that no-ops or misplaces would fail BEFORE the zero-alloc window, and one that allocates would fail INSIDE it (counting allocator catches alloc-free pairs too, being count-based).
— verified the request is built once and passed by reference (L98, L125) — the window cannot hide request-construction allocations.

## 3. Findings
F-001 [S4]
Where: crates/renderer-wgpu/tests/zero_alloc_present.rs:1-4 (header) and the headless-only surface
Code: (verbatim)
```
//! Zero-allocation proof for the renderer's present path (W2-06,
//! materializing the evidence the W1-20 re-audit found missing:
//! staging.rs claimed a "counting-allocator proof binary" that did
//! not exist — this is that binary, and now the claim is real).
```
What is wrong (scope note, not a defect): the proof covers the software headless surface only; "the renderer's present path" is today exactly that path (wgpu not yet imported — see color.rs audit F-002), so the claim is currently complete, but the moment a GPU backend lands this test no longer covers it. The W1-20 history ("a proof binary that did not exist") is UNVERIFIABLE without git. Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ALLOC (global) | 25-27 | REAL | - | counting global allocator, testkit |
| request | 53-82 | REAL | - | 32x16 gradient RGBA, 2:1 aspect letterbox driver |
| present_path_is_zero_allocation_after_warm_up | 84-128 | REAL | - | serialize_tests; warm-up 8; readback pixel pins; 128-cycle assert_zero window — executed |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| present_path_is_zero_allocation_after_warm_up | YES (both directions) | a broken composite fails the pixel pins; any steady-state allocation (even alloc+free) fails assert_zero; executed |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "After warm-up, N × (present → stage → place → composite → present) allocates zero bytes" | L6-8 | REAL | executed: assert_zero passed over 128 cycles |
| "this is that binary, and now the claim is real" (W2-06 fix) | L1-4 | REAL (binary exists + passes); the historical nonexistence is UNVERIFIED (no git) | my execution |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
testkit (alloc_count, Must), video_engine (Presenter/PresentRequest), renderer_wgpu. Test-only. One #[test] per binary by design (process-global allocator) — documented rationale.

## 9. Open questions for the next agent
- When the GPU backend lands (Lead-gated wgpu import), this proof needs a hardware-surface twin; note in F-001.
- staging.rs's OTHER allocation claims (RecyclePool 3 slots, grown-only-on-geometry-change block) are asserted indirectly here; direct unit pins live in staging tests (not assigned).
