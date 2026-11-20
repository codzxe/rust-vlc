# apps/desktop-tauri/src — combined index audit (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 (GUI reality trace, hostile evidence-only pass) |
| Date | 2026-11-20 |
| Scope | `apps/desktop-tauri/src/` — full listing + verdicts; `main.rs`, `lib.rs`, `frame_channel.rs` audited here in depth; `app.rs` (the central file) carries its own audit file (`app.rs.audit.md`). Assigned-module map: "lumen_dispatch" = `app.rs::lumen_dispatch` + `apps/desktop/src/dispatch.rs` (own audit file); "shell runtime / pacer" = `apps/desktop/src/shell.rs` (own audit file); "crash handler" = `apps/desktop/src/diagnostics.rs` + `src/main.rs` §faces (below + own audit file). |
| Reachability | lumen-tauri binary → `main.rs::main` → `lumen_desktop_tauri::app::run()` → tauri Builder (one command `lumen_dispatch`, event pump ticker, drag-drop, argv open) → webview ui/dist. Compiled only in the dedicated webkit2gtk/WebView2/WKWebView CI lanes (workspace-EXCLUDED crate; `src/main.rs:1-4`). **Runtime GUI behavior UNVERIFIED in this sandbox — no display, no webview (INFRA-BLOCKED.md).** |
| Verdict | **Structurally real, headless-proven shell; runtime pixel/audio behavior UNVERIFIED.** The wrapper is a genuine tauri 2.12.5 app (own `Cargo.lock`: `name = "tauri" / version = "2.12.5"`, `wry 0.57.2`), registers exactly one IPC command, drives the engine through the same `CliHost::pump` the proven CLI baseline uses, and ships LUF1 frames as raw binary channel messages. No theater found in this directory; the gaps live elsewhere (audio-output null-only; runtime launch verdicts still manual). |

## Directory listing (complete)

```
src/app.rs           320 LOC  THE tauri wrapper (own audit file)
src/frame_channel.rs  57 LOC  audited below
src/lib.rs             8 LOC  audited below
src/main.rs          201 LOC  audited below
```

Crate-level facts (sed-verified): `Cargo.toml:30-32` `[[bin]] name = "lumen-tauri" path = "src/main.rs"`; deps only `desktop` (path), `commands` (path), `tauri = "2"`, `serde` — `Cargo.toml:49-62`. Lints deny `unsafe_code`/`unwrap_used`/`expect_used`/`panic` crate-wide (`Cargo.toml:34-47`) with the single documented `#[allow(unsafe_code)]` exception in `main.rs`'s `mod win32` (`main.rs:117-118`).

---

## src/main.rs — the admission binary (201 LOC)

**Verdict: real.** Three-failure-face startup + Windows-native gated dialog. Not a print-only stub.

Functions:

| Function | LOC | What it does |
|---|---|---|
| `main` | 45-67 | installs the panic hook first (`desktop::diagnostics::install_panic_hook()`, line 49), runs `app::run()`, on failure writes the crash log (`write_crash_log("startup", &error)`, line 53), prints the two-line stderr receipt (lines 59-61), shows the gated dialog, exits 1 |
| `fatal_dialog` | 74-89 | builds the dialog text; appends the WebView2 bootstrapper URL when the chain names a webview (`is_webview_class`, lines 95-97) |
| `show_message_box` | 101-110 | Windows: `win32::show_message_box_when_no_other_face`; non-Windows: deliberate no-op (`#[cfg(not(windows))] fn show_message_box(_text, _caption) {}`, line 110) |
| `mod win32` | 117-201 | hand-rolled FFI: `MessageBoxW`/`GetConsoleWindow`/`GetStdHandle`/`GetFileType` (`unsafe extern "system"`, lines 125-136); the gate at lines 162-165: `if console_attached() || stderr_is_live() { return; }` |

Evidence quotes (sed-verified):
- `main.rs:19` `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
- `main.rs:59-61`:
  ```
  eprintln!("lumen-tauri: admission shell failed: {error:#}");
  eprintln!("lumen-tauri: crash log: {}", log.display());
  ```
- `main.rs:172-179` — the `unsafe { MessageBoxW(null_mut(), wide_text.as_ptr(), wide_caption.as_ptr(), MB_ICONERROR | MB_OK | MB_SETFOREGROUND) }` call with a per-block SAFETY comment (lines 168-171).

Findings:
- **S3 — crash coverage is panic/startup only; OS-signal kills leave nothing.** The hook is `std::panic::set_hook` (`apps/desktop/src/diagnostics.rs:167-168`). A `SIGSEGV`/`SIGABRT` inside wry/webkit native code (a tauri app's realistic hard-death mode) is not a Rust panic; no signal handler exists anywhere in this crate (`rg -n -i "signal|SIGSEGV|SIGABRT" apps/desktop-tauri/src` → no code hits). The crash artifact is genuine for what it covers.
- **S4 — `is_webview_class` heuristic is ASCII-lowercase `contains("webview")`** (`main.rs:95-97`) — self-acknowledged in its own doc comment; a false positive only adds an unneeded hint. Recorded, not actionable now.

---

## src/lib.rs — the library face (8 LOC)

**Verdict: real, trivial.** `pub mod app; pub mod frame_channel;` (lines 7-8) — the crate's whole library surface. The doc comment names the §7.4 anti-bypass test (`tests/single_command.rs`). No dead exports.

---

## src/frame_channel.rs — the IPC frame shipper (57 LOC)

**Verdict: real and minimal; the binary lane is correctly chosen.**

| Function | LOC | What it does |
|---|---|---|
| `frame_channel` | 39-48 | rehydrates the UI-sent bare `u32` id as `__CHANNEL__:<id>` (`line 40`) → `JavaScriptChannelId` → `id.channel_on(webview)` bound to the `main` webview (lines 44-47) |
| `send_record` | 55-57 | `channel.send(InvokeResponseBody::Raw(record.to_vec()))` — one LUF1 record per message, raw bytes |

Evidence (sed-verified): `frame_channel.rs:56` `channel.send(InvokeResponseBody::Raw(record.to_vec()))`. The module doc states the cost discipline (lines 17-22): "a 1280×720 RGBA frame is ~3.7 MB — the JSON-array form a plain `Channel<Vec<u8>>` would produce is 4-6× that and evaluates as a script; the Raw form is the binary lane".

Findings:
- **S4 — no backpressure/cost guard on the ship.** `send_record` does one full `to_vec()` copy per record (3.7 MB at 720p) on the ticker thread, one frame per ~30.6 ms tick. Consistent with the pacer contract; the copy is not counted anywhere. Perf posture only — no correctness claim broken.
- **S4 — error handling is `eprintln!` + drop-until-next-bind** (quoted in `app.rs.audit.md` finding). Unverifiable at runtime here.

---

## tests/single_command.rs (referenced; sed-verified)

- Line 39: `fn the_shell_registers_exactly_one_command_and_it_is_lumen_dispatch()` reads `include_str!("../src/app.rs")` with comment lines stripped (lines 22-32) and asserts the `generate_handler!` array is exactly `vec!["lumen_dispatch"]` (lines 59-64).
- Lines 71-78: the registration site must be the crate's only `generate_handler!` site.
- Verdict: real source-scanning anti-bypass test (doc 05 §7.4). Weakness (S4): it scans `src/app.rs` only — a second command registered from another file would need the unique-site check to be extended crate-wide (`registration_source()` includes only `../src/app.rs`, line 23). The 4-file crate makes this a thin surface today.

## Doc claims vs code (this directory)

- **INFRA-BLOCKED.md §A drift (S4):** "the 40 ms event pump" (INFRA-BLOCKED.md, §A bullet "The wrapper") vs the shipped constant `PUMP_INTERVAL = PACER_TICK` = `Duration::from_micros(30_625)` (sed-verified at `apps/desktop/src/shell.rs:725`). 30.6 ms, not 40 ms. The doc is describing the pre-w31 pacer.
- INFRA-BLOCKED.md §B.2 admits: "the `tauri-launch.yml` deep verdicts (window/file/playback/settings per OS, the event-frame round-trip) remain manual." — i.e., the repo's own record concedes no committed runtime playback proof exists for the GUI.

## Test honesty (this directory)

- No test in this crate executes a webview (impossible here; `INFRA-BLOCKED.md` §A "Local simulation PASS; CI compiles it for real"). The frame ship half is proven only headlessly through `apps/desktop/src/shell.rs` tests (see that audit file) — which drive `take_frame_records`, NOT `frame_channel.rs::send_record`. **`send_record`/`frame_channel` have zero committed test coverage** (unit or integration) — the tauri-API half is exercised only by the CI compile lane. S3.

## Dead code / deps / flags

- No dead code in this directory. Deps: `tauri = "2"` resolves to 2.12.5 in this crate's `Cargo.lock` (sed-verified line 3688-3689), matching the version the UI shim claims to mirror (`ui/src/client/frame-channel.ts:29`).
- `withGlobalTauri: false` (`tauri.conf.json:10`) — the UI must inject nothing; the UI's hand-rolled internals binding is the only route. Consistent.

## Open questions

1. Does `core:default` permit the `plugin:__TAURI_CHANNEL__|fetch` invoke the UI shim uses defensively (frame-channel.ts:212)? On the shipped tauri 2.12.5 the shell's own injected script performs the fetch (frame-channel.ts:30-33), so the permission question is dormant — but it is a live drift risk if the shell build moves. UNVERIFIED (no runtime).
2. First-frame loss window: the UI mounts the canvas (and thus sends `session.set-frame-channel`) only after a session is non-idle — see `PlayerScreen.tsx.audit.md` finding on mount gating; frames presented before the bind are never tapped.
