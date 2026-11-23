# AUDIT: crates/playback-engine/src/pipeline.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 3338 / 46 (incl. 10 unit tests, 2 trait impls, 3 Display/error impls) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen open`/REPL → `apps/cli/src/run.rs:198 lumen_dispatch` → `apps/cli/src/host.rs:448 CliHost::pump_pipeline` → `Engine::poll` (session.rs:1157) → `Session::tick_step` (session.rs:2428-2431 `pipeline.tick(state, &self.runtime.budget)`) → `Pipeline::tick` (pipeline.rs:1904). Desktop twin: `apps/desktop/src/shell.rs:280 pump_session` → `CliHost::pump_wallclock` → same chain. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 4 |
| Commands run | `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 pipeline::` → 10 passed, 0 failed, exit 0. `printf 'advance 200\nquit\n' | ./target/release/lumen open /home/z/my-project/samples/sample.mp4` → full 3 s play-out, `end-of-media`, `state: Ended (position 3.041814s)`, exit 0. |

## 1. One-paragraph verdict
This is a real, working tick-driven pipeline: bounded demux → per-kind bounded queues → per-limb decoders → generation-checked sink, with parking backpressure on both queue-full and consumer-not-ready, seek pre-roll hold, switch limb-swap with decoder cache, and starvation debounce. I executed its unit tests (10/10 pass) and drove a real H.264+AAC MP4 through the shipped release binary to `end-of-media` at position 3.041814 s — decode, clock advance, and end-of-track all work. The engine has NO internal thread or timer: it advances only when the host polls (`Engine::poll`), and in the pure CLI media time moves only via the `advance` verb or the desktop shell's wall-clock ticker (`pump_wallclock`). The baseline "MP4 Playing→Stopped in 19 ms" is the M17 closed-stdin graceful teardown (run.rs:293 `graceful_exit` after `Ok(0)` at run.rs:227), not a decode failure and not end-of-media. One genuine state-machine wedge exists: a seek issued when all A/V limbs are deselected never completes (F-001).

## 2. What actually works (verified)
- Unit tests: `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 pipeline::` → `test result: ok. 10 passed; 0 failed` (starve debounce, packets-axis low, decode-park backpressure, codec recycle).
- End-to-end MP4: `printf 'advance 200\nquit\n' | ./target/release/lumen open …/sample.mp4` → `event: time-tick 0.633334s (21.1%)` … `event: PlaybackEndOfTrack { track_kind: Video, id: TrackId(0) }` … `event: end-of-media`, `state: Ended (position 3.041814s)`. The h264+aac lanes decode and the audio-master clock over the null device advances exactly with host `advance` pumps.
- Bounded-budget claims in module docs verified against the loops: demux ≤ `budget.demux_events` (pipeline.rs:2140), decode ≤ `budget.packets_per_limb` (2317), flush ≤ `budget.flush_calls_per_limb` (2452); no blocking call anywhere in the tick path.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/src/pipeline.rs:1519-1521 (contrast 1653-1658)
Code:
```
        if self.limbs.is_empty() {
            return SeekPreroll::NotReady;
        }
```
What is wrong: With every A/V limb deselected (`tracks.set-audio-track {"kind":"none"}` + `tracks.set-video-track {"kind":"none"}` — both wire-reachable, crates/commands/src/catalog/types.rs:834-845 round-trips `{"kind":"none"}`; session.rs:2252 maps Deselect→`switch_limbs(..., None, ...)` which retires the limb, pipeline.rs:1745 "`new_track == None` is a deselect: retire only"), a subsequent `playback.seek` arms the seek window but `seek_preroll` answers `NotReady` forever: the `for limb in &self.limbs` loop below cannot run, and unlike `switch_preroll` (which explicitly completes with `None => SwitchPreroll::Ready { anchor_ns: None }`, "the switch completes honestly rather than hanging", 1653-1656) the seek path has no empty-limb completion. The session only completes on `SeekPreroll::Ready` (session.rs:2526-2528), so no `Seeked`/`SeekFailed` terminal is ever emitted — violating doc 05 §4.3 invariant 4 ("exactly one terminal per generation", cited at session.rs:2582-2585) — and the session sits in `Buffering` (Playing+Seek → Buffering, state.rs:523-533) with the clock frozen until the demuxer is pulled to EOF (all packets dropped at push_packet, no limbs) and `Buffering + EofReached → Playing` (state.rs:441-445) unwedges it. On a long file at 64 demux events per tick that is minutes of frozen buffering under a 30 Hz desktop pump. Why it matters: a wire-reachable command pair turns every later seek into a terminal-less wedge. What correct looks like: the `is_drained` precedent two functions up (2100-2107, "The empty-limb set counts as drained (card 3.A3: deselecting both A/V kinds …)") applied here — `limbs.is_empty()` should complete the seek `Ready { audio_anchor_ns: None }`. Confidence: HIGH — Verification: read (state machine traced session.rs:2526-2591, state.rs:441-445/523-533; not executed — no scripted harness for deselect-both+seek found in tests/; UNVERIFIED by run).

F-002 [S3]
Where: crates/playback-engine/src/pipeline.rs:1083-1087
Code:
```
                Err(_error) if wired == WiredKind::Subtitle => {
                    // The optional kind: deselect, never fail the
                    // open (the class-S seam's open-time half).
                    continue;
                }
```
What is wrong: a subtitle decoder-provider failure is swallowed whole — the typed `EngineError` is bound to `_error` and dropped; no event, no counter, no log carries WHY subtitles are absent. The deselect itself is the documented contract, but the reason is unobservable: a user whose subtitle track silently vanished cannot distinguish "container had no subtitles" from "decoder build failed". Why it matters: missing error context on a realistic path (the shipped build's subtitle support is exactly the optional kind). What correct looks like: fold the error into the wire outcome / a non-fatal `playback.error`-class fact, as decode damage already is (report.damaged, 2394-2396). Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/playback-engine/src/pipeline.rs:401-413 (with packet-queue/src/limits.rs:324-329)
Code:
```
        let high = QueueUsage {
            packets: limits.max_packets.get() - (limits.max_packets.get() / 4),
            bytes: limits.max_bytes.get() - (limits.max_bytes.get() / 4),
            duration: media_types::Nanoseconds(limits.max_duration.0 - limits.max_duration.0 / 4),
        };
```
What is wrong: `Watermark::evaluate` uses `usage.duration.0 >= marks.high.duration.0` (limits.rs:326), so a directly-constructed `QueueLimits` with `max_duration: Nanoseconds(0)` (fields are pub; the struct-literal form is used in this file's own tests, 2823-2827) yields `high.duration == 0` → the all-axes-eager `high` predicate is true on an EMPTY queue → `primed` fires with zero content and Playing starts instantly on nothing. Same degenerate family: caps 1..=3 give `high.packets == cap` (reachable but tight), cap 0 is impossible (`NonZeroUsize`). The validating constructors (`QueueLimits::new`, `audio_default`) and `from_product_limits` (1 s/2 s constants) never mint these, so the shipped paths are safe — this is a library-misuse trap, not a shipped bug. What correct looks like: `QueuePlan::new`/`with_subtitle` should revalidate the handed-in `QueueLimits` (the packet-queue constructors' own rules) or the thresholds should floor high marks at ≥1 packet and require nonzero durations. Confidence: MEDIUM — Verification: read (watermark semantics verified against limits.rs; degenerate-limits behavior not executed).

F-004 [S4]
Where: crates/playback-engine/src/pipeline.rs:2020-2027
Code:
```
            report.buffer_fill = self
                .limbs
                .iter()
                .filter(|limb| self.paces(limb))
                .map(|limb| {
                    (limb.queue.usage().packets as f32 / limb.high_packets as f32).clamp(0.0, 1.0)
                })
                .fold(1.0_f32, f32::min);
```
What is wrong: when every limb is non-pacing (all subtitles or all `first_frame_pending`), the empty iterator folds to the init `1.0` — the in-episode progress reports "full" (1.0) while `primed` is simultaneously false (`let mut primed = !self.limbs.is_empty();`, 1971, is limb-existence-based, not pacing-based). The session then emits a mid-episode `Buffering { progress: 1.0 }` (session.rs:2489-2495, gated only on `fill > last`) followed by the exit bookend 1.0 — a duplicate full-progress event describing a state that does not hold. Cosmetic observability blur, no state corruption. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/playback-engine/src/pipeline.rs:2225-2229
Code:
```
            // Stale and Closed can only strike once mints and
            // teardown race producer threads (multi-threaded Phase
            // 3+); the synchronous v0 engine never observes them
            // here, and the queue's own stats count both.
            PushOutcome::Stale { .. } | PushOutcome::Closed { .. } => {}
```
What is wrong: the packet is dropped here with no engine-side accounting; the comment's mitigation is accurate (queue.rs:395 counts `stale_rejected`, teardown closes after the flush) and the single-writer v0 really cannot observe these arms (the pipeline stamps `self.generation` immediately before push, 2147, and the queue's published generation only moves inside pipeline methods that also reset `pending`). Accepted as designed for v0, but the pipeline's `StarvationCounters` (704-724) will under-report loss if Phase 3 threads arrive and this arm is forgotten. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/playback-engine/src/pipeline.rs:448-452
Code:
```
const fn nonzero(value: usize) -> std::num::NonZeroUsize {
    match std::num::NonZeroUsize::new(value) {
        Some(n) => n,
        None => std::num::NonZeroUsize::MIN,
    }
}
```
What is wrong: a `0` argument silently becomes `1` instead of being a compile-time impossibility. All current call sites are validated literals (floor 64 / ceilings 8192/4096 in `from_product_limits`; fixed defaults), so unreachable today, but the function is `const fn` callable with any runtime-constant `0` — a future caller gets a 1-packet cap with no diagnostic. Documented in its own doc comment, hence S4 not S3. Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/playback-engine/src/pipeline.rs:1856
Code:
```
        outcome.stale_frames_flushed = self.take_seek_flush_stale();
```
What is wrong: the switch outcome's `stale_frames_flushed` is sourced from the pipeline-global `seek_flush_stale` counter, which may still hold residue from an earlier seek flush that the session never drained (`take_seek_flush_stale` is only called here and at session fold points). A switch can therefore report another flow's stale drops as its own. Counters are totals-oriented and saturating, so the sum stays true — only the per-command attribution can lie. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DecoderProvider::decoder_for` (trait) | 120-133 | REAL | — | Implemented by apps/cli/src/pipeline_host.rs `HostDecoderProvider` (mjpeg/pcm/flac/mp3/mp2/vorbis/opus/aac-lc/alac/adpcm/av1/vp9/dnxhd/h264 arms); called at 1069/1708. |
| `FrameConsumer::accept` (trait) | 147 | REAL | — | Impl: CLI `CliFrameConsumer` (pipeline_host.rs:1229), desktop bridge. |
| `FrameConsumer::ready_for` (default) | 167-170 | REAL | — | Default `true`; CLI overrides with the ring-fill gate (audio only). `let _ = kind` is the ungated-default contract, not an ignored param. |
| `QueuePlanKind::fmt` | 183-188 | REAL | — | Trivial. |
| `QueuePlanError::fmt` | 218-235 | REAL | — | Names axis + bound; matches variants. |
| `QueuePlan::new` | 284-291 | REAL | — | Subtitle/debounce defaults; no validation (F-003 family). |
| `QueuePlan::with_subtitle` | 296-307 | REAL | — | Total explicit form. |
| `QueuePlan::from_product_limits` | 325-370 | REAL | — | Reject-never-clamp verified: floor 64 / ceilings 8192/4096 checked before the `as usize` casts (358/363) — no truncation reachable. Byte/duration caps hard-coded (documented PROPOSAL). |
| `QueuePlan::for_kind` | 382-388 | REAL | — | Total over `WiredKind`. |
| `QueuePlan::thresholds` | 400-415 | REAL | S3 | F-003 degenerate-caps trap. Packets-only low degeneration verified against limits.rs:327-329. |
| `default_subtitle_limits` | 420-426 | REAL | — | 128 pkts / 16 MiB / 10 s (documented PROPOSAL figures). |
| `QueuePlan::default` | 433-442 | REAL | — | 200/300 packets via packet-queue constructors. |
| `nonzero` | 448-452 | REAL | S4 | F-006 silent 0→1. |
| `TickBudget::default` | 469-476 | REAL | — | 64/16/8; document cites PROPOSAL measurements. |
| `per_frame_damage` | 668-676 | REAL | — | Corrupt + Decoder{Bitstream} only; verified matches! arms. |
| `concealment_budget_exhausted` | 683-694 | REAL | — | Typed ResourceLimit terminal carrying the last error as source. |
| `WindowOwner::union/without/contains/is_empty/switch_for` | 827-849 | REAL | — | Bitmask set ops; `switch_for` folds non-Audio to video (Subtitle never owns a bit — class-S completes in-receipt). |
| `ObserveWindow::observe` | 903-923 | REAL | — | First-frame/first-audio latch; no allocation. |
| `ObserveWindow::first_pts_ns` | 926-931 | REAL | — | Linear find, ≤3 limbs, rare path. |
| `ObserveWindow::saw` | 935-939 | REAL | — | Linear find, bounded. |
| `ObserveWindow::held_count` | 942-946 | REAL | — | Saturating sum. |
| `ObserveWindow::hold_slot` | 953-962 | REAL | — | None outside seek pre-roll (switch-only windows hold nothing). |
| `Pipeline::new` | 999-1018 | REAL | — | No limbs; generation FIRST. |
| `Pipeline::tracks` | 1022-1024 | REAL | — | Accessor. |
| `Pipeline::wire` | 1051-1092 | REAL | S3 | One limb per kind; subtitle error swallowed (F-002); `outcome.wired==0` → session `NoDecodableTrack` (session.rs:2408-2411). |
| `Pipeline::build_limb` | 1104-1133 | REAL | — | Fresh queue published at generation; `high_packets.max(1)`; the `let _ = queue.flush` is the documented FIRST-publish no-op. |
| `Pipeline::start_demuxer` | 1141-1143 | REAL | — | Delegates to plugin-api. |
| `Pipeline::teardown` | 1153-1164 | REAL | — | Flush→close→clear; held pre-roll counted into `preroll_discarded` (M10). |
| `Pipeline::has_audio_limb` | 1170-1174 | REAL | — | Used by F5 gate in session. |
| `Pipeline::last_video_end_ns` | 1180-1185 | REAL | — | Max over limbs. |
| `Pipeline::seekability` | 1189-1191 | REAL | — | Delegates. |
| `Pipeline::duration_ns` | 1198-1203 | REAL | — | Max track span end; `stamp_ns` Option-propagated. |
| `Pipeline::seek_flush` | 1239-1355 | REAL | — | Flush queues, drop pending, reset eof + debounce state, drain+reconfigure live AND cached decoders (bounded), open M10 window. Early `Err` return documented as limping-on (D4). Verified in test `seek_mint_resets_the_debounce_state`. |
| `Pipeline::window_open_seek` | 1365-1403 | REAL | — | Re-arms entries + pre-allocated hold (`Vec::with_capacity`). |
| `Pipeline::window_arm_switch` | 1413-1455 | REAL | — | Seek-in-flight keeps its hold for the swapped limb; lone switch holds nothing. |
| `Pipeline::window_end` / `window_live` | 1459-1473 | REAL | — | Owner-bit bookkeeping. |
| `Pipeline::take_seek_flush_stale` / `take_preroll_discarded` | 1478-1488 | REAL | S4 | `mem::take` counters (F-007 attribution). |
| `Pipeline::demuxer_reanchor` | 1498-1500 | REAL | — | Delegates (D4 invariant in demuxer). |
| `Pipeline::seek_preroll` | 1512-1533 | REAL | S2 | F-001 zero-limb never-Ready wedge. `can_never_deliver` completion for EOF limbs verified. |
| `Pipeline::release_preroll_hold` | 1552-1604 | REAL | — | Merged cross-limb pts-ordered release, defense-in-depth staleness check, consumer refusal = counted discard, concealment latch by track id. |
| `Pipeline::end_seek` | 1615-1625 | REAL | — | Discards+counts residual hold; owner bit release. |
| `Pipeline::switch_preroll` | 1634-1660 | REAL | — | Deselect/EOF/no-limb all complete Ready — the exact shape `seek_preroll` lacks (F-001). |
| `Pipeline::end_switch` | 1664-1666 | REAL | — | Delegates. |
| `Pipeline::obtain_decoder` | 1679-1710 | REAL | — | Cache hit → reconfigure with parked codec (F-4 fix; test-verified); miss → provider. |
| `Pipeline::switch_limbs` | 1750-1858 | REAL | — | Prepare-before-commit verified (provider `?` precedes retire); retire parks decoder with codec; deselect path honored; pending of retired track dropped. |
| `Pipeline::take_ended_limbs` | 1865-1867 | REAL | — | Drain keeps capacity (D-1). |
| `Pipeline::can_never_deliver` | 1874-1876 | REAL | — | eof ∧ ¬pending ∧ drained ∧ empty queue. |
| `Pipeline::paces` | 1895-1900 | REAL | — | Subtitles + switch-installed limbs never pace. |
| `Pipeline::tick` | 1904-2083 | REAL | S4 | Orchestrator; decode carve-out while window live; producer-parked priming/starve escapes (1992-1999, 2049-2073) both verified against the parked-FIFO wedge they cite; buffer_fill fold (F-004). |
| `Pipeline::starvation_counters` | 2090-2092 | REAL | — | Copy out. |
| `Pipeline::is_drained` | 2100-2107 | REAL | — | Empty-limb set = drained (the F-001 fix's precedent). |
| `Pipeline::demux_step` | 2112-2188 | REAL | — | Re-offer-then-bounded-pull; re-offer refusal ends the slice (the 6.FIX data-loss fix, verified quote); stamps generation at push; Recovered/Discontinuity/Eof/Err handled typed. |
| `Pipeline::push_packet` | 2193-2231 | REAL | S4 | Unwired-track drop = selection truth; Full → pending + axis tally; Stale/Closed silent (F-005, queue counts). |
| `Pipeline::decode_step` | 2269-2533 | REAL | — | Per-limb generation floor; hold-cap + audio-ring backpressure breaks; concealment budget >1000 → typed fatal (verified quote); EOF flush gate (`queue_empty && pending.is_none()`, ring-gated for audio); slice-local sink merged back (last_frame/last_video_end_ns); first_frame_pending cleared at LOW watermark (anti-flash). |
| `stamp_ns` | 2538-2544 | REAL | — | Option-propagated, no guessing. |
| `LimbSink::observe_video_end` | 2591-2597 | REAL | — | `as i64` on u64 nanos wraps only past ~292 years. |
| `LimbSink::conceal_last` | 2615-2637 | REAL | — | Hold-owned → hold-black; stale latch → hold; repeat latched frame at damaged pts; refusal propagates as fatal per contract. |
| `LimbSink::hold_is_full` | 2641-2643 | REAL | — | No hold → false. |
| `PrerollHoldSlot::is_full` / `push` | 2657-2671 | REAL | — | Hard cap; refusal counted upstream (accept, 2696-2698). |
| `FrameSink::accept` (LimbSink) | 2675-2723 | REAL | — | Stale drop-on-sight; video end bound; hold capture; consumer hand-off `?`; first-frame latch; conceal latch on delivered frames only. |
| tests: fakes (`StalledDemuxer`,`NullDecoder`,`NullProvider`,`NullConsumer`,`GatedConsumer`,`EmittingDecoder`,`TwoCodecDemuxer`,`RecordingDecoder`,`RecordingProvider`) | 2743-3280 | REAL | — | Minimal honest fakes; queue levels hand-scripted as documented. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| packets_axis_is_the_starvation_axis | YES | Evaluates the real `Watermark::evaluate` against the plan's thresholds with bytes/duration at caps — breaks if the degeneration regresses. |
| not_ready_consumer_parks_audio_decode_and_resumes | YES | Counts real accepts through `GatedConsumer` (0 parked / 4 resumed) and real queue residency — breaks if the park probe is ignored or drops packets. |
| first_stall_of_a_fresh_episode_fires_at_one_tick | YES | Asserts `report.starved` on the exact scripted low tick + counter==1. |
| single_tick_reentry_dips_do_not_refire | YES | Asserts starve_signals==1 and starved_ticks==3 across flapping — breaks if debounce streak resets wrongly. |
| sustained_low_fires_at_the_debounce_count | YES | Fires exactly at streak 3 of 3 with counters 4/2. |
| full_recovery_closes_the_episode | YES | Post-high dip fires immediately (fresh episode). |
| seek_mint_resets_the_debounce_state | YES | Reads `low_streak`/`episode_open` post `seek_flush`. |
| debounce_one_is_the_undebounced_behavior | YES | Every low tick fires (3 signals). |
| unknown_duration_mix_reaches_low | YES | Duration-less packets still fire the packets-axis low. |
| parked_codec_rides_the_recycle_path | YES | RecordingDecoder log must equal exactly `["fake-pcm-b","fake-pcm-a"]` — breaks if the recycle fabricates a descriptor (the retired `unreachable!` fallback would have panicked). |
All 10 executed by me (see Commands run) — none tautological, none `#[ignore]`, none asserting on mocks of the unit under test (fakes sit at the plugin seams by design).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v0 is synchronous and thread-free: the pipeline advances only when the session polls it, in bounded slices" | pipeline.rs:32-35 module docs | REAL | No thread/timer/sleep in crate; `Engine::poll` is the only driver (session.rs:1157); budgets bound every loop (2140/2317/2452). |
| "a full queue hands the packet back and the engine *holds* it (`pending`) for the next tick" | pipeline.rs:36-38 | REAL | push_packet Full arm (2223) + demux_step re-offer (2122-2133). |
| "generation-stamped at push" | pipeline.rs:14-15 | REAL | `packet.generation = self.generation;` (2147) before `try_push`. |
| "backpressure modulates state, generations modulate data — the mint-only flush rule holds everywhere in this file" | pipeline.rs:96-98 | REAL | Grep of the file: every `queue.flush(` call site (1117, 1158, 1255, 1784) is a mint boundary (wire/teardown/seek_flush/switch retire); none in tick paths. |
| "at most budget.demux_events … packets_per_limb … flush_calls_per_limb per tick" | pipeline.rs:33-36 | REAL | Loop bounds verified (2140, 2317, 2452). |
| "the decode→engine frame queue stage is deliberately absent" | pipeline.rs:39-43 | REAL | Frames flow sink-direct (2675-2723); no frame queue type used. |
| Empty-limb drains honestly | pipeline.rs:2096-2099 (is_drained docs) | REAL for drain, FALSE-ADJACENT for seek | is_drained treats empty as drained; but seek_preroll (1519-1521) treats the same empty set as never-ready — F-001. |

## 7. Dead / duplicated / unreachable code
- `QueuePlan::with_subtitle` (296-307): no production caller found (rg in crates/ apps/ shows only tests + the constructor); the CLI/desktop wire one lane each via `from_product_limits`/`Default`. UNREACHABLE-in-product, exercised by tests/queue_policy.rs presumably — flagged, not a defect.
- `StarvationCounters.full_rejections_bytes/_duration` (721-723): tallied (2211-2218) but unreachable-while-binding in the engine's own threshold scheme (low bytes/duration sit at caps by design, 406-413) — the tallies exist for the packet-queue's independent default thresholds; observability-only.
- No `todo!`/`unimplemented!`/`unreachable!` in this file (rg verified).

## 8. Dependencies & flags
- Uses `codec_api` (Decoder/FrameSink/DecodeOutcome/DecoderConfig/ThreadBudget), `media_types` (Packet/Frame/Stamp/GenerationId/EngineError/ErrorKind), `packet_queue` (BoundedQueue/Stamped/PushOutcome/PopOutcome/QueueLimits/QueueUsage/WatermarkThresholds/LimitAxis), `plugin_api` (Demuxer/DemuxEvent/SeekSupport). No feature gates inside this file; the crate builds in the release lane (baseline cargo-build-release-lumen-cli.txt exit 0).
- `crate::seek::nanos_i64`, `crate::state::SessionState`, `crate::tracks::WiredKind` — intra-crate, no layering violation (engine-internal).

## 9. Open questions for the next agent
- Execute F-001 (deselect-both + seek) as a scripted session — I found no existing test covering a seek with zero limbs; the wedge is traced but not run (blocked: no cheap harness without writing a new test, which is a source change).
- session.rs owns the tick signal translation and the F5 play-out gate (`last_video_end_ns` consumer) — verify its `Drained` translation cannot bypass the gate on audio-bearing sessions.
- The `#[PROPOSAL]` starve-debounce default (2) and subtitle caps await config wiring — check product.toml keys actually read (config-validator baseline shows 4 missing brand assets, unrelated).
- decode-bench's "no decoder in this build (first vertical: mjpeg + pcm)" (apps/cli/src/bench.rs:334) is false for this build — my `lumen open` run decoded h264+aac to end-of-media. Whoever owns bench.rs should account for the stale copy.
