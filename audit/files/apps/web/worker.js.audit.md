# AUDIT: apps/web/worker.js
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2026-11-21 |
| Lines / functions | 170 / 5 (2 consts, `send`, `pumpOutputs`, `onmessage`, init IIFE + catch) |
| File verdict | REAL-UNPROVEN (executed green in a real Chromium once — at foreign commit 3755dc3; at the baseline SHA its only evidence is that run plus the native tests of the Rust side it drives; no unit test covers the glue itself) |
| Reachable from shipped entry point? | NO in any product — deployed only by the e2e lane: `ui/scripts/web-matrix-run.mjs:459` copies worker.js into the site dist; its sole instantiator is `ui/e2e/web/site/harness.js:85` (`new Worker("worker.js", { type: "module" })`). No shipped page, app, or package includes it (`rg "worker.js\|lumen_web" ui/src ui/test` → zero). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `rg "surfaceLost" -t js,ts,tsx,html` repo (no sender anywhere — cross-ref backend.rs F-001) · `rg "new Worker\|type.*module\|audioContext" ui/e2e/web/site/harness.js` (module-type instantiation :85; audioContext IS sent :639,655; surfaceLost is NOT) · `rg '"pumped"\|"presentFrame"\|"controlError"' apps/web/src/protocol.rs` (wire names match the glue's string literals exactly) · Read host.rs:1110-1176 (pump structure: ONE `present_one()` per pump message at :1130 — the "at most one presentFrame per dispatch" invariant the glue's single-frame transfer relies on; `Pumped` pushed last at :1044 after `pump()` returns — the re-arm's last-message check holds structurally) · `rg "PresentFrame" apps/web/src/host.rs` (present_one at 1145-1166 is the ONLY PresentFrame producer) · `rg -ln "worker.js\|lumen_web" ui/src ui/test` (zero unit coverage) · `sed -n '99,109p;131,134p'` quote verification · evidence: `docs/web/browser-matrix-20261110.md` (real Playwright Chromium run — the harness boots this glue against the real wasm artifact) |

## 1. One-paragraph verdict
A small, disciplined glue: word-padded JSON downstream, staging-buffer read upstream with the addr→len safe order and no wasm calls between, one owned copy of the frame bytes transferred to the page, pre-instantiation messages queued not dropped, typed controlErrors for fetch/instantiate failures, and a `setTimeout(0)` re-arm that keeps the Worker's event loop free. Every wire-name string literal matches protocol.rs exactly, and every ABI invariant it relies on (staging validity window, one-present-per-dispatch, Pumped-last ordering) is real in the Rust I traced. Its execution evidence is one honest Playwright Chromium run at a foreign commit — no unit test covers the JS, no product ships it, and one of its three documented "page-driven facts" (`surfaceLost`) has no sender anywhere in the repo (the S2 filed against backend.rs). The findings are four hygiene-grade fragilities: an undocumented module-worker requirement that fails at parse time with zero diagnostics, a frame buffer not nulled after transfer, an unguarded `JSON.parse` of engine output, and a silent-stall dependency on `pumped` being the last message.

## 2. What actually works (verified)
— Wire fidelity: `lumen_web_write_u32`/`lumen_web_dispatch`/`output_addr`/`output_len`/`frame_addr`/`frame_len` calls at 71-98 match the six abi.rs exports; `pump`/`presentFrame`/`pumped`/`controlError` literals match protocol.rs:213-221 wire names (rg-verified one-to-one).
— Space-padding (64-68): `(4 - (len % 4)) % 4` handles the already-aligned case correctly; JSON trailing-whitespace tolerance is the right padding choice.
— Frame-lane safety: the `new Uint8Array(memory.buffer, frameAddr, frameLen).slice()` at 100 copies before any further wasm call, then transfers the copy — the page gets an owned buffer; the detachment invariant (no wasm call between addr/len and read) is respected at 79-87 and 97-100.
— Pre-boot queue (118-124 + 159-161): messages that arrive before instantiation are queued and drained in arrival order; single-threaded event loop makes the drain race-free.
— Real-browser execution: `docs/web/browser-matrix-20261110.md` — the harness booted this glue + the real wasm artifact in Chromium 157 (boot-handshake, renderer-ladder, transport scenarios ✅) — at commit 3755dc3, 2026-11-11 (predates baseline e83e5bd).

## 3. Findings

### F-001 [S4] Module-worker requirement is hard, undocumented in the serving section, and fails with zero diagnostics
Where: apps/web/worker.js:133 (with the serving doc at 38-42)
Code:
```js
/** Instantiate the module relative to this script's own URL. */
(async () => {
  const url = new URL("lumen_web.wasm", import.meta.url);
```
What is wrong: `import.meta` is a syntax error in a classic Worker script — the file fails to PARSE, so the `.catch` at 162-170 never runs and the page receives no error at all: a silent dead worker. The serving paragraph (38-42) documents the wasm MIME requirement and the no-COOP/COEP fact but never says the Worker must be created with `{ type: "module" }`. The only instantiator in the repo knows (harness.js:85); an integrator reading the serving section does not.
Why it matters: The whole "degraded-but-honest" error posture of this glue (typed controlErrors for fetch and instantiate failures) is bypassed by the one failure mode an integrator is most likely to hit first.
What correct looks like: Document `{ type: "module" }` in the serving section (or drop `import.meta.url` for `self.location` and support classic workers too — `new URL("lumen_web.wasm", self.location.href)` works in both).
Confidence: HIGH — Verification: read; `self.location` equivalence reasoned (same-origin worker script URL semantics); no execution needed to establish the parse-time failure class.

### F-002 [S4] Frame buffer not detached-guarded after transfer — a second `presentFrame` in one batch would throw mid-loop and drop the batch's tail
Where: apps/web/worker.js:103-108
Code:
```js
  for (const message of messages) {
    if (message.type === "presentFrame" && frame !== null) {
      message.rgba = frame;
      self.postMessage(message, [frame.buffer]);
    } else {
```
What is wrong: After the first transfer, `frame.buffer` is detached, but `frame` stays non-null. If a batch ever contained two `presentFrame` messages (the JSON wire format does not forbid it), the second `postMessage` would attach an already-transferred buffer — a `DataCloneError` thrown mid-`for`, losing every message after it and stalling the run loop.
Why it matters: The safety rests on an unstated cross-language invariant: host.rs calls `present_one()` exactly once per pump message (host.rs:1130) and `pump` is the only PresentFrame producer (rg over host.rs) — true today, one refactor away from false, and the failure mode is a silent stall, not a typed error.
What correct looks like: `frame = null` immediately after the transfer (or capture the buffer in a local and null it), so a hypothetical second present degrades to a pixel-less message instead of throwing.
Confidence: HIGH — Verification: read + host.rs structure traced; the double-present case is unreachable with today's host (structural proof), hence S4 not higher.

### F-003 [S4] `JSON.parse` of the engine's output batch is unguarded — corrupted staging throws in the message handler and silently kills the loop
Where: apps/web/worker.js:87-89
Code:
```js
  const bytes = new Uint8Array(wasm.exports.memory.buffer, addr, len);
  const text = decoder.decode(bytes);
  const messages = JSON.parse(text);
```
What is wrong: No try/catch. The glue invents typed controlErrors for its own failures (module_fetch at 136-141, module_instantiate at 165-169) but has no answer for a corrupted or truncated output batch — the exact hazard the ABI's len-without-staging quirk (abi.rs F-003) or any future staging bug would produce. A throw here propagates out of `pumpOutputs` → `send` → `onmessage`: the remaining messages of the batch are lost, the `pumped` re-arm at 112-115 never runs, and playback freezes with no diagnostic.
Why it matters: Every other failure lane in this file reports a typed error to the page; this one — the lane carrying all engine output — fails silent. It is the glue-side twin of the Rust module's typed-refusal philosophy, missing.
What correct looks like: Wrap the parse in try/catch and post `{type:"controlError", code:"web.worker.output_parse", ...}` (matching the shape at 136-141), then continue.
Confidence: HIGH — Verification: read; the corrupted-output case is unreachable with a correct Rust side (traced staging), so latent.

### F-004 [S4] Run-loop re-arm depends on `pumped` being the LAST batch element — an ordering change in host.rs silently stalls playback
Where: apps/web/worker.js:112-115
Code:
```js
  const last = messages[messages.length - 1];
  if (last && last.type === "pumped" && last.report && last.report.busy) {
    setTimeout(() => send('{"type":"pump","ticks":1}'), 0);
  }
```
What is wrong: The re-arm only fires when `pumped` is the final message. Host.rs currently guarantees this (the Pump handler pushes `Pumped` at 1044 after `pump()` returns, and everything `pump()` emits — events, PresentFrame, GestureRequired — is pushed during the call), but nothing on either side documents the dependency; a future message pushed after `Pumped` in the same dispatch turns a busy engine into a frozen one with no error anywhere.
Why it matters: Cross-language positional invariants that live only in one file's control flow are the cheapest way to break a message-loop system; the cost of robustness here is scanning the batch for the last `pumped` instead of the last element.
What correct looks like: `const pumped = messages.filter(m => m.type === "pumped").pop()` — order-independent — or document the Pumped-last invariant in protocol.rs where a future editor would see it.
Confidence: HIGH — Verification: read + host.rs ordering traced (present at 1130, autoplay at 1131, handler push at 1044).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `encoder`/`decoder` consts | 51-52 | REAL | — | Module-global; fine single-threaded. |
| `wasm`/`pending` state | 54-59 | REAL | — | Queue-not-drop documented and honored (118-124, 159-161). |
| `send(json)` | 61-75 | REAL | — | Pad-to-4 with spaces; word loop; dispatch; then pumpOutputs. Matches abi.rs contract exactly. No input-size guard in JS (the Rust side's 32 MiB typed overflow is the enforcement point — correct layering). |
| `pumpOutputs()` | 77-116 | REAL | S4 | F-002 (frame not nulled), F-003 (unguarded parse), F-004 (last-element re-arm). addr→len order safe (abi.rs F-003's hazard avoided here). `len===0` early return unreachable post-dispatch (staging is at minimum `"[]"`) — dead defensive branch, harmless. |
| `self.onmessage` | 118-124 | REAL | — | Pre-wasm queue; else stringify+send. `JSON.stringify` of unserializable page data would throw here — page's bug, not guarded (noted, not finding). |
| init IIFE | 131-161 | REAL | S4 | F-001 (import.meta/module requirement). Streaming-vs-fallback MIME check (147-155) honest; `instance.instance ? ... : instance` ternary at 156 is defensive (both branches return the wrapper — harmless). Drain order FIFO ✓. |
| `.catch` → controlError | 162-170 | REAL | — | Typed instantiate failure — the honest-error posture F-001 bypasses at parse time. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (no unit tests exist for worker.js) | — | `rg -ln "worker.js\|lumen_web" ui/src ui/test` → zero. The glue's only executed evidence is the Playwright matrix run (real browser, foreign SHA): boot-handshake, renderer-ladder-consistency, transport scenarios — which WOULD fail on wire-name, padding, or staging-contract breakage, but cover none of the F-001..F-004 edge paths (they are unreachable-with-correct-Rust). |
| Native Rust tests (cross-ref) | YES for the shared contract | abi.rs part2 tests pin the ABI invariants this glue depends on (word lane ≡ JSON lane, staging stability, frame staging) — green in `orchrun-test-workspace-nofailfast.log.txt:3090-3094`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`surfaceLost` reports a dead canvas context / WebGPU device; the engine answers `renderDegraded`" | worker.js:26-27 | FALSE as an end-to-end flow (relay is generic; NO page sends it) | backend.rs F-001 (S2, this wave): repo-wide rg — only this comment mentions it. |
| "`audioContext` reports the page's AudioContext state" | worker.js:28-31 | REAL (e2e lane) | harness.js:639,655 posts it; host.rs:1058-1066 handles it. |
| "Messages that arrived before the module was instantiated are QUEUED, never dropped" | worker.js:55-58 | REAL | 118-124 + 159-161; single-threaded drain race-free. |
| "the engine NEVER blocks this Worker's event loop; every slice is bounded" | worker.js:33-36 | REAL | setTimeout(0) re-arm (112-115) + host.rs SliceBudgets/tick cap (host.rs:1121 `ticks.min(max_ticks_per_pump)`); no sync waits in the glue. |
| "Serving: … server must answer the wasm with `application/wasm` … No COOP/COEP headers required" | worker.js:38-42 | REAL but INCOMPLETE | F-001: the `{type:"module"}` Worker requirement is missing from the serving contract. |
| "Browser-runtime behavior is [VERIFY]" | worker.js:44-46 | REAL (honest label) | Verified once in Chromium at 3755dc3 (browser-matrix-20261110.md); UNVERIFIED at baseline SHA. |

## 7. Dead / duplicated / unreachable code
- `len === 0` early return (81-83): unreachable after any dispatch (staging minimum is `"[]"`) — dead defensive branch.
- The `instance.instance ? instance.instance : instance` fallback (156): both call forms used return the wrapper — dead ternary arm.
- The file itself is unreachable from any shipped product (e2e-only deployment, per header row).

## 8. Dependencies & flags
- Zero dependencies (hand-written, `"use strict"`). No build step — copied verbatim by web-matrix-run.mjs:459. No lint/format tooling covers it (no eslint/prettier config for apps/web found — noted, not verified beyond rg for config files).

## 9. Open questions for the next agent
- A03 (audio-worklet.js): the serving claim (38-39) says this file, audio-worklet.js and the wasm sit in the same directory — confirm the worklet is likewise copied by web-matrix-run.mjs and that its message contract matches the audio lane's.
- Wave-19 (docs/web): the browser-matrix record is this file's only execution evidence — its provenance (json record vs rendered md, capture SHA vs baseline SHA) needs the docs-audit treatment.
- A01/A04: if abi.rs adopts the `output_len`-stages fix (my abi.rs F-003), the addr→len order here stops mattering — coordinate so both sides of the contract stay honest in the same change.
