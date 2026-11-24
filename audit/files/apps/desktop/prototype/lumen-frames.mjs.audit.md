# AUDIT: apps/desktop/prototype/lumen-frames.mjs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-24 |
| Lines / functions | 451 / 13 (2 classes, 6 functions, 1 const, 1 error class, 3 exports) |
| File verdict | REAL as prototype (parser executed: 23/23 checks) — the "byte-for-byte mirror, conformance via harness.mjs" claim is FALSE |
| Reachable from shipped entry point? | NO — `apps/desktop-tauri/tauri.conf.json` frontendDist is `../../ui/dist`; the shipped UI ships its OWN decoder (`ui/src/client/frame-channel.ts:11` calls this file a "sibling of the reference"). Reference/dev artifact only. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `node audit/runs/W13-A03-lumen-frames-check.mjs` → 23 checks, `ALL CHECKS PASSED`, exit 0 (script committed under audit/runs/); `find . -name 'harness.mjs'` → no hits; `rg -rn 'harness' .github/workflows/` → no CI step |

## 1. One-paragraph verdict
The parser core is real and now EXECUTED: I ran a 23-check Node conformance suite (golden decode from wire.rs's hand-written spec-table bytes, chunked feed at every split point, 8 malformed-input refusals, truncation, 1 GiB cap, encoder round-trip) and everything within the module's self-imposed safe-integer domain passes. But the module's headline claim — "mirrors apps/desktop/src/wire.rs BYTE FOR BYTE (the conformance proof is prototype/harness.mjs over the Rust-emitted golden stream)" — is FALSE on both halves: `harness.mjs` does not exist anywhere in the repo (three shipped-code files and one doc cite it), and the ONE committed Rust golden vector (wire.rs:720-756, frame_id 0x1122334455667788, emitted_ns 0x0102030405060708) is UNDECODABLE by this module — both fields exceed 2^53 and the decoder throws `unsafe-integer` (executed, twice). The claimed cross-language conformance proof is fabricated-by-reference: no harness, no golden stream file, no CI run. The canvas presenter additionally has a latent BGRA/WebGL2 color-swap bug, and the WebGPU rung is an honestly-labeled stub.

## 2. What actually works (verified)
— `node audit/runs/W13-A03-lumen-frames-check.mjs` → 23 `ok` lines, `ALL CHECKS PASSED`, exit 0: golden header fields (headerLen 48, rgba8, repeat, 2x1, payloadLen 8, pts -1000, payload bytes), every-split-point chunked decoding, bad-magic / version-2 / reserved-flag / reserved-u16 / payload-mismatch / zero-width / 1-GiB-cap refusals, truncated-finish error, encodeFrame round-trip (bgra8). Plus two executed divergence proofs (below).
— `rg -ln 'synthetic' crates/renderer-wgpu/src/` → synthetic.rs exists with the matching spec (4x4 red corners, every-16th white grid, (5,5) marker); `rg -n 'LatencyAccountant' apps/desktop/src/mailbox.rs` → L202 exists — the two other "mirror" targets are real.

## 3. Findings
F-001 [S2] (borderline S0 on the fabricated-evidence axis)
Where: apps/desktop/prototype/lumen-frames.mjs:5-7
Code: `// This module mirrors apps/desktop/src/wire.rs BYTE FOR BYTE (the\n// conformance proof is prototype/harness.mjs over the Rust-emitted\n// golden stream). It is deliberately dependency-free and`
What is wrong: The claimed proof does not exist: `find . -name 'harness.mjs'` → nothing; no golden stream file; no CI step (.github/workflows has no harness/node run). The reference is repeated in shipped code: wire.rs:15-17 ("cross-language conformance via `apps/desktop/prototype/harness.mjs`"), lib.rs:50-52 ("Rust encoder → the committed JS decoder via `prototype/harness.mjs`, run under Node"), docs/prototypes/video-in-webview.md:52 ("also runs headless in CI with Node" — no such CI step exists). Worse, the one committed golden vector is undecodable by the mirror (F-002), so the claimed proof could never have passed on it. Why it matters: four files point developers at a verification artifact that is absent while the actual mirror demonstrably diverges from the Rust test vectors — the "proven" cross-language contract is unproven and partially false. What correct looks like: commit harness.mjs (or delete the four references), and align the safe-integer policy with wire.rs's vectors or vice versa. Confidence: HIGH — Verification: ran (Node divergence proofs) + `find`/`rg` (absence).

F-002 [S3]
Where: apps/desktop/prototype/lumen-frames.mjs:64-73 (guard) with apps/desktop/src/wire.rs:729-748 (golden vector), wire.rs:696 (test header)
Code: `function toSafeNumber(bigint, field) {\n  const n = Number(bigint);\n  if (!Number.isSafeInteger(n)) {\n    throw new WireError(\n      "unsafe-integer",\n      \`${field} ${bigint.toString()} exceeds the JS safe-integer range\`,\n    );\n  }\n  return n;\n}`
What is wrong: The decoder refuses every protocol-legal u64 frame_id / emitted_ns above 2^53-1. The Rust golden test uses frame_id 0x1122334455667788 (1.23e18) and emitted_ns 0x0102030405060708 (7.26e16); wire.rs's own `header()` fixture uses emitted_ns 1_700_000_000_000_000_000 (1.7e18). All three are refused typed by this module (executed: `unsafe-integer` thrown for frame_id and, separately, for emitted_ns). Production streams stay small (bridge.rs MonotonicEmitClock = `Instant` ns since construction, bridge.rs:58-84; frame ids increment from 1), so real traffic survives — but "BYTE FOR BYTE" is false at the type boundary, and any epoch-based or >104-day-uptime clock kills the JS side. Confidence: HIGH — Verification: ran.

F-003 [S3]
Where: apps/desktop/prototype/lumen-frames.mjs:372-392
Code: `if (frame.header.formatCode === PROTOCOL.FORMAT_BGRA8 && backend === "2d") {\n      // The 2D path takes RGBA; BGRA frames swizzle draw-side\n      // (recorded cost; WebGL2 uploads BGRA natively where the\n      // extension exists [VERIFY on desktop]).\n      swizzleBgraToRgba(frame.payload);\n    }\n    if (backend === "webgl2") {\n      const gl = this.gl;\n      gl.bindTexture(gl.TEXTURE_2D, this.texture);\n      gl.texImage2D(\n        gl.TEXTURE_2D,\n        0,\n        gl.RGBA,`
What is wrong: The BGRA→RGBA swizzle runs ONLY on the 2D backend; the WebGL2 path uploads every frame as `gl.RGBA`/`UNSIGNED_BYTE`, so a protocol-legal Bgra8 frame (wire.rs:130, code 2) renders with red and blue channels SWAPPED, silently. The comment's claim that "WebGL2 uploads BGRA natively where the extension exists" describes no standard WebGL2 path for ArrayBufferView uploads (core WebGL2 texImage2D takes RGBA for UNSIGNED_BYTE data). The `[VERIFY on desktop]` marker and the module-level "UNVERIFIED IN A BROWSER" label (L30-32) keep this honest-but-broken. Confidence: HIGH (code path is unconditional; rendering itself UNVERIFIED — no browser in sandbox) — Verification: read; UNVERIFIED visually (no browser).

F-004 [S4]
Where: apps/desktop/prototype/lumen-frames.mjs:311-316
Code: `detect() {\n    if (this.backend !== null) return this.backend;\n    if (typeof navigator !== "undefined" && navigator.gpu) {\n      this.detectedGpu = true; // present but a typed stub this revision\n    }`
What is wrong: The WebGPU rung of doc 08 §7.4's chain is a no-op detection flag — `detectedGpu` is set and reported in `status()` but no upload code exists; the presenter silently falls through to WebGL2/2D. This is an HONEST stub (disclosed at L30-35 and L331 "WebGPU detected (stub in this revision — falls through)"), recorded here per checklist 6.1. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: apps/desktop/prototype/lumen-frames.mjs:152-163
Code: `finish() {\n    if (this.pending.length === 0) return;\n    const remaining = this.pending.length;\n    const missing =\n      remaining < PROTOCOL.MIN_HEADER_LEN\n        ? PROTOCOL.MIN_HEADER_LEN - remaining\n        : (() => {\n            const header = parseFrameHeader(this.pending.view(PROTOCOL.MIN_HEADER_LEN));\n            return header.headerLen + header.payloadLen - remaining;\n          })();`
What is wrong: `finish()` parses the header BEFORE reporting truncation, so a partial record with an unsafe frame_id reports `unsafe-integer` instead of `truncated` (executed: my first truncation probe threw `unsafe-integer`). Diagnostic-order divergence from the Rust twin (which has no safe-integer gate). Minor; mislabels the failure mode on a truncated tail. Confidence: HIGH — Verification: ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PROTOCOL` const | 37-46 | REAL | — | Matches wire.rs consts (48..=1024, 1<<30, 65535, codes 1/2). |
| `WireError` | 55-62 | REAL | — | Typed error, mirror of desktop::WireError kinds (verified by my malformed suite). |
| `toSafeNumber` | 64-73 | REAL | — | F-002 divergence; guard itself correct. |
| `ByteQueue.feed` | 86-100 | REAL | — | Amortized O(1) growth; compaction correct. |
| `ByteQueue.view`/`skip` | 102-116 | REAL | — | Internal invariant: callers check length first (nextFrame L142/145). |
| `FrameDecoder.feed`/`nextFrame` | 133-150 | REAL | — | Executed incl. every-split-point chunking. Payload copied via `.slice` before skip — safe. |
| `FrameDecoder.finish` | 152-163 | REAL | — | F-005 diagnostic order. |
| `parseFrameHeader` | 171-228 | REAL | — | All field validations executed; exact integer arithmetic (dims ≤65535 → implied ≤2^34, exact in f64). |
| `parseHeader` export | 232 | REAL | — | Re-export for hosts. |
| `encodeFrame` | 242-263 | REAL | — | Round-trip executed; does NOT validate width/height bounds (a 0-dim encode is decodable-side refused) — generator-tool asymmetry, hygiene only. |
| `LatencyLedger` | 269-291 | REAL | — | Exact min/max, floor mean; mirrors mailbox.rs LatencyAccountant (exists, L202). |
| `CanvasPresenter.detect`/`status` | 312-333 | REAL | — | Honest backend report; WebGPU stub (F-004). |
| `initWebgl2` | 335-360 | REAL | — | No shader-compile error checks (prototype); UNVERIFIED IN BROWSER per module doc. |
| `CanvasPresenter.draw` | 367-406 | SHALLOW | S3 | BGRA/WebGL2 color swap (F-003); no shader/link diagnostics. |
| `swizzleBgraToRgba` | 410-416 | REAL | — | In-place; mutates the decoder-owned payload (documented cost). |
| `syntheticPattern` | 425-451 | REAL | — | Spec matches renderer-wgpu/src/synthetic.rs (corners/grid/(5,5) marker); marker is `index & 0xff` — wraps at 256 (Rust twin's marker semantics UNVERIFIED byte-for-byte). |

## 5. Test honesty
No tests in this file. The claimed external proof (harness.mjs) is absent (F-001). My substitute suite is committed at audit/runs/W13-A03-lumen-frames-check.mjs (23 checks, exit 0).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mirrors apps/desktop/src/wire.rs BYTE FOR BYTE" | lumen-frames.mjs:5-6 | FALSE (partial) | Executed: the committed Rust golden vector (wire.rs:729-748) is refused (`unsafe-integer` ×2); safe-range bytes decode correctly (23/23). |
| "the conformance proof is prototype/harness.mjs over the Rust-emitted golden stream" | lumen-frames.mjs:6 | PAPER-ONLY | `find . -name 'harness.mjs'` → nothing; no golden file; no CI step. |
| "also runs headless in CI with Node" | docs/prototypes/video-in-webview.md:52 | FALSE | .github/workflows has no harness/node step (rg). |
| "cross-language conformance via apps/desktop/prototype/harness.mjs" | wire.rs:16, lib.rs:52 | PAPER-ONLY | Same absence. |
| "the 2D and WebGL2 paths are implemented but UNVERIFIED IN A BROWSER" | lumen-frames.mjs:30-32 | REAL | Honest label; still true in this sandbox (no browser). |
| "WebGPU rung … is a typed stub in this revision" | lumen-frames.mjs:33-35 | REAL | detect() only flags (L314-316). |

## 7. Dead / duplicated / unreachable code
Not shipped: frontendDist is ui/dist; the shipped UI reimplements the decoder (ui/src/client/frame-channel.ts with its own vitest suite). This file is a reference prototype — its only consumers are doc references and my audit run.

## 8. Dependencies & flags
Zero dependencies (deliberate, per header). Plain .mjs, no bundler config, no feature flags. Node 24.22.3 executes it as-is.

## 9. Open questions for the next agent
- ui/src/client/frame-channel.ts (the SHIPPED twin): does it share the safe-integer refusal policy, and does its vitest suite use the same Rust golden vector? If yes, the same divergence ships; if no, the two JS decoders already disagree.
- Did harness.mjs ever exist (git history is the orchestrator's domain — UNVERIFIED here)? Four references suggest a deleted file, not a never-written one.
