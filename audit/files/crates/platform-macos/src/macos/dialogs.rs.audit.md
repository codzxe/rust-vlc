# AUDIT: crates/platform-macos/src/macos/dialogs.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 371 / 13 units (4 consts, 1 extern block, 8 fns, 2 impls) |
| File verdict | REAL-UNPROVEN (genuine hand-rolled AppKit FFI; NO committed compile evidence for the darwin targets the doc claims; runtime never executed; crate is an orphan) |
| Reachable from shipped entry point? | NO — platform-macos is an orphan crate: nothing depends on it (established by W01: audit/files/crates/platform-macos/Cargo.toml.audit.md — "the crate is an orphan: nothing in the repository depends on it"; re-confirmed for dialogs: `rg -l "AppKitPanels"` matches only this crate). On a macOS-target build it would be reachable via the crate's own `Platform::discover` → dialog seam. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `sed -n 1,371p` (full file, twice); `rg -n "^fn …"` (inventory); Read-verification of quotes (Bash display eats `[m` after `#`); `rg -n "AppKitPanels" crates/ apps/` (no external callers); baseline scan for darwin compile evidence (`ls audit/runs/baseline/` — no `*-apple-darwin` run exists); cross-cited audit/files/crates/platform-macos/Cargo.toml.audit.md (W01's verdict on the same evidence claim). |

## 1. One-paragraph verdict
Genuine, carefully-shaped AppKit FFI: a `pthread_main_np` main-thread guard with
a typed refusal, one autorelease pool per round, correct factory/selector/
BOOL/NSInteger msg_send shapes via the objc.rs alias table, cancel as a
first-class outcome, honest degradation for inexpressible filters and
unparsable directories, and single-release ownership on the one owned object
(the filter NSArray). But the module's own evidence claim — "compile-verified
via `cargo check --target aarch64-apple-darwin` and `x86_64-apple-darwin`" —
has **no committed run anywhere** (Wave 1 already established this for the
crate; it is equally true for this file), the code has **zero tests and zero
runtime evidence**, and the crate it lives in is **orphaned** — no shipped
entry point on any OS reaches it. Minor code nit: one error path hardcodes the
NSOpenPanel API name in save rounds.

## 2. What actually works (verified)
— Objective-C ABI shapes: `runModal` → i64 (NSModalResponse), `count` → u64, BOOL args as u8, `initWithObjects:count:` as (ptr, u64) — all match the pinned aliases in objc.rs:84-131 (read and cross-checked).
— Filter translation policy verified by reading: `*.ext` strip + reject empty/star-containing, empty set = unfiltered round (documented as the honest superset), lines 234-241.
— Ownership: the only owned object (NSArray from alloc+init) is released exactly once on the success path and the alloc is released on the init-failure path (lines 252-273). No leaks or double releases traced.
— Nothing executed: no macOS host in the audit sandbox; no baseline run touches a darwin target (audit/runs/baseline/ contains exactly one cross-target check: orchrun-check-platform-windows.log.txt).

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/src/macos/dialogs.rs:26-28
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.
```
What is wrong: an executed-evidence claim with no committed evidence. The
wave-0/2 baseline (audit/runs/baseline/, 26 runs + 6 orchestrator runs) contains
no `cargo check --target *-apple-darwin` log; the only cross-target check is
`orchrun-check-platform-windows.log.txt` (Windows). W01 reached the same
verdict for the crate manifest (audit/files/crates/platform-macos/Cargo.toml.audit.md,
F-003): "an unverifiable sentence … the project's macOS story rests on an
uncommitted run."
Why it matters: this file is 371 lines of hand-rolled unsafe FFI whose only
assurance that it even typechecks on its sole target platform is a doc comment
no run backs. Hard rule 8 forbids me from re-running it.
What correct looks like: an orchestrator-run `cargo check -p platform-macos
--target aarch64-apple-darwin` (+ x86_64) committed to audit/runs/baseline/ —
then this claim becomes REAL. Logged as REQUEST-ORCH-RUN in my log.
Confidence: HIGH — Verification: read + baseline directory scan (UNVERIFIED
that the check passes; VERIFIED that no evidence exists).

### F-002 [S4]
Where: crates/platform-macos/src/macos/dialogs.rs:130-135
Code:
```
    if response != NS_MODAL_RESPONSE_OK {
        return Err(PlatformError::CallFailed {
            api: "NSOpenPanel runModal",
            code: Some(response),
            detail: "the panel round ended with an unexpected response".to_string(),
        });
```
What is wrong: the unexpected-response error hardcodes `"NSOpenPanel runModal"`
even when the round was an `NSSavePanel` save (the main-thread guard 30 lines
above correctly branches on `save`; this site does not).
Why it matters: misleading error context — a save-panel failure would be
reported to the caller as an open-panel failure, sending any debugging down
the wrong lane.
What correct looks like: the same `if save { … } else { … }` literal the guard
uses, or a shared helper.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: whole file; evidence: `rg -n "AppKitPanels|dialogs::" crates/ apps/` → matches only within platform-macos itself
What is wrong: zero test coverage and zero runtime evidence for the AppKit
panel lane (the module doc admits the runtime half; the compile half is
F-001's problem). No `#[test]` exists in or referencing this file; the
workspace suite (4534/7/49) exercises none of it (cfg-compiled-out on the
Linux test host).
Why it matters: same class as wasapi.rs F-005 — the least-evidenced code is
the unsafe FFI surface where one wrong selector or constant silently
misbehaves.
What correct looks like: a macOS CI lane smoke (open+cancel+save through the
seam), or at minimum selector-constant unit tests against the AppKit headers.
Confidence: HIGH — Verification: rg (no references).

### F-004 [S4]
Where: crates/platform-macos/src/macos/dialogs.rs:166-217 (`configure_panel`) with dialog.rs:36-44 (`FileDialogRequest`)
What is wrong: not a code bug — a seam vocabulary gap: `FileDialogRequest`
carries title/filters/default_dir/multiple but **no suggested filename**, so
save panels can never pre-fill the name field (`setNameFieldStringValue:`).
NSSavePanel without it starts at an empty/last-used name.
Why it matters: a save dialog that cannot suggest a filename ("lumen-render.mp4")
is a real UX regression vs every platform dialog API; the seam as written
cannot express it, so the macOS lane is not at fault alone.
What correct looks like: a `default_name: Option<String>` on the request,
applied via `setNameFieldStringValue:` here (and the equivalent on other
lanes).
Confidence: HIGH — Verification: read of both files (no such field exists).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts NS_MODAL_RESPONSE_OK/CANCEL, YES/NO | 45-50 | REAL | — | values match AppKit (OK=1, Cancel=0, BOOL 8-bit) |
| extern `pthread_main_np` | 52-57 | REAL-UNPROVEN | — | `#[link(name = "System")]` correct for macOS; libSystem symbol |
| `is_main_thread` | 60-63 | REAL-UNPROVEN | — | trivial flag read |
| `AppKitPanels` + `new` | 73-79 | REAL | — | stateless unit |
| `AppKitPanels::probe` | 81-94 | REAL-UNPROVEN | — | class-existence probe, no UI; typed ServiceUnavailable when AppKit absent |
| `FileDialogs for AppKitPanels` | 96-105 | REAL-UNPROVEN | — | delegates to run_panel |
| `run_panel` | 107-138 | REAL-UNPROVEN | S4 | main-thread guard typed refusal (107-118, correct branch on save); pool per round; runModal i64; Cancel first-class; F-002 misnamed api on the unexpected path |
| `create_panel` | 142-164 | REAL-UNPROVEN | — | +openPanel/+savePanel; null → typed error; autoreleased (pool scope) — correct |
| `configure_panel` | 166-216 | REAL-UNPROVEN | — | title/dir/multi applied; open-panel BOOL trio; unparsable dir = silent default location (documented); F-004 vocabulary gap |
| `apply_filters` | 218-283 | REAL-UNPROVEN | — | `*.ext` translation with honest unfiltered fallback; NSArray alloc/init/set/release with correct single ownership; init-failure path releases the alloc |
| `collect` | 286-346 | REAL-UNPROVEN | — | URL vs URLs; per-item null skip; empty-after-count → typed error ("reporting no paths would be a lie") — honest |
| `url_path` | 349-371 | REAL-UNPROVEN | — | NSURL.path → UTF8String copy-out within pool scope; empty-path typed error |

## 5. Test honesty
No tests exist in or for this file. N/A — recorded as F-003.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | dialogs.rs:26-28 | REAL-UNPROVEN (no committed evidence) | F-001; audit/runs/baseline/ has no darwin run; W01's Cargo.toml audit reached the same verdict |
| "Cancel … is a first-class FilePick::Cancelled, never an error" | dialogs.rs:16-18 | REAL (as code) | run_panel:127-129 |
| "setAllowedFileTypes: … soft-deprecated … [PROPOSAL]: move to setAllowedContentTypes:" | dialogs.rs:19-24 | REAL (honest admission) | the deprecated API is what the code calls (apply_filters:271) |
| main-thread guard via pthread_main_np | dialogs.rs:6-9 | REAL (as code) | run_panel:107-118 + is_main_thread |

## 7. Dead / duplicated / unreachable code
- Whole file is unreachable from shipped entry points (orphan crate — W01
  established; re-confirmed). No internal dead code.

## 8. Dependencies & flags
- `#![allow(unsafe_code)]` module-scoped (documented SAFETY-SCOPED exception).
- Depends only on crate::dialog / crate::error / crate::macos::objc — no
  external crates (clean-room: no objc2). No feature flags; gating is
  `target_os = "macos"` at the `macos` module level.

## 9. Open questions for the next agent
- If the orchestrator runs the darwin check (my REQUEST-ORCH-RUN), does
  dialogs.rs compile clean? The `#[link(name = "System")]` extern and the
  objc alias table are the two spots most likely to break cross-target.
- Owner of the seam vocabulary: add `default_name` (F-004) — the Linux/Windows
  lanes have the same gap by construction.
