# AUDIT: crates/playback-engine/tests/session_flow.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 2017 / 46 (34 tests + 12 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only file, but it drives the REAL production engine: `Engine::poll` (crates/playback-engine/src/session.rs:1157) — the same fn the shipped CLI host calls (apps/cli/src/host.rs:231 `Engine::new()`, :448 `self.engine.poll()` in `CliHost::pump_pipeline`). Stage objects are test fakes (see F-001 context). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p playback-engine --test session_flow` -> 34 passed / 0 failed / 0 ignored (exit 0); `cargo test -p playback-engine --test session_flow the_engine_debug_renders` -> 1 passed (exit 0); rg/sed verifications listed per finding; baseline cross-check audit/runs/baseline/cargo-test-workspace.txt:4442-4483 (34 passed, matches). |

## 1. One-paragraph verdict
Hostile question answered: the session_flow tests exercise the REAL session executor, not a model double — every test drives `playback_engine::Engine::poll` (the production state machine in src/session.rs, the same entry the shipped `CliHost` pumps), and the module doc admits up front that the stage objects (demuxer, decoder, sink, clock) are fakes injected through the real `SessionConfig` seams. The engine's own pipeline wiring (queues, watermarks, backpressure parking, epoch/staleness hub) is production code under test. What is NOT exercised here: real demuxers, real decoders, real audio output, wall-clock pacing — the fakes stand in for all of those (inherent to the design, honestly disclosed at lines 1-5). The assertions are unusually strong (exact event trails, exact generation stamps, seq monotonicity, frame-count equalities, concealment pixel equality); 34/34 pass in my own run. Defects found are test-honesty blemishes: one test whose name and doc comment claim a transition "from Playing" that its body never reaches, a decoy clock field for video fixtures, a near-tautological Debug test, and one under-specified close-during-open test.

## 2. What actually works (verified)
— `cargo test -p playback-engine --test session_flow` -> `test result: ok. 34 passed; 0 failed; 0 ignored` (exit 0). All lifetimes, epoch, ordering, damage-isolation, and bound-pinning tests pass against the real Engine. Mutation reasoning (no edits): break the epoch suppression and `superseded_events_are_suppressed_but_receipts_survive` (1050) + `superseded_epoch_events_never_leak_into_the_next_session` (1966) fail on the gen/leak asserts; drop a queued packet and `a_full_queue_holds_the_packet_and_never_drops_fresh_data` (1257) fails on `frames == 200`; mis-order end-of-media and the happy-path test fails on `end_index < ended_index` (130). These tests bite.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/session_flow.rs:760-769, 817-827
Code:
```
fn an_armed_decode_failure_transitions_to_error_from_playing() {
    // The provider arms every decoder it hands out with a one-shot
    // failure; the first decode call turns fatal from Playing (the
```
and
```
    // The failure fires during the first decode (Buffering tick): the
    // session lands in Error with the typed cause on the stream.
```
What is wrong: the test NAME says "transitions to error from_playing" and the doc comment says "the first decode call turns fatal from Playing", but the body's own inline comment (817) admits the failure fires during a Buffering tick — decoding starts during priming (proven by `pause_and_resume...` line 175: "buffering ticks already decode"), so the session never reaches Playing and no assert ever checks it did. The `Playing -> Error` row (mid-stream non-damage decode failure while playing) is claimed but never exercised. Why it matters: a regression that broke only the Playing→Error edge (e.g., a missing table row or a tick_step that swallows fatal decode errors once playing) would pass this test. What correct looks like: prime to Playing with a healthy provider, then arm/swap the failure (or use a decoder that fails on packet N after priming), then assert the transition's from-state on the wire. Confidence: HIGH — Verification: read + ran (test passes; the doc/name and the body comment contradict each other in the same function).
Confidence: HIGH — Verification: read / ran

F-002 [S3]
Where: crates/playback-engine/tests/session_flow.rs:1508-1512, 1497-1504
Code:
```
/// The parked fixture clock placeholder (the real session clock is
/// the hand-advanced source returned by [`open_video`]).
fn media_clock_virtual() -> Arc<media_clock::MediaClock> {
    virtual_clock()
}
```
What is wrong: for every video fixture, `Fixture.clock` is a decoy — a freshly minted virtual clock that the session never uses (the real session clock is the `SharedNow`-driven one built in `open_video` at 1471-1479 and returned separately). Any future test that reads `fixture.clock` on a video fixture (e.g., `fixture.clock.is_paused()`, as the happy-path test does at line 167) reads a clock nothing is attached to. Why it matters: misleading test-scaffold field invites false asserts (a paused-check on the decoy would pass vacuously or lie). What correct looks like: put the real `Arc<MediaClock>` into `Fixture.clock` (it is already in hand at 1472) and delete `media_clock_virtual`. Confidence: HIGH — Verification: read (the real clock is `clock` at 1472-1479; the fixture gets `media_clock_virtual()` at 1501 instead).
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/playback-engine/tests/session_flow.rs:1325-1331
Code:
```
fn the_engine_debug_renders_the_observable_surface() {
    let fixture = open_audio(40);
    let text = format!("{:?}", fixture.engine);
    assert!(text.contains("Engine"), "{text}");
    assert!(text.contains("Opening"), "{text}");
}
```
What is wrong: "Engine" is the struct name — a derive(Debug) output always contains it; that assert cannot fail while Debug is derived on the type at all. Only "Opening" does any work (pins that the state enum renders its variant name). The test name ("the observable surface") oversells two substring checks. Not dishonest, just thin. What correct looks like: assert the debug string of a couple of states (Opening, Playing) and that no internal payload leaks if that is the intent, or delete the "Engine" assert. Confidence: HIGH — Verification: read + ran (passes).
Confidence: HIGH — Verification: read / ran

F-004 [S4]
Where: crates/playback-engine/tests/session_flow.rs:394-408
Code:
```
fn close_while_opening_returns_to_idle() {
    // doc 05 §3.3's `opening -> idle` edge: a close racing the open.
    let mut fixture = open_with(
        vec![track(1, MediaKind::Audio)],
        vec![DemuxEvent::EndOfStream],
        None,
        None,
    );
    // The session is Opening before the first poll runs the open step.
    assert_eq!(fixture.engine.state(), SessionState::Opening);
    fixture.handle.close(RequestId(1)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
}
```
What is wrong: asserts only the terminal state, never the `opening -> idle` edge on the event stream (the doc comment names that edge), never the receipt shape — unlike its sibling `stop_while_opening_tears_down_without_wiring` (417) which checks the full trail. A regression that reached Idle via a wrong trail (e.g., `opening -> stopped -> idle` or a missing receipt) would pass. Blast radius: one edge of the state table. What correct looks like: assert the receipt (`Completed`) and the exact one-edge trail, as the sibling test does. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| the_happy_path_reaches_ended_with_the_documented_event_chain | 66-168 | REAL | — | exact 4-edge trail, ordering, bracket, 40-frame equality; ran ok |
| pause_and_resume_freeze_and_thaw_the_clock | 170-195 | REAL | — | frozen-collector quiet loop while Paused; ran ok |
| an_underrun_rebuffers_and_recovery_primes_again | 197-242 | REAL | — | stall via fake Discontinuity markers; 2 episodes asserted |
| starvation_metrics_count_the_underrun_episode_in_engine_stats | 251-316 | REAL | — | pins starve_episodes/signals/ticks + full-rejection axes |
| short_media_starts_with_what_exists_and_drains_to_ended | 318-351 | REAL | — | below-watermark start path; 1.0 bookend |
| stop_while_playing_tears_down_and_bumps_the_generation | 353-392 | REAL | — | gen 1 in Accepted receipt; inertness loop; close→Idle |
| close_while_opening_returns_to_idle | 394-408 | SHALLOW | S4 | terminal state only; no trail/receipt (F-004) |
| stop_while_opening_tears_down_without_wiring | 417-465 | REAL | — | exact one-edge trail + gen-bump + inert loop |
| stop_while_buffering_closes_the_episode | 475-533 | REAL | — | 1.0 bookend ordering vs → stopped |
| close_from_playing_is_the_two_step_composition | 543-605 | REAL | — | one receipt, exact 2-edge trail, gen==2 on all events |
| error_cycle_twice | 612-684 | REAL | — | 2 cycles, fresh epochs, engine-global seq monotonic across close/reopen |
| an_open_failure_lands_in_error_with_the_fatal_first | 686-758 | REAL | — | fatal-before-error, wire PlaybackError{Inaccessible, fatal}, inert-until-close |
| an_armed_decode_failure_transitions_to_error_from_playing | 760-840 | SHALLOW | S3 | name/doc say "from Playing"; body fires in Buffering (F-001) |
| nothing_decodable_is_a_typed_config_failure | 842-882 | REAL | — | typed `session.no-decodable-track` + wire Unsupported |
| a_subtitle_only_source_plays_and_drains_honestly | 890-913 | REAL | — | would fail if subtitle limb regressed to refusal (Fatal asserted absent) |
| a_stale_handle_is_rejected_typed_after_close | 919-937 | REAL | — | typed SessionClosed on play + submit |
| a_second_open_from_playing_replaces_the_session | 939-985 | REAL | — | session id 2, old handle dead, old collector frozen at switch |
| reopening_mints_a_fresh_epoch_and_continues_the_seq | 987-1047 | REAL | — | seq continuity across epochs, strictly increasing |
| superseded_events_are_suppressed_but_receipts_survive | 1049-1090 | REAL | — | gen>=1 filter + suppressed_stale_events > 0 |
| receipts_precede_the_events_they_cause | 1092-1135 | REAL | — | receipt index < paused index; origin == RequestId |
| rejected_commands_answer_typed_receipts_and_change_nothing | 1137-1178 | REAL | — | AlreadySettled typed rejection; play-while-playing self-transition |
| demux_marks_are_counted_and_damage_is_reported | 1184-1218 | REAL | — | demux_discontinuities == 1, DamagedData [2] |
| first_frame_is_reported_and_stays_engine_private | 1220-1254 | REAL | — | pts==0; FirstFrame never maps to wire |
| a_full_queue_holds_the_packet_and_never_drops_fresh_data | 1256-1265 | REAL | — | 200 packets, 16-cap queue, all 200 delivered |
| taking_outputs_twice_yields_nothing_the_second_time | 1267-1274 | REAL | — | drain semantics |
| the_outbox_drops_oldest_events_at_capacity_and_counts_them | 1276-1323 | REAL | — | outbox cap 4; receipts survive; dropped count > 0 |
| the_engine_debug_renders_the_observable_surface | 1325-1331 | SHALLOW | S4 | two substring checks; "Engine" is tautological (F-003) |
| VideoDecoder (struct+impl Decoder) | 1343-1380 | REAL | — | scripted corrupt schedule keyed on packet payload; honest flush |
| VideoProvider (struct+impl DecoderProvider) | 1384-1401 | REAL | — | per-limb decoder seam |
| video_packet | 1405-1416 | REAL | — | index-in-payload, 1 ms duration |
| packet_index | 1419-1425 | REAL | — | u64::MAX on short payload (safe) |
| video_frame | 1428-1441 | REAL | — | pixel = index % 256 (distinguishes concealment repeats) |
| corrupt_frame_error | 1445-1455 | REAL | — | real MJPEG damage vocabulary (Corrupt/InvalidStructure) |
| open_video | 1463-1506 | REAL | S3 | real clock returned separately; decoy parked in fixture (F-002) |
| media_clock_virtual | 1510-1512 | STUB | S3 | placeholder admitted by its own doc (F-002) |
| poll_until_ended_presenting | 1517-1538 | REAL | — | advances virtual clock 40 ms/poll; asserts at limit |
| collected | 1541-1547 | REAL | — | poison-recovering clone of collector frames |
| one_corrupt_video_frame_is_concealed_and_playback_runs_to_end | 1549-1672 | REAL | — | 23+1 presentations, pixel-identical concealment re-stamped at slot 10 pts, one coalesced non-fatal wire error |
| a_corrupt_first_video_frame_holds_and_playback_continues | 1674-1735 | REAL | — | held slot, no fabrication, FirstFrame at slot 1 pts 1000 |
| all_corrupt_video_reaches_the_typed_terminal_within_the_budget | 1737-1808 | REAL | — | ResourceLimit/RecoveryAttempts, attempted == budget+1, damage sum == budget |
| non_damage_video_decode_errors_stay_fatal | 1810-1855 | REAL | — | Unsupported stays fatal verbatim, zero concealments |
| the_documented_session_bounds_are_pinned | 1863-1880 | REAL | — | constant tripwire (64/1024/4096), documented intent |
| the_inbox_refuses_typed_at_its_bound_and_drains_whole | 1889-1925 | REAL | — | exactly 64 admitted, 8 InboxBusy, in-order drain, room after |
| open_on | 1933-1953 | REAL | — | builder for engine-scoped two-session scenarios |
| prime_playing | 1955-1963 | REAL | — | bounded prime with assert |
| superseded_epoch_events_never_leak_into_the_next_session | 1965-2017 | REAL | — | no session!=2 events; receipts 80/81 survive; suppressions counted |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_happy_path... | YES | exact trail/ordering/counts; any state-machine or pipeline defect moves an edge or a count |
| an_armed_decode_failure... | PARTIALLY | fails if Buffering→Error fatal path breaks; passes if ONLY Playing→Error breaks (F-001) |
| close_while_opening... | PARTIALLY | fails if open/close handling breaks; passes if the trail/receipt is wrong (F-004) |
| the_engine_debug... | BARELY | only the "Opening" substring can fail |
| all others (30) | YES | exact receipts, generations, seqs, counts, pixel equality, ordering indices |
| (fakes honesty) | N/A | FakeDemuxer/FakeDecoder are the things under test's collaborators, not the thing tested — Engine is the target; no mock of Engine exists in this file (rg `Engine` -> only `playback_engine::Engine`) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "driven end-to-end through Engine::poll with fake stage objects ... the real executor, not a mock of it" | session_flow.rs:1-5 | REAL | `Engine::poll` is src/session.rs:1157 (production); CliHost calls same fn (apps/cli/src/host.rs:448); my run: 34 passed |
| "output ordering: receipts before the events they cause ... end-of-media before state-changed → ended; tracks.list-ready before first playing" | session_flow.rs:13-17 | REAL | asserts at 130, 149, 1126-1129; ran ok |
| "generation bump at stop accept time with the NEW gen in the receipt" | session_flow.rs:18-20 | REAL | assert at 366-372 (gen 1 in Accepted); ran ok |
| "backpressure parking that never drops fresh data" | session_flow.rs:24-25 | REAL | 200/200 delivered at 1263-1264 |
| "the checkpoint never dropping a live-generation frame" | session_flow.rs:26-27 | REAL (indirect) | stats.stale_frames_dropped == 0 at 165 on live frames |
| "seq engine-global strictly increasing" | session_flow.rs:17 | REAL | windows(2) checks at 999-1004, 1044-1046, 652-658 |
| Baseline "34 passed" | audit/runs/baseline/cargo-test-workspace.txt:4442-4483 | REAL | my own run reproduced: 34 passed, 0 failed |

## 7. Dead / duplicated / unreachable code
- `media_clock_virtual` (1510-1512): dead-in-effect placeholder, only called at 1501 to fill a decoy field (F-002).
- `Fixture.clock` is live for audio fixtures (built by `open_with`) but a decoy for video fixtures (F-002).
- No `#[ignore]`, no `todo!`/`unimplemented!` in this file (rg verified); the 12 `unreachable!` are match-guards on already-filtered shapes (test-internal, acceptable).

## 8. Dependencies & flags
Test-only deps via tests/common/mod.rs: codec_api, commands, media_types, media_clock, packet_queue, playback_engine, plugin_api, testkit — all workspace crates; no feature gates, no external fixtures, no network, no sleeps (virtual clock). The suite is single-threaded (direct poll), no cfg-gating anywhere in the file.

## 9. Open questions for the next agent
- Is the `Playing | fatal decode | Error` table row pinned anywhere at unit level (table.rs / recovery_media.rs)? If not, F-001's gap is repo-wide. (table.rs is another agent's file.)
- `stop_while_buffering_closes_the_episode` relies on the FakeDemuxer emitting Discontinuity markers forever while stalled — does the REAL demux contract define "no data yet, not ended" the same way (would real poll loops count unbounded discontinuities)? Check demux host contract.
