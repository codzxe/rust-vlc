# AUDIT: apps/cli/src/pipeline_host.rs#part2 (lines 675–1402)
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-21 |
| Lines / functions | 728 (of 2033) / 37 items |
| File verdict | REAL (with one registered S2 defect in the shipped hold path) |
| Reachable from shipped entry point? | YES (call chain: `lumen` main → apps/cli/src/host.rs:936 `CliFrameConsumer::new` on open; host.rs:609/627/721/743/819 `tick_video` on pump; engine poll thread → `FrameConsumer::accept` at 1228) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | `rg -n "tick_video|set_present_tap|…"` (caller map: host.rs pump ×5, desktop shell.rs:187, frame_tap.rs:106 impl PresentTap); `sed -n` on every quoted range; read crates/video-engine/src/{scheduler.rs,queue.rs,policy.rs}, crates/renderer-wgpu/src/{renderer.rs,staging.rs}, crates/media-types/src/{frame.rs,packet.rs} to verify cross-crate claims |

**Chunk boundary note (for A06/A05):** part1 = lines 1–674 (consts, `host_error`,
`HostDecoderProvider`, `ReseedingDecoder`, `DrainGatedDecoder`, `AudioLimb`);
**part2 (this audit) = lines 675–1402** (`PresentTap`, `VideoLimb`,
`LatchingPresenter`, `ConsumerState`, `CliFrameConsumer`, `accept_audio`,
`accept_video`); part3 = lines 1403–2033 (`YuvToRgb` conversion, `AudioSession`,
`wall_clock`, `mod tests`). Split chosen at clean item boundaries nearest the
2033/3 ≈ 678 line marks (674 / 1402).

## 1. One-paragraph verdict
This chunk is the real frame-consumption core of the shipped headless pipeline:
scheduler + queue + renderer wiring, the window-full hold, the seek boundary's
presentation duties, and the audio straggler/exact-prefix bookkeeping. It is
genuine, traced, and reachable from `lumen open`. It carries one
admitted-in-code S2 defect: the window-full hold arm stashes the **unconverted**
I420 original, so every hold-retried frame on an H.264 lane is refused by the
renderer (`render.upload.format-unsupported`) and the presentation queue wedges
on the refused front request — verified by code trace into renderer-wgpu's
staging refusal, corroborated by the code's own `[KNOWN DEFECT, registered as
the W1-06 follow-up card]` comment. Two S3s: the `enqueue_and_drain` Full arm's
"unreachable" comment is false after a mid-drain failure (out-of-order direct
render), and subtitle frames are silently `Ok(())`-swallowed while being counted
as accepted.

## 2. What actually works (verified)
- Reachability: `rg` → host.rs:936 constructs `CliFrameConsumer`; host.rs:609,
  627, 721, 743, 819 call `tick_video()` per pump step; desktop shell.rs:187 +
  frame_tap.rs:106 (`impl PresentTap`) install the w31.1 tap. The AVI
  first-vertical lifecycle in `audit/runs/baseline/avi-firstvertical-open-session.txt`
  (Buffering → Playing → Stopped → Idle) exercises this exact accept/tick path.
- Scheduler/queue/latch arithmetic traced against crates/video-engine
  (queue.rs:188 `drain_to` keeps the refused request; scheduler.rs:597–601
  `WindowFull` hands the frame back) and renderer-wgpu (renderer.rs:218–222
  `Presenter` impl; staging.rs:275 refuses non-RGBA) — every cross-crate claim
  in the comments checked out at the cited sites.
- `VideoFrame::clone` is cheap (media-types/frame.rs:210–226: `FrameStorage::Cpu(SharedBytes)`
  with `Arc<Vec<u8>>` backing, packet.rs:47) — the per-retry `front().cloned()`
  (1027) and hold-arm `frame.clone()` (1383) are refcount bumps, not copies.

## 3. Findings

### F-001 [S2] — Window-full hold stashes the UNCONVERTED frame; hold-retried I420 frames are refused at the renderer and wedge the presentation queue
Where: apps/cli/src/pipeline_host.rs:1383 (arm opens 1357), comment 1362–1382
Code:
```rust
            // [KNOWN DEFECT, registered as the W1-06 follow-up card
            // (Lead decision required): this arm stashes the
            // UNCONVERTED original (`frame.clone()`), so every
            // hold-retried I420 frame is refused at the renderer
            // (`render.upload.format-unsupported`) and the surface
            // freezes on an early window frame. […]
            state.video_hold.push_back(frame.clone());
```
What is wrong: `accept_video` converts I420→RGBA into `uploadable` (1340–1342)
before `scheduler.push(uploadable)`, but the `WindowFull` arm discards
`uploadable` and stashes `frame.clone()` — the original I420. On retry
(`retry_video_hold` → `scheduler_push_for_hold`, 1026–1046) the I420 frame is
admitted to the scheduler window; at the next tick the renderer refuses it
(verified: crates/renderer-wgpu/src/staging.rs:275
`if !matches!(format, PixelFormat::Rgba8 | PixelFormat::Bgra8)` →
`render.upload.format-unsupported`, asserted by the renderer's own test at
staging.rs:399). The refusing present is the queue front; queue.rs:188–197
keeps the refused request and everything behind it, so every subsequent
`drain_to` re-fails at the same front — a permanent wedge.
Why it matters: this is the shipped video path for every I420 lane — i.e. the
H.264 MP4 vertical the project claims. Any decode burst that overflows the
scheduler window (window bound from `SchedulerCfg`; decode-ahead bursts of 16
packets per poll vs a bounded window) puts frames on the hold; each retried
frame then fails at the renderer. The baseline MP4 smoke
(mp4-smoke-open-session-420.txt) did not hit it, but the defect is latent in
the default path, not an exotic input.
What correct looks like: stash the converted frame — `PushOutcome::WindowFull
{ frame, .. }` hands the untouched frame back (scheduler.rs:122–126); the arm
already has `uploadable` in scope. The code itself names this "the obvious fix"
and defers it behind a pacing decision (W1-06 card, D-5 register) — a
registered, deliberately-kept defect.
Confidence: HIGH — Verification: read (full trace: accept_video → hold →
retry → scheduler → LatchingPresenter::present → staging refusal; queue
wedge semantics read in queue.rs:180–201).

### F-002 [S3] — `enqueue_and_drain`'s Full arm is reachable after a failed drain, and then presents the NEWER request out of order while older requests stay queued
Where: apps/cli/src/pipeline_host.rs:785–790
Code:
```rust
            EnqueueOutcome::Full { request, .. } => {
                // One slot per tick with spare capacity: unreachable
                // in this wiring; honored honestly by presenting the
                // refused request directly rather than dropping it.
                return self.render_request(request, tap, repeat);
            }
```
What is wrong: the comment's "unreachable in this wiring" is false. `drain_to`
returns early on a presenter refusal and **keeps the refused request and
everything behind it** (queue.rs:180–187), so a mid-drain failure leaves up to
4 resident slots; the next tick's `try_push` hits capacity 4 → `Full` → the
newer request is rendered directly, bypassing the queued older requests. Order
inversion on the degrade path. Note F-001 is exactly the kind of persistent
front-of-queue failure that triggers this arm repeatedly.
Why it matters: on any present-path failure the video output silently
reorders (newer frames jump older ones), and the stale comment hides the
behavior from the next maintainer.
What correct looks like: either drain-before-push ordering in `tick_once`, or
make the Full arm honest: drop-and-count the request (or retry the queue front
first), and fix the comment to state the reachable condition.
Confidence: HIGH — Verification: read (queue.rs try_push capacity check 168–174,
drain_to early-return 197).

### F-003 [S3] — Subtitle frames: silent `Ok(())` swallow, counted as accepted
Where: apps/cli/src/pipeline_host.rs:1229–1239
Code:
```rust
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        let mut state = self.lock();
        state.frames_accepted = state.frames_accepted.saturating_add(1);
        match frame {
            Frame::Audio(audio) => accept_audio(&mut state, &audio),
            Frame::Video(video) => {
                accept_video(&mut state, &video, track, &self.facts, &self.clock)
            }
            // Subtitle/data limbs are not wired in the first vertical.
            Frame::Subtitle(_) => Ok(()),
        }
    }
```
What is wrong: a decoded subtitle frame is dropped with a success return AND
inflates `frames_accepted`. Contrast the audio limb's honest typed refusal for
the analogous inconsistency (`cli.audio_limb_absent`, 1259–1263). The only
thing that keeps this arm dead today is the dispatch-level subtitle refusal
(audited separately in dispatch.rs — the wave context says subtitles are
refused at dispatch); if that gate ever opens, subtitle frames vanish silently
with a success code.
Why it matters: swallowed work with `Ok(())` is the exact fakeness pattern the
audit hunts; the counter then over-reports delivered frames.
What correct looks like: mirror `cli.audio_limb_absent`: a typed
`cli.subtitle_limb_absent` refusal (or at minimum do not count the frame).
Confidence: HIGH — Verification: read; dispatch-side gate cross-checked in the
dispatch.rs audit (this wave).
Severity note: S3 not S2 because the dispatch gate makes the arm unreachable
in the shipped binary today (blast radius limited to a future wiring change).

### F-004 [S4] — Undocumented 30 fps fallback for the scheduler config
Where: apps/cli/src/pipeline_host.rs:1296–1299
Code:
```rust
        let frame_ns = facts
            .fact_for(track)
            .and_then(|fact| fact.frame_ns)
            .unwrap_or(33_333_333);
```
What is wrong: when the demux facts carry no `frame_ns` for the track, the
scheduler's pacing baseline silently becomes 30 fps. A variable-frame-rate or
fact-less video track gets pacing derived from a guess; nothing logs or marks
the fallback.
Why it matters: pacing drift for non-30fps content without any observable.
What correct looks like: derive from the first frame's PTS deltas or surface a
counter/log line that the fallback engaged.
Confidence: HIGH — Verification: read (facts source: `crate::demux_host::MediaFacts`,
part1/A06 territory — the fallback itself is in my lines).

### F-005 [S4] — `LatchingPresenter`'s single `repeat` bit is applied to every drained request
Where: apps/cli/src/pipeline_host.rs:738–744, 746–757
Code:
```rust
    /// This tick's `PresentAction` repeat bit. The queue holds at
    /// most the tick's own request in this wiring (one push per tick
    /// plus a full drain), so the action's bit describes every
    /// drained request; a future wiring that batches mixed actions
    /// must ride the bit per-request instead `[PROPOSAL]`.
    repeat: bool,
```
What is wrong: the tap receives the tick-level repeat flag for all drained
requests. Correct in the current one-push-per-tick wiring (F-002's Full arm
breaks even that invariant by direct-rendering), but the field is a
future-wiring trap the code itself flags.
Why it matters: the desktop's frame channel would mislabel repeats once
batching lands.
What correct looks like: carry the repeat bit on `PresentRequest` (or per-slot
in the queue) rather than on the presenter.
Confidence: HIGH — Verification: read.

### F-006 [S4] — `VideoLimb::presented` / `refused` are `pub` mutable fields
Where: apps/cli/src/pipeline_host.rs:719–723
Code:
```rust
    /// Frames handed to the renderer (presents + repeats).
    pub presented: u64,
    /// Frames the scheduler refused (window full / unschedulable) —
    /// observable, never silent.
    pub refused: u64,
```
What is wrong: public mutable counters on the limb; any holder of `&mut
ConsumerState` (the pump) can corrupt them, and there is no accessor parity
with the private fields around them.
Why it matters: encapsulation debt on the path the desktop shell also touches.
What correct looks like: private fields + `#[must_use]` accessors, matching
`frames_accepted`'s treatment on `ConsumerState` (961–963).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `trait PresentTap` / `tap_present` | 692–700 | REAL | — | w31.1 seam; pure CLI passes `None` (verified: only desktop frame_tap.rs:106 implements it); error contract documented |
| `struct VideoLimb` (fields) | 703–724 | REAL | S4 | pub counters (F-006); latch `(gen, ns)` correct shape |
| `struct LatchingPresenter` | 733–744 | REAL | S4 | single `repeat` bit (F-005) |
| `Presenter::present` (LatchingPresenter) | 746–758 | REAL | — | latch written before tap; tap error propagates after latch (comment contract upheld); verified against renderer's Presenter impl |
| `VideoLimb::tick_once` | 768–775 | REAL | — | maps Present/RepeatLast/Background honestly; Background is a no-op by design |
| `VideoLimb::enqueue_and_drain` | 777–823 | REAL | S3 | Full arm reachable + out-of-order (F-002); Stale arm drops-and-returns-Ok with `let _ = request` (documented, queue counts it); delivered-before-failure accounting correct (817) |
| `VideoLimb::render_request` | 825–838 | REAL | — | direct-render fallback used by F-002's arm; latch+counter+tap ordering consistent with presenter |
| `VideoLimb::scheduler_push_for_hold` | 842–849 | REAL | — | maps 6 PushOutcome variants to 3 HoldRetry outcomes; WindowFull→Refused, Stale/Unschedulable→Dropped (counted); verified against scheduler.rs enum |
| `VideoLimb::seek_boundary` | 860–862 | REAL | — | `queue.publish(generation)` drops resident requests wholesale (queue.rs:150–156 verified) |
| `VideoLimb::scheduler_stats` | 866–868 | REAL | — | delegation |
| `VideoLimb::last_presented_ns` | 878–881 | REAL | — | returns None unless latch generation == clock-published generation (F3/issue #16 contract, traced) |
| `VideoLimb::staging_stats` | 885–887 | REAL | — | delegation |
| `VideoLimb::queue_stats` | 893–895 | REAL | — | delegation |
| `VideoLimb::published_generation` | 900–902 | REAL | — | delegation |
| `VideoLimb::screenshot_png` | 909–919 | REAL | — | readback+extent+encode, typed errors mapped via `to_engine_error` |
| `const MAX_VIDEO_HOLD` | 927 | REAL | — | 64-frame bound, [PROPOSAL] sizing documented |
| `enum HoldRetry` | 930–939 | REAL | — | 3-state outcome, honest |
| `struct ConsumerState` (fields) | 944–986 | REAL | — | generation bookkeeping fields documented; `present_tap` read per tick |
| `ConsumerState::video` | 991–993 | REAL | — | accessor |
| `ConsumerState::audio` | 997–999 | REAL | — | accessor |
| `ConsumerState::tick_video` | 1010–1020 | REAL | — | tick error propagates before hold retry (hold retry skipped on error tick — bounded degrade, acceptable); split-borrow comment verified |
| `ConsumerState::retry_video_hold` | 1026–1046 | REAL | — | clones are Arc-bumps (verified SharedBytes); Dropped double-counts into both `video.refused` and `frames_refused` (consistent with accept_video's refusal arm); video==None breaks (unreachable when hold non-empty) |
| `ConsumerState::note_video_error` | 1049–1053 | REAL | — | first-error-wins, surfaced once |
| `ConsumerState::observe_audio_drift` | 1058–1062 | REAL | — | no-op without audio limb (documented) |
| `ConsumerState::audio_drift_stats` | 1068–1072 | REAL | — | default when no limb |
| `ConsumerState::seek_boundary` | 1095–1109 | REAL | — | hold retain `>= mint`, queue publish, audio drift reset, floor arm — traced to worklog archive-2026-10.md:7449's host-duty list; correct |
| `ConsumerState::audio_stale_prefix` | 1130–1147 | REAL | — | write-order exact-prefix arithmetic; `audio_seen_gen >= mint` selects change-point vs total-written; saturating_sub on consumed; logic traced, no defect found |
| `struct CliFrameConsumer` | 1152–1156 | REAL | — | Mutex + facts + clock |
| `CliFrameConsumer::new` | 1165–1187 | REAL | — | audio pre-wired, video lazy; all counters zeroed; GenerationId::FIRST floors |
| `CliFrameConsumer::lock` | 1192–1197 | REAL | — | poisoned-lock recovery documented (std semantics) |
| `CliFrameConsumer::set_present_tap` | 1203–1205 | REAL | — | state-safe install (verified callers: host.rs:273, desktop shell.rs:187) |
| `CliFrameConsumer::take_video_error` | 1209–1211 | REAL | — | take-once |
| `CliFrameConsumer::last_presented_ns` | 1220–1225 | REAL | — | delegation through lock |
| `FrameConsumer::accept` | 1229–1240 | REAL | S3 | F-003 (subtitle swallow + accepted-count inflation) |
| `FrameConsumer::ready_for` | 1242–1250 | REAL | — | audio park gate via `ring_ready` (part1 const AUDIO_PARK_FILL); video always-ready rationale documented and consistent with hold bound |
| `accept_audio` | 1254–1285 | REAL | — | typed refusal when limb absent; straggler floor gate before change-point bookkeeping (order documented and correct); change-point records write position BEFORE staging |
| `accept_video` | 1288–1393 | REAL | S2/S4 | F-001 (hold arm stashes unconverted original); F-004 (frame_ns fallback); lazy limb construction under the consumer mutex (cold path, once); renderer extent from first frame — later size changes letterbox via compute_placement (renderer.rs:208), no defect |

## 5. Test honesty
No tests live inside lines 675–1402. The file's `mod tests` (1638+, part3/A05)
covers I420→RGBA conversion and the decoder reseeding (part1), not this
chunk's hold/tick/seek machinery. Grep across tests/ found no test driving
`video_hold` or the hold-retry present path; the code's own D-5 note concedes
"the scenarios keep their bug-masked shape" (pipeline_host.rs:1380–1382) —
i.e. no test would fail if F-001's arm were fixed or if it corrupted ordering
further. The seek-boundary and audio-prefix arithmetic is exercised only
indirectly via first-vertical journey tests (baseline test-workspace.log.txt
1681 passed, no hold-path case among them).
| Test (nearest coverage) | Would it fail if the code were broken? | Reason |
|---|---|---|
| first-vertical journey/seek scenarios | NO (not for this chunk's defects) | They drive the paced AVI lane where the window never overflows; F-001's hold arm is never entered (mp4-smoke-open-session-420.txt shows no present error) |
| renderer staging tests (staging.rs:399) | YES (for the refusal itself) | Proves I420 is refused — this is what makes F-001 a wedge, but it tests the renderer, not the hold arm |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Window-full hold … stashes scheduler-window-full frames … the decode burst no longer drops the back half of the media" | docs/worklog/archive-2026-10.md:1487 | FALSE (for I420 lanes, as shipped) | pipeline_host.rs:1383 stashes the unconverted original; renderer-wgpu/staging.rs:275 refuses it — the back half is not dropped at push but is lost to the renderer-refusal wedge (F-001). In-code comment (1362–1382) admits it. |
| "the consumer's `video_hold` deque … retries after every tick" | docs/worklog/archive-2026-10.md:2240, 2420 | REAL | pipeline_host.rs:1018/1026–1046 (tick_video → retry_video_hold) |
| Seek host duties: "hold clear = `video_hold.retain(gen >= mint)`; queue publish = `VideoLimb::seek_boundary`" | docs/worklog/archive-2026-10.md:7449 | REAL | pipeline_host.rs:1095–1109, 860–862 |
| "Deliberate salvage decision KEPT … the converted-frame experiment exposed the `advance_time` whole-ring-drain defect … registered in-code as the W1-06 follow-up card" | docs/worklog/archive-2026-10.md:7461 | REAL | pipeline_host.rs:1362–1382 verbatim register |
| Present tap mirrors "every frame the renderer presents" w31.1 | module docs 29–31, 676–691 | REAL-UNPROVEN (CLI side proven inert by code: default None; desktop side is A03/A09's files) | LatchingPresenter 751–755 + tick wiring 1014–1017; no baseline run exercises the desktop tap |

## 7. Dead / duplicated / unreachable code
- `Frame::Subtitle(_) => Ok(())` arm (1238): unreachable in the shipped binary
  only because dispatch refuses subtitles upstream (see dispatch.rs audit);
  dead-by-gate, not dead-by-construction.
- No duplicated logic found in the chunk; `render_request` vs
  `LatchingPresenter::present` overlap is deliberate (direct path vs drain
  path) and kept consistent.
- `HoldRetry::Refused` while `video == None`: `break` at 1030 is unreachable
  when the hold is non-empty (limb exists), defensive only.

## 8. Dependencies & flags
- Consumes: audio-engine, audio-output (NullSink), codec-api (FrameConsumer/
  Decoder seams), media-types, media-clock, playback-engine, renderer-wgpu,
  video-engine, crate::demux_host (MediaFacts). All are workspace crates on
  the shipped `lumen` path (baseline build #1 clean; engine-info features
  `video.software-compositor`).
- `#[allow(clippy::empty_loop)]` at 139 is part1's (not my lines). No cfg
  gating in 675–1402 — this code is unconditional in the shipped binary.

## 9. Open questions for the next agent
- A06 (part1): `HostDecoderProvider::decoder_for` — where does the 4:4:4 MP4's
  `decode.unsupported` originate, and does the engine's Error state stop
  polling audio (the HANG root cause is not in my chunk; consumer-side
  nothing blocks — `accept` is never called once decode refuses).
- A05 (part3): the `advance_time` whole-ring-drain defect named in the F-001
  comment ("one `advance` tick plays the ENTIRE 2.000 s fixture") — verify in
  the pump/hosts files whether that still holds; it is the stated reason the
  S2 is deliberately kept.
- Whoever owns host.rs (A09/A10): the pump's `tick_video` error handling
  (host.rs:837 take_video_error) — confirm the repeated per-tick error after a
  queue wedge (F-001) is surfaced once and then silently ignored forever.
