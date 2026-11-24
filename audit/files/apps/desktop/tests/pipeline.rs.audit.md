# AUDIT: apps/desktop/tests/pipeline.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2025-06-27 |
| Lines / functions | 340 / 4 tests + Sim harness (4 methods) + common/mod.rs dependency |
| File verdict | REAL |
| Reachable from shipped entry point? | Test target: `cargo test -p desktop --test pipeline` (executed by me). It exercises the SHIPPED variant-B components (video-engine scheduler/queue, CanvasBridge, MailboxTransport, wire codec) but NOT the shipped wiring shape (drives `queue.drain_to(&mut bridge)` directly; the shipped GUI uses PresentTap + shell pacer, which has its own proof in shell.rs `frame_channel_proof_pump_once_ships_luf1_records`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p desktop --test pipeline` → 4 passed / 0 failed; `cargo test -p desktop --lib wire::` (cross-ref); read of tests/common/mod.rs |

## 1. One-paragraph verdict
A genuine end-to-end virtual-time suite, not a mock parade: the REAL `PresentationScheduler` + `PresentationQueue` drive the REAL `CanvasBridge` over `MailboxTransport` (through wire encode/decode) on one shared `AtomicU64` nanosecond timeline (common/mod.rs), with a simulated consumer drawing at fixed ticks. The nominal test proves exact arithmetic — 60 frames drawn in order, transport latency exactly 11,666,667 ns (= 16,666,667 tick − 5,000,000 encode cost, min=max=mean), slot drift exactly one consumer tick, mailbox conservation, 60 records shipped. The stressor proves latest-wins (20 Hz consumer vs 60 fps: drawn < 60, overflow > 0, monotonic ids, last drawn = id 59, latency ≤ 2·frame + consumer period). The generation test wires the real command-layer `CommandResponse` through `DesktopHost::observe_response` to a `flush_stale` that drops exactly 3 in-flight frames, conserved. The stub test runs the actual `lumen-desktop` binary and pins exit 86 plus the startup report strings (including the product.toml-derived "1280x720 (min 940x600)" — a config-drift tripwire). I ran all 4 — pass. Two hygiene findings: the suite verifies counters, never payload content (the marker-pixel helper exists in common but is unused here), and `scheduler.push`'s outcome is discarded.

## 2. What actually works (verified)
— `cargo test -p desktop --test pipeline` → `test result: ok. 4 passed; 0 failed; 0 ignored` (0.01s).
— Baseline cross-ref: audit/runs/baseline/cargo-test-workspace.txt:2598-2605 (same 4 tests ok).
— The exact-latency assertions are derivable from the constants (FRAME_30/CONSUMER_60/ENCODE_COST) — arithmetic verified by reading: 16,666,667 − 5,000,000 = 11,666,667 (L187-189), drift = one 60 Hz tick (L192-193).
— common/mod.rs verified: `Timeline` is one `Arc<AtomicU64>` shared by `FakeMonotonic`→`MediaClock::system_monotonic` and `TimelineEmitClock` — the single-coordinate-system claim is structurally true (L46-101).

## 3. Findings
F-001 [S4]
Where: apps/desktop/tests/pipeline.rs:179-205 (asserts) vs apps/desktop/tests/common/mod.rs:12-14 (claim)
Code: (verbatim)
```
    // Every frame drawn, in order, exactly once.
    assert_eq!(sim.drawn_ids, (0..60).collect::<Vec<_>>());
```
What is wrong: the end-to-end suite never inspects payload content — `drawn_ids`, latency summaries and mailbox stats are all counters. common/mod.rs:12-14 advertises "a per-frame **marker pixel** ... so every claim is checked against *content*, not counters", and `payload_marker` is exported for it, but pipeline.rs does not import it (only protocol.rs:77 does). A payload-corrupting bug in the mailbox/pool path (e.g., a recycled buffer returned with stale bytes and the refill skipped) keeps this suite green; only the protocol suite's bridge round-trip would catch it, one layer earlier and without the scheduler/queue in the loop. What correct looks like: `draw_one` records `payload_marker(EXTENT, &frame.payload)[0]` alongside the id and the nominal test asserts the 0..60 marker ladder. Confidence: HIGH — Verification: read + rg (`payload_marker` used only in protocol.rs).

F-002 [S4]
Where: apps/desktop/tests/pipeline.rs:104
Code: (verbatim)
```
        let _ = self.scheduler.push(frame);
```
What is wrong: `PresentationScheduler::push` returns `PushOutcome` with typed refusal variants (crates/video-engine/src/scheduler.rs:583-589: stale-reject path increments a stat and returns `PushOutcome::Stale`), and the harness discards it. A refused push would surface only as the confusing downstream `assert_eq!(delivered, 1, "one tick delivers one request")` failure instead of the honest cause. Cannot fire today (1 push per cycle into an 8-slot queue, same generation), so hygiene. Confidence: HIGH — Verification: read + scheduler signature check.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Sim::new | 70-92 | REAL | — | real scheduler cfg, queue(8), mailbox(capacity), bridge on shared timeline |
| Sim::present_cycle | 101-143 | REAL | S4 | F-002 discarded push; tick ordering model documented (no-op ticks never back-dated) |
| Sim::draw_one | 147-155 | REAL | S4 | records id + both latency series; F-001 no content check |
| Sim::drain_all | 159-164 | REAL | — | end-of-stream drain |
| test nominal_pipeline_delivers_every_frame_with_exact_latency | 170-206 | REAL | — | exact latency/drift/conservation; would fail on any ordering, drop, or timing bug |
| test slow_consumer_latest_wins_with_structural_bounds | 211-254 | REAL | — | conservation + monotonic + latest-wins + structural bound |
| test generation_bump_flushes_in_flight_frames | 260-313 | REAL | — | real CommandResponse through DesktopHost; flush 3, conserved, post-flush id 3 |
| test stub_binary_reports_pending_state | 317-340 | REAL | — | runs the real binary; exit 86 + report strings incl. product.toml-derived sizes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| nominal_pipeline_delivers_every_frame_with_exact_latency | YES (ordering/drop/timing) / NO (payload corruption — F-001) | exact counts + exact latency equalities |
| slow_consumer_latest_wins_with_structural_bounds | YES (policy/order) / NO (payload) | conservation identity + bounds |
| generation_bump_flushes_in_flight_frames | YES | exact flush count + conservation + post-flush frame_id |
| stub_binary_reports_pending_state | YES | subprocess exit code + stdout substrings (catches main.rs/product.toml drift) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "video-engine's REAL PresentationScheduler + PresentationQueue driving the CanvasBridge over the MailboxTransport" | L1-3 | REAL | imports + test run |
| "all on ONE virtual nanosecond timeline shared by the scheduler and the bridge's emit clock" | L4-6 | REAL | common/mod.rs:46-101 single AtomicU64 |
| "zero sleeps; end-to-end latency is measurable in a single coordinate system" | L6-8 | REAL | no sleep/Instant in suite; exact ns arithmetic asserted |
| "the honest stub: `lumen-desktop` reports the typed pending state and exits with the documented code" | L10-11 | REAL | test run, exit 86 |
| "every claim is checked against *content*, not counters" (common) | common/mod.rs:12-14 | SHALLOW for this suite | payload_marker unused in pipeline.rs (F-001) |

## 7. Dead / duplicated / unreachable code
`payload_marker` (common/mod.rs:142-150) is dead from this suite's perspective (used only by protocol.rs). The suite's wiring (direct `drain_to`) diverges from the shipped PresentTap/pacer shape — covered separately by shell.rs's own proof test, so no coverage hole claimed beyond F-001's content gap.

## 8. Dependencies & flags
Test-only deps: desktop (lib), media-clock (dev-dep, documented in apps/desktop/Cargo.toml), renderer-wgpu (synthetic frames + Extent2D), video-engine, testkit, commands (real response types). `#![allow(dead_code)]` in common (justified: shared fixture surface). No feature gates, no unsafe.

## 9. Open questions for the next agent
- Does any test anywhere assert the LUF1 payload content END-TO-END through the shell pacer (FrameTap drain)? shell.rs's `frame_channel_proof_pump_once_ships_luf1_records` asserts record count + magic prefix only — combine with F-001: content identity through the full shipped path (scheduler→tap→re-encode→tauri record) may be unpinned anywhere.
