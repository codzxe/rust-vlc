# AUDIT: crates/video-engine/src/scheduler.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 1711 / 40 (27 production items + 13 test items; 19 in-file tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen-cli`: `run.rs` REPL/machine mode → `CliHost::pump/advance_time` (apps/cli/src/host.rs:703,721) → `ConsumerState::tick_video` (apps/cli/src/pipeline_host.rs:1010-1016) → `VideoLimb::tick_once` (pipeline_host.rs:768-769) → `PresentationScheduler::tick` (this file:701). Construction: `accept_video` (pipeline_host.rs:1295-1334) on the first decoded video frame. Desktop lane: `shell.rs:279-282 pump_session → CliHost::pump_wallclock` (wall-bounded). Verified by running the release binary and the lib tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p video-engine --lib scheduler` -> 19 passed, 0 failed (exit 0); `cargo test -p lumen-cli --test pipeline_accounting` -> 3 passed (exit 0); `printf 'advance 200\nquit\n' | target/release/lumen open samples/sample.mp4` -> full state walk to `Ended`, `PlaybackEndOfTrack Video`, position 3.041814s (exit 0) |

## 1. One-paragraph verdict
This is a genuine, carefully engineered frame scheduler — the hostile questions land on the *wiring around it*, not on this file. `PresentationScheduler::push/tick` implements exactly the documented algorithm (generation purge → held-frame admission → late purge with pre-roll exemption → largest-due selection with structural supersede → bounded repeat → background), all arithmetic is i128-widened and saturating, all refusals hand the frame back, all drops are counted and trail-recorded in fixed-capacity inline arrays, and 19 in-file unit tests plus 28 tests in video-engine/tests (21 scheduling, 5 sync, 2 zero-alloc) pin the behavior. The MP4/no-decode question is answered by the wiring: the scheduler is constructed lazily on the first decoded frame, so when nothing decodes there is no scheduler at all — nothing is being faked. The rc-smoke 60/60 "pacing" is NOT this file: `lumen decode-bench` decodes to a null sink (bench.rs:94) and never touches the scheduler; the scheduler-paced proof of the same vertical is `pipeline_accounting.rs::whole_media_accounting_closes` (ran live: scheduler_pushed=60, dropped_late=0, presented+repeats≥60, position exactly 2.000s).

## 2. What actually works (verified)
— verified by `cargo test -p video-engine --lib scheduler` -> `test result: ok. 19 passed; 0 failed` (order, lateness, supersede, repeat-budget, generation purge, missing-PTS inference/hold, window bound, unschedulable PTS, base normalization, drop-trail truncation, both storm classes, all three pre-roll-hold scenarios).
— verified by `cargo test -p lumen-cli --test pipeline_accounting` -> 3 passed, including `whole_media_accounting_closes`: the AVI/MJPEG/PCM vertical driven through the one command layer reaches the scheduler — `assert_eq!(accounting.scheduler_pushed, 60)`, `assert_eq!(accounting.scheduler_dropped_late, 0)`, staging bytes `60 * 320 * 240 * 4`, final position exactly `2_000_000` µs.
— verified by `printf 'advance 200\nquit\n' | ./target/release/lumen open samples/sample.mp4` -> `event: PlaybackEndOfTrack { track_kind: Video }` … `event: end-of-media` … `state: Ended (position 3.041814s)`: the H.264 MP4 **does** decode in the `open` path (pipeline_host.rs:346 wires `H264SwDecoder`), the video limb is built and ticked to the end of media — the 19 ms MP4 smoke in the baseline is stdin-EOF teardown, not missing decode.

## 3. Findings
F-001 [S3]
Where: apps/cli/src/bench.rs:330-336 (cross-file evidence; bears on every "scheduler paces nothing" reading of the MP4 smoke)
Code:
```
                } else {
                    "no decoder in this build (first vertical: mjpeg + pcm)"
                };
```
What is wrong: The decode-bench registry knows only mjpeg (video) and pcm (audio) (bench.rs:300-304), and its refusal message asserts "no decoder in this build". That claim is false for this build: the `open` path wires h264, aac, flac, mp3, mp2, vorbis, opus, alac, adpcm, av1, vp9, dnxhd (pipeline_host.rs:241-370). The baseline MP4 smoke therefore under-reports: decode-bench says `v_frames=0` "no decoder", while `lumen open` on the same MP4 decodes the video track to `PlaybackEndOfTrack` (my run above). Two registries, contradictory user-facing claims, same binary.
Why it matters: The litmus baseline ("MP4 case: nothing decodes") is an artifact of bench.rs's stub registry, not of the build; any coverage or parity conclusion drawn from decode-bench is wrong for every codec past mjpeg/pcm.
What correct looks like: decode-bench dispatches through `HostDecoderProvider` (the same registry as `open`), or its message says "not benched (bench registry covers mjpeg+pcm only)".
Confidence: HIGH — Verification: ran both paths on samples/sample.mp4; read both registries.

F-002 [S4]
Where: crates/video-engine/src/scheduler.rs:874-880
Code:
```
        if self
            .sync
            .tick_index
            .is_multiple_of(u64::from(self.cfg.sync.decide_every))
        {
            self.apply_sync_ladder(band);
        }
```
What is wrong: `decide_every: u32` is a pub field set without validation (`SchedulerCfg::with_sync`, policy.rs:313, no checks). With `decide_every = 0`, `is_multiple_of(0)` returns `false` (verified empirically: `16u64.is_multiple_of(0) == false`, no panic), so the 3.B1 ladder silently never applies — no widening, no repeat extension, ever — with no counter and no error.
Why it matters: A host misconfiguration silently disables the A/V-sync actuator path; the module's "bounded churn, deterministic" story has an unmarked off switch.
What correct looks like: `with_sync` rejects `decide_every == 0` (and zero windows) typed, like `SchedulerCfg::new` rejects zero frame durations.
Confidence: HIGH — Verification: read + ran an is_multiple_of(0) probe (returns false, no panic).

F-003 [S4]
Where: crates/video-engine/src/scheduler.rs:318-322
Code:
```
    /// The earliest clock position at which the next pending frame
    /// becomes due (`None` when nothing is pending) — the sleep target
    /// for a timer-driven wiring (doc 01 §2.2 `WaitUntil`).
    pub next_due: Option<Timestamp>,
```
What is wrong: A frame sitting in the `held_missing` slot is pending (it will present once inference becomes possible) but contributes nothing to `next_due` — with an empty window and a held frame, `next_due` is `None` and the doc's "None when nothing is pending" is wrong. In practice inference depends on a future push, not on clock position, so no correct finite hint exists — the doc just overstates.
Why it matters: A timer-driven wiring sleeping on `next_due == None` would still be correct (wakes on pushes), but the field's contract text is inaccurate.
What correct looks like: Doc: "`None` when no windowed frame is pending (a held missing-stamp frame waits on a push, not on time)".
Confidence: HIGH — Verification: read (held_missing excluded from the front()-only computation at lines 894-898).

F-004 [S4]
Where: crates/video-engine/src/scheduler.rs:629-642
Code:
```
                    let continuation = self
                        .window
                        .back()
                        .map(|tail| tail.pts.ticks)
                        .or_else(|| self.last.as_ref().map(|request| request.when.ticks));
```
What is wrong: The PTS-gap detector's continuation reference ignores frame generations: a pre-roll frame of a not-yet-published seek generation sitting at the window tail (a backward seek's landing, pts far below the old stream's continuation) makes the NEXT push's forward gap measured against the seek landing — arming or failing to arm snap on the wrong baseline. The anchor-epoch seam (line 714-724) re-arms snap on the seek anyway, so the error is masked for real seeks; only a pathological interleaving of generations with no re-anchor differs.
Why it matters: Dormant ordering subtlety in a discontinuity detector; masked today by tier 3.
What correct looks like: Compare within the same generation (continuation = tail of the frame's own generation, else last presented of that generation).
Confidence: MEDIUM — Verification: read; no failing scenario constructed (masked by the anchor-epoch path in every seek shape the tests drive).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PushOutcome (enum) | 96-148 | REAL | — | Every refusal variant hands the frame back — no silent swallowing (verified in all 7 arms). |
| PresentAction (enum) | 151-165 | REAL | — | Present/RepeatLast/Background; consumed by VideoLimb::tick_once (pipeline_host.rs:769-774). |
| MAX_DROP_TRAIL / DropTrail | 178-286 | REAL | — | Fixed-capacity inline trail; len/recorded/truncated semantics exact; equality ignores fill; `dropped_storm` tests pin truncation. |
| TickDrops / TickOutcome / SyncSample (+Default) | 295-367 | REAL | — | F-003 (next_due doc gap lives in TickOutcome). |
| SchedulerStats | 371-416 | REAL | — | 20 counters; `sync_error_sum` i128 saturating (line 835-836) — overflow-proof. |
| Pending / PresentationScheduler (state) | 419-448 | REAL | — | Bounded window + single hold slot + last/repeat budget. |
| SchedulerSync::new/reset_estimators/note_late_drop/rotate_windows | 487-539 | REAL | — | Tumbling O(1) windows; saturating adds. |
| Debug for PresentationScheduler | 542-556 | REAL | — | Honest: elides the clock handle, `finish_non_exhaustive`. |
| PresentationScheduler::new | 564-577 | REAL | — | Reads live generation from the clock snapshot. |
| push | 583-679 | REAL | — | Stale-boundary (strictly-older), checked NANO_BASE conversion, window bound, PTS-gap snap arming (F-004), missing-stamp infer/hold/replace. |
| tick | 701-905 | REAL | — | The binding order exactly as documented; purge prefix w/ pre-roll break; supersede drain unconditional (DropPolicy::Never included — verified by test at 1531-1571); i128 due math; EMA sample on Running presents only; ladder at decide_every cadence (F-002). |
| effective_late_threshold / effective_max_repeat | 909-923 | REAL | — | Widening never narrows configured; saturating repeat extension. |
| apply_sync_ladder | 932-980 | REAL | — | WidenLate guards: Never policy, no frame duration, no EMA, drop budget, rate limit — all five bounds present and ordered. |
| cfg/stats/pending_len/holds_missing/last_presented/current_generation | 983-1016 | REAL | — | Accessors. |
| sequence_base | 1021-1026 | REAL | — | Window tail else last presented. |
| admit_held | 1031-1049 | REAL | — | Re-admission retry; drop-and-count on full window; re-hold when still uninferable. |
| insert_sorted | 1054-1063 | REAL | — | O(window≤max_pending) linear insert, stable for equals. |
| saturating_add | 1068-1070 | REAL | — | i128 clamp — correct. |
| is_late | 1078-1083 | REAL | — | Strict `<`, i128 widening, None ⇒ Never policy. |
| tests::cfg + 19 tests | 1092-1711 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| presents_in_order_under_a_normally_advancing_clock | YES | 50 frames, per-slot present, all drop counters zero — any misordering/leak shows. |
| early_frames_are_held_until_their_slot | YES | Background until `100 − lead 5`, then present at 95; next_due pinned to 95. |
| late_frames_are_dropped_counted_never_presented | YES | 10/10 late at now=100/threshold 10; dropped frames never resurrect. |
| the_largest_due_frame_wins_and_supersedes | YES | Supersede trail asserts the exact dropped PTS. |
| out_of_order_arrivals_present_in_pts_order | YES | Shuffled arrival → non-decreasing presents, 1 supersede. |
| repeat_budget_then_background | YES | Exactly 2 repeats then background; fresh present resets budget. |
| generation_change_purges_the_window_and_kills_repeats | YES | 3 stale drops, repeat source invalidated, Stale push boundary. |
| missing_pts_is_inferred… | YES | Inferred at last+dur; frame's own `Stamp::Missing` untouched. |
| uninferable_missing_pts_is_held… | YES | Hold with no frame_duration; replacement drops the older, counted. |
| window_bound_is_enforced… | YES | 9th push refused with the frame handed back; drain then re-accept. |
| unrepresentable_pts_is_unschedulable… | YES | i64::MAX ticks on 1/1 base → TimeError::Overflow verbatim. |
| stream_base_pts_are_normalized… | YES | 1/90_000 base → ns normalization before scheduling. |
| drop_trail_records_to_capacity_then_counts_only | YES | Cap, exact count, truncation, equality semantics. |
| drop_trail_debug_shows_only_retained_state | YES | Debug prints retained prefix only. |
| late_storm…/superseded_storm… | YES | 40-drop storm: recorded=40, len=16, onset prefix exact; second storm structural. |
| preroll_holds_are_not_judged… / …present_in_pts_order_once… / …discarded_when_a_later_generation_publishes | YES | The M10 triple: no present/no purge/no wait-hint against the frozen position; PTS-order presents after publication; wholesale discard on a superseded generation. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one vsync-paced scheduler thread in production… v0 delivers the deterministic core" | module doc, lines 5-12 | REAL-UNPROVEN (the thread) / REAL (the core) | Core: 19 lib tests + pipeline_accounting (ran). Thread: no host drives per-vsync; CLI drives per-quantum (host.rs:715), desktop per-wall-tick (shell.rs:279-282) — doc itself defers the thread to "Phase 3", honest. |
| "the late frames are a prefix" (sorted window) | module doc, lines 21-23 | REAL | Window is PTS-sorted (insert_sorted); purge is front-only (lines 775-788). |
| "the structural drop applies under every policy, DropPolicy::Never included" | module doc, lines 29-32 | REAL | Supersede drain (807-810) is policy-independent; proven by superseded_storm test under `DropPolicy::Never` (1536-1541). |
| "Missing presentation timestamps are never invented silently" | module doc, lines 39-42 | REAL | `Inferred` outcome + `inferred_pts` counter (tests at 1312-1335). |
| "drives nothing itself: the wiring enqueues the action's request" | tick doc, lines 681-685 | REAL | VideoLimb::tick_once does the enqueue+drain (pipeline_host.rs:768-823). |
| "Zero heap traffic (D-1/D-8)… measured by tests/zero_alloc_scheduler.rs" | tick doc, lines 693-694 | REAL-UNPROVEN (here) | File exists (2 tests, counting global allocator); not my assigned file — cited, not re-run. |
| rc-smoke "headless-decode … 60/60 frames" paced by this scheduler | task prompt / rc-smoke.txt:130 | FALSE (as pacing evidence) | decode-bench path never constructs a scheduler (bench.rs decode-to-null sink, bench.rs:94); the paced 60/60 lives in pipeline_accounting.rs (ran, PASS). |
| "no decoder in this build (first vertical: mjpeg + pcm)" | bench.rs:334 | FALSE | `lumen open samples/sample.mp4` decodes H.264 to end-of-media (my run); registry contradiction — F-001. |

## 7. Dead / duplicated / unreachable code
- None found in this file: every pub item has an in-file test or an external consumer (PresentationQueue/Presenter in sink.rs; scheduling/sync_scenarios/zero_alloc tests; pipeline_host wiring). `PushOutcome::MissingDropped` and `Held` are consumed by pipeline_host.rs:1354-1356 (accepted silently-but-counted — the host's choice, not a swallow).

## 8. Dependencies & flags
- Depends on: `media_clock` (NANO_BASE, ClockState, PlaybackClock, sync::{DriftEstimator, SyncBand, SyncBands}), `media_types` (GenerationId, VideoFrame, time), `crate::policy::SchedulerCfg`, `crate::sink::PresentRequest`. No feature gates; ships in the release lane via lumen-cli's `video_engine` dependency (verified by running the release binary).
- Note for the drift question: this file consumes `DriftEstimator`/`SyncBands` from media-clock — the self-referential-drift question is decided in crates/media-clock/src/drift.rs (my file #12) and media-clock/src/sync.rs (not mine).

## 9. Open questions for the next agent
- bench.rs (decode-bench) registry contradiction — F-001 belongs to whoever owns apps/cli/src/bench.rs; the fix is one dispatch table away.
- The KNOWN DEFECT registered at pipeline_host.rs:1362-1383 (window-full hold stashes the UNCONVERTED I420 frame → renderer refusal → frozen surface) interacts with this scheduler's `WindowFull` outcome; the scheduler side is correct, the host's hold is the defect — owner: pipeline_host.rs.
- `apps/desktop/tests/pipeline.rs` and `apps/cli/tests/sync_scenarios.rs` drive the scheduler closed-loop with drift; not my files — their honesty is unaudited here.
