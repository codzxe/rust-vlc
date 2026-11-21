# AUDIT: apps/web/src/render/mod.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-21 |
| Lines / functions | 52 / 0 functions (3 `pub mod` + 3 `pub use` declarations under a 44-line module doc) |
| File verdict | REAL (live halves backend+present) with one phantom pipeline step and a dead re-export |
| Reachable from shipped entry point? | Module YES-in-crate / NO-from-shipped: `host.rs:106-107` uses `render::backend::{RenderBackend, RenderLane}` and `render::present::{PresentMailbox, PresentStats, UploadFrame}`; `lib.rs:75` re-exports the backend trio. From the shipped `lumen`/`lumen-tauri` binaries the whole `apps/web` crate is unreachable (no dependents — see host.rs#part3 audit). The canvas half is dead even in-crate (canvas.rs audit F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n` over mod.rs (full read) + lib.rs:74-76 (crate re-exports); `sed -n '85,120p' apps/web/worker.js` + `od -c` on l.113-116 (presentFrame relay verified: worker copies RGBA out of linear memory, `postMessage`s it transferred to the page); `grep -n "presentFrame\|rgba\|onmessage" ui/e2e/web/site/harness.js` (the only page-side consumer asserts message SHAPE, never draws); `rg -n "render::" apps/web/src` excluding render/ (host.rs usage enumerated); baseline: orchrun nofailfast (backend 7/7, canvas 5/5, present tests all `ok` under lumen_web 111/0) |

## 1. One-paragraph verdict
A declaration file whose value is its map: three submodules — backend negotiation (live, host-driven), the canvas seam (dead, zero callers), the present mailbox (live, host-driven) — plus honest "what this is NOT" boundaries (no wgpu, browser steps `[VERIFY]`). The map's one lie: the documented present pipeline ends "→ the JS glue's canvas draw", and no canvas draw exists anywhere in the repository — worker.js faithfully relays `presentFrame` + RGBA bytes to the page, and the only page-side consumer ever written (the e2e harness) counts messages and asserts byte lengths without drawing. The pipeline is real up to the MessagePort and phantom after it. Secondarily the file's `pub use canvas::{…}` is the wiring that keeps the dead canvas surface compiled into the artifact.

## 2. What actually works (verified)
— The declared live halves execute: `render::backend` (7/7 tests) and `render::present` (all tests) `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (lumen_web 111/0), and host.rs drives both (`render_lane.boot/surface_lost/active`, `PresentMailbox.store/take` via WebFrameConsumer — host.rs:1019, 1052, 1145-1166).
— worker.js frame relay verified present (worker.js:99-110): reads `lumen_web_frame_addr/len` before any further wasm call, `.slice()`s out of linear memory, transfers the buffer — the documented "ONE present per dispatch, valid until next dispatch" contract is honored on the JS side.

## 3. Findings

### F-001 [S3]
Where: apps/web/src/render/mod.rs:18-21 (the `present` bullet)
Code:
```rust
//! - [`present`] — the **frame-upload path** (doc 09 §4.2, row 1):
//!   decoded CPU frames → RGBA (the single allowed CPU→GPU-adjacent
//!   copy; I420 conversion uses the frame's own colorimetry) → the
```
(continues: "…latest-wins present mailbox → one `PresentFrame` message per pump slice → the JS glue's canvas draw.")
What is wrong: the chain's final step does not exist. Verified end-to-end: Rust stages and emits PresentFrame (real, test-executed); worker.js relays it with the RGBA payload to the host page (worker.js:103-109, verified); the page-side consumer — the only one in the repo — is `ui/e2e/web/site/harness.js:138-140,417-422`, which records the messages and asserts `rgba.byteLength == w*h*4`. No `drawImage`/`putImageData`/`texImage2D` call exists anywhere (rg over all JS/TS).
Why it matters: the documented render pillar implies frames reach pixels; they reach a MessagePort and stop. The e2e assertion proves bytes cross the boundary — good — but "the render lane" it names delivers frames to a void. Any claim of web video display built on this map is hollow at the last step.
What correct looks like: the doc's arrow should read "→ the page's canvas draw (unwritten — card 7.I1 `[VERIFY]`)" until a draw exists.
Confidence: HIGH — Verification: read + rg over all JS/TS for draw calls + worker.js/harness.js traced.

### F-002 [S4]
Where: apps/web/src/render/mod.rs:51
Code:
```rust
pub use canvas::{CanvasSurface, pixel_extent};
```
What is wrong: re-exports the dead canvas surface API (zero callers — canvas.rs audit F-001). `lib.rs:75` deliberately re-exports only the backend trio, so this line is the sole in-crate reference keeping the dead module's names public.
Why it matters: the public surface of the render module advertises a `CanvasSurface` nothing can obtain or use — API surface area implying capability.
What correct looks like: drop the re-export until the canvas lane wires (or gate the module).
Confidence: HIGH — Verification: read + caller enumeration (canvas.rs audit).

## 4. Per-function table (declaration rows — no functions exist)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod backend` | 46 | REAL | — | live: host.rs render_lane; 7/7 tests pass. |
| `pub mod canvas` | 47 | REAL-code/UNREACHABLE | S3/S4 context | dead module (canvas.rs F-001). |
| `pub mod present` | 48 | REAL | — | live: host.rs mailbox path. |
| `pub use backend::{…}` | 50 | REAL | — | the live re-export (also lib.rs:75). |
| `pub use canvas::{…}` | 51 | REAL-code/dead | S4 | F-002. |
| `pub use present::{…}` | 52 | REAL | — | used via `crate::render::present::` path by host.rs (path imports make this re-export itself redundant — hygiene note, no severity). |

## 5. Test honesty
No tests in this file. The submodule tests are covered in their own audits (backend: 7/7, canvas: 5/5, present: all pass — orchrun nofailfast).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "decoded CPU frames → RGBA → mailbox → one PresentFrame per pump slice" | module docs l.18-21 | REAL | host.rs present_one (one per pump) + present.rs tests + worker.js relay verified. |
| "→ the JS glue's canvas draw" | module docs l.20-21 | FALSE | no draw code exists repo-wide (F-001). |
| "DPR-aware pixel extent (the same pure arithmetic as native)" | module docs l.15-16 | REAL-UNPROVEN | canvas.rs arithmetic is real and test-pinned; "same as native" (renderer-wgpu RenderSurface) not verified — that crate is another wave's assignment and not compiled into this crate. |
| "No wgpu here … the v1 upload lane is the CPU-staged RGBA path over the MessagePort" | module docs l.31-36 | REAL | Cargo.toml confirms no wgpu dep; the CPU path is the live one. |
| "Everything on the browser side … is `[VERIFY]`" | module docs l.42-45 | honest gap | matches F-001's finding — the [VERIFY] label is present, but the pipeline map above it omits the same caveat on the draw step. |

## 7. Dead / duplicated / unreachable code
- canvas re-export (F-002) and the module it exposes (canvas.rs F-001).
- present re-export redundancy (path-imported by host.rs directly) — hygiene note only.

## 8. Dependencies & flags
- None of its own; declares the three submodules. serde enters via submodules' wire types.

## 9. Open questions for the next agent
- present.rs's auditor: does `rgba_from_video` (re-exported here) have callers beyond tests? If not, a second dead export rides this file.
- Card 7.I1's browser matrix will need to answer F-001: the harness proves bytes cross the port; nothing proves pixels.
