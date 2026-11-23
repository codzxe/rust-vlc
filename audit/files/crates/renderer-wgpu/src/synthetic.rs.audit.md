# AUDIT: crates/renderer-wgpu/src/synthetic.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 168 (75 source + 93 tests) / 2 non-test fns + 5 tests |
| File verdict | REAL (deterministic pattern generator; every asserted pixel value independently recomputed) |
| Reachable from shipped entry point? | NO by design — test-support generator ("card 2.D1 AC generator"): feeds renderer-wgpu's png_hash acceptance test (baseline ok), tests/common, and the lib doc example. No shipped-binary consumer |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of full file; hand-recomputation of all 8 asserted pixel values; `Grep` consumer map; baseline citation audit/runs/baseline/cargo-test-workspace.txt:5252-5256, 5274 |

## 1. One-paragraph verdict
A small, correct, genuinely deterministic test-frame generator. The pattern is pure integer math (gradient via round-half-up `(i·255+(n−1)/2)/(n−1)`, corner markers, 16-px grid, `(x+y)%256` blue channel), and I recomputed every value its tests assert — (5,5)→[20,27,10], (17,1)→[69,5,18], 9×2 (4,1)→[128,255,5], 9×3 (4,1)→[128,128,5], corner/grid pixels — all exact, including the half-up 127.5→128 cases. The generator's whole reason to exist (byte-identical frames across machines so a committed PNG hash is an acceptance proof) is honored: no time, no entropy, no environment input. The one defect is an unguarded capacity computation that can overflow for absurd extents.

## 2. What actually works (verified)
— Hand recomputation of all 8 asserted pixels in `pattern_matches_its_specification` and `degenerate_axes_map_to_the_midpoint` — every value exact (round-half-up verified at the 127.5 and 20.24 boundaries).
— Baseline: `synthetic::tests` 5/5 ok (audit/runs/baseline/cargo-test-workspace.txt:5252-5256) plus the downstream `ac_synthetic_frame_headless_render_produces_the_expected_png_hash ... ok` (5274) — the generator's output survives a real render+PNG-hash round trip.
— Determinism test asserts self-equality and exact byte length.

## 3. Findings
F-001 [S4]
Where: synthetic.rs:33
Code: (verbatim)
```
    let mut out = Vec::with_capacity((w as usize) * (h as usize) * 4);
```
What is wrong: For near-`u32::MAX` extents the capacity arithmetic overflows `usize` (e.g. w=h=2^32−1 → ~2^66): debug builds panic on the overflow, release builds compute a bogus (wrapped) capacity — harmless correctness-wise because `extend_from_slice` grows the Vec as needed, but it is the only unchecked arithmetic in an otherwise fully-checked crate and contradicts the file's "free of ... environment" discipline only in the sense that it trusts its caller with sane extents. Current callers (tests, 64×48-ish extents) cannot reach it.
Confidence: HIGH — Verification: read (arithmetic); not executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| synthetic_pattern | 30-54 | REAL | S4 | gradient/corner/grid/b all spec-exact; F-001 capacity |
| synthetic_frame | 59-74 | REAL | — | Rgba8, PTS 0, sRGB/full-range metadata, CPU storage |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pattern_is_deterministic_and_sized | YES | self-equality + exact length |
| pattern_matches_its_specification | YES | 8 exact pixels incl. rounding boundaries (independently recomputed) |
| degenerate_axes_map_to_the_midpoint | YES | n≤1 midpoint + 2×2/3-px gradient extremes |
| frame_wraps_the_pattern_with_presentation_metadata | YES | format/size/pts/generation/color/storage asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "fully specified in code and free of time, entropy and environment ... byte-identical frames" | synthetic.rs:4-7 | REAL | pure function read + determinism test + downstream PNG-hash acceptance test passing |
| pattern spec (gradient/corners/grid) | synthetic.rs:9-18 | REAL | every rule verified against the code and recomputed pixel values |

## 7. Dead / duplicated / unreachable code
None dead. Not reachable from shipped binaries — by design (AC generator for the render acceptance test, cited as evidence by png_hash.rs which another agent audited as REAL).

## 8. Dependencies & flags
media_types (VideoFrame/SharedBytes/ColorDescription), video_engine::NANO_BASE, crate::params::Extent2D. No features.

## 9. Open questions for the next agent
None.
