# AUDIT: crates/platform-windows/src/dialog.rs
| Field | Value |
|---|---|
| Auditor | W08-A01 |
| Date | 2026-11-23 |
| Lines / functions | 107 / 7 |
| File verdict | REAL (seam + typed refusal, by design) + UNREACHABLE (shipped builds) |
| Reachable from shipped entry point? | NO — `platform-windows` is an orphan crate; the whole FileDialog vocabulary has zero users outside the three `platform-*` crates (rg -l proof) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `cargo test -p platform-windows` -> 16 passed / 0 failed (exit 0); rg `-l "FilePick|FileDialogRequest|FileDialogs|FileFilter"` over crates+apps; diff vs platform-macos dialog.rs; sed/od quote verification |

## 1. One-paragraph verdict
This file is the OS-neutral dialog **seam** (vocabulary + trait + typed refusal fallback) — it contains no FFI; the real Common-Item-Dialog COM implementation lives in `windows/dialogs.rs` (cfg-windows, true `CLSID_FileOpenDialog` GUID, compile-verified only). What is here is clean: `FilePick::Cancelled` modeled as a non-error, an always-`Err` typed fallback that cannot fake success, and honest docs about modal-on-calling-thread. The defect is shipping reality: `platform-windows` has no dependents, so no shipped lumen-cli/desktop build can ever show a file dialog through this seam — the vocabulary is used by nobody outside the three orphan platform crates. The code is a typed refusal **by design** (honest), and dead **by architecture** (the defect).

## 2. What actually works (verified)
— verified by `cargo test -p platform-windows` -> "test result: ok. 16 passed; 0 failed", including `unavailable_dialogs_returns_the_typed_reason` (open/save both return the exact reason) and `file_dialog_request_carries_the_os_neutral_shape`.
— verified by `sed -n '38,45p' windows/dialogs.rs` -> real FFI constants (`CLSID_File_OPEN_DIALOG` data1 0xDC1C_5A9C matches the SDK CLSID_FileOpenDialog) back the seam doc's claims.

## 3. Findings
F-001 [S2]
Where: crates/platform-windows/src/dialog.rs:63-77 (the `FileDialogs` seam) and 99-107 (fallback)
Code:
```
impl FileDialogs for UnavailableDialogs {
    fn open_file(&mut self, _request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        Err(self.reason.clone())
    }
```
What is wrong: Not the fallback's honesty (returning `Err(reason)` is correct and tested) — the problem is that this is the **only** dialog path any non-Windows build of this crate can reach, and the crate itself is an orphan: `rg -l "FilePick|FileDialogRequest|FileDialogs|FileFilter"` lists exactly the 16 files of the three `platform-*` crates. No app (apps/cli, apps/web, apps/desktop) or core crate constructs a `FileDialogRequest` or calls `WindowsPlatform::dialogs()`. The entire seam — and the 596-line real implementation behind it — is unreachable from every shipped entry point.
Why it matters: Any claim of "Windows file dialogs" in docs/matrix is FALSE-by-unreachability; the shipped CLI's only file-selection mechanism is CLI arguments.
What correct looks like: apps/cli/desktop wire the platform crate and route their open-file UI through the seam.
Confidence: HIGH — Verification: read (rg over manifests and code).

F-002 [S4]
Where: crates/platform-windows/src/dialog.rs:31-36
Code:
```
    pub fn new(label: &str, patterns: &[&str]) -> FileFilter {
        FileFilter {
            label: label.to_string(),
            patterns: patterns.iter().map(|p| (*p).to_string()).collect(),
        }
    }
```
What is wrong: No validation — an empty label, an empty `patterns` slice, or a non-glob string ("mkv" without `*.`) constructs a silently useless filter row that travels to the OS conversion (windows/dialogs.rs) unchallenged. The seam is typed everywhere else (errors carry codes); here garbage passes as data.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/platform-windows/src/dialog.rs (whole file) vs platform-macos/src/dialog.rs
Code: (diff — only the header differs)
```
< //! The Windows lane implements this seam with the Common File Dialogs
< //! (`IFileOpenDialog`/`IFileSaveDialog`, `explorerframe`/`shell32`)
---
> //! The macOS lane implements this seam with `NSOpenPanel`/`NSSavePanel`
> //! (AppKit) behind `cfg(target_os = "macos")` — driven through the
```
What is wrong: The code body (FileFilter/FileDialogRequest/FilePick/FileDialogs/UnavailableDialogs) is triplicated across the three platform crates, byte-identical except the per-OS header (correctly adapted here, unlike events.rs's PulseAudio residue). Admitted mirroring per lib.rs:47-50, unification still `[PROPOSAL]`. Any seam fix lands three times.
Confidence: HIGH — Verification: ran diff.

F-004 [S4]
Where: crates/platform-windows/src/dialog.rs:10-11
Code:
```
//! `NotYetImplemented`/`WrongOs` rows in the platform report say
//! exactly that. Compile-verified via `cargo check --target
```
What is wrong: The compile-verification claim is unreproducible in this sandbox (my `cargo check -p platform-windows --target x86_64-pc-windows-msvc` fails: target rust-std not installed, E0463 in media-types). Not contradicted — but the file's own evidence status rests on a toolchain state I cannot confirm; runtime remains `[VERIFY]` everywhere.
Confidence: HIGH — Verification: ran (check failed for environment reasons).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FileFilter::new | 31-36 | REAL | S4 | No pattern/label validation (F-002) |
| FileDialogs::open_file (trait) | 70 | REAL / unreachable | S2 | Seam contract (F-001) |
| FileDialogs::save_file (trait) | 76 | REAL / unreachable | S2 | Same |
| UnavailableDialogs::because | 88-90 | REAL | — | Typed-refusal constructor |
| UnavailableDialogs::reason | 94-96 | REAL | — | Accessor |
| UnavailableDialogs::open_file | 100-102 | REAL | — | Always Err(reason) — tested |
| UnavailableDialogs::save_file | 104-106 | REAL | — | Same |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unavailable_dialogs_returns_the_typed_reason | YES | open/save must return the exact reason object; a fake Ok would fail `assert_err` |
| file_dialog_request_carries_the_os_neutral_shape | YES (narrowly) | Default-shape + filter construction assertions; catches field regressions, not validation gaps (F-002) |
Both pass in my run (16/16). The real Windows dialog path (windows/dialogs.rs) has **zero** executable tests on any lane — Linux compiles it out, and no Windows CI runs here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The Windows lane implements this seam with the Common File Dialogs … hand-declared COM vtables in [`crate::windows::dialogs`]" | dialog.rs:3-5 | REAL-UNPROVEN | windows/dialogs.rs exists with true SDK GUIDs; compile-only, runtime `[VERIFY]` |
| "User cancel is … a first-class `FilePick::Cancelled`, never an error" | windows/dialogs.rs:14-15 (via seam) | REAL-UNPROVEN | ERROR_CANCELLED mapping is in dialogs.rs; unreachable to execute here |
| "The typed `NotYetImplemented`/`WrongOs` rows in the platform report say exactly that" | dialog.rs:10-11 | REAL | capability.rs:45 (WrongOs), :67 (NotYetImplemented), :78-79 maps NotCompiledForThisOs→WrongOs |
| "Compile-verified via `cargo check --target x86_64-pc-windows-msvc`" | dialog.rs:11-13 | UNVERIFIED | My re-run fails (target absent); cannot reproduce (F-004) |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped builds (F-001) — the entire file plus its 596-line implementation sibling.
- Duplicated: byte-identical bodies in platform-macos and platform-linux dialog.rs (F-003).

## 8. Dependencies & flags
Std-only (`PathBuf`, `String`). The real implementation additionally links ole32/shell32 via `windows::com` (cfg `target_os = "windows"` only). No feature gates. Crate builds and tests green on Linux (fallback lanes).

## 9. Open questions for the next agent
- windows/dialogs.rs (596 lines) is another agent's file — cross-check its STA-per-round thread model against the `subscribe`/handshake patterns audited here in wasapi.rs; the two share com.rs primitives.
- If any doc claims the desktop app opens files via a native dialog on Windows, verify against apps/desktop-tauri (excluded from workspace — its JS side may use tauri's dialog plugin, which would make this Rust seam redundant even if wired).
