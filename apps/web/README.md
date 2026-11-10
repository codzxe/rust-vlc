# apps/web — the engine-in-Worker wasm host (cards 7.H1/7.H2/7.H3/7.H4, issue #47 + issue #48)

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
| `src/abi.rs` | The `extern "C"` exports + the dispatch core (`AbiWorker`) — no `unsafe`, no deps; the raw frame lane (`lumen_web_frame_addr`/`len`) stages present bytes apart from the JSON |
| `src/host.rs` | `WebHost`: the engine, the cooperative pump/slice budgets (doc 09 §3.2), the render lane + present mailbox + autoplay gate wiring, the served command surface |
| `src/render/` | Card 7.H2 (doc 09 §4): the backend negotiation state machine (WebGPU → WebGL2 → honest `unavailable`), the DPR-aware canvas seam, and the CPU RGBA upload path + latest-wins present mailbox |
| `src/audio/` | Card 7.H3 (doc 09 §5): the AudioWorklet sink model, the sample-counter clock model (drift/wallclock/resample arithmetic), and the autoplay-policy gate (`paused-pending-gesture`, never a fake playing state) |
| `src/io/` | Card 7.H4 (doc 09 §6): the fetch seam (`FetchPort`), seekability detection, and the staged bounded-window source |
| `src/open.rs` | The open path: blob registry → probe → AVI wiring → `SessionConfig` |
| `src/source.rs` | The in-memory `BlobSource` + bounded `BlobRegistry` |
| `src/capabilities.rs` | The honest capability snapshot (`threading: single`, doc 09 §10 as typed rows) |
| `worker.js` | The minimal hand-written Worker glue (postMessage ↔ wasm ABI; drains the JSON output lane and the raw frame lane, re-arms the pump) |
| `audio-worklet.js` | The hand-written AudioWorklet processor — the audited transcription of the tested `WorkletQueue` model (bounded queue, 128-frame quantum, silence-on-underrun, played counter) |

## Building the module

```sh
cargo build -p lumen-web --target wasm32-unknown-unknown --locked
cp $CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/lumen_web.wasm ./
```

Serve `worker.js`, `audio-worklet.js` and `lumen_web.wasm` from the
same directory. The server must answer the module with
`application/wasm` (streaming instantiation; the glue degrades to
array-buffer instantiation — honest, not dead — when the MIME is
wrong). No COOP/COEP headers are needed: v1 is the single-threaded
cooperative lane; the boot handshake's capability snapshot says
`threading: "single"`. The page probes `navigator.gpu` /
`getContext("webgl2")` BEFORE sending `boot` (the probe facts ride
the handshake; the snapshot reports the ACTIVE backend — doc 09
§4.4), owns the `AudioContext` + gesture (doc 09 §5.3), and loads
`audio-worklet.js` through `audioContext.audioWorklet.addModule`
when it wires the audio lane (card 7.I1).

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
- **Renderer / audio output**: the typed render seam, the backend
  negotiation ladder and the CPU RGBA upload path landed (7.H2), and
  the AudioWorklet sink model + clock arithmetic + autoplay gate
  landed (7.H3) — all headless-proven. The BROWSER halves (the real
  `navigator.gpu` probe, the canvas draw, `AudioContext` creation,
  chunk shipping, counter feedback) are card 7.I1's `[VERIFY]`
  matrix; this build's own output lanes remain the headless counting
  presenter and the null audio device, and the snapshot says so.
- **Network**: the fetch seam + seekability detection + staged source
  landed (7.H4), proven against the mock double; real-`fetch()`
  behavior (CORS, preflight, redirects) is `[VERIFY]` (card 7.I1).
  Raw UDP/RTSP are refused forever (doc 09 §10).
- **Seek**: refused typed (the seek boundary's host duties land with
  the H-pillar pump integration).

## Evidence labels

Proven headless-native by `cargo test -p lumen-web` (the SAME message
paths the JS glue drives, through the ABI's dispatch core with an
in-memory MessagePort double — boot with simulated capability
reports, the frame lane's staged RGBA bytes, the loss ladder, the
autoplay park, the mock-fetch playback) and by the real wasm artifact
from the build above. **Browser-runtime behavior (worker
instantiation, `postMessage`, `setTimeout` re-arming, the real
`navigator.gpu`/WebGL2 probes, `AudioWorklet` on the real-time audio
thread, `fetch()`) is `[VERIFY]`** — no browser exists in this
sandbox, and nothing here claims it was exercised.
