# AUDIT: crates/playback-engine/tests/contract_suite.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 1499 / 42 (12 #[test], ~30 helpers/fakes/builders) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for the surface under test: the wire contract (EngineOutput→WireEvent, EventPolicy throttle, stats counters) is drained by the shipped CLI host — apps/cli/src/host.rs:448-449 `self.engine.poll(); self.drain_outputs();` → :645 `self.engine.drain_outputs_into(&mut outputs)` and :976 `config.throttle = playback_engine::EventPolicy { .. }`. Same pattern in apps/web/src/host.rs. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg test count (12); baseline cargo-test-workspace.txt:4191-4210 (12 passed, 0 failed); sed quote verification; rg EventPolicy/take_outputs wiring (apps/cli, apps/web). First repo-wide rg timed out (120s, killed) — narrowed to crates/*/src + apps/*/src, exit 0. |

## 1. One-paragraph verdict
This is a genuine engine-level contract suite and the best file audited so far in this domain: every observable is taken from the real `Engine`'s output channel (`wire_stream` maps `EngineOutput::Receipt/Event`), every command goes through the real `SessionHandle` (play/pause/seek/stop), time is a virtual monotonic source shared by the media clock and the event throttle, and the assertions pin exact sequences (state trails, tick times, seek terminals with generations, coalesced damage counts). All 12 tests pass in the baseline run. The fakes are appropriate (scripted demuxer/decoder seams), and the stall shape (`DemuxEvent::Discontinuity`) is load-bearing: the engine counts and carries on (src/pipeline.rs:2169-2174), so the underrun scenarios genuinely depend on non-terminal stall semantics. Findings are hygiene-level only.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:4191-4210` -> "running 12 tests … test result: ok. 12 passed; 0 failed; 0 ignored" (0.00s — deterministic, no sleeps).
— verified by rg `EventPolicy` over crates/*/src apps/*/src -> wired in apps/cli/src/host.rs:976 and apps/web/src/host.rs; drained via `drain_outputs_into` (host.rs:645). The contract proven here is the contract the shipped host consumes.
— verified by sed of src/pipeline.rs:2169-2174 -> `Ok(DemuxEvent::Discontinuity) => { … report.discontinuities = report.discontinuities.saturating_add(1); }` — the stall stand-in is a counted no-op, exactly what the fake's underrun shape needs.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/contract_suite.rs:1172-1176 and 1258-1263
Code:
```
    assert!(
        fixture.engine.stats().suppressed_stale_events >= 3,
        "the gen-1 pipeline events were suppressed, not delivered"
    );
```
What is wrong: Lower-bound-only assertions on suppression counters. A regression that suppresses MORE than the stale set (e.g., dropping the new generation's legitimate events as well) still passes `>= 3` / `>= 2`; only under-suppression is caught. The delivered-set assertions elsewhere (tick_times empty, no 0.0 bookend) partially compensate, but the count bound is not an equality pin. Why it matters: over-suppression is the more dangerous failure mode (user-visible event loss) and it is unbounded above here. What correct looks like: assert the exact suppressed count, or enumerate the delivered set exhaustively. Confidence: MEDIUM — Verification: read.
F-002 [S4]
Where: crates/playback-engine/tests/contract_suite.rs:85-90
Code:
```
#[derive(Default)]
struct Collector {
    #[allow(dead_code)]
    frames: Mutex<Vec<(TrackId, Frame)>>,
}
```
What is wrong: The sink collects every frame into a Vec that no test in this suite reads (`#[allow(dead_code)]` admits it). Wasted per-frame work and an `#[allow]` hiding a lint the spec flags (121 in tree). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/playback-engine/tests/contract_suite.rs:81-83
Code:
```
fn advance_ms(now: &SharedNow, ms: u64) {
    now.0.fetch_add(ms * 1_000_000, Ordering::Relaxed);
}
```
What is wrong: `ms * 1_000_000` can overflow u64 in theory and `fetch_add` wraps silently. Test values are tiny; no practical impact, but unlike recovery_env's `advance_now` this one is not saturating. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow::now_ns | 61-64 | REAL | — | Virtual monotonic source. |
| advance_ms | 81-83 | REAL | S4 | F-003 non-saturating. |
| Collector::accept | 91-100 | REAL | S4 | F-002 dead collector. |
| Media / lock_media | 103-118 | REAL | — | Scripted media state. |
| ScriptDemuxer::start/next_event | 126-145 | REAL | — | Script feed, stall (`Discontinuity`), EOS. |
| ScriptDemuxer::seek | 146-153 | REAL | — | Typed refusal; engine uses reanchor. |
| ScriptDemuxer::seekability / reanchor | 154-185 | REAL | — | Compute-then-commit landing (D4); honest rposition. |
| MarkDemuxer (start/next_event/seek/seekability) | 189-224 | REAL | — | Fixed script incl. `Recovered` marks. |
| reanchor_refusal / stamp_ns | 226-244 | REAL | — | Typed refusal; saturating ns conversion. |
| FakeDecoder (configure/decode/flush) | 248-272 | REAL | — | 1:1 packet→frame, stale filter. |
| StallFlushDecoder (configure/decode/flush) | 279-316 | REAL | — | 9 flush calls vs 8 budget — forces cross-tick drain. |
| LimbProvider::decoder_for | 322-344 | REAL | — | Video limb gets the slow decoder. |
| base / at_millis / track / packet / audio_frame | 346-412 | REAL | — | Builders; 1 ms strides, 64-byte payloads. |
| tiny_queues / budget | 414-433 | REAL | — | Deterministic caps; 12 demux events/tick. |
| Fixture / open / open_with / open_script | 435-541 | REAL | — | Real Engine + handle; EventPolicy wired to the same virtual clock. |
| audio_media | 543-551 | REAL | — | n-packet audio script. |
| poll_until | 553-577 | REAL | — | Bounded, polls at least once, asserts on failure. |
| WireItem / wire_stream / kinds / receipt_for / event_index / kind_at | 579-655 | REAL | — | Wire-surface extractors over the real output channel. |
| tick_times / buffering_progress / state_trail / seek_terminals | 657-697 | REAL | — | Honest sequence extractors. |
| the_wire_stream_honors_every_doc_05_ordering_invariant | 715-896 | REAL | — | seq monotonic, receipt-before-cause, terminal invariants, exact trail + single pause tick. |
| c8_idempotent_commands_confirm_with_self_transition_echoes | 899-985 | REAL | — | Self-transition echo, receipt result pinned, no re-tick. |
| a_fatal_error_precedes_its_state_changed_to_error | 988-1022 | REAL | — | Fatal ordering + stage/code mapping. |
| time_ticks_fire_on_the_interval_and_carry_the_wire_payload | 1027-1056 | REAL | — | Exact 250 ms cadence, clamped fraction. |
| undelivered_time_ticks_are_never_queued | 1058-1086 | REAL | — | `time_ticks_replaced == 3` exact counter pin. |
| a_seek_completion_emits_the_immediate_tick_at_the_landing | 1088-1131 | REAL | — | Tick at terminal_at+1, landing µs. |
| a_completed_seek_terminal_survives_a_later_generation_mint | 1136-1184 | REAL | S3 | F-001 loose `>= 3`. |
| superseded_seeks_are_silent_and_stale_events_are_suppressed | 1188-1273 | REAL | S3 | F-001 loose `>= 2`; terminal exactly (Gen(2), true) at 250 ms. |
| in_episode_buffering_progress_is_monotonic_and_window_capped | 1278-1339 | REAL | — | Exact 3-sample shape (0.0, one fill, 1.0). |
| an_elapsed_window_emits_the_monotonic_climb | 1341-1401 | REAL | — | ≥2 strictly monotonic in-episode samples. |
| end_of_track_fires_for_a_limb_that_ends_before_the_media | 1403-1462 | REAL | — | Exactly one EOT (audio limb), precedes EOM. |
| non_fatal_damage_coalesces_into_one_counted_playback_error | 1464-1499 | REAL | — | One event, count "9" (2+3+4), non-fatal, still ends. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_wire_stream_honors_every_doc_05_ordering_invariant | YES | Exact state trail, seq monotonic across batches, payload equalities. |
| c8_idempotent… | YES | Receipt result + echo origin/generation pinned; tick absence pinned. |
| a_fatal_error_precedes… | YES | Ordering + stage/code equality. |
| time_ticks_fire… | YES | Exact tick-time vector. |
| undelivered_time_ticks_are_never_queued | YES | Exact surviving tick + exact replaced count (3). |
| a_seek_completion_emits… | YES | terminal_at+1 adjacency + landing value. |
| a_completed_seek_terminal_survives… | PARTIAL | Deliveries pinned exactly; suppression count only lower-bounded (F-001). |
| superseded_seeks… | PARTIAL | Same F-001 caveat; delivered set strongly pinned. |
| in_episode_buffering… | YES | Exact 3-sample shape with real queue-fill arithmetic. |
| an_elapsed_window… | YES | Strict monotonicity, ≥2 samples. |
| end_of_track… | YES | Exactly-one + kind/id + ordering. |
| non_fatal_damage… | YES | Exactly-one event, count string "9", fatal=false, ends cleanly. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every observable is a WireEvent/EventKind … never an engine-internal type, except the receipts" | file lines 1-6 | REAL | wire_stream (579-600) filters through `as_wire_event`; raw kinds never asserted. |
| "time-tick … never queued — an undelivered older tick is replaced" | lines 20-25 | REAL | Test at 1058-1086: only newest survives, `time_ticks_replaced == 3`. |
| "a superseded seek is silent (exactly one terminal, for the newest generation)" | lines 27-30 | REAL | Test at 1258-1271: `seek_terminals == [(Generation(2), true)]`. |
| "coalesces over the 1 s window into one counted fatal: false wire event" | lines 33-35 | REAL | Test at 1464-1499: one event, count "9". |

## 7. Dead / duplicated / unreachable code
`Collector.frames` (F-002). `Media::default()` dummy in `open_script`'s Fixture (the MarkDemuxer owns the real script) — cosmetic only. Nothing unreachable: the surface is the shipped host's drain path.

## 8. Dependencies & flags
No feature gates. Depends on playback_engine (Engine, EventPolicy, TimeTickPolicy, stats), commands (WireEvent/EventKind), media_clock, packet_queue (QueueLimits), plugin_api (SeekSupport/reanchor), testkit::Must (panicking unwrap, verified testkit/src/lib.rs:53-63).

## 9. Open questions for the next agent
- Does the web host (apps/web/src/host.rs) honor the same receipt-before-event and never-queued-tick contract when it drains, or only the CLI host? (Contract proven engine-side; host-side drain discipline untested here.)
- `engine.stats().suppressed_stale_events` / `time_ticks_replaced` — are these stats surfaced anywhere a host can see them, or test-only observability?
