# AUDIT: apps/web/src/audio/mod.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2025-06-27 |
| Lines / functions | 48 / 3 (module decls + 3 re-export lines; no code) |
| File verdict | REAL (module hub; docs verified against tree — one soft spot) |
| Reachable from shipped entry point? | PARTIAL — `autoplay` YES: host.rs:102 wires `AutoplayGate`, `HostToWorker::AudioContext` (host.rs:1057-1066) drives it, `GestureRequired` rides the ABI out (protocol.rs:192,220), test-proven end to end (`host::tests::the_autoplay_gate_parks_freezes_and_resumes ... ok`, baseline:3367). `sink` + `clock` NO: re-exported (lib.rs:71, mod.rs:47-48) but constructed by nothing outside their own tests (see clock.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (existence/wiring tracing, exit 0); sed -n + od -c (quote byte-verification, exit 0); baseline test citation |

## 1. One-paragraph verdict
A 48-line module hub whose value is its doc contract — and the contract survives hostile checking unusually well: the three named pieces exist; the "arithmetic twin" JS processor (`apps/web/audio-worklet.js`, 233 lines) exists and pins the same constants (JS `QUANTUM = 128` / `PREBUFFER_TARGET = 256` vs Rust `WORKLET_QUANTUM: usize = 128` / `PREBUFFER_TARGET = 2 * WORKLET_QUANTUM`); "No `AudioContext` is constructed here" is true (no DOM, no imports); and the v1 null-device boundary is stated in the doc itself rather than hidden. The one soft spot: "the page's `AudioContext` state reports drive it" describes the Rust message contract — which is real and test-proven — but the shipped page (ui/src) creates no AudioContext and sends no `audioContext` report, so nothing drives the gate in a real browser session today.

## 2. What actually works (verified)
— `rg 'AudioContext' ui/src` → no hits: the page never creates a context (consistent with the doc's page-owns-it framing, and with the missing driver, F-001).
— Constants match across the seam: `apps/web/audio-worklet.js:56,58` (`const QUANTUM = 128;` / `const PREBUFFER_TARGET = 2 * QUANTUM;`) vs `apps/web/src/audio/sink.rs:52,121` — verified by sed.
— Baseline: `audio::autoplay::tests::*` 4/4 ok (3336-3339) + `host::tests::the_autoplay_gate_parks_freezes_and_resumes ... ok` (3367).
— `GestureRequired` exists in the wire protocol (protocol.rs:192 enum, :220 `"gestureRequired"` tag).

## 3. Findings
F-001 [S3]
Where: apps/web/src/audio/mod.rs:9-14 (autoplay bullet), vs ui/src (no AudioContext anywhere)
Code:
```
//! - [`autoplay`] — the autoplay-policy gate (doc 09 §5.3): the
//!   page's `AudioContext` state reports drive it; a
```
What is wrong: The Rust half is genuinely wired (message → gate → `GestureRequired`, test-proven headless), but the doc's active voice ("the page's reports drive it") has no page-side driver in the shipped UI: `rg 'AudioContext' ui/src` returns nothing — no context is created, no `audioContext` message is ever sent, so in a real browser the gate sits open and `GestureRequired` can never fire. The doc's own "honest boundary" section discloses the null-device reality for OUTPUT but not the missing context-state REPORTS. Why it matters: a reader concludes the paused-pending-gesture UX is live in the browser; it is not. What correct looks like: one more sentence in the boundary section: "the page-side context probing/reporting is card 7.I1" — same disclosure style as the fetch lane. Confidence: HIGH — Verification: read (rg tracing, exit 0).
F-002 [S4]
Where: apps/web/src/audio/mod.rs:47-48 (re-exports)
Code:
```
pub use clock::{DriftPolicy, Resampler, WallclockAnchor, WorkletClock};
pub use sink::{ProcessReport, WORKLET_QUANTUM, WorkletQueue};
```
What is wrong: `DriftPolicy`/`Resampler`/`WallclockAnchor`/`WorkletClock`/`ProcessReport`/`WorkletQueue` are re-exported up to lib.rs:71 but consumed by no non-test code — the sink/clock half of the "pillar" is a library island. Hygiene: public surface wider than the wiring. Confidence: HIGH — Verification: read (rg usage tracing).

## 4. Per-function table
File is 48 lines of declarations — table not required (<100 LOC). Items: `pub mod autoplay` (39), `pub mod clock` (40), `pub mod sink` (41), three `pub use` lines (43-48). All resolve to real files; all exports exist in the submodules (verified by rg).

## 5. Test honesty
No tests in this file. Submodule tests (autoplay 4, clock 6, sink suite) all pass in baseline (3336-3345 + sink entries above them).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| three pieces: autoplay/sink/clock | mod.rs:7-8 | REAL | three files exist |
| sink is "the arithmetic twin of the hand-written JS processor (`apps/web/audio-worklet.js`)" | mod.rs:17-19 | REAL (constants) / REAL-UNPROVEN (full arithmetic equivalence) | file exists, QUANTUM/PREBUFFER pinned equal (sed-verified); the complete queue-behavior equivalence is a hand-audit claim |
| "No `AudioContext` is constructed here … the PAGE owns the context" | mod.rs:33-36 | REAL | no DOM/imports in the wasm crate; page indeed owns nothing (it creates none either — F-001) |
| "the v1 host's own audio output remains the null device" | mod.rs:36-37 | REAL | host.rs:38-44, 1730-1741 (NullBackend + MediaClock::audio_master) — see clock.rs audit |
| "models' arithmetic is proven native-cfg by tests (labeled simulation …)" | mod.rs:41-43 | REAL | baseline 3336-3345 all ok |
| GestureRequired typed event `[PROPOSAL]` | mod.rs:12-14 | REAL | protocol.rs:192,220; host emits once per park cycle (host.rs:1171-1176) |

## 7. Dead / duplicated / unreachable code
`sink` + `clock` submodules unreachable from shipped entry points (their audits). The re-exports at 47-48 are the only "use" of most of them.

## 8. Dependencies & flags
None beyond the crate's own submodules. No feature gates.

## 9. Open questions for the next agent
- The JS processor (`apps/web/audio-worklet.js`) is "hand-audited transcription" per the docs — nobody in this wave audited it line-by-line against sink.rs. Assign it explicitly; it is the only shipped-file half of the audio lane.
- worker.js mangles in raw tool output too: `messages[messages.length - 1]` displays as `messagesessages.length - 1]` — the `[m`-eating output mangler again (od -c verified true bytes `m e s s a g e s [ m e s s a g e s`). Any agent quoting worker.js must od-verify.
