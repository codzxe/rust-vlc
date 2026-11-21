# AUDIT: crates/platform-windows/src/dialog.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 9 (1 struct + ctor, 1 struct, 1 enum, 1 trait + 2 methods, 1 fallback struct + 2 ctors/accessors + 1 impl, module docs; 0 tests in file, 2 in tests/seam_behaviour.rs) |
| File verdict | UNREACHABLE (real seam + typed fallback; crate orphaned — dxgi.rs F-001) |
| Reachable from shipped entry point? | NO (platform-windows depends into nothing — dxgi.rs F-001; additionally `rg "FileDialogs\|FilePick\|FileDialogRequest"` over apps/ and non-platform crates returns zero consumers: no UI, engine, or CLI code calls this seam) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | `ls crates/platform-windows/src/windows/` (dialogs.rs exists — the doc's implementation pointer is valid) · `diff` vs platform-linux/src/dialog.rs (bodies mirrored; module docs correctly platform-specific this time) · `rg -l "FileDialogs\|FilePick\|FileDialogRequest"` excluding platform-* (zero external users) · baseline log: `file_dialog_request_carries_the_os_neutral_shape`, `unavailable_dialogs_returns_the_typed_reason` passed (:3723, :3734) |

## 1. One-paragraph verdict
A clean OS-neutral dialog seam: request/filter/pick vocabulary, a `Send` trait whose modal-on-calling-thread shape is documented as the OS API's own, and a typed fallback that returns the wrapped reason on every call — no fake success anywhere. The referenced Windows implementation (`windows::dialogs.rs`) exists. The single finding is the inherited one: the crate is a dependency of nothing, so this seam — like the DXGI and events seams — can never be invoked by a shipped build, and no code outside the platform crates references its types at all. Unlike events.rs, the mirrored body here carries correctly platform-specific module docs, and the two covering tests passed in the baseline.

## 2. What actually works (verified)
— 2 covering tests passed in the no-fail-fast baseline (:3723, :3734, within the 16/16 suite): `file_dialog_request_carries_the_os_neutral_shape`, `unavailable_dialogs_returns_the_typed_reason` — on this Linux host the exercised path is the typed fallback, which is the only path any current build can reach.
— The doc's implementation pointer is real: `crates/platform-windows/src/windows/dialogs.rs` exists (cfg-gated module, confirmed by ls).
— `UnavailableDialogs` honesty verified by reading: both methods return `Err(self.reason.clone())` — a typed refusal, never `Ok` with invented paths; the `unavailable_dialogs_returns_the_typed_reason` test pins it.
— Mirroring check: the code body is identical to platform-linux's dialog.rs (deliberate [PROPOSAL] mirroring per lib.rs), but unlike events.rs the module docs here are genuinely Windows-specific — no copy-paste platform soup.

## 3. Findings
### F-001 [S3]
Where: crates/platform-windows/src/dialog.rs:63-77 (the seam), cross-ref dxgi.rs F-001
Code (dialog.rs:63-70):
```
/// The dialog seam.
pub trait FileDialogs: Send {
    /// Show a modal file-open dialog.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when no dialog backend could run
    /// (integration absent, every fallback program missing).
    fn open_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError>;
```
What is wrong: the crate-level orphan status (dxgi.rs F-001 evidence chain: no Cargo.toml in apps/ or crates/ depends on platform-windows) plus a seam-specific fact: `rg` for `FileDialogs|FilePick|FileDialogRequest` across apps/ and all non-platform crates finds zero consumers. No shipped UI, engine, or CLI ever constructs a `FileDialogRequest` or implements `FileDialogs` — the desktop app's file pickers (if any) do not go through this seam.
Why it matters: the file-dialog capability advertised in the platform report (`Feature::FileDialogs` → "platform.dialogs", capability.rs) is a seam with no caller on any platform; the "compile-verified" Windows implementation (dialogs.rs, runtime NOT exercised per this file's own header) is doubly unreachable — cfg-gated and unlinked.
What correct looks like: wire the desktop app's open/save flows through the platform seam (that is what it is for), or document the seam as reserved vocabulary.
Confidence: HIGH — Verification: Cargo.toml exhaustion (dxgi.rs audit) + type-name rg exhaustion.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-13 | REAL-UNPROVEN | — | honest banner; the "compile-verified" claim is UNVERIFIED (no windows-target run in baseline, same as dxgi.rs) |
| `FileFilter` + `new` | 20-37 | REAL | — | label + glob strings; shape test passed |
| `FileDialogRequest` | 40-50 | REAL | — | OS-neutral (ARCH-6), Default derive; `multiple` documented open-only |
| `enum FilePick` | 53-61 | REAL | — | Cancelled-is-not-an-error semantics documented; Paths for both open (many) and save (one) — documented |
| `trait FileDialogs` (open_file/save_file) | 63-77 | UNREACHABLE | S3 | F-001; modal-on-calling-thread documented as the OS shape |
| `UnavailableDialogs` + `because` + `reason` | 80-97 | REAL | — | typed fallback; `because` used by the cfg(not(windows)) discover path (lib.rs) |
| `impl FileDialogs for UnavailableDialogs` | 99-107 | REAL | — | both methods return the wrapped reason — tested |

## 5. Test honesty (2 tests in tests/seam_behaviour.rs, both passed — baseline :3723, :3734)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `file_dialog_request_carries_the_os_neutral_shape` | WEAK-YES | structural: constructs a request and asserts fields round-trip — catches vocabulary drift, not behavior |
| `unavailable_dialogs_returns_the_typed_reason` | YES | asserts open/save both return exactly the wrapped reason |
| Gap | — | the real dialogs.rs (Windows COM path) has no executable coverage anywhere — consistent with its own [VERIFY] banner; nothing in-repo can test it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The Windows lane implements this seam with the Common File Dialogs ... behind cfg(target_os = "windows")" | dialog.rs:3-5 | REAL (code exists) / UNREACHABLE (shipped) | windows/dialogs.rs exists (ls); crate unlinked (F-001) |
| "The dialog is modal on the calling thread (the OS API's own shape — the seam is a synchronous request by design)" | dialog.rs:6-8 | REAL (design) | trait shape is synchronous `&mut self` |
| "On any other OS the seam is the typed UnavailableDialogs fallback" | dialog.rs:8-10 | REAL | cfg(not(windows)) discover path + test passed |
| "Compile-verified via cargo check --target x86_64-pc-windows-msvc (runtime NOT exercised ...)" | dialog.rs:11-13 | UNVERIFIED | no such run in audit/runs/baseline/ — an in-repo claim with no committed evidence |

## 7. Dead / duplicated / unreachable code
- The whole seam is unreachable in shipped builds (F-001) — and unlike events.rs, no consumer exists even inside other crates.
- Body mirrored in platform-linux/src/dialog.rs and (presumably) platform-macos — deliberate [PROPOSAL] mirroring; docs correctly diverge per platform (verified by diff).

## 8. Dependencies & flags
- std (PathBuf) + crate::error only. No unsafe in this file (the FFI lives in windows/dialogs.rs, outside my assignment). File compiles on all OSes (not cfg-gated) — which is why its fallback tests run and pass on Linux.

## 9. Open questions for the next agent
- windows/dialogs.rs (unassigned to me): the COM vtable declarations for IFileOpenDialog/IFileSaveDialog — same ABI-layout verification I did for dxgi.rs's vtables is needed there, plus the STA-thread claim from the crate docs.
- UI wave: does the desktop/tauri app have file-open UI, and if so what does it call? If it uses tauri's native dialog plugin, this seam is redundant vocabulary; if it has nothing, the product cannot open files via dialog at all — either way the seam's absence of callers is a product-level fact worth recording.
- platform-macos auditor: confirm the third dialog.rs mirror and that its docs are macos-specific (the events.rs copy-paste failure suggests checking each mirror).
