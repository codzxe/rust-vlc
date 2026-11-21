# AUDIT: apps/desktop/src/main.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 4 (`main`, `os_name`, `host_label`, `WEBVIEW_PENDING`) |
| File verdict | REAL |
| Reachable from shipped entry point? | It IS an entry point (the `lumen-desktop` bin, built in baseline #1, integration-tested), but NOT the shipped desktop face — that is `lumen-desktop-tauri` per ADR-004 (desktop-tauri/Cargo.toml). This binary is the documented pre-ADR-004 placeholder that always exits 86. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "parse_window_table\|include_str\|product.toml" apps/desktop-tauri/src/app.rs` (:243 — the real shell bakes product.toml at compile time identically); `rg -n "WEBVIEW_PENDING\|from\(86\)\|status().code" apps/` (the exit contract IS pinned: apps/desktop/tests/pipeline.rs:322); Read apps/desktop/tests/pipeline.rs:315-340 (the integration test runs the real binary and pins stdout/stderr/exit); `sed -n` quote verification |

## 1. One-paragraph verdict
An honest skeleton that does exactly what it claims: parse the window build spec, print the presentation-path and per-OS ADR-003 report, then exit 86 (`WEBVIEW_PENDING`) — never a fake window, never exit 0. Its whole observable contract is pinned by a real integration test that executes the binary and asserts the exit code, the window-spec line (1280x720 min 940x600), the geometry line, the variant labels, the webview names, LUF1, and the stderr ADR-004 notice — and that test passed in the baseline (pipeline.rs 4/4). Two hygiene debts only: the "consumed at startup" wording hides that product.toml is baked in at compile time via `include_str!` (the parse is at startup; the file is never read at runtime — and the real tauri shell does the same), and `pub const WEBVIEW_PENDING` is dead-public in a binary crate whose only test hard-codes 86.

## 2. What actually works (verified)
- The exit contract is proven by execution: `stub_binary_reports_pending_state` (apps/desktop/tests/pipeline.rs:317-340) runs `CARGO_BIN_EXE_lumen-desktop`, asserts `status.code() == Some(86)`, and pins nine stdout/stderr substrings — passed in the baseline rerun (pipeline.rs "4 passed", orchrun-test-workspace-nofailfast.log.txt, desktop section). — verified by reading the test + the cited passing result.
- The report is driven by real lib calls (`parse_window_table`, `DesktopHost::active_presentation_path`, `strategy_for`) — not hard-coded strings (only the labels `os_name`/`host_label` are local, and their outputs are pinned by the same test). — verified by reading.
- Parse-failure path: `Err` → stderr + exit 86 (lines 54-57) — the honest-parse rule stated and followed. — verified by reading (the failure path itself has no dedicated test — see §5).

## 3. Findings

### F-001 [S4]
Where: apps/desktop/src/main.rs:26-31
Code:
```rust
    // Issue #3 (4.H1 handoff c): the [window] table is CONSUMED at
    // startup — the build spec the ADR-004 wrapper applies verbatim at
    // window build (inner/min sizes, maximized, geometry policy, idle
    // title key). A parse failure is loud (the honest-parse rule:
    // config drift never invents a window).
    match parse_window_table(include_str!("../../../product.toml")) {
```
What is wrong: The comment says the `[window]` table is "CONSUMED at startup"; the `include_str!` bakes the bytes into the binary at compile time. The PARSE runs at startup, but the FILE is never read at runtime — a deployed `product.toml` edit changes nothing this binary reports. The real shell has the identical shape (`parse_window_table(include_str!("../../../product.toml"))`, desktop-tauri/src/app.rs:243), so the bake is a product-level decision, but this file's comment describes runtime consumption that does not happen.
Why it matters: A reader/operator tuning `[window]` in a deployed product.toml expects the documented "consumed at startup" behavior; the actual contract is "consumed at build".
What correct looks like: State the compile-time bake in the comment (or read the file at startup if runtime configurability is intended — a Lead decision that applies to app.rs too).
Confidence: HIGH — Verification: read (both call sites).
### F-002 [S4]
Where: apps/desktop/src/main.rs:19-21
Code:
```rust
/// The documented exit code for "the webview host layer is not
/// compiled into this build" (ADR-004 pending).
pub const WEBVIEW_PENDING: u8 = 86;
```
What is wrong: `pub` on a constant in a binary crate is meaningless (no external consumer can import it), and the only test that pins the value hard-codes `86` (pipeline.rs:322) rather than referencing the constant — the name and the literal can drift apart with no compile error.
Why it matters: Trivial hygiene; the drift risk is one renamed constant away from a silently stale doc.
What correct looks like: Drop `pub`, and have the integration test reference the value via a shared test helper or re-state the pin deliberately.
Confidence: HIGH — Verification: read + rg (no other references).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-12 | REAL | — | "0 never" exit contract accurate and test-pinned |
| `WEBVIEW_PENDING` | 19-21 | REAL | S4 | F-002 |
| `main` | 23-87 | REAL | S4 | F-001 wording; every path exits 86 (line 56 error path, line 86 success-of-report path) — matches docs; println/eprintln split sane |
| `os_name` | 89-97 | REAL | — | stable lowercase names; output pinned via test's "WebView2"/"WKWebView"/"WebKitGTK 4.1" assertions (via `report.os.webview()`) |
| `host_label` | 99-106 | REAL | — | stable labels; same pinning story |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `stub_binary_reports_pending_state` (apps/desktop/tests/pipeline.rs:317-340) | YES | executes the real binary; pins exit 86 + nine report substrings including the live product.toml-derived window spec |
| (no test) the parse-FAILURE path (lines 54-57) | NO TEST EXISTS | requires a malformed product.toml at compile time — untestable without a fixture binary; acceptable for a skeleton, noted for completeness |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "reports the typed pending state … then exits WEBVIEW_PENDING" | module docs 5-7 | REAL | integration test pins exit 86 + stderr ADR-004 |
| "0 never (there is no window to serve)" | module docs 11-12 | REAL | both exit sites are `ExitCode::from(WEBVIEW_PENDING)`; test-pinned |
| "the [window] table is CONSUMED at startup" | comment 26-27 | SHALLOW | parse at startup, bytes at compile time (F-001) |
| "never block a script waiting on one" | module docs 8-9 | REAL | the binary exits immediately; no loops, no waits (read) |

## 7. Dead / duplicated / unreachable code
- `pub` on `WEBVIEW_PENDING` (F-002). No other dead code — 106 lines, all executed by `main`.

## 8. Dependencies & flags
Depends only on the `desktop` lib (`DesktopHost`, `TargetOs`, `WebviewHostKind`, `WindowTitle`, `parse_window_table`, `strategy_for`) and std. No feature gates. The `include_str!("../../../product.toml")` couples the binary to the repo root layout (compile-time checked).

## 9. Open questions for the next agent
1. (desktop shell.rs part2 / desktop host.rs owners — W13-A08/A09) The shipped desktop face is `lumen-desktop-tauri` (app.rs), which this skeleton defers to. The wave's hostile question — where the 4:4:4 H.264 decode error becomes an infinite HANG — must live in the tauri app's event loop or the desktop shell's settle loop, not here (this binary has no loops). `apps/desktop-tauri/src/app.rs` is not on any wave-13 list I can see — flag to the orchestrator if unassigned.
2. (desktop-tauri owner / packaging) F-001's compile-time product.toml bake is inherited by app.rs:243 — the same finding applies to the shipped shell; decide whether window config is meant to be runtime-tunable.
