# apps/desktop/tauri — INFRA-BLOCKED build record (card 4.H1, honest AC)

> **Status: INFRA-BLOCKED — scaffold committed, cannot build or link in this
> sandbox.** This directory holds the Tauri 2.x shell scaffold (tauri.conf.json
> + capabilities/default.json). The Rust wrapper (the `#[tauri::command]`
> `lumen_dispatch` + `invoke_handler(tauri::generate_handler![lumen_dispatch])`
> shown in `src/dispatch.rs` module docs) deliberately does NOT exist yet:
> importing `tauri` here would (a) require linking WebKitGTK, which is absent
> below, and (b) put the unaudited tauri/wry/tao graph into `Cargo.lock` —
> both forbidden until the ADR-004 admission build runs on a desktop machine
> (`docs/dependencies/records/tauri-wry-2.x.md`: "exception-ADR-004, NOT
> imported").

## Why blocked — exact missing system deps (verified this session)

`pkg-config --exists <name>` results from this sandbox:

| pkg-config name | present? | Debian/Ubuntu package to install |
| --- | --- | --- |
| `webkit2gtk-4.1` | **MISSING** | `libwebkit2gtk-4.1-dev` |
| `javascriptcoregtk-4.1` | **MISSING** | `libjavascriptcoregtk-4.1-dev` |
| `gtk+-3.0` | **MISSING** | `libgtk-3-dev` |
| `soup-3.0` | **MISSING** | `libsoup-3.0-dev` |
| `xdo` | **MISSING** | `libxdo-dev` |
| `librsvg-2.0` | ok | `librsvg2-dev` (already present) |

Plus the usual Tauri 2.x Linux build prerequisites: `build-essential`,
`libssl-dev`, `libayatana-appindicator3-dev`, `curl`/`wget`/`file`. Even a
`cargo check` of a crate that merely *imports* tauri fails at the
`pkg-config` build script of `webkit2gtk-4.1` — there is no honest way to
"partially" build it here. No network install was attempted (sandbox has no
root; system packages are out of scope for the workspace).

## What IS proven (green, committed) vs what this file blocks

- **Proven in-sandbox:** `apps/desktop/src/dispatch.rs` (ShellHost + the
  single `lumen_dispatch` routing body + EventSink ordered exactly-once
  drain), `shell.rs` (issue #3: `ShellRuntime` — the composed app-layer
  runtime the wrapper's per-invoke body delegates to: dispatch + §4.2
  event filter + the bounded drop-oldest egress queue),
  `window_config.rs` (parsed AND consumed as a `WindowBuildSpec` at
  startup), `files.rs` (dialog/drop seams + the path-kind funnel), and
  the UI halves `ui/src/client/tauri-transport.ts` (invoke seam mockable,
  unit-tested), `ui/src/client/app-transport.ts` (the host probe that
  selects TauriTransport vs the browser demo fallback),
  `ui/src/client/open-path.ts` + `ui/src/client/config-settings.ts`
  (the UI-side file-open funnel and the engine-backed settings write
  path). `cargo test -p desktop` = 63/0 (lib 52/0, incl. a real-engine
  open→settle→events-in-order run, the config set→get round-trip, and
  the bounded-queue drop-oldest conservation proof); ui typecheck +
  vitest 634/0 green.
- **Blocked here (the launch/3-OS AC):** the `#[tauri::command]` wrapper +
  `tauri::Builder` run loop, `cargo build`/`tauri dev`/`tauri build`, and
  any window actually opening on Linux/Windows/macOS. The doc 05 §7.4
  anti-bypass architecture test ("invoke_handler registers exactly
  `lumen_dispatch`") is likewise a CI-row for the same reason — there is no
  invoke handler to read until the wrapper exists.

## How to unblock (ADR-004 admission checklist)

1. `sudo apt install libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev
   libgtk-3-dev libsoup-3.0-dev librsvg2-dev libxdo-dev libssl-dev
   libayatana-appindicator3-dev build-essential`
2. Add `tauri = "2"` (+ `tauri-build` build dep) to `apps/desktop/Cargo.toml`
   behind the ADR-004 admission gate; run the full tarball/graph audit per
   `docs/dependencies/audit-method.md` §4 and record it in
   `docs/dependencies/records/tauri-wry-2.x.md` (R1/R2 are [VERIFY-P1]).
3. Write the wrapper per the `src/dispatch.rs` module-doc signature
   (State-managed `ShellHost`, ONE registered command) delegating to
   `ShellRuntime` (`src/shell.rs` — dispatch, event pump,
   `take_pending_events` → `app_handle.emit("lumen://event", …)`;
   `pick_and_open`/`on_files_dropped` for the dialog/drop acquisitions;
   `WindowBuildSpec` from `window_config.rs` at window build), add the
   §7.4 architecture test, and verify the `plugin:event|listen` payload
   details marked [VERIFY] in `ui/src/client/tauri-transport.ts`.
4. Build + run on all three OSes (CI matrix per doc 10).

## Scaffold notes (deliberate, minimal)

- `tauri.conf.json`: bootstrap-only window geometry; the REAL window config
  is product-owned and read at runtime from `product.toml` `[window]` via
  `apps/desktop/src/window_config.rs` — the conf values are fallbacks for
  first paint, not a second source of truth. `csp: null` must be replaced
  with a real asset policy at the admission build (filed above).
- `capabilities/default.json`: `core:default` only. `lumen_dispatch` is a
  shell command (no permission entry needed); event listening is covered by
  `core:event:default`. No plugin permissions.
- `frontendDist` points at `ui/dist` (the vite build output).
