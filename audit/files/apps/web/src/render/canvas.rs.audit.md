# AUDIT: apps/web/src/render/canvas.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-30 |
| Lines / functions | 439 / 9 fns + 5 tests |
| File verdict | STUB as a render surface (present() does nothing); REAL as unused pure arithmetic — module is dead code |
| Reachable from shipped entry point? | NO — zero non-test callers of `CanvasSurface`/`pixel_extent`/`CanvasError`/`CanvasExtent` anywhere in the repo |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `cargo test -p lumen-web render::canvas` → 5 passed, exit 0; rg reachability sweeps; `node --check apps/web/worker.js` → exit 0 |

## 1. One-paragraph verdict
There is NO canvas render path in the web build, and this file is the proof: `CanvasSurface::present()` is a lost-flag check that draws nothing, takes no frame, and returns Ok — a typed no-op dressed as a present contract. Repo-wide, nothing constructs a `CanvasSurface` or calls `pixel_extent` (the only hits are this file, its tests, and the mod.rs re-export). The real web frame lane (present.rs mailbox → `PresentFrame` message → worker.js `postMessage`) ends at the page boundary with **no consumer**: the UI mounts a canvas only inside the desktop Tauri shell (`__TAURI_INTERNALS__` probe), and in a plain browser deliberately mounts none. The DPR arithmetic itself is correct, tested, and pointless.

## 2. What actually works (verified)
— verified by `cargo test -p lumen-web render::canvas` → 5 passed, exit 0 (rounding pin `100.25×2.0→201`, fractional DPR, typed refusals incl. NaN/∞/sub-pixel/ceiling, non-fatal resize, permanent loss, construction parity).
— Baseline: cargo-test-workspace.txt:3324-3451 `### PACKAGE lumen-web exit_code=0`.
— `node --check apps/web/worker.js` → exit 0 (the frame relay parses; an earlier suspicion of a syntax error was a display artifact, disproven by hexdump — bytes at offset 5052 are `messages[messages.length - 1]`).

## 3. Findings
F-001 [S2]
Where: apps/web/src/render/canvas.rs:231-242
Code:
```
/// Present one frame (the Fifo-shaped contract: returns with
/// the frame logically on the surface).
...
pub fn present(&mut self) -> Result<(), CanvasError> {
    if self.lost {
        return Err(CanvasError::SurfaceLost);
    }
    Ok(())
}
```
What is wrong: `present` accepts no frame, touches no buffer, no canvas, no GPU state — it implements the "Fifo-shaped present contract" as `if lost { Err } else { Ok }`. The doc sentence "returns with the frame logically on the surface" is false: there is no frame and no surface, logically or otherwise. This is the STUB at the heart of the "canvas seam". Why it matters: the web lane's advertised render surface (doc 09 §4.3) is a boolean flag; any embedder trusting this type believes frames are being presented when nothing happens. What correct looks like: present takes the RGBA staging buffer and the extent and feeds the frame lane (present.rs already owns that path — this type should either wrap it or be deleted). Confidence: HIGH — Verification: read + ran its tests (they pass because they only assert the flag).

F-002 [S2]
Where: apps/web/src/render/canvas.rs:1-27 (module doc) vs apps/web/src/host.rs:1046-1052
Code (canvas.rs:16-17):
```
//!   the [`CanvasError::SurfaceLost`] vocabulary the JS glue's
//!   `webglcontextlost` / `device.lost` reports drive.
```
Code (host.rs:1046-1052):
```
HostToWorker::SurfaceLost { backend, reason } => {
    ...
    if let Some(transition) = self.render_lane.surface_lost(backend, reason) {
```
What is wrong: the loss reports the JS glue actually sends are consumed by `backend.rs::RendererNegotiation::surface_lost`, not by `CanvasSurface::mark_lost` (rg: `mark_lost` has zero non-test callers). The module's headline claim — that this type is the surface the glue's loss reports drive — is contradicted by the crate's own dispatch. The entire module is unwired: `rg 'CanvasSurface|pixel_extent'` across the repo returns only canvas.rs, its tests, and render/mod.rs:51's re-export. Why it matters: two parallel "surface" vocabularies exist (canvas.rs `CanvasError` and backend.rs's negotiation transitions); the dead one is the one named "the canvas seam". What correct looks like: delete canvas.rs or route the surfaceLost/resize messages through it. Confidence: HIGH — Verification: read + rg.

F-003 [S3]
Where: apps/web/src/render/canvas.rs:191-193
Code:
```
pub fn format(&self) -> &'static str {
    "rgba8"
}
```
What is wrong: a constant string accessor named `format` on a surface that knows its `RenderBackend` — WebGL2 and WebGPU upload layouts differ (the UI's own presenter swizzles BGRA, MediaSurface.tsx:215-220), so a backend-independent constant is wrong the day this becomes live; today it is dead stringly-typed decoration. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/web/src/render/canvas.rs:201-205
Code:
```
/// How many resizes were applied (the ResizeObserver evidence).
#[must_use]
pub fn resize_count(&self) -> u64 {
    self.resizes
}
```
What is wrong: an "evidence" counter nothing reads (resize is never called outside tests). Confidence: HIGH — Verification: rg.

F-005 [S4]
Where: apps/web/src/render/canvas.rs:36-38 (doc)
Code:
```
/// 16384 is the classic WebGL2
/// `MAX_TEXTURE_SIZE` floor across browsers `[VERIFY per browser]`;
```
What is wrong: the ceiling guard is honest but unverified per browser, and it is never enforced anywhere except this dead module — the actual upload lane (present.rs/abi.rs) has its own bounds; a 16384-edge guard that no shipped path consults protects nothing. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (>100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CanvasError::as_str / Display | 75-93 | REAL | — | dead but correct sentences |
| pixel_extent | 109-132 | REAL | — | correct validated rounding; test-pinned; zero callers |
| CanvasSurface::new | 160-173 | REAL | — | validates extent; zero non-test callers |
| CanvasSurface::extent/backend | 177-185 | REAL | — | dead accessors |
| CanvasSurface::format | 191-193 | STUB | S3 | constant "rgba8" (F-003) |
| CanvasSurface::is_lost/resize_count | 197-205 | REAL | S4 | dead flag/counter readouts (F-004) |
| CanvasSurface::resize | 213-223 | REAL | — | non-fatal semantics correct; never driven by real resize messages |
| CanvasSurface::mark_lost | 227-229 | REAL | S2 | never called by the loss path (F-002) |
| CanvasSurface::present | 237-242 | STUB | S2 | no-op (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| dpr_arithmetic_rounds_half_up | YES | exact-value assertions incl. the 200.5→201 pin (ran: ok) |
| extent_refusals_are_typed | YES | NaN/∞/0/negative/sub-pixel/ceiling all asserted (ran: ok) |
| resize_is_non_fatal_and_counted | YES | refusal-keeps-extent + count assertions (ran: ok) |
| loss_is_explicit_and_permanent | YES | present/resize-after-loss typed errors (ran: ok) — but cannot detect the module is dead |
| construction_validates_typed | YES | zero/oversized construct refusals (ran: ok) |

## 6. Doc claims touching this file — THE CANVAS ANSWER
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "canvas.width = round(cssWidth × devicePixelRatio)" arithmetic exists for the web lane | canvas.rs:11-13 | REAL but UNREACHABLE | pixel_extent correct (tests); zero callers |
| "the JS glue's webglcontextlost / device.lost reports drive" CanvasError::SurfaceLost | canvas.rs:16-17 | FALSE | host.rs:1046-1052 routes loss to backend.rs; mark_lost uncalled |
| "Present one frame … logically on the surface" | canvas.rs:231-232 | STUB | present() body is a flag check (F-001) |
| Web build renders decoded frames to a canvas | render/mod.rs:21-23 ("the JS glue's canvas draw") | FALSE (no drawer exists) | worker.js:95-100 posts presentFrame+rgba; rg `texImage2D|drawImage|putImageData` in apps/web + worker + ui browser path → only MediaSurface.tsx (shell-only) has putImageData; MediaSurface.test.tsx:140-151 pins "browser mode: no canvas" (`expect(document.querySelector("canvas")).toBeNull()`) |
| e2e: "presentFrame messages arrived (the render lane)" | ui/e2e/web/site/harness.js:417 | REAL (messages) / NOT pixels | counts postMessages; asserts `rgba.byteLength` only — no draw ever asserted |

**Blunt answer to the wave question:** the web build has NO canvas render path — the chain decode→RGBA→mailbox→PresentFrame→worker postMessage terminates in a message nobody draws. The DESKTOP shell is the opposite of the wave's rumor: it HAS a real canvas path (`session.set-frame-channel` intercepted at apps/desktop/src/shell.rs:160-162, LUF1 records blitted via 2D putImageData in ui/src/components/player/MediaSurface.tsx:198-228, test-pinned "renders decoded RGBA frames into the canvas"). The web lane and the desktop lane are two disjoint frame channels; canvas.rs belongs to neither.

## 7. Dead / duplicated / unreachable code
The entire module is unreachable (F-002): compiled into lumen-web, referenced by nothing. Two parallel surface-loss vocabularies (canvas.rs CanvasError vs backend.rs transitions) — the dead one carries the name.

## 8. Dependencies & flags
serde (Serialize/Deserialize on CanvasExtent/CanvasError — serde round-trips exist only for a type nothing sends), testkit (dev). No features. No unsafe.

## 9. Open questions for the next agent
- present.rs (875 LOC, not my file): does the PresentMailbox→PresentFrame path survive into the shipped wasm exports, and does the e2e web lane (web-matrix.spec.ts) run in any gate? Baseline parity-report says web 0/252 verified.
- If canvas.rs is retained as "the seam wgpu will slot into" (render/mod.rs:29-33), that is a [PROPOSAL], not an implementation — the wave-0 UNVERIFIED list should carry it.
