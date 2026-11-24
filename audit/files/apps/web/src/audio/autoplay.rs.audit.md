# AUDIT: apps/web/src/audio/autoplay.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-24 |
| Lines / functions | 197 / 7 (struct + enum + 5 methods, 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | QUALIFIED YES — wired end-to-end inside the wasm host (host.rs:1064 report, :1131 observe_autoplay in the pump, :1213 fill_audio freeze, protocol.rs:95-122/184-220 messages), and driven in a real browser by the e2e harness (ui/e2e/web/site/harness.js:638-665); but apps/web is not in the release lane (lumen-cli/desktop) and the harness page self-declares "not the product UI" (ui/e2e/web/site/index.html:6-8) — browser contexts of the product UI run DemoTransport (ui/src/client/app-transport.ts:8-11) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n` quote verification; `rg -n 'AutoplayGate|GateChange|GestureRequired' apps/web/src` (wiring map); `rg -rln 'gestureRequired' ui/src` (no consumer); baseline cite cargo-test-workspace.txt:3336-3339,3367 (4/4 unit + host integration ok) |

## 1. One-paragraph verdict
A small, correct state machine, genuinely wired into the wasm host's pump and message handling, with an honest freeze-like-pause implementation (host integration test proves position stays 0 while parked and advances after the gesture) and once-per-cycle event discipline proven for both arrival orders. The gap is at the consumer end: the "paused-pending-gesture … explicit substate, never a lie" presentation exists nowhere — the engine state remains `Playing` while frozen (the host test itself asserts this), the transport-level `GestureRequired` event has zero product-UI consumers (browsers run DemoTransport; the wasm-host site is a QA harness), and the catalog row is [PROPOSAL]. Until a consumer lands, a parked session is exactly the "Playing but nothing moves" presentation the file's own design doc forbids.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/cargo-test-workspace.txt:3336-3339` (suspension_cycles_arm_and_clear, observation_surfaces_once_per_cycle, both_arrival_orders_surface_once, the_default_gate_is_open — all ok) and `:3367` (`host::tests::the_autoplay_gate_parks_freezes_and_resumes` ok — asserts engine reaches `Playing`, exactly 1 `GestureRequired`, `position_us()==0` frozen across pumps + a duplicate report, and advance after resume). State-machine invariant checked by read: `surfaced == true` implies `parked == true`; every unpark path resets both bits.

## 3. Findings
F-001 [S3]
Where: apps/web/src/audio/autoplay.rs:11-12 (design claim) vs host.rs:2591-2632 and ui reality
Code:
```
//! - the state presentation is *paused-pending-gesture* — an
//!   explicit substate, never a lie;
```
What is wrong: No such substate exists on any consumer surface. The engine state stays `PlaybackState::Playing` while parked — the host integration test asserts exactly that (`assert!(reached_playing, "the engine's own state machine plays");` host.rs:2618) with position frozen at 0. The only surfacing is the transport-level `WorkerToHost::GestureRequired` (host.rs:1173-1175), and no product code receives it: `rg` over ui/src finds no `gestureRequired` handler, browser contexts of the product UI run DemoTransport (app-transport.ts:8-11: "a plain browser context … the friendly DemoTransport simulation drives the UI"), the only page that speaks the protocol is the self-declared QA harness (ui/e2e/web/site/index.html: "not the product UI — this page is test tooling"), and the command-catalog row `playback.gesture-required` is [PROPOSAL] (doc 09 §12 OQ-3, admitted at autoplay.rs:36-40). Why it matters: the file pins "never a lie" as the binding design; today a parked session presents as Playing-with-frozen-clock and nothing explains it — the lie is delegated to a consumer that does not exist. What correct looks like: ship the catalog row + a UI treatment of `gestureRequired` (or a queryable parked flag in the state snapshot) with the 7.I1 browser lane. Confidence: HIGH — Verification: read + rg.
F-002 [S4]
Where: apps/web/src/host.rs:1064 (consumer of this file's API)
Code:
```
                // the page drove the gesture, and the freeze-thaw
                // is observable through the position reports.
                let _change = self.autoplay.report(running);
```
What is wrong: The host discards `GateChange::Resumed` on the floor. The variant exists to instruct the host (autoplay.rs:61-63: "playback may truly flow … the truth rule") and is unit-asserted, but the only production caller throws it away — resume works only because `fill_audio` polls `is_parked()` every slice. A swallowed-instruction seam: any future behavior keyed on Resumed (e.g. a resumed event to the page, metrics) silently never fires. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: apps/web/src/audio/autoplay.rs:96-102
Code:
```
        if !self.parked {
            // A fresh suspension: arm, and re-arm the latch (an
            // already-parked gate keeps its surfaced bit — duplicate
            // reports must not re-fire the event).
            self.parked = true;
            self.surfaced = false;
        }
```
What is wrong: A consumer that joins mid-cycle (UI remount, late-attaching page script) can never obtain the parked condition: a duplicate suspended report intentionally keeps `surfaced`, so the once-per-cycle latch means no re-notify, and the protocol exposes no queryable parked state. Documented as "never spam" — the trade-off is real but leaves a blind-join hole. Confidence: MEDIUM — Verification: read (no protocol state-query found: `rg 'parked' protocol.rs` = none).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AutoplayGate (struct) | 44-51 | REAL | — | 2-bit state, invariant holds |
| GateChange (enum) | 53-64 | REAL | S4 | Resumed produced+tested, discarded by the only caller (F-002) |
| AutoplayGate::new / Default | 66-75, 121-125 | REAL | — | open gate = headless truth |
| AutoplayGate::is_parked | 77-82 | REAL | — | polled by fill_audio |
| AutoplayGate::report | 84-104 | REAL | S4 | idempotent arm; blind-join hole (F-003) |
| AutoplayGate::observe | 106-118 | REAL | — | once-per-cycle latch; order-independent (tested) |
| 4 tests | 127-197 | REAL | — | full report/observation matrices |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| suspension_cycles_arm_and_clear | YES | arm/dup/resume/re-arm transitions asserted exactly |
| observation_surfaces_once_per_cycle | YES | once-latch + not-wanting + next-cycle all asserted |
| both_arrival_orders_surface_once | YES | the join-point property both ways |
| the_default_gate_is_open | YES | trivial but pins the headless default |
| host: the_autoplay_gate_parks_freezes_and_resumes | YES | end-to-end freeze at position 0, once, resume advances — the strongest evidence in this domain |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "playback.play is accepted; the engine enters its normal pre-play path" | autoplay.rs:8-10 | REAL | host test: engine reaches Playing while parked |
| "the state presentation is paused-pending-gesture — an explicit substate, never a lie" | autoplay.rs:11-12 | FALSE (no consumer/substate exists) | F-001 |
| "never fake playing … media time does not advance" | autoplay.rs:13-17 | REAL | host test asserts position_us()==0 across pumps |
| browser policies "[VERIFY] at test time per doc 09 §8" | autoplay.rs:4-5 | honest deferral | e2e harness asserts gesture-once/frozen in real browsers (Playwright) |

## 7. Dead / duplicated / unreachable code
`GateChange::Resumed` is dead-in-production (F-002): asserted by unit tests, discarded by host.rs:1064. Nothing else in-file.

## 8. Dependencies & flags
None (std-only, no serde — the wire forms live in protocol.rs). No unsafe. No features. Not in the lumen-cli/desktop release lane; exercised by the wasm lane (rc-smoke wasm-web-build) and the e2e browser matrix.

## 9. Open questions for the next agent
- 7.I1 browser-matrix owner: does the shipped posture (doc 09 §9.5, COOP/COEP off) ever put the harness site in front of users, or is apps/web engine-lane-only until a product web UI exists? The reachability verdicts of every apps/web file hinge on this.
