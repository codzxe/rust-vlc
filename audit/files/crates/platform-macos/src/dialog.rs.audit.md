# AUDIT: crates/platform-macos/src/dialog.rs
| Field | Value |
|---|---|
| Auditor | W08-A02 |
| Date | 2026-11-15 |
| Lines / functions | 108 / 6 (2 ctors, 2 accessors, 2 trait-impl refusals + trait) |
| File verdict | UNREACHABLE (real typed seam; the macOS implementation it fronts is real-but-unexercised FFI; the whole crate has zero consumers) |
| Reachable from shipped entry point? | NO — same crate-level fact as coreaudio.rs: no crate/app depends on `platform-macos`; `MacOsPlatform::discover` (the only wiring) is itself never called outside the crate's own tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg for `platform-macos|platform_macos` consumers (only self-references, see coreaudio.rs audit); rg `pthread_main_np|fn probe|NSOpenPanel` in src/macos/dialogs.rs (exists, guard real); rg dialog seam usage in tests/seam_behaviour.rs (fallback-only coverage) |

## 1. One-paragraph verdict
A clean, small typed seam: request/response data carriers, a `FileDialogs` trait with two methods, and an `UnavailableDialogs` fallback that returns the exact wrapped reason from every method — no fake success, no swallowed errors. The code does what its docs say. The two problems are structural, not local: nothing in the shipped product can ever reach this file (the crate is an orphan), and the header's "compile-verified via `cargo check --target aarch64-apple-darwin`" claim has no surviving log anywhere I can cite. On Linux the fallback is genuinely tested (seam_behaviour.rs:62-94); the AppKit path behind it (macos/dialogs.rs, another agent's file) has zero runtime coverage.

## 2. What actually works (verified)
— rg confirms `src/macos/dialogs.rs` exists and declares the `pthread_main_np` main-thread guard and `AppKitPanels::probe()` the header describes (dialogs.rs:7,52-63,89) — the doc's references are not dangling.
— The fallback's refusal semantics are tested on this host's lane: tests/seam_behaviour.rs:70-72 asserts `open_file`/`save_file` return the typed reason (host-runnable; baseline cargo-test-workspace green).

## 3. Findings
F-001 [S3]
Where: crates/platform-macos/src/dialog.rs (whole file; crate-level)
Code:
```
/// The dialog seam.
pub trait FileDialogs: Send {
```
What is wrong: unreachable from any shipped entry point. `rg platform-macos|platform_macos` over crates/, apps/, tools/, tests/ finds only this crate's own manifest and its own tests. No lumen-cli/desktop path constructs `MacOsPlatform`, so neither the seam nor the AppKit implementation behind it can execute in a shipped binary. Why it matters: the typed-fallback honesty this file implements is theater with no audience; the product's real dialog story (Tauri/webview, if any) lives elsewhere. What correct looks like: wire the platform crates into a facade the engine consumes, or record the crate as intentionally-not-yet-wired. Confidence: HIGH — Verification: rg.
F-002 [S4]
Where: crates/platform-macos/src/dialog.rs:12-14
Code:
```
//! Compile-verified via `cargo check --target aarch64-apple-darwin`
//! and `x86_64-apple-darwin` (runtime NOT exercised in this sandbox —
//! no macOS SDK).
```
What is wrong: the compile-verification claim has no artifact behind it — no log under audit/runs/baseline/ or docs/ records that cross-check run (rg `aarch64-apple-darwin` in docs/ returns only packaging/h264 records). The runtime-not-exercised half is honest and matches this sandbox. Why it matters: "compile-verified" is the ONLY verification this FFI lane claims; if it did not happen (or cannot be reproduced), the lane's entire evidence base is reading the code. What correct looks like: a checked-in log of the cross-check (command + exit code) next to the claim. Confidence: MEDIUM (absence of evidence in repo). Confidence on the honesty half: HIGH.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FileFilter::new | 32-37 | REAL | — | label + patterns into owned Strings |
| FileDialogRequest (struct) | 42-51 | REAL | — | OS-neutral DTO, Default derived |
| FilePick (enum) | 55-62 | REAL | — | Cancelled / Paths; PartialEq for tests |
| FileDialogs::open_file / save_file (trait) | 65-78 | REAL (contract) | — | typed Errors documented |
| UnavailableDialogs::because / reason | 89-97 | REAL | — | wraps + exposes reason |
| UnavailableDialogs::{open_file, save_file} | 101-107 | REAL (typed refusal) | — | `Err(self.reason.clone())` — never a fake success; tested on host |

## 5. Test honesty
| Test (seam_behaviour.rs) | Would it fail if code were broken? | Reason |
|---|---|---|
| unavailable_dialogs_returns_the_typed_reason (62-72) | YES | asserts both methods return the exact reason object passed in |
| trait-object + FilePick equality checks (85-94) | YES (weakly) | dyn dispatch + PartialEq sanity; would catch derive regressions only |
No test covers the AppKit path (cannot, on Linux) — the real implementation has zero runtime coverage anywhere.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The macOS lane implements this seam with NSOpenPanel/NSSavePanel ... behind cfg(target_os = 'macos')" | dialog.rs:3-6 | TRUE (code exists) | src/macos/dialogs.rs:1-7,89 (rg) |
| "enforced main-thread guard (pthread_main_np)" | dialog.rs:6-7 | TRUE (code exists) | dialogs.rs:52-63 |
| "Compile-verified via cargo check --target aarch64-apple-darwin" | dialog.rs:12-13 | REAL-UNPROVEN | no artifact in repo (F-002) |
| "On any other OS the seam is the typed UnavailableDialogs fallback" | dialog.rs:9-11 | TRUE | lib.rs:204-209 wires it; seam test asserts refusals |

## 7. Dead / duplicated / unreachable code
Whole crate unreachable (F-001). Deliberate triplication of this seam across platform-{macos,windows,linux} is recorded as a Lead-level [PROPOSAL] to unify (lib.rs:47-50) — known, documented duplication, not hidden debt.

## 8. Dependencies & flags
std only (PathBuf). No unsafe in this file (the unsafe lives in macos/dialogs.rs). cfg gates: none here (OS dispatch happens in lib.rs).

## 9. Open questions for the next agent
- Whoever audits macos/dialogs.rs: the modal-on-main-thread design makes `open_file` block the caller; if the engine ever calls this on its session thread, playback stalls for the dialog's lifetime — check the intended call site (none exists today).
