# AUDIT: crates/playback-engine/src/session.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 3058 / ~62 (incl. 5 unit tests) |
| File verdict | REAL (host-driven executor: no session thread exists; disclosed deferral) |
| Reachable from shipped entry point? | YES — `lumen` main → run.rs → `CliHost::open_session` (apps/cli/src/host.rs:983 `self.engine.open(config)`) → `CliHost::pump_pipeline` (host.rs:448 `self.engine.poll()`) → session.rs `Engine::open`/`Engine::poll` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `cargo test -p playback-engine --lib session::` → 5 passed/0 failed; `./target/release/lumen open samples/sample.mp4 </dev/null` → exit 0, full state walk reproduced (see §2) |

## 1. One-paragraph verdict
The session loop is real: `Engine::poll` (1157–1206) drains the bounded command inbox, then advances the pipeline by one bounded tick (`open_step` in Opening, `tick_step` in Buffering/Playing/Paused), so the executor polls MEDIA, not just commands — `tick_step` → `Pipeline::tick` → `demux_step`/`decode_step` (pipeline.rs:1904/2112/2269, budget-bounded). But there is NO session thread, timer, or self-scheduling anywhere: the module doc admits "the thread-per-session spawn, the demux and decode workers, and the event broadcast bus land with cards 2.E1/3.B3. Until then the host's poll loop *is* the session thread" (session.rs:14–17) — a host that stops polling freezes playback. For the 19ms MP4 litmus: a track whose decoder provider refuses fails the WHOLE session (`open_step` → `Some(Err(error)) => self.fail_with(error)` at 2412; `wired == 0` → `NoDecodableTrack` at 2394–2411) — that is the HEVC exit-65 path; the h264+aac sample instead wired REAL decoders (apps/cli/src/pipeline_host.rs:346 `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)`; rust_h264-0.6.1 verified inside the shipped binary via `strings`), and its Playing→Stopped at ~19ms is the CLI's stdin-EOF teardown (run.rs M17 `graceful_exit`: stop→close), NOT end-of-media — no `end-of-media` event is emitted. The baseline's "zero decode" comes from decode-bench's own stale hardcoded allowlist (apps/cli/src/bench.rs:300–334 "no decoder in this build (first vertical: mjpeg + pcm)"), not from session behavior. One real defect found (subtitle selection lies on provider failure, F-001).

## 2. What actually works (verified)
- `cargo test -p playback-engine --lib session::` → `test result: ok. 5 passed; 0 failed` (OutputHub bounds/tick-replacement/receipt-pinning + inbox busy — all assert counters and drain contents, not tautologies).
- `timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample.mp4 </dev/null` → exit 0; reproduced baseline exactly: `Idle→Opening`, `tracks 2 ready`, `buffering 0.00/0.02/1.00`, `Buffering→Playing`, `Playing→Stopped`, `Stopped→Idle` — real demux (queue-fill progression) and real decoder wiring; NO `end-of-media` and NO `first-frame` event, proving the stop is host teardown (stdin EOF), not media completion.
- `strings target/release/lumen | rg rust_h264` → `rust_h264-0.6.1` compiled into the shipped release binary ⇒ engine-info's "decoders: mjpeg, pcm_s16le" (host.rs:1649–1664, hardcoded) and decode-bench's refusal are stale lists that lie about the shipped build.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/src/session.rs:2278–2330 (with quote sites 2269–2271, 2278–2286, 2309–2315)
Code:
```
        // A provider that cannot build the subtitle decoder deselects
        // honestly (the optional kind — never a failure).
```
```
            if let Ok(outcome) = pipeline.switch_limbs(
```
```
        let outputs = vec![self.emit(
            EngineEventKind::TrackSelectionChanged {
                track_kind: MediaKind::Subtitle,
                selected,
            },
            Some(command.request),
        )];
```
What is wrong: a subtitle selection whose `switch_limbs` returns Err is silently skipped (`if let Ok`), yet the function still emits `TrackSelectionChanged { selected }` with the REQUESTED id and answers `ReceiptResult::Completed`. The session's own mirror (`sync_selected_mirror`, called at 2298) still holds the OLD selection — the event and receipt contradict the engine's actual state. Why it matters: a host keying off `tracks.changed`/the Completed receipt shows a subtitle track as selected while no decoder limb exists and no subtitles will ever render; the "deselects honestly" comment describes the opposite of what the event says (an honest deselect would emit `selected: None`). What correct looks like: on Err, either emit the mirror's truth (`selected: None` / old id) or surface a typed rejection; the receipt must not claim Completed for work that did not land. Confidence: HIGH — Verification: read (control flow traced line by line; mutation reasoning: flipping the provider to refuse makes the event false with no test failing — no test covers a failing subtitle provider here).
F-002 [S3]
Where: crates/playback-engine/src/session.rs:1504–1510 (doc claim) vs crates/playback-engine/src/state.rs:525–533 and session.rs:1985–2095 (code)
Code:
```
    /// The two can diverge legitimately: a follow-up command that
    /// fails *resolution* (an unresolvable seek target, a track id
    /// that is not live) still mints nothing — but a follow-up mint
```
vs state.rs:525–533:
```
        input: SessionInput::Seek,
        to: SessionState::Buffering,
        effects: &[
            Effect::FreezeClock,
            Effect::BumpPipeline,
            Effect::SeekPipeline,
```
What is wrong: the doc claims an unresolvable seek "mints nothing"; the row mints (BumpPipeline) BEFORE `SeekPipeline` resolves, and resolution failure is reachable (`SeekTarget::Percent` with unknown duration → `Failure(OutOfRange)`, any `SeekTarget::Chapter` → always Failure — seek.rs:291–302). A failed-resolution seek from Playing therefore mints a fresh generation, freezes the clock, emits a spurious `buffering 0.0` episode, lands in Buffering, and re-primes next tick (Primed row resumes the clock). Recoverable, but every Chapter seek and every percent seek on a live/unknown-duration stream burns a generation and emits a fake buffering episode. What correct looks like: resolve the target pre-table (like track selectors do in `apply_command`) so a bad target never mints. Confidence: HIGH — Verification: read (effect order and Failure arms traced; not executed).
F-003 [S3]
Where: crates/playback-engine/src/session.rs:1648–1654; state.rs:516/632 (EmitEndOfMedia rows)
Code:
```
                Effect::FreezeClock | Effect::StopClock => {
                    // Underrun/pause entry freezes; Ended freezes at
                    // the end position (v0: pause — see the effect's
                    // docs).
                    self.runtime.clock.pause();
```
What is wrong: `StopClock` ("Ended freezes at the end position") is implemented as a bare `clock.pause()` — the clock stays at whatever position it held, never clamped to the media end. Disclosed in the effect's doc ("v0 pauses — a distinct 'stopped at end' anchor state is 2.C3/3.B2 work"), so this is a documented deferral, but any consumer relying on the frozen position after `Ended` gets the pre-end position whenever EOF and play-out race (mitigated by the F5 `video_played_out` gate, 2740–2751). Why it matters: the ended-position invariant doc 03 promises is not upheld by code; drift is possible for audio-limb sessions whose ring gate is device-dependent. Confidence: MEDIUM — Verification: read.
F-004 [S4]
Where: crates/playback-engine/src/session.rs:1061–1098 (`Engine::open`)
What is wrong: `next_epoch` is incremented (epoch consumed) before the fresh `Session::new`/`apply(Open)`; if the (unreachable-by-construction) `Idle → Opening` apply ever rejected, the epoch is burned with no session minted — `GenerationExhausted` arrives one open early. Blast radius: nil today (pinned row). Confidence: LOW — Verification: read.
F-005 [S4]
Where: crates/playback-engine/src/session.rs:1–17 (module docs)
What is wrong: doc 03 §4.6(c)/§9's session thread, demux/decode workers, and event broadcast bus do not exist in this file (or crate); the doc comment honestly discloses the deferral, but every "session thread" invariant in docs 03/05 is currently upheld vacuously (there is no second writer to exclude — single-writer "by construction"). Hostile-question answer recorded: the single-writer executor is a synchronous library object stepped by the host's loop (`CliHost::pump_pipeline`), not a thread. Confidence: HIGH — Verification: read + executed the host-driven loop end to end (§2).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SessionCommandKind::as_command_id` | 159–171 | REAL | — | 1:1 wire-id mapping |
| `SessionCommandKind::as_input` | 174–195 | REAL | — | subtitle maps to SwitchTrack but never reaches table (pre-table branch) |
| `SessionCommandKind::selection_kind` | 197–208 | REAL | — | lane-typed, no silent audio fallback |
| `ReceiptResult::accepted_generation` | 257–263 | REAL | — | trivial accessor |
| `SessionHandle::session_id` | 307–309 | REAL | — | |
| `SessionHandle::submit/play/pause/stop/close` | 320–365 | REAL | — | thin queue wrappers |
| `SessionHandle::seek` | 368–380 | REAL | — | payload beside command |
| `SessionHandle::set_video_track/set_audio_track/set_subtitle_track` | 383–436 | REAL | — | |
| `SessionHandle::queue` | 437–443 | REAL | — | try_send → typed InboxBusy/SessionClosed; never blocks/drops |
| `SessionConfig::new` | 487–504 | REAL | — | everything injectable; defaults |
| `OutputHub::new` | 571–585 | REAL | — | capacity.max(1) |
| `OutputHub::push` | 599–644 | REAL | — | receipt/event lanes, tick replacement, capacity drop; cursors maintained correctly on all traced paths |
| `OutputHub::drop_oldest_event/drop_oldest_receipt` | 646–666 | REAL | — | tombstone + cursor advance |
| `OutputHub::kill` | 668–670 | REAL | — | |
| `OutputHub::next_event_after/next_receipt_after` | 674–691 | REAL | — | forward-only scans, amortized O(1) claim holds between drains/compactions |
| `OutputHub::compact_if_due` | 696–738 | REAL | — | in-place retain, cursors + resident_tick recomputed; dead ≥ live trigger |
| `OutputHub::drain_into` | 740–770 | REAL | — | stale suppression (epoch/pipeline pair), receipts survive, counters honest |
| `PipelineTotals::merge_tick` | 860–872 | REAL | — | saturating adds |
| `PipelineTotals::absorb_starvation` | 881–884 | REAL | — | assign-not-add; no double count at close (verified merge chain) |
| `PipelineTotals::merge` | 886–920 | REAL | — | |
| `PipelineTotals::fold_preroll_discards` | 931–937 | REAL | — | corrects frames_delivered down |
| `Engine::fmt` (Debug) | 968–976 | REAL | — | manual, observable surface only |
| `Engine::new/with_outbox_capacity` | 981–998 | REAL | — | |
| `Engine::open` | 1061–1098 | REAL | — | replacement flow R1–R9; epoch burn before apply = F-004 |
| `Engine::state/session_id/generation/cancel_token` | 1099–1122 | REAL | — | accessors |
| `Engine::stats` | 1126–1155 | REAL | — | merges live session totals on the fly |
| `Engine::poll` | 1157–1200 | REAL | — | THE loop: inbox drain → one bounded tick → closed-session destroy; HQ1/HQ2 core |
| `Engine::drain_outputs_into/take_outputs` | 1202–1219 | REAL | — | reuse API + owned form |
| `Engine::current_pair` | 1220–1222 | REAL | — | |
| `Engine::replace_live_session` | 1245–1277 | REAL | — | close_plan composition, totals merged before drop |
| `Engine::default` | 1280–1282 | REAL | — | |
| `SessionError::generation_exhausted_boxed` | 1286–1296 | REAL | — | typed i64-exhaustion corner |
| `Session::new` | 1420–1464 | REAL | — | pipeline constructed, runtime split out |
| `Session::pair/emit/emit_hook/emit_raw` | 1466–1516 | REAL | — | seq fetch_add ordering; borrow-split form |
| `Session::emit_terminal` | 1518–1540 | REAL | — | doc claim false for seek resolution (F-002) |
| `Session::apply` | 1544–1737 | REAL | — | table lookup + effect interpreter; pause-entry tick, buffering bookends, damage flush, C8 confirmations — all traced |
| `Session::flush_damage` | 1739–1753 | REAL | — | |
| `Session::position_fraction` | 1765–1776 | REAL | — | 0.0 floor on unknown duration |
| `Session::apply_command` | 1784–1960 | REAL | — | payload guards, class-S subtitle, close composition with generation restamp (documented deviation) |
| `Session::seek_pipeline_step` | 1985–2096 | REAL | — | resolution → capability → pipeline_seek; D4 rollback path; M10 preroll accounting |
| `Session::fold_preroll_discards` | 2098–2104 | REAL | — | |
| `Session::absorb_pipeline_starvation` | 2114–2121 | REAL | — | |
| `Session::switch_pipeline_step` | 2132–2159 | REAL | — | audio freeze w/ deselect exemption (F5/#16), deferred_fatal for provider failures |
| `Session::subtitle_select_step` | 2221–2329 | REAL w/ defect | S2 | F-001: lying event + Completed on provider Err |
| `Session::sync_selected_mirror` | 2331–2339 | REAL | — | |
| `Session::open_step` | 2342–2426 | REAL | — | once-only probe; wired>0 / wired==0 (NoDecodableTrack) / Err (fail_with) — the no-decoder answer (HQ3) |
| `Session::tick_step` | 2428–2672 | REAL | — | one bounded tick, side events, buffering window, seek/switch completion, one translated signal, time-tick throttle |
| `Session::complete_switch_kind` | 2674–2738 | REAL | — | re-anchor failure → fatal path; terminal under switch's own generation |
| `Session::video_played_out` | 2740–2751 | REAL | — | F5 gate; audio exempt |
| `Session::inflight_switch_of/take_inflight_switch_of` | 2752–2768 | REAL | — | |
| `Session::emit_open_facts` | 2771–2796 | REAL | — | once-per-open duration + info-ready |
| `Session::fail_with` | 2798–2807 | REAL | — | Failed input with typed payload |
| `clock_position_ns` | 2817–2824 | REAL | — | 0 floor on i64 exhaustion |
| `internal_missing_fatal/internal_missing_pipeline` | 2826–2853 | REAL | — | typed stand-ins, never panic |
| `transition_table` | 2856–2858 | REAL | — | test exposure; not called in prod (see §7) |
| tests: `the_hub_stays_bounded…`, `a_newer_tick_replaces…`, `capacity_pressure_may_drop…`, `receipts_are_pinned…`, `the_inbox_is_bounded…` | 2922–3058 | REAL | — | executed: 5/5 pass; would fail on broken hub (counter + drain-content asserts) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `the_hub_stays_bounded_and_counts_every_drop_at_capacity` | YES | asserts live bound, exact drop count (10_000−8), slot-count ≤ 2×live, and survivor seq identity |
| `a_newer_tick_replaces_the_undelivered_older_one` | YES | asserts replacement counter, live count, drained order, and post-drain resident state |
| `capacity_pressure_may_drop_the_resident_tick_and_counts_it` | YES | asserts drop count + resident cursor death |
| `receipts_are_pinned_from_events_but_hard_capped` | YES | asserts 16 event drops, 4 pinned receipts, oldest-receipt drop, interleave order |
| `the_inbox_is_bounded_and_answers_typed_busy` | YES | asserts typed InboxBusy at bound, order preserved, room after drain |
Coverage gap: no unit test for the subtitle provider-failure path (F-001) or for seek resolution failure (F-002).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Doc 03 gives the session its own thread with a command inbox and an event bus" | session.rs:7–8 | PAPER-ONLY at this layer (self-disclosed deferral to cards 2.E1/3.B3) | no `thread::`/spawn anywhere in crate; host.rs:448 `engine.poll()` is the driver (executed, §2) |
| "poll drains the inbox first … then advances the pipeline by one bounded tick" | session.rs:21–24 | REAL | poll body 1164–1198; executed via CLI run |
| "a follow-up command that fails resolution (an unresolvable seek target …) still mints nothing" | session.rs:1506–1507 | FALSE for seek | state.rs:525–533 BumpPipeline precedes SeekPipeline; seek.rs:295/301 Failure reachable (F-002) |
| "Nothing decodable: the engine refuses to fabricate playback (typed config failure)" | session.rs:2395–2396 | REAL | wired==0 → NoDecodableTrack → fail_with; provider refusal → Err arm 2412 (HEVC exit-65 baseline) |
| "The engine cannot learn [container] from the Demuxer trait" / container consumed by NoDecodableTrack | session.rs:455–460 | REAL | error enrichment path 2399–2408 |
| "receipts are never dropped for event overflow" | OUTBOX doc, session.rs:82–84 | REAL | push 599–625 + unit test executed |

## 7. Dead / duplicated / unreachable code
- `transition_table()` (2856): pub API exposed "for the exhaustive tests" — not called by any shipped entry point (test/model-walk surface only). Intentional, harmless.
- `EngineStats.stale_frames_dropped`: doc itself says "Zero by construction in v0's single-writer ticks" — a counter that cannot increment in the shipped threading model; honest checkpoint for the threaded future.
- No dead branches found in OutputHub; all arms traced to live behavior.

## 8. Dependencies & flags
- Deps: media-clock, media-types, plugin-api, commands (ids/envelope), crate-local epoch/error/event/events/pipeline/seek/state/tracks. No new external deps; no feature gates inside this file. Reachability of the crate's real pipeline in the shipped lane proven via apps/cli (default features of `codecs` include `codec-h264-sw` etc. — codecs/Cargo.toml:44–56 — so the release lane ships the full decoder set; engine-info's 2-decoder list is stale, see §1/§2).
- `#[must_use]` (15 sites) verified present — the display artifact `#ust_use]` in tooling output is an ANSI-strip artifact, not file content (verified `rg -c must_use` → 15, `cat -A`).

## 9. Open questions for the next agent
- Does any host rely on `TrackSelectionChanged` for subtitles to decide UI selected-state (F-001 blast radius)? Check ui/ state-mapping and apps/desktop.
- pipeline.rs `decode_step` (2269–2317): confirm the paused-state decode behavior (decode while Paused — does it present?) — I did not audit pipeline.rs (not assigned).
- engine-info/decode-bench stale decoder lists (host.rs:1649, bench.rs:334) — belongs to whichever agent owns apps/cli/src/host.rs and bench.rs; the "landed:" list at pipeline_host.rs:375–381 also omits vp9/dnxhd which decoder_for does wire.
