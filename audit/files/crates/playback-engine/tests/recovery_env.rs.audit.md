# AUDIT: crates/playback-engine/tests/recovery_env.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 1848 / 21 tests + fixture (NetworkDemuxer, PassDecoder, EnvFixture, builders, wire collectors) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — integration test binary for the recovery policies (card 3.C3). W00 baseline: 1751/2/25; this binary's individual result UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | `cat` chunks (1-250, 1188-1327); `rg` test/function inventory + assert scan (exit 0). No cargo. |

## 1. One-paragraph verdict

A serious end-to-end recovery suite: the three environment policies (interrupted network source, audio device loss, renderer surface loss) proven first as pure virtual-time schedules (exact backoff bounds, typed terminals, idempotent exhaustion) and then through the real `Engine` with guard-wired fake stages — with exact-value asserts throughout: full state-change traces compared as vectors, buffering episodes pinned to `[0.0, 1.0, 0.0, 1.0]`, retry counts read back from the guard, clock-continuity equality across the rebuffer, exactly-once delivery (40/40, both at the sink and at the fake source's counter), catalog wire rows checked field-by-field, and the doc 05 §3.3 fatal-before-state-changed ordering asserted by index. The module doc is unusually honest about its one big stand-in: a stalled source has no first-class signal in the engine, so the fake answers an observably-inert `Recovered { skipped: 0 }` token — the blocked-read shape of a real input thread is untested, and the inertness itself is asserted (no `playback.error` ever surfaces in the transient scenario). The engine-level device-loss tests duplicate (deliberately, per the one-bridge rule) the hotplug.rs hooks' coverage — which matters because hotplug.rs itself is unreachable in production (my hotplug audit); this suite is the only place the suspend/resume rows run, and it drives them through the same `Session::apply` path the hooks would.

## 2. What actually works (verified)

- `engine_interrupted_network_rebuffers_then_resumes` (1188-1315): exact six-entry state trace, both episodes bookended, retry-1-made-and-failed read from the guard, clock resumed at the frozen position (equality), 40/40 delivery at both sink and source counters, stall-token inertness asserted.
- Policy tests (608-912): backoff schedule (100/200/400/400 ms cap, max_attempts 6), transient recovery with exact schedule, exhaustion → `Network { ReconnectExhausted }` with stage/context/last-failure chained, time-expiry with attempts left, TLS never retried, non-network pass-through, retryability classification pinned, exhausted error → catalog wire row.
- Device tests (913-1187): suspend at remembered position with `audio.device-lost`, resume from remembered position (clock continuity), backoff-then-single-non-fatal-notice, repeated loss fresh windows, idempotence.
- Surface tests (1081-1187): dirty + recreate + audio-only degradation non-fatal, reattach with NO catalog event (the recorded v1 gap, asserted as absence), resize non-fatal, idempotent repeats.
- Engine-level device/surface tests (1470-1848) drive the same flows through the command layer and the real engine.

## 3. Findings

### F-001 [S3] The "interrupted network" AC is proven against a stall stand-in, not a blocking read
Where: crates/playback-engine/tests/recovery_env.rs:67-79 (doc), 161-168 (the token), 213/219 (its emission points)
Code (67-74, verbatim):
```rust
//! A stalled source must answer `Demuxer::next_event` *something*
//! without ending the stream or failing: the real input thread
//! **blocks** on its byte-stream read (doc 03 §4.1(c)) — the
//! synchronous test engine has no blocking seam and no stall event
//! kind. The fake answers
//! `DemuxEvent::Recovered { skipped: 0, … }`, which is observably
```
What is wrong: nothing dishonest — the limitation is documented and the token's inertness is asserted — but the shipped build's actual behavior when a network source blocks (real input layer, doc 03 §4.1(c)) has no coverage here: the fake returns instantly, the engine polls on, and the retry-window mechanics are what's tested. A real blocked read would hold the poll thread; whether the engine tolerates that (poll-time bound, deadline) is untested by this suite. The `[PROPOSAL: a first-class stall signal]` note (77-79) is the gap's owner.
Why it matters: doc 13's "interrupted network" row, if cited as proven end-to-end, overstates: the policy and engine integration are proven; the blocking-seam behavior is not.
Confidence: HIGH — Verification: read.

### F-002 [S4] `unreachable!` as failure vehicle and `None => unreachable!` fixture accessors
Where: 383 (`None => unreachable!("fixture has no network link")`), 598, and the per-test budget exhaustion panics. Same pattern as the property suites. The fixture accessor's `unreachable!` is a test-internal invariant — fine, but a `must()` would be idiomatic.

### F-003 [S4] Local fakes duplicate testkit's future fault primitives by design
Where: 56-65 ("testkit carries no synthetic fault sources yet (cards 3.D1/3.D2 …); per the card, every fault primitive this suite needs is a local fake"). Deliberate, documented, wave-collision-avoiding. Recorded so the D1/D2 cards' agents know the seam exists twice.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `advance_now`/`clock_ns` | 124-138 | REAL | — | virtual time; i64::MAX saturation |
| `NetworkLink`/`new` | 143-159 | REAL | — | delivered counter = no-fabrication surface |
| `stall_tick` | 163-168 | REAL | S3 | inert stand-in (F-001) |
| `NetworkDemuxer::{start,next_event,seek}` | 183-242 | REAL | S3 | guard-gated reads; time-expiry first; stall while waiting |
| `PassDecoder`/`PassProvider` | 247-294 | REAL | — | kind-aware pass-through |
| `video_frame` + error constructors | 296-362 | REAL | — | network_down/tls_failed/corrupt_media/device_gone |
| `EnvFixture` + `frames_of_kind`/`frames_total`/`frame_kind` | 369-414 | REAL | — | |
| `open_env`/`open_network`/`open_script_audio`/`open_script_av` | 420-513 | REAL | — | tiny queues/budget from tests/common |
| `poll_round`/`poll_engine_until` | 521-557 | REAL | — | bounded with context messages |
| wire collectors (`state_changes`/`buffering_values`/`wire_index`/`fatals`/`take_fail_source`) | 560-604 | REAL | — | |
| 8 source-policy tests | 607-912 | REAL | — | exact schedules, typed terminals, classification pins |
| 7 device/surface policy tests | 913-1187 | REAL | — | incl. asserted-absence of the v1 reattach event |
| 4 engine end-to-end tests | 1188-1848 | REAL | — | exact traces/counts/orderings (sampled in full: 1188-1315) |

## 5. Test honesty

| Test (representative) | Would it fail if the code were broken? | Reason |
|---|---|---|
| engine_interrupted_network_rebuffers_then_resumes | YES (reasoned) | vector-equality on the state trace, exact buffering bookends, guard retry counts, clock equality, 40/40 at two independent counters |
| source_retry_backoff_schedule_is_bounded_and_deterministic | YES (reasoned) | exact ms values incl. the cap binding |
| engine_permanent_network_interruption_is_the_typed_fatal | YES (reasoned) | catalog row field-by-field + ordering by wire index (doc 05 §3.3) |
| surface_recreate_resumes_compositing_without_events | YES (reasoned) | asserted absence of any event — a regression that emits one fails |
| all 21 | YES (reasoned) | every test sampled or inventoried carries exact-value asserts; none assert-only-not-panic |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| The three environment ACs "proven first as policy … then through the real engine" | header 1-7 | REAL-UNPROVEN | both halves present with exact asserts; run UNVERIFIED |
| "zero fabricated packets" | header 24-27 | REAL-UNPROVEN | delivered+sink counters both asserted; run UNVERIFIED |
| stall stand-in limitation | header 67-79 | REAL (an admission) | F-001 |
| "no catalog event (the recorded v1 gap)" for surface reattach | header 49-50 | REAL | asserted absence in test — the gap is pinned as behavior, so a future event addition breaks the test deliberately |

## 7. Dead / duplicated / unreachable code

Local fakes vs future testkit faults (F-003). No dead tests.

## 8. Dependencies & flags

playback-engine (incl. `recovery` module), commands (wire forms), media-clock, media-types, codec-api, plugin-api, testkit, tests/common. No features; virtual time only, no sleeps.

## 9. Open questions for the next agent

- src/recovery.rs (the policy core under test) is in another agent's list — the exact-schedule asserts here make it well-pinned, but the guard's `poll`/`read` state machine itself needs the source audit.
- Does the production streaming/input layer ever construct a `SourceRetryGuard`? If not, this policy core is another armed-nowhere seam (cf. my hotplug.rs and sync.rs findings). File: crates/streaming/src/*, crates/input/src/file.rs.
