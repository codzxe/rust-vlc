# AUDIT: apps/web/src/render/mod.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-14 |
| Lines / functions | 52 / 0 (module hub: 3 `pub mod`, 3 `pub use`, docs only) |
| File verdict | REAL as a hub; its PIPELINE DOC is FALSE at the last step ("the JS glue's canvas draw" does not exist) |
| Reachable from shipped entry point? | Same lane answer as its children: compiled into the lumen-web wasm artifact (rc-smoke build, baseline); the product UI never loads the artifact. Within the lane: present half wired (host.rs:271/292 PresentMailbox), backend half wired (host.rs:847/1019/1052/1146), canvas half DEAD (zero consumers). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | Read of mod.rs (full); rg consumers of every re-exported name over apps/web/src; Read of worker.js (glue side); harness.js present section. |

## 1. One-paragraph verdict
A clean 52-line module hub whose three children split cleanly into two wired halves and one dead one: `present` feeds the ABI frame lane through a host-owned mailbox (real, tested), `backend` drives the capability snapshot and present gating (real, tested), and `canvas` — the "canvas seam" — is a model with zero consumers, exactly like the audio sink. The module doc, however, narrates a complete pipeline ending in "the JS glue's canvas draw": no such draw exists anywhere (worker.js transfers RGBA bytes out of the worker; the e2e harness asserts `byteLength` and never draws; `drawImage`/`texImage2D` have zero hits in the repository). The doc also lists "the WebGL2 texture upload, `requestVideoFrameCallback` pacing" as browser-side `[VERIFY]` items — code that awaits verification — when the truth is stronger: that code was never written.

## 2. What actually works (verified)
— The re-export surface is accurate: `RenderBackend`/`RendererCaps`/`RendererNegotiation` (consumed by lib.rs:75, host.rs), `PresentMailbox`/`UploadFrame` (host.rs:107, 271, 292). Module compiles in the shipped wasm lane (baseline rc-smoke build; clippy clean per baseline).
— The honest-boundary paragraph (26-34) is TRUE: no wgpu dependency exists in apps/web (Cargo.toml verified — serde/serde_json only), and the seam discipline it claims matches what backend.rs/present.rs implement.

## 3. Findings
F-001 [S3]
Where: apps/web/src/render/mod.rs:18-22
Code:
```
//! - [`present`] — the **frame-upload path** (doc 09 §4.2, row 1):
//!   decoded CPU frames → RGBA (the single allowed CPU→GPU-adjacent
//!   copy; I420 conversion uses the frame's own colorimetry) → the
//!   latest-wins present mailbox → one `PresentFrame` message per
//!   pump slice → the JS glue's canvas draw.
```
What is wrong: the pipeline's terminal step — "the JS glue's canvas draw" — does not exist IN THIS LANE. worker.js (the JS glue this module pair drives) does no drawing: it reads the frame lane, `.slice()`s the bytes, posts them to the page (worker.js:96-108); the only page-side consumer (ui/e2e/web/site/harness.js:417-423) asserts `frame.rgba.byteLength === w*h*4` and moves on. The one real canvas draw in the repository belongs to a DIFFERENT lane: `presentLuf1Frame` (2D `putImageData`) in ui/src/components/player/MediaSurface.tsx — the desktop-shell frame channel (w31.4), explicitly gated on `__TAURI_INTERNALS__` and documented "In a plain browser it NEVER renders (the same `__TAURI_INTERNALS__` presence probe the transport uses) — the honest empty surface stands." So in a plain browser — the apps/web target — no code anywhere draws a presented frame: the bytes reach a counter (harness) or nobody (product UI). The doc chain implies pixels reach a canvas; in the web lane they never do. Why it matters: this is the module-level version of the backend.rs finding — the render lane is described end-to-end while the last mile is absent, and every reader (including the doc-09 conformance story) inherits the false completion. What correct looks like: "→ the JS glue's frame transfer (the page owns the draw — unwired in v1's browser lane; the desktop shell draws via its own frame channel)". Confidence: HIGH — Verification: read + repo-wide rg (`drawImage|texImage2D|putImageData` -> only MediaSurface.tsx + its test, desktop-gated).

F-002 [S3]
Where: apps/web/src/render/mod.rs:15-17 vs zero consumers
Code:
```
//! - [`canvas`] — the **canvas seam** (doc 09 §4.3): DPR-aware pixel
//!   extent (the same pure arithmetic as native), non-fatal resize,
//!   and the typed surface-lost vocabulary.
```
What is wrong: the canvas seam is never instantiated by anything: `rg "CanvasSurface|pixel_extent" apps/web/src --glob '!render/*'` -> 0 hits (only canvas.rs's own tests and this mod's re-export). Like `WorkletQueue` in the audio tree, it is a tested model with no wiring — the host never owns a surface, resizes are never applied, `mark_lost` is never called by the surfaceLost path (host.rs:1052 goes straight to the backend lane's `surface_lost`). The mod doc lists it as one of the three halves of "the typed render-surface seam" without marking it unwired. Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: apps/web/src/render/mod.rs:40-44
Code:
```
//! side — the actual `navigator.gpu` probe, the WebGL2 texture
//! upload, `requestVideoFrameCallback` pacing, device loss — is
//! `[VERIFY]` (no browser in this sandbox; card 7.I1's matrix owns
//! those rows).
```
What is wrong: "the WebGL2 texture upload" and "requestVideoFrameCallback pacing" are listed as implemented-but-unverified browser-side behavior. They are not implemented at all — there is no WebGL texture upload code and no rVFC pacing code in the repository (`rg texImage2D|requestVideoFrameCallback` -> 0 code hits; the only canvas presenter is the desktop lane's 2D putImageData). The [VERIFY] label is being stretched from "runtime behavior we could not exercise" to "code that does not exist", which is exactly the inflation the label system exists to prevent. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
Not applicable (no functions; < 100 LOC). Inventory: `pub mod backend/canvas/present` + three `pub use` lines — all three children audited or cross-referenced (backend.rs: this wave, mine; canvas.rs/present.rs: other agents; consumer wiring verified here).

## 5. Test honesty
No tests in this file. Child-test evidence cited from baseline: backend 7/7, present/canvas suites pass (cargo-test-workspace.txt lumen-web block).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one `PresentFrame` message per pump slice" | mod.rs:21-22 | REAL | host.rs present_one: one drain per pump; abi frame-lane test pins the bytes |
| "the JS glue's canvas draw" | mod.rs:22 | FALSE for the web lane | F-001: worker.js transfers, harness counts; the repo's only draw (presentLuf1Frame) is desktop-shell-gated and "NEVER renders" in a plain browser |
| "canvas — the canvas seam ... non-fatal resize" | mod.rs:15-17 | REAL (model) / UNREACHABLE (wiring) | F-002: zero consumers |
| "No wgpu here ... this crate is zero-dep by card" | mod.rs:26-28 | REAL | apps/web/Cargo.toml: serde + serde_json only |
| "the WebGL2 texture upload, rVFC pacing ... [VERIFY]" | mod.rs:41-43 | INFLATED | F-003: absent code, not untested code |

## 7. Dead / duplicated / unreachable code
`pub use canvas::{CanvasSurface, pixel_extent}` — dead re-export (zero consumers). The backend/present re-exports are live.

## 8. Dependencies & flags
None of its own (std only); children: serde (backend), std (canvas/present). No feature gates.

## 9. Open questions for the next agent
- present.rs auditor: the mailbox's latest-wins displacement + `busy()`'s `present_pending()` tail (host.rs:1366) — if the mailbox can hold a frame with the pump ended, the busy tail keeps the loop alive; confirm the end-of-media ordering cannot strand a frame.
- canvas.rs auditor: whether ANY doc/matrix row cites the canvas seam as evidence of web rendering (this file's F-001/F-002 make that citable-but-false).
