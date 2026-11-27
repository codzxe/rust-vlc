# apps/desktop-tauri — ADR-004 admission close package (issue #67)

> **Status: ADMITTED (code half) — 2026-11-13; compile claim
> corrected 2026-11-25 (FX1-A01 / FIX-PLAN F0.1); compile-status role
> demoted 2026-11-26 (FX8-A10 / FIX-PLAN §1 Principle 1 + §8.4 — this
> file owns the infra record with its dates, never a status layer).**
> The wrapper exists, the anti-bypass audit exists, and the E0425 that
> broke this crate's build on EVERY host is gone after FX1-A01:
> app.rs:301 called a ticker name that is defined nowhere in the
> repository — a one-token typo for the real ticker `spawn_event_pump`
> (app.rs:190; the Devil audit's S0, four independent confirmations) —
> and the call site now names the defined function. **The compile
> status is pointed at, not claimed: the compiles oracle is CI — the
> `tauri-check` lane in ci.yml (Linux + webkit2gtk; explicit
> `cargo build --manifest-path apps/desktop-tauri/Cargo.toml`, lib +
> the real `lumen-tauri` bin, ahead of `cargo test --all-targets`;
> FX1-A01) and the `tauri-launch.yml` launch half — and the
> capability/status truth is the ledger `docs/feature-matrix.toml` +
> the generated `docs/generated/status-truth.toml` (drift gate:
> `bash scripts/quality/status_truth_check.sh`).** Honesty note: this
> sandbox cannot compile the crate (no webkit2gtk/gdk-sys — the baseline
> dies at gdk-sys before these sources are type-checked), so local proof
> is rg-level only (the never-defined name → 0 hits anywhere under
> apps/desktop-tauri/; `fn spawn_event_pump` → the one definition); the
> 2026-11-13 "the crate compiles" sentence was FALSE against that tree,
> and the green-run state of the fixed tree is the lane's own record,
> never this file's claim — the lane is the authority that proves the
> compile, not this file. The 3-OS bundling
> + launch verdicts are the remaining half. This file supersedes the
> 4.H1 INFRA-BLOCKED handoff (same document, rewritten; the 4.H1 text
> lives in git history at the move commit).
>
> **w30 progress (2026-11-15, this wave):** §B.1 is DONE (the real app
> ships in every payload; the stub is gone), §B.2 is advanced (Windows
> launch smokes ride the release lane; deep verdicts still manual), the
> crash face + the playback pacer landed, and the WebView2 posture has
> a visible-dialog/crash-log half — see the §B notes. **FX1-A01
> correction (2026-11-25; pointer-ized 2026-11-26, FX8-A10):** §B.1's
> DONE was false while the E0425 stood — the release lane builds
> this crate and could not pass; it becomes true again only on a green
> release-lane run of the fixed tree, and that green-run state is the
> release lane's own record — never this file's claim.

## A. DONE at admission (2026-11-13, issue #67)

- **The crate**: `apps/desktop-tauri` — workspace-EXCLUDED (root
  Cargo.toml `exclude`; see its Cargo.toml header for the design note:
  `--all-features` would have dragged webkit + ui/dist into every main
  lane). Own lockfile (`Cargo.lock`, 468 packages incl. the tauri
  closure). Edges: `desktop` (path, the shell library), `commands`
  (path, the shared wire envelope), `tauri` (registry, E1 exception).
- **The wrapper** (`src/app.rs`): the verbatim `#[tauri::command]
  lumen_dispatch` from `apps/desktop/src/dispatch.rs`'s module docs —
  `State<ShellState>` + `WireRequest` in, `Result<WireResponse,
  ShellError>` out; the 40 ms event pump; `apply_window_spec` from
  `product.toml [window]`; drag-drop + argv file-open through the
  production `PathDragDrop` handler; ordering discipline under one
  `Mutex<ShellRuntime>` (doc 05 §4.3 guarantee 2).
- **The §7.4 anti-bypass audit** (`tests/single_command.rs`): reads the
  registration site, fails on any second command; plus a
  unique-site check. (Local simulation PASS; CI compiles it for real.)
- **Real CSP** (`tauri.conf.json`): `default-src 'self'` … with the
  tauri IPC (`connect-src ipc: http://ipc.localhost`), `style-src
  'unsafe-inline'` for the UI's dynamic inline styles (sized sliders,
  positioned overlays), media/blob for playback, and hard `none`s for
  form/object/frame. `csp: null` is gone.
- **Capabilities**: unchanged `core:default` only (no plugins — the
  4.H1 posture).
- **CI lane** (`ci.yml` → `tauri-check`): webkit2gtk dev packages, the
  ui/dist build (frontendDist embeds at compile time), `cargo build
  --locked` over the whole crate (F0.1/FX1-A01: lib + the real
  `lumen-tauri` bin — the GUI binary itself, not just test-harness
  targets), `cargo test --locked --all-targets`, clippy `-D warnings`,
  cargo-deny (bans/licenses/advisories) over the subcrate lock with the
  repo-root deny.toml (the scoped `libloading` wrapper covers the
  tray-icon edge — an INACTIVE optional feature path, allowlisted only
  through `libappindicator-sys`, never blanket), and an ldd link smoke (the
  built `target/debug/lumen-tauri`: every dynamic dependency resolves;
  no exec on a display-less host — that half is tauri-launch.yml's).
- **The dependency record** (`docs/dependencies/records/tauri-wry-2.x.md`):
  verdict flipped to "imported (this crate's lockfile only)"; the
  workspace graph stays tauri-free. R1–R9 refreshes there.

## B. REMAINS (the launch half)

1. ~~**3-OS bundling in release.yml** — the webview app joins the release
   lanes (payload lists for zip/tar.gz/AppImage/deb/rpm/dmg/MSI + the
   `--features` build step... not a feature anymore: the lane builds
   `--manifest-path apps/desktop-tauri/Cargo.toml`; the existing
   packaging scripts gain the third binary (BIN_TAURI via
   `product-env.py`).~~ **DONE (w30.8/w30.9):** the webview app ships in
   every release payload; the exit-86 `lumen-desktop` stub LEFT every
   user payload (zip/tar.gz/AppImage/deb/rpm/dmg + the MSI, component
   dropped in both twins), and the GUI payloads exec the real shell —
   the AppImage `AppRun` and the macOS `CFBundleExecutable` launch
   `lumen-tauri`, not the stub.
2. **Launch verdicts** (ADR-004 decision 4 / the AC): window opens, file
   opens, playback + settings round-trip on Linux/Windows/macOS. Linux
   rides xvfb; macOS/Windows runners launch GUI apps natively. The
   `[VERIFY at the ADR-004 admission build]` marker in
   `ui/src/client/tauri-transport.ts` (the `plugin:event|listen` frame
   shape) clears ONLY when the launch lane proves an event frame
   round-trip — no earlier. **w30.8 progress:** the release lane now
   carries the Windows launch smokes — the zip lane launches the
   extracted `lumen-tauri.exe` (10 s survival + failure receipts) and
   the MSI upgrade test launches the installed exe; the
   `tauri-launch.yml` deep verdicts (window/file/playback/settings per
   OS, the event-frame round-trip) remain manual.
3. **Icons** (`bundle.icon: []`) + per-OS bundle metadata — a packaging
   follow-up (the existing lanes' icon pipeline owns it).
4. **The dialog plugin** (`pick_and_open`): needs its own admission row
   (capabilities stay `core:default`); in-webview inputs, drag-drop and
   argv cover the launch-verdict paths meanwhile.
5. **ADR-004 per-OS verdicts + the security review** (TB3: webview
   sandbox posture, IPC surface, navigation/content isolation) —
   execute against the real launched app, then flip the ADR rows.
6. **WebView2 posture** (the audit's open decision): a missing/broken
   WebView2 runtime was the top silent-exit candidate — **the failure
   is visible now** (w30.6: panic hook + crash log under `<tmp>/lumen/`
   + a Windows no-console native MessageBox carrying the WebView2
   actionable hint). Still open: bundling the Evergreen bootstrapper in
   the MSI vs. requiring the runtime — the no-bootstrapper assumption
   is still a pinned comment in the twins; decide and record it.

## C. Recipe (the release half, in order)

```bash
# 0) CI lanes do this; locally you need the webkit deps (§A) and:
npm --prefix ui ci && npm --prefix ui run build
# 1) compile + audit:
cargo test  --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets
cargo clippy --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets -- -D warnings
# 2) run it (a display or xvfb on Linux):
./apps/desktop-tauri/target/debug/lumen-tauri file.mkv
# 3) bundle: the release-lane integration (B.1) drives the packaging
#    scripts; tauri-cli is NOT used (no dev-side tool dep; the existing
#    WiX/ditto/appimagetool lanes own the installers).
```
