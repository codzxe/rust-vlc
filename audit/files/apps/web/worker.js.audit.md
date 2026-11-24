# AUDIT: apps/web/worker.js
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2025-06-27 |
| Lines / functions | 170 / 4 (send, pumpOutputs, onmessage, init IIFE) + module state |
| File verdict | REAL (message lane proven in a real browser; frame lane REAL-UNPROVEN) |
| Reachable from shipped entry point? | YES — the shipped web lane: served beside `lumen_web.wasm` (header L38-42); rc-smoke browser-load PASS (baseline rc-smoke.txt:205-210, "0 page errors"); committed real-browser matrix docs/web/browser-matrix-20261110.md boots the real Worker (10/17 scenarios per mode) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `node --check apps/web/worker.js` -> exit 0 (parses); python3 hex-verify of L112 (see F-004 note on the `[m` output artifact); `rg "presentFrame" docs/web/browser-matrix-20261110.{md,json}` -> 0 matches; `rg "fetch|Fetch" apps/web/worker.js` -> only the module fetch; baseline citations below |

## 1. One-paragraph verdict
The off-main-thread engine glue is real and its core message protocol is PROVEN in a real browser: the committed, determinism-proven Chromium matrix (docs/web/browser-matrix-20261110.md) boots the Worker, streams the wasm (14 fetches, application/wasm), completes the boot handshake through the JSON→u32 word lane, loads a 425,516-byte blob through `loadBlob`, and answers typed refusals — 10/17 scenarios per mode. Backpressure is correctly shaped: inputs are serialized by the worker event loop with a 32 MiB typed input bound, outputs are pumped after every dispatch with a 4096-message wasm-side bound, and the run loop re-arms one `setTimeout(0)` slice per busy pump report with per-tick work capped by SliceBudgets (host.rs:148-163). The hostile-focus answers: (a) the protocol works end-to-end in Chromium for everything except the frame lane; (b) the `presentFrame`/RGBA read-and-transfer path (L96-108) is exercised by ZERO committed browser evidence (the active presenter is `video.headless-counter-presenter`), and the blob lane pays one FFI call per 4 bytes with a ~7.5 MiB effective media ceiling.

## 2. What actually works (verified)
— `node --check apps/web/worker.js` -> exit 0 (this session): the file is syntactically valid JS. (Note: this check was mandatory — see F-004's transport artifact; the Bash output channel eats the two-byte sequence `[m`, which made raw sed/python dumps of L112 LOOK corrupted. Hex decode `...73616765735b6d65737361676573...` proves the line is `const last = messages[messages.length - 1];` — the file is intact; the earlier mangled view was the audit toolchain's display layer, not the file. Flagged for the verification pass.)
— Real-browser evidence, committed: docs/web/browser-matrix-20261110.md:65 `boot-handshake | capability | pass`, :79-80 fetch transport rows pass, :73 `open-garbage-blob-refused ... pass`; 10/17 per mode (:45,:61); determinism of the report proven per worklog (byte-identical regeneration). The blob lane carried a 425,516-byte fixture (byte count in :80's evidence) through `Array.from` JSON to `blobLoaded`.
— rc-smoke.txt:205-210: `web-ui-browser-load: PASS (loaded, booted to the app surface, 0 page errors)`.
— Re-arm semantics proven natively: apps/web/src/abi.rs:527-568 (baseline lumen_web 111 tests ok, cargo-test-workspace.txt:3441) — the busy/release truth the setTimeout chain depends on.

## 3. Findings
F-001 [S2]
Where: apps/web/worker.js:62-75 (with apps/web/src/abi.rs:87-91)
Code:
```
function send(json) {
  const bytes = encoder.encode(json);
  // Pad to a 4-byte boundary with spaces (JSON whitespace).
  const paddedLength = bytes.length + ((4 - (bytes.length % 4)) % 4);
  const padded = new Uint8Array(paddedLength);
  padded.set(bytes);
  padded.fill(0x20, bytes.length);
  const view = new DataView(padded.buffer);
  for (let offset = 0; offset < paddedLength; offset += 4) {
    wasm.exports.lumen_web_write_u32(view.getUint32(offset, true));
```
What is wrong: The blob lane's cost profile. `loadBlob` rides the JSON contract (abi.rs:29-30: "One call per 4 bytes — the no-`unsafe` price"; protocol has no binary lane), and the page must send the file as a JSON number array (ui/e2e/web/site/harness.js:188 `bytes: Array.from(bytes)`). Per byte of media: ~8 bytes page-side (Array.from doubles), ~4.2 chars of JSON, one TextEncoder pass, one padded copy, and ¼ of an FFI call. A 425 KB fixture costs ~450,000 `lumen_web_write_u32` calls — it worked in the matrix, but a 5 MB file costs ~5.5M calls plus ~21 MB of transient strings, and at ~7.5 MB the JSON exceeds `MAX_INPUT_BYTES` (32 MiB, abi.rs:91) and the load is refused typed. Why it matters: the blob lane is the ONLY way user media enters the browser product (no filesystem, doc 09 §9.2), so the effective v1 media ceiling is ~7 MiB and everything under it pays seconds of word-pushing and ~10x memory amplification. The ABI comment admits the raw linear-memory blob lane is "card 7.H4's follow-up" — until then this is the real cost of the shipped path. What correct looks like: the reserved raw lane (write bytes straight into exported memory) or chunked base64 with a bulk export. Confidence: HIGH — Verification: read + arithmetic; executed at 425 KB scale in the committed matrix (larger sizes UNVERIFIED in browser).

F-002 [S3]
Where: apps/web/worker.js:118-124 and 78-89
Code:
```
self.onmessage = (event) => {
  if (!wasm) {
    pending.push(event.data);
    return;
  }
  send(JSON.stringify(event.data));
};
```
What is wrong: Zero error containment on the hot path. `send` -> `lumen_web_dispatch` (a wasm call that can trap on an engine bug), `pumpOutputs` -> `new Uint8Array(wasm.exports.memory.buffer, addr, len)` (RangeError if addr/len are inconsistent), `JSON.parse` (SyntaxError if the engine stages malformed JSON) — none are wrapped. An exception escapes `onmessage`, the outbound batch is never posted, the wasm input lane may be left mid-message (next message appended to a partial JSON -> permanent `STATUS_PARSE_ERROR`), and the page receives NO typed `controlError` — the lane hangs silently. The file's own convention (L136-141, L162-169) posts typed `controlError`s for fetch/instantiate failures; the dispatch path gets none. What correct looks like: try/catch around send() posting `controlError` code `web.worker.dispatch` and marking the lane dead. Confidence: HIGH — Verification: read (no execution of a trapping path; UNVERIFIED by run).

F-003 [S3]
Where: apps/web/worker.js:96-108
Code:
```
let frame = null;
const frameAddr = wasm.exports.lumen_web_frame_addr();
const frameLen = wasm.exports.lumen_web_frame_len();
if (frameAddr !== 0 && frameLen > 0) {
  frame = new Uint8Array(wasm.exports.memory.buffer, frameAddr, frameLen).slice();
}
for (const message of messages) {
  if (message.type === "presentFrame" && frame !== null) {
    message.rgba = frame;
    self.postMessage(message, [frame.buffer]);
```
What is wrong: (1) REAL-UNPROVEN: `rg "presentFrame" docs/web/browser-matrix-20261110.md .json` -> 0 matches; the committed browser matrix never exercises the frame lane (the booted build's presenter is `video.headless-counter-presenter`, matrix :65), and no other committed browser evidence does — the JS-side read of `frame_addr`/`frame_len`, the `.slice()` copy, and the transfer have never run in a browser. (2) The documented invariant "at most ONE presentFrame per dispatch" (L90-91) is enforced nowhere: if a batch ever contained two, the second `postMessage` would transfer the already-detached `frame.buffer` and throw `DataCloneError`, killing the loop (uncaught, per F-002). What correct looks like: a `framesPosted` guard (post the second frame without transfer or diagnose loudly), and one browser-matrix row that forces a real presentFrame. Confidence: HIGH — Verification: read + rg (absence of evidence pinned); execution UNVERIFIED.

F-004 [S4]
Where: apps/web/worker.js:110-115 (re-arm) and ui/e2e/web/site/harness.js:211-220 (the complementary driver)
Code:
```
  // Re-arm the run loop while the engine reports work outstanding
  // (doc 09 §3.2: host timers re-arm the pump).
  const last = messages[messages.length - 1];
  if (last && last.type === "pumped" && last.report && last.report.busy) {
    setTimeout(() => send('{"type":"pump","ticks":1}'), 0);
  }
```
(Quote verified byte-exact via python hex: `...65735b6d6573...` — the Bash sed output of this line is mangled by the audit transport, which eats the two bytes `[m`; the file itself is correct and `node --check` exits 0.)
What is wrong: The pump-driving contract is split and implicit: the worker re-arms ONLY on busy, the page arms when idle (harness `drivePump` re-arms when `!last.report.busy`). An embedder that also re-arms on busy doubles the chain every cycle (each busy pumped reply -> 2 pumps -> 2 busy replies -> 4...), an unbounded pump flood; nothing in the ABI docs states the invariant, only the code comment "the future @lumen/wasm-client owns that policy" (L126-129). Why it matters: the protocol's backpressure correctness depends on an undocumented division of labor. What correct looks like: document the invariant in abi.rs's wire contract (page arms idle, glue arms busy), or have the glue own ALL re-arming. Confidence: HIGH — Verification: read (both drivers); flood not executed.

## 4. Per-function table (mandatory, file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module state (`encoder/decoder/wasm/pending`) | 51-59 | REAL | — | pending-queue-before-boot prevents boot-message loss (documented + correct) |
| `send` | 62-75 | REAL | S2 | word lane correct (space padding is JSON-legal); cost per F-001 |
| `pumpOutputs` | 78-116 | REAL | S3 | reads before any further wasm call (detach-safe, documented invariant); frame lane F-003; re-arm F-004; unguarded F-002 |
| `self.onmessage` | 118-124 | REAL | S3 | queue-then-send correct; no containment (F-002) |
| init IIFE | 132-170 | REAL | — | module URL relative to import.meta.url; typed controlError on fetch failure (status included); streaming + arrayBuffer fallback; drain in arrival order |

## 5. Test honesty (if tests exist)
No tests in this file. External evidence:
| Evidence | Would it fail if the glue were broken? | Reason |
|---|---|---|
| browser-matrix boot-handshake (pass, :65) | YES | the reply rode the full JSON→u32→dispatch→output→postMessage round trip in Chromium |
| browser-matrix blob scenarios (:73, fixture lane :188-196) | YES | blobLoaded ack + typed probe refusal both crossed the lane |
| rc-smoke browser-load (PASS, 0 page errors) | WEAK | only proves the page boots, not the worker lane |
| native abi.rs re-arm tests (baseline 111 ok) | PARTIAL | prove the wasm side of the busy/release truth the JS chain rides; the JS setTimeout chain itself is only proven by the matrix's flowing pumped reports |
| frame lane | NOTHING | zero browser evidence (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Messages that arrived before the module was instantiated are QUEUED, never dropped" | L55-58 | REAL | L118-124 + drain L159-161 |
| "the engine NEVER blocks this Worker's event loop; every slice is bounded" | L33-36 | REAL | setTimeout re-arm L114; SliceBudgets host.rs:148-163; native re-arm test abi.rs:527-568 |
| "a `presentFrame` element additionally carries the RGBA bytes" | L16-19 | REAL-UNPROVEN | zero presentFrame in committed browser evidence (F-003) |
| "the server must answer the wasm with `application/wasm` (streaming instantiation)" | L39-40 | REAL | matrix :58 "content-type application/wasm" + 14 fetches; fallback path UNVERIFIED in browser |
| "Browser-runtime behavior is [VERIFY]" | L44-46 | REAL (honest) | partially discharged by the committed matrix; frame lane still open |

## 7. Dead / duplicated / unreachable code
None dead. Cross-file observation: worker.js contains NO media-fetch glue — `rg "fetch"` hits only the module fetch (L134). The fetch transport that `apps/web/src/io` models (mock_fetch/staged_source, "the Worker's JS glue calling fetch()") does not exist in the shipped glue; the browser host refuses http opens (`open-http-pending-lane ... unsupported-for-host`, matrix :72). The io lane is native-test-only today.

## 8. Dependencies & flags
Zero dependencies (hand-written, "use strict"); uses TextEncoder/TextDecoder/WebAssembly/fetch/import.meta.url — all baseline browser APIs. No build step. Sits beside lumen_web.wasm + audio-worklet.js (serving contract, L38-42).

## 9. Open questions for the next agent
- Who owns the pump policy when the real UI (not the e2e harness) drives the lane? The undocumented split (F-004) is a trap for the @lumen/wasm-client author.
- Does any planned browser-matrix row force a real presentFrame (WebGL upload of engine RGBA)? Until then the frame lane stays REAL-UNPROVEN.
