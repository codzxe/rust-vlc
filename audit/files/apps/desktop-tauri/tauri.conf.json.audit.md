# AUDIT: apps/desktop-tauri/tauri.conf.json
| Field | Value |
|---|---|
| Auditor | W14-A08 |
| Date | 2026-11-24 |
| Lines / config keys | 34 / 16 (incl. 2 absent-but-relevant: devUrl, transparent) |
| File verdict | REAL-UNPROVEN (genuine config, compiled into the shipped binary; not executable in this sandbox — gdk-sys, baseline) |
| Reachable from shipped entry point? | YES: lumen-tauri main.rs:50 `lumen_desktop_tauri::app::run()` → app.rs:321 `tauri::generate_context!()` embeds this file; built by CI `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` (.github/workflows/tauri-launch.yml:156) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | sed -n on all quoted ranges (exit 0); od -c window_config.rs:129-146 (exit 0); `ls ui/dist` (exit 2 — absent, gitignored); `ls assets/brand` (exit 2 — absent); file+md5sum icons (exit 0); grep tauri-build-2.7.4 + tauri-2.12.5 registry sources (exit 0); grep repo for tauri-cli/"tauri build" (no matches, exit 1); baseline cites (see §2) |

## 1. One-paragraph verdict
This is a real, lean Tauri v2 config that is genuinely compiled into the shipped desktop shell, and its three load-bearing parts hold up under cross-examination: the CSP is real and hardened (matches the INFRA-BLOCKED.md §A claim byte-for-byte), `frontendDist` correctly targets the vite output (`ui/dist`, CI-asserted before compile), and the frameless window (`decorations: false`) is consistently wired to the capabilities' four window grants and the UI's custom title bar. The two defects that matter: (1) the first-paint height 800 contradicts product.toml's 720 — tauri.conf.json does NOT override product.toml; the vendored tauri-2.12.5 source proves the config window is created at 1280x800 and only THEN resized to 1280x720 by `apply_window_spec`, so every launch flashes an 80px-wrong window (and `remember_geometry = true` is unimplemented, so it never stops flashing); (2) the `bundle` section half-lies: `targets: "all"` and `icon.png` are consumed by NOTHING in the shipped pipeline (tauri-cli is never invoked; the WiX/ditto/appimagetool scripts own packaging), only the `.ico` and `version` are live, and only on Windows.

## 2. What actually works (verified)
— CSP matches the doc claim byte-for-byte: conf:23 vs INFRA-BLOCKED.md:34-38 (`sed -n '34,40p'`), both listing `connect-src ipc: http://ipc.localhost` and `style-src 'unsafe-inline'`; script-src has no unsafe-eval/inline; form/object/frame are hard `none`.
— Window-creation ordering proven from the vendored dep: tauri-2.12.5 `src/app.rs` `fn setup()` creates config windows BEFORE the app's setup callback — `for window_config in app.config().app.windows.iter().filter(|w| w.create) { WebviewWindowBuilder::from_config(...)?.build()?; }` then `(setup)(app)` — so conf sizes are first-paint, `apply_window_spec` (app.rs:237-268) re-asserts product.toml afterwards.
— `withGlobalTauri: false` is consistent with the UI: tauri-2.12.5 `src/manager/webview.rs:174-175` unconditionally injects `window.__TAURI_INTERNALS__` (grep), which is exactly what ui/src/client/tauri-transport.ts binds to; ui/package.json carries no `@tauri-apps/api` (deps: react, react-dom, react-router-dom only).
— `frontendDist: "../../ui/dist"` is correct: ui/vite.config.ts sets no `outDir` (default `dist`); CI builds it and hard-asserts `test -f ui/dist/index.html` (ci.yml:719-723, tauri-launch.yml:144-148); baseline ui-build.txt ends `✓ built in 389ms` with `dist/assets/...` output. `ui/dist` is absent in-tree because ui/.gitignore:2 is `dist/` — build artifact, as designed.
— Capabilities wiring matches: no `capabilities` key in conf (Tauri v2 auto-discovers `capabilities/`), and capabilities/default.json:5 `"windows": ["main"]` matches conf:13 `"label": "main"`, app.rs:249 and frame_channel.rs:45 `get_webview_window("main")`.
— Icons exist and are valid: `file` → icon.ico "MS Windows icon resource - 1 icon, 256x256", icon.png "PNG image data, 512 x 512".
— tauri-build-2.7.4 consumes `bundle.icon`'s `.ico` and `version` on Windows compiles (exe icon + FileVersion/ProductVersion): registry src/lib.rs:797-809 (`config.bundle.icon.iter().find(|i| i.ends_with(".ico"))`) and :826-829.

## 3. Findings
F-001 [S3]
Where: apps/desktop-tauri/tauri.conf.json:15-16 vs product.toml:64-65
Code:
```
        "width": 1280,
        "height": 800,
```
vs
```
width = 1280                  # schema 480..=3840
height = 720                  # schema 320..=2160
```
What is wrong: First-paint height 800 contradicts the product spec's 720 (width and both mins agree — 940x600, the w28 fix 5 landed on both sides, product.toml:66-71). Answer to the Wave 11 open question (audit/runs/W11-A06.log.md:116, runtime.rs.audit.md:93): the conf does NOT override product.toml — tauri's internal setup creates the window from the conf at 1280x800 (tauri-2.12.5 src/app.rs:2691-2692), then Lumen's `apply_window_spec` (app.rs:255-258 `window.set_size(tauri::LogicalSize::new(f64::from(spec.inner_size.0), f64::from(spec.inner_size.1)))`) resizes to 1280x720. Why it matters: every launch paints an 80px-too-tall window then visibly jumps to 720; since `remember_geometry = true` (product.toml:74) is parsed but dropped — `apply_window_spec` never reads `spec.remember_geometry` despite window_config.rs:133-134 claiming "the wrapper branches on config (restore persisted geometry vs apply `inner_size`), never on a guess" — there is no persistence to ever mask the flash. The file is the one place the fix belongs: height 800 → 720 (or visible:false + show after sizing). What correct looks like: conf height 720 (or apply before show). Confidence: HIGH — Verification: read + vendored-source proof; executed UNVERIFIED (desktop-tauri cannot build here: baseline cargo-build-desktop-tauri.txt "The system library `gdk-3.0` required by crate `gdk-sys` was not found").

F-002 [S3]
Where: apps/desktop-tauri/tauri.conf.json:26-33
Code:
```
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/icon.png",
      "icons/icon.ico"
    ]
  }
```
What is wrong: `targets: "all"` and `icon.png` are consumed by nothing shipped. Repo-wide grep for tauri-cli users finds exactly three references: INFRA-BLOCKED.md:105 (the "NOT used" statement itself), docs/10-testing-ci-quality.md:85 (documents gate row 9), and xtask gate.rs:259-260 — `argv: &["cargo", "tauri", "build", "--debug", "--no-bundle"]` with cwd `apps/desktop` — the WRONG crate (no tauri.conf.json there; already filed as guaranteed-fail by the gate.rs audit) and `--no-bundle` skips bundling regardless. Every real lane compiles with plain cargo (tauri-launch.yml:156, ci.yml:731) and INFRA-BLOCKED.md:104-106 states "tauri-cli is NOT used (no dev-side tool dep; the existing WiX/ditto/appimagetool lanes own the installers)". Only the `.ico` entry and top-level `version` are live — Windows-only, via tauri-build's exe resource (registry src/lib.rs:797-809). Why it matters: a reader of this file believes the tauri bundler produces "all" package formats; in reality scripts/packaging/{build-msi.ps1,build-appimage.sh,build-deb.sh,build-rpm.sh,build-dmg.sh,build-zip.sh,build-targz.sh} own packaging (W17 cross-ref), and there is no `.icns` here for a macOS tauri bundle anyway. What correct looks like: drop `targets`, or comment the section as Windows-resource-only. Confidence: HIGH — Verification: read + grep (no packaging-lane consumer exists).

F-003 [S4]
Where: apps/desktop-tauri/tauri.conf.json:4; cross-ref tools/xtask/src/cmd/rebrand.rs:15
Code:
```
  "version": "0.1.3",
```
and rebrand.rs:15 lists the rewrite surface as "`productName`, window `title`, `identifier` (reverse-DNS + `.desktop`)" — version is not a token. What is wrong: the version is hand-synced in three places (conf:4 = apps/desktop-tauri/Cargo.toml:25 `version = "0.1.3"` = product.toml:27 `display_version = "0.1.3"` — in sync TODAY), but rebrand rewrites product.toml's display_version while leaving conf's version stale, producing an internally inconsistent installer face in every rebranded dest (already filed as rebrand.rs audit F-003). What correct looks like: rebrand token for conf version, or a generator. Confidence: HIGH — Verification: read (rebrand.rs audit cross-ref).

F-004 [S4]
Where: apps/desktop-tauri/INFRA-BLOCKED.md:39-40 and :78-79 vs this file
Code:
```
- **Capabilities**: unchanged `core:default` only (no plugins — the
  4.H1 posture).
```
and
```
3. **Icons** (`bundle.icon: []`) + per-OS bundle metadata — a packaging
   follow-up (the existing lanes' icon pipeline owns it).
```
What is wrong: both claims about this file's neighborhood are stale: capabilities/default.json:6-12 now grants `core:default` PLUS four `core:window:allow-*` permissions (w28 fix 3 — the "no plugins" half still holds), and bundle.icon now has two entries, not `[]`. The admission-close doc was never refreshed. Why it matters: this file's own admission record misdescribes its security surface. Confidence: HIGH — Verification: read (cross-ref audit/files/apps/desktop-tauri/capabilities/default.json.audit.md F-001).

F-005 [S4]
Where: apps/desktop-tauri/tauri.conf.json:23 vs product.toml:231
Code (conf):
```
connect-src 'self' ipc: http://ipc.localhost
```
vs product.toml:231 `update_feed = "https://updates.lumen-player.example.org/feed.json"`. What is wrong: if the desktop webview ever fetches the configured update feed, the CSP blocks it (no https: origin in connect-src). Latent, not live: `[telemetry] update_checks = false` (product.toml:129) and nothing in the UI fetches it today. What correct looks like: when update checks ship, connect-src must gain the feed origin. Confidence: HIGH — Verification: read.

## 4. Per-key inventory (config file — no functions; 34 LOC < 100)
| Key | Verdict | Notes |
|---|---|---|
| $schema | REAL | pointer to schema.tauri.app/config/2 |
| productName "Lumen" | REAL | rebrand token (rebrand.rs:15); = product.toml identity.name |
| version "0.1.3" | REAL | Windows exe resource via tauri-build; rebrand-orphaned (F-003) |
| identifier "dev.lumen.desktop" | REAL | = product.toml reverse_dns "dev.lumen" + ".desktop" (product.toml:37; rebrand.rs:15 contract; runtime.rs audit confirms shape) |
| build.frontendDist "../../ui/dist" | REAL | vite default outDir; CI-asserted (§2) |
| build.devUrl (ABSENT) | — | consistent: tauri-cli/`tauri dev` never used; dev loop = npm build + cargo run (INFRA-BLOCKED §C) |
| app.withGlobalTauri false | REAL | consistent with UI's direct __TAURI_INTERNALS__ binding (§2) |
| windows[0].label "main" | REAL | matches capabilities + app.rs + frame_channel.rs |
| windows[0].title "Lumen" | REAL-UNPROVEN | hard literal; product.toml title_template/idle_key never applied at runtime (app.rs:74-76 admits binding not landed) — W11-A06 F-001 cross-ref |
| width 1280 / height 800 | SHALLOW | width agrees with product.toml; height 800 is first-paint-only drift (F-001) |
| minWidth 940 / minHeight 600 | REAL | agree with product.toml min_width/min_height (w28 fix 5) |
| decorations false | REAL | frameless; 4 window grants in capabilities; UI custom titlebar |
| transparent (ABSENT) | — | window is opaque; docs' "transparent webview" is rejected Candidate A only (docs/08-ui-figma-fidelity.md:398) |
| security.csp | REAL | hardened; matches INFRA-BLOCKED §A byte-for-byte (§2); F-005 latent gap |
| bundle.active/targets/icon.png | UNREACHABLE | no consumer in shipped pipeline (F-002) |
| bundle.icon icons/icon.ico | REAL | Windows exe icon via tauri-build |

## 5. Test honesty
No tests target this file directly. Indirect: tests/single_command.rs (anti-bypass) compiles against the same generate_context!; ci.yml:731 runs it. The height-800 value is asserted by NOBODY — window_config.rs tests assert product.toml's 720 (window_config.rs:360 `assert_eq!(config.height, 720);`), so the conf drift is invisible to every test in the tree.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Real CSP (tauri.conf.json): default-src 'self' … csp: null is gone" | INFRA-BLOCKED.md:34-38 | REAL | conf:23 byte-compare (sed) |
| "Capabilities: unchanged core:default only" | INFRA-BLOCKED.md:39-40 | FALSE (stale) | capabilities/default.json:6-12 has 4 more grants (F-004) |
| "Icons (bundle.icon: [])" | INFRA-BLOCKED.md:78-79 | FALSE (stale) | conf:29-32 two icons (F-004) |
| "tauri-cli is NOT used" | INFRA-BLOCKED.md:104-106 | REAL | repo grep zero matches |
| "the UI floor (tauri.conf.json first-paint minWidth/minHeight …) — 940x600" | product.toml:66-71 | REAL | conf:17-18 = 940/600 |
| window default 1280x720 | product.toml:63-65 | SHALLOW on desktop | first paint is 800 (F-001); 720 only post-setup |

## 7. Dead / duplicated / unreachable code
- Dead in shipped pipeline: `bundle.active`, `bundle.targets: "all"`, `bundle.icon` png entry (F-002).
- Duplicated identity surface: two icon systems — conf → apps/desktop-tauri/icons/ (exists) vs product.toml [brand] → assets/brand/{icon.ico,icon.icns,icon-512.png,logo.svg} (ALL absent: `ls assets/brand` exit 2; baseline xtask-config-validator.txt:15 `result: FAILURE (4 violation(s))`).
- `remember_geometry` parsed + carried in WindowBuildSpec but dropped by its only consumer (see F-001).

## 8. Dependencies & flags
- Consumed by: tauri-build 2.7.4 (build.rs), tauri 2.12.5 generate_context! (app.rs:321) — both verified in ~/.cargo/registry sources; the crate is workspace-EXCLUDED with its own Cargo.lock (Cargo.toml header).
- No feature flags gate this file; release lane builds it unconditionally on webview hosts.

## 9. Open questions for the next agent (Wave 17 — packagers)
1. What do scripts/packaging/* actually use for icons per OS — apps/desktop-tauri/icons/ (exists) or product.toml [brand] assets/brand/ (missing 4/4)? If the latter, every installer ships iconless.
2. Does build-msi.ps1/WiX read the conf `version`/`identifier` or product.toml's display_version/version_code/msi_upgrade_code (product.toml:43)? Two possible version/id sources.
3. Confirm on a GTK-capable host the visible 800→720 resize flash (F-001) — UNVERIFIED here (gdk-sys).
4. Does the CSP get injected into the served UI as expected (tauri runtime behavior) — needs a launched app; UNVERIFIED.
