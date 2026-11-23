# AUDIT: crates/playback-engine/tests/seek_flow.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 2023 / 67 (21 `#[test]`, 46 fakes/helpers/builders) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test file: dev lane only (`cargo test`). Code under test YES: apps/cli/src/dispatch.rs:131 `Command::PlaybackSeek { target } => host.dispatch_seek(*target)` -> apps/cli/src/host.rs:1059 `dispatch_seek` -> crates/playback-engine/src/session.rs:368 `SessionHandle::seek` -> `SessionInput::Seek` rows in src/state.rs:523-554 -> `Pipeline::demuxer_reanchor` (src/pipeline.rs:1498) -> `Demuxer::reanchor`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p playback-engine --test seek_flow` -> 21 passed, 0 failed (exit 0); baseline audit/runs/baseline/cargo-test-workspace.txt lines 4429-4453 (21 passed) |

## 1. One-paragraph verdict
This is a genuine, adversarial end-to-end suite of the engine's seven-step seek flow, driven through the real `Engine::poll` state machine with scripted stage objects at the correct seams (demuxer, decoders, sink, clock). The fakes are not mocks of the thing tested — the engine's session executor, state table, queue flush, generation minting, pre-roll hold, and event hub are all the real code. Tests assert exact event sequences, exact clock anchors, exact frame-pts sequences after release, and exact stats counters; a broken seek (no cursor move, no flush, no re-anchor, stale frames surviving) fails multiple assertions. I executed the suite myself: 21 passed. The one real problem is a doc-level lie about the CLI mechanism (see F-001), which this suite's existence actually refutes.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p playback-engine --test seek_flow` -> `test result: ok. 21 passed; 0 failed; 0 ignored` (exit 0). Also matches baseline cargo-test-workspace.txt:4429-4453.

## 3. Findings
F-001 [S3]
Where: apps/cli/src/help.rs:55-58 (claim touching this file's domain; cross-file)
Code:
```
    (
        "seek 90s | +10s | -1m | 50% | chapter 3",
        "playback.seek",
        "seek (v0: reopen at the target through the engine's real transitions)",
    ),
```
What is wrong: The CLI help claims v0 seek reopens the media at the target. The code does not reopen anything. `dispatch_seek` (apps/cli/src/host.rs:1059-1150) submits the engine-native `Seek` command; host.rs:26-38 states the stop→close→reopen composition "is FORBIDDEN for seek" and host.rs:74 states "the reopen fallback does not exist". The engine runs the seven-step in-pipeline flow (state.rs:523-554, pipeline.rs:1498 `demuxer_reanchor`). The help string is stale 2.D2-era text; this whole test file is the W1-06 replacement of that behavior. Why it matters: user-facing help describes a mechanism that is forbidden in the code; a reader would wrongly expect file re-open / device loss on every seek. What correct looks like: "seek (in-place: the engine's real transitions — mint, flush, re-anchor, pre-roll)". Confidence: HIGH — Verification: read (help.rs:55-58, host.rs:26-74, host.rs:1059-1150, pipeline.rs:1498-1500) + ran the suite proving the in-place flow works.
Note (context, not this file's defect): in-place seek works only for demuxers that implement `reanchor` (AVI, OGG, WAV, ADTS, PS, MKV, IVF, AIFF — verified overrides). MP4 keeps the trait default typed refusal (apps/cli/src/containers/mp4.rs:173-178: "the moving half of in-session seek is its honest gap"), so `playback.seek` on MP4 is always a typed `timing.seek-failed` with the old position — honest but a functional gap the help text hides.

F-002 [S4]
Where: crates/playback-engine/tests/seek_flow.rs:780-1567 (whole suite)
Code:
```
fixture
    .handle
    .seek(
        RequestId(50),
        SeekTarget::Absolute {
            position_ns: 200 * MS,
        },
    )
    .must();
```
What is wrong: every flow test uses `SeekTarget::Absolute` or `Percent`; no test drives `SeekTarget::Relative` through the flow. Relative resolution/clamping is unit-tested only in src/seek.rs (`relative_resolves_against_the_position_then_clamps`). A relative-target bug that only manifests across the flush/re-anchor (e.g. position snapshot taken after the flush) would not be caught here. Limited blast radius: resolution happens pre-reanchor in the same code path. Confidence: MEDIUM — Verification: read.

F-003 [S4]
Where: crates/playback-engine/tests/seek_flow.rs:568-619
Code:
```
fn open_slow_prime_video(count_each: u64, after: u32) -> Fixture {
    let mut packets = Vec::new();
    for step in 0..count_each {
        packets.push(packet(TrackId::new(1), step as i64 * 2));
        packets.push(packet(TrackId::new(2), step as i64 * 2 + 1));
    }
```
What is wrong: `open_slow_prime_video` and `open_flaky_video` (596-619) duplicate the identical interleaved packet/track construction differing only in the provider closure — copy-paste scaffolding. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/playback-engine/tests/seek_flow.rs:135-143
Code:
```
    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The pure query is not the engine's seek path (doc 03 §6
        // step 4 goes through `reanchor`); the honest refusal.
        Err(EngineError::new(
```
What is wrong: nothing functionally — this guard is good (if the engine ever called the pure `seek` path instead of `reanchor`, every test fails loudly with `demux.seek.not-the-reanchor-path`). Noted here because the analogous guard is absent at the CLI/MP4 level: nothing forces `StartedMp4::reanchor` to be wired to a real implementation, and it is not (see F-001 note). Confidence: HIGH — Verification: read + the guard is exercised implicitly by every passing test.

## 4. Per-function table (all fakes/helpers; every `#[test]` named)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow/MonotonicSource | 69-75 | REAL | — | frozen virtual clock, no sleeps (doc 03 §8) |
| Collector (FrameConsumer) | 79-92 | REAL | — | poison-recovering lock, delivery-ordered |
| Media + lock_media | 97-108 | REAL | — | shared cursor enables mid-run failure arming |
| SeekDemuxer::start/next_event | 120-133 | REAL | — | in-order emission, then EoS |
| SeekDemuxer::seek | 135-143 | REAL | — | refusal guard (F-004) |
| SeekDemuxer::seekability | 145-151 | REAL | — | seekable toggle drives NotSeekable path |
| SeekDemuxer::reanchor | 153-178 | REAL | — | compute-then-commit D4; moves cursor only on success; rposition landing ≤ target |
| stamp_ns / reanchor_refusal | 182-198 | REAL | — | helpers |
| FakeDecoder | 202-227 | REAL | — | 1:1 packet→frame, stale-gen drop |
| ReorderDecoder | 234-271 | REAL | — | owes one old-gen frame at flush — the stale-purge probe |
| DelayedDecoder | 277-309 | REAL | — | slow-prime across ticks |
| FlakyDecoder | 316-351 | REAL | — | per-frame Corrupt damage below threshold |
| ProviderKind/FakeProvider | 357-414 | REAL | — | per-track scripted decoders |
| base/at_millis/track/packet/audio_frame | 420-479 | REAL | — | builders; all packets keyframes (MJPEG shape) |
| tiny_queues/tiny_budget | 484-506 | REAL | — | sizing rationale documented and load-bearing |
| Fixture/open_audio/open_audio_video/open | 509-561 | REAL | — | |
| open_slow_prime_video / open_flaky_video | 568-619 | REAL | S4 | duplication (F-003) |
| open_kinds | 621-671 | REAL | — | wires real `Engine::new()` + `engine.open` |
| poll_until | 679-697 | REAL | — | always polls ≥1; hang = fail |
| take_events/frames_of/frames_snapshot | 699-718 | REAL | — | |
| receipt_for/seeked_terms/seek_failed_terms/event_index | 720-769 | REAL | — | exact-match event filters |
| buffering_count | 2010-2023 | REAL | — | |
| a_happy_seek_from_playing_runs_the_seven_steps | 778-873 | REAL | — | receipt gen, Buffering, terminal, wire µs form, clock anchor, post-seek pts ≥ landing |
| the_audio_frame_timestamp_is_the_landing_truth_not_the_request | 875-920 | REAL | — | actual=101ms vs request=100ms; anchor on audio truth |
| a_seek_from_paused_completes_without_leaving_paused | 926-1004 | REAL | — | state stays Paused every poll; bounded carve-out decode; resume works |
| a_seek_to_zero_lands_at_the_start | 1010-1046 | REAL | — | incl. negative clamp to 0 |
| a_past_end_seek_clamps_to_the_duration_then_ends_cleanly | 1048-1091 | REAL | — | requested=clamped 40ms, actual=last keyframe 39ms, ends cleanly |
| an_unseekable_source_refuses_typed_and_playback_continues | 1097-1161 | REAL | — | class-A accept + exactly one NotSeekable; all 400 frames still delivered |
| a_failed_reanchor_rolls_back_d4_and_stays_playable | 1163-1227 | REAL | — | armed failure; no repeated pts; loss bounded by queue slice (≥380/400) |
| percent_without_a_duration_is_the_typed_out_of_range | 1229-1251 | REAL | — | |
| a_superseded_seek_is_silent_only_the_newest_terminal_fires | 1257-1298 | REAL | — | two seeks before one poll; exactly one terminal (gen 2) |
| a_resolution_failure_does_not_retire_an_in_flight_seek | 1300-1333 | REAL | — | resolution failure ≠ supersession; both get terminals |
| stale_content_never_reappears_after_a_seek | 1339-1397 | REAL | — | stale_frames_dropped ≥ 1; no old-gen frame post-seek |
| a_seek_terminal_survives_a_later_generation_mint | 1403-1439 | REAL | — | drain exemption; suppressed_stale_events > 0 |
| a_seek_without_its_payload_is_rejected_typed | 1445-1470 | REAL | — | MissingPayload receipt |
| a_seek_while_opening_is_rejected_typed | 1472-1498 | REAL | — | NotReady; open unaffected |
| a_stale_handle_cannot_seek_after_close | 1500-1513 | REAL | — | Err(SessionClosed) |
| preroll_frames_are_held_until_the_seek_terminal_then_released_in_pts_order | 1519-1607 | REAL | — | zero sink growth accept→terminal; exact release order [101,103,105,107,110,112]ms; tracks [2,2,2,2,1,1] |
| a_superseded_seek_discards_its_held_preroll_frames_counted | 1609-1683 | REAL | — | preroll_frames_discarded == 3 exactly; no leak 100..250ms |
| a_session_stopped_mid_preroll_discards_its_held_frames_counted | 1685-1744 | REAL | — | stop mid-hold; count survives close, no double count |
| preroll_damage_conceals_nothing_the_hold_owns_presentation_until_the_terminal | 1746-1825 | REAL | — | damaged pre-roll; concealment off sink; frames_concealed == 0 |
| a_failed_seek_holds_nothing_and_publishes_the_minted_generation | 1827-1877 | REAL | — | rollback re-fills in failing poll; clock publishes gen |
| pause_during_buffering_holds_and_primes_silently | 1888-2007 | REAL | — | bookend 1.0 before Paused→Paused; resume direct, no second 0.0 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 21 | YES | Mutation reasoning (read): if `demuxer_reanchor` stopped moving the cursor, `a_happy_seek...` fails at the pts≥200ms assertion (862-872) and the anchor assertion (848-849); if the flush stopped purging stale frames, `stale_content_never_reappears...` fails (old-gen frame check 1386-1389); if the terminal were never emitted, `poll_until` asserts; if the pure `seek()` path were used instead of `reanchor`, every seek test fails on `demux.seek.not-the-reanchor-path`; if the hold leaked, `preroll_frames_are_held...` fails at the zero-growth assert (1553-1557). Exact-sequence asserts (1582-1598, 1804-1811) pin tick-level behavior. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seek (v0: reopen at the target through the engine's real transitions)" | apps/cli/src/help.rs:57 | FALSE (mechanism) / the "real transitions" half is REAL | host.rs:74 "the reopen fallback does not exist"; host.rs:1059-1150 submits engine-native Seek; this suite (executed, 21 passed) proves in-place reanchor; see F-001 |
| "the seven steps from `Playing`" | file docs lines 1-33; src/seek.rs docs | REAL | test 1 asserts every observable step: receipt gen (806-812), Buffering (797), terminal (815-818), origin (824), wire µs (839-845), clock anchor (848-849), drain with one terminal (852-857) |
| "clamping is truth (doc 05 §2.4)" past-end | file doc 15-17 | REAL | test 1048-1091: requested 40ms/actual 39ms, clean end |
| D4 rollback keeps old position playable | file doc 18-20 | REAL | tests 1097-1161, 1163-1227 |

## 7. Dead / duplicated / unreachable code
No dead code found (every helper has ≥1 use — checked by reading all call sites). Duplication: F-003. The file itself is not in the shipped binary (test lane), but exercises shipped engine code (see header table).

## 8. Dependencies & flags
Uses `testkit::Must` (panic-on-None helpers), `packet_queue::QueueLimits` (real queue core), `media_clock`, `plugin_api` Demuxer trait, `codec_api`. No feature gates. No unsafe. Runs on stable test lane; no `#[ignore]`, no cfg-gating in this file.

## 9. Open questions for the next agent
- Who owns help.rs text freshness (F-001)? The directives-audit C8/ED-FIX-4 fix replaced the mechanism but not the help string.
- MP4 `reanchor` gap (F-001 note): is there a matrix row claiming MP4 seek works? Trace in feature-matrix.toml.
- Seek from `Ended` is a typed rejection by design (state.rs:27-30) — check whether the CLI REPL surfaces that honestly (host.rs:1050-1052 mentions the doc 05 §3.3 wire edge).
