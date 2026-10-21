# apps/desktop — Lumen desktop composition-root skeleton

Card **2.D3** (doc 13 wave 2.D) · ADR-003 evidence vehicle · doc 01
§2.7 registry row (L6, "Phase 2 skeleton → Phase 4 product").

**Status:** the variant-B frame-streaming prototype core — std-only,
no webview compiled. The Tauri/wry import is a typed pending state
behind **ADR-004** (see
`docs/dependencies/records/tauri-wry-2.x.md` for the environment
evidence: this build sandbox has no webkit2gtk-4.1 dev packages and
no display server).

## What is here

| Path | What it is |
|---|---|
| `src/wire.rs` | The frame-streaming wire protocol v1 (`LUF1` records — the spec table is in the module docs) |
| `src/bridge.rs` | `CanvasBridge` — the `Presenter` that serializes presentation decisions into records |
| `src/transport.rs` | The `FrameTransport` seam (null / capture / mailbox) + typed ADR-004 pending states |
| `src/mailbox.rs` | The bounded latest-wins engine→webview hand-off, drop accounting, latency ledger |
| `src/host.rs` | The command-layer correlation seam (doc 05 §4 host half) + the ADR-003 per-OS landing as data |
| `src/main.rs` | The honest skeleton binary (`lumen-desktop`, exit 86 = webview pending) |
| `prototype/` | The canvas-side reference implementation (browser + Node) + the conformance harness |
| `examples/emit_stream.rs` | Emits the golden `stream.bin` + `expected.json` the Node harness checks |

## The deliverable

`docs/prototypes/video-in-webview.md` — everything measured, everything
pending, and the exact run-book for a desktop machine.

## Running what can run headlessly

```sh
cargo test -p desktop                     # protocol + bridge + mailbox + host suites
cargo run -p desktop --example emit_stream -- --out target/proto   # golden stream
node prototype/harness.mjs target/proto   # cross-language conformance (Rust → JS)
cargo run -p desktop                      # the honest status report (exit 86)
```
