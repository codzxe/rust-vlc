/*
 * Lumen engine worker glue (card 7.H1, issue #47; doc 09 §3.2/§7).
 *
 * The minimal hand-written bridge between the Worker MessagePort and
 * the wasm module's extern "C" ABI (apps/web/src/abi.rs — the
 * no-dependency, no-unsafe lane; see that module's docs for the wire
 * contract):
 *
 *   downstream  postMessage(obj) -> JSON -> 4-byte words (space-padded
 *                tail — JSON tolerates trailing whitespace) pushed via
 *                lumen_web_write_u32, then lumen_web_dispatch()
 *   upstream    lumen_web_output_addr() + lumen_web_output_len() ->
 *                read the JSON array straight out of memory.buffer ->
 *                one postMessage per element
 *
 * The run loop is re-armed per doc 09 §3.2: after every pump report
 * that says `busy`, a setTimeout(0) tick sends the next pump slice —
 * the engine NEVER blocks this Worker's event loop; every slice is
 * bounded (SliceBudgets, apps/web/src/host.rs).
 *
 * Serving (doc 09 §2.3): this file and lumen_web.wasm sit in the
 * same directory; the server must answer the wasm with
 * `application/wasm` (streaming instantiation). No COOP/COEP headers
 * are required — v1 is the single-threaded cooperative lane
 * (`threading: "single"` in the boot handshake).
 *
 * Browser-runtime behavior is [VERIFY]: exercised only in a real
 * browser (card 7.I1); the same message paths are proven headless by
 * the crate's native tests.
 */

"use strict";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

let wasm = null;
// Messages that arrived before the module was instantiated are
// QUEUED, never dropped (the port contract has no "not ready"
// answer — the host page may send `boot` while the fetch is still
// in flight; a silent drop would hang it forever).
const pending = [];

/** Push one JSON-encoded message through the word lane. */
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
  }
  wasm.exports.lumen_web_dispatch();
  pumpOutputs();
}

/** Read the staged output batch and post each message. */
function pumpOutputs() {
  const addr = wasm.exports.lumen_web_output_addr();
  const len = wasm.exports.lumen_web_output_len();
  if (len === 0) {
    return;
  }
  // Read immediately — the staging buffer is valid until the next
  // dispatch (the ABI's documented invariant; no wasm call in
  // between, so memory cannot grow and detach the buffer).
  const bytes = new Uint8Array(wasm.exports.memory.buffer, addr, len);
  const text = decoder.decode(bytes);
  const messages = JSON.parse(text);
  for (const message of messages) {
    self.postMessage(message);
  }
  // Re-arm the run loop while the engine reports work outstanding
  // (doc 09 §3.2: host timers re-arm the pump).
  const last = messages[messages.length - 1];
  if (last && last.type === "pumped" && last.report && last.report.busy) {
    setTimeout(() => send('{"type":"pump","ticks":1}'), 0);
  }
}

self.onmessage = (event) => {
  if (!wasm) {
    pending.push(event.data);
    return;
  }
  send(JSON.stringify(event.data));
};

// The engine host page's own pump driver: when the page opens a
// session it starts the loop by sending pump messages itself (the
// future @lumen/wasm-client owns that policy; this glue only
// CONTINUES a busy loop).

/** Instantiate the module relative to this script's own URL. */
(async () => {
  const url = new URL("lumen_web.wasm", import.meta.url);
  const fetched = await fetch(url);
  if (!fetched.ok) {
    self.postMessage({
      type: "controlError",
      code: "web.worker.module_fetch",
      message: "the engine module could not be fetched: " + fetched.status,
    });
    return;
  }
  // instantiateStreaming when the MIME is right; the array-buffer
  // fallback keeps a misconfigured server degraded-but-honest
  // instead of dead.
  let instance;
  if (
    typeof WebAssembly.instantiateStreaming === "function" &&
    (fetched.headers.get("content-type") || "").includes("application/wasm")
  ) {
    instance = await WebAssembly.instantiateStreaming(fetched, {});
  } else {
    const bytes = await fetched.arrayBuffer();
    instance = await WebAssembly.instantiate(bytes, {});
  }
  wasm = instance.instance ? instance.instance : instance;
  // Drain everything that arrived while the module was loading, in
  // arrival order (typically just the `boot` handshake).
  while (pending.length > 0) {
    send(JSON.stringify(pending.shift()));
  }
})().catch((error) => {
  // Module-script workers have no error event for top-level awaits;
  // surface instantiation failures as the typed control error.
  self.postMessage({
    type: "controlError",
    code: "web.worker.module_instantiate",
    message: "the engine module could not be instantiated: " + error,
  });
});
