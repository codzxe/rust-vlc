# AUDIT: crates/subtitles/src/compositing/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-21 |
| Lines / functions | 37 / 0 functions — 5 `pub mod` declarations, 3 re-export groups, module-map doc |
| File verdict | REAL (wiring correct, re-exports complete) / pipeline it maps is UNREACHABLE (the "engine glue" caller does not exist) |
| Reachable from shipped entry point? | NO — crate-level (see lib.rs audit header). In-crate: `SubtitleCompositor` (the module's driver type) is constructed ONLY by `crates/subtitles/tests/compositing_ac.rs` (8 sites) and `ass/render.rs`'s end-to-end test (:867). Zero production callers. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `grep -n "^pub struct\|^pub enum\|^pub fn\|^pub trait" compositing/{layout,target,timing}.rs` (every re-exported name verified present at source); `rg "SubtitleCompositor" crates/ apps/` (test-only callers, listed above); Read mod.rs + compose.rs head; baseline `orchrun-test-workspace-nofailfast.log.txt:5569-5581` (compositing_ac 8/8). `sed -n` quote verify. |

## 1. One-paragraph verdict
A clean 37-line module map whose five submodule descriptions match the submodule docs I read while auditing bitmap.rs (placement/z/timing/target/compose), and whose re-exports are complete — all 14 re-exported names exist as `pub` at their sources (verified item by item). The single defect is inherited from the crate root: the doc's operative sentence describes a per-frame pipeline driven by "engine glue, a later card" that was never written, so the mapped pipeline runs only under `cargo test`.

## 2. What actually works (verified)
— Re-export check (this file's only code): `bitmap::{CompositedCue, CueBitmap, CueError}`, `compose::SubtitleCompositor`, `layout::{Anchor, CueAlignment, CueLayout, HorizontalAlign, Rational, Rect, SafeMargin, VerticalAlign, place_cue}`, `target::{ComposeError, ComposeTarget, Rgba, Rgba8Surface}` — all present at source (layout.rs:32,79,114,126,143,152,182,195,264; target.rs:26,77,136,155; bitmap.rs; compose.rs). Nothing re-exported that doesn't exist; nothing load-bearing left un-exported (`timing::active_at` is reached via `CompositedCue::active_at`, `blend_row`/`clip` are compose-internal — consistent with the map).
— Module-map descriptions cross-checked against the submodules read for my bitmap.rs audit: placement/safe-areas (layout), payloads (bitmap), half-open rule (timing), seam + reference surface (target), driver (compose) — accurate.

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/src/compositing/mod.rs:4-11
Code:
```
//! Pipeline shape: a caller (engine glue, a later card) holds decoded
//! cues as [`CompositedCue`]s — an RGBA8 [`CueBitmap`] carrying its
//! baseline row, plus a [`CueLayout`] (anchor, alignment, safe margin,
//! reference extent) and a z-order key. Per presented video frame it
//! calls [`SubtitleCompositor::compose`] with the clock's `now()`;
```
What is wrong: the "engine glue, a later card" caller does not exist anywhere in the tree. `SubtitleCompositor` — the type this entire module map funnels into — is constructed only by `crates/subtitles/tests/compositing_ac.rs` (8 call sites) and the ass render test (ass/render.rs:867). No playback-engine leg, no renderer-wgpu adapter, no CLI path (censuses in lib.rs audit header and render.rs#part2 audit header).
Why it matters: the doc's "per presented video frame it calls …" describes a frame loop that never executes outside tests; the module map documents the exact contract the missing integration card would satisfy, which makes it useful (as a spec) and misleading (as a description of running code) at once.
What correct looks like: one clause — "no engine glue exists yet; the only current driver is the test suite" — mirroring compose.rs's own honest "the engine glue's job once subtitle frames flow" (:6-7 of that file).
Confidence: HIGH — Verification: read + rg census.

## 4. Per-function table (declarations)
| Declaration | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod bitmap` | 25 | REAL | — | audited by me (S4:4) |
| `pub mod compose` | 26 | REAL | — | A05's file; driver, test-only callers |
| `pub mod layout` | 27 | REAL | — | A05/A07 files |
| `pub mod target` | 28 | REAL | — | A03's file; the seam + reference surface |
| `pub mod timing` | 29 | REAL | — | A07's file; `active_at` half-open rule |
| `pub use bitmap::{…}` | 31 | REAL | — | verified complete |
| `pub use compose::SubtitleCompositor` | 32 | REAL | — | verified |
| `pub use layout::{…}` | 33-36 | REAL | — | all 9 names verified at source |
| `pub use target::{…}` | 37 | REAL | — | all 4 names verified at source |

## 5. Test honesty
No tests in this file. The submodule suite: bitmap 3/3 + compositing_ac 8/8 in baseline (`orchrun-test-workspace-nofailfast.log.txt:5347-5349, :5569-5581`).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a caller (engine glue, a later card) holds decoded cues" | mod.rs:4-5 | FALSE (caller never landed) | F-001 census |
| "the compositor filters by the half-open [t0,t1) interval, sorts by z, computes each cue's destination rectangle and blends onto the renderer's rows" | mod.rs:9-11 | REAL (in-crate, test-proven) | compositing_ac 8/8 incl. z-order + pixel-exact seams; compose.rs read |
| Module map (5 bullets) | mod.rs:13-23 | REAL | cross-checked vs submodules (§2) |

## 7. Dead / duplicated / unreachable code
Nothing dead in-file. The mapped pipeline is unreachable from shipped builds (crate-level, lib.rs audit).

## 8. Dependencies & flags
None beyond the crate's single `media-types` edge. No unsafe, no features.

## 9. Open questions for the next agent
- A05 (compose.rs): your "zero allocation per frame after warm-up" AC — the compositing_alloc test file exists (tests/compositing_alloc.rs); confirm it pins the steady state with the counting allocator, since mod.rs's "persistent-scratch" framing leans on it.
