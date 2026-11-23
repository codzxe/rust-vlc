# AUDIT: crates/platform-windows/src/windows/dialogs.rs
| Field | Value |
|---|---|
| Auditor | W08-A05 |
| Date | 2026-11-23 |
| Lines / functions | 596 / 9 (+1 extern fn, 3 vtable structs, 6 const GUIDs/masks) |
| File verdict | REAL-UNPROVEN (real hand-rolled COM implementation, never executed; one init bug makes the probe dead-on-arrival on real Windows) |
| Reachable from shipped entry point? | NO — no workspace crate depends on platform-windows (verified: `rg platform-windows` over every Cargo.toml matches only its own manifest + a comment in platform-linux's). Only caller is platform-windows's own `dialog_seam()` (lib.rs:200) behind `cfg(target_os="windows")`; nothing pulls `WindowsPlatform` from lumen-cli/desktop. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | rg (callers/deps, exit 0); sed quote-verification (exit 0); rg unsafe counts (exit 0); baseline cite (no build run — cargo-build-budget + no windows-msvc target installed) |

## 1. One-paragraph verdict
This is a genuine, carefully-shaped hand-rolled Common Item Dialog adapter: the IFileDialog/IFileOpenDialog/IShellItem/IShellItemArray vtable layouts match shobjidl_core.h slot-for-slot, all five GUIDs are byte-correct (DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7, C0B4E2F3-BA21-4773-8DCA-3357EB2EB9B1, D57C7288-D4AD-4768-BE02-9D969532D960, 84BCCD23-5FDE-4CDB-AEA4-AF64B83D78AB, 43826D1E-E718-42EE-BC55-A1E261C37BFE), SIGDN_FILESYSPATH=0x80058000, ERROR_CANCELLED=0x800704C5 (win32 1225, correct HRESULT_FROM_WIN32), FOS_* masks correct, and every HRESULT is checked. But it is dead code twice over: (a) nothing in the shipped build reaches the crate at all, and (b) the `probe()` gate never calls `CoInitializeEx` on its spawned thread, so on real Windows `CoCreateInstance` returns CO_E_NOTINITIALIZED and the real dialogs can never be wired — the code contradicts its own "on its own STA thread" doc. Runtime never exercised anywhere (Linux sandbox; 16 passing crate tests exercise only the fallback paths per baseline cargo-test-workspace.txt:4044-4082).

## 2. What actually works (verified)
- Static ABI verification (my knowledge of shobjidl_core.h, cross-checked against file): vtable slot order for IFileDialog (23 slots after IUnknown+Show), IFileOpenDialog (+GetResults/GetSelectedItems), IShellItem, IShellItemArray — all correct; GUID constants correct; ERROR_CANCELLED/SIGDN/FOS values correct.
- Contrast check proving the probe bug is an omission, not a policy: `rg ComApartment` in wasapi.rs:211 and wasapi.rs:722 shows both WASAPI threads join an apartment before `co_create_instance`; dialogs.rs `run_dialog` (line 341) does too; only `probe` does not.
- Baseline: platform-windows 16/16 tests pass on Linux (audit/runs/baseline/cargo-test-workspace.txt:4044-4082) — but all of them test the `#[cfg(not(windows))]` fallback arms, not this file.

## 3. Findings
F-001 [S2]
Where: crates/platform-windows/src/windows/dialogs.rs:280-297
Code:
```
    pub fn probe() -> Result<CommonItemDialogs, PlatformError> {
        // Diagnostic thread name (debugger/log surface only, not a
        // user-visible string).
        // lumen-allow-literal: diagnostic thread name for the STA probe round.
        spawn_sta("lumen-cid-probe", || {
            // SAFETY: created on this thread inside this closure and
            // released exactly once before it returns; the vtable
            // layout is the documented IFileOpenDialog order.
            let dialog =
                unsafe { co_create_instance(&CLSID_FILE_OPEN_DIALOG, &IID_FILE_OPEN_DIALOG)? };
            // SAFETY: the single release of the probe's reference.
            unsafe {
                release_interface(dialog);
            }
            Ok::<(), PlatformError>(())
        })??;
        Ok(CommonItemDialogs)
    }
```
What is wrong: `spawn_sta` only spawns a plain `std::thread` (line 302-319); no `ComApartment::new()` is created inside this closure. COM initialization is per-thread; a fresh thread has none, so `CoCreateInstance` (com.rs:172-180) returns CO_E_NOTINITIALIZED (0x800401F0) on every real Windows machine. The probe therefore always fails, `dialog_seam()` (lib.rs:200-206) always wires `UnavailableDialogs`, and the working `run_dialog` path is unreachable even on Windows. The doc comment claims it runs "on its own STA thread" and "verifies the shell classes are present" — neither is true: the thread is never made an STA and the classes can never be verified. `run_dialog` (line 341: `let _apartment = ComApartment::new()?;`) and both WASAPI threads (wasapi.rs:211, 722) prove the discipline exists and was missed here.
Why it matters: the sole gate that enables the real dialog integration is broken; on the target OS the feature is permanently `Unavailable` with a misleading `CallFailed` reason.
What correct looks like: `let _apartment = ComApartment::new()?;` as the first line of the probe closure.
Confidence: HIGH — Verification: read (runtime UNVERIFIED: no Windows SDK; doc itself marks real-device `[VERIFY]`, dialogs.rs:20-22).

F-002 [S3]
Where: whole file; evidence: crates/platform-windows/Cargo.toml `[dependencies]` (audio-output, media-types only) + rg over all manifests
Code:
```
[dependencies]
audio-output = { path = "../audio-output" }
media-types = { path = "../media-types" }
```
What is wrong: no crate in the workspace depends on platform-windows; `WindowsPlatform::discover` (its only entry) is called only by its own test file (tests/seam_behaviour.rs:401). 596 lines of real FFI are shipped-island code: the release lane (`cargo build --release -p lumen-cli -p desktop`) never compiles the `#[cfg(target_os="windows")]` module on any OS it ships.
Why it matters: the "Windows platform adapter" adapts nothing in the product; matrix row `ui-dialog-open-file` is implemented by the webview dialog instead (docs/feature-matrix.toml:2825-2827, "NATIVE file/folder pickers (4.H1) ... still open").
What correct looks like: a `device-discovery`/`platform` crate that actually consumes the seam, or an explicit dead-code note in the registry.
Confidence: HIGH — Verification: ran rg over every Cargo.toml (exit 0, no dependents).

F-003 [S4]
Where: crates/platform-windows/src/windows/dialogs.rs:366 (also 530, 556); contradicts lib.rs:7-9
Code:
```
) -> Result<FilePick, PlatformError> {
    let vtbl = unsafe { (*(dialog as *mut ObjectHeader)).vtbl as *const FileDialogVtbl };
```
lib.rs claims:
```
//! Windows — the only `unsafe` in this crate lives in the FFI
//! modules ([`windows`]), every block safety-commented — it
```
What is wrong: 24 `unsafe {` blocks; 21 carry a local SAFETY comment; the three raw vtable-header reads (366, 530, 556) carry none, contradicting the crate's stated "every block safety-commented" rule. (The reads themselves are the standard COM pattern documented in com.rs:12-22 and are sound.)
Confidence: HIGH — Verification: rg -c "unsafe \{" → 24, rg -c "SAFETY:" → 22 (extern block + vtable-read gaps accounted).
F-004 [S4]
Where: crates/platform-windows/src/windows/dialogs.rs:20-22 (module doc)
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! x86_64-pc-windows-msvc`; runtime NOT exercised in this sandbox
//! (no Windows SDK) — `[VERIFY]` on real-device behavior.
```
What is wrong: the compile-verification claim is recorded (worklog.md:556) but not re-runnable here (no rustup, no windows-msvc target installed) — UNVERIFIED; also `cargo check` typechecks declarations without linking, so even when it ran it proved nothing about symbol resolution against ole32/shell32.
Confidence: MEDIUM — Verification: UNVERIFIED (no cross-target toolchain in sandbox).
F-005 [S4]
Where: crates/platform-windows/src/windows/dialogs.rs:313-318 (spawn_sta join error) and 405 (`dir.display().to_string()`)
Code:
```
        .join()
        .map_err(|_| PlatformError::CallFailed {
            api: name,
            code: None,
            detail: "the dialog thread panicked".to_string(),
        })
```
What is wrong: the panic payload is discarded (`|_|`) — the actual panic message never reaches the error; and `display().to_string()` lossifies non-UTF-8 default-dir paths before `SHCreateItemFromParsingName` (WTF-16 paths cannot round-trip through Display). Both are small-blast-radius quality defects. Raw COM pointers also have no RAII guard, so a panic between create (350) and release (353-355) would leak — no panicking operation exists on that path today.
Confidence: HIGH (code read) / runtime UNVERIFIED.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CommonItemDialogs::new | 268-271 | REAL | — | returns unit struct, Default-derived too |
| CommonItemDialogs::probe | 280-297 | FALSE (doc) / broken impl | S2 | F-001: no CoInitializeEx → always CO_E_NOTINITIALIZED on Windows |
| spawn_sta | 302-319 | REAL | S4 | name lies for probe (not an STA); discards panic payload |
| FileDialogs::open_file | 322-328 | REAL-UNPROVEN | — | clones request, delegates to run_dialog on spawned thread |
| FileDialogs::save_file | 330-336 | REAL-UNPROVEN | — | same shape |
| run_dialog | 340-357 | REAL-UNPROVEN | — | correct apartment+create+release ordering |
| configure_and_show | 361-526 | REAL-UNPROVEN | S4 | all HRESULTs checked; cancel→Cancelled; vtbl read uncommented (F-003); release ordering on early-Err paths verified correct |
| shell_item_path | 529-552 | REAL-UNPROVEN | S4 | GetDisplayName→from_wide→CoTaskMemFree, single free, correct |
| array_paths | 555-596 | REAL-UNPROVEN | — | per-item release; skip-vanished-item policy; empty→typed Err (honest) |
| extern SHCreateItemFromParsingName | 99-107 | REAL-UNPROVEN | — | prototype matches shell32 ABI; SAFETY comment present |

## 5. Test honesty
No test in this file. Crate tests (tests/seam_behaviour.rs, 16 passing per baseline) exercise only the non-Windows fallback arms; `discover_on_a_foreign_host_reports_wrong_os_everywhere` (line 400-430) asserts the *fallback* is wired on Linux — it would still pass if dialogs.rs were deleted entirely. Nothing anywhere would fail if this file's logic were broken. Mutation verdict: zero test coverage of the audited code.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via cargo check --target x86_64-pc-windows-msvc" | dialogs.rs:20-22 | UNVERIFIED | worklog.md:556 records it; cannot re-run (no target installed) |
| probe runs "on its own STA thread" / "verifies the shell classes are present" | dialogs.rs:273-279 + module doc 8-12 | FALSE | F-001: no CoInitializeEx on the spawned thread |
| "every block safety-commented" (crate-level) | lib.rs:7-9 | FALSE for this file | 3 of 24 blocks uncommented (366/530/556) |
| "the OS centers it on the active monitor" (NULL hwnd) | dialogs.rs:13-16 | UNVERIFIED | runtime claim, no Windows host |
| native pickers "still open", ui-dialog-open-file implemented via webview | docs/feature-matrix.toml:2825-2827 | REAL | consistent with F-002 (this crate unused) |

## 7. Dead / duplicated / unreachable code
Entire file is UNREACHABLE from shipped entry points (F-002). No internal dead code found; all vtable slots used or deliberately underscore-prefixed.

## 8. Dependencies & flags
`#![allow(unsafe_code)]` (line 26) opts out of the workspace `unsafe_code = "deny"` lint — matches the documented "SAFETY-SCOPED MODULE" exception (doc 01 §2.7 / MP §9). No windows-s/winapi dependency (Cargo.toml is std-only + audio-output + media-types). No cargo features. Links shell32 (dialog path) + ole32 via com.rs.

## 9. Open questions for the next agent
- WasapiAudioDevices / DxgiAdapters have the same island status — confirm crate-level reachability verdict once for all platform-* crates (this agent covers audio.rs/events.rs/objc.rs next; dxgi.rs is unassigned to me).
- If anyone ever wires platform-windows in: re-run the probe on real hardware before trusting the capability table (F-001).
