# apps/web — the engine-in-Worker wasm host (card 7.H1, issue #47)

The wasm32 composition root of the Lumen engine: the same std-only
engine core the CLI drives (probe → demux → engine → queues → clock),
exposed to a Web Worker through a typed MessagePort contract whose
command lane carries the ONE command layer's envelopes (64 commands /
23 events ride the `crates/commands` catalog — never re-defined here)
and a hand-rolled `extern "C"` ABI over shared linear memory with
**zero new dependencies** (doc 09 §2.2's raw-import fallback lane;
nothing in doc 09 §3 requires wasm-bindgen for the single-threaded
cooperative v1 — the choice and its reversal seam are recorded in
`src/abi.rs`'s module docs).

## Layout

| File | What it is |
|---|---|
| `src/lib.rs` | The crate root + the honest posture summary |
| `src/protocol.rs` | The typed `HostToWorker`/`WorkerToHost` MessagePort contract (JSON wire, round-trip pinned by tests) |
| `src/abi.rs` | The `extern "C"` exports + the dispatch core (`AbiWorker`) — no `unsafe`, no deps |
| `src/host.rs` | `WebHost`: the engine, the cooperative pump/slice budgets (doc 09 §3.2), the served command surface |
| `src/open.rs` | The open path: blob registry → probe → AVI wiring → `SessionConfig` |
| `src/source.rs` | The in-memory `BlobSource` + bounded `BlobRegistry` |
| `src/capabilities.rs` | The honest capability snapshot (`threading: single`, doc 09 §10 as typed rows) |
| `worker.js` | The minimal hand-written Worker glue (postMessage ↔ wasm ABI) |

## Building the module

```sh
cargo build -p lumen-web --target wasm32-unknown-unknown --locked
cp $CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/lumen_web.wasm ./
```

Serve `worker.js` and `lumen_web.wasm` from the same directory. The
server must answer the module with `application/wasm` (streaming
instantiation; the glue degrades to array-buffer instantiation —
honest, not dead — when the MIME is wrong). No COOP/COEP headers are
needed: v1 is the single-threaded cooperative lane; the boot
handshake's capability snapshot says `threading: "single"`.

## The honest limits (this build)

See `src/capabilities.rs` — every gap is a typed row, never a crash
and never silence:

- **Decoders**: the wasm32 module ships the engine core WITHOUT the
  builtin decoders — the `codecs` crate's external decode engines do
  not build for `wasm32-unknown-unknown` (rav1d 1.1.0; measured —
  see the worklog). Native builds link the real MJPEG + PCM decoders
  (the first vertical), and the headless lane proof decodes for real.
  `[PROPOSAL: Lead splits the in-house decoders out of the codecs
  crate to lift this.]`
- **Renderer / audio output**: v1 presents to the headless counting
  presenter and the null audio device (cards 7.H2/7.H3).
- **Network**: `http(s)` sources answer the typed
  `unsupported-for-host` naming card 7.H4 (fetch + Range); raw
  UDP/RTSP are refused forever (doc 09 §10).
- **Seek**: refused typed (the seek boundary's host duties land with
  the H-pillar pump integration).

## Evidence labels

Proven headless-native by `cargo test -p lumen-web` (the SAME message
paths the JS glue drives, through the ABI's dispatch core with an
in-memory MessagePort double) and by the real wasm artifact from the
build above. **Browser-runtime behavior (worker instantiation,
`postMessage`, `setTimeout` re-arming) is `[VERIFY]`** — no browser
exists in this sandbox, and nothing here claims it was exercised.
