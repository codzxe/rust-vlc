# AUDIT: crates/platform-linux/src/dialog.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 7 items (FileFilter + new, FileDialogRequest, FilePick, FileDialogs trait, UnavailableDialogs + because/reason + trait impl) |
| File verdict | REAL (seam + fallback; the working zenity/kdialog implementation lives in linux/xdg.rs) |
| Reachable from shipped entry point? | NO — crate-level orphan (no manifest depends on platform-linux; filed once at dl.rs audit F-001, this wave). Within the crate: xdg.rs implements this trait; baseline-executed tests: `xdg_dialogs_kdialog_fallback_parses_output`, `xdg_dialogs_cancel_is_not_an_error`, `xdg_dialogs_run_the_zenity_fallback`, `xdg_dialogs_without_any_backend_refuse_with_the_typed_error`, `xdg_discovery_uses_the_real_environment`, `xdg_portal_probe_reports_bus_truth` — all ok (orchrun log, linux_integrations 13/13). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `FileDialogRequest\|FilePick\|FileFilter` in linux/xdg.rs (the real consumer, lines 26/190-207) · rg `UnavailableDialogs` (used by lib.rs seam assembly) · sed -n quotes verified · baseline orchrun log (xdg tests ok) |

## 1. One-paragraph verdict
The smallest seam in the batch and an honest one: the module doc states
plainly that the xdg-desktop-portal D-Bus client is unbuilt `[PROPOSAL]` work
*because shipping unexercised protocol code would violate the
no-fabrication rule*, and that zenity/kdialog is the real working path — which
the baseline's six passing xdg tests corroborate (including the
no-backend-refuses-typed case and cancel-is-not-an-error). The types are
OS-neutral, the fallback refuses both methods with its constructed reason,
and I found nothing to file.

## 2. What was checked, and how (zero-findings file — full list)
— Fallback totality (99-107): both trait methods return the constructed
reason; `_request` unused by design (refusal is the contract). Test-pinned
(`xdg_dialogs_without_any_backend_refuse_with_the_typed_error`, baseline ok).
— Type surface: `FileFilter::new` allocates copies (cold path);
`FileDialogRequest: Default` (all-optional fields — coherent); `FilePick`
models Cancelled as a first-class non-error (asserted by
`xdg_dialogs_cancel_is_not_an_error`, baseline ok).
— Consumer wiring: linux/xdg.rs:26 imports all three types and implements
`FileDialogs` (open_file/save_file at 203-207) — the seam is not vacuous.
— ARCH-6: no OS types in any signature (PathBuf/String/bool only) ✓.
— Doc claims: the portal-is-[PROPOSAL] statement and the
`UnavailableReason::NotYetImplemented` report row are cross-checked in the
capability module's own test (`xdg_portal_probe_reports_bus_truth` ok) — the
report says what the doc says the report says.
— No panics/unwrap/unsafe/features/dead code (read line-by-line).

## 3. Findings
None in this file. (Crate-level orphan S3 filed once at dl.rs audit F-001,
this wave; not re-counted here.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-13 | REAL | — | honest [PROPOSAL] scoping with an explicit no-fabrication rationale |
| struct FileFilter + new | 19-37 | REAL | — | label + glob patterns; #[must_use] |
| struct FileDialogRequest | 39-50 | REAL | — | OS-neutral; Default derive coherent |
| enum FilePick | 52-61 | REAL | — | Cancelled ≠ error; Paths for both dialog kinds |
| trait FileDialogs | 63-77 | REAL | — | two modal methods, typed errors documented |
| UnavailableDialogs (+because/reason) | 79-97 | REAL | — | mirrors the audio fallback; #[must_use] |
| impl FileDialogs for UnavailableDialogs | 99-107 | REAL | — | total typed refusal; test-pinned |

## 5. Test honesty
No in-file tests. Baseline-executed (linux_integrations, all ok):
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| xdg_dialogs_kdialog_fallback_parses_output | YES | real output parsing against the FilePick contract |
| xdg_dialogs_cancel_is_not_an_error | YES | pins the Cancelled-not-Err discipline |
| xdg_dialogs_run_the_zenity_fallback | YES | the primary working path |
| xdg_dialogs_without_any_backend_refuse_with_the_typed_error | YES | the fallback arm of this file's trait |
| xdg_discovery_uses_the_real_environment / xdg_portal_probe_reports_bus_truth | YES | the probe-vs-fallback selection and the honest NotYetImplemented report row |
(The implementations under test are xdg.rs's — that file's auditor owns their
internals; this file's contribution is the contract, which the tests exercise
end to end.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the zenity/kdialog chain is the working path today" | dialog.rs:12-13 | REAL | 4 passing xdg_dialogs_* tests in baseline |
| "the portal's D-Bus FileChooser client itself is [PROPOSAL] follow-up work … shipping unexercised protocol code would violate the no-fabrication rule" | dialog.rs:5-10 | REAL | no portal client code exists (rg); the report row says NotYetImplemented (test-pinned) |
| "Cancel is a first-class outcome, not an error" (via FilePick::Cancelled doc) | dialog.rs:55-56 | REAL | `xdg_dialogs_cancel_is_not_an_error` ok |

## 7. Dead / duplicated / unreachable code
None in-file. Crate-level orphan: dl.rs audit F-001 (this wave). The
`UnavailableDialogs` mirror of `UnavailableAudioDevices` is the documented
cross-crate mirroring doctrine (lib.rs headers, platform-windows audit F-002
notes the same pattern).

## 8. Dependencies & flags
Crate-internal error only; std types. No external deps, no features, no
unsafe. Compiles in the default Linux workspace lanes.

## 9. Open questions for the next agent
- xdg.rs auditor: the zenity/kdialog subprocess path — check timeout/zombie
  handling and locale-sensitive output parsing (the tests pin the happy
  shapes; hostile environments are that file's scope).
- If the portal client is ever built, this seam's types need no change —
  good stability property; keep FileFilter's glob semantics documented when
  the portal's D-Bus filter structs arrive.
