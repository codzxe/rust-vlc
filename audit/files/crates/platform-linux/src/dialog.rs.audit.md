# AUDIT: crates/platform-linux/src/dialog.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-23 |
| Lines / functions | 107 / 6 (FileFilter::new; FileDialogRequest; FilePick; FileDialogs:2; UnavailableDialogs:2 + impl:2) |
| File verdict | REAL (code) / UNREACHABLE (from shipped entry point) |
| Reachable from shipped entry point? | NO — orphan crate (see xdg.rs audit §7). Within the crate: `FileDialogs` implemented by `linux::xdg::XdgDialogs` (xdg.rs:202) and `UnavailableDialogs` here; wired by lib.rs::dialog_seam (203-231). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | prebuilt `linux_integrations-ca1702194c57d626` (13/13, dialog tests included) and `seam_behaviour` (15/15); `sed -n '203,231p' src/lib.rs` (reason construction); rg for UnavailableReason::NotYetImplemented producers |

## 1. One-paragraph verdict
A clean, small vocabulary file: filter row, OS-neutral request, a first-class Cancelled outcome, a 2-method trait, and a typed refusal fallback mirroring audio's. The code is correct and the Cancel-not-an-error design is right. The one real defect is a doc lie: the module header claims the platform report carries the portal gap as a typed `UnavailableReason::NotYetImplemented` row "say[ing] exactly this" — but `LinuxPlatform::discover` never produces that row: a backend-less host gets `ServiceUnavailable` → `Unavailable(ServiceNotReachable)` (the "[PROPOSAL]" text survives only inside the detail string), and `NotYetImplemented` is produced exclusively by `PlatformReport::get` for unprobed features, which discover() never leaves on Linux (it sets all four rows). The truth is still typed and honest — but it is a different typed reason than the one documented.

## 2. What actually works (verified)
- Typed fallback executed: `unavailable_dialogs_returns_the_typed_reason` + `typed_fallbacks_satisfy_the_seam_traits` (seam_behaviour, 15/15 run) drive `UnavailableDialogs` and assert the reason round-trips.
- `FilePick::Cancelled` mapping executed end-to-end through real zenity/kdialog stub exits (linux_integrations run: `xdg_dialogs_cancel_is_not_an_error ... ok`).

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/dialog.rs:10-13
Code:
```rust
//! violate the no-fabrication rule. The typed
//! [`crate::capability::UnavailableReason::NotYetImplemented`] row in
//! the platform report says exactly this; the zenity/kdialog chain
//! is the working path today.
```
What is wrong: The claim is false. On a host with no dialog backend, the report row is `Unavailable(ServiceNotReachable { service: "file dialogs" })`, produced by `Capability::from_probe_error` on a `ServiceUnavailable` error (lib.rs:210-227, verified by sed and by the executed `discover_assembles_this_hosts_report`, which accepts exactly LibraryNotFound|ServiceNotReachable). `UnavailableReason::NotYetImplemented` is emitted only by `PlatformReport::get` for un-probed features (capability.rs:185-194), and `discover()` probes all four rows — so the documented row state never occurs in the Linux report. Why it matters: a consumer matching on `NotYetImplemented` to detect the known portal gap (e.g. to show "install zenity") will never match; the gap text is buried in a detail string of a different variant. What correct looks like: either map the no-backend-with-unimplemented-portal case to `NotYetImplemented` in `from_probe_error`, or fix the doc to name ServiceNotReachable. Confidence: HIGH — Verification: read + executed test + sed of lib.rs:214-227.
F-002 [S4]
Where: crates/platform-linux/src/dialog.rs:57-60
Code:
```rust
    /// One or more chosen paths (one for save dialogs unless the
    /// user typed a name that already exists — overwrite
    /// confirmation is the dialog's own).
    Paths(Vec<PathBuf>),
```
What is wrong: The doc sentence does not parse into a behavior claim: "one for save dialogs unless the user typed a name that already exists — overwrite confirmation is the dialog's own" conflates the return cardinality with the tool's overwrite prompt. A save dialog returns exactly one path regardless of overwrite confirmation; the "unless" clause describes nothing the code distinguishes. Confidence: HIGH — Verification: read (xdg.rs save path returns `FilePick::Paths` with whatever stdout lines the tool printed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FileFilter::new | 28-37 | REAL | — | Copies label + patterns |
| FileDialogRequest (struct) | 40-50 | REAL | — | OS-neutral, Default; shape tested (`file_dialog_request_carries_the_os_neutral_shape`, windows suite) |
| FilePick (enum) | 53-61 | REAL | S4 | Garbled doc on Paths (F-002); Cancelled first-class — correct |
| FileDialogs::open_file / save_file | 64-77 | REAL | — | Contract: typed error only when every backend missing |
| UnavailableDialogs::because / reason | 85-97 | REAL | — | Same pattern as audio fallback |
| FileDialogs for UnavailableDialogs | 99-107 | REAL | — | Refusal returns the reason; executed by test |

## 5. Test honesty
No tests in this file. `unavailable_dialogs_returns_the_typed_reason` (executed) would fail if the fallback faked success. The request-shape test is in platform-windows' suite (also executed there, 16/16). Nothing here tests FileFilter label propagation end-to-end — and indeed the kdialog path drops labels (xdg.rs audit F-002), which no test catches: the seam vocabulary promises `label` is a filter row, one backend ignores it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The typed `UnavailableReason::NotYetImplemented` row in the platform report says exactly this" | dialog.rs:10-12 | FALSE | Report row is ServiceNotReachable (lib.rs:210-227 + executed discover test); NotYetImplemented only for unprobed rows (capability.rs:185-194), of which Linux discover() has none (F-001) |
| "zenity/kdialog chain is the working path today" | dialog.rs:12-13 | REAL | xdg.rs run_zenity/run_kdialog + executed stub tests |
| "Cancel is a first-class outcome, not an error" (via FilePick::Cancelled) | dialog.rs:55-56 | REAL | Executed: exit-1 stub → Cancelled |

## 7. Dead / duplicated / unreachable code
- Orphan crate: zero production reachability (xdg.rs audit §7).
- Near-verbatim mirror in platform-windows/src/dialog.rs and platform-macos/src/dialog.rs (documented [PROPOSAL]; cross-ref platform-windows lib.rs audit F-003).
- `UnavailableReason::NotYetImplemented` is effectively dead on the Linux assembled report (only `PlatformReport::get` on an unprobed row can produce it, and discover() probes all rows) — its only living use is the dishonest doc citation above.

## 8. Dependencies & flags
- std::path::PathBuf + crate::error only. No unsafe, no feature gates, no cfg in this file.

## 9. Open questions for the next agent
- Should `Capability::from_probe_error` surface the portal gap as NotYetImplemented (making the dialog.rs doc true), or should the doc be corrected? Current state: doc false, behavior defensible.
