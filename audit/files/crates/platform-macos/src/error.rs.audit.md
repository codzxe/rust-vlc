# AUDIT: crates/platform-macos/src/error.rs
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-23 |
| Lines / functions | 114 / 2 impls (Display, Error) over an 8-variant enum |
| File verdict | REAL (all 8 Display forms pinned and executed by the macos host-runnable suite) — but a byte-identical triplicate inside an orphan crate |
| Reachable from shipped entry point? | NO — `platform-macos` has zero workspace dependents (rg over all Cargo.tomls) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `cargo test -p platform-macos --test seam_behaviour` (exit 0, 16 passed); diff vs linux/windows copies; rg variant-construction sweep |

## 1. One-paragraph verdict
A correct, fully typed error vocabulary: 8 variants covering wrong-OS, missing
library, missing symbol, unreachable service, failed call, timeout, bad
encoding, failed spawn; Display renders every field; std::error::Error is
implemented; no bare strings. Every Display form is pinned by
`platform_errors_render_their_details` and the suite passes on this host
(16/16). It is nonetheless the second byte-identical triplicate audited in this
assignment (error.rs x3 differs only in the card-number comment line), two of
its variants are never constructed anywhere in this crate, and the crate is
unreachable from any shipped entry point.

## 2. What actually works (verified)
— `cargo test -p platform-macos --test seam_behaviour` -> 16 passed, exit 0;
the errors test asserts all 8 Display strings (NotCompiledForThisOs,
LibraryUnavailable, SymbolMissing, ServiceUnavailable, CallFailed with
Some/None code, TimedOut, SpawnFailed) via contains().
— `diff platform-linux/src/error.rs platform-macos/src/error.rs` -> only line
1 differs (card 7.A2 vs 7.A3); windows copy likewise (7.A1).

## 3. Findings
F-001 [S2]
Where: crates/platform-macos/src/error.rs:1-114 (crate context)
Code:
```
//! The typed platform-error vocabulary (card 7.A3).
```
What is wrong: fourth consecutive orphan-crate file in this assignment:
`platform-macos` is depended on by nothing (rg sweep over every Cargo.toml in
the tree). The error vocabulary is defined for callers that do not exist.
Confidence: HIGH — Verification: ran (rg).
F-002 [S3]
Where: crates/platform-macos/src/error.rs:15-74 (whole enum)
Code:
```
pub enum PlatformError {
    /// This build does not contain the OS integration (e.g. the Linux
    /// adapter running on a non-Linux host).
    NotCompiledForThisOs {
```
What is wrong: byte-identical triplication of the entire 114-line vocabulary
across platform-linux/-macos/-windows (diff: only line 1 differs), same as
capability.rs (W08-A07 file 4 F-002). Documented as deliberate mirroring
pending a Lead-unified crate, but three copies of an error contract that can
drift independently. Confidence: HIGH — Verification: ran (diff x2).
F-003 [S4]
Where: crates/platform-macos/src/error.rs:29-36 and 67-73
Code:
```
    SymbolMissing {
        /// The library the symbol was expected in.
        library: String,
```
What is wrong: `SymbolMissing` and `SpawnFailed` are never constructed
anywhere in platform-macos (rg over src/: matches only in this file's
definition/Display and capability.rs's match arm). The macos integrations
report missing classes via ServiceUnavailable (objc.rs:145-148) and spawn
nothing. Dead vocabulary in this crate, justified only by the mirroring rule.
Confidence: HIGH — Verification: ran (rg construction sweep).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlatformError (8 variants) | 15-74 | REAL | S4 | 2 of 8 variants never constructed in this crate (F-003) |
| Display::fmt | 76-112 | REAL | — | all 8 forms pinned by passing test |
| impl std::error::Error | 114 | REAL | — | empty impl, correct (Display+Debug derived) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| platform_errors_render_their_details (tests/seam_behaviour.rs) | YES | 8 contains() assertions on exact substrings incl. dynamic fields (names.join, code Some/None, deadline value) |
Suite executed 16/16 on this host — see §2.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every failure a platform seam can produce is one of these variants — never a bare string, never a fake success" | header 3-5 | REAL for macos FFI files audited (dialogs.rs returns only these variants) | read dialogs.rs + coreaudio/metal call sites (45 PlatformError constructions, all in-vocabulary) |
| "The same shape is mirrored in platform-windows/platform-macos" (linux copy) | linux error.rs:5-8 | REAL | diff-verified byte-identical modulo line 1 |

## 7. Dead / duplicated / unreachable code
- F-002: the triplication itself.
- F-003: SymbolMissing, SpawnFailed unconstructed in this crate.
- Crate-level: unreachable from shipped entry points (F-001).

## 8. Dependencies & flags
std-only (use std::fmt). No unsafe, no cfg, no features. Compiles on Linux
(proven — the crate's host tests build and pass here).

## 9. Open questions for the next agent
- If the Lead unification crate lands, these three error.rs files should be
  deleted, not wrapped — check nobody grows crate-specific variants in the
  meantime (a cross-crate identity test would freeze the mirror, same
  recommendation as file 4).
