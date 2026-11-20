# apps/desktop/src/diagnostics.rs — the crash handler (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `apps/desktop/src/diagnostics.rs` (458 LOC; 209 production / 249 test) |
| Reachability | lumen-tauri binary → `main.rs:49` `desktop::diagnostics::install_panic_hook()` (first statement of main) + `main.rs:53` `write_crash_log("startup", &error)` on every startup failure. Every lumen-tauri launch installs it before anything can fail. |
| Verdict | **Real crash handler — for Rust panics and startup errors. It is NOT an OS-signal handler.** The artifact (deterministic crash log under `<OS tmp>/lumen/lumen-tauri-crash.log`, latest-wins) is genuinely written, panic-free by construction, and pinned by real tests including a REAL `panic!` under `catch_unwind`. A native crash (SIGSEGV inside wry/webkit FFI) leaves nothing — that class has no face. |

## Per-function table (production half)

| Function | Lines | What it does (sed-verified) |
|---|---|---|
| `crash_log_path` | 66-70 | `temp_dir()/lumen/lumen-tauri-crash.log` (consts at 52-57). |
| `crash_log_path_with_stamp` | 76-83 | rotation sibling for tests. |
| `write_crash_log` | 95-97 | writes the default sink for any `&dyn Error`; best-effort. |
| `write_crash_log_at` | 103-117 | `create_dir_all` + `fs::write`, BOTH failures swallowed (lines 109-115), returns the intended path regardless (line 116). |
| `crash_log_body` | 121-139 | banner + `env!("CARGO_PKG_VERSION")` + platform + stage + UTC timestamp (no-dep civil-from-days, 239-260) + error text + how-to-report. |
| `error_chain` | 144-153 | walks `source()` hop by hop as `caused by:` lines. |
| `install_panic_hook` | 166-179 | `take_hook` + `set_hook`: writes the SAME artifact (stage `"panic"`) with location + payload (lines 168-175), then `previous(info)` (line 177) — wraps, never replaces. |
| `payload_text` | 201-209 | `&str`/`String` downcasts; else "non-string panic payload". |
| `platform_label` / `unix_now` | 212-231 | compile-time platform; pre-epoch clock degrades to epoch (never fails). |

Key quotes (sed-verified):
- Lines 167-168: `let previous = std::panic::take_hook(); std::panic::set_hook(Box::new(move |info| {`.
- Line 175: `let _sink = write_crash_log(STAGE_PANIC, &failure);` followed by line 177 `previous(info);`.
- Lines 30-35 (module doc): "`write_crash_log` NEVER panics and NEVER masks the failure being reported: every IO error ... is swallowed".

## Findings

- **S3 — no OS-signal coverage.** `rg -n -i "signal|SIGSEGV|SIGABRT|sigaction"` over `apps/desktop-tauri/src` + `apps/desktop/src` → zero code hits. The webview process's most realistic hard-death (a segfault in native webkit/WebView2 code, or an abort in tauri's own FFI) produces no crash log, no stderr face, no dialog: the window vanishes. The handler's honest scope is Rust panics and startup `Err` returns only. The module docs do not claim otherwise (they never say "signal") — but nothing in the repo records this gap for the native half either. The W13-A02 verdict on "crash handler: real signal handling or a print?" — it is a real panic hook + real file artifact, and it is neither an OS-signal handler nor a mere print.
- **S3 — latest-wins single sink collides on shared Unix hosts.** The module doc itself concedes it (lines 23-26): "on shared Unix hosts the fixed path can collide across users — accepted for a best-effort diagnostic sink". A two-user terminal server trades crash reports silently.
- **S4 — panic location of `install_panic_hook` double-call chains hooks** (line 155-157 doc) — the tauri shell calls it exactly once (main.rs:49). Dormant.
- **S4 — `PanicFailure` has no `source()`** (line 183-195) — the chain walk terminates at the payload by design; a `#[panic]`-payload wrapping an error loses the chain. Cosmetic.

## Test honesty

Genuine and adversarial: the sabotaged-sink test (402-421) proves the intended-path contract; `panic_hook_writes_the_log_and_still_runs_the_previous_hook` (424-457) executes a REAL `panic!("payload {marker}")` under `catch_unwind` and asserts the log carries `stage: panic`, the payload, and `diagnostics.rs` as the location file. `iso_utc_pins_known_instants` (355-363) pins the calendar math incl. a leap day. Tests serialize on the real sink via a mutex (278) — honest about the shared fixed path.

## Doc claims

- KNOWN-LIMITS.md:50 claims "w30.6 gave it a visible crash face (panic hook + crash log under `<tmp>/lumen/` + a Windows no-console MessageBox with the WebView2 hint)" — matches the code exactly (this file + main.rs win32).
- INFRA-BLOCKED.md §B.6 repeats the same claim — consistent.

## Dead code / deps / flags

- `crash_log_path_with_stamp` is test-only (used at line 341) — pub API for a test facility; dormant in production. S4.
- Deps: std only (no `chrono`/`time`) — matches the crate's std-only posture; no unsafe (the win32 FFI half lives in the excluded crate).

## Open questions

1. Would a `signal-hook`-style minimal SIGSEGV/SIGABRT writer be admissible under the dependency posture? (Std-only crate + unsafe deny means it would land in the excluded wrapper, like win32.) Nothing in the docs plans it — the native-crash blind spot stands at HEAD.
