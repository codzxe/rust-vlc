# AUDIT: crates/video-engine/src/scheduler.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 1712 / 33 production items + 19 `#[test]` fns |
| File verdict | REAL-UNPROVEN (genuine, reachable implementation; not executed by me) |
| Reachable from shipped entry point? | **YES** — `lumen open` → `CliHost::open_session` (apps/cli/src/host.rs:890) → `Engine::open` with `CliFrameConsumer` → first decoded video frame builds `VideoLimb { scheduler: PresentationScheduler::new(..) }` (apps/cli/src/pipeline_host.rs:1324-1331) → every pump step calls `tick_video()` (apps/cli/src/host.rs:609,627,721,743,819) → `VideoLimb::tick_once` → `self.scheduler.tick()` (apps/cli/src/pipeline_host.rs:769). In the CLI the pump runs only on the `advance` verb / scripted sessions; in desktop-tauri the shell ticker thread pumps it (apps/desktop-tauri/src/app.rs:42-48). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:2 |
| Commands run | rg for `PresentationScheduler` construction sites (crates + apps); sed quote verification; `grep -c "#\[test\]"` → 19; read of policy.rs SyncPolicy defaults (368-431) |

## 1. One-paragraph verdict

This is a real frame scheduler, not paper: a bounded PTS-sorted reorder window, generation-gated admission and wholesale purge, a late-purge prefix walk, largest-due selection with in-order supersede drops, bounded repeat-then-background, missing-PTS inference/hold, an M10 pre-roll hold that keeps seek landings out of a frozen clock's judgment, and a 3.B1 drift-observation seam (EMA + Schmitt bands + bounded WidenLate/ExtendEarly actuators). It is constructed and ticked by the shipped CLI host and (per code reading) the desktop shell. The state machine is the strongest code I have audited in this wave: edge arithmetic is i128-widened and saturating, every drop is counted and trail-logged, refusals hand the frame back. The defects are architectural, not algorithmic: nothing drives `tick` at a vsync cadence anywhere (the doc's "one vsync-paced scheduler thread" is explicitly deferred), so in the CLI the scheduler only advances on the manual `advance` verb — the same pump starvation as the clock (see backends.rs audit F-002); and the presentation it produces flows into a headless renderer in the CLI (`Renderer::new_headless`, pipeline_host.rs:1321), so no shipped host shows the frames on screen (the GUI no-`<canvas>` problem is W15's, but the CLI side is proven here).

## 2. What actually works (verified)

- Reachability chain above — verified by reading every hop (host.rs open_session 890-1000; pipeline_host.rs VideoLimb build 1288-1331, tick_once 768-776, tick_video 1010-1022).
- W00 baseline proves the surrounding pipeline runs in a scripted session: audit/runs/baseline/advance-test.txt shows `state-changed Buffering → Playing` … `Playing → Ended` with time advancing to `3.000000s` on the MJPEG+PCM AVI — the scheduler is on that path (video limb is built from decoded frames); the transcript does not print presents, so the scheduler's own execution is UNVERIFIED by run.
- 19 in-file tests are behavioral over a `VirtualVideoClock` and assert exact present order, drop trails, and stats (§5) — they would fail on real logic mutations.

## 3. Findings

### F-001 [S3] — No vsync/timer driver exists: the scheduler only ever ticks when a host pump runs, and the shipped CLI REPL never pumps between commands

Where: crates/video-engine/src/scheduler.rs:6-9; the driving gap is in apps/cli (cross-file).

Code (scheduler.rs:6-9, verbatim):
```rust
//! The doc places one vsync-paced scheduler thread in production. That
//! thread is session wiring (the playback engine's, Phase 3): it hosts
//! this state machine and calls [`PresentationScheduler::tick`] once
//! per vsync or per timer wake. v0 delivers the deterministic core —
```

What is wrong: the deferred "session wiring" was never written inside playback-engine (no ticker thread exists in crates/; only apps/desktop-tauri/src/app.rs has `spawn_event_pump`). The CLI REPL blocks on stdin (apps/cli/src/run.rs:226), so between user commands the scheduler is never ticked; frames pile into the bounded window and repeat/background actions never fire. The comment is honest about the deferral, which keeps this at S3 for this file (the user-visible blast radius is carried by the CLI host, logged as F-002 in the backends.rs audit).

Why it matters: a scheduler that is not ticked cannot present; "the engine advances ONLY when a host pumps it" (apps/desktop-tauri/src/app.rs:40-41) is the project-wide root cause of the frozen interactive session.

What correct looks like: playback-engine (or the CLI host) owns a pacer thread calling `tick` at the configured cadence, consuming `TickOutcome::next_due` as the wake target — the `next_due` field exists and is tested, but no shipped driver uses it as a timer target (grep: only tests read `next_due` besides the CLI's pump-time use).

Confidence: HIGH — Verification: read (repo-wide grep for tick drivers; no timer/vsync loop in crates/).

### F-002 [S3] — `PushOutcome::Stale` discards the frame silently at the CLI seam, and the scheduler's own boundary rejects equal-to-live generations only for strictly-older ones — a frame from a *failed* seek's mint can occupy the window indefinitely

Where: crates/video-engine/src/scheduler.rs:587-593 and 775-778; consumer at apps/cli/src/pipeline_host.rs:1386.

Code (scheduler.rs:776-778, verbatim):
```rust
                if front.frame.generation > published {
                    break;
                }
```

What is wrong: two coupled edge behaviors. (a) Frames of a generation *between* the live one and the clock-published one (a second seek minted before the clock published the first) are admitted (587: only `<` is stale) but never selected (800: `<= published`) and never purged as late (776: `> published` breaks) — they sit in the bounded window until some future generation flip. (b) The CLI seam drops `Stale` frames with `Ok(())` (pipeline_host.rs:1386) — counted in scheduler stats, invisible in host logs. Both are bounded (window ≤ max_pending) and the in-file test `held_preroll_frames_are_discarded_when_a_later_generation_publishes` (1625-1657) pins the eventual purge, so this is a latency/memory-edge note, not data loss.

Why it matters: §3.2 state-machine edges; a pathological seek storm keeps stale-mint frames resident up to the window bound.

What correct looks like: the push boundary should treat `frame.generation` not in `{live, live.next()}` conservatively, or the purge should also drop generations older than `published` when they can no longer present.

Confidence: MEDIUM — Verification: read; mutation reasoning only (UNVERIFIED by execution).

### F-003 [S4] — Doc-header overreach: "Zero heap traffic (D-1/D-8) … measured by `tests/zero_alloc_scheduler.rs`" — the measurement is in another file I did not audit and cannot run

Where: crates/video-engine/src/scheduler.rs:693-694.

Code (verbatim):
```rust
    /// Zero heap traffic (D-1/D-8): the drop trails are fixed-capacity
    /// inline arrays — measured by `tests/zero_alloc_scheduler.rs`.
```

What is wrong: nothing in this file; the claim is plausible (all tick-path types are inline/Copy; `PresentRequest` moves a `VideoFrame` whose payload is `SharedBytes`, i.e. refcount) but I could not run the counting-allocator test (ADAPTATION: no cargo). REAL-UNPROVEN.

Confidence: MEDIUM — Verification: read + file existence (`ls crates/video-engine/tests/` → zero_alloc_scheduler.rs present).

### F-004 [S4] — `DropTrail` fill slots are `Timestamp::new(0, NANO_BASE)` zeros that `Clone`/`Copy` propagate; equality semantics carefully ignore them, but any future consumer reading `pts` directly (not via `as_slice`) sees fake zeros

Where: crates/video-engine/src/scheduler.rs:193-202, 255-264.

Code (195-197, verbatim):
```rust
    /// Retained PTS prefix, inline. Slots at or beyond the retained
    /// length are meaningless zero-tick fill — never read, compared
    /// or printed.
    pts: [Timestamp; MAX_DROP_TRAIL],
```

What is wrong: the invariant "never read" is enforced only by discipline (private field, `as_slice` truncates); `Debug` and `PartialEq` respect it. Fragile-but-guarded; hygiene note.

Confidence: HIGH — Verification: read (private field + all accessors audited).

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PushOutcome` (enum) | 96-148 | REAL-UNPROVEN | — | refusals hand the frame back; `#[must_use]` |
| `PresentAction` (enum) | 151-165 | REAL-UNPROVEN | — | Present/RepeatLast/Background |
| `MAX_DROP_TRAIL` const | 178 | REAL-UNPROVEN | — | 16, matches default window |
| `DropTrail::len/is_empty/recorded/truncated/as_slice/iter` | 208-241 | REAL-UNPROVEN | — | exact-count + retained-prefix semantics; tested |
| `DropTrail::record` | 246-252 | REAL-UNPROVEN | — | overflow = count-only (D-1) |
| `DropTrail::Default/Debug/PartialEq/Eq` | 255-286 | REAL-UNPROVEN | S4 | F-004 |
| `TickDrops` (struct) | 296-310 | REAL-UNPROVEN | — | late/superseded trails + stale count |
| `TickOutcome` (struct) | 316-328 | REAL-UNPROVEN | — | carries `next_due` — unused by any shipped driver (F-001) |
| `SyncSample` + Default | 337-367 | REAL-UNPROVEN | — | counters-not-events |
| `SchedulerStats` (struct) | 372-416 | REAL-UNPROVEN | — | 19 counters; i128 sum; saturating max |
| `Pending` (struct) | 419-422 | REAL-UNPROVEN | — | pts + frame |
| `PresentationScheduler` (struct) | 426-448 | REAL-UNPROVEN | — | bounded window, held slot, sync seams |
| `SchedulerSync` (struct) + `new` | 454-503 | REAL-UNPROVEN | — | fixed-size state |
| `SchedulerSync::reset_estimators` | 508-515 | REAL-UNPROVEN | — | resets band machine |
| `SchedulerSync::note_late_drop` / `rotate_windows` | 519-539 | REAL-UNPROVEN | — | tumbling windows, O(1) |
| `Debug for PresentationScheduler` | 542-556 | REAL-UNPROVEN | — | clock elided |
| `PresentationScheduler::new` | 564-577 | REAL-UNPROVEN | — | generation seeded from clock snapshot |
| `push` | 583-679 | REAL-UNPROVEN | — | boundary check, gap seam (forward-only, deliberate), sorted insert, missing-PTS infer/hold |
| `tick` | 701-905 | REAL-UNPROVEN | — | the core: anchor-epoch seam, gen purge, held admission, late purge (purge_skip), largest-due selection + supersede drain, repeat/background, band decision, wait hint; i128 math throughout |
| `effective_late_threshold` | 909-915 | REAL-UNPROVEN | — | widened ≥ configured |
| `effective_max_repeat` | 919-923 | REAL-UNPROVEN | — | saturating add |
| `apply_sync_ladder` | 932-980 | REAL-UNPROVEN | — | bounded: cap, Never-policy guard, drop budget, change rate limit |
| `cfg`/`stats`/`pending_len`/`holds_missing`/`last_presented`/`current_generation` | 984-1016 | REAL-UNPROVEN | — | accessors |
| `sequence_base` | 1021-1026 | REAL-UNPROVEN | — | window tail else last presented |
| `admit_held` | 1031-1049 | REAL-UNPROVEN | — | re-tries inference per tick; full window = counted drop |
| `insert_sorted` (free fn) | 1054-1063 | REAL-UNPROVEN | — | O(n), n ≤ max_pending (default 16); stable |
| `saturating_add` (free fn) | 1068-1070 | REAL-UNPROVEN | — | i128 clamp |
| `is_late` (free fn) | 1078-1083 | REAL-UNPROVEN | — | strict `<`, i128; None threshold = never late |

## 5. Test honesty (19 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `presents_in_order_under_a_normally_advancing_clock` (1108-1130) | YES | 50 presents on exact slots, zero drops asserted |
| `early_frames_are_held_until_their_slot` (1133-1157) | YES | wait-hint value (95 = 100−lead) pinned |
| `late_frames_are_dropped_counted_never_presented` (1160-1199) | YES | 10 late drops, no late present, no resurrection |
| `the_largest_due_frame_wins_and_supersedes` (1203-1221) | YES | supersede trail asserted |
| `out_of_order_arrivals_present_in_pts_order` (1223-1243) | YES | shuffle → monotone presents, 1 supersede |
| `repeat_budget_then_background` (1245-1271) | YES | exact repeat count then background, reset on present |
| `generation_change_purges_the_window_and_kills_repeats` (1274-1310) | YES | purge count, repeat kill, boundary refusal, new-gen flow |
| `missing_pts_is_inferred…` (1313-1344) | YES | inferred at last+duration; frame untouched |
| `uninferable_missing_pts_is_held…` (1347-1379) | YES | hold semantics + bounded replacement |
| `window_bound_is_enforced…` (1382-1408) | YES | capacity handed back, drain then re-accept |
| `unrepresentable_pts_is_unschedulable…` (1411-1423) | YES | i64::MAX on 1/1 base → typed Overflow |
| `stream_base_pts_are_normalized…` (1426-1441) | YES | 90 kHz → ns |
| `drop_trail_records_to_capacity_then_counts_only` (1444-1480) | YES | cap, truncation, equality semantics |
| `drop_trail_debug_shows_only_retained_state` (1483-1498) | YES | Debug excludes truncated entries |
| `late_storm_beyond_the_trail_cap_keeps_exact_counts` (1501-1540) | YES | 40-drop storm, exact counts, structural bound |
| `superseded_storm_beyond_the_trail_cap…` (1543-1578) | YES | 39 superseded under Never policy |
| `preroll_holds_are_not_judged…` (1585-1620) | YES | M10: no present/purge/hint during frozen clock |
| `preroll_holds_present_in_pts_order_once_the_generation_publishes` (1623-1664) | YES | terminal then PTS-order presents |
| `held_preroll_frames_are_discarded_when_a_later_generation_publishes` (1667-1700) | YES | failed-seek frames die with their mint |

All tests run on a `VirtualVideoClock` (injected `PlaybackClock`) — the subject itself is not mocked. Caveat: none of them exercise the CLI/tauri wiring (that is apps' territory), and none drive tick at a cadence (F-001).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one vsync-paced scheduler thread in production … That thread is session wiring (the playback engine's, Phase 3)" — deferred | scheduler.rs:6-9 (self-description, honest) | REAL (the deferral) / PAPER-ONLY (the thread) | no ticker thread exists in crates/ (grep); only apps/desktop-tauri/src/app.rs:42 `spawn_event_pump` |
| doc 03 §4.10 e binding order (gen check → late purge → selection → repeat/background) | module docs 14-37 | REAL-UNPROVEN | implemented at tick 727-861 in exactly that order; not executed by me |
| M10 pre-roll hold | module docs 52-71 | REAL-UNPROVEN | implemented at 776-778, 797-801, 894-898; three dedicated tests |
| Card 3.B1 A/V sync seams in the scheduler | module docs 687-691 | REAL-UNPROVEN | seams 1-4 at 714-724, 756-759, 823-845, 863-884 |
| "2.C4 presentation scheduler" card complete | docs/13-agent-wave-program.md row 2.C4 (referenced at scheduler.rs:1) | REAL-UNPROVEN | the core exists and is tested; the production driver does not (F-001) |

## 7. Dead / duplicated / unreachable code

- `TickOutcome::next_due`: computed every tick, tested, but no shipped driver uses it as a timer target — dead-in-production observability (F-001).
- No other dead code found in this file; every pub item has an in-crate or apps/cli consumer except `holds_missing`/`last_presented`/`current_generation` (test-only accessors, small).

## 8. Dependencies & flags

Depends on `media-clock` (`PlaybackClock`, `sync::{DriftEstimator, SyncBand, SyncBands}`), `media-types` (VideoFrame/Stamp/Timestamp/GenerationId). No feature gates; `video-engine` is an unconditional workspace member consumed by apps/cli (verified in apps/cli Cargo imports at pipeline_host.rs:84). Toolchain pin 1.99.1 covers `u64::is_multiple_of` (used at 877) — stable since 1.87, no issue.

## 9. Open questions for the next agent

- Does the desktop-tauri pump cadence (`SESSION_TICK`, app.rs:102-105) actually call into `CliFrameConsumer`-equivalent tick_video on the video limb every tick? (apps/desktop-tauri wave W14.)
- The CLI `video_hold` workaround comment (pipeline_host.rs:1370-1380) admits the render loop "drains the whole ring per pump" and that a longer media would fatal `ring_full` — a registered defect that bounds the scheduler's usefulness in the CLI; belongs to W13's apps/cli audit but originates in the pump design.
- `SchedulerCfg::for_frame_duration` (policy.rs) — policy file is another agent's; the scheduler trusts its output.
