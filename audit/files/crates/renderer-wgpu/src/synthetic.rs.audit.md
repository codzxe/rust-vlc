# AUDIT: crates/renderer-wgpu/src/synthetic.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 168 / 2 functions + 4 tests + 1 test helper |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (crate is a shipped CLI/desktop dependency) but with ZERO production callers: `synthetic_pattern`/`synthetic_frame` are consumed only by `apps/desktop/examples/emit_stream.rs:29/:99`, `apps/desktop/tests/common/mod.rs:106`, `crates/renderer-wgpu/tests/{png_hash,common}` — test/example support shipped in the production lib (F-002). The crate itself is reached via `apps/cli/src/pipeline_host.rs:81-83`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n` full read · hand-verification of every test constant (r=round(5·255/63)=20, g=27, (17,1)→[69,5,18], (4,1)@9×2→[128,255,5], (4,1)@9×3→[128,128,5] — all recomputed against the closure at :34-40) · `rg -n 'synthetic_pattern\|synthetic_frame' crates apps` (test/example callers only) · `sed -n '4841,4852p' orchrun…log` (png_hash 5/5 pass incl. the AC) · `sed -n '40,120p' tests/png_hash.rs` (hash provenance + oracle verification) · `rg -n 'video-engine' crates/renderer-wgpu/Cargo.toml` (:29 — dep declared; boundaries envelope "grants media-types + video-engine only") |

## 1. One-paragraph verdict
The deterministic pattern generator does exactly what it claims, and — unlike the first-vertical final-frame lane — its committed-hash acceptance proof is GREEN in the baseline: `png_hash.rs` ran 5/5, including the AC (`ac_synthetic_frame_headless_render_produces_the_expected_png_hash`), double-run byte-identity, and a committed SHA-256 whose provenance is documented as independently verified (system sha256sum + an out-of-tree Python zlib oracle that decoded the stream and recomputed the pattern probes). I recomputed every pixel constant the unit tests assert and they are all arithmetically correct against the closure; the gradient's rounding (`(i·255 + (n−1)/2)/(n−1)`) hits exactly 0 and 255 at the extremes; degenerate single-pixel axes map to 128 as specified. Two S4 nits: the module doc's "override order" enumerates Base → Corner markers → Grid, which reads as later-overrides-earlier while the code (and its test) make corners beat grid — the precedence is only recoverable from the test; and the generator is a test fixture shipped in the production crate with zero production callers.

## 2. What actually works (verified)
— Gradient math (:34-40): recomputed by hand — max i=n−1 gives (A·255 + A/2)/A = 255 (integer), min gives 0; rounding is round-half-up as the spec's `round()` claims; n≤1 → 128 midpoint (spec :13-14) — pinned by `degenerate_axes_map_to_the_midpoint` (:125-146, every constant recomputed by this auditor: (4,1)@9×2 → [128,255,5,255], (4,1)@9×3 → [128,128,5,255]).
— Corner blocks (:43): `(x < 4 && y < 4) || (x + 4 >= w && y + 4 >= h)` — exactly 4×4 at top-left and bottom-right, clipped on tiny extents; 1×1 is all-marker (test :131).
— Grid (:45): `x.is_multiple_of(16) || y.is_multiple_of(16)` — every 16th column/row white, as specified (:18).
— b channel `(x + y) % 256` (:48): verified at all four probed pixels.
— Executed acceptance chain: `synthetic_frame(64×48)` → present → readback → `screenshot::encode_rgba` → SHA-256 == committed constant — **passed in the no-fail-fast baseline** (orchrun :4846), alongside `double_run_renders_are_byte_identical` (:4849) and `the_committed_hash_pins_a_verified_image` (:4848). This is the module's entire reason to exist, working, with committed evidence.
— Frame wrapper (:59-74): Rgba8, PTS 0 on NANO_BASE, BT.709/sRGB/full-range, `GenerationId::FIRST`, CPU storage — all pinned by `frame_wraps_the_pattern_with_presentation_metadata` (:149-167).

## 3. Findings

### F-001 [S4]
Where: crates/renderer-wgpu/src/synthetic.rs:9-18
Code:
```
//! # Pattern specification (per pixel, in override order)
//!
//! 1. Base: a two-axis linear gradient —
//!    `r = round(x · 255 / (w−1))` (clamped), `g = round(y · 255 /
//!    (h−1))`, `b = (x + y) mod 256`, `a = 255`; a single-pixel-wide
//!    axis maps the gradient to the midpoint 128.
//! 2. Corner markers: 4×4 opaque red blocks at the top-left and
//!    bottom-right corners (visual anchoring for humans; a fixed
//!    reference for tests).
//! 3. Grid: every 16th column and row is opaque white.
```
What is wrong: "In override order" with items numbered 1→3 naturally reads as each later layer overriding the earlier (grid over corners) — but the code applies the opposite precedence: corner markers win over grid lines (`:43-46` checks corners first; pixel (0,0) — a grid intersection — is red, not white, pinned by the test at :114). The specification's precedence is only recoverable by reading the implementation or the test, not the spec.
Why it matters: The module's contract is "fully specified in code and free of time, entropy and environment" (:4-5) so that an independent reimplementation (the Python oracle the AC cites) can reproduce it byte-for-byte; an ambiguous precedence in the written spec is exactly the kind of gap an independent implementer resolves wrongly (grid-white at corners) and then "fails" against the committed hash for no visible reason.
What correct looks like: One sentence: "later layers do NOT override earlier ones — corner markers take precedence over grid lines, which take precedence over the gradient" (or renumber so the prose matches).
Confidence: HIGH — Verification: read (code order at :43-48 vs doc enumeration at :9-18; test :114 pins the tiebreak).

### F-002 [S4]
Where: crates/renderer-wgpu/src/synthetic.rs:29-30
Code:
```
/// The fully-specified synthetic RGBA8 pattern (module docs) as raw
/// bytes — `extent.width * extent.height * 4`, row-major.
#[must_use = "pure generator"]
pub fn synthetic_pattern(extent: Extent2D) -> Vec<u8> {
```
What is wrong: `synthetic_pattern`/`synthetic_frame` are `pub` items of the production crate — compiled into the shipped `lumen` binary (renderer-wgpu is an apps/cli dependency) — with zero production callers: every consumer is a test or an example (`apps/desktop/examples/emit_stream.rs`, `apps/desktop/tests/common/mod.rs:106`, `crates/renderer-wgpu/tests/{png_hash,common}`). This is a test-fixture generator living in production code.
Why it matters: The shipped API surface advertises a capability no shipped feature uses — an auditor tracing "what does the renderer do" finds a pattern generator that nothing in the product invokes. Cost is small (168 lines, no deps), and the placement is arguably deliberate (cross-crate test support needs the pub item), but nothing marks it as test-support.
What correct looks like: Either `#[doc(hidden)]` + a doc note "test/AC support — no production consumer", or move the generator behind a dev-only feature with the tests enabling it (the cross-crate consumers can use a `[dev-dependencies]`-visible re-export).
Confidence: HIGH — Verification: read + exhaustive caller rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `synthetic_pattern` | 30-54 | REAL | S4 | Math verified by hand at 5 probe pixels; extremes exact; capacity exact; F-002 (pub test fixture). |
| `synthetic_frame` | 59-74 | REAL | S4 | Presentation metadata pinned by test; uses `video_engine::NANO_BASE` (declared dep, boundaries-legal). |
| tests ×4 + `pattern_pixel` helper | 76-168 | REAL | — | Every constant recomputed by this auditor; all pass in baseline (renderer_wgpu 60/60, orchrun :4767-4781). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `pattern_is_deterministic_and_sized` (:93) | YES (weakly) | Equality + exact byte count. |
| `pattern_matches_its_specification` (:103) | YES | Hand-verifiable constants at 6 pixels including both corner markers, both grid axes, and two gradient points — kills rounding/off-by-one/precedence mutations. |
| `degenerate_axes_map_to_the_midpoint` (:125) | YES | n=1, n=2, n=3 axis behaviors pinned. |
| `frame_wraps_the_pattern_with_presentation_metadata` (:149) | YES | Every wrapper field asserted; OpaqueHandle arm explicitly unreachable-by-construction. |
| (external) `png_hash.rs` 5 tests | YES | The full AC: committed SHA-256, readback == generator, staging measured (exactly 1 copy of 12288 bytes), double-run identity, hash sensitivity. Passed in baseline (orchrun :4843-4852). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "two renderers… produce byte-identical frames" | synthetic.rs:4-7 | REAL | `double_run_renders_are_byte_identical` passed (orchrun :4849). |
| "which is what makes a committed PNG hash an acceptance proof rather than a flake" | synthetic.rs:6-8 | REAL | The AC hash test passed (orchrun :4846); hash provenance independently verified per png_hash.rs:52-66 docs. |
| "Corner markers: 4×4 opaque red blocks… Grid: every 16th column and row is opaque white" | synthetic.rs:15-18 | REAL (with F-001 precedence ambiguity) | Code + test pins; precedence not stated. |
| "a single-pixel-wide axis maps the gradient to the midpoint 128" | synthetic.rs:13-14 | REAL | Closure :35-36 + test :125-146. |

## 7. Dead / duplicated / unreachable code
F-002: the generator is dead in production (test/example-only consumers). No duplication.

## 8. Dependencies & flags
`media_types` (frame/packet/time/ids), `video_engine::NANO_BASE` (declared dep, boundaries envelope "grants media-types + video-engine only" per renderer-wgpu Cargo.toml header), `crate::params::Extent2D`. No feature gates, no unsafe.

## 9. Open questions for the next agent
- png_hash.rs (not in my list): its committed hash depends on `screenshot.rs`'s encoder — any encoder change breaks it by design; the two files move together.
- The BGRA→RGBA swizzle between surface format (Bgra8Srgb) and readback (`readback_rgba`) lives in renderer.rs — the AC's value-truth assertion covers it for the synthetic case; real decoded frames are not covered by this lane (see first-vertical final-frame drift, screenshot.rs F-001).
