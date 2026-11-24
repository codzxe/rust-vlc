# AUDIT: apps/desktop/src/main.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-24 |
| Lines / functions | 107 / 3 (main, os_name, host_label) |
| File verdict | REAL (an honest, documented skeleton — the shipped desktop binary is a typed-pending status printer, and it says so) |
| Reachable from shipped entry point? | IS the shipped entry point: `lumen-desktop` is a release-lane binary (baseline: `cargo build --release -p desktop` exit 0). Everything else in apps/desktop/src (shell/bridge/dispatch) is UNREACHABLE from it — see the W13 desktop dispatch audit |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/lumen-desktop` -> full status printout, exit 86 (transcript below); `rg` rc-smoke/packaging coverage; read window_config/host seams |

## 1. One-paragraph verdict
This is the honest stub done right: the shipped `lumen-desktop` binary prints
exactly what it is (skeleton, card 2.D3), consumes the product.toml [window]
table at startup (a real parse — I executed it: "window build spec
(product.toml [window]): 1280x720 (min 940x600) — title idle key
window.title.idle"), enumerates the three webview host kinds with per-host
pending reasons, prints the ADR-003 per-OS strategy statuses (all honestly
"unmeasured in this sandbox"), and exits 86. I executed it: exit code 86,
"0 never" is true (both return paths are 86). Every doc claim in the header
matches observed behavior. The debts are small: a broken product.toml and the
normal pending state share exit code 86, and rc-smoke defines the binary path
but never runs it. The packaging lanes actively verify this stub's ABSENCE
from user payloads (scripts/packaging/verify-linux-artifacts.sh:37-78: "the
exit-86 stub ... is a CI-only diagnostic") — the CI-only posture is enforced,
not just claimed.

## 2. What actually works (verified)
— verified by `./target/debug/lumen-desktop; echo $?` -> prints the skeleton
notice, the parsed window spec (1280x720 min 940x600, idle title key
window.title.idle, remembered-geometry note), "presentation path: skeleton:
canvas-stream protocol core only", the three webview host lines (all
pending), the three ADR-003 per-OS rows (windows A-or-B prototype-pending,
macos A accepted-unmeasured, linux B accepted-unmeasured), "frame protocol:
v1 (LUF1)", stderr "no webview host compiled: ADR-004 pending", EXIT=86.

## 3. Findings
F-001 [S4]
Where: apps/desktop/src/main.rs:54-57 vs :86
Code:
```
        Err(error) => {
            eprintln!("window config: {error}");
            return ExitCode::from(WEBVIEW_PENDING);
        }
```
What is wrong: a product.toml [window] parse failure exits with the SAME code
(86) as the normal "webview not compiled" pending state, while the header
defines 86 as "host layer not compiled". A script cannot distinguish
config-drift from pending except by scraping stderr. What correct looks like:
a distinct exit code (or a detail code) for the config-parse failure.
Confidence: HIGH — Verification: read (Err path not executed — UNVERIFIED:
would need a rebuild with a broken product.toml, cargo-build-budget).
F-002 [S4]
Where: scripts/rc-smoke.sh:60 (this binary's only smoke wiring)
Code:
```
LUMEN_DESKTOP=$TARGET_DIR/release/lumen-desktop
```
What is wrong: `$LUMEN_DESKTOP` is defined and never used again in the script
(rg: zero further references) — the release smoke builds the binary but never
asserts its exit-86 contract or its output. The binary's behavior is verified
only by ad-hoc runs (like this audit's). What correct looks like: one
rc-smoke line asserting `"$LUMEN_DESKTOP" >/dev/null; [ $? -eq 86 ]`.
Confidence: HIGH — Verification: rg + ran the binary manually.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 23-87 | REAL | — | status print + exit 86; window table consumed at startup (executed); both exit paths 86 (F-001) |
| os_name | 91-97 | REAL | — | stable lowercase names |
| host_label | 100-106 | REAL | — | stable labels |

## 5. Test honesty
No tests target the binary itself (a `fn main`); the desktop crate's tests
cover the library seams it calls (parse_window_table — window_config.rs;
strategy_for / active_presentation_path — host.rs; suites in
apps/desktop/tests/). The binary-level contract (exit 86, output shape) has
NO automated check anywhere I could find — F-002.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this binary reports the typed pending state and" + "the ADR-003 landing, then exits [`WEBVIEW_PENDING`]" | :6-7 | REAL | executed: exit 86 with the pending report |
| "Exit codes: `0` never (there is no window to serve); [`WEBVIEW_PENDING`]" | :11 | REAL | both return paths are ExitCode::from(86); executed |
| "the [window] table is CONSUMED at" + "startup — the build spec the ADR-004 wrapper applies verbatim at" | :26-27 | REAL | executed: the parsed spec prints (1280x720/min 940x600 from product.toml:63-84) |
| "A parse failure is loud (the honest-parse rule:" | :29 | REAL-UNPROVEN | Err path reads correctly (eprintln + nonzero exit); not executed (cargo-build-budget) |
| "enumerate nothing, never fake a" + "window, never block a script waiting on one." | :8-9 | REAL | no window creation code exists; the process exits immediately (executed) |

## 7. Dead / duplicated / unreachable code
None in this file. (The unreachable-by-this-binary library modules are the
W13 desktop dispatch audit's finding, repeated here for the reachability
record: shell.rs/bridge.rs/dispatch.rs/etc. compile but no shipped entry
reaches them.)

## 8. Dependencies & flags
desktop crate only (DesktopHost, TargetOs, WebviewHostKind, WindowTitle,
parse_window_table, strategy_for — all exist, rg-verified). No features, no
unsafe. include_str! pins product.toml at compile time (build fails if the
file is absent — a loud dependency, fine).

## 9. Open questions for the next agent
- Who owns ADR-004 ratification? Until then every desktop capability row in
  the matrix should read blocked/pending — if any row claims desktop
  playback, it is false against THIS shipped entry point.
- The packaging guards (verify-linux-artifacts.sh:37-78) prove the stub is
  excluded from deb/rpm/AppImage/tar.gz/dmg payloads — worth a matrix-side
  cross-check that "desktop" platform claims never cite the stub binary as
  evidence.
