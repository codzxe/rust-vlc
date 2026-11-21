# AUDIT: crates/platform-windows/src/windows/dialogs.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 596 / 12 items |
| File verdict | UNREACHABLE (real hand-written COM code, cross-compile-verified, never linked into any shipped binary, never executed) |
| Reachable from shipped entry point? | NO — module is `#[cfg(target_os = "windows")]` (windows/mod.rs:17-18) so Linux builds do not even compile it; more decisively, `rg -g Cargo.toml "platform-windows"` shows NO crate or app depends on platform-windows at all (apps/cli deps: 16 crates, none platform-*; apps/desktop-tauri: desktop+commands+tauri). Nothing in the shipped graph can reach this code on ANY OS. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `rg -n "CommonItemDialogs\|FileDialogs" -l` → only platform-{linux,macos,windows} internal files; `rg -g Cargo.toml "platform-"` in apps/* → zero deps; `sed -n '1,30p' windows/mod.rs` → cfg gate + honesty header; read com.rs helper signatures (ComApartment STA at 119-149, co_create_instance 166, release_interface 198, from_wide 311, to_wide 337, free_wide 355-359); `rg -i dialog docs/feature-matrix.toml` → no matrix row claims this; baseline `orchrun-check-platform-windows.log.txt` read (cross-check EXIT=0) |

## 1. One-paragraph verdict
A careful, hand-declared COM implementation of the Windows Common Item Dialogs: the GUIDs, FOS option bits, SIGDN value, ERROR_CANCELLED HRESULT and both vtable layouts match the real Windows SDK ABI (verified against SDK knowledge, slot-by-slot), and the reference-counting discipline is traced correct on every path — every interface released exactly once, CoTaskMemFree paired with GetDisplayName's allocation, the STA apartment outliving all usage. But it is dead weight in the shipped product: the module is cfg-gated off on every OS the baseline built, no crate in the workspace depends on platform-windows, and the runtime has never been exercised anywhere (the header honestly says so, and the committed cross-compile check proves only typechecking). The remaining defects are hygiene: a doc contradiction with the seam's own header, a swallowed panic payload, a lossy default-dir encoding, and no filter-row validation.

## 2. What actually works (verified)
- ABI constants verified against the real Windows SDK values (auditor knowledge, shobjidl_core.h): CLSID_FileOpenDialog {DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7}, CLSID_FileSaveDialog {C0B4E2F3-BA21-4773-8DCA-3357EB2EB9B1}, IID_IFileOpenDialog {D57C7288-D4AD-4768-BE02-9D969532D960}, IID_IFileSaveDialog {84BCCD23-5FDE-4CDB-AEA4-AF64B83D78AB}, IID_IShellItem {43826D1E-E718-42EE-BC55-A1E261C37BFE} — all correct; ERROR_CANCELLED 0x800704C5 = HRESULT_FROM_WIN32(1225) correct; SIGDN_FILESYSPATH 0x80058000 correct; FOS_OVERWRITEPROMPT 0x2 / FORCEFILESYSTEM 0x40 / ALLOWMULTISELECT 0x200 / PATHMUSTEXIST 0x800 / FILEMUSTEXIST 0x1000 all correct.
- Vtable layouts: `FileDialogVtbl` = IUnknown(3) + IModalWindow::Show + the 23 IFileDialog slots in shobjidl_core.h order + IFileOpenDialog's GetResults/GetSelectedItems — slot-for-slot correct (counted: 3+1+23+2). `ShellItemVtbl` and `ShellItemArrayVtbl` orders also match.
- Reference discipline traced on every path: dialog released at 353-355 after configure_and_show; folder released at 424-426 on both success and SetDefaultFolder-failure; result item released at 503-505 before the path Result is mapped (506); array released at 522-524; per-item released at 581-583 before the `?` at 584; GetDisplayName's PWSTR freed at 548-550 after from_wide copies (545). No leak path found.
- `cargo check -p platform-windows --target x86_64-pc-windows-msvc` → EXIT=0 (`audit/runs/baseline/orchrun-check-platform-windows.log.txt`) — declarations typechecked. NOTE: `cargo check` does not link, so the `#[link(name = "shell32")]` extern resolution is still unverified.
- com.rs dependency contracts spot-checked: ComApartment::new uses COINIT_APARTMENTTHREADED (com.rs:124) — a genuine STA per round, matching this file's design claim; free_wide null-safe CoTaskMemFree (com.rs:355-359).

## 3. Findings

### F-001 [S3]
Where: crates/platform-windows/src/windows/dialogs.rs:261-264 (and the crate-wide fact)
Code:
```rust
/// The Common Item Dialogs adapter: stateless — every round runs on
/// its own short-lived STA thread.
#[derive(Debug, Clone, Default)]
pub struct CommonItemDialogs;
```
What is wrong: the entire crate has zero dependents. No workspace member's Cargo.toml references platform-windows (rg over all Cargo.tomls: only the crate's own and a comment in platform-linux's). apps/cli depends on 16 crates, none platform-*; apps/desktop-tauri depends on desktop/commands/tauri. The parallel seams (platform-linux/src/linux/xdg.rs, platform-macos/src/macos/dialogs.rs) are equally orphaned. This is finished, documented, tested-in-isolation platform code wired to nothing.
Why it matters: dead scaffolding in the shipped graph — built by `cargo build --workspace` (baseline #1, 3m46s includes it) and tested by the workspace suites, consuming build/test time while no shipped entry point can ever call it. Any claim that "Lumen has Windows file dialogs" is FALSE at the product level regardless of this code's quality.
What correct looks like: either the desktop shell (apps/desktop) gains a platform-crate dependency behind cfg, or the crate is marked as Phase-7+ not-yet-integrated scaffolding in the capability report it already ships.
Confidence: HIGH — Verification: `rg -g 'Cargo.toml' 'platform-'` in apps/* → zero matches; apps/cli + apps/desktop-tauri dep lists read directly.

### F-002 [S4]
Where: crates/platform-windows/src/dialog.rs:6-8 (contradicted by this file's design)
Code:
```rust
//! [`crate::windows::dialogs`]. The dialog is **modal on the calling
//! thread** (the OS API's own shape — the seam is a synchronous
//! request by design).
```
What is wrong: the seam's header claims the dialog is modal on the CALLING thread; this implementation runs the modal loop on a dedicated short-lived STA thread (dialogs.rs:1-12, spawn_sta at 300-319) while the caller blocks in `join()`. Functionally synchronous, structurally different — the two docs describe different designs.
Why it matters: the seam doc's stated reason (the OS API's own shape) is exactly what this implementation deliberately avoids (thread-model isolation); a maintainer reading dialog.rs will misunderstand the threading contract.
What correct looks like: dialog.rs should say "modal on a dedicated STA thread, joined synchronously — the caller's thread model never matters".
Confidence: HIGH — Verification: read both files.

### F-003 [S4]
Where: crates/platform-windows/src/windows/dialogs.rs:313-318
Code:
```rust
        .join()
        .map_err(|_| PlatformError::CallFailed {
            api: name,
            code: None,
            detail: "the dialog thread panicked".to_string(),
        })
```
What is wrong: the join-error arm discards the panic payload (`|_|`) — the panic's message and location are lost; the caller gets a fixed string.
Why it matters: a panicked dialog round (e.g. a COM helper unwrap) is undebuggable from the typed error; missing error context.
What correct looks like: downcast the payload (`Box<dyn Any>` → `&str`/`String`) into `detail`, falling back to the fixed string.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/platform-windows/src/windows/dialogs.rs:404-405
Code:
```rust
    if let Some(dir) = request.default_dir.as_ref() {
        let wide = to_wide(&dir.display().to_string())?;
```
What is wrong: `PathBuf::display().to_string()` lossily re-encodes a non-UTF-8 path (legal on Windows via OsStr) before conversion to wide chars; the correct conversion is OsStr→UTF-16 without a UTF-8 round-trip.
Why it matters: a default dir that is not valid UTF-8 is silently corrupted into U+FFFD soup; SHCreateItemFromParsingName then fails and the dialog silently falls back to the OS default (documented non-error at 435-436). Blast radius: a suggestion field only.
What correct looks like: `dir.as_os_str().encode_wide()`-style direct conversion (or a to_wide overload taking &OsStr).
Confidence: HIGH on the code fact / LOW practical impact — Verification: read.

### F-005 [S4]
Where: crates/platform-windows/src/windows/dialogs.rs:440-461
Code:
```rust
        let specs: Vec<Vec<u16>> = request
            .filters
            .iter()
            .map(|f| to_wide(&f.patterns.join(";")))
            .collect::<Result<_, _>>()?;
```
What is wrong: no validation of filter rows: an empty `patterns` vec joins to `""` and an empty label to `""`, both handed raw to `IFileDialog::SetFileTypes`. The OS API's behavior on an empty spec string is undocumented-at-best (broken filter row or E_INVALIDARG).
Why it matters: caller-error class input reaches an FFI boundary unvalidated; the seam is the natural place to refuse a malformed filter table typed.
What correct looks like: filter out or reject rows with empty label or empty patterns before the wide conversion.
Confidence: MEDIUM — Verification: read; OS behavior on empty spec UNVERIFIED (no Windows runtime available — matches the file's own [VERIFY] caveat).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module consts (CLSID/IID ×5, ERROR_CANCELLED, SIGDN, FOS ×5) | 43-92 | REAL | — | All values verified against the real SDK ABI (§2) |
| `SHCreateItemFromParsingName` extern decl | 99-107 | REAL | — | Signature matches shell32; linkage unverified (cargo check does not link) |
| `FilterSpec` (struct) | 122-128 | REAL | — | repr(C) COMDLG_FILTERSPEC — correct |
| `FileDialogVtbl` (struct) | 135-191 | REAL | — | Slot order verified against shobjidl_core.h; unused slots underscore-prefixed |
| `ShellItemVtbl` / `ShellItemArrayVtbl` (structs) | 195-249 | REAL | — | Orders verified |
| `ObjectHeader` (struct) | 252-255 | REAL | — | The vtable-access header |
| `CommonItemDialogs::new` | 269-271 | REAL | — | Stateless unit struct |
| `CommonItemDialogs::probe` | 280-297 | REAL | — | Create+release on its own STA round; honest typed failure; unused by anything (F-001) |
| `spawn_sta` | 302-319 | REAL | S4 | F-003 (panic payload dropped); correct join-based synchronous shape |
| `FileDialogs::open_file` / `save_file` (impl) | 322-337 | REAL | — | Clone-request-into-thread shape; correct |
| `run_dialog` | 340-357 | REAL | — | Apartment outlives dialog usage (drop order traced); single release on every path |
| `configure_and_show` | 361-526 | REAL | S4 | Options/title/folder/filters/show/collect — all HRESULTs checked, cancel first-class (475-477); F-004 (display()), F-005 (no filter validation) |
| `shell_item_path` | 529-552 | REAL | — | GetDisplayName + from_wide copy + exactly-one CoTaskMemFree — traced correct |
| `array_paths` | 555-596 | REAL | — | GetCount/GetItemAt loop; per-item release before `?`; skip-on-vanish (577) with honest empty-result error (586-594) |

## 5. Test honesty
No tests exist in or for this file (the module is cfg'd off on the test OS). The crate's host-runnable tests cover only the typed fallback seams (per windows/mod.rs:8-11 honesty header). Nothing anywhere executes this COM code — flagged as the file's central evidence gap, honestly self-declared (lines 20-22).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "compile-verified via `cargo check --target x86_64-pc-windows-msvc`" | module header 20-22 | REAL | `audit/runs/baseline/orchrun-check-platform-windows.log.txt`: CMD + EXIT=0 (typecheck only, no link) |
| "runtime NOT exercised in this sandbox" | module header 21-22 | REAL (honest) | No Windows runtime exists in any baseline run; consistent |
| "Every dialog round runs on a dedicated short-lived STA thread" | module header 8-9 | REAL | spawn_sta (302-319) + ComApartment STA (com.rs:124) — traced |
| "The dialog is modal on the calling thread" | dialog.rs:6-7 (seam doc) | FALSE | F-002: modal loop runs on the spawned STA thread; caller only joins |
| Windows file dialogs as a product capability | docs/feature-matrix.toml | NO ROW FOUND | `rg -i dialog docs/feature-matrix.toml` → only UI-dialog rows (GotoTime, Bookmarks) — no over-claim to falsify |

## 7. Dead / duplicated / unreachable code
- The whole file is unreachable from every shipped entry point (F-001) — by cfg on Linux, by zero dependents on Windows.
- No duplication within the file; the vtable declarations are necessarily verbose, not duplicated.

## 8. Dependencies & flags
- `#![allow(unsafe_code)]` at module scope (26) — the documented OS-service exception (doc 01 §2.7); acceptable given the SAFETY-comment density (every unsafe block carries one, all read as accurate).
- Depends on crate-internal com.rs helpers whose contracts were spot-checked (§2) — com.rs itself is another agent's assignment.
- No feature gates beyond the target_os cfg.

## 9. Open questions for the next agent
- Q-1 (com.rs auditor): confirm `to_wide` rejects interior NULs (a NUL in a title/default-dir would truncate the wide string at the OS boundary) — I read its signature only.
- Q-2 (product owner): is platform-windows slated for desktop-shell integration (Phase 7)? If not, F-001 upgrades from scaffolding-by-plan to permanent dead code.
- Q-3: SetFileTypes with an empty-pattern row (F-005) — needs a real-Windows run to classify; REQUEST-ORCH-RUN is not applicable (no Windows host exists in this audit infrastructure); keep as [VERIFY] with the file's own caveat.
