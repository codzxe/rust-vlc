# AUDIT: crates/platform-linux/src/error.rs
| Field | Value |
|---|---|
| Auditor | W08-A10 |
| Date | 2026-11-24 |
| Lines / functions | 114 / 2 (Display::fmt, Error impl) + 8 enum variants |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — platform-linux has no dependents (rg over all Cargo.tomls) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg per-variant construction counts (exit 0); sed -n quote checks (exit 0); cross-crate mirror spot-check via rg TimedOut/deadline_ms in platform-windows/error.rs + platform-macos/error.rs (exit 0); baseline cargo-test-workspace.txt:3943-4002 |

## 1. One-paragraph verdict
A clean, typed error vocabulary: eight structured variants, a Display arm for each,
std::error::Error, no bare strings anywhere in the crate's seams. The mirroring claim
against platform-windows/platform-macos is true (same variant set, same Display strings —
spot-verified). One variant is dead weight on Linux: `InvalidEncoding` is constructed by
no production path in this crate — the one place that should raise it (PulseAudio string
conversion) uses `to_string_lossy` and silently mangles invalid UTF-8 instead, and the
"all variants render" test quietly omits it. Duplication across three crates is
deliberate and documented with a [PROPOSAL] to unify.

## 2. What actually works (verified)
— Baseline cargo-test-workspace.txt:3943-4002: platform-linux tests passed, including
  `platform_errors_render_their_details` (seam_behaviour.rs:412) — 8 Display cases
  assert their substring against real `to_string()` output.
— rg construction counts: 7 of 8 variants are constructed by real code paths
  (NotCompiledForThisOs×lib/capability, LibraryUnavailable×dl/pulse, SymbolMissing×dl,
  ServiceUnavailable×pulse/xdg, CallFailed×pulse/xdg, TimedOut×pulse, SpawnFailed×xdg).
— Mirror claim spot-verified: platform-windows/error.rs:15,56-60,98-101 and
  platform-macos/error.rs:15,56-60,98-101 carry the identical enum, TimedOut/deadline_ms,
  and Display text.

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/error.rs:62-66
Code:
```
    /// The OS returned a string that is not valid UTF-8/UTF-16.
    InvalidEncoding {
        /// The API that produced the string.
        api: &'static str,
    },
```
What is wrong: `PlatformError::InvalidEncoding` is constructed nowhere in platform-linux
(rg: only the definition site). The path that owns this failure mode converts PulseAudio
strings with `to_string_lossy()` (pulse.rs:151,164 — `CStr::from_ptr(raw).to_string_lossy().into_owned()`),
so a daemon or driver handing back invalid UTF-8 produces U+FFFD mojibake device names,
never the typed error the vocabulary promises ("never a bare string, never a fake
success" — a silently lossy string is exactly a swallowed decode error). The Display
test `platform_errors_render_their_details` (seam_behaviour.rs:413) has 8 cases covering
only 7 distinct variants — InvalidEncoding's Display arm (error.rs:104-106) is untested
and dead. Why it matters: the variant exists to make encoding failures first-class; on
Linux it cannot ever fire, and consumers branching on it (capability.rs) carry a
dead arm. What correct looks like: `CStr::from_ptr(raw).to_str().map_err(|_| PlatformError::InvalidEncoding { api: "pa_sink_info.name" })?.to_owned()` in pulse.rs, plus add
the variant to the rendering test.
Confidence: HIGH. Verification: read + rg counts.

F-002 [S4]
Where: crates/platform-linux/src/error.rs:5-8
Code:
```
//! — never a bare string, never a fake success (MP §8: "errors are
//! data, and the error path is a first-class outcome"). The same
//! shape is mirrored in `platform-windows`/`platform-macos` so the
//! Lead can later unify them into a shared L5 vocabulary crate
```
What is wrong: The vocabulary is triple-duplicated by hand across platform-linux,
platform-windows, platform-macos (verified identical today). The mirroring is
documented and intentional, but hand-maintained duplication of an error contract across
three crates invites silent drift — F-001 is the early symptom (one crate's variant set
already diverges in usage). Confidence: HIGH (duplication verified); severity LOW
because the [PROPOSAL] records the plan to unify.
Verification: read + rg.

## 4. Per-function table
| Function/variant | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NotCompiledForThisOs | 18-21 | REAL | — | Constructed by lib.rs foreign-OS arms; rendered+tested |
| LibraryUnavailable | 23-28 | REAL | — | Constructed by dl.rs; tested |
| SymbolMissing | 31-36 | REAL | — | Constructed by dl.rs/pulse_ffi load; tested |
| ServiceUnavailable | 39-44 | REAL | — | Constructed by pulse.rs/xdg.rs; exercised for real on this host (no PA daemon) |
| CallFailed | 46-53 | REAL | — | The workhorse (11 sites in pulse.rs); Some/None code both tested |
| TimedOut | 56-61 | REAL | — | Constructed by pulse.rs bounded waits; deadline_ms units documented and rendered |
| InvalidEncoding | 62-66 | UNREACHABLE | S3 | Dead on Linux; Display untested (F-001) |
| SpawnFailed | 68-73 | REAL | — | Constructed by xdg.rs zenity/kdialog fallbacks; tested |
| Display::fmt | 76-112 | REAL | — | All 8 arms correct; 7 tested, InvalidEncoding arm untested |
| impl std::error::Error | 114 | REAL | — | Empty impl over Display+Debug — correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| platform_errors_render_their_details (seam_behaviour.rs:412-475) | YES for the 7 covered variants | Asserts real `to_string()` substrings incl. both CallFailed code shapes; would fail on any Display rewrite. Gap: InvalidEncoding arm untested (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every failure a platform seam can produce is one of these variants" | error.rs:3-4 | SHALLOW | True for the constructed 7; the encoding failure mode bypasses the vocabulary via to_string_lossy (F-001) |
| "The same shape is mirrored in platform-windows/platform-macos" | error.rs:5-8 | REAL | Spot-verified identical enums/Display (windows/error.rs:15+, macos/error.rs:15+) |

## 7. Dead / duplicated / unreachable code
InvalidEncoding variant: dead in this crate (F-001). Whole crate unwired from the
shipped product (no dependents). Triple duplication of the vocabulary (F-002).

## 8. Dependencies & flags
std-only; no thiserror (hand-rolled Display per the zero-dep rule); derives
Debug/Clone/PartialEq/Eq — all fields Eq-able, sound.

## 9. Open questions for the next agent
- capability.rs branches on these variants to build the PlatformReport — check whether
  its InvalidEncoding arm is likewise dead code.
- When the Lead unifies the three vocabularies, the lossy-vs-typed divergence (F-001)
  must be resolved or Windows (typed) and Linux (lossy) will keep behaving differently
  for the same failure.
