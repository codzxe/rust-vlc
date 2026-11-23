# AUDIT: crates/codecs/tests/zero_alloc_decode.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 153 / 2 helpers + 1 test |
| File verdict | REAL (a genuine counting-allocator zero-event pin, executed green in the baseline; coverage scoped to one fixture geometry) |
| Reachable from shipped entry point? | N/A (test binary). The decode path pinned (`codecs::video::mjpeg::MjpegDecoder`) is the CLI's mjpeg lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | sed read; baseline cite; fixture-structure cross-check |

## 1. One-paragraph verdict
This is a real allocation proof, not a doc claim: a `CountingGlobalAlloc` process-global allocator with a guard around 64 steady-state decodes and a hard `assert_eq!(delta.alloc_calls, 0)` — any single allocation event in the measured window fails the suite. It ran and passed in the baseline (:1397-1399, 0.09s — 68 real decodes of a committed 320×240 4:2:0 MJPEG frame extracted from the generated AVI fixture, whose 60-video-frame structure I already verified against the committed structure map in my color.rs audit). The setup is honest: fixture read, frame extraction, configure and 4 warm-up decodes are outside the guard; the per-iteration `packet.clone()` is an Arc bump; the sink drops frames so the measured window sees only decode-side behavior. The one limitation: the pin covers exactly one output geometry (4:2:0 → I420) and a non-retaining sink — the Rgba8 assembly lane (4:2:2/4:4:4, color.rs) and the pool-exhaustion path are outside the measured claim.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:1395-1399: `mjpeg_contract_path_allocates_nothing_per_frame_after_warm_up ... ok` (1 passed, 0.09s).
- Mechanism read-verified: `#[global_allocator]` + `ALLOC.state().guard()` + `delta.alloc_calls` — a true counter, and the assert is exact-zero (the comment documents the W6-02 tightening from "two events" to zero).
- Frame-count integrity: `assert_eq!(jpegs.len(), 60)` matches the AVI structure map (60 `00dc` chunks — verified earlier via tests/fixtures/generated/valid/avi_mjpeg_pcm.avi.structure.json in my color.rs audit).
- Sink accounting: `assert_eq!(sink.frames, 4 + FRAMES)` — the measured loop really decoded.

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/zero_alloc_decode.rs:44, 112
Code:
```
    const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
...
    let packet = packet_for(&jpegs[0], generation);
```
What is wrong: the zero-alloc pin is measured on ONE frame shape — the fixture's 320×240 4:2:0 baseline JPEG (I420 output path). The MJPEG decoder's other assembly lane (Rgba8 for 4:2:2/4:4:4/4:4:0 — the color.rs compose path) and the grayscale lane are never in the measured window, so a per-frame allocation regression there passes this test silently. Likewise the pool-exhaustion path (a sink that RETAINS frames past FRAME_SLOTS=4) is by design not measured — the module docs promise boundedness there, not zero-alloc, so that half is honest, but the geometry half is a coverage gap in a test whose name claims the whole "contract path". What correct looks like: a second fixture row with a 4:2:2 JPEG (the mjpeg corpus has 422 lanes) through the same guard. Confidence: HIGH — Verification: read + fixture SOF parse (4:2:0, my color.rs audit).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DroppingSink | 51-60 | REAL | — | counts and drops — measures the handoff, not retention |
| packet_for | 62-73 | REAL | — | SharedBytes::from_vec once at setup |
| mjpeg_contract_path_allocates_nothing_per_frame_after_warm_up | 76-153 | REAL | S4 | F-001 scope; exact-zero assert; warm-up 4 + measured 64; serialize_tests for the global allocator |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mjpeg_contract_path_allocates_nothing... | YES (for the measured geometry) | exact-zero alloc assert over 64 decodes through the real contract surface; frame-count asserts guard against silent no-ops |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "each steady-state `MjpegDecoder::decode` ... allocates **nothing**" | zero_alloc_decode.rs:8-11 | REAL (measured) | baseline pass; exact-zero assert |
| "The pool is bounded at FRAME_SLOTS (4) ... a geometry change re-enters the cold path" | :13-17 | REAL (design) | decoder module tests (other file); this test's warm-up/cold-path behavior consistent |
| "real media, not self-authored bytes (D-5)" | :42-43 | REAL | generated AVI fixture + committed per-frame sha256s (fixtures_parent suite, baseline ok) |

## 7. Dead / duplicated / unreachable code
None. `jpegs[1..60]` are extracted but unused beyond the count assert — a cheap integrity check, not dead weight.

## 8. Dependencies & flags
Test-target with `#[global_allocator]` (one test per binary by design, serialize_tests). Depends on testkit::alloc_count, tests/common (AVI frame extraction — another agent's file). No features.

## 9. Open questions for the next agent
- tests/common.rs auditor: confirm `extract_avi_mjpeg_frames` parses the AVI rather than trusting structure.json offsets.
- If a 4:2:2 MJPEG fixture joins the corpus, extend this binary with a second geometry row (F-001).
