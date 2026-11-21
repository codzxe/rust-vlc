# AUDIT: apps/desktop/prototype/lumen-frames.mjs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-21 |
| Lines / functions | 451 / 22 (1 const table, 1 error class, 1 helper fn, 4 classes with 14 methods, 3 free fns, 1 re-export) |
| File verdict | UNREACHABLE (real, working reference code; no shipped entry point loads it) — the parser/encoder mirror of wire.rs is faithful (verified by direct cross-read), but the claimed conformance harness does not exist in the repo and the WebGL2 arm renders BGRA frames channel-swapped |
| Reachable from shipped entry point? | NO — `apps/desktop/prototype/` is loaded by neither the `lumen` CLI nor `lumen-tauri` (grep: the only consumers of the filename are comments — apps/desktop-tauri/src/frame_channel.rs:21, ui/src/client/frame-channel.ts:11, apps/desktop/tests/protocol.rs:5, docs). The tauri shell itself is UNVERIFIED-BUILD (baseline #2). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `Read` lumen-frames.mjs 1-451 (full); `ls apps/desktop/prototype/` (ONLY lumen-frames.mjs — no harness.mjs); `find . -name "harness.mjs"` (NO RESULTS repo-wide); `rg -ln "lumen-frames\|harness.mjs"` (reference sites: README.md:36, docs/prototypes/video-in-webview.md:52, wire.rs/examples/emit_stream.rs headers); `rg -n "MAGIC\|VERSION\|MIN_HEADER\|MAX_HEADER\|MAX_PAYLOAD\|MAX_DIMENSION" apps/desktop/src/wire.rs` (constants identical: LUF1/1/48/1024/1<<30/65535); `sed -n '620,700p'` + `sed -n '196,232p'` + `sed -n '600,625p'` apps/desktop/src/wire.rs (parse order, validate_geometry, finish semantics — all mirrored faithfully); `sed -n '28,60p' crates/renderer-wgpu/src/synthetic.rs` (gradient/corner/grid rules match; NO marker write in Rust); `rg "0xee\|0x77\|marker" crates/renderer-wgpu/src/synthetic.rs` (marker absent). |

## 1. One-paragraph verdict
The wire-protocol half of this reference module is genuine and, despite its proof harness being missing, demonstrably faithful to `apps/desktop/src/wire.rs`: same constants (LUF1, v1, header 48..=1024, payload cap 1 GiB, dims 1..=65535), same validation order (magic→header_len→version→format→flags→reserved→zero-dims→dim-cap→payload-cap→length-match), same error reasons verbatim ("zero dimension", "dimension exceeds 65535", "payload exceeds the 1 GiB cap"), same finish/truncated semantics, and a JS-specific safe-integer guard where f64 meets u64 — I verified the mirror by direct cross-read since the cited conformance harness (`prototype/harness.mjs`) does not exist anywhere in the repo while the file header, `apps/desktop/README.md:36` and `docs/prototypes/video-in-webview.md:52` all instruct running it. The presenter half has a real defect: the BGRA→RGBA swizzle runs only on the 2D fallback arm, so the PRIMARY WebGL2 backend uploads BGRA payloads as `gl.RGBA` textures — red and blue swapped on every BGRA frame, with an in-code "[VERIFY on desktop]" admitting the unverified upload path — and the `drawn` counter increments without any GL error check, so `status()` can report frames drawn on a black canvas. Nothing shipped loads this file; its value is as the documented reference for the ui/ TypeScript port and the tauri frame channel, both of which cite it.

## 2. What actually works (verified)
— Verified by direct cross-read against `apps/desktop/src/wire.rs` (constants 70-89, parse_header 625-686, validate_geometry 208-231, finish 607-620): the JS `parseFrameHeader`/`FrameDecoder`/`finish` mirror the Rust parser field-for-field, order-for-order, message-for-message; the "BYTE FOR BYTE" header claim (line 5) holds for the decoder.
— Verified by cross-read of `crates/renderer-wgpu/src/synthetic.rs:29-53`: `syntheticPattern`'s gradient formula (`n<=1 → 128` else `(i*255+(n-1)/2)/(n-1)`), 4×4 red corners, every-16th white grid, `(x+y)%256` blue channel all match — except the marker (F-005).
— Verified by read: `ByteQueue` grow/compact arithmetic is sound (compaction before growth preserves live bytes; `view` lifetimes are respected because `nextFrame` copies via `.slice` before `skip`).

## 3. Findings

### F-001 [S3]
Where: apps/desktop/prototype/lumen-frames.mjs:5-6; broken instruction sites apps/desktop/README.md:36, docs/prototypes/video-in-webview.md:52
Code:
```js
// This module mirrors apps/desktop/src/wire.rs BYTE FOR BYTE (the
// conformance proof is prototype/harness.mjs over the Rust-emitted
// golden stream). It is deliberately dependency-free and
```
What is wrong: The cited conformance harness `prototype/harness.mjs` does not exist — `apps/desktop/prototype/` contains only `lumen-frames.mjs`, and a repo-wide `find` returns nothing. Three documents instruct running it (`node prototype/harness.mjs target/proto`, README:36; `node apps/desktop/prototype/harness.mjs`, video-in-webview.md:52) — every one fails with "Cannot find module".
Why it matters: The cross-language (Rust→JS) conformance claim has no runnable proof in the tree; a reader cannot reproduce the "byte for byte" guarantee the way the docs tell them to. (I restored the guarantee by hand-comparison — F-001 is about the broken evidence path, not a parser divergence.)
What correct looks like: commit the harness (golden stream from `examples/emit_stream.rs` + diff against the JS decoder), or fix the three references to describe the actual verification (Rust-side protocol.rs round-trip + this audit's cross-read).
Confidence: HIGH — Verification: read + find/ls (file absent).

### F-002 [S3]
Where: apps/desktop/prototype/lumen-frames.mjs:372-391
Code:
```js
    if (frame.header.formatCode === PROTOCOL.FORMAT_BGRA8 && backend === "2d") {
      // The 2D path takes RGBA; BGRA frames swizzle draw-side
      // (recorded cost; WebGL2 uploads BGRA natively where the
      // extension exists [VERIFY on desktop]).
      swizzleBgraToRgba(frame.payload);
    }
    if (backend === "webgl2") {
```
What is wrong: The swizzle is gated to the 2D backend only. The WebGL2 arm then calls `gl.texImage2D(..., gl.RGBA, ..., gl.RGBA, gl.UNSIGNED_BYTE, frame.payload)` (381-391) with the un-swizzled BGRA bytes — WebGL2 core has no BGRA texture format, so the bytes are interpreted as RGBA and red/blue are swapped on screen. The comment's claim "WebGL2 uploads BGRA natively where the extension exists" is not backed by any code: no extension is queried, no branch exists.
Why it matters: WebGL2 is the fallback chain's PRIMARY backend (WebGPU is a typed stub, 314-316); every FORMAT_BGRA8 frame — a first-class wire format the parser accepts — renders with wrong colors on the main path. The "[VERIFY on desktop]" marker makes it honest, not correct.
What correct looks like: swizzle for the WebGL2 arm too (or upload to an RGBA texture via a swizzling shader; or detect and refuse BGRA on backends without a native path — the typed-refusal pattern the rest of the repo uses).
Confidence: HIGH — Verification: read (WebGL2 BGRA-unavailability is spec-level knowledge; the code demonstrably never swizzles or checks on this arm).

### F-003 [S4]
Where: apps/desktop/prototype/lumen-frames.mjs:335-347, 405
Code:
```js
    gl.linkProgram(prog);
    gl.useProgram(prog);
```
(…and `this.drawn += 1;` at 405 after `drawArrays`.)
What is wrong: `initWebgl2` never checks `getShaderParameter(COMPILE_STATUS)`/`getProgramParameter(LINK_STATUS)`, and `draw` never checks `gl.getError()`; `this.drawn` counts draw calls, not successful presents. On a context-lost or compile-failed canvas the module reports `backend=webgl2 … frames drawn=N` over a black surface.
Why it matters: `status()` is documented as "the truthful one-line capability report" (328-329) — the counter can lie about the only thing it reports.
What correct looks like: check compile/link status (throw `WireError("no-backend", …)` on failure), query `gl.getError()` in `draw`, count only clean presents.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/desktop/prototype/lumen-frames.mjs:242-263
Code:
```js
export function encodeFrame(header, payload) {
  const implied = header.width * header.height * 4;
  if (payload.length !== implied) {
    throw new WireError("payload-length-mismatch", `payload ${payload.length} != w*h*4 (${implied})`);
  }
```
What is wrong: The JS encoder validates only payload length. The Rust encoder runs the full `validate_geometry` ("shared by encode and decode paths — one rule set, no drift", wire.rs:207-209): zero dimensions, the 65535 cap, and the 1 GiB cap. The JS encoder happily emits `width: 0` (payload 0) or `width: 70000` records its own decoder (and the Rust one) refuses. Additionally `setBigUint64/setBigInt64` (258-260) throw an untyped `RangeError` for values beyond 64 bits — not a `WireError`.
Why it matters: The mirror's "one rule set, no drift" property is broken at the encode seam; a JS-side generator (the standalone page) can produce streams the protocol rejects.
What correct looks like: run the same geometry checks in `encodeFrame` (a shared `validateGeometry(header)` helper), and range-check the BigInts with a typed error.
Confidence: HIGH — Verification: read + wire.rs cross-read.

### F-005 [S4]
Where: apps/desktop/prototype/lumen-frames.mjs:419-423, 445-449
Code:
```js
// `synthetic.rs` spec: two-axis gradient, 4x4 red corner blocks,
// white every-16th grid line, and the (5,5) frame-index marker) —
```
```js
  const at = (5 * width + 5) * 4;
  out[at] = markerIndex & 0xff;
  out[at + 1] = 0xee;
```
What is wrong: The doc attributes the (5,5) frame-index marker to the `synthetic.rs` spec, but the Rust generator has no marker write (verified: synthetic.rs:29-53 writes only gradient/corner/grid; no 0xee/0x77/marker anywhere in the file). The JS pattern diverges from the Rust bytes at (5,5) by design (so the standalone page can display a frame counter) — but the comment claims mirror-ship for a deliberate divergence.
Why it matters: A "mirror" claim with an undocumented delta is exactly how conformance drift starts; any future golden comparison at (5,5) fails mysteriously.
What correct looks like: state the marker as a JS-only addition for the standalone page, not part of the mirrored spec.
Confidence: HIGH — Verification: read of both files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PROTOCOL` (const) | 37-46 | REAL | — | Identical to wire.rs constants 70-89 (verified) |
| `WireError` | 55-62 | REAL | — | Typed error mirror of desktop::WireError |
| `toSafeNumber` | 64-73 | REAL | — | JS-specific f64/u64 seam, honestly documented (48-51) |
| `ByteQueue` ctor/length | 76-84 | REAL | — | 64 KiB initial |
| `ByteQueue::feed` | 86-100 | REAL | — | Grow-with-compact arithmetic correct; may grow before compacting (churn, not bug) |
| `ByteQueue::view` | 102-104 | REAL | — | Caller-guaranteed length; lifetime documented |
| `ByteQueue::skip` | 106-116 | REAL | — | Half-buffer compaction; reset at drain |
| `FrameDecoder` ctor/feed | 127-136 | REAL | — | Fed/consumed counters |
| `FrameDecoder::nextFrame` | 141-150 | REAL | — | Copies payload via `.slice` before skip — safe against queue reuse; header-extension bytes skipped per headerLen |
| `FrameDecoder::finish` | 152-163 | REAL | — | Mirrors wire.rs finish 607-620 incl. parse-error propagation (verified) |
| `parseFrameHeader` | 171-228 | REAL | — | Faithful mirror (constants, order, messages — verified against wire.rs 625-686/208-231) |
| `parseHeader` re-export | 232 | REAL | — | Harness/host escape hatch |
| `encodeFrame` | 242-263 | REAL | F-004 | Missing geometry validation the Rust encoder enforces; untyped RangeError path |
| `LatencyLedger::{ctor,record,summary}` | 269-291 | REAL | — | Exact min/max, floor mean; mirror of mailbox::LatencyAccountant |
| `CanvasPresenter` ctor | 300-309 | REAL | — | State fields for both backends |
| `CanvasPresenter::detect` | 312-326 | REAL | — | webgl2→2d→none chain; WebGPU honestly stubbed |
| `CanvasPresenter::status` | 328-333 | REAL | F-003 | "Truthful" report can count failed presents |
| `CanvasPresenter::initWebgl2` | 335-360 | REAL | F-003 | No compile/link/error checks |
| `CanvasPresenter::draw` | 367-406 | REAL | F-002 | BGRA swizzle 2d-only; WebGL2 uploads BGRA as RGBA (swap); 2d path correct (clamped view + offscreen) |
| `swizzleBgraToRgba` | 410-416 | REAL | — | In-place, correct loop bound |
| `syntheticPattern` | 425-451 | REAL | F-005 | Gradient/corner/grid match synthetic.rs; marker is a JS-only delta misattributed to the Rust spec |

## 5. Test honesty
No tests exist for this file (the would-be harness is missing — F-001). The Rust-side wire protocol IS tested (`apps/desktop/tests/protocol.rs`, baseline-green) but that proves wire.rs, not this mirror. Nothing in ui/ executes this file (frame-channel.ts is an independent TS port that cites it as reference only).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "mirrors apps/desktop/src/wire.rs BYTE FOR BYTE" (5-6) | lumen-frames.mjs | REAL (decoder half) | My cross-read: constants/order/messages identical (F-001 notes the broken proof path, not a divergence); encoder half drifts (F-004) |
| "the conformance proof is prototype/harness.mjs over the Rust-emitted golden stream" (5-6) | lumen-frames.mjs | FALSE (missing artifact) | find: no harness.mjs anywhere; README:36/video-in-webview.md:52 instruct running it |
| "the 2D and WebGL2 paths are implemented but UNVERIFIED IN A BROWSER" (30-32) | lumen-frames.mjs | REAL (honest) | No browser in sandbox; and the WebGL2 BGRA arm is not just unverified but wrong (F-002) |
| "WebGL2 uploads BGRA natively where the extension exists" (374-375) | lumen-frames.mjs | FALSE (no such code) | No extension query, no branch; upload is unconditional gl.RGBA (F-002) |
| syntheticPattern "mirrors renderer-wgpu's synthetic.rs spec … and the (5,5) frame-index marker" (419-423) | lumen-frames.mjs | PARTLY FALSE | Marker absent from synthetic.rs (F-005) |

## 7. Dead / duplicated / unreachable code
The whole file is unreachable from shipped binaries (prototype/). It is triple-duplicated logic by design: wire.rs (Rust authority), this .mjs (JS reference), ui/src/client/frame-channel.ts (TS production port) — three implementations of one parser with only the Rust one under test; the drift found in F-004/F-005 is the predicted cost of that triplication.

## 8. Dependencies & flags
Zero dependencies (deliberate, documented). No build integration — plain .mjs, not part of any npm workspace or cargo build. No feature flags.

## 9. Open questions for the next agent
- W13 desktop wire.rs owner: confirm which FORMAT the bridge actually emits (Rgba8 or Bgra8) — if the shipped stream is BGRA (wgpu's common native), F-002 becomes the default-rendering path for the tauri shell when it ever builds; if Rgba8, it is latent.
- W26 (harness.mjs referencers): docs/waves/w26/findings/w26.g19.md and ratings/w26.h17.md mention harness.mjs — the artifact may have been dropped in a worktree merge; worth a recovery check before the wave gate.
- Whoever owns ui/src/client/frame-channel.ts: does the TS port share the F-002 swizzle gap, or does it swizzle correctly for its WebGL backend? (One grep: `swizzle` in frame-channel.ts.)
