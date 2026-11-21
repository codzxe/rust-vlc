# AUDIT: apps/desktop-tauri/src/main.rs
| Field | Value |
|---|---|
| Auditor | W14-A02 |
| Date | 2026-11-21 |
| Lines / functions | 201 / 11 (+4 extern decls, 5 consts) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO, as the tree stands — main.rs IS the `[[bin]] lumen-tauri` entry, but the crate cannot compile (cross-file E0425: app.rs:301 calls undefined `spawn_session_pump`; the definition is `spawn_event_pump` at app.rs:190). No `lumen-tauri` binary can be produced from this tree on ANY host, sandbox or CI. Baseline run #2 (build-desktop-tauri.log.txt) died at gdk-3.0 pkg-config in a dependency build script BEFORE this crate's sources compiled, so it neither proves nor disproves main.rs's compilability. |
| Findings | S0: 1 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "spawn_session_pump\|spawn_event_pump" apps/desktop-tauri` → call app.rs:301 vs def app.rs:190, rename drift confirmed; repo-wide rg → exactly one hit for `spawn_session_pump` (the call site). `sed -n` on every quoted range below (verbatim verification). `rg -n "admission shell failed" .github/workflows` → tauri-launch.yml:228,392,530; release.yml:359 (CI grep contract exists and matches main.rs:59). `rg -n "tauri" .github/workflows/ci.yml` → tauri-check lane is Linux+webkit2gtk (ci.yml:680-775). Read callees: apps/desktop/src/diagnostics.rs (both called fns exist, pub, std-only), apps/desktop/src/lib.rs (`pub mod diagnostics`), apps/desktop-tauri/src/{lib.rs,app.rs,frame_channel.rs}, Cargo.toml, tauri.conf.json, capabilities/default.json, tests/single_command.rs, product.toml `[window]` (present, parse inputs exist). |

## 1. One-paragraph verdict

main.rs is the boot shim of the `lumen-tauri` GUI binary: install a
process-global panic hook, call `lumen_desktop_tauri::app::run()`, and on
failure leave up to three "faces" (crash log, stderr receipt, Windows-only
native MessageBox), then exit 1. The shim's own logic is sound — both
`desktop::diagnostics` callees exist with matching signatures, the Win32 FFI
declarations match the SDK, the CI-grepped receipt string is byte-accurate —
but every claim about it is unexecutable: the crate does not compile
(app.rs:301 references a function that does not exist), so the binary this
file is the entry of cannot be built anywhere, by the sandbox OR by the
claimed `tauri-check` CI lane (INFRA-BLOCKED.md's "the crate compiles in its
dedicated CI lane" is FALSE for this tree). Additionally, the module's
headline guarantee — a visible failure face for the double-click user — is
delivered only on Windows; on the Linux AppImage and macOS dmg that
release.yml ships, a startup failure is still invisible. REAL-UNPROVEN, with
the S0 caveat that "unproven" is generous: the file's whole purpose is
currently blocked by a one-token rename drift in a sibling file.

## 2. What actually works (verified)

— Verified by `rg`/`sed` tracing (no build possible — standing UNVERIFIED-BUILD,
   baseline run #2, build-desktop-tauri.log.txt, exit 101 at gdk-3.0 pkg-config):
  - `desktop::diagnostics::install_panic_hook()` (main.rs:49) and
    `desktop::diagnostics::write_crash_log("startup", &error)` (main.rs:53)
    both exist as `pub fn`s in apps/desktop/src/diagnostics.rs:166 and :95 with
    exactly the called signatures; `pub mod diagnostics` is exported
    (apps/desktop/src/lib.rs:79); the `desktop` path dep is declared
    (apps/desktop-tauri/Cargo.toml:53).
  - The CI grep contract is real and matched: workflows grep
    "admission shell failed" (tauri-launch.yml:228,392,530; release.yml:359);
    main.rs:59 emits exactly `lumen-tauri: admission shell failed: {error:#}`.
  - FFI signatures/consts checked against the Win32 SDK:
    `MessageBoxW(*mut c_void, *const u16, *const u16, u32) -> i32`,
    `GetConsoleWindow() -> *mut c_void`, `GetStdHandle(u32) -> *mut c_void`,
    `GetFileType(*mut c_void) -> u32` (main.rs:125-136); `STD_ERROR_HANDLE =
    0xFFFF_FFF4` ((DWORD)-12 ✓), `MB_ICONERROR = 0x10` ✓, `MB_OK = 0` ✓,
    `MB_SETFOREGROUND = 0x0001_0000` ✓ (main.rs:139-143). `unsafe extern
    "system"` is the correct edition-2024 form (Cargo.toml:26 edition = "2024").
  - `utf16()` NUL-terminates correctly (`encode_utf16().chain(once(0))`,
    main.rs:147-149) and both buffers outlive the `MessageBoxW` call
    (main.rs:166-179).
  - `#[allow(unsafe_code)]` on `mod win32` legally overrides the crate-level
    `[lints.rust] unsafe_code = "deny"` (deny is overridable by an inner
    allow; only `forbid` is not).
— Nothing was executed. Nothing in this crate can be: see F-001.

## 3. Findings

### F-001 [S0]
Where: apps/desktop-tauri/src/app.rs:301 (call) vs app.rs:190 (definition);
blocks main.rs:50. Cross-file: primary ownership W14-A01 — recorded here
because it voids main.rs's sole nontrivial call and the crate's buildability.
Code:
```rust
// app.rs:300-302 — inside the .setup closure of run():
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
```
```rust
// app.rs:190 — the ONLY ticker definition (repo-wide rg: `spawn_session_pump`
// has exactly one hit — this call site; no definition exists anywhere):
fn spawn_event_pump(app: AppHandle) {
```
What is wrong: `spawn_session_pump` does not exist. The function is
`spawn_event_pump`. This is E0425 ("cannot find function") on every platform,
unconditionally — no feature flag, no cfg, no glob import in scope
(app.rs:84-94 imports contain no glob). The lib target (`lumen_desktop_tauri`)
does not compile, therefore the bin target `lumen-tauri` (main.rs) does not
compile, therefore `cargo test --manifest-path apps/desktop-tauri/Cargo.toml
--locked --all-targets` (the tauri-check lane, ci.yml:731) cannot pass on this
tree.
Why it matters: (1) main.rs is the shipped GUI entry point; the entire
desktop-tauri story — launch smokes, release payloads shipping
`lumen-tauri.exe`/AppImage/dmg, INFRA-BLOCKED.md's admission claims — is
vacuous while the crate cannot produce a binary. (2)
INFRA-BLOCKED.md:5-6 states "the crate compiles in its dedicated CI lane
(`tauri-check`, Linux + webkit2gtk)" — that claim is FALSE for this tree;
either the lane never ran on it or it is red. (3) The baseline's
UNVERIFIED-BUILD (gdk pkg-config) masked this: the build died in a dependency
build script before ever type-checking this crate's sources.
What correct looks like: app.rs:301 calls `spawn_event_pump` (or the fn is
renamed to match, updating the module docs at app.rs:42 that already say
`spawn_event_pump`), and a green tauri-check run on this exact tree is cited
in INFRA-BLOCKED.md instead of a stale claim.
Confidence: HIGH — Verification: read (repo-wide `rg` for both identifiers;
`sed -n '299,303p'` and `sed -n '188,192p'` on the real file). Static proof;
no build needed to establish E0425.

### F-002 [S2]
Where: apps/desktop-tauri/src/main.rs:109-110 (and 21-26 for the claim).
Code:
```rust
#[cfg(not(windows))]
fn show_message_box(_text: &str, _caption: &str) {}
```
What is wrong: face 3 — the only face a double-click user can SEE — is a
no-op on Linux and macOS. The module header promises the fix for "an
eprintln! into a null sink plus exit(1) is exactly the v0.1.2 complaint shape
('brief loading then nothing')" (main.rs:23-26), but on the non-Windows
payloads release.yml actually ships (AppImage, dmg — whose CFBundleExecutable
launches `lumen-tauri`), a desktop-launcher start has no console and stderr
discarded: the user gets face 1 (a crash log in <tmp>/lumen/ they do not know
exists) + face 2 (a receipt into a dead sink) + silent exit(1). Exactly the
"brief loading then nothing" shape the lane claims to have killed.
Why it matters: the GUI-failure-face guarantee is delivered on 1 of 3 shipped
platforms; Linux is the dev/CI platform and ships an AppImage. A user hitting
the top startup-failure class (missing/broken webkit2gtk) sees nothing.
What correct looks like: either the header stops claiming the complaint shape
is fixed and states "Windows only", or the non-Windows lane gains a visible
sink (zenity/osascript are the std-free-adjacent options; a GTK dependency is
the documented reason it was skipped — then the doc must carry that cost as a
known gap, which it currently buries in a cfg'd no-op).
Confidence: HIGH — Verification: read (cfg pair main.rs:101-110; workflow
payload evidence release.yml:292-359 shipping lumen-tauri.exe, and the dmg
lane). Runtime behavior UNVERIFIED (no GUI sandbox), but the cfg elimination
is compile-time certain.

### F-003 [S3]
Where: apps/desktop-tauri/src/main.rs:191-200 (gate), 134-135 (mis-stated
semantics).
Code:
```rust
    fn stderr_is_live() -> bool {
        // SAFETY: reads the process's own handle slot; SDK constant.
        let handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
        if handle.is_null() || handle as isize == -1 {
            return false; // no inherited handle: the null sink
        }
        // SAFETY: a read-only probe of a handle we never invalidate.
        let kind = unsafe { GetFileType(handle) };
        kind != FILE_TYPE_UNKNOWN
    }
```
What is wrong: two semantic defects. (a) The inline doc (main.rs:134-135,
"0 (FILE_TYPE_UNKNOWN) when the handle is no real sink") inverts MSDN:
`GetFileType` returns FILE_TYPE_UNKNOWN both when the type is unknown AND on
failure — UNKNOWN is not a proof of "no real sink". (b) The gate equates
"handle has a type" with "a human can see what we write": a stderr handle
redirected to the NUL device (`2>NUL`, a char device) returns FILE_TYPE_CHAR →
`stderr_is_live() == true` → the dialog is suppressed while every byte written
to face 2 is discarded. The same holds for any write-only sink. The gate's
contract — "shows ONLY when BOTH provable sinks are dead" — is therefore
provable-false in the NUL case.
Why it matters: the Explorer-double-click user in exactly the suppressed case
gets no visible face; blast radius is limited to this best-effort dialog at
process death (crash log still written), hence S3 not S2.
What correct looks like: probe observability, not type — the redirecting-smoke
case the comment worries about is a PIPE, and `GetFileType` on a pipe reliably
returns FILE_TYPE_PIPE; the UNKNOWN arm is the wrong discriminator for "live".
Gate on `console_attached()` alone plus an explicit env/flag opt-out for CI,
or accept and document the NUL hole.
Confidence: MEDIUM — Verification: read + SDK semantics from knowledge
(no Windows host to run it; UNVERIFIED at runtime, code-logic reasoning is
complete).

### F-004 [S4]
Where: apps/desktop-tauri/src/main.rs:74-89 (composition) with 109-110 (sink).
Code:
```rust
fn fatal_dialog(error: &dyn Error, log: &Path) {
    let chain = format!("{error:#}");
    let mut text = String::new();
```
What is wrong: on non-Windows, `fatal_dialog` builds the entire dialog text —
error chain, crash-log path, the WebView2 hint and its URL push (main.rs:78-86)
— and hands it to a documented no-op; the names `fatal_dialog` and
`show_message_box` assert behavior that does not exist on 2 of 3 platforms.
Deliberate per the doc comment ("The text is built on every platform; only
the sink is Windows-conditional"), i.e. dead computation by design and a
misleading name by design.
Why it matters: hygiene only (one-time cost at process death); but the
"deliberate" label does not change that `is_webview_class` + string building
are effect-free code on Linux/macOS.
What correct looks like: `#[cfg(windows)] fn fatal_dialog(...)` building the
text at the sink, or a no-op `fatal_dialog` on non-Windows so the composition
does not pretend.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: apps/desktop-tauri/src/main.rs (whole file); CI evidence
ci.yml:680-775, release.yml:292-359, tauri-launch.yml.
What is wrong: zero tests exist for this file — `fatal_dialog`,
`is_webview_class`, and the whole `win32` module have no test coverage
anywhere in the repo (`tests/single_command.rs` only source-scans app.rs).
The `#[cfg(windows)]` half is never compiled by any test lane: tauri-check
runs on Linux (webkit2gtk), and the Windows release lanes only build +
launch-smoke. The gate logic F-003 found defective is exactly the kind of
thing a test would have pinned.
Code (the only test file in the crate, which never touches main.rs):
```rust
// tests/single_command.rs:23
    let source = include_str!("../src/app.rs");
```
Why it matters: diagnostic-only code, but it is the user's last hope on a
failed install; its gating has a defect (F-003) that test-free drift will
worsen.
What correct looks like: unit tests for `is_webview_class` (pure fn,
platform-free — trivially testable today) and for `utf16`/gate decisions
behind a seam; at minimum a cfg(windows) compile assert lane.
Confidence: HIGH — Verification: read (rg for `mod tests`/`#[test]` in the
crate → only single_command.rs).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 45-67 | REAL-UNPROVEN | S0 (via F-001) | Boot shim. Calls verified-existing diagnostics fns; the `lumen_desktop_tauri::app::run()` call is E0425-blocked by app.rs:301. No unwrap/expect/panic; `exit(1)` on the error path is the documented contract. |
| `fatal_dialog` | 74-89 | REAL-UNPROVEN | S4 (F-004) | Pure text composition; correct `{error:#}` chain; Windows-only effect (F-002/F-004). |
| `is_webview_class` | 95-97 | REAL-UNPROVEN | — | Trivial `contains("webview")`; false-positive cost documented (an extra hint line). Pure and testable on Linux today, but untested (F-005). |
| `show_message_box` (windows) | 101-104 | REAL-UNPROVEN | — | Thin delegate to `win32`. |
| `show_message_box` (not windows) | 109-110 | STUB | S2 (F-002) | Documented deliberate no-op; the stub is the whole non-Windows failure face. |
| `mod win32` extern block | 117-136 | REAL-UNPROVEN | — | 4 declarations; signatures + `unsafe extern "system"` (edition 2024) verified against SDK. Never compiled by any test lane. |
| `mod win32` consts | 138-143 | REAL-UNPROVEN | — | STD_ERROR_HANDLE/MB_* values all correct per SDK. |
| `win32::utf16` | 147-149 | REAL-UNPROVEN | — | Correct NUL-terminated UTF-16; no panics. |
| `win32::show_message_box_when_no_other_face` | 162-180 | REAL-UNPROVEN | S3 (via F-003) | SAFETY comments present and valid (buffers outlive call, null owner HWND). Gate has the F-003 defect. |
| `win32::console_attached` | 183-187 | REAL-UNPROVEN | — | `GetConsoleWindow() != null`; correct semantics. |
| `win32::stderr_is_live` | 191-200 | REAL-UNPROVEN | S3 (F-003) | INVALID_HANDLE_VALUE check (`as isize == -1`) correct; FILE_TYPE_UNKNOWN/NUL semantics wrong (F-003). |

## 5. Test honesty (if tests exist in/for this file)

No tests exist in or for this file. The crate's only test file,
`tests/single_command.rs`, source-scans `src/app.rs` and never references
main.rs. Mutation check (reasoned, not run): deleting the entire win32 module
or inverting `stderr_is_live` would change zero CI outcomes today — no lane
compiles (Linux cfg) or tests (any OS) this code.

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "main() now leaves THREE faces on every failure" | main.rs:25-26 | FALSE (non-Windows double-click: 0 visible faces) | main.rs:109-110 no-op sink; shipped payloads release.yml (AppImage/dmg exec lumen-tauri) |
| "a native MessageBox — Windows ONLY" | main.rs:36-37 | REAL | cfg pair main.rs:101-110 |
| "tauri-launch.yml and release smokes grep 'admission shell failed'" | main.rs:55-56 | REAL | tauri-launch.yml:228,392,530; release.yml:359 |
| "the crate compiles in its dedicated CI lane (tauri-check, Linux + webkit2gtk)" | INFRA-BLOCKED.md:5-6 | FALSE (this tree) | E0425 app.rs:301 (F-001); ci.yml:731 `--all-targets` compiles the bin |
| "the panic hook writes the same artifact, so exit-101 panics are no quieter than startup errors" | main.rs:29-31 | REAL (structure) | diagnostics.rs:166-179 wraps the hook, writes the same sink; unexecuted (UNVERIFIED-BUILD) |
| "Rust std maps the null stderr handle to a silent sink — the eprintln! below is a no-op there, not a panic" | main.rs:16-18 | REAL | std behavior (comment claim; consistent with std's null-handle mapping); not run |
| main.rs prints `lumen-tauri: admission shell failed: …` and exits | tauri-launch.yml:15 | REAL | main.rs:59,65 |

## 7. Dead / duplicated / unreachable code

- `fatal_dialog`'s text building + `is_webview_class` on non-Windows:
  computed, then discarded (F-004).
- The whole file is currently unbuildable-dead in effect (F-001): the bin
  target cannot be produced from this tree.
- No other duplication found; the module is 201 lines and tight.

## 8. Dependencies & flags

- `desktop` (path, apps/desktop): used by main.rs only for
  `desktop::diagnostics` — verified present/`pub`. std-only callee, matches
  the "std-only lane" claim.
- `commands`, `serde`, `tauri`: not used by main.rs itself (app.rs /
  frame_channel.rs — other agents' files).
- Crate lints (Cargo.toml:34-47): `unsafe_code = "deny"` etc. — the
  module-level `#[allow(unsafe_code)]` on `mod win32` (main.rs:118) is a
  legal, locally-scoped override, documented with per-block SAFETY notes.
  All unsafe blocks reviewed: 4, each with a stated invariant, each necessary
  (raw FFI).
- `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
  (main.rs:19): correctly placed inner attribute after `//!` docs; release-only
  GUI subsystem per the w28 rule.

## 9. Open questions for the next agent

- **W14-A01 (urgent):** confirm and own F-001 — app.rs:301
  `spawn_session_pump` vs `spawn_event_pump` (app.rs:190). Until it is fixed,
  every verdict in this crate's files must carry "crate does not compile".
  Also check whether INFRA-BLOCKED.md:5-6's compile claim has ANY green run
  on this tree (comments cite CI runs 43384519598 / 43400610890 — comments
  are not evidence per §4 of the instructions; no run output for
  desktop-tauri exists in audit/runs/baseline/).
- W14-A07/A08: the CSP (`connect-src ipc: http://ipc.localhost`,
  `img-src ... blob:`, `style-src 'unsafe-inline'`) and the capabilities set
  (`core:default` + 4 window grants incl. `start-dragging` for the frameless
  chrome) are the security posture claims — my read found no grant reachable
  from main.rs (it registers nothing); the single-command surface is
  app.rs's registration, A01's to verify against `capabilities/default.json`.
- W14-A06: does the crate's Cargo.lock pin a `tauri` 2.x that matches the
  edition-2024 `unsafe extern` posture (lock vs toolchain/edition drift)?
- Wave-level: apps/desktop/src/main.rs (the exit-86 `lumen-desktop` skeleton
  twin) does NOT install the panic hook or write a crash log — if any payload
  still ships that binary, its GUI-failure face is the pre-w30.6 silence.
  INFRA-BLOCKED says it left every user payload; whoever audits
  apps/desktop/src/main.rs should verify against release.yml payload lists.
