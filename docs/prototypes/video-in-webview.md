# Video-in-Webview Prototype — card 2.D3 evidence report (ADR-003 input)

Status: **B-protocol proven at the protocol/virtual-time level; per-OS browser
measurement PENDING (headless sandbox — no webview runtime).** This is the
honest state per doc 13 card 2.D3's AC ("measured latency + compatibility
matrix recorded; honest recommendation written for ADR-003"): everything
measurable in this sandbox is measured; everything needing a desktop browser
runtime is listed as a to-run with exact commands.

## What was built (variant B first, per the card + ADR-003)

- `apps/desktop` crate: the frame-streaming protocol core behind the ADR-004
  engine/host seam — `wire.rs` (the LUF1 v1 wire format), `bridge.rs` (the
  renderer interop: consumes renderer-wgpu presents + repeats),
  `mailbox.rs` (bounded latest-wins mailbox with slow-consumer policy),
  `transport.rs` (byte transport + LatencyAccountant), `host.rs` (the host
  trait the future Tauri shell implements), `error.rs`. 2,541 LOC, std-only,
  zero new dependencies (the tauri/wry shell is a later card — this crate is
  the engine-side half that does not need it).
- `apps/desktop/prototype/lumen-frames.mjs`: the canvas-side reference
  implementation — dependency-free, byte-for-byte mirror of `wire.rs`, runs
  in Node and every browser; canvas presenter with 2D and WebGL2 backends
  (feature-detected).
- `apps/desktop/examples/emit_stream.rs`: emits a golden LUF1 stream (the
  conformance corpus for the JS parser).

## What WAS measured (this sandbox, deterministic, virtual time)

The `apps/desktop/tests/` suites (32 tests):

- **Protocol conformance**: renderer-interop round-trip (60 frames through
  bridge→mailbox→transport with per-frame payload hash equality), 1080p
  frames, stream offset handling, malformed-input typed refusals; the JS
  parser conformance runs over the Rust-emitted golden stream.
- **End-to-end pipeline latency (virtual time)**: the exact nominal
  pipeline — 30 fps present → mailbox → transport draw — delivers every
  frame **in order, min 11,666,667 ns (11.67 ms) end-to-end** (one frame
  period + the transport hop), exact and reproducible (zero sleeps; virtual
  clock per doc 03 §8).
- **Slow-consumer policy**: latest-wins mailbox drops the correct (oldest)
  frames under a consumer slower than the producer; no reordering, no
  stalling.
- **Generation flush**: a generation switch purges in-flight frames (the
  seek/stop safety property).

## What is PENDING (needs a desktop machine — exact to-run list)

The per-OS browser matrix (WebView2 / WKWebView / WebKitGTK) cannot run
here: no display server, no browser runtime, no webview SDK. On a machine
with the three runtimes:

1. `node apps/desktop/prototype/harness.mjs` — the JS parser conformance
   over the golden stream (also runs headless in CI with Node).
2. Serve `apps/desktop/prototype/` and drive the canvas presenter per
   browser; record present-latency (the `emitted_ns` header field makes the
   measurement self-contained: `performance.now() - emitted_ns` at present
   time), CPU, compositing artifacts, fullscreen, multi-monitor.
3. Record verdicts into the matrix below and finalize ADR-003 §"Phase 2
   prototype gates".

## Compatibility matrix (honest current state)

| Runtime | Protocol | Presenter | Latency measured? | Status |
|---|---|---|---|---|
| Node (parser only) | LUF1 v1 conformance green | n/a | n/a | proven (this sandbox) |
| WebView2 (Chromium) | expected green [VERIFY on desktop] | WebCodecs/ImageBitmap path [VERIFY] | pending | to-run |
| WKWebView (macOS) | expected green [VERIFY] | floors per ADR-003 (16.4+/26) | pending | to-run |
| WebKitGTK (Linux) | expected green [VERIFY] | per ADR-003 R-04 | pending | to-run |

## Recommendation status for ADR-003

**PROVISIONAL-B stands** (doc 08 §7's B-first + ADR-003's per-OS landing):
the B protocol core is proven end-to-end at the deterministic level, with
exact latency accounting built into the wire format. The per-OS A-vs-B
verdicts remain ADR-003's own per-OS gates, honestly pending the desktop
runs above. Nothing here contradicts the accepted decision; the Linux-B
contingency (R-04) remains bounded-and-planned.
