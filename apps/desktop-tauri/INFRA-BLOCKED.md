# apps/desktop-tauri — ADR-004 admission close package (issue #67)

> **Status: ADMITTED (code half) — 2026-11-13.** The wrapper exists, the
> anti-bypass audit exists, the crate compiles in its dedicated CI lane
> (`tauri-check`, Linux + webkit2gtk), and the 3-OS bundling + launch
> verdicts are the remaining half. This file supersedes the 4.H1
> INFRA-BLOCKED handoff (same document, rewritten; the 4.H1 text lives
> in git history at the move commit).

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
  ui/dist build (frontendDist embeds at compile time), `cargo test
  --locked --all-targets`, clippy `-D warnings`, cargo-deny (bans/
  licenses/advisories) over the subcrate lock with the repo-root
  deny.toml (the scoped `libloading` wrapper covers the tray-icon
  edge — an INACTIVE optional feature path, allowlisted only through
  `libappindicator-sys`, never blanket), and a link/exit smoke
  (0 or 86 acceptable).
- **The dependency record** (`docs/dependencies/records/tauri-wry-2.x.md`):
  verdict flipped to "imported (this crate's lockfile only)"; the
  workspace graph stays tauri-free. R1–R9 refreshes there.

## B. REMAINS (the launch half)

1. **3-OS bundling in release.yml** — the webview app joins the release
   lanes (payload lists for zip/tar.gz/AppImage/deb/rpm/dmg/MSI + the
   `--features` build step... not a feature anymore: the lane builds
   `--manifest-path apps/desktop-tauri/Cargo.toml`; the existing
   packaging scripts gain the third binary (BIN_TAURI via
   `product-env.py`).
2. **Launch verdicts** (ADR-004 decision 4 / the AC): window opens, file
   opens, playback + settings round-trip on Linux/Windows/macOS. Linux
   rides xvfb; macOS/Windows runners launch GUI apps natively. The
   `[VERIFY at the ADR-004 admission build]` marker in
   `ui/src/client/tauri-transport.ts` (the `plugin:event|listen` frame
   shape) clears ONLY when the launch lane proves an event frame
   round-trip — no earlier.
3. **Icons** (`bundle.icon: []`) + per-OS bundle metadata — a packaging
   follow-up (the existing lanes' icon pipeline owns it).
4. **The dialog plugin** (`pick_and_open`): needs its own admission row
   (capabilities stay `core:default`); in-webview inputs, drag-drop and
   argv cover the launch-verdict paths meanwhile.
5. **ADR-004 per-OS verdicts + the security review** (TB3: webview
   sandbox posture, IPC surface, navigation/content isolation) —
   execute against the real launched app, then flip the ADR rows.

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
