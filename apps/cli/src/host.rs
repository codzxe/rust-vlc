//! The CLI host (card 2.D2): owns the `playback-engine` [`Engine`],
//! pumps it, correlates receipts to requests, records the event
//! stream, and answers the class-S snapshot commands.
//!
//! # The host IS the session thread (2.C2's invariant S1)
//!
//! The minimal engine is synchronous and single-writer: state moves
//! only inside [`Engine::poll`]. The CLI's [`CliHost::pump`] is
//! therefore the whole scheduler — one pump is
//!
//! 1. `engine.poll()` — drain the command inbox (transition table),
//!    then one bounded pipeline tick (demux ≤ 64 events, decode ≤ 16
//!    packets per limb),
//! 2. drain the output hub — receipts first (they precede the events
//!    they caused, doc 05 §4.3), then events through the wire
//!    conversion and the staleness filter,
//! 3. **advance time**: render audio quanta from the null device's
//!    ring (each render moves the sample counter → the audio-master
//!    clock), ticking the video scheduler once per quantum so
//!    presents track the clock.
//!
//! Deterministic by construction on the audio-bearing path: nothing
//! reads a wall clock, and every observable (states, events, frames
//! staged, samples rendered) is a pure function of the pump count.
//!
//! # Seek (the engine's native flow — card 3.A1 wired host-side by
//! W1-06, per the directives-audit C8/ED-FIX-4 fix)
//!
//! `playback.seek` submits the engine's `Seek` command through
//! [`SessionHandle::seek`] and the seven-step flow of doc 03 §6 runs
//! IN-PIPELINE: mint + flush, demux re-anchor, pre-roll,
//! clock re-anchor on the first audio frame's own timestamp, the
//! `Seeked{requested, actual}` terminal. The session epoch, the
//! audio device and its ring, the decoders, the renderer surface and
//! the whole observability trail SURVIVE the seek — the old
//! stop→close→reopen composition (2.D2's) destroyed all of them and
//! is FORBIDDEN for seek (D-4/D-7: seeking must never destroy the
//! session, drop hardware handles, or re-open files).
//!
//! ## The host duties at the seek boundary
//!
//! Doc 03 §6 step 3 flushes the pipeline; the halves behind THIS
//! host's own seams (the audio ring, the window-full hold, the
//! presentation queue) are the host's to move —
//! [`CliHost::seek_boundary`], run once per accepted seek whose
//! pipeline actually moved:
//!
//! 1. **ring flush** — discard exactly the ring's PRE-SEEK prefix
//!    (doc 03 §6 “flush … ring (to silence)”): the undrained tail of
//!    the old position, never the landing frames the accept poll's
//!    decode may already have appended behind it (FIFO);
//! 2. **hold clear** — drop the window-full hold's pre-mint frames;
//! 3. **queue publish** — the presentation queue's renderer-mailbox
//!    clear (the new generation replaces the residents).
//!
//! A seek that failed AT the accept poll (unseekable, unresolvable,
//! reanchor refusal) performs NONE of these: contract D4 rolled the
//! pipeline back and the old content is still live. And because the
//! engine's `poll` may run the accept AND the whole pre-roll in one
//! call (the fast-completion case), the clock re-anchor compensation
//! documented on [`CliHost::seek_boundary`] absorbs the discard's
//! counter movement into the landing anchor.
//!
//! ## Class-A truth
//!
//! The receipt answers at accept (class A: accepted ≠ completed);
//! the landing truth arrives as the `Seeked`/`SeekFailed` terminal
//! event, awaited bounded here (the same pacing the REPL's settle
//! gives every class-A command — a seek visibly lands before the
//! next line is read, and a Paused seek — whose wire state never
//! leaves `paused` — still reports). Typed failures surface as the
//! `timing.seek-failed` event with the old position (contract D4:
//! rollback, playable-at-old-position, never a fake success); the
//! reopen fallback does not exist.

use std::collections::BTreeMap;
use std::sync::Arc;

use audio_output::null::NullDevice;
use commands::catalog::command::{CommandListing, CommandResult};
use commands::catalog::types::{
    Chapter, DecoderInfo, Duration, EngineInfo, MediaInfo, SettingValue, TimeRange, Timestamp,
    TrackInfo, TrackKind, Volume,
};
use commands::envelope::{CommandOutcome, Generation, RequestId, SessionId};
use commands::{CommandError, ErrorCode, EventKind, PlaybackState, Stage, WireEvent};
use media_clock::{MediaClock, MonotonicSource, PlaybackClock};
use media_types::MediaKind;
use media_types::Rate;
use playback_engine::event::EngineEventKind;
use playback_engine::{
    CommandReceipt, Engine, EngineOutput, ReceiptResult, SeekTarget, SessionCommandKind,
    SessionConfig, SessionError, SessionHandle, SessionState,
};
use plugin_api::SeekSupport;

use crate::demux_host::{MediaFacts, OpenedMedia, open_source, wire_error};
use crate::pipeline_host::{
    AudioSession, CliFrameConsumer, HostDecoderProvider, LANDED_DECODER_FAMILIES, PresentTap,
    RENDER_QUANTUM,
};

/// The pump bound for "await one observable" (receipts, states) —
/// generous against the fixture's shape (≈10 pumps to full drain).
const AWAIT_PUMPS: usize = 64;

/// The headless frame accounting snapshot (see
/// [`CliHost::headless_accounting`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadlessAccounting {
    /// Frames that reached the consumer (both limbs).
    pub frames_accepted: u64,
    /// Frames the scheduler refused (window full / unschedulable).
    pub frames_refused: u64,
    /// Frames handed to the renderer (presents + repeats).
    pub video_presented: u64,
    /// Scheduler admission refusals.
    pub video_refused: u64,
    /// Frames admitted to the scheduling window.
    pub scheduler_pushed: u64,
    /// Due frames selected for presentation.
    pub scheduler_presented: u64,
    /// Repeat presents within the budget.
    pub scheduler_repeats: u64,
    /// Frames dropped by the late policy.
    pub scheduler_dropped_late: u64,
    /// Frames staged through the renderer's upload path.
    pub staging_frames: u64,
    /// Bytes copied into the staging buffer.
    pub staging_bytes: u64,
    /// Staging buffer copies performed.
    pub staging_copies: u64,
    /// Frames dropped wholesale at scheduler generation changes
    /// (doc 03 §5(7)) — the pre-seek window content purged at the
    /// first post-seek tick (the seek-boundary proof on the
    /// scheduler side).
    pub scheduler_dropped_stale: u64,
    /// Present-requests dropped wholesale by the presentation
    /// queue's generation publish (the renderer-mailbox clear of doc
    /// 03 §6 step 3 — the seek-boundary proof on the queue side).
    pub queue_flushed: u64,
    /// The presentation queue's published generation counter (the
    /// host-duty proof: after a seek it carries the seek's mint).
    pub queue_published: u64,
}

/// The pump bound for run-to-end (the scripted session's `advance`).
pub const RUN_PUMPS: usize = 512;

/// The wire-visible state observation history entry: `(from, to)`.
pub type StateStep = (PlaybackState, PlaybackState);

/// The CLI's engine host. One instance per process (the engine is
/// single-session v0; `lumen_dispatch` is the only writer).
pub struct CliHost {
    engine: Engine,
    handle: Option<SessionHandle>,
    consumer: Option<Arc<CliFrameConsumer>>,
    /// The presented-frame tap stash (w31.1): installed into the
    /// consumer at open time when no session exists yet — the desktop
    /// binds its webview frame channel BEFORE the first `session.open`
    /// lands, and the tap must ride the first session's consumer from
    /// its first present (never missing the opening frames).
    pending_present_tap: Option<Arc<dyn PresentTap>>,
    device: Option<NullDevice>,
    clock: Option<Arc<MediaClock>>,
    facts: Option<Arc<MediaFacts>>,
    /// The engine's declared seek capability for the open session
    /// (F0.3, W13-A02 F-001): the demuxer's `SeekSupport` read at
    /// open time — the exact fact the engine's seek flow consults
    /// through `Pipeline::seekability` (doc 03 §6 step 1) and the
    /// `media.info-ready` event carries; the class-S `info` snapshot
    /// answers it (never the retired `true` constant). Cleared with
    /// the other session-scoped halves at teardown.
    seekable: Option<bool>,
    source: Option<commands::catalog::types::SourceSpec>,
    allocator: commands::RequestIdAllocator,
    staleness: commands::StalenessFilter,
    /// Receipts drained but not yet matched (bounded by command flow).
    pending_receipts: Vec<CommandReceipt>,
    /// The wire events observed since construction (tests + printing).
    events: Vec<WireEvent>,
    /// The state-transition trail (from the `state-changed` events).
    state_history: Vec<StateStep>,
    /// The last fatal error (wire form) — the honest `open`/`seek`
    /// failure answer.
    last_fatal: Option<CommandError>,
    /// The last track-list snapshot (wire form).
    tracks: Vec<TrackInfo>,
    /// Session-local settings overrides (the settings-schema seam).
    settings: BTreeMap<String, SettingValue>,
    /// The reused output-drain buffer (card 3.B4's addendum, D-1):
    /// `take`-out / `drain_outputs_into` / move-back per pump —
    /// steady state allocates nothing on the drain path.
    outbox_scratch: Vec<EngineOutput>,
    /// The injected event-throttle source (card 3.B4): when set,
    /// sessions open with this monotonic source driving the doc 05
    /// §3.4 emission windows instead of real time — the
    /// determinism seam the first-vertical evidence harness uses
    /// (a frozen source keeps the transcripts byte-stable).
    event_throttle: Option<Arc<dyn MonotonicSource>>,
    /// The wall-clock pacing anchor ([`CliHost::pump_wallclock`]'s
    /// truth): `Some` exactly while the session is `Playing` — the
    /// (instant, device counter, rate) the wall owes zero at. Every
    /// non-Playing stretch clears it, so pause/underrun/seek all
    /// re-anchor on the next `Playing` (the clock's own re-anchor is
    /// the position truth; the wall anchor only answers "how much
    /// real time has elapsed since then").
    wall_anchor: Option<WallAnchor>,
}

/// The wall-pacing anchor: the device counter and clock rate at the
/// instant the current `Playing` stretch began. The due-frames
/// arithmetic of [`CliHost::advance_time_wall`] renders
/// `anchor.frames + elapsed × rate` — never more, whatever the poll
/// cadence (a 10 ms session tick renders at most the quanta the wall
/// has become owed; a slow tick catches up only to the wall, never
/// past it).
#[derive(Debug, Clone, Copy)]
struct WallAnchor {
    /// When this `Playing` stretch was anchored.
    at: std::time::Instant,
    /// The device's consumed-frame counter at the anchor.
    frames: u64,
    /// The clock's rate at the anchor (a rate change re-anchors —
    /// the clock anchor's own `rate` field is the comparison truth).
    rate: Rate,
}

impl Default for CliHost {
    fn default() -> Self {
        Self::new()
    }
}

impl CliHost {
    /// An idle host (no session).
    #[must_use]
    pub fn new() -> CliHost {
        CliHost {
            engine: Engine::new(),
            handle: None,
            consumer: None,
            pending_present_tap: None,
            device: None,
            clock: None,
            facts: None,
            seekable: None,
            source: None,
            allocator: commands::RequestIdAllocator::new(),
            staleness: commands::StalenessFilter::new(),
            pending_receipts: Vec::new(),
            events: Vec::new(),
            state_history: Vec::new(),
            last_fatal: None,
            tracks: Vec::new(),
            settings: BTreeMap::new(),
            outbox_scratch: Vec::new(),
            event_throttle: None,
            wall_anchor: None,
        }
    }

    /// Inject the event-throttle source (card 3.B4): every session
    /// opened after this call reads its doc 05 §3.4 emission windows
    /// from `source` instead of real monotonic time. The
    /// first-vertical evidence harness injects a frozen source so
    /// the in-episode `session.buffering` windows and the non-fatal
    /// damage windows are deterministic — production hosts leave it
    /// unset and get the real clock.
    pub fn set_event_throttle(&mut self, source: Arc<dyn MonotonicSource>) {
        self.event_throttle = Some(source);
    }

    /// Install the presented-frame tap (w31.1, lane `luf1-channel`):
    /// every frame the renderer presents is mirrored to `tap` from
    /// that moment on. State-safe at ANY point — before the first
    /// session the tap stashes and rides the first consumer from its
    /// first present; mid-session it installs live (the consumer
    /// reads it per scheduler tick, so a playing session picks it up
    /// on the next pump). The pure CLI binary never calls this
    /// (byte-identical behavior with the seam inert); the desktop
    /// shell binds its webview frame channel through it.
    pub fn set_present_tap(&mut self, tap: Arc<dyn PresentTap>) {
        match &self.consumer {
            Some(consumer) => consumer.set_present_tap(tap),
            None => self.pending_present_tap = Some(tap),
        }
    }

    // -- observation ------------------------------------------------------

    /// Pump until the engine sits in a settled state (not the
    /// transient `Opening`/`Buffering`) or the bound — the REPL's
    /// post-command pacing policy (see [`crate::run`]). Bounded and
    /// deterministic: every observable is a pure function of the pump
    /// count. Public for the integration suites (the scripted
    /// sessions' open-and-settle step).
    pub fn settle(&mut self) {
        self.pump_pipeline_until(
            |host| {
                !matches!(
                    host.state(),
                    PlaybackState::Opening | PlaybackState::Buffering
                )
            },
            crate::host::AWAIT_PUMPS,
        );
    }

    /// The engine's current state (wire form).
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        session_state_as_wire(self.engine.state())
    }

    /// The live session id, if any.
    #[must_use]
    pub fn session_id(&self) -> Option<SessionId> {
        self.engine.session_id()
    }

    /// The live generation.
    #[must_use]
    pub fn generation(&self) -> Option<Generation> {
        self.engine.generation()
    }

    /// The observed wire events (in causal order; stale ones filtered).
    #[must_use]
    pub fn events(&self) -> &[WireEvent] {
        &self.events
    }

    /// The state-transition trail.
    #[must_use]
    pub fn state_history(&self) -> &[StateStep] {
        &self.state_history
    }

    /// The current position (µs). Audio-master sessions read the
    /// sample-counter clock (nanosecond base → µs) — deterministic
    /// headless, since the null device only moves on `advance`.
    ///
    /// Audio-less sessions (no audio limb — video-only media, or the
    /// limb-less geometry gap) are F3 (issue #16): their clock is the
    /// system monotonic, so `now()` extrapolates **wall elapsed**
    /// since the anchor — measured ±60-200 µs across identical
    /// `--start-time` runs (adts 1.004017 s vs 1.004071 s). The
    /// deterministic truth is what the session has actually
    /// presented: the last frame the video scheduler delivered under
    /// the live generation (its slot time), or — before anything
    /// presents, the exact `--start-time`/post-seek case — the
    /// clock's anchor, the landing the engine itself re-anchored on.
    /// Wall-elapsed is never reported.
    #[must_use]
    pub fn position_us(&self) -> i64 {
        match self.clock.as_ref() {
            Some(clock) if self.device.is_some() => clock.now().ticks / 1000,
            Some(clock) => {
                let latched_ns = self
                    .consumer
                    .as_ref()
                    .and_then(|consumer| consumer.last_presented_ns())
                    .unwrap_or_else(|| clock.anchor().media.ticks);
                latched_ns / 1000
            }
            None => 0,
        }
    }

    /// The harvested facts, when a session is (or was) open.
    #[must_use]
    pub fn facts(&self) -> Option<&Arc<MediaFacts>> {
        self.facts.as_ref()
    }

    /// The last `tracks.list-ready` snapshot (wire form) — the class-S
    /// `tracks.list` answer.
    #[must_use]
    pub fn track_snapshot(&self) -> Vec<TrackInfo> {
        self.tracks.clone()
    }

    /// The engine's pipeline observability counters.
    #[must_use]
    pub fn engine_stats(&self) -> playback_engine::EngineStats {
        self.engine.stats()
    }

    /// The consumer's delivery counters for AUDIO-ONLY lanes (the
    /// lanes `headless_accounting` cannot answer — it requires a
    /// video limb): frames that reached `accept`, frames the
    /// scheduler refused, and frames the seek boundary's audio floor
    /// dropped as stale stragglers (6.FIX-ogg-reanchor's observable
    /// honesty). `None` before a session wired a consumer.
    #[must_use]
    pub fn audio_lane_counters(&self) -> Option<(u64, u64, u64)> {
        let consumer = self.consumer.as_ref()?;
        let guard = consumer.lock();
        Some((
            guard.frames_accepted,
            guard.frames_refused,
            guard.audio_stale_dropped,
        ))
    }

    /// The headless pipeline accounting (the AC's frame evidence):
    /// the consumer's delivery counts plus the video limb's scheduler
    /// and staging measurements. `None` before a session wired a
    /// consumer.
    #[must_use]
    pub fn headless_accounting(&self) -> Option<HeadlessAccounting> {
        let consumer = self.consumer.as_ref()?;
        let guard = consumer.lock();
        let video = guard.video()?;
        let scheduler = video.scheduler_stats();
        let staging = video.staging_stats();
        let queue = video.queue_stats();
        Some(HeadlessAccounting {
            frames_accepted: guard.frames_accepted,
            frames_refused: guard.frames_refused,
            video_presented: video.presented,
            video_refused: video.refused,
            scheduler_pushed: scheduler.pushed,
            scheduler_presented: scheduler.presented,
            scheduler_repeats: scheduler.repeats,
            scheduler_dropped_late: scheduler.dropped_late,
            scheduler_dropped_stale: scheduler.dropped_stale,
            queue_flushed: queue.flushed,
            queue_published: video.published_generation().counter(),
            staging_frames: staging.frames_staged,
            staging_bytes: staging.bytes_staged,
            staging_copies: staging.staging_copies,
        })
    }

    /// The current surface as a PNG screenshot (2.D1's byte-stable
    /// encoder) — `None` before a video limb exists.
    #[must_use]
    pub fn screenshot_png(&self) -> Option<Result<Vec<u8>, media_types::EngineError>> {
        let consumer = self.consumer.as_ref()?;
        let guard = consumer.lock();
        let video = guard.video()?;
        Some(video.screenshot_png())
    }

    // -- pumping ----------------------------------------------------------

    /// One **pipeline-only** pump: engine poll + output drain + video
    /// error surfacing. NO time advance, NO tail play-out — the
    /// between-commands pacing primitive. Media time moves ONLY via
    /// [`CliHost::advance`] (the `advance` verb), which is what makes
    /// the scripted sessions deterministic and pause/seek landable:
    /// an awaited receipt or a settled open must not consume the
    /// media (the 2.D2 session ACs — pause freezes at 0, seek lands
    /// at its target exactly).
    pub fn pump_pipeline(&mut self) {
        self.engine.poll();
        self.drain_outputs();
        self.surface_video_error();
    }

    /// One FULL pump: pipeline-only steps + time advance + end-tail
    /// play-out. See the module docs — this is the `advance` verb's
    /// tick (the session thread under wall-clock pacing would be
    /// this plus a sleep; headless, `advance N` is N of these).
    pub fn pump(&mut self) {
        self.pump_pipeline();
        self.advance_time();
        self.drain_tail_at_end();
    }

    /// Pipeline-only pump-until (awaits: receipts, settles).
    pub fn pump_pipeline_until(
        &mut self,
        mut pred: impl FnMut(&CliHost) -> bool,
        bound: usize,
    ) -> bool {
        for _ in 0..bound {
            if pred(self) {
                return true;
            }
            self.pump_pipeline();
        }
        pred(self)
    }

    /// Pump until `pred` holds (or the bound). Answers whether the
    /// predicate was observed. FULL pumps (time advances) — the
    /// `advance` verb's driver; awaits use [`CliHost::pump_pipeline_until`].
    pub fn pump_until(&mut self, mut pred: impl FnMut(&CliHost) -> bool, bound: usize) -> bool {
        for _ in 0..bound {
            if pred(self) {
                return true;
            }
            self.pump();
        }
        pred(self)
    }

    /// Pump until the engine reports `target` (or the bound).
    pub fn pump_until_state(&mut self, target: PlaybackState, bound: usize) -> bool {
        self.pump_until(|host| host.state() == target, bound)
    }

    /// Pump a fixed number of ticks (the `advance` verb).
    pub fn advance(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.pump();
        }
    }

    /// One **real-time** pump (the GUI session thread's tick — the
    /// module docs' "session thread under wall-clock pacing would be
    /// this plus a sleep", made real): poll + drain + a
    /// **wall-bounded** time advance + the end-tail play-out.
    ///
    /// The difference from [`CliHost::pump`] is exactly the advance:
    /// the scripted sessions' [`CliHost::advance`] drains the whole
    /// ring per pump (every observable a pure function of the pump
    /// count — the headless determinism contract), which under a
    /// wall-clock ticker would play a full ring (up to 3 s) per tick.
    /// This pump renders only the quanta the device is **owed by wall
    /// time** — `anchor.frames + elapsed × rate` — so playback runs at
    /// 1.0× real time under any tick cadence, the ring's fill stays
    /// bounded by the decode park ([`crate::pipeline_host::
    /// AUDIO_PARK_FILL`]) plus one burst, and the position advances
    /// smoothly between UI polls.
    ///
    /// Safe to call with no session, paused, or buffering: the
    /// anchor logic clears it outside `Playing`, and the paused arm
    /// of the advance never renders (the null device pulls nothing).
    pub fn pump_wallclock(&mut self) {
        let now = std::time::Instant::now();
        self.pump_wallclock_at(now);
    }

    /// The injectable-now half of [`CliHost::pump_wallclock`] (the
    /// determinism discipline: tests drive synthetic instants, never
    /// real sleeps — same posture as the event-throttle seam).
    pub fn pump_wallclock_at(&mut self, now: std::time::Instant) {
        self.refresh_wall_anchor(now);
        self.pump_pipeline();
        self.advance_time_wall(now);
        self.drain_tail_at_end();
    }

    /// Keep the wall anchor truthful (the pacers' shared truth):
    /// `Some` exactly while `Playing`, re-anchored at every entry and
    /// on every rate change (the clock's own anchor rate is the
    /// comparison truth — when the set-rate command lands, the new
    /// rate re-anchors here and the due math follows it).
    fn refresh_wall_anchor(&mut self, now: std::time::Instant) {
        let playing = self.state() == PlaybackState::Playing;
        let device_frames = self.device.as_ref().map(NullDevice::sample_count);
        let clock_rate = self.clock.as_ref().map(|clock| clock.anchor().rate);
        match (self.wall_anchor, playing, device_frames, clock_rate) {
            // Not playing (or nothing to pace): clear — every Playing
            // entry re-anchors below. Pause, underrun-buffering and
            // the seek's frozen window all land here.
            (_, false, _, _) => self.wall_anchor = None,
            // Playing with a live anchor and the same rate: keep.
            (Some(anchor), true, Some(_), Some(rate)) if rate == anchor.rate => {}
            // Playing (first tick, a re-entry, or a rate change):
            // anchor at the device's counter NOW — the wall owes zero
            // at this instant. The clock's position is its own truth;
            // the anchor only measures elapsed wall time from here.
            (_, true, Some(frames), rate) => {
                self.wall_anchor = Some(WallAnchor {
                    at: now,
                    frames,
                    rate: rate.unwrap_or(Rate::ONE),
                });
            }
            // Playing but no device/clock yet (the limb-less gap):
            // nothing to pace on the audio axis — the limb-less arm
            // of the wall advance does not read the anchor.
            (_, true, None, _) => self.wall_anchor = None,
        }
    }

    /// The wall-bounded advance (the pacers' shared body — the same
    /// vsync-then-advance order and drift observation as
    /// [`CliHost::advance_time`], with the render loop bounded by the
    /// anchor's due-frames instead of draining the ring whole).
    fn advance_time_wall(&mut self, now: std::time::Instant) {
        if self.clock.as_ref().is_some_and(|clock| clock.is_paused()) {
            return;
        }
        let consumer = match self.consumer.as_ref() {
            Some(consumer) => Arc::clone(consumer),
            None => return,
        };
        let channels = self.audio_channels();
        // The device's wall due: FRAMES the wall has become owed since
        // the anchor — elapsed ns × playback rate × device rate / 1e9
        // (u128 intermediate; the saturating conversions keep the
        // discipline at the extremes).
        let rate_hz = u128::from(self.audio_rate_hz());
        let due_frames = self.wall_anchor.map(|anchor| {
            let elapsed_ns = u128::from(now.saturating_duration_since(anchor.at).as_nanos() as u64);
            let numer = elapsed_ns * u128::from(anchor.rate.num) * rate_hz;
            let denom = u128::from(anchor.rate.den) * 1_000_000_000;
            anchor
                .frames
                .saturating_add(u64::try_from(numer / denom).unwrap_or(u64::MAX))
        });
        match self.device.as_mut() {
            Some(device) => {
                let mut guard = consumer.lock();
                while device.buffered_frames() >= RENDER_QUANTUM
                    && due_frames
                        .is_some_and(|due| device.sample_count() + (RENDER_QUANTUM as u64) <= due)
                {
                    // Same vsync-then-advance order as `advance_time`
                    // (the slot decision sees the clock at the slot
                    // boundary; a frame presents exactly when its slot
                    // arrives).
                    if let Err(error) = guard.tick_video() {
                        guard.note_video_error(error);
                        break;
                    }
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
                    if let Err(error) = device.render(&mut buffer) {
                        guard.note_video_error(error);
                        break;
                    }
                    guard.observe_audio_drift();
                }
            }
            None => {
                // No audio limb: the bounded scheduler slice per pump
                // (wall-clock presents — the monotonic master paces
                // itself; the anchor is not read on this arm).
                let mut guard = consumer.lock();
                for _ in 0..16 {
                    if let Err(error) = guard.tick_video() {
                        guard.note_video_error(error);
                        break;
                    }
                }
            }
        }
    }

    /// Drain the engine's output hub: receipts stashed for matching,
    /// events converted to wire, staleness-filtered and recorded.
    /// The buffer is REUSED across pumps (D-1): take → clear →
    /// `drain_outputs_into` → drain while processing → move back —
    /// the Vec's allocation survives, steady state allocates
    /// nothing per drain.
    fn drain_outputs(&mut self) {
        let mut outputs = std::mem::take(&mut self.outbox_scratch);
        outputs.clear();
        self.engine.drain_outputs_into(&mut outputs);
        for output in outputs.drain(..) {
            match output {
                EngineOutput::Receipt(receipt) => self.pending_receipts.push(receipt),
                EngineOutput::Event(event) => {
                    if event.kind.is_pipeline_scoped() {
                        // Record the machine's trail before the wire
                        // conversion (the engine-private kinds included —
                        // they never reach the wire, doc 05 §4.2).
                        if let EngineEventKind::StateChanged { from, to } = &event.kind {
                            self.state_history
                                .push((session_state_as_wire(*from), session_state_as_wire(*to)));
                        }
                        if let EngineEventKind::Fatal { error } = &event.kind {
                            self.last_fatal = Some(wire_error(error));
                        }
                        if let EngineEventKind::TracksChanged { tracks, selected } = &event.kind {
                            let facts = self.facts.as_ref();
                            self.tracks = tracks
                                .iter()
                                .zip(selected.iter())
                                .map(|(track, &is_selected)| {
                                    enrich_track_info(
                                        track_info_wire_host(track, is_selected),
                                        facts.and_then(|facts| facts.fact_for(track.id)),
                                    )
                                })
                                .collect();
                        }
                    }
                    if let Some(wire) = event.as_wire_event()
                        && self.staleness.filter_event(&wire)
                    {
                        self.events.push(wire);
                    }
                }
            }
        }
        self.outbox_scratch = outputs;
    }

    /// Advance media time: render audio quanta (each moves the sample
    /// counter — the clock's truth), ticking the video scheduler once
    /// per quantum. With no audio limb (video-only or `--no-audio`)
    /// the clock is the system monotonic and only the scheduler is
    /// ticked (wall-clock-paced presents).
    ///
    /// **Paused clock = the device pulls nothing.** The engine freezes
    /// the session clock on `pause`/underrun (`Effect::FreezeClock`);
    /// rendering a quantum anyway would (a) spin forever — a render
    /// under a frozen `MediaClock` still consumes the ring but the
    /// loop's `buffered_frames` predicate would never fall below the
    /// quantum — and (b) desynchronize: `MediaClock::resume` re-anchors
    /// at the frozen position, so audio consumed while paused is
    /// positionally skipped. Skipping renders while the clock is
    /// frozen keeps the ring's content and the clock's position in
    /// lockstep; `resume` continues exactly where the pause landed
    /// (the 2.C3 pause test's own pattern).
    fn advance_time(&mut self) {
        if self.clock.as_ref().is_some_and(|clock| clock.is_paused()) {
            return;
        }
        let consumer = match self.consumer.as_ref() {
            Some(consumer) => Arc::clone(consumer),
            None => return,
        };
        let channels = self.audio_channels();
        match self.device.as_mut() {
            Some(device) => {
                let mut guard = consumer.lock();
                while device.buffered_frames() >= RENDER_QUANTUM {
                    // Tick BEFORE the render: the scheduler's slot
                    // decision must see the clock as it stands at the
                    // slot boundary (the vsync-then-advance order — a
                    // frame presents exactly when its slot arrives,
                    // never one quantum late).
                    if let Err(error) = guard.tick_video() {
                        guard.note_video_error(error);
                        break;
                    }
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
                    if let Err(error) = device.render(&mut buffer) {
                        guard.note_video_error(error);
                        break;
                    }
                    // Card 3.B1's loop closure (W1-18 §7 step 8): one
                    // drift observation per rendered quantum — the
                    // device just consumed, so the fill this sample
                    // sees is the steady-state residue. Counters only,
                    // never events; an unarmed governor ignores it.
                    guard.observe_audio_drift();
                }
            }
            None => {
                // No audio limb: tick the scheduler a bounded slice per
                // pump (wall-clock presents — see the module docs).
                let mut guard = consumer.lock();
                for _ in 0..16 {
                    if let Err(error) = guard.tick_video() {
                        guard.note_video_error(error);
                        break;
                    }
                }
            }
        }
    }

    /// The audio limb's channel count (render-buffer sizing; the
    /// harvested track geometry — any audio codec, stereo default).
    fn audio_channels(&self) -> usize {
        match self.facts.as_ref().and_then(|facts| {
            facts
                .facts
                .iter()
                .find(|fact| fact.kind == MediaKind::Audio)
                .and_then(|fact| fact.audio_geometry())
        }) {
            Some((channels, _rate)) => channels as usize,
            None => 2,
        }
    }

    /// The audio limb's sample rate (the wall pacer's due-frames
    /// arithmetic — frames the wall owes per elapsed second). The
    /// harvested track geometry, mono/stereo-agnostic; 44 100 when
    /// no audio fact carries geometry (the same default the render
    /// buffer's channel count falls back to).
    fn audio_rate_hz(&self) -> u32 {
        match self.facts.as_ref().and_then(|facts| {
            facts
                .facts
                .iter()
                .find(|fact| fact.kind == MediaKind::Audio)
                .and_then(|fact| fact.audio_geometry())
        }) {
            Some((_channels, rate)) => rate,
            None => 44_100,
        }
    }

    /// **The end-of-media tail play-out.** The engine stops the
    /// session clock when the *pipeline* drains (`Drained → Ended`),
    /// but the null ring may still hold the audio tail — the decode
    /// side runs ahead of the device by up to the ring's content. A
    /// real device keeps pulling its callback until the buffer empties
    /// (then silence); the null equivalent is: resume the clock, render
    /// the residue (each render moving the counter — the clock's
    /// truth), then freeze again. The observable position reaches the
    /// media end exactly; the final render is sized to the residue so
    /// no sub-quantum remainder is stranded.
    fn drain_tail_at_end(&mut self) {
        if self.engine.state() != SessionState::Ended {
            return;
        }
        let channels = self.audio_channels();
        let consumer = match self.consumer.as_ref() {
            Some(consumer) => Arc::clone(consumer),
            None => return,
        };
        let clock = match self.clock.as_ref() {
            Some(clock) => Arc::clone(clock),
            None => return,
        };
        let Some(device) = self.device.as_mut() else {
            return;
        };
        if device.buffered_frames() == 0 {
            return;
        }
        clock.resume();
        let mut guard = consumer.lock();
        while device.buffered_frames() > 0 {
            let take = device.buffered_frames().min(RENDER_QUANTUM);
            // Same vsync-then-advance order as `advance_time`.
            if let Err(error) = guard.tick_video() {
                guard.note_video_error(error);
                break;
            }
            let mut buffer = vec![0.0_f32; take * channels];
            if let Err(error) = device.render(&mut buffer) {
                guard.note_video_error(error);
                break;
            }
        }
        clock.pause();
    }

    /// Surface a present-path failure once (recorded as a wire-visible
    /// fatal marker the runner prints; audio continues — the 2.D1
    /// degrade pattern).
    fn surface_video_error(&mut self) {
        if let Some(consumer) = self.consumer.as_ref()
            && let Some(error) = consumer.take_video_error()
        {
            self.last_fatal = Some(wire_error(&error));
        }
    }

    // -- command routing (the dispatch table's engine half) --------------

    /// Allocate the next request id (doc 05 §4.1 — host-assigned,
    /// echoed by the engine).
    #[must_use]
    pub fn next_request(&mut self) -> RequestId {
        self.allocator.allocate()
    }

    /// Submit one minimal-set command and pump until its receipt
    /// arrives (the truthful-UI correlation point).
    pub(crate) fn submit_and_await(
        &mut self,
        kind: SessionCommandKind,
    ) -> Result<CommandReceipt, CommandError> {
        let request = self.allocator.allocate();
        let handle = self.handle.as_ref().ok_or_else(|| {
            CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
        })?;
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
        self.await_receipt(request)
    }

    /// Translate one receipt into the truthful-UI outcome.
    pub(crate) fn receipt_outcome(
        &self,
        receipt: CommandReceipt,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        match receipt.result {
            ReceiptResult::Accepted {
                session,
                generation,
            } => Ok(CommandOutcome::Accepted {
                session: Some(session),
                generation,
            }),
            ReceiptResult::Completed { session } => {
                let _ = session;
                Ok(CommandOutcome::Completed(CommandResult::Unit))
            }
            ReceiptResult::Rejected { state, reason } => Err(rejection_wire(state, reason)),
        }
    }

    /// The `session.open` path (also the seek reopen).
    pub(crate) fn open_session(
        &mut self,
        source: commands::catalog::types::SourceSpec,
        start_time_us: Option<i64>,
        no_video: bool,
        no_audio: bool,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        if self.engine.session_id().is_some() {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "a session is already open — close it first (source replacement is card 3.A2)",
            ));
        }
        let opened: OpenedMedia = open_source(&source, start_time_us, no_video, no_audio)?;
        let facts = Arc::new(opened.facts);
        let anchor_ns = opened.start_position_ns;
        // F0.3 (W13-A02 F-001): read the engine's own seek-capability
        // fact before the demuxer moves into the engine's
        // `SessionConfig` — the demuxer's declared `SeekSupport`, the
        // same truth `Pipeline::seekability` consults for the seek
        // flow's step 1 and the `media.info-ready` event carries
        // (`None` sources refuse seeks typed, never a fake success).
        let seekable = opened.demuxer.seekability() != SeekSupport::None;
        // The audio half (and with it the session clock) exists only
        // when an audio track survived selection — through the
        // GENERIC geometry lookup (card 6.OPW-ogg's seam fix: any
        // audio-kind track whose harvest names channels + rate —
        // the PCM strf pair, the Vorbis ID chain, OpusHead — not
        // just the PCM pair; a non-PCM session without geometry
        // opens limb-less on the wall clock, never a guess).
        let audio_facts: Option<(u32, u32)> = facts
            .facts
            .iter()
            .find(|fact| fact.kind == MediaKind::Audio)
            .and_then(|fact| fact.audio_geometry());
        let (audio_limb, device, clock) = match audio_facts {
            Some((channels, rate)) => {
                let AudioSession {
                    limb,
                    device,
                    clock,
                    ..
                } = crate::pipeline_host::open_audio_session(channels, rate, anchor_ns)
                    .map_err(|err| wire_error(&err))?;
                (Some(limb), Some(device), clock)
            }
            None => (
                None,
                None,
                crate::pipeline_host::wall_clock(anchor_ns).map_err(|err| wire_error(&err))?,
            ),
        };
        let consumer = Arc::new(CliFrameConsumer::new(
            audio_limb,
            Arc::clone(&facts),
            Arc::clone(&clock),
        ));
        // The pre-open stash rides the first session's consumer from
        // its first present (the desktop binds its frame channel
        // before opening; a later bind installs mid-session via the
        // same seam — CliFrameConsumer::set_present_tap is state-safe).
        if let Some(tap) = self.pending_present_tap.clone() {
            consumer.set_present_tap(tap);
        }
        let mut provider = HostDecoderProvider::new(Arc::clone(&facts));
        // The drain-gate probe: the PCM decoder's `Drained` answer
        // waits on the null device's ring actually emptying (see
        // DrainGatedDecoder — headless time moves only via `advance`).
        provider.attach_consumer(Arc::clone(&consumer));
        let provider = Arc::new(provider);
        let mut config = SessionConfig::new(
            opened.uri,
            opened.demuxer,
            provider,
            Arc::clone(&consumer) as Arc<dyn playback_engine::FrameConsumer>,
            Arc::clone(&clock),
        );
        // Card 3.C2 (defect G1): the probe's container truth rides the
        // config so the engine's `NoDecodableTrack` failure can name
        // the format (the `--no-video --no-audio` all-disabled path is
        // the reachable production case).
        config.container = Some(facts.container.clone());
        // Card 3.B3: per-kind queue caps from the product limits —
        // the open path consumes the product posture (1024/512 packet
        // queues) instead of the library defaults (200/300). The
        // values ride the product.toml mirror constants until the
        // product-config load lands (see `demux_host`'s `[PROPOSAL]`).
        config.queues = crate::demux_host::queue_plan_from_product_limits(
            crate::demux_host::PRODUCT_QUEUE_PACKETS_AUDIO,
            crate::demux_host::PRODUCT_QUEUE_PACKETS_VIDEO,
        )?;
        if let Some(source) = self.event_throttle.clone() {
            config.throttle = playback_engine::EventPolicy {
                tick: playback_engine::TimeTickPolicy::default_policy(),
                throttle: source,
            };
        }
        let handle = self
            .engine
            .open(config)
            .map_err(|err| session_error_wire(&err))?;
        self.handle = Some(handle);
        self.consumer = Some(consumer);
        self.device = device;
        self.clock = Some(clock);
        self.facts = Some(facts);
        self.seekable = Some(seekable);
        self.source = Some(source);
        self.tracks.clear();
        // Pump the open to a settled state (Opening → Buffering → …).
        // The truthful answer for the class-A open is `Accepted` —
        // but an immediate open failure (corrupt header, no decodable
        // track) must surface in THIS response, so we pump until the
        // engine leaves `Opening` and check the fatal trail.
        self.pump_pipeline_until(|host| host.state() != PlaybackState::Opening, AWAIT_PUMPS);
        if let Some(error) = self.last_fatal.take() {
            self.teardown_after_fatal();
            return Err(error);
        }
        let session = self.engine.session_id();
        let generation = self.engine.generation();
        Ok(CommandOutcome::Accepted {
            session,
            generation: generation.unwrap_or(Generation(0)),
        })
    }

    /// After a fatal open failure the engine sits in `Error` with a
    /// dead session — close it so the host is reusable (the wire's
    /// `error → idle` edge).
    fn teardown_after_fatal(&mut self) {
        if self.state() == PlaybackState::Error {
            let _ = self.submit_and_await(SessionCommandKind::Close);
        }
        self.teardown_session_scoped();
    }

    /// Clear every session-scoped host half (after a settled Close or
    /// a fatal): the snapshot answers (`time`, `info`) then honestly
    /// refuse with not-ready instead of answering from a dead
    /// session's leftover clock/facts.
    fn teardown_session_scoped(&mut self) {
        self.handle = None;
        self.consumer = None;
        self.device = None;
        self.clock = None;
        self.facts = None;
        self.seekable = None;
        self.source = None;
        self.tracks.clear();
        self.pending_receipts.clear();
    }

    /// A settled Close: the engine went to Idle — drop the host's
    /// session-scoped halves so later snapshots refuse honestly.
    pub(crate) fn session_closed(&mut self) {
        self.teardown_session_scoped();
    }

    /// The `playback.seek` path — the engine's native seek flow (see
    /// the module docs). The session, the device, the ring, the
    /// decoders and the observability trail all survive: the host
    /// submits the `Seek` command, performs the boundary duties
    /// engine-side flush cannot reach ([`Self::seek_boundary`]) and
    /// awaits the terminal event (the landing truth, class A).
    ///
    /// # Errors
    /// Typed at each honest stage: no session (`not-ready`), the
    /// state table's refusal (`rejection_wire` — e.g. seek from
    /// `Ended` needs the doc 05 §3.3 wire edge), a submit failure
    /// (`session_error_wire`), or the bounded terminal wait expiring
    /// (`internal` — the engine never answered). The flow's own
    /// typed failures (unseekable source, unresolvable target,
    /// reanchor refusal) are NOT errors here: they answer the class-A
    /// accept and surface as the `timing.seek-failed` event with the
    /// old position (contract D4 — never a fake success, never a
    /// reopen).
    pub(crate) fn dispatch_seek(
        &mut self,
        target: commands::catalog::types::SeekTarget,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        use commands::catalog::types::SeekTarget as WireTarget;
        // The wire target → the engine's. Resolution and clamping are
        // the ENGINE's (doc 05 §2.4: clamping is truth — the engine
        // clamps against the live pipeline's duration and the
        // position); the host no longer pre-resolves. Chapters stay
        // the host's typed refusal: no chapter source exists anywhere
        // in the vertical (seek.rs: "hosts resolve chapters to
        // `Absolute`" — with nothing to resolve against, not-found is
        // the honest answer, unchanged from 2.D2).
        let engine_target = match target {
            WireTarget::Absolute { time } => SeekTarget::Absolute {
                position_ns: time.0.saturating_mul(1000),
            },
            WireTarget::Relative { delta_ms } => SeekTarget::Relative {
                delta_ns: delta_ms.saturating_mul(1_000_000),
            },
            WireTarget::Percent { fraction } => SeekTarget::Percent { fraction },
            WireTarget::Chapter { index } => {
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::NotFound,
                    format!(
                        "no chapter {index}: AVI v0 carries no chapter markers (the chapter card owns this)"
                    ),
                ));
            }
        };
        let handle = self.handle.as_ref().ok_or_else(|| {
            CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
        })?;
        let request = self.allocator.allocate();
        // Snapshot the event stream: THIS seek's terminal is the first
        // `timing.seek-completed`/`timing.seek-failed` observed after
        // submission (a superseded seek is silent by contract — doc 05
        // §4.2 — so the first terminal is always the live one's).
        let terminal_from = self.events.len();
        // The pre-seek ring content: exactly the frames that must NOT
        // survive the boundary (doc 03 §6 step 3 “flush … ring (to
        // silence)”). Snapshot BEFORE submitting — nothing writes
        // between here and the accept poll (the host is the single
        // writer), and the accept poll's own decode appends only
        // landing frames BEHIND this prefix (the ring is FIFO).
        let stale_ring_frames = self.device.as_ref().map_or(0, NullDevice::buffered_frames);
        handle
            .seek(request, engine_target)
            .map_err(|error| session_error_wire(&error))?;
        // 1. The class-A receipt (accept under the bumped generation,
        //    or the state table's typed refusal).
        let receipt = self.await_receipt(request)?;
        let outcome = self.receipt_outcome(receipt)?;
        // 2. The seek boundary's host duties — only on accept, and
        //    only when the pipeline actually moved: a seek that
        //    failed AT the accept poll (unseekable source,
        //    unresolvable target, reanchor refusal — the engine emits
        //    those synchronously, so the terminal is already in the
        //    drained events) rolled back per contract D4 and the
        //    ring's pre-seek content is still LIVE audio at the old
        //    position; discarding it would punch a hole in
        //    playable-at-old-position. A seek still in flight (or one
        //    that completed within the accept poll — see
        //    [`Self::seek_boundary`]) moved the demuxer: the prefix
        //    dies here.
        if let CommandOutcome::Accepted { generation, .. } = &outcome {
            let failed_at_accept = self.events[terminal_from..]
                .iter()
                .any(|event| matches!(event.kind, EventKind::TimingSeekFailed { .. }));
            if !failed_at_accept {
                self.seek_boundary(*generation, stale_ring_frames)?;
            }
        }
        // 3. The terminal: `Seeked{requested, actual}` (the landing
        //    truth — the clock re-anchored at the first audio frame's
        //    own timestamp) or `SeekFailed` (contract D4). Bounded
        //    pipeline-only pumps; the pre-roll completes on the tick
        //    path (the Paused decode carve-out included).
        let landed = self.pump_pipeline_until(
            |host| host.events[terminal_from..].iter().any(is_seek_terminal),
            AWAIT_PUMPS,
        );
        if !landed {
            return Err(CommandError::new(
                Stage::Control,
                ErrorCode::Internal,
                "the engine never reported the seek terminal (bounded wait expired)",
            ));
        }
        Ok(outcome)
    }

    /// Await one submitted command's receipt (pipeline-only pumps; the
    /// shared half of every `submit_and_await` shape).
    fn await_receipt(&mut self, request: RequestId) -> Result<CommandReceipt, CommandError> {
        for _ in 0..AWAIT_PUMPS {
            self.pump_pipeline();
            if let Some(position) = self
                .pending_receipts
                .iter()
                .position(|receipt| receipt.request == request)
            {
                return Ok(self.pending_receipts.remove(position));
            }
        }
        Err(CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            "the engine never answered the command (bounded wait expired)",
        ))
    }

    /// The seek boundary's host duties (doc 03 §6 step 3 — the halves
    /// behind this host's seams; see the module docs): the audio
    /// ring's pre-seek prefix (`stale_ring_frames`) is discarded, the
    /// window-full hold is cleared, and the presentation queue's
    /// generation is published (the renderer-mailbox clear). Called
    /// once per accepted-and-moving seek, after the receipt pump — a
    /// cold path, never per tick (D-1).
    ///
    /// # The fast-completion compensation
    ///
    /// The engine's `poll` drains the inbox (the accept: mint, flush,
    /// reanchor) and THEN ticks the pipeline in the same call — so a
    /// seek whose pre-roll completes in its first tick (the common
    /// case: every MJPEG frame is a keyframe, one decode slice
    /// delivers both limbs) has ALREADY re-anchored the clock at the
    /// landing by the time the host regains control. The discard
    /// renders advance the device sample counter (the clock's truth
    /// source) AFTER that anchor — unabsorbed, the discarded tail
    /// would fabricate a position overshoot of exactly its length.
    /// The compensation: snapshot the clock BEFORE discarding (it
    /// reads the completion's landing truth — the engine just
    /// re-anchored it), discard, then re-anchor at the snapshot so
    /// the new anchor adopts the post-discard counter. The slow
    /// pre-roll path needs no compensation (its completion re-anchor
    /// runs after the discard, on a later pump).
    ///
    /// # Errors
    /// The clock re-anchor's typed failure (an unrepresentable
    /// position — the compensation is honest, never silent).
    fn seek_boundary(
        &mut self,
        generation: Generation,
        stale_ring_frames: usize,
    ) -> Result<(), CommandError> {
        // The EXACT old-generation prefix (6.FIX-ogg-reanchor): the
        // consumer's write-order bookkeeping — the ring's cumulative
        // written count at the first new-generation frame's arrival
        // (or the whole cumulative written when the landing has not
        // staged yet), minus the device counter's consumed frames —
        // NOT the submit-time snapshot, which misses the frames the
        // accept window's in-flight decode staged between submission
        // and accept (measured overrun: ~1.0 s of pre-seek audio
        // surviving a vorbis seek, the clock at 2.54 s of a 1.5 s
        // file). The snapshot stays only as the no-consumer fallback.
        let mint = media_types::GenerationId::new(generation.0);
        let consumed = self.device.as_ref().map_or(0, NullDevice::sample_count);
        let stale_frames = match self.consumer.as_ref() {
            Some(consumer) => consumer.lock().audio_stale_prefix(mint, consumed) as usize,
            None => stale_ring_frames,
        };
        let landed_ns = self.clock.as_ref().map(|clock| clock.now());
        let discarded = self.discard_ring_prefix(stale_frames);
        if discarded > 0
            && let (Some(clock), Some(landed)) = (self.clock.as_ref(), landed_ns)
        {
            // Absorb the discard's counter movement into the anchor.
            clock
                .reanchor(landed)
                .map_err(|error| wire_error(&media_types::EngineError::from(error)))?;
        }
        if let Some(consumer) = self.consumer.as_ref() {
            consumer
                .lock()
                .seek_boundary(media_types::GenerationId::new(generation.0));
        }
        Ok(())
    }

    /// The ring-flush seam (doc 03 §6 step 3 “flush … ring (to
    /// silence)”): discard exactly the first `frames` frames of the
    /// null device's ring with discarded renders — the device's own
    /// consumption mechanism (the null device has no audible output;
    /// a discarded render is exactly the nothing a real device's
    /// flush API would play). Exact-prefix, not whole-ring: the
    /// accept poll's decode may already have appended landing frames
    /// behind the stale prefix (FIFO), and those MUST survive — they
    /// are the first audio of the new position. Bounded by the
    /// requested prefix over the render quantum, once per seek.
    /// Answers how many frames were actually discarded.
    ///
    /// `[PROPOSAL: the null-device render-discard is this host's
    /// flush; a real backend gets a true device-flush API on the
    /// audio-output seam (drop buffered content WITHOUT advancing
    /// the sample counter — then the clock compensation disappears) —
    /// registered as the engine-side follow-up of W1-06, with the CLI
    /// switching to it when the backends land]`
    fn discard_ring_prefix(&mut self, frames: usize) -> usize {
        if frames == 0 {
            return 0;
        }
        let channels = self.audio_channels();
        let Some(device) = self.device.as_mut() else {
            return 0;
        };
        let mut remaining = frames.min(device.buffered_frames());
        let requested = remaining;
        // The exact-residue pattern of `drain_tail_at_end`: one sized
        // discard buffer per render, no underrun counted (the ring is
        // never over-read).
        while remaining > 0 {
            let take = remaining.min(RENDER_QUANTUM);
            let mut discard = vec![0.0_f32; take * channels];
            if device.render(&mut discard).is_err() {
                break;
            }
            remaining -= take;
        }
        requested - remaining
    }

    /// The class-S snapshot answers.
    pub(crate) fn media_info(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            ));
        };
        let duration_us = facts.duration_us();
        Ok(CommandResult::MediaInfo {
            info: MediaInfo {
                source: self.source.clone().unwrap_or_else(|| {
                    commands::catalog::types::SourceSpec::File {
                        path: std::path::PathBuf::new(),
                    }
                }),
                container: Some(facts.container.clone()),
                title: None,
                duration: match duration_us {
                    Some(us) => Duration::Known {
                        time: Timestamp(us),
                    },
                    None => Duration::Unknown,
                },
                // F0.3 (W13-A02 F-001): the engine's declared
                // `SeekSupport` read at open time — the retired hard
                // `true` contradicted the engine's own event and the
                // seek flow in the same run. `None` is the no-session
                // shape (unreachable behind the `facts` gate above)
                // and answers the honest `false`.
                seekable: self.seekable.unwrap_or(false),
                is_live: false,
                metadata: BTreeMap::new(),
                tracks: self.tracks.clone(),
                chapters: Vec::<Chapter>::new(),
            },
        })
    }

    /// The `media.get-duration` snapshot (class S).
    pub(crate) fn duration(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            ));
        };
        Ok(CommandResult::Duration {
            duration: match facts.duration_us() {
                Some(us) => Duration::Known {
                    time: Timestamp(us),
                },
                None => Duration::Unknown,
            },
        })
    }

    pub(crate) fn current_time(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            ));
        };
        let position_us = self.position_us();
        let duration_us = facts.duration_us();
        let fraction = match duration_us {
            Some(duration) if duration > 0 => {
                position_us.clamp(0, duration) as f32 / duration as f32
            }
            _ => 0.0,
        };
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
        Ok(CommandResult::CurrentTime {
            time: Timestamp(position_us.max(0)),
            position: fraction,
            buffered,
        })
    }

    // -- the audio-control verbs (§2.6 — F1.1c, the live controls seam) ----

    /// The `audio.set-volume` verb (class S — the response says what
    /// IS, §2.6): validate at the control boundary, then apply on the
    /// LIVE session's master chain and answer the chain's own
    /// read-back, never the request echoed. W07-A02's finding was
    /// `audio.set-volume` refused by every shipped host with zero
    /// production callers of `set_volume` — this is the routing the
    /// FX3-A03 engine surface waited for.
    ///
    /// # Errors
    /// Typed at each honest boundary: a non-finite or negative volume
    /// (the engine's own `audio.gain_*` codes, mapped through
    /// [`wire_error`] — W07-A02 F-001's discipline: never a clamp),
    /// no session (`not-ready`), and a session without an audio limb
    /// (video-only or limb-less open — [`no_audio_limb`], the ADTS
    /// family's recorded gap).
    pub(crate) fn audio_set_volume(
        &mut self,
        volume: Volume,
    ) -> Result<CommandResult, CommandError> {
        let gain = audio_engine::Volume::from_linear(volume).map_err(|error| wire_error(&error))?;
        let consumer = self.consumer.as_ref().ok_or_else(no_session_open)?;
        let mut guard = consumer.lock();
        let controls = audio_limb_controls(&mut guard).ok_or_else(no_audio_limb)?;
        controls.set_volume(gain);
        Ok(CommandResult::AudioVolume {
            volume: controls.volume().linear(),
        })
    }

    /// The `audio.get-volume` verb (class S — the hydration read,
    /// §2.6's "prefer the snapshot on hydration"): answers the live
    /// chain's getter, never a remembered request — a fresh session
    /// answers the identity (`1.0`).
    ///
    /// # Errors
    /// The same typed boundaries as [`CliHost::audio_set_volume`]
    /// (no session; a session with no audio limb).
    pub(crate) fn audio_get_volume(&self) -> Result<CommandResult, CommandError> {
        let consumer = self.consumer.as_ref().ok_or_else(no_session_open)?;
        let mut guard = consumer.lock();
        let controls = audio_limb_controls(&mut guard).ok_or_else(no_audio_limb)?;
        Ok(CommandResult::AudioVolume {
            volume: controls.volume().linear(),
        })
    }

    /// The `audio.set-mute` verb (class S, independent of volume,
    /// §2.6): `true` zeroes every mixed quantum; `false` restores
    /// the signal bit-exactly — the stage is a stateless multiply by
    /// zero, so mute never disturbs the volume or the EQ state
    /// (FX3-A03's composition property, kept honest here).
    ///
    /// # Errors
    /// The same typed boundaries as [`CliHost::audio_set_volume`]
    /// (no session; a session with no audio limb).
    pub(crate) fn audio_set_muted(&mut self, muted: bool) -> Result<CommandResult, CommandError> {
        let consumer = self.consumer.as_ref().ok_or_else(no_session_open)?;
        let mut guard = consumer.lock();
        let controls = audio_limb_controls(&mut guard).ok_or_else(no_audio_limb)?;
        controls.set_muted(muted);
        Ok(CommandResult::AppliedBool {
            value: controls.muted(),
        })
    }

    /// The settings-schema seam: one setting's value + meta.
    pub(crate) fn setting_get(&self, area: &str, key: &str) -> Result<CommandResult, CommandError> {
        let full = format!("{area}.{key}");
        let invalid =
            |message: String| CommandError::new(Stage::Config, ErrorCode::InvalidArgument, message);
        let def =
            settings_schema::runtime::schema_meta(&full).map_err(|err| invalid(err.to_string()))?;
        let value = match self.settings.get(&full) {
            Some(stored) => stored.clone(),
            None => default_value(def),
        };
        let mut meta = BTreeMap::new();
        meta.insert("type".to_string(), setting_type_name(def).to_string());
        meta.insert(
            "restart_required".to_string(),
            def.restart_required.to_string(),
        );
        meta.insert("flag".to_string(), def.cli.flag.clone());
        Ok(CommandResult::SettingResult {
            value,
            schema_meta: meta,
        })
    }

    /// The settings-schema seam: validate + store one setting.
    pub(crate) fn setting_set(
        &mut self,
        area: &str,
        key: &str,
        value: SettingValue,
    ) -> Result<CommandResult, CommandError> {
        let full = format!("{area}.{key}");
        let invalid =
            |message: String| CommandError::new(Stage::Config, ErrorCode::InvalidArgument, message);
        let def =
            settings_schema::runtime::schema_meta(&full).map_err(|err| invalid(err.to_string()))?;
        let toml_value = setting_value_to_toml(&value);
        settings_schema::runtime::validate_value(&full, &toml_value)
            .map_err(|err| invalid(err.to_string()))?;
        self.settings.insert(full, value.clone());
        Ok(CommandResult::SettingApplied {
            applied: value,
            restart_required: def.restart_required,
        })
    }
}

// ---------------------------------------------------------------------------
// The audio-control verbs' helpers (F1.1c — the W07 "dead audio-controls
// surface", host half)
// ---------------------------------------------------------------------------

/// The no-session refusal (the class-S snapshot shape `media_info`
/// answers with).
fn no_session_open() -> CommandError {
    CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
}

/// The no-audio-limb refusal: a session that opened without an audio
/// half (video-only media, `--no-audio`, or the limb-less geometry
/// gap the ADTS family records) carries no live controls chain to
/// drive — the honest typed answer, never a fabricated default
/// applied nowhere.
fn no_audio_limb() -> CommandError {
    CommandError::new(
        Stage::AudioOutput,
        ErrorCode::NotReady,
        "this session has no audio output to control (video-only or limb-less open)",
    )
}

/// The live audio session's controls (the frozen FX4-A01 contract):
/// the host-facing audio session exposes `audio_controls()` — the
/// runtime master chain FX3-A03 activated on the streaming pipeline
/// (volume/mute/balance/EQ), reached through the consumer's locked
/// state, the same audio-half seam the pump's renders read. The
/// engine's own docs name `AudioLimb` as the host seam (audio-engine
/// `controls.rs`), so the limb twin of `ConsumerState::audio` carries
/// the entry. Setters run between quanta: the next mixed quantum
/// reads the new parameters — never a rebuild.
///
/// This is the ONE call site of the sibling seam in this file; if the
/// landed pipeline_host shape differs, it reconciles here alone.
fn audio_limb_controls<'guard>(
    guard: &'guard mut std::sync::MutexGuard<'_, crate::pipeline_host::ConsumerState>,
) -> Option<&'guard mut audio_engine::AudioControls> {
    guard
        .audio_mut()
        .map(crate::pipeline_host::AudioLimb::audio_controls)
}

/// The engine's own wire conversion of its session state (the mapping
/// is the identity — `SessionState::as_wire` — but that method is the
/// engine's; the CLI re-exports through it via this shim to avoid
/// depending on the private field).
fn session_state_as_wire(state: SessionState) -> PlaybackState {
    state.as_wire()
}

/// Whether one wire event is a seek flow terminal — the class-A
/// completion truth of `playback.seek` (doc 05 §3.2
/// `timing.seek-completed` / `timing.seek-failed`; exactly one
/// terminal per completed generation, §4.3 invariant 4).
fn is_seek_terminal(event: &WireEvent) -> bool {
    matches!(
        event.kind,
        EventKind::TimingSeekCompleted { .. } | EventKind::TimingSeekFailed { .. }
    )
}

/// The engine's `track_info_wire` — reused, not re-implemented
/// (one implementation of the L0→wire track mapping in the tree).
pub(crate) fn track_info_wire_host(track: &media_types::Track, is_selected: bool) -> TrackInfo {
    playback_engine::track_info_wire(track, is_selected)
}

/// Fold the open path's harvested facts into one wire track row: the
/// L0 [`Track`] deliberately carries no codec parameters, but the
/// demuxer's `strf` did — `media.get-info` owes the caller the
/// container's own declaration (dimensions, channel count, rates).
fn enrich_track_info(
    mut info: TrackInfo,
    fact: Option<&crate::demux_host::TrackFact>,
) -> TrackInfo {
    let Some(fact) = fact else {
        return info;
    };
    if let Some((width, height)) = fact.dimensions {
        info.video = Some(commands::catalog::types::VideoTrackInfo {
            width,
            height,
            frame_rate: fact.frame_ns.map(|ns| (1.0e9 / ns as f64) as f32),
            bit_depth: None,
            hdr: false,
        });
    }
    if let Some((channels, rate)) = fact.audio_geometry() {
        info.audio = Some(commands::catalog::types::AudioTrackInfo {
            channels: channels.min(u16::MAX as u32) as u16,
            sample_rate: Some(rate),
            bit_depth: None,
            codec_params: None,
        });
    }
    info
}

/// A rejection's wire form (doc 05 §2.4's error columns).
fn rejection_wire(state: SessionState, reason: playback_engine::RejectionReason) -> CommandError {
    let stage = Stage::Session;
    let state_name = session_state_as_wire(state);
    match reason {
        playback_engine::RejectionReason::NoSession => {
            CommandError::new(stage, ErrorCode::NotReady, "no session open")
        }
        playback_engine::RejectionReason::NotReady => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!("the session is still opening (state: {state_name:?})"),
        ),
        playback_engine::RejectionReason::SettleFirst => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "close from {state_name:?} composes through stop (the executor applies it; \
                 the table stays pure)"
            ),
        ),
        playback_engine::RejectionReason::AlreadySettled => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "the session already settled (state: {state_name:?}); close is the remaining exit"
            ),
        ),
        playback_engine::RejectionReason::RestartNeedsSeek => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "play in {state_name:?} means restart-from-ended — the Phase-3 seek flow (card 3.A1)"
            ),
        ),
        playback_engine::RejectionReason::SourceReplacementPhase3 => CommandError::new(
            stage,
            ErrorCode::NotReady,
            "open on a live session is source replacement (card 3.A2)",
        ),
        playback_engine::RejectionReason::SignalNotRaisedHere => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            format!(
                "a pipeline signal fired in a state whose driver never raises it ({state_name:?}) — bug report"
            ),
        ),
        playback_engine::RejectionReason::EndedNeedsWireEdge => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "seek in {state_name:?} needs the doc 05 §3.3 ended→buffering wire edge (Lead-owned change)"
            ),
        ),
        playback_engine::RejectionReason::MissingPayload => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            "a seek command arrived without its target payload — caller bug",
        ),
        playback_engine::RejectionReason::TrackNotFound => CommandError::new(
            Stage::Control,
            ErrorCode::NotFound,
            "track selection named a track this session does not have",
        ),
    }
}

/// A session-construction error's wire form.
fn session_error_wire(error: &SessionError) -> CommandError {
    match error {
        SessionError::Config(cause) => {
            CommandError::new(Stage::Session, ErrorCode::Unsupported, cause.to_string())
                .with_detail("code", "session.no-decodable-track")
        }
        SessionError::GenerationExhausted(engine) => wire_error(engine),
        SessionError::SessionClosed => CommandError::new(
            Stage::Session,
            ErrorCode::Canceled,
            "the session closed before the command landed",
        ),
        SessionError::InboxBusy => CommandError::new(
            Stage::Session,
            ErrorCode::ResourceLimit,
            "the session inbox is full — poll the engine, then retry",
        )
        .with_detail("limit", "inbox-capacity"),
        SessionError::Invariant(detail) => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            format!("engine invariant breach: {detail}"),
        ),
    }
}

/// The schema default as a wire value. `product.toml` references are
/// reported as their ref marker (the product-config join is owed to
/// the rebrand-propagation card) `[PROPOSAL]`.
fn default_value(def: &settings_schema::SettingDef) -> SettingValue {
    use settings_schema::model::DefaultValue;
    match &def.default {
        DefaultValue::Literal(value) => toml_to_setting_value(value),
        DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}")),
    }
}

fn toml_to_setting_value(value: &toml::Value) -> SettingValue {
    match value {
        toml::Value::Boolean(value) => SettingValue::Bool(*value),
        toml::Value::Integer(value) => SettingValue::Number(*value as f64),
        toml::Value::Float(value) => SettingValue::Number(*value),
        toml::Value::String(value) => SettingValue::String(value.clone()),
        toml::Value::Array(items) => SettingValue::List(
            items
                .iter()
                .map(|item| match item {
                    toml::Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .collect(),
        ),
        toml::Value::Table(_) => SettingValue::String("(table)".to_string()),
        toml::Value::Datetime(when) => SettingValue::String(when.to_string()),
    }
}

fn setting_value_to_toml(value: &SettingValue) -> toml::Value {
    match value {
        SettingValue::Bool(value) => toml::Value::Boolean(*value),
        SettingValue::Number(value) => {
            if value.fract() == 0.0 && value.abs() < 9.0e15 {
                toml::Value::Integer(*value as i64)
            } else {
                toml::Value::Float(*value)
            }
        }
        SettingValue::String(value) => toml::Value::String(value.clone()),
        SettingValue::List(items) => toml::Value::Array(
            items
                .iter()
                .map(|item| toml::Value::String(item.clone()))
                .collect(),
        ),
    }
}

fn setting_type_name(def: &settings_schema::SettingDef) -> &'static str {
    use settings_schema::model::SettingType;
    match def.ty {
        SettingType::Bool => "bool",
        SettingType::Int => "int",
        SettingType::Float => "float",
        SettingType::String => "string",
        SettingType::Enum => "enum",
        SettingType::Path => "path",
        SettingType::Url => "url",
        SettingType::List(_) => "list",
        SettingType::DurationMs => "duration-ms",
        SettingType::Percent => "percent",
    }
}

/// The wire track-kind of one dispatch family's lane kind (the
/// registry stores the engine-side [`MediaKind`]; the engine-info
/// wire row carries the catalog's [`TrackKind`]).
fn wire_kind(kind: MediaKind) -> TrackKind {
    match kind {
        MediaKind::Video => TrackKind::Video,
        MediaKind::Audio => TrackKind::Audio,
        MediaKind::Subtitle => TrackKind::Subtitle,
        MediaKind::Data => TrackKind::Data,
    }
}

/// The engine-info snapshot (class S, data-driven).
#[must_use]
pub fn engine_info() -> EngineInfo {
    EngineInfo {
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        catalog_version: "1".to_string(),
        protocol_version: "unwired (control-protocol is a later card)".to_string(),
        host: "cli".to_string(),
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
            "audio.null-device".to_string(),
            "video.software-compositor".to_string(),
        ],
        plugins: Vec::new(),
        // F0.2 (W13-A02 F-002): the decoder list DERIVES from the
        // dispatch registry — `pipeline_host`'s
        // [`LANDED_DECODER_FAMILIES`], the `decoder_for` match exported
        // as data and the same one source the `landed:` refusal string
        // derives from — never a hand-maintained snapshot. Pre-fix,
        // the hand-written `mjpeg`/`pcm_s16le` pair answered while the
        // same binary dispatched 14 families (h264+aac Playing
        // in-session). A session-less snapshot, so `active` is
        // honestly false for every family.
        decoders: LANDED_DECODER_FAMILIES
            .iter()
            .map(|family| DecoderInfo {
                name: family.name.to_string(),
                kind: wire_kind(family.kind),
                hardware: false,
                backend: None,
                active: false,
            })
            .collect(),
        sessions: Vec::new(),
    }
}

/// The command catalog listing (class S — the catalog as data).
#[must_use]
pub fn command_listings() -> Vec<CommandListing> {
    commands::catalog::catalog_data()
        .iter()
        .map(|spec| CommandListing {
            id: spec.id.to_string(),
            class: spec.class,
            since: "1".to_string(),
            deprecated: None,
            requires: Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod audio_limb_seam_tests {
    //! Card 6.OPW-ogg's shared-seam fix, proven end to end through
    //! the REAL `open_session`: the audio limb (device + clock) is
    //! built from the TRACK DESCRIPTOR's geometry, not the
    //! PCM-scoped pair — a non-PCM Vorbis session opens WITH a limb
    //! sized from the Vorbis ID header, the PCM lane keeps its limb
    //! (no regression), and an audio track with no geometry-bearing
    //! truth (ADTS's in-band config) stays honestly limb-less.

    use super::*;
    use std::path::PathBuf;

    fn fixture(root: &str, name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(root)
            .join(name)
    }

    /// A mono Vorbis-in-Ogg session opens WITH an audio limb — and
    /// the limb is sized 1-channel from the track geometry (the
    /// stereo default would answer 2, so the channel count proves
    /// the geometry flowed, not the fallback).
    #[test]
    fn vorbis_session_opens_with_an_audio_limb() {
        let mut host = CliHost::new();
        let path = fixture(
            "../../crates/codecs/tests/fixtures",
            "vorbis_sine_8000_mono.ogg",
        );
        let outcome = host.open_session(
            commands::catalog::types::SourceSpec::File { path },
            None,
            false,
            false,
        );
        assert!(outcome.is_ok(), "{outcome:?}");
        assert!(host.device.is_some(), "the audio limb + device exist");
        assert_eq!(host.audio_channels(), 1, "the Vorbis ID geometry (mono)");
        assert!(host.clock.is_some(), "the session clock is the audio clock");
    }

    /// The PCM lane keeps its limb through the generic lookup (the
    /// wav regression): the committed stereo fixture answers the
    /// same geometry from its `pcm` pair.
    #[test]
    fn pcm_session_keeps_its_audio_limb() {
        let mut host = CliHost::new();
        let path = fixture(
            "../../tests/fixtures/generated/valid",
            "wav_pcm_s16le_44k1_st.wav",
        );
        let outcome = host.open_session(
            commands::catalog::types::SourceSpec::File { path },
            None,
            false,
            false,
        );
        assert!(outcome.is_ok(), "{outcome:?}");
        assert!(host.device.is_some(), "the PCM limb is unchanged");
        assert_eq!(host.audio_channels(), 2);
    }

    /// An audio session with no geometry-bearing truth stays
    /// honestly limb-less (the ADTS family: the config rides
    /// in-band, the demuxer exposes no blob — the wall clock is the
    /// honest fallback, recorded as the family's open gap).
    #[test]
    fn in_band_config_audio_stays_limb_less() {
        let mut host = CliHost::new();
        let path = fixture("../../crates/codecs/tests/fixtures", "sine48_lc_mono.aac");
        let outcome = host.open_session(
            commands::catalog::types::SourceSpec::File { path },
            None,
            false,
            false,
        );
        assert!(outcome.is_ok(), "{outcome:?}");
        assert!(host.device.is_none(), "no geometry source — no limb");
    }
}

#[cfg(test)]
mod session_backpressure_tests {
    //! Card 3.B3's host half: the ring→decode backpressure and the
    //! wall-clock session pump, proven through the REAL `open_session`
    //! over a media LONGER than the ring (the committed fixtures are
    //! 2 s ring-sized; the GUI bug needs a media whose decode-ahead
    //! can exceed 131 072 frames). The generated WAV is written to a
    //! unique temp path (never the repo; cleaned up by the OS).

    use super::*;
    use crate::pipeline_host::AUDIO_PARK_FILL;
    use crate::pipeline_host::RING_FRAMES;
    use std::path::PathBuf;
    use testkit::Must;

    /// Write a `seconds`-long 44.1 kHz mono s16le WAV to a unique
    /// temp path and answer it.
    fn long_wav(seconds: u32) -> PathBuf {
        use std::io::Write as _;
        let rate: u32 = 44_100;
        let frames = u64::from(seconds) * u64::from(rate);
        let data_len = frames * 2 * 2; // stereo s16le
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&2u16.to_le_bytes()); // stereo
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&(rate * 4).to_le_bytes()); // byte rate
        wav.extend_from_slice(&4u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data_len as u32).to_le_bytes());
        // Silence: the lane's geometry is the subject, not its content.
        wav.resize(44 + data_len as usize, 0);
        let path = std::env::temp_dir().join(format!(
            "lumen-backpressure-{}-{}s.wav",
            std::process::id(),
            seconds
        ));
        let mut file = std::fs::File::create(&path).must();
        file.write_all(&wav).must();
        path
    }

    /// Open + settle a long session (auto-play): the harness the two
    /// proofs below share.
    fn playing_host(seconds: u32) -> CliHost {
        let mut host = CliHost::new();
        let path = long_wav(seconds);
        let outcome = host.open_session(
            commands::catalog::types::SourceSpec::File { path },
            None,
            false,
            false,
        );
        assert!(outcome.is_ok(), "{outcome:?}");
        host.settle();
        assert_eq!(host.state(), PlaybackState::Playing);
        host
    }

    /// THE GUI regression (the user-visible `audio.ring_full` fatal,
    /// v0.1.2): a session driven by pipeline-only pumps — the exact
    /// shape of the shell's settle loop and the 40 ms event ticker
    /// before the session thread existed — decodes ahead while the
    /// clock never moves. Ungated, the ring fills to its 131 072
    /// ceiling in a handful of polls and the next staged frame is a
    /// FATAL at the audio-output stage; parked (card 3.B3), the fill
    /// tops out at [`AUDIO_PARK_FILL`] + one packet burst and the
    /// session stays `Playing` — then plays to its exact end once
    /// time is allowed to move.
    #[test]
    fn settle_loop_parks_instead_of_fatalling_the_ring() {
        // 6 s = 264 600 frames — over 2× the ring, so the ungated
        // code path had more than enough media to overflow with.
        let mut host = playing_host(6);
        // The settle-loop shape: pipeline-only pumps, NO time advance.
        for _ in 0..200 {
            host.pump_pipeline();
        }
        assert_ne!(
            host.state(),
            PlaybackState::Error,
            "decode-ahead without time movement is parking, never fatal"
        );
        assert!(host.last_fatal.is_none(), "{:?}", host.last_fatal);
        let fill = host
            .device
            .as_ref()
            .map(|device| device.buffered_frames())
            .must();
        // The park bound: fill ≤ AUDIO_PARK_FILL + the worst single
        // packet burst (16 384 frames of 16-bit stereo at the 64 KiB
        // packetizer cap) — structurally below RING_FRAMES.
        let worst_burst = 16_384;
        assert!(
            fill <= AUDIO_PARK_FILL + worst_burst,
            "ring fill {fill} must stay under the park bound {}",
            AUDIO_PARK_FILL + worst_burst
        );
        assert!(fill < RING_FRAMES, "never at the ring's ceiling");
        // And the parked session is not wedged: with time moving
        // again it plays to its exact end (the 6 s of media, the
        // tail play-out included — no stranded residue).
        assert!(
            host.pump_until_state(PlaybackState::Ended, 4_000),
            "the parked session resumes and ends honestly"
        );
        let position_us = host.position_us();
        assert!(
            (5_990_000..=6_010_000).contains(&position_us),
            "the full media played out, got {position_us} µs"
        );
    }

    /// The wall-clock session pump ([`CliHost::pump_wallclock_at`]):
    /// synthetic instants drive the render — the device only ever
    /// consumes what the WALL is owed since the Playing anchor, at
    /// the quantum's granularity, whatever the pump cadence or the
    /// ring's fill. The deterministic `advance`'s whole-ring drain
    /// (the scripted sessions' contract) would have consumed the
    // ~66 k-frame fill on the FIRST tick; the wall pump consumes
    /// 1 470 frames per 33 ms owed and no more.
    #[test]
    fn wall_pump_renders_only_what_the_wall_owes() {
        let mut host = playing_host(6);
        let t0 = std::time::Instant::now();
        // The anchor forms on the first wall pump (Playing entry).
        host.pump_wallclock_at(t0);
        let fill_at_anchor = host
            .device
            .as_ref()
            .map(|device| device.buffered_frames())
            .must();
        assert!(
            fill_at_anchor > 0,
            "decode-ahead staged frames before the anchor"
        );
        // 50 ms of wall: 2 205 frames owed → exactly one quantum
        // (1 470) renders (the second does not fit under the due).
        host.pump_wallclock_at(t0 + std::time::Duration::from_millis(50));
        assert_eq!(
            host.device.as_ref().map(|device| device.sample_count()),
            Some(1_470),
            "50 ms owes 2 205 frames; the quantum fits once"
        );
        // 1 s of wall (a burst catch-up after a slow tick): the
        // counter lands on the wall's due, quantized down.
        host.pump_wallclock_at(t0 + std::time::Duration::from_millis(1_000));
        let count = host
            .device
            .as_ref()
            .map(|device| device.sample_count())
            .must();
        assert_eq!(count, 44_100, "1 s at 44.1 kHz, quantum-quantized");
        assert!(
            (count as usize) < fill_at_anchor,
            "the wall pump never drains the ring whole ({count} vs fill {fill_at_anchor})"
        );
        // The position is the audio-master clock over the same
        // counter: ≈1 s of media time, not the decode-ahead depth.
        let position_us = host.position_us();
        assert!(
            (995_000..=1_005_000).contains(&position_us),
            "position tracks the wall, got {position_us} µs"
        );
        // Pause: the anchor clears (not Playing) and the paused arm
        // renders nothing even at +2 s of wall.
        host.submit_and_await(SessionCommandKind::Pause).must();
        host.pump_wallclock_at(t0 + std::time::Duration::from_millis(2_000));
        assert_eq!(
            host.device.as_ref().map(|device| device.sample_count()),
            Some(44_100),
            "a paused session renders nothing"
        );
    }
}
