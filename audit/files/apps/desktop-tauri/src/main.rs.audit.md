# AUDIT: apps/desktop-tauri/src/main.rs
| Field | Value |
|---|---|
| Auditor | W14-A03 |
| Date | 2026-11-24 |
| Lines / functions | 201 / 10 (incl. `mod win32` with 4 fns + extern block) |
| File verdict | REAL-UNPROVEN (real, correct failure-face code; unexecutable — its own crate does not compile, F-001) |
| Reachable from shipped entry point? | main.rs IS the shipped entry (bin `lumen-tauri`, shipped in every GUI payload: docs/packaging/README.md:55-58) — but NO binary can be produced from this tree: E0425 in app.rs:301 (F-001). Call chain that cannot link: `main` (main.rs:45) → `lumen_desktop_tauri::app::run` (app.rs:288) → **compile break**. |
| Findings | S0: 1 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | sed quote-verify ×15 (all exit 0); `rg spawn_session_pump apps/ crates/` → exactly 1 hit (the call site); `rg "macro_rules|use .*\*;" apps/desktop-tauri` → no matches; `./target/debug/deps/desktop-c2864a891fe6f82b diagnostics` → 6 passed / 0 failed, exit 0; `ls ui/dist` → absent; `rg "crash log" .github/workflows/` → no matches; registry reads of tauri-2.12.5 (Cargo.lock:3688-3690) permission/ACL sources. |

## 1. One-paragraph verdict
This file is the honest, well-engineered failure face of a binary that cannot be built. main.rs itself contains no fakery: it installs the process-global panic hook first (main.rs:49), calls the one real entry (main.rs:50), and on failure writes a crash log, prints two stderr receipts, and shows a Windows-native MessageBox through hand-rolled std-only FFI with a correct "no other face alive" gate. The crash-handler backing is REAL and executed (6/6 diagnostics unit tests pass, run by me). But the single statement this file exists to guard — `lumen_desktop_tauri::app::run()` — lives in a crate that does not compile: app.rs:301 calls `spawn_session_pump`, a function that has NO definition anywhere in the repository (the defined name is `spawn_event_pump`, app.rs:190). E0425, every host, GTK or not. Every face in this file is therefore unreachable in any build produced from the current tree, and INFRA-BLOCKED.md's claim that "the crate compiles in its dedicated CI lane" is FALSE against this tree (ci.yml's tauri-check runs `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets`, which cannot pass). Secondary defect: the "THREE faces on every failure" doc claim is false off-Windows, where face 3 is a cfg'd no-op.

## 2. What actually works (verified)
— verified by `./target/debug/deps/desktop-c2864a891fe6f82b diagnostics` → `test result: ok. 6 passed; 0 failed` (exit 0): the panic-hook + crash-log machinery main.rs:49/:53 calls (`desktop::diagnostics::install_panic_hook`, `write_crash_log`) is real, tested code — including `panic_hook_writes_the_log_and_still_runs_the_previous_hook`.
— verified by `rg` (exactly one tree-wide hit, the call site) + no glob imports/macros in the crate: `spawn_session_pump` is undefined → the compile break is proven statically, not assumed.
— verified by sed against `~/.cargo/registry/.../tauri-2.12.5`: the UI's full invoke surface is registered/permitted (see §3 F-001 context and the IPC answer below).
— NOT verified at runtime: main() itself, the Windows dialog, the stderr receipts through a real launch — cargo-build-budget + no GTK (baseline cargo-build-desktop-tauri.txt: gdk-sys failure) + the F-001 compile break.

## 3. Findings
F-001 [S0]
Where: apps/desktop-tauri/src/main.rs:50 (victim) + apps/desktop-tauri/src/app.rs:298-302 (cause)
Code: (main.rs:50, verbatim)
`    if let Err(error) = lumen_desktop_tauri::app::run() {`
(app.rs:298-302, verbatim)
```
            // bridge. Passing `app` itself died on the first real compile
            // (CI tauri-check, run 43384519598, two E0308s).
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
```
What is wrong: `spawn_session_pump` has no definition anywhere in the tree — `rg spawn_session_pump apps/ crates/` returns exactly one hit, the call site; the real function is `fn spawn_event_pump(app: AppHandle)` (app.rs:190); app.rs has no glob imports and no macros (`rg "macro_rules|use .*\*;" apps/desktop-tauri` → no matches). E0425: the lib target fails, so the bin target containing THIS file fails, on every host. Every line of main.rs — panic hook, crash log, stderr receipts, MessageBox — is unreachable because no `lumen-tauri` binary can be produced from this tree. Why it matters: this binary is the shipped GUI entry point of the product (docs/packaging/README.md:55-58: AppImage AppRun, macOS CFBundleExecutable, MSI shortcuts all exec `lumen-tauri`); the "GUI plays video" story rests entirely on a crate that cannot compile. INFRA-BLOCKED.md:4 asserts "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" — FALSE against the current tree: ci.yml:731 runs `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets` (compile = mandatory). First found by W13-A10 (shell.rs audit); confirmed here from the entry-point side. What correct looks like: rename the call to `spawn_event_pump` and run the tauri-check lane green. Confidence: HIGH — Verification: read + rg (static proof; cargo build impossible here: no GTK + cargo-build-budget).

F-002 [S3]
Where: apps/desktop-tauri/src/main.rs:25-27 vs :109-110
Code: (main.rs:25-27, verbatim)
```
// ("brief loading then nothing"). main() now leaves THREE faces on
// every failure, in this order:
//   1. the crash log (desktop::diagnostics, std-only): product
```
(main.rs:109-110, verbatim)
```
#[cfg(not(windows))]
fn show_message_box(_text: &str, _caption: &str) {}
```
What is wrong: "THREE faces on every failure" is false on Linux and macOS — the two platforms that ship the AppImage/dmg GUI payloads — where face 3 is compiled out to a no-op and a double-click launch has only the crash log (face 2's stderr is a null/silent sink in GUI launches by the file's own analysis, lines 16-18). The same comment block admits "Windows ONLY" at lines 36-37, contradicting its own headline. Why it matters: support triage reading the headline expects a visible dialog on all platforms; on non-Windows the honest description is "one artifact file, nothing visible". What correct looks like: "three faces on Windows, two (log + stderr) elsewhere". Confidence: HIGH — Verification: read.

F-003 [S4]
Where: apps/desktop-tauri/src/main.rs:74-89
Code: (main.rs:78, verbatim)
`    text.push_str("Lumen could not start.\n\n");`
What is wrong: `fatal_dialog` composes the full dialog text — chain, crash-log path, WebView2 hint — on every platform, then hands it to a sink that is a cfg'd no-op off-Windows (F-002). Dead string-building work and dead copy on Linux/macOS; trivial cost (once per process death) but per-platform dead code. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/desktop-tauri/src/main.rs:59-61 vs .github/workflows/
Code: (main.rs:59-61, verbatim)
```
        eprintln!("lumen-tauri: admission shell failed: {error:#}");
        // lumen-allow-literal: argv0/diagnostic notice — the crash-log path receipt; same argv0 prefix family, so CI receipts and support triage share one vocabulary
        eprintln!("lumen-tauri: crash log: {}", log.display());
```
What is wrong: only receipt line 1 is CI-asserted (`rg "admission shell failed"` → tauri-launch.yml:228,230,392,394,530,536 + scripts/packaging/test-msi-upgrade.ps1); `rg "crash log" .github/workflows/` returns nothing. The comment's own claims stay true (it promises exact-compatibility for line 1, self-sufficiency for line 2 — not a CI grep), but the crash-log path receipt that makes captured stderr "self-sufficient" is asserted by zero lanes: if face 1 silently failed to write, no CI would notice. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (mandatory — file is 201 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 45-67 | REAL-UNPROVEN | S0 via F-001 | hook→run→[crash log→stderr→dialog]→exit(1); order is correct; unexecutable (crate E0425) |
| `fatal_dialog` | 74-89 | REAL-UNPROVEN | S3/S4 (F-002/F-003) | builds text on all platforms; Windows-only sink |
| `is_webview_class` | 95-97 | REAL | — | honest heuristic; false-positive cost documented at :91-94 |
| `show_message_box` (win) | 101-104 | REAL-UNPROVEN | — | delegates to gated win32 sink |
| `show_message_box` (non-win) | 109-110 | STUB (deliberate, documented) | S3 (F-002) | no-op; documented scope cut |
| `win32::utf16` | 147-149 | REAL-UNPROVEN | — | NUL-terminated UTF-16, correct for MessageBoxW LPCWSTR |
| `win32::show_message_box_when_no_other_face` | 162-180 | REAL-UNPROVEN | — | double gate; SAFETY comment; null owner HWND |
| `win32::console_attached` | 183-187 | REAL-UNPROVEN | — | GetConsoleWindow null probe |
| `win32::stderr_is_live` | 191-200 | REAL-UNPROVEN | — | null + `as isize == -1` (INVALID_HANDLE_VALUE) + GetFileType != FILE_TYPE_UNKNOWN — correct SDK semantics |
| `win32` extern block | 125-136 | REAL-UNPROVEN | — | signatures match SDK (LPCWSTR not LPSTR; handle-sized pointers); edition-2024 `unsafe extern "system"` |

## 5. Test honesty
No tests in this file. The crate's only test file (tests/single_command.rs) audits the registration site by source-scanning app.rs with comments stripped and asserts exactly `["lumen_dispatch"]` plus a single `generate_handler!` site — it would fail on a second registered command (honest), but it cannot run against this tree: `cargo test` requires compiling the lib, which fails at F-001. INFRA-BLOCKED.md:33's "(Local simulation PASS; CI compiles it for real.)" is therefore unproducible CI-side on the current tree.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This crate is workspace-EXCLUDED (root Cargo.toml `exclude`)" | main.rs:1-4 | REAL | root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]` (sed-verified) |
| "main() now leaves THREE faces on every failure" | main.rs:25-26 | FALSE (non-Windows) | F-002; :109-110 no-op |
| "tauri-launch.yml and release smokes grep 'admission shell failed'" | main.rs:54-56 | REAL | tauri-launch.yml:228,230,392,394,530,536; scripts/packaging/test-msi-upgrade.ps1 |
| "the subsystem bit only stops WINDOWS from auto-allocating a console, it never detaches explicit handles" | main.rs:13-15 | REAL-UNPROVEN | standard Windows semantics; not executable in sandbox |
| "Rust std maps the null stderr handle to a silent sink — the eprintln! below is a no-op there" | main.rs:16-18 | REAL-UNPROVEN | std behavior; not executable in sandbox |
| "the crate compiles in its dedicated CI lane (`tauri-check`)" | INFRA-BLOCKED.md:4 | FALSE | F-001; ci.yml tauri-check compiles the crate |
| "the 40 ms event pump" | INFRA-BLOCKED.md:27 | FALSE | `const SESSION_TICK: Duration = Duration::from_millis(10)` (app.rs:109) |

## 7. Dead / duplicated / unreachable code
- ALL of main.rs is unreachable in current-tree builds (F-001) — the file as shipped entry, the crate as build product.
- In-crate: `spawn_event_pump` (app.rs:190-232) is defined but never called (the call site names the wrong identifier) — the session-pacer/frame-shipper thread never spawns even in intent until the typo is fixed (app.rs domain, W14-A04).
- No duplicated blocks in main.rs; the two `show_message_box` cfg twins are the deliberate seam pattern.

## 8. Dependencies & flags
- Cargo.toml (apps/desktop-tauri): `desktop` (path), `commands` (path), `tauri = "2"` (locked 2.12.5, Cargo.lock:3688-3690), `serde`; build-dep `tauri-build = "2"`. **No `[features]` table exists** — zero feature gates; nothing is feature-gated away from any lane.
- Env/feature gates in main.rs: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` (:19, release-only GUI subsystem); `#[cfg(windows)]` (:101,:117) / `#[cfg(not(windows))]` (:109) face-3 sink split. No env-var reads anywhere in main.rs; the crate's only env access is `std::env::args()` (app.rs:276, argv file-open).
- `unsafe_code = "deny"` crate-wide (Cargo.toml:35) with exactly one localized `#[allow(unsafe_code)]` on `mod win32` (:118), each unsafe block carrying a SAFETY comment — invariant discipline upheld (§4 table).
- tauri.conf.json: `frontendDist: "../../ui/dist"` — ui/dist does not exist in the repo (`ls ui/dist` → No such file); every lane must build the UI first (ci.yml does; local builds cannot compile without it).

## 9. IPC surface (the task question — registered vs UI-called)
Registered on the Rust side: EXACTLY ONE custom command — app.rs:293 `.invoke_handler(tauri::generate_handler![lumen_dispatch])` (sed-verified), pinned by tests/single_command.rs. NO plugins registered — the builder chain is `default().manage().invoke_handler().setup().on_window_event().run()` (app.rs:289-321); zero `.plugin(` calls. UI production invoke surface (rg over ui/src, tests excluded): `lumen_dispatch` (tauri-transport.ts:143, const :28, payload key `{ request }` matching the `request: WireRequest` parameter); `plugin:event|listen|unlisten` (tauri-transport.ts:91,106; window-controls.ts:79,90); `plugin:window|minimize|toggle_maximize|close|is_maximized` (window-controls.ts:70-75); `plugin:__TAURI_CHANNEL__|fetch` (frame-channel.ts:212,:433); `plugin:window|start_dragging` via injected drag.js on `data-tauri-drag-region` (AppShell.tsx:541). Permission audit against the ACTUAL locked tauri 2.12.5 sources in the registry: capabilities/default.json grants `core:default` + `core:window:allow-minimize|toggle-maximize|close|start-dragging`; `core:event:default` = allow-listen/unlisten/emit/emit-to (permissions/event/autogenerated/reference.md:7-10); `core:window:default` includes `allow-is-maximized` (permissions/window/autogenerated/reference.md:15); and `plugin:__TAURI_CHANNEL__|fetch` is explicitly ACL-exempt (`&& request.cmd != crate::ipc::channel::FETCH_CHANNEL_DATA_COMMAND`, tauri-2.12.5 src/webview/mod.rs:2081-2084, "TODO: Remove this special check in v3"). **Answer: PERFECT MATCH — one registered custom command, one UI-called custom command, every other UI invoke is a permitted Tauri core command; nothing unregistered is called, nothing registered is uncalled.** All of it is moot until F-001 is fixed; the event-frame wire shape stays UNVERIFIED (tauri-transport.ts:93-97 carries its own [VERIFY] marker; no webview in sandbox).

## 10. Open questions for the next agent
- W14-A04 (app.rs): owns F-001's fix-side analysis; check whether tauri-check CI history shows when the lane went red (no CI logs in sandbox — UNVERIFIED, reason: no-network/no-ci-logs) and whether v0.1.3 release payloads were built from a pre-typo tree (provenance of shipped binaries unverifiable here).
- Whether the MSI zip-lane 10s-survival smoke (tauri-launch.yml) ever exercised the three-face path on a real WebView2-missing host — the dialog gate (stderr_is_live) is designed to skip under redirected stderr, which is exactly the smoke's configuration; the dialog itself is CI-unobservable by design.
