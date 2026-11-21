# AUDIT: apps/web/src/render/canvas.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-21 |
| Lines / functions | 439 / 13 (+5 tests) |
| File verdict | UNREACHABLE (real, tested state model; zero production callers) |
| Reachable from shipped entry point? | NO — `rg "CanvasSurface\|pixel_extent\|CanvasError\|MAX_CANVAS_EDGE\|CanvasExtent"` over apps/web/src → sole hit outside the file is the re-export `render/mod.rs:51`. The host's actual render path uses `render/backend.rs` (RenderLane) + `render/present.rs` (PresentMailbox); this module is never constructed by any production code. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "CanvasSurface\|pixel_extent\|CanvasError\|MAX_CANVAS_EDGE\|CanvasExtent" apps/web/src` (caller enumeration); `rg -rn "player.js"` repo-wide (only canvas.rs:22 — the file does not exist); `ls apps/web/` (Cargo.toml, README.md, audio-worklet.js, src, worker.js — no player.js); `grep -n "canvas\|getContext\|drawImage\|putImageData"` over ui/e2e/web/site/harness.js + index.html + apps/web/worker.js (no draw code anywhere; harness only probes `getContext("webgl2")`); arithmetic checks (16384² × 4 B = 1 GiB ✓; 100.25 × 2.0 = 200.5 → 201 ✓); baseline: `orchrun-test-workspace-nofailfast.log.txt` (5/5 canvas tests `ok`) |

## 1. One-paragraph verdict
A small, clean, correctly-arithmetic typed state model for a canvas surface that nothing uses. The DPR rounding, the typed refusal vocabulary, and the non-fatal-resize/loss semantics are all implemented and pinned by exact tests that passed in the orchestrator's no-fail-fast baseline run. But the module is production-dead: no caller constructs `CanvasSurface` or calls `pixel_extent` anywhere in the workspace — the live render path is `backend.rs` negotiation plus `present.rs`'s mailbox. Worse, the module doc asserts "The JS glue (`player.js`) owns the real canvas" — a file that does not exist anywhere in the repository, and no canvas-draw code exists at all (worker.js relays `presentFrame`; the e2e harness only probes for a webgl2 context). The "canvas seam" is a contract for a lane whose browser half was never written; the honest `[VERIFY]` labels elsewhere in the module tree say so, but this specific reference names a phantom file as the owner of real behavior.

## 2. What actually works (verified)
— `render::canvas::tests::{dpr_arithmetic_rounds_half_up, extent_refusals_are_typed, resize_is_non_fatal_and_counted, loss_is_explicit_and_permanent, construction_validates_typed}` → all `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (lumen_web 111/0).
— Arithmetic re-verified by hand: 100.25 × 2.0 = 200.5 → `f64::round` → 201 ✓; 1920 × 1.25 = 2400 ✓; 16384² × 4 B = 1 073 741 824 = 1 GiB (the doc's guard-rationale claim) ✓. `Math.round` parity holds because inputs are validated positive before rounding (JS half-up ≡ Rust half-away-from-zero on the positive quadrant).

## 3. Findings

### F-001 [S3]
Where: apps/web/src/render/canvas.rs:146-243 (whole `CanvasSurface`), 109-132 (`pixel_extent`)
Code:
```rust
pub struct CanvasSurface {
    extent: CanvasExtent,
    backend: RenderBackend,
    lost: bool,
    resizes: u64,
}
```
What is wrong: production-dead module. The only reference outside the file is `render/mod.rs:51`'s `pub use`; no code in the workspace constructs a `CanvasSurface`, resizes it, marks it lost, or presents through it. The live render path is `RenderLane` (backend.rs) + `PresentMailbox` (present.rs) — host.rs:1145-1166 presents via the mailbox, never via this surface.
Why it matters: another 200+ tested-dead lines in the size-budgeted wasm artifact; the module tree reads as if the web target has a canvas surface — it has a typed model of one with no renderer behind it.
What correct looks like: land it with the card that wires the browser canvas (7.I1), or gate it out of the shipped build.
Confidence: HIGH — Verification: read + rg caller enumeration.

### F-002 [S3]
Where: apps/web/src/render/canvas.rs:21-23
Code:
```rust
//! There is no DOM here: the wasm module cannot touch
//! `HTMLCanvasElement`. The JS glue (`player.js`) owns the real
//! canvas and reports resize/loss through the typed messages; this
```
What is wrong: `player.js` does not exist. `rg "player.js"` over the entire repo (outside audit/) returns exactly one hit — this line. `apps/web/` contains only `worker.js`, `audio-worklet.js`, `README.md`, `Cargo.toml`, `src/`. No canvas-draw code exists anywhere: `worker.js` relays `presentFrame` messages onward, and the e2e harness (`ui/e2e/web/site/harness.js:160-161`) creates a canvas only to probe `getContext("webgl2")` for capability facts.
Why it matters: the doc names a phantom file as the owner of the real surface behavior — present-tense assertion of glue nobody wrote. This is the same defect class the project's own `xtask feature-audit` flags as a violation (evidence strings citing missing paths, baseline INDEX row 8). A reader tracing "who resizes the canvas" lands on nothing.
What correct looks like: either the file exists (write it, card 7.I1) or the doc says "the page glue (unwritten, card 7.I1 [VERIFY])".
Confidence: HIGH — Verification: read + repo-wide rg + directory listing.

### F-003 [S4]
Where: apps/web/src/render/canvas.rs:231-242
Code:
```rust
    /// Present one frame (the Fifo-shaped contract: returns with
    /// the frame logically on the surface).
    ///
    /// # Errors
    /// [`CanvasError::SurfaceLost`] once lost — the typed degrade
    /// the render lane surfaces as the audio-only event.
    pub fn present(&mut self) -> Result<(), CanvasError> {
```
What is wrong: `present` takes no frame, stages nothing, and draws nothing — its body is a lost-flag check returning `Ok(())`. "Returns with the frame logically on the surface" describes a contract with no frame in its signature.
Why it matters: a name and doc that promise frame presentation over a no-op success; a future caller wiring this "surface" would believe frames reach it. The real upload lane is `PresentMailbox`/`UploadFrame` in present.rs.
What correct looks like: rename to `assert_presentable()` / `present_gate()`, or give it the frame parameter and the actual staging semantics.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pixel_extent` | 109-132 | REAL (dead) | — | validation order correct (finite/positive CSS → DPR → zero → ceiling); `as u32` casts provably in-range (post-check 1.0..=16384.0); saturating f64 casts for adversarial inputs land in the typed refusals first. |
| `CanvasError::as_str` | 78-86 | REAL (dead) | — | 5-arm exhaustive sentences. |
| `Display for CanvasError` | 89-93 | REAL (dead) | — | delegates. |
| `CanvasSurface::new` | 160-173 | REAL (dead) | — | validates zero + ceiling; same vocabulary as resize (test-pinned "no drift"). |
| `CanvasSurface::extent` | 177-179 | REAL (dead) | — | getter. |
| `CanvasSurface::backend` | 183-185 | REAL (dead) | — | getter. |
| `CanvasSurface::format` | 191-193 | REAL (dead) | — | returns the literal "rgba8" — a string, not a typed format (model-level). |
| `CanvasSurface::is_lost` | 197-199 | REAL (dead) | — | getter. |
| `CanvasSurface::resize_count` | 203-205 | REAL (dead) | — | getter. |
| `CanvasSurface::resize` | 213-223 | REAL (dead) | — | lost → typed SurfaceLost; refusal keeps previous extent (test-pinned); saturating counter. |
| `CanvasSurface::mark_lost` | 227-229 | REAL (dead) | — | idempotent, no un-loss (test-pinned). |
| `CanvasSurface::present` | 237-242 | REAL (dead, defect) | S4 | F-003: no-op body under a frame-shaped doc. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `dpr_arithmetic_rounds_half_up` | YES | four exact extents incl. the 200.5→201 rounding pin and fractional-DPR 1.25×. |
| `extent_refusals_are_typed` | YES | enumerates NaN/±inf/zero/negative CSS, bad DPR, sub-pixel zero, ceiling 16385 vs legal 16384 — each refuses its exact variant. |
| `resize_is_non_fatal_and_counted` | YES | refusal-keeps-extent + refusal-does-not-count asserts; format/backend getters pinned. |
| `loss_is_explicit_and_permanent` | YES | present-after-loss, resize-after-loss, frozen-extent postmortem, fresh-surface recovery — each asserted. |
| `construction_validates_typed` | YES | zero and oversized construction refuse the same variants as resize. |
All five → `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "canvas.width = round(cssWidth × devicePixelRatio)" | module docs l.11-12 (doc 09 §4.3) | REAL | pixel_extent l.120-121 + passing exact tests. |
| "The JS glue (`player.js`) owns the real canvas" | module docs l.22-23 | FALSE | no player.js exists (rg repo-wide); no canvas-draw code exists (F-002). |
| "Real-canvas behavior … is `[VERIFY]`" | module docs l.25-27 | honest gap | no baseline browser run (rc-smoke web stages skipped). |
| "resize is non-fatal by construction" | module docs l.8, 139-141 | REAL | refusal-keeps-extent pinned by passing test. |

## 7. Dead / duplicated / unreachable code
- The entire module (F-001).
- Duplication risk: extent validation logic exists twice (`pixel_extent` post-round checks vs `CanvasSurface::new` u32 checks) — deliberately, one vocabulary, test-pinned as equal.

## 8. Dependencies & flags
- serde (Serialize/Deserialize on the wire types — the only reason this file touches a non-std crate) + super::backend::RenderBackend. No feature gates.

## 9. Open questions for the next agent
- present.rs auditor (this wave): confirm the mailbox/upload lane is the only present path the host drives (host.rs:1145-1166) — F-001's caller enumeration depends on it.
- Whoever owns card 7.I1: the phantom `player.js` reference (F-002) should be resolved with the browser lane — either the file or the doc correction.
