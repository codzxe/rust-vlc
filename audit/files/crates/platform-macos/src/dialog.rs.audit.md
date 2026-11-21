# AUDIT: crates/platform-macos/src/dialog.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 108 / 9 units (2 structs + ctor, 1 enum, 1 trait + impl, 1 fallback struct + 2 methods + impl) |
| File verdict | REAL (the seam vocabulary + typed fallback compile on every OS and their tests executed green in the baseline; the macOS implementation behind the seam is REAL-UNPROVEN — see dialogs.rs audit) |
| Reachable from shipped entry point? | The seam+fallback: executed in baseline (platform-macos tests/seam_behaviour.rs, 16 passed). The crate as a whole remains an orphan (W01: audit/files/crates/platform-macos/Cargo.toml.audit.md) — no shipped entry point reaches platform-macos on any OS. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cat` (full file, twice); `rg -n "fn \|struct \|trait"` (inventory); `rg -n "FileDialogRequest\|FileFilter\|UnavailableDialogs\|FilePick" crates/platform-macos/tests/seam_behaviour.rs` (test coverage); baseline verification: `rg -n "seam_behaviour\|platform_macos" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 16 tests ok incl. `unavailable_dialogs_returns_the_typed_reason`, `file_dialog_request_carries_the_os_neutral_shape`; Read-verification of `#[must_use]` sites. |

## 1. One-paragraph verdict
The dialog seam vocabulary done right: an OS-neutral request type, a
two-variant outcome where Cancel is not an error, a `Send`-bound trait, and a
typed fallback whose every method returns the exact reason it was constructed
with — never a fake success. The doc header states the macOS lane's model
(main-thread modal AppKit) and its evidence status (compile-verified claim —
which, as with its siblings, has no committed darwin run). The fallback half
of this file has executed, green, mutation-sensitive tests; the vocabulary
half has one genuine gap (no suggested-filename field), which I filed against
this file as the vocabulary owner and cross-referenced from my dialogs.rs
audit (F-004 there).

## 2. What actually works (verified)
— Fallback honesty executed: baseline orchrun-test-workspace-nofailfast.log.txt:3690-3712 — platform-macos tests/seam_behaviour.rs `running 16 tests … test result: ok. 16 passed`, including `unavailable_dialogs_returns_the_typed_reason` (asserts open+save both return the exact typed reason) and `typed_fallbacks_satisfy_the_seam_traits` (the `dyn FileDialogs` wiring shape `MacOsPlatform::discover` uses).
— Request-shape test executed: `file_dialog_request_carries_the_os_neutral_shape ... ok` (same run).
— `#[must_use]` discipline on `because`/`reason` (verified via Read; Bash display mangles the attribute).

## 3. Findings

### F-001 [S4]
Where: crates/platform-macos/src/dialog.rs:42-50
Code:
```
pub struct FileDialogRequest {
    /// Window title (None = OS default).
    pub title: Option<String>,
    /// Type filters (empty = all files).
    pub filters: Vec<FileFilter>,
    /// Suggested default location.
    pub default_dir: Option<PathBuf>,
    /// Allow picking several files at once (open dialogs only).
    pub multiple: bool,
}
```
What is wrong: the OS-neutral request cannot express a **suggested filename**
for save dialogs — no `default_name` field. Every platform's save dialog
(NSSavePanel `setNameFieldStringValue:`, Windows IFileDialog
`SetFileName`, GTK `set_current_name`) supports it; the seam drops the
concept, so no lane can implement it.
Why it matters: a save dialog that cannot pre-fill "lumen-render.mp4" is a
daily-use UX defect, and the vocabulary gap silently constrains all three
platform lanes identically (the Linux zenity lane and the Windows lane have
the same hole by construction — their auditors should confirm).
What correct looks like: `pub default_name: Option<String>` here, applied by
each lane (`setNameFieldStringValue:` in macos/dialogs.rs `configure_panel`).
Cross-reference: filed as F-004 in my dialogs.rs audit — same defect, that
instance against the macOS consumer, this one against the vocabulary owner.
Confidence: HIGH — Verification: read (field list complete; no default_name
anywhere in the seam).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct FileFilter` + `new` | 22-39 | REAL | — | label + patterns; pure construction; `#[must_use]` |
| `struct FileDialogRequest` | 42-50 | REAL | S4 | F-001 vocabulary gap |
| `enum FilePick` | 55-62 | REAL | — | Cancelled first-class; Paths(Vec<PathBuf>) — distinct-from-error vocabulary (asserted by test) |
| `trait FileDialogs` | 65-79 | REAL | — | Send bound; typed #Errors docs |
| `UnavailableDialogs` + `because`/`reason` | 82-97 | REAL | — | typed fallback; `#[must_use]` on both |
| `impl FileDialogs for UnavailableDialogs` | 100-107 | REAL | — | both methods return the exact reason clone — never a fake success; executed green in baseline |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `unavailable_dialogs_returns_the_typed_reason` (seam_behaviour.rs:61-74) | YES | exact-equality asserts on open+save errors against the constructed reason — breaks if the fallback ever fakes success or mutates the reason |
| `typed_fallbacks_satisfy_the_seam_traits` (:80-98) | YES | dyn-trait wiring + distinctness asserts (Cancelled ≠ Paths) |
| `file_dialog_request_carries_the_os_neutral_shape` (:101 log line) | YES | pins the request's field shape (would catch a vocabulary drift; would NOT catch the *missing* default_name — the gap is invisible to tests, which is how F-001 survived) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The dialog is **modal on the main thread** … enforced main-thread guard (pthread_main_np)" | dialog.rs:5-9 | REAL (as code, macOS lane) | dialogs.rs:107-118 guard; runtime unproven (no macOS host) |
| "Compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | dialog.rs:11-13 | REAL-UNPROVEN | no committed darwin run in audit/runs/baseline/ (W01 crate audit F-003; my dialogs.rs F-001) |
| "On any other OS the seam is the typed UnavailableDialogs fallback" | dialog.rs:9-11 | REAL | executed: 16/16 seam_behaviour tests green in baseline on Linux |

## 7. Dead / duplicated / unreachable code
- The file itself is reachable only inside the orphan platform-macos crate
  (W01-established). Internally nothing is dead: `FileDialogs` is object-safe
  and used as `dyn` by the discover wiring + tests.

## 8. Dependencies & flags
- std + crate::error only. No unsafe (this file is pure vocabulary). Mirrors
  the platform-windows seam shape (cross-OS vocabulary symmetry is deliberate
  per the crate docs).

## 9. Open questions for the next agent
- platform-linux / platform-windows dialog auditors: confirm the same
  default_name gap (F-001) in your lanes — it is a seam-level defect, so a
  fix must land here first.
- If the seam grows, keep `FilePick::Paths` non-empty as an invariant
  (macOS collect errors on empty-paths-with-OK; a lane that returns
  Paths(vec![]) would violate the "never lie about what the OS said" rule).
