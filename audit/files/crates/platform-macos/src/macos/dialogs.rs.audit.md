# AUDIT: crates/platform-macos/src/macos/dialogs.rs
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-23 |
| Lines / functions | 371 / 8 (is_main_thread, new, probe, open_file, save_file, run_panel, create_panel, configure_panel, apply_filters, collect, url_path) |
| File verdict | REAL-UNPROVEN (genuine AppKit FFI, parse-verified here; never compiled in this environment, never executed anywhere per its own header) |
| Reachable from shipped entry point? | NO — `platform-macos` is an orphan crate (no dependents) AND cfg(target_os="macos")-gated off on every lane this sandbox can build |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | rustc parse-check (exit: parse OK, 3 expected unresolved-import errors); rg manifest sweep; rustup target list |

## 1. One-paragraph verdict
This is real integration code, not a stub: a hand-rolled NSOpenPanel/NSSavePanel
driver over the libobjc C ABI with a main-thread guard, a per-round autorelease
pool, typed errors on every failure path, and per-signature `objc_msgSend`
aliases. The unsafe blocks carry specific SAFETY contracts, and the
ownership discipline (alloc/init/release, pool scoping) is coherent across the
11 functions. But none of it is reachable: `platform-macos` has zero dependents
in the workspace, the module is cfg-gated to macOS, this sandbox has no Apple
target installed, and the file's own header admits runtime was never exercised.
The doc comment on `apply_filters` states a fallback behavior the code does not
implement, and two error constructors name the wrong API in half their paths.

## 2. What actually works (verified)
— `rustc --edition 2024 --crate-type lib macos/dialogs.rs` -> only E0432/E0433
unresolved-crate-root errors (expected standalone): the file parses clean; the
`--cfg target_os` flag was rejected by this rustc, so this is a parse check,
not a typecheck.
— Manual signature cross-check of all 16 call sites against the aliases in
macos/objc.rs:76-132 (msg_send_i64/u64/void_id/void_bool/id/id_id/id_u64/
id_ptr_u64/id_cstr): every call matches a declared alias shape. NSModalResponse
constants (OK=1 line 45, CANCEL=0 line 47) match AppKit.
— Nothing executed. No test in the repo drives AppKitPanels (checked
crates/platform-macos/tests/ — only the host-runnable seam suite).

## 3. Findings
F-001 [S2]
Where: crates/platform-macos/src/macos/dialogs.rs:1-371 (file's crate)
Code:
```
#[cfg(target_os = "macos")]
pub mod dialogs;
```
(macOS/mod.rs:17-18)
What is wrong: `platform-macos` is an orphan crate. `rg "platform-macos" --glob
Cargo.toml` over the whole tree matches only its own manifest; neither
apps/cli nor apps/desktop lists it. No shipped entry point (lumen-cli, desktop)
can reach this code on any OS. Why it matters: 371 lines of unsafe FFI that no
user can trigger; the "L5 registry row" exists only on paper. What correct
looks like: a host-dispatch crate selecting platform-macos on darwin targets,
or removal until a consumer card lands. Confidence: HIGH — Verification: ran
(rg sweep). Same defect as platform-linux (W08-A07 F-001) — three orphan
"platform" crates suggests the wiring card never happened.
F-002 [S3]
Where: crates/platform-macos/src/macos/dialogs.rs:212-233
Code:
```
/// Translate the request's `*.ext` patterns into
/// `setAllowedFileTypes:` (an NSArray of extension strings). A
/// pattern without a simple trailing extension (e.g. `*` or `a*b`)
/// disables filtering for the round — the honest reading of "any
/// file"; the label column is the UI's own concern (AppKit shows
/// extensions only).
```
What is wrong: the doc is FALSE about the code beneath it. The code
(`filter_map(|pattern| pattern.strip_prefix("*."))` + `filter(...)`) drops only
the individual inexpressible pattern; if any other filter row carries an
expressible `*.ext`, the panel IS filtered. Only when ALL patterns are
inexpressible (`extensions.is_empty()`) does the round run unfiltered. A
request with filters `[{*.mkv}, {*}]` filters to .mkv — the doc says the `*`
"disables filtering for the round". Why it matters: the next maintainer
implements the documented semantics and silently changes behavior. What correct
looks like: fix the comment to "an inexpressible pattern is ignored; if no
pattern is expressible the round runs unfiltered". Confidence: HIGH —
Verification: read (pure function, deterministic).
F-003 [S3]
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
What is wrong: `save_file` reaches this branch too, but the api string is
hardcoded "NSOpenPanel runModal". The main-thread guard 8 lines above (110-114)
selects the panel name from `save` — the inconsistency is a copy-paste slip.
The error misattributes the failing API in every save-panel failure. Why it
matters: error context is this crate's core promise ("typed truth"); a wrong
api name is a lie in the data. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/platform-macos/src/macos/dialogs.rs:296-301
Code:
```
            return Err(PlatformError::CallFailed {
                api: "NSSavePanel URL",
```
What is wrong: single-pick OPEN rounds (`save || !request.multiple` with
save=false) also report api "NSSavePanel URL". Same misattribution class as
F-003. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/platform-macos/src/macos/dialogs.rs:223-224
Code:
```
        .filter_map(|pattern| pattern.strip_prefix("*."))
        .filter(|ext| !ext.is_empty() && !ext.contains('*'))
```
What is wrong: the pattern `*.*` yields extension `"."` (non-empty, no `*`) and
is passed to `setAllowedFileTypes:` — a nonsense extension the panel will match
against files ending in `.`. Edge case, wrong behavior on a valid-ish input.
Confidence: HIGH — Verification: read (pure string logic).
F-006 [S4 — claim]
Where: crates/platform-macos/src/macos/dialogs.rs:26-28
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.
```
What is wrong: not reproducible here — `rustup target list --installed` shows
only wasm32-unknown-unknown and x86_64-unknown-linux-gnu; the cross-check
cannot be re-run, so "compile-verified" is UNVERIFIED (claimed by the file,
not demonstrated in-tree; no CI lane visible for darwin targets). The runtime
gap is honestly disclosed. Confidence: HIGH that the claim is unverifiable —
Verification: ran (rustup target list).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| is_main_thread | 60-64 | REAL-UNPROVEN | — | pthread_main_np FFI; sound shape; never executed |
| AppKitPanels::new | 78-80 | REAL | — | trivial |
| AppKitPanels::probe | 89-93 | REAL-UNPROVEN | — | class lookup probe; honest Err on absent class |
| FileDialogs::open_file | 97-99 | REAL-UNPROVEN | — | delegates run_panel(false) |
| FileDialogs::save_file | 101-103 | REAL-UNPROVEN | — | delegates run_panel(true) |
| run_panel | 107-138 | REAL-UNPROVEN | S3 | guard ok; F-003 wrong api name in one branch |
| create_panel | 142-161 | REAL-UNPROVEN | — | factory + null check, typed error |
| configure_panel | 166-210 | REAL-UNPROVEN | — | applicable-setters discipline; multi-select gated to open |
| apply_filters | 218-282 | REAL-UNPROVEN | S3/S4 | F-002 doc false; F-005 `*.*` → "." |
| collect | 286-346 | REAL-UNPROVEN | S4 | F-004 api name; null-item skip + empty-selection error coherent |
| url_path | 349-371 | REAL-UNPROVEN | — | null/empty checks, typed errors |

## 5. Test honesty
No tests exist for this file (host tests cover only the cfg-independent seam
modules). Nothing to grade — the file's own header admits it ([VERIFY]).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | header 26-28 | UNVERIFIED | no apple target in sandbox (rustup target list) |
| "runtime NOT exercised … [VERIFY]" | header 28 | TRUE (honest disclosure) | no test drives AppKitPanels (rg over tests/) |
| inexpressible pattern "disables filtering for the round" | 212-217 doc | FALSE | code drops one pattern only (F-002) |
| "Cancel … is a first-class FilePick::Cancelled, never an error" | header 15-18 | REAL-UNPROVEN | code at 127-129 matches; never executed |

## 7. Dead / duplicated / unreachable code
Whole file is unreachable from shipped entry points (F-001). No internal dead
code found. Structure parallels platform-linux's dialogs (by design, mirrored
vocabulary).

## 8. Dependencies & flags
`#![allow(unsafe_code)]` module-level (line 32) — the audited OS-service
exception per Cargo.toml doc; justified. Links `System` (pthread_main_np) and
via objc.rs `libobjc`. No external crates (std-only crate). cfg-gated behind
target_os="macos" (mod.rs:17-18).

## 9. Open questions for the next agent
- Did any CI lane ever run the claimed `cargo check --target *-apple-darwin`?
  Check .github/workflows for darwin runners — if absent, the header's
  "compile-verified" is a local-only claim with no durable evidence.
- macos/coreaudio.rs and macos/metal.rs share this file's pattern — audit for
  the same wrong-api-name copy-paste class (F-003/F-004).
