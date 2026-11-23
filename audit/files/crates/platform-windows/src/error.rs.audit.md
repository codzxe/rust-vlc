# AUDIT: crates/platform-windows/src/error.rs
| Field | Value |
|---|---|
| Auditor | W08-A04 |
| Date | 2026-11-23 |
| Lines / functions | 114 / 1 enum (9 variants) + Display + Error impl |
| File verdict | REAL code, UNREACHABLE in product — a clean typed-error vocabulary with no shipped consumer, triplicated across the platform crates |
| Reachable from shipped entry point? | NO. No app depends on platform-windows (rg over apps/*/Cargo.toml: zero). The crate's only consumers are its own tests; the WASAPI FFI behind it is compiled only on Windows targets and this audit host is Linux (UNVERIFIED on a real Windows build — same class as the baseline's desktop-tauri gdk-sys limit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p platform-windows` → 16 seam_behaviour pass on the Linux fallback lane (exit 0); `diff` windows↔linux↔macos error.rs; rg app deps + variant mints; sed quote verification |

## 1. One-paragraph verdict
The vocabulary itself is exactly what its docs promise: nine typed
variants, every field documented, Display renders every arm, no bare
strings, no fake success — and the crate's tests render each variant
and pass (16/16, run on this host via the not-compiled fallback lane).
Two defects: the module doc is a verbatim copy-paste that names the
wrong crates (it says the shape is mirrored in "platform-windows/
platform-macos" from inside platform-windows itself, omitting linux),
and its own crate's WASAPI adapter misuses the `SpawnFailed` variant
for std::thread spawn failures, rendering a thread-resource failure as
"could not execute lumen-wasapi-enum thread" — a program-execution
diagnosis that is wrong. And like the whole platform lane, nothing
shipped ever sees one of these errors.

## 2. What actually works (verified)
— verified by `cargo test -p platform-windows` (exit 0):
`platform_errors_render_their_details` (seam_behaviour.rs:354)
renders NotCompiledForThisOs / CallFailed / InvalidEncoding /
SpawnFailed arms; 15 more tests pass over the fallback seams. `diff`
confirms the three platform vocabularies are byte-identical except the
card id (7.A1 / 7.A2 / 7.A3) — the [PROPOSAL] unification claim is
accurate about the current state.

## 3. Findings
F-001 [S3]
Where: crates/platform-windows/src/error.rs:1-8 (whole file's product
role).
Code:
```
//! Every failure a platform seam can produce is one of these variants
//! — never a bare string, never a fake success (MP §8: "errors are
//! data, and the error path is a first-class outcome"). The same
```
What is wrong: no shipped binary can ever produce, catch, or display
one of these errors — no app depends on platform-windows (or
platform-linux; macos is the same by symmetry). The vocabulary is a
first-class outcome for a process that does not exist. Same unwired
lane as platform-linux/src/events.rs F-001 (the platform seams →
engine poll_hotplug lane has no consumer anywhere). Confidence: HIGH —
Verification: rg over apps/*/Cargo.toml (zero matches).
F-002 [S4]
Where: crates/platform-windows/src/error.rs:1-8.
Code:
```
//! shape is mirrored in `platform-windows`/`platform-macos` so the
//! Lead can later unify them into a shared L5 vocabulary crate
```
What is wrong: this IS platform-windows — the sentence names the file
itself and macos, omitting platform-linux, from which it was copied.
`diff` against platform-linux/src/error.rs shows the files are
byte-identical except `card 7.A1` vs `card 7.A2` — the doc was never
updated for its new home. Confidence: HIGH — Verification: diff (one
line differs besides the card id).
F-003 [S4]
Where: crates/platform-windows/src/error.rs:67-73 vs
crates/platform-windows/src/windows/wasapi.rs:609-613, 637-641.
Code:
```
    /// A helper program the seam shells out to could not be executed.
    SpawnFailed {
        /// The program that was attempted.
        program: String,
```
vs the only in-crate mints (wasapi.rs):
```
            .spawn(list_endpoints)
            .map_err(|err| PlatformError::SpawnFailed {
                // lumen-allow-literal: same diagnostic name, surfaced in the typed error.
                program: "lumen-wasapi-enum thread".to_string(),
```
What is wrong: `SpawnFailed`'s contract is "a helper program the seam
shells out to"; the WASAPI adapter uses it for a std::thread::Builder
spawn failure (COM apartment threads). A thread-spawn failure means
resource exhaustion (threads/memory), not a missing executable — the
rendered message "could not execute lumen-wasapi-enum thread: …"
misdiagnoses the class of failure in a vocabulary whose entire thesis
is that errors are machine-checkable truth. The vocabulary lacks a
ThreadSpawnFailed variant, so the caller shoehorned. Confidence: HIGH —
Verification: read + rg (SpawnFailed mints: only wasapi.rs:609/637).
F-004 [S4]
Where: crates/platform-windows/src/error.rs (whole enum), plus rg
evidence.
Code:
```
    /// The OS returned a string that is not valid UTF-8/UTF-16.
    InvalidEncoding {
```
What is wrong: `InvalidEncoding` is never minted in this crate (rg:
zero constructors outside error.rs itself; only platform-macos's
coreaudio.rs/objc.rs mint it). The WASAPI wide-string decode path
(wasapi.rs:417-427 `id_of`/`from_wide`) degrades decode failure to
`None` → `AudioSetChanged` advice instead of the typed error — a
defensible callback-context choice, but it leaves a documented variant
dead in the crate that documents it. Triple duplication of the
vocabulary (windows/linux/macos byte-identical modulo card id) is the
recorded [PROPOSAL] — drift risk already demonstrated by F-002.
Confidence: HIGH — Verification: rg mints + diff.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlatformError (enum, 9 variants) | 14-74 | REAL | F-001, F-004 | fully documented fields; InvalidEncoding dead in-crate |
| Display::fmt | 76-112 | REAL | — | every arm renders; tested |
| Error impl | 114 | REAL | — | no source() chain (none exists) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| platform_errors_render_their_details (seam_behaviour.rs:354-388) | YES | asserts rendered content per variant |
| capability_classifies_and_converts_probe_errors (115-139) | YES | variant → Capability mapping pinned |
No tests inside error.rs itself; the crate-level suite covers it. All
16 tests pass on the Linux fallback lane (run by me); the Windows FFI
paths behind these errors are UNVERIFIED (no Windows host).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never a bare string, never a fake success" | error.rs:3-5 | REAL | all 9 variants structured; fallback seams return typed reasons |
| "The same shape is mirrored in `platform-windows`/`platform-macos`" | error.rs:5-8 | FALSE as written here | names itself + macos, omits linux (copy-paste, F-002); the mirroring itself is real (diff) |
| "A helper program the seam shells out to" (SpawnFailed) | error.rs:67 | FALSE at the only call sites | wasapi.rs uses it for thread spawns (F-003) |
| "errors are data, and the error path is a first-class outcome" (MP §8) | error.rs:4-5 | REAL in-crate, UNREACHABLE in product | no app depends on the crate (F-001) |

## 7. Dead / duplicated / unreachable code
Three byte-identical copies (windows/linux/macos error.rs) — the
recorded unification [PROPOSAL]. InvalidEncoding unused in this crate
(F-004). Whole vocabulary product-dead (F-001).

## 8. Dependencies & flags
std only; no unsafe; no feature gates. The crate compiles on all
targets (cfg fallback lanes) — verified by the Linux test run.

## 9. Open questions for the next agent
- wasapi.rs (787 LOC) and its macos twin coreaudio.rs (766 LOC) need a
  Windows/macOS host to verify the FFI lanes; the Linux lane only
  proves the fallbacks. Whoever audits platform-windows/src/windows/
  should own that UNVERIFIED item.
- If the Lead unifies the L5 vocabulary ([PROPOSAL]), SpawnFailed needs
  a ThreadSpawnFailed sibling first — the misuse is the vocabulary's
  own gap, not just the caller's.
