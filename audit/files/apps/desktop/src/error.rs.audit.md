# AUDIT: apps/desktop/src/error.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2025-07-14 |
| Lines / functions | 153 / 2 enums + 2 Display + 2 Error impls (0 logic functions, 0 tests in file) |
| File verdict | REAL vocabulary (consumed by wire.rs ×41, bridge.rs ×11, host.rs, mailbox.rs, shell.rs) — UNREACHABLE from the shipped binary, like the whole desktop library stack |
| Reachable from shipped entry point? | NO — `lumen-desktop` main.rs never constructs a WireError/HostError (it hand-prints its own pending message and exits 86); `open_webview` (the only HostError-returning public fn) has zero non-test callers |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p desktop --lib error` -> 2 passed (name-filter hit window_config tests only; no error.rs tests exist); `cargo test -p desktop --lib` (full) -> 65 passed, 0 failed; `rg "frame stream:|desktop host:"` across desktop src -> 0 matches outside error.rs |

## 1. One-paragraph verdict
A clean, typed, payload-carrying error vocabulary with correct Display renderings — exactly what a hostile-feed decoder should answer instead of panicking, and wire.rs constructs it in 41 places against malformed records. Nothing here is fake. The defects are small and documentation-shaped: no test anywhere pins a single Display string (the "hex-stable" BadMagic rendering claim is unexercised), the BadGeometry doc claims "five outcomes, all named" while wire.rs names exactly three distinct reasons, and the "typed pending state" HostError::WebviewPending is never actually returned by the shipped binary — main.rs duplicates the message with its own eprintln instead of calling the typed `open_webview`.

## 2. What actually works (verified)
— verified by `rg -c "WireError::|HostError::"`: wire.rs 41 construction sites, bridge.rs 11, host.rs 4, mailbox.rs 1, shell.rs 2 — the vocabulary is load-bearing for the frame-streaming decoder, not decorative.
— verified by `cargo test -p desktop --lib` (full run) -> `test result: ok. 65 passed; 0 failed` (includes wire.rs decoder suites that round-trip these variants).

## 3. Findings
F-001 [S4]
Where: apps/desktop/src/error.rs:80-84
Code:
```
            WireError::BadMagic { found, offset } => write!(
                f,
                "frame stream: bad magic 0x{:02x}{:02x}{:02x}{:02x} at offset {offset}",
                found[0], found[1], found[2], found[3]
            ),
```
What is wrong: No test anywhere asserts any Display string — `rg "frame stream:|desktop host:"` over apps/desktop/src returns matches only inside error.rs itself. The doc claim on BadMagic ("hex-stable in `Display`", L16) is unverified by any test; a formatting regression ships silently.
Confidence: HIGH — Verification: read + rg (0 external matches).

F-002 [S4]
Where: apps/desktop/src/error.rs:43-47
Code:
```
    BadGeometry {
        /// Human-readable reason (a closed set — geometry validation
        /// has five outcomes, all named).
        reason: &'static str,
    },
```
What is wrong: wire.rs names exactly three distinct BadGeometry reasons ("zero dimension", "dimension exceeds 65535", "payload exceeds the 1 GiB cap" — wire.rs:212,217,223) across 7 construction sites. "Five outcomes, all named" does not match the code; the "closed set" is also only a convention (any `&'static str` compiles).
Confidence: HIGH — Verification: read + rg (3 distinct reasons counted).

F-003 [S4]
Where: apps/desktop/src/error.rs:121-137 vs apps/desktop/src/main.rs:89-90
Code:
```
    /// The webview host is not available in this build — the typed
    /// ADR-004 pending state (this sandbox: no webkit2gtk-4.1 dev
```
What is wrong: The "typed ADR-004 pending state" is dead in the shipped flow: `DesktopHost::open_webview` (host.rs:222-224, the only constructor of `WebviewPending` in a public fn) has zero non-test callers; the shipped main.rs hand-prints `eprintln!("no webview host compiled: ADR-004 pending ...")` and returns its own `WEBVIEW_PENDING` constant. Two sources of the same truth (typed error vs free-text eprintln + const) — the typed one is the one never used.
Confidence: HIGH — Verification: read + rg (`open_webview` callers: only host.rs's own test :383).

## 4. Per-function table (file < 100 logic LOC; table covers the types)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WireError (enum, 9 variants) | 13-75 | REAL | — | every variant carries structured payload (bytes, offsets, lengths) — no stringly-typed errors |
| Display for WireError | 77-117 | REAL | S4(F-001) | correct renderings, untested |
| Error for WireError | 119 | REAL | — | blanket |
| HostError (enum, 2 variants) | 123-137 | REAL | S4(F-003) | typed pending state; shipped binary never mints it |
| Display for HostError | 139-151 | REAL | S4(F-001) | untested |
| Error for HostError | 153 | REAL | — | blanket |

## 5. Test honesty
No tests in this file. The variants are exercised indirectly by wire.rs/bridge.rs/host.rs suites (63 desktop lib tests green); the Display strings by nothing.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a hostile or buggy webview feed must never crash the host" | L5-6 | REAL (at this layer) | all 9 WireError variants are typed data errors; no panic paths in error.rs; wire.rs decoder returns them |
| "hex-stable in Display" (BadMagic) | L16 | REAL-UNPROVEN | rendering code read (L80-84); no test pins it |
| "five outcomes, all named" (BadGeometry) | L44-45 | FALSE | 3 distinct reasons in wire.rs (F-002) |
| "the typed ADR-004 pending state" | L124-125 | UNREACHABLE | open_webview has no non-test caller; main.rs hand-prints instead (F-003) |

## 7. Dead / duplicated / unreachable code
- `HostError::WebviewPending` minted only by uncalled `open_webview` (F-003).
- Whole module library-only (the desktop stack's shared condition — see my dispatch.rs audit).

## 8. Dependencies & flags
std-only (fmt, error). No features. Part of the `desktop` lib (release lane builds it clean).

## 9. Open questions for the next agent
- Does the ADR-004 wrapper plan (records/tauri-wry-2.x.md) commit to the typed HostError at the boundary, or will the eprintln duplication persist?
- Should Display pins be added when wire.rs's decoder suite is next touched (one-line asserts each)?
