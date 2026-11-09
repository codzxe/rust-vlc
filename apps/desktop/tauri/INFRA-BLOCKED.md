# apps/desktop/tauri — INFRA-BLOCKED close package (issue #4 / card 4.H1)

> **Status: INFRA-BLOCKED — everything provable without a real OS webview is
> proven, committed, and green; the binary build + 3-OS launches need real
> machines.** This file is the definitive handoff for issue #4: §A what is
> DONE in-repo (each claim cites its file/commit), §B what REMAINS blocked
> with the exact unblock conditions, §C the run-order recipe for whoever gets
> the machines. The shell scaffold stays `tauri.conf.json` +
> `capabilities/default.json` only — the Rust wrapper deliberately does NOT
> exist yet: importing `tauri` here would (a) require linking WebKitGTK,
> absent below, and (b) put the unaudited tauri/wry/tao graph into
> `Cargo.lock` — both forbidden until the ADR-004 admission build runs on a
> desktop machine (`docs/dependencies/records/tauri-wry-2.x.md`: verdict
> "exception-ADR-004, NOT imported").

## A. DONE in-repo (typed seams, landed and tested)

Landed by the 4.H1 app-layer session — commit `4439777`, merge `8a56f0b`
(issue #3), with the follow-up `7b78362` (issue #37). Nothing in §A imports
tauri; all of it compiles and tests std-only.

- **Rust halves** (`cargo test -p desktop`, all green — see counts below):
  - `src/dispatch.rs` — THE single `lumen_dispatch` routing body + the
    `EventSink` ordered exactly-once drain (doc 05 §5.1 single-command rule);
    the `#[tauri::command]` wrapper's verbatim signature lives in its module
    docs, ready to copy at admission time.
  - `src/shell.rs` — `ShellRuntime`: the composed app-layer runtime the
    wrapper's per-invoke body delegates to (dispatch + §4.2 event filter +
    the bounded drop-oldest egress queue), plus the file-open acquisitions
    (`open_path` / `on_files_dropped` / `pick_and_open`) funneling through
    the same command, and `take_pending_events` — exactly what the wrapper
    emits on `"lumen://event"`.
  - `src/window_config.rs` — parsed AND consumed as a `WindowBuildSpec` at
    startup; `src/files.rs` — dialog/drop seams + the path-kind funnel.
  - Phase-2 core underneath (card 2.D3): `wire.rs`/`bridge.rs`/`mailbox.rs`/
    `transport.rs`/`host.rs`/`error.rs` — std-only, incl. the typed
    always-refusing `open_webview()` → `HostError::WebviewPending` and
    `WebviewHostKind` gated on `GATING_ADR = "ADR-004"`.
- **UI halves**: `ui/src/client/tauri-transport.ts` (the mockable invoke
  seam, unit-tested), `ui/src/client/app-transport.ts` (the host probe that
  selects TauriTransport vs the browser demo fallback),
  `ui/src/client/open-path.ts` + `ui/src/client/config-settings.ts` (the
  UI-side file-open funnel and the engine-backed settings write path).
- **Test counts (honest, at close HEAD `913008d`)**:
  - `cargo test -p desktop` = **63 passed / 0 failed**, verified this close
    session in two consecutive full runs: lib 52 + `tests/pipeline` 4 +
    `tests/protocol` 4 + `tests/zero_alloc_transport` 2 + doctest 1. (One
    observed flake, recorded for honesty: the zero-alloc warm-up row failed
    once under parallel sandbox load, then passed in isolation and in both
    full re-runs — it is load-sensitive, not shell-related.)
  - ui vitest = **634/0 at the 4.H1 landing** (`4439777`/`8a56f0b`). Not
    re-run at close: this sandbox has no `ui/node_modules`; CI re-runs it
    (`npm ci` + vitest, ubuntu, merge-blocking — `.github/workflows/ui.yml`)
    on every PR.
- **Scaffold + design notes** (this directory): `tauri.conf.json` is
  bootstrap-only — the REAL window config is product-owned, read at runtime
  from `product.toml` `[window]` via `src/window_config.rs`; conf values are
  first-paint fallbacks, not a second source of truth. `csp: null` is a
  known debt owed at admission (§B.4). `capabilities/default.json`:
  `core:default` only — `lumen_dispatch` is a shell command (no permission
  entry), event listening covered by `core:event:default`, no plugins.
  `frontendDist` → `ui/dist` (the vite build output).

## B. REMAINS BLOCKED — exact unblock conditions

Every item below needs a real desktop machine and/or the ADR-004 admission.
None can be honestly closed in this sandbox (re-verified at close HEAD
`913008d`):

1. **Linux system deps** — `pkg-config --exists` re-verified this session:

   | pkg-config name | present? | Debian/Ubuntu package |
   | --- | --- | --- |
   | `webkit2gtk-4.1` | **MISSING** | `libwebkit2gtk-4.1-dev` |
   | `javascriptcoregtk-4.1` | **MISSING** | `libjavascriptcoregtk-4.1-dev` |
   | `gtk+-3.0` | **MISSING** | `libgtk-3-dev` |
   | `soup-3.0` | **MISSING** | `libsoup-3.0-dev` |
   | `xdo` | **MISSING** | `libxdo-dev` |
   | `librsvg-2.0` | ok | (already present) |

   Plus `build-essential`, `libssl-dev`, `libayatana-appindicator3-dev`,
   `curl`/`wget`/`file`. Even a `cargo check` that merely *imports* tauri
   fails at the `webkit2gtk-4.1` build script; there is no honest partial
   build. No root in this sandbox; system packages out of scope.
2. **`tauri = "2"` admission** (the dependency gate, comes FIRST): full
   tarball/graph audit per `docs/dependencies/audit-method.md` §4, filling
   the R1/R2 `[VERIFY-P1]` fields in `docs/dependencies/records/
   tauri-wry-2.x.md`, plus the Lead allowlist row (final acceptance is a
   Lead decision, doc 11 §3.1). Only then may `tauri` + `tauri-build` enter
   `apps/desktop/Cargo.toml`.
3. **The `#[tauri::command]` `lumen_dispatch` wrapper + run loop** — write
   per the verbatim signature in `src/dispatch.rs` module docs
   (State-managed `ShellHost`, ONE registered command via
   `invoke_handler(tauri::generate_handler![lumen_dispatch])`), delegating
   to `ShellRuntime` (`src/shell.rs`); `take_pending_events` →
   `app_handle.emit("lumen://event", …)`; `pick_and_open` /
   `on_files_dropped`; `WindowBuildSpec` at window build. Includes the
   **doc 05 §7.4 anti-bypass architecture test** (fails if any command
   other than `lumen_dispatch` is registered) — impossible until the
   handler exists.
4. **Real CSP** — replace `csp: null` in `tauri.conf.json` with a real
   asset policy at the admission build.
5. **`[VERIFY]` payload details** — the `plugin:event|listen` frame shape
   marked `[VERIFY at the ADR-004 admission build]` in
   `ui/src/client/tauri-transport.ts`.
6. **3-OS build + launch verdicts** — the actual launch AC: window opens,
   file opens, playback + settings round-trip on Linux/Windows/macOS.
   ADR-004 per-OS verdicts stay PENDING until real runs (no flips without
   them); ADR-003's per-OS presentation gates ride the same runs
   (`docs/prototypes/video-in-webview.md` §"What is PENDING").
7. **Security review** owed at this card per ADR-004 decision 4: webview
   sandbox posture, the IPC surface, navigation/content isolation (TB3,
   doc 11 §5) — executes against the real import.

## C. Handoff recipe — exact commands, in order

```bash
# 0) A real desktop machine. Linux shown (root); equivalents:
#    macOS: xcode-select --install (WKWebView is system);
#    Windows: VS Build Tools/MSVC + WebView2 (preinstalled Win 10/11).
# 1) System deps (Linux):
sudo apt install libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
  libgtk-3-dev libsoup-3.0-dev librsvg2-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev build-essential
# 2) ADMISSION AUDIT FIRST (dependency policy — before tauri enters Cargo.lock):
#    run the audit-method.md §4 tarball/graph scan for tauri 2.x + wry + tao;
#    fill R1/R2 [VERIFY-P1] in docs/dependencies/records/tauri-wry-2.x.md;
#    get the Lead allowlist row (doc 11 §3.1). THEN add to apps/desktop/Cargo.toml:
#      tauri = "2"   (+ tauri-build as build-dependency)
# 3) Write the wrapper per the verbatim signature in apps/desktop/src/dispatch.rs
#    module docs, delegating to ShellRuntime (apps/desktop/src/shell.rs);
#    add the §7.4 anti-bypass architecture test;
#    replace csp:null in apps/desktop/tauri/tauri.conf.json with a real asset policy;
#    clear the [VERIFY] marker in ui/src/client/tauri-transport.ts.
# 4) Prove the graph compiles + nothing regressed (today's baseline: 63/0):
cargo test -p desktop
# 5) Build the UI the shell embeds (frontendDist = ui/dist):
npm --prefix ui ci && npm --prefix ui run build
# 6) Dev-run + package (tauri-cli is a dev-side tool, not a workspace dep —
#    Lead call if policy wants it recorded):
cargo install tauri-cli --version "^2"    # if absent
cargo tauri dev                           # window opens: open a file, play, settings
cargo tauri build                         # bundle targets "all"
# 7) Repeat 4–6 on all three OSes; record each verdict in
#    docs/adr/ADR-004-os-webview.md (flip PENDING only on real runs) and the
#    to-run list in docs/prototypes/video-in-webview.md.
# 8) CI: the 3-OS check/clippy/test matrix already exists non-blocking
#    (.github/workflows/ci-full.yml, issue #5); extend it with the tauri
#    build job and flip to blocking once the admission is accepted
#    (minute-economics note in .github/workflows/ci.yml header).
```
