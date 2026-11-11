//! The local command-layer harness of the scenarios-B suite (card
//! 3.E2 part 2; doc 10 §6.4 scenarios 9–16).
//!
//! # Why a LOCAL dispatch shape (the honest ARCH-5 note)
//!
//! Doc 05 §5.1's `lumen_dispatch` has exactly one production
//! instantiation in the tree: `lumen_cli::dispatch::lumen_dispatch`
//! (apps/cli). This suite lives in `crates/playback-engine/tests/`
//! per the card's ownership, and the real entry is **unreachable from
//! here**: a `lumen-cli` dev-dependency would close a dev-edge cycle
//! (`lumen-cli` → `playback-engine` → dev `lumen-cli`), which
//! boundaries.toml check 2 rejects outright (ARCH-8, "no cycles over
//! normal AND dev edges") and which the `[crates.playback-engine]`
//! dev-dep allowlist does not admit. Relocating the suite to a
//! `tests/integration/` crate (doc 13's original 3.E2 ownership, the
//! first-vertical precedent) would reach the real entry, but the card
//! pins the files here and the acceptance command is
//! `cargo test -p playback-engine --test scenarios_b`.
//!
//! The reconciliation, per the card's own sanction ("build local
//! fakes in your test dir if needed"): this module is a **local
//! shape** of the command layer over the REAL engine and the REAL
//! pipeline crates (`input` access, `demux` AVI, `codecs`
//! MJPEG/PCM — the same dev-edge composition
//! `tests/track_switch_fixture.rs` already proved in this crate).
//! Scenarios speak the real `commands` catalog vocabulary
//! ([`WireRequest`]/[`Command`]/[`WireResponse`]) and never touch an
//! engine API — every interaction goes through [`lumen_dispatch`].
//! What is a fake is named below; everything else is the production
//! stack.
//!
//! `[PROPOSAL]` for the Lead (the handoff this note replaces): (a)
//! move scenarios 9–16 to `tests/integration/scenarios-b/` so they
//! drive `lumen_cli::dispatch::lumen_dispatch` for real, or (b)
//! extract the CLI's host core into a crate below `playback-engine`
//! so both hosts share it. Until either lands, the first-vertical
//! crate remains the only real-dispatch scenario evidence, and this
//! suite pins the command-layer CONTRACT (catalog vocabulary, served
//! semantics, typed refusals, wire events) against the real engine.
//!
//! # The fakes, by name
//!
//! - [`ScriptedStream`] — an access-layer fake for scenario 11: the
//!   `input::ScriptSource` contract (scripted truncation + scripted
//!   mid-read I/O failure) with a **shared handle** so a scenario can
//!   arm a fault after the engine owns the stream (the stock
//!   `ScriptSource` is owned wholesale and cannot be scripted
//!   mid-play). Byte semantics are copied verbatim from
//!   `input::ScriptSource`/`SyntheticStream`.
//! - [`TimelineDemuxer`] — a scripted demux stage for scenarios 13/14
//!   (the `tests/common` `FakeDemuxer` shape, generating its packet
//!   timeline on demand so producer memory is bounded by
//!   construction). Decoding uses the real `tests/common`
//!   `FakeDecoder` behind the shared `FakeProvider` (one audio frame
//!   per packet).
//! - [`ObservingSink`] — the consumer seam: frame accounting, a
//!   virtual sample-counter clock model, and the **vanish** arm of
//!   scenario 12 (the card's
//!   "device loss = simulated via … local fakes": once armed, every
//!   `accept` answers the typed `Device{AudioOutput, Disconnected}`
//!   error — the engine's documented sink-contract fatal).
//! - The `mp4`/unknown-signature **sniff** in the open path mirrors
//!   `media-probe`'s builtins for the class-B matrix (this crate
//!   cannot dev-depend on `media-probe`); AVI detection itself is the
//!   demuxer's own header validation.
//! - The open options `no_video`/`no_audio` answer a typed refusal
//!   (the real host's filtered re-wiring is its own composition; no
//!   scenario here exercises it).
//!
//! Everything else — `Engine`, `SessionHandle`, the state machine,
//! `AviDemuxer`, `MjpegDecoder`/`PcmDecoder`, `input::FileAccess`,
//! the wire event conversion — is production code.
//!
//! # Determinism
//!
//! Every observable is a pure function of the pump count (the 2.C2/2.D2
//! poll model): the media clock advances only when the sink consumes
//! audio (the null-device sample-counter truth), the event-throttle
//! windows read a frozen source, and no test ever sleeps. The
//! counting allocator installed by `main.rs` makes memory assertions
//! exact in-process (D-2: allocator-live-bytes deltas stand in for
//! RSS, which is not measurable in-proc).

use std::collections::{HashMap, VecDeque};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{Decoder, DecoderConfig};
use commands::catalog::command::CommandResult;
use commands::catalog::types::{
    Duration as WireDuration, MediaInfo, OpenOptions, SourceSpec, TimeRange,
    Timestamp as WireTimestamp,
};
use commands::envelope::{CommandOutcome, Generation, RequestId, SessionId};
use commands::{
    Command, CommandError, ErrorCode, EventKind, PlaybackState, Stage, WireEvent, WireRequest,
    WireResponse,
};
use demux::avi::AviDemuxer;
use demux::avi::header::StreamFormat;
use media_clock::{MonotonicSource, NANO_BASE, PlaybackClock};
use media_types::error::{DeviceCause, DeviceSlot};
use media_types::frame::SampleFormat;
use media_types::packet::SharedBytes;
use media_types::{
    Duration, EngineError, ErrorKind, Frame, MediaKind, Nanoseconds, Packet, PipelineStage,
    SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId, TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEventKind, EngineOutput, EngineStats,
    EventPolicy, FrameConsumer, QueuePlan, ReceiptResult, SeekTarget, SessionCommandKind,
    SessionConfig, SessionError, SessionHandle, SessionState, TickBudget, TimeTickPolicy,
    TrackSelector, track_info_wire, wire_error_code, wire_stage,
};
use plugin_api::{Access, DemuxEvent, Demuxer, SeekOrigin, SourcePosition, SourceStream};
use testkit::Must;

use crate::common::{FakeProvider, SharedNow};

/// A present stamp's ns (None when missing).
fn stamp_ns(stamp: &media_types::Stamp) -> Option<i64> {
    stamp
        .timestamp()
        .ok()
        .and_then(|ts| ts.convert(&NANO_BASE).ok())
        .map(|ts| ts.ticks)
}

/// The bounded await (the CLI host's own pacing constant).
pub const AWAIT_PUMPS: usize = 64;

/// The class-A parent fixture (MJPEG+PCM, 2.000 s, 320x240 @ 30 fps,
/// stereo s16le 44.1 kHz — the first vertical's own fixture).
pub const PARENT: &str = "generated/valid/avi_mjpeg_pcm.avi";

/// The committed multi-track parent (W2-FIX-MT): 1 MJPEG video +
/// 2 PCM audio streams (48 kHz stereo s16le + 44.1 kHz mono u8) —
/// scenario 8's track-switching container.
pub const MULTITRACK: &str = "generated/valid/avi_mjpeg_2pcm.avi";

// ---------------------------------------------------------------------------
// The scripted access source (scenario 11's local fake)
// ---------------------------------------------------------------------------

/// The mutable half of [`ScriptedStream`], shared so a scenario can
/// arm faults after the engine owns the stream.
struct ScriptState {
    /// The currently visible length (scripted truncation target).
    visible_len: u64,
    /// The absolute read position.
    pos: u64,
    /// A fault to return from the next read instead of data.
    next_failure: Option<EngineError>,
    /// Reads served (assertion surface: when the access layer
    /// actually touches the source).
    reads: u64,
    /// Bytes served so far.
    bytes_served: u64,
}

/// The scripted source's test-side handle (the fault-injection seam).
pub struct ScriptHandle(Arc<Mutex<ScriptState>>);

impl ScriptHandle {
    /// Truncate the visible stream to `new_len` bytes (the
    /// mid-play network cut).
    pub fn shrink_to(&self, new_len: u64) {
        let mut state = self.lock();
        state.visible_len = state.visible_len.min(new_len);
    }

    /// Fail the next underlying read with the typed mid-read I/O
    /// failure (the connection-reset shape).
    pub fn fail_next_read(&self) {
        self.lock().next_failure = Some(input::error::read_io(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "scripted connection reset",
        )));
    }

    /// The currently visible length (assertion surface).
    pub fn visible_len(&self) -> u64 {
        self.lock().visible_len
    }

    /// The read stats (reads served, bytes served) — the assertion
    /// surface for "the fault arm actually bites".
    pub fn read_stats(&self) -> (u64, u64) {
        let state = self.lock();
        (state.reads, state.bytes_served)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ScriptState> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// An in-memory byte source with scriptable faults and a shared
/// handle — the `input::ScriptSource` contract, plus shared ownership
/// (see the module docs). Reads past the visible length return a
/// clean end-of-stream; the demuxer's own extent checks mint the
/// typed truncation from the container's declared numbers.
pub struct ScriptedStream {
    data: Arc<Vec<u8>>,
    state: Arc<Mutex<ScriptState>>,
    uri: SourceUri,
}

impl ScriptedStream {
    /// Build over `data`; the returned handle arms faults mid-play.
    pub fn new(data: Arc<Vec<u8>>) -> (ScriptedStream, ScriptHandle) {
        let state = Arc::new(Mutex::new(ScriptState {
            visible_len: data.len() as u64,
            pos: 0,
            next_failure: None,
            reads: 0,
            bytes_served: 0,
        }));
        let stream = ScriptedStream {
            data,
            uri: SourceUri::parse("file:///lumen-scenarios-b-scripted").must(),
            state: Arc::clone(&state),
        };
        (stream, ScriptHandle(state))
    }
}

impl SourceStream for ScriptedStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        Some(self.lock_state().visible_len)
    }

    fn supports_seek(&self) -> bool {
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = vec![0u8; len];
        let got = self.read_into(&mut out)?;
        out.truncate(got);
        Ok(out)
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let mut state = self.lock_state();
        if let Some(failure) = state.next_failure.take() {
            return Err(failure);
        }
        let available = state.visible_len.saturating_sub(state.pos);
        let n = (out.len() as u64).min(available) as usize;
        let start = state.pos as usize;
        let end = start + n;
        if end <= self.data.len() {
            out[..n].copy_from_slice(&self.data[start..end]);
        }
        state.pos += n as u64;
        state.reads += 1;
        state.bytes_served += n as u64;
        Ok(n)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let mut state = self.lock_state();
        let target = match origin {
            SeekOrigin::Start(to) => to,
            SeekOrigin::Current(delta) => {
                let sum = i128::from(state.pos) + i128::from(delta);
                if sum < 0 {
                    return Err(seek_typed("input.seek.before-start"));
                }
                u64::try_from(sum).unwrap_or(u64::MAX)
            }
        };
        if target > state.visible_len {
            return Err(seek_typed("input.seek.out-of-range"));
        }
        state.pos = target;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.lock_state().pos)
    }
}

impl ScriptedStream {
    fn lock_state(&self) -> std::sync::MutexGuard<'_, ScriptState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn seek_typed(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new(code),
    )
}

// ---------------------------------------------------------------------------
// The scripted demux stage (scenarios 13/14's local fake)
// ---------------------------------------------------------------------------

/// The abrupt-teardown telemetry (scenario 16): drop-hook counters
/// the pipeline's stage objects bump when the engine releases them —
/// the in-process stand-in for the teardown report's "device closed,
/// threads joined" evidence (this engine is poll-driven and
/// threadless; the object graph's release IS the teardown).
pub struct DropHooks {
    /// Demuxer drops observed (the engine released the demux stage).
    pub demuxer_drops: AtomicU64,
    /// Sink drops observed (the engine released the consumer seam —
    /// the "device closed" oracle for the null-device limb).
    pub sink_drops: AtomicU64,
}

impl DropHooks {
    /// Whether the demuxer half observed its drop.
    pub fn demuxer_dropped(&self) -> bool {
        self.demuxer_drops
            .load(std::sync::atomic::Ordering::Relaxed)
            > 0
    }

    /// Whether the sink half observed its drop.
    pub fn sink_dropped(&self) -> bool {
        self.sink_drops.load(std::sync::atomic::Ordering::Relaxed) > 0
    }
}

/// A forwarding demuxer that counts its own drop (the demuxer-half
/// drop hook — the engine owns the boxed stage, so its release is the
/// counter's proof).
struct DropCountingDemuxer {
    inner: Box<dyn Demuxer>,
    hooks: Arc<DropHooks>,
}

impl Demuxer for DropCountingDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        self.inner.start()
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        self.inner.next_event()
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        self.inner.seek(target)
    }
}

impl Drop for DropCountingDemuxer {
    fn drop(&mut self) {
        self.hooks.demuxer_drops.fetch_add(1, Ordering::Relaxed);
    }
}

/// An on-demand packet-timeline demuxer: `count` audio packets of
/// `step_us` each (pts = index × step), then end-of-stream. Producer
/// memory is bounded by construction — the timeline is synthesized,
/// never stored. The unseekable typed refusal matches the shared
/// `tests/common` fake (no scenario here seeks).
pub struct TimelineDemuxer {
    track: Track,
    remaining: u64,
    step_us: i64,
    index: u64,
    started: bool,
}

impl TimelineDemuxer {
    /// A single-audio-track timeline of `count` packets, each
    /// `step_us` long, with a declared span to match.
    pub fn audio_timeline(count: u64, step_us: i64) -> TimelineDemuxer {
        let span_end_us = i64::try_from(count).must().checked_mul(step_us).must();
        TimelineDemuxer {
            track: timeline_track(Some(span_end_us)),
            remaining: count,
            step_us,
            index: 0,
            started: false,
        }
    }

    fn packet(&mut self) -> Packet {
        let base = TimeBase::new(1, 1_000_000).must();
        let pts = Stamp::Present(Timestamp::new((self.index as i64) * self.step_us, base));
        self.index += 1;
        self.remaining -= 1;
        Packet {
            track: self.track.id,
            pts,
            dts: Stamp::Missing,
            duration: Some(Duration::Known(Nanoseconds(
                u64::try_from(self.step_us * 1_000).must(),
            ))),
            data: SharedBytes::from_static(&[7u8; 64]),
            keyframe: true,
            discontinuity: false,
            generation: media_types::GenerationId::FIRST,
        }
    }
}

impl Demuxer for TimelineDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        self.started = true;
        Ok(vec![self.track.clone()])
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if !self.started {
            return Err(EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new("demux.timeline.not-started"),
            ));
        }
        if self.remaining > 0 {
            return Ok(DemuxEvent::Packet(self.packet()));
        }
        Ok(DemuxEvent::EndOfStream)
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.timeline.not-seekable"),
        ))
    }
}

/// The timeline's declared track (span end `span_end_us`, or the
/// unknown span for a zero-packet timeline).
fn timeline_track(span_end_us: Option<i64>) -> Track {
    let base = TimeBase::new(1, 1_000_000).must();
    let span = span_end_us.map_or(TimeSpan::UNKNOWN, |end| TimeSpan {
        start: Stamp::Present(Timestamp::new(0, base)),
        end: Stamp::Present(Timestamp::new(end, base)),
    });
    Track {
        id: TrackId::new(1),
        stream: media_types::StreamId::new(1),
        kind: MediaKind::Audio,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span,
            base,
            gaps: Vec::new(),
        },
    }
}

// ---------------------------------------------------------------------------
// The consumer seam (frame accounting + the vanish arm of scenario 12)
// ---------------------------------------------------------------------------

/// The observing consumer: counts everything, models the null-device
/// sample counter on the
/// virtual clock, and — once armed — answers the typed device-lost
/// error on every accept (the sink-contract fatal the engine
/// documents: "a consumer refusal is the sink contract's fatal").
pub struct ObservingSink {
    frames_accepted: AtomicU64,
    audio_accepted: AtomicU64,
    frames_while_vanished: AtomicU64,
    last_pts_ns: AtomicU64,
    /// The vanish arm (device loss). `Some(error)` once armed.
    vanish: Mutex<Option<EngineError>>,
    /// The bounded frame ring `(track, is-audio, pts-ns)` — the
    /// stamp and stale-frame assertion surface for the A/V-sync and
    /// track-switch scenarios. Capped; old entries roll out (D-2:
    /// the sink never grows unboundedly).
    frame_ring: Mutex<VecDeque<(u32, bool, i64)>>,
    /// The virtual monotonic source (the sample-counter stand-in).
    now: Arc<SharedNow>,
    /// The session clock (paused checks mirror the CLI's render
    /// guard: a frozen clock must not advance).
    clock: Arc<media_clock::MediaClock>,
    /// The PCM shape (samples-derived durations when the frame
    /// carries none).
    pcm: Option<(u32, u32)>,
    /// The abrupt-teardown drop hook (scenario 16; `None` on every
    /// other session).
    drop_hook: Option<Arc<DropHooks>>,
}

impl ObservingSink {
    /// The frame ring's cap (D-2: bounded by construction).
    const RING_CAP: usize = 1024;

    /// Push one ring entry (accept-order, capped).
    fn ring_push(&self, track: u32, audio: bool, pts_ns: i64) {
        let mut ring = self
            .frame_ring
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if ring.len() >= Self::RING_CAP {
            ring.pop_front();
        }
        ring.push_back((track, audio, pts_ns));
    }

    /// The bounded frame ring's snapshot: `(track, is-audio, pts-ns)`
    /// in accept order — the stamp/stale assertion surface (A/V
    /// sync, track-switch boundaries).
    pub fn frame_ring(&self) -> Vec<(u32, bool, i64)> {
        self.frame_ring
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .copied()
            .collect()
    }

    /// The last ring pts of one track id (None before any).
    ///
    /// Harness observation API kept for track-scoped stamp queries
    /// (symmetric with `frame_ring`); no scenario asserts on it yet —
    /// the gate doc §2 row 4c dead-code hit that surfaced this.
    #[allow(dead_code)]
    pub fn last_pts_ns_of_track(&self, track: u32) -> Option<i64> {
        self.frame_ring
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .rev()
            .find(|(id, _, _)| *id == track)
            .map(|(_, _, pts)| *pts)
    }

    /// A counting sink over `clock`/`now` with `pcm` facts.
    pub fn counting(
        now: Arc<SharedNow>,
        clock: Arc<media_clock::MediaClock>,
        pcm: Option<(u32, u32)>,
        drop_hook: Option<Arc<DropHooks>>,
    ) -> Arc<ObservingSink> {
        Arc::new(ObservingSink {
            frames_accepted: AtomicU64::new(0),
            audio_accepted: AtomicU64::new(0),
            frames_while_vanished: AtomicU64::new(0),
            last_pts_ns: AtomicU64::new(0),
            vanish: Mutex::new(None),
            frame_ring: Mutex::new(VecDeque::new()),
            now,
            clock,
            pcm,
            drop_hook,
        })
    }

    /// Arm the device loss: every subsequent accept answers the typed
    /// `Device{AudioOutput, Disconnected}` error (doc 01 §3.4).
    pub fn vanish(&self) {
        *self
            .vanish
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(EngineError::new(
            PipelineStage::AudioOutput,
            ErrorKind::Device {
                which: DeviceSlot::AudioOutput,
                cause: DeviceCause::Disconnected,
            },
            media_types::error::ErrorContext::new("audio.output.device-disconnected"),
        ));
    }

    /// Frames that reached the consumer (both limbs).
    pub fn frames_accepted(&self) -> u64 {
        self.frames_accepted.load(Ordering::Relaxed)
    }

    /// Audio frames accepted.
    pub fn audio_accepted(&self) -> u64 {
        self.audio_accepted.load(Ordering::Relaxed)
    }

    /// Accepts attempted while vanished (every one refused).
    pub fn frames_while_vanished(&self) -> u64 {
        self.frames_while_vanished.load(Ordering::Relaxed)
    }

    /// The last accepted audio frame's pts (ns; 0 before the first).
    pub fn last_pts_ns(&self) -> u64 {
        self.last_pts_ns.load(Ordering::Relaxed)
    }

    /// The frame's duration in nanoseconds (declared, then
    /// samples-derived for PCM, else none).
    fn frame_duration_ns(&self, frame: &Frame) -> Option<u64> {
        let Frame::Audio(audio) = frame else {
            return None;
        };
        if let Some(Duration::Known(Nanoseconds(ns))) = audio.duration {
            return Some(ns);
        }
        let (channels, rate) = self.pcm?;
        let bytes = u64::try_from(audio.samples.len()).ok()?;
        let sample_bytes = match audio.format.sample {
            SampleFormat::U8 => 1,
            SampleFormat::S16 => 2,
            SampleFormat::S24 | SampleFormat::S32 | SampleFormat::F32 => 4,
            SampleFormat::F64 => 8,
        };
        if channels == 0 || rate == 0 {
            return None;
        }
        let samples = bytes / (u64::from(channels) * sample_bytes);
        Some(samples * 1_000_000_000 / u64::from(rate))
    }
}

impl FrameConsumer for ObservingSink {
    fn accept(&self, frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        if let Some(error) = self
            .vanish
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
        {
            self.frames_while_vanished.fetch_add(1, Ordering::Relaxed);
            return Err(EngineError::new(
                error.stage,
                error.kind.clone(),
                media_types::error::ErrorContext {
                    code: error.context.code.clone(),
                },
            ));
        }
        if let Frame::Audio(audio) = &frame {
            self.audio_accepted.fetch_add(1, Ordering::Relaxed);
            // The sample-counter clock model: the device consumed
            // this quantum (the CLI's `advance_time` guard — a frozen
            // clock never advances).
            if !self.clock.is_paused()
                && let Some(ns) = self.frame_duration_ns(&frame)
            {
                self.now.advance_ns(ns);
            }
            if let Ok(ns) = audio
                .pts
                .timestamp()
                .and_then(|ts| ts.convert(&NANO_BASE).map(|ts| ts.ticks))
            {
                self.last_pts_ns.store(ns.max(0) as u64, Ordering::Relaxed);
                self.ring_push(_track.handle(), true, ns);
            }
        } else {
            // Video limb: counted only (the clock stays the audio
            // sample counter's truth — doc 03 §4.7) — but the stamp
            // rides the ring (the A/V-sync assertion surface).
            let Frame::Video(video) = &frame else {
                unreachable!("the third frame kind never wires in v0");
            };
            if let Some(ns) = stamp_ns(&video.pts) {
                self.ring_push(_track.handle(), false, ns);
            }
        }
        self.frames_accepted.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

impl Drop for ObservingSink {
    fn drop(&mut self) {
        if let Some(hooks) = &self.drop_hook {
            hooks.sink_drops.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl SharedNow {
    /// Advance the virtual monotonic source (the sample counter).
    fn advance_ns(&self, ns: u64) {
        self.0.fetch_add(ns, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// The real decoder provider (the track_switch_fixture shape)
// ---------------------------------------------------------------------------

/// MJPEG as-is; PCM seeded with the `strf` facts harvested before the
/// demuxer moved into the engine (the `PcmDecoder::with_format`
/// seam — the engine's wire passes no WAVEFORMATEX `extra_data`).
pub struct RealProvider {
    pcm_facts: HashMap<u32, (u32, u32)>,
}

impl DecoderProvider for RealProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        match track.kind {
            MediaKind::Video => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            MediaKind::Audio => {
                let Some((channels, rate)) = self.pcm_facts.get(&track.id.handle()).copied() else {
                    return Err(EngineError::new(
                        PipelineStage::Decode,
                        ErrorKind::Decoder {
                            cause: media_types::error::DecodeCause::Bitstream,
                        },
                        media_types::error::ErrorContext::new("decode.pcm.no-strf-facts"),
                    ));
                };
                let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)?;
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            _ => Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new("decode.kind.unsupported"),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// The open-path facts and wire conversions
// ---------------------------------------------------------------------------

/// What the open path harvested: the PCM shape and the declared
/// duration.
struct SessionFacts {
    pcm: Option<(u32, u32)>,
    duration_us: Option<i64>,
    container: String,
}

/// Harvest the facts from a started AVI demuxer (the CLI
/// `harvest_facts` shape: PCM `strf` for the decoder provider, the
/// greatest declared span end for the duration).
fn harvest_avi_facts(demuxer: &AviDemuxer, tracks: &[Track]) -> SessionFacts {
    let mut pcm = None;
    let mut duration_ns: Option<i64> = None;
    for track in tracks {
        if let Some(StreamFormat::Pcm {
            channels,
            samples_per_sec,
            ..
        }) = demuxer.stream_format(track.id)
        {
            pcm = Some((u32::from(channels), samples_per_sec));
        }
        if let Some(end) = track.timing.span.end.timestamp().ok()
            && let Ok(ns) = end.convert(&NANO_BASE)
        {
            duration_ns = Some(duration_ns.map_or(ns.ticks, |prev: i64| prev.max(ns.ticks)));
        }
    }
    SessionFacts {
        pcm,
        duration_us: duration_ns.map(|ns| ns / 1_000),
        container: "avi".to_string(),
    }
}

/// The PCM fact map for the real provider.
fn pcm_fact_map(demuxer: &AviDemuxer, tracks: &[Track]) -> HashMap<u32, (u32, u32)> {
    let mut map = HashMap::new();
    for track in tracks {
        if let Some(StreamFormat::Pcm {
            channels,
            samples_per_sec,
            ..
        }) = demuxer.stream_format(track.id)
        {
            map.insert(track.id.handle(), (u32::from(channels), samples_per_sec));
        }
    }
    map
}

/// The wire conversion of an engine error (the CLI `demux_host`
/// shape, over the engine's own public stage/code bridges).
pub fn wire_error(error: &EngineError) -> CommandError {
    let full = error.to_string();
    let stage_prefix = format!("[{}] ", error.stage);
    let base = full.strip_prefix(&stage_prefix).unwrap_or(full.as_str());
    let message = match error.source.as_ref() {
        Some(root) => format!("{base}: {root}"),
        None => base.to_string(),
    };
    CommandError::new(
        wire_stage(error.stage),
        wire_error_code(&error.kind),
        message,
    )
    .with_detail("code", error.context.code.as_str())
}

/// The access-layer refusal with the source named by its redacted
/// form (V8: the basename survives, the machine path does not). The
/// input crate's errors are deliberately path-free (its module
/// doctrine), so the HOST layer — the one that knows the URI —
/// attaches the name.
fn access_refusal(error: &EngineError, uri: &SourceUri) -> CommandError {
    let base = wire_error(error);
    let name = uri.redacted();
    if base.message.contains(&name) {
        return base;
    }
    CommandError::new(base.stage, base.code, format!("{}: {name}", base.message))
        .with_detail("code", error.context.code.as_str())
}
/// The wire conversion of a session-construction failure (the CLI's
/// `session_error_wire`).
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
            "the engine's command inbox is full (bounded wait expired)",
        ),
        SessionError::Invariant(rendered) => CommandError::new(
            Stage::Session,
            ErrorCode::Internal,
            format!("engine state-table invariant breach: {rendered}"),
        ),
    }
}

/// The percent-encoding set for a `file:` URI path (the CLI's own
/// minimal RFC 3986 encoder, copied so open-path URIs agree).
fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Canonicalize a user path and build its `file:` URI (the CLI's
/// `file_uri`: the canonicalize failure IS the truth).
fn file_uri(path: &Path) -> Result<SourceUri, CommandError> {
    let canonical: PathBuf = std::fs::canonicalize(path).map_err(|err| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            format!("cannot resolve {}: {err}", path.display()),
        )
    })?;
    if !canonical.is_file() {
        return Err(CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            format!("not a regular file: {}", path.display()),
        ));
    }
    let text = canonical.to_str().ok_or_else(|| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Unsupported,
            "the path is not valid UTF-8",
        )
    })?;
    SourceUri::parse(&format!("file://{}", encode_path(text))).map_err(|err| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Internal,
            format!("the encoded file URI was rejected: {err}"),
        )
    })
}

/// The signature sniff (the `media-probe` builtins this matrix needs,
/// mirrored locally — this crate cannot dev-depend on `media-probe`):
/// `RIFF….AVI ` is avi; `ftyp` at offset 4 is mp4; anything else is
/// unknown. AVI validation itself is the demuxer's own header check.
fn sniff_format(stream: &mut dyn SourceStream) -> Result<&'static str, CommandError> {
    let head = stream.read(12).map_err(|err| wire_error(&err))?;
    let _ = stream.seek(SeekOrigin::Start(0));
    if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"AVI " {
        return Ok("avi");
    }
    if head.len() >= 8 && &head[4..8] == b"ftyp" {
        return Ok("mp4");
    }
    Ok("unknown")
}

/// The probe-stage refusal for a non-AVI signature (the CLI's own
/// sentence — the first vertical serves AVI only, D-018).
fn probe_refusal(format: &str) -> CommandError {
    if format == "unknown" {
        return CommandError::new(
            Stage::Probe,
            ErrorCode::Unsupported,
            "the signature is not a known container (probe window)",
        )
        .with_detail("format", format);
    }
    CommandError::new(
        Stage::Probe,
        ErrorCode::Unsupported,
        format!("format `{format}` has no demuxer in this build (first vertical: AVI, D-018)"),
    )
    .with_detail("format", format)
}

// ---------------------------------------------------------------------------
// Queue pacing
// ---------------------------------------------------------------------------

/// The queue caps for the real AVI fixtures (the
/// `track_switch_fixture` shape: 100 ms interleave granularity, both
/// duration caps at 400 ms so the demuxer stands at most ~400 ms
/// ahead of the decoded position).
fn avi_queues() -> QueuePlan {
    let audio = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(400_000_000),
    };
    let video = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(8 << 20).must(),
        max_duration: Nanoseconds(400_000_000),
    };
    QueuePlan::new(audio, video)
}

/// The queue caps for the scripted timelines: 1 s packets with a
/// 60 s duration cap (the timeline feeds well within every axis; the
/// soak-shaped scenarios must not starve on the pacing choice).
fn timeline_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(8 << 20).must(),
        max_duration: Nanoseconds(60_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

fn avi_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

/// The multi-audio-track parents' tick budget: ONE packet per limb
/// per poll. The prime watermarks are evaluated AFTER the decode
/// slice, so the slice must stay at or below the high watermark's
/// distance from the queue cap for the queues to still read `high`
/// post-decode (the green `tests/track_switch_fixture.rs` doctrine,
/// its `fixture_budget` docs): the track-switch parents carry
/// ~100 ms-granularity packets against the 400 ms duration caps, so
/// the 4-packet `avi_budget` slice drains the queue below `high`
/// every tick — priming never fires mid-media and the open races
/// the whole stream to EOS while still `Buffering` (observed:
/// scenario 8's switch landed at the end, the new limb
/// `can_never_deliver`-completed, no new-track frame ever flowed).
fn multitrack_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 1,
        flush_calls_per_limb: 8,
    }
}

fn timeline_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 8,
        flush_calls_per_limb: 8,
    }
}

// ---------------------------------------------------------------------------
// The host
// ---------------------------------------------------------------------------

/// What the open path wired for one session.
struct Wired {
    handle: SessionHandle,
    sink: Arc<ObservingSink>,
    clock: Arc<media_clock::MediaClock>,
    facts: SessionFacts,
    script: Option<ScriptHandle>,
    source: SourceSpec,
}

/// The scenario host: the local command layer's engine half — one
/// `Engine`, the wired session, the recorded wire events, and the
/// bounded pacing primitives. Scenarios reach it ONLY through
/// [`lumen_dispatch`].
pub struct ScenarioHost {
    engine: Engine,
    wired: Option<Wired>,
    now: Arc<SharedNow>,
    next_request: u64,
    pending_receipts: Vec<CommandReceipt>,
    /// The observed wire events (in causal order). Cleared at each
    /// settled close (bounded per session — the soak-shaped
    /// scenarios must not grow the HOST; the frozen throttle emits
    /// no time-ticks at all).
    events: Vec<WireEvent>,
    /// The state-transition trail (from the `state-changed` events).
    state_history: Vec<(PlaybackState, PlaybackState)>,
    /// The last fatal error (wire form).
    last_fatal: Option<CommandError>,
    /// The last `tracks.changed` snapshot (wire form).
    tracks_wire: Vec<commands::catalog::types::TrackInfo>,
    /// The reused output-drain buffer (D-1).
    outbox_scratch: Vec<EngineOutput>,
    /// The abrupt-teardown telemetry (scenario 16; armed per host,
    /// consulted at wiring time so every session's stage objects are
    /// counted).
    drop_hooks: Option<Arc<DropHooks>>,
    /// A one-shot queue-plan override for the NEXT open (scenario
    /// 11's mid-play faults: tight packet caps keep the demuxer
    /// genuinely mid-file when the fault is armed, so a "mid-play"
    /// cut actually bites — with the default caps the sink races the
    /// whole stream during the open's own pumps).
    paced_queues: Option<QueuePlan>,
}

impl Default for ScenarioHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioHost {
    /// An idle host (no session).
    #[must_use]
    pub fn new() -> ScenarioHost {
        ScenarioHost {
            engine: Engine::new(),
            wired: None,
            now: Arc::new(SharedNow(AtomicU64::new(0))),
            next_request: 1,
            pending_receipts: Vec::new(),
            events: Vec::new(),
            state_history: Vec::new(),
            last_fatal: None,
            tracks_wire: Vec::new(),
            outbox_scratch: Vec::new(),
            paced_queues: None,
            drop_hooks: None,
        }
    }

    /// Arm the abrupt-teardown drop hooks: every session wired after
    /// this call wraps its demuxer and sink with the counters
    /// (scenario 16's "resources released" oracle).
    pub fn arm_drop_hooks(&mut self) -> Arc<DropHooks> {
        let hooks = Arc::new(DropHooks {
            demuxer_drops: AtomicU64::new(0),
            sink_drops: AtomicU64::new(0),
        });
        self.drop_hooks = Some(Arc::clone(&hooks));
        hooks
    }

    /// Pace the NEXT open with tight per-limb packet caps (scenario
    /// 11's mid-play fault arming). The default caps let the sink
    /// race the whole stream during the open's own pumps — a fault
    /// armed after `settle()` then finds the demuxer already done.
    /// With `max_packets` per limb the demuxer stays genuinely
    /// mid-file, so the armed cut/reset bites during playback.
    pub fn pace_next_open(&mut self, max_packets: usize) {
        let limits = QueueLimits {
            max_packets: NonZeroUsize::new(max_packets).must(),
            max_bytes: NonZeroUsize::new(8 << 20).must(),
            max_duration: Nanoseconds(400_000_000),
        };
        self.paced_queues = Some(QueuePlan::new(limits, limits));
    }

    // -- pacing -----------------------------------------------------------

    /// One pipeline-only pump: engine poll + output drain (no time
    /// advance — media time moves only via the sink's sample
    /// counter, the CLI's own doctrine).
    pub fn pump_pipeline(&mut self) {
        self.engine.poll();
        self.drain_outputs();
    }

    /// Pump until the engine sits in a settled state (not the
    /// transient `Opening`/`Buffering`) or the bound.
    pub fn settle(&mut self) {
        self.pump_pipeline_until(|host| {
            !matches!(
                host.state(),
                PlaybackState::Opening | PlaybackState::Buffering
            )
        });
    }

    /// Pipeline-only pump-until (bounded; answers whether the
    /// predicate was observed).
    pub fn pump_pipeline_until(&mut self, mut pred: impl FnMut(&ScenarioHost) -> bool) -> bool {
        for _ in 0..AWAIT_PUMPS {
            if pred(self) {
                return true;
            }
            self.pump_pipeline();
        }
        pred(self)
    }

    /// Pipeline-only pump-until with an explicit bound — the
    /// long-timeline progress driver (`AWAIT_PUMPS` is the
    /// command-await bound; a half-hour-scale timeline needs its own
    /// arithmetic, stated at the call site).
    pub fn pump_pipeline_until_within(
        &mut self,
        bound: usize,
        mut pred: impl FnMut(&ScenarioHost) -> bool,
    ) -> bool {
        for _ in 0..bound {
            if pred(self) {
                return true;
            }
            self.pump_pipeline();
        }
        pred(self)
    }

    /// Pump until a terminal state (`Ended`/`Error`) or the bound —
    /// the run-to-end driver. Answers the number of pumps spent (the
    /// deadline evidence: a hang is a bound breach, never a mystery).
    pub fn run_until_terminal(&mut self, bound: usize) -> (usize, PlaybackState) {
        for spent in 0..bound {
            let state = self.state();
            if matches!(state, PlaybackState::Ended | PlaybackState::Error) {
                return (spent, state);
            }
            self.pump_pipeline();
        }
        (bound, self.state())
    }

    // -- observation ------------------------------------------------------

    /// The live wire state.
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        self.engine.state().as_wire()
    }

    /// The live session id, if any.
    #[must_use]
    pub fn session_id(&self) -> Option<u64> {
        self.engine.session_id().map(|id| id.0)
    }

    /// The live generation, if any.
    #[must_use]
    pub fn generation(&self) -> Option<u64> {
        self.engine.generation().map(|generation| generation.0)
    }

    /// The observed wire events (causal order).
    #[must_use]
    pub fn events(&self) -> &[WireEvent] {
        &self.events
    }

    /// The state-transition trail.
    #[must_use]
    pub fn state_history(&self) -> &[(PlaybackState, PlaybackState)] {
        &self.state_history
    }

    /// The engine's pipeline observability counters.
    #[must_use]
    pub fn stats(&self) -> EngineStats {
        self.engine.stats()
    }

    /// The current position (µs) — the virtual sample counter's view.
    #[must_use]
    pub fn position_us(&self) -> i64 {
        self.wired
            .as_ref()
            .map_or(0, |wired| wired.clock.now().ticks / 1_000)
    }

    /// The wired sink (assertions).
    #[must_use]
    pub fn sink(&self) -> Option<&Arc<ObservingSink>> {
        self.wired.as_ref().map(|wired| &wired.sink)
    }

    /// The scripted source's fault handle (scenario 11).
    #[must_use]
    pub fn script(&self) -> Option<&ScriptHandle> {
        self.wired.as_ref().and_then(|wired| wired.script.as_ref())
    }

    // -- internals --------------------------------------------------------

    /// Drain the engine's output hub (the CLI's shape: receipts
    /// stashed, events converted to wire and recorded, the reused
    /// scratch buffer — D-1).
    fn drain_outputs(&mut self) {
        let mut outputs = std::mem::take(&mut self.outbox_scratch);
        outputs.clear();
        self.engine.drain_outputs_into(&mut outputs);
        for output in outputs.drain(..) {
            match output {
                EngineOutput::Receipt(receipt) => self.pending_receipts.push(receipt),
                EngineOutput::Event(event) => {
                    if let EngineEventKind::StateChanged { from, to } = &event.kind {
                        self.state_history.push((from.as_wire(), to.as_wire()));
                    }
                    if let EngineEventKind::Fatal { error } = &event.kind {
                        self.last_fatal = Some(wire_error(error));
                    }
                    if let EngineEventKind::TracksChanged { tracks, selected } = &event.kind {
                        self.tracks_wire = tracks
                            .iter()
                            .zip(selected.iter())
                            .map(|(track, &is_selected)| track_info_wire(track, is_selected))
                            .collect();
                    }
                    if let Some(wire) = event.as_wire_event() {
                        self.events.push(wire);
                    }
                }
            }
        }
        self.outbox_scratch = outputs;
    }

    /// Submit one minimal-set command and pump until its receipt
    /// arrives (the CLI's `submit_and_await`).
    fn submit_and_await(
        &mut self,
        kind: SessionCommandKind,
    ) -> Result<CommandReceipt, CommandError> {
        let request = RequestId(self.next_request);
        self.next_request += 1;
        let handle = self
            .wired
            .as_ref()
            .map(|wired| &wired.handle)
            .ok_or_else(|| {
                CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
            })?;
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
        self.await_receipt(request)
    }

    /// Submit `playback.seek` with its target (the CLI
    /// `dispatch_seek` shape, minus the CLI's audio ring — this
    /// host's sink is the direct seam): map the wire target, submit,
    /// answer the class-A receipt, then pump to the seek terminal
    /// (`timing.seek-completed`/`-failed` — the first terminal after
    /// submission is the live seek's, doc 05 §4.2).
    fn dispatch_seek(
        &mut self,
        target: commands::catalog::types::SeekTarget,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        use commands::catalog::types::SeekTarget as WireTarget;
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
                        "no chapter {index}: AVI v0 carries no chapter markers \
                         (the chapter card owns this)"
                    ),
                ));
            }
        };
        let request = RequestId(self.next_request);
        self.next_request += 1;
        let terminal_from = self.events.len();
        let handle = self
            .wired
            .as_ref()
            .map(|wired| &wired.handle)
            .ok_or_else(|| {
                CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
            })?;
        handle
            .seek(request, engine_target)
            .map_err(|error| session_error_wire(&error))?;
        let receipt = self.await_receipt(request)?;
        let outcome = self.receipt_outcome(receipt)?;
        let landed = self.pump_pipeline_until(|host| {
            host.events[terminal_from..]
                .iter()
                .any(|event| is_seek_terminal(&event.kind))
        });
        if !landed {
            return Err(CommandError::new(
                Stage::Control,
                ErrorCode::Internal,
                "the engine never reported the seek terminal (bounded wait expired)",
            ));
        }
        Ok(outcome)
    }

    /// Submit `tracks.set-audio-track`/`tracks.set-video-track`
    /// (card 3.A3's command-layer composition): map the selector,
    /// submit, answer the class-A receipt, then pump to the switch
    /// completion terminal (`tracks.changed` under the post-bump
    /// generation).
    fn dispatch_set_track(
        &mut self,
        kind: SessionCommandKind,
        selector: commands::catalog::types::TrackSelector,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        use commands::catalog::types::TrackSelector as WireSelector;
        let engine_selector = match selector {
            WireSelector::Id { id } => TrackSelector::Id {
                id: media_types::TrackId::new(id.0),
            },
            WireSelector::None => TrackSelector::None,
            WireSelector::Auto => {
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::InvalidArgument,
                    "Auto selection is the subtitle-only form (§2.5) — \
                     audio/video take Id or None",
                ));
            }
        };
        let request = RequestId(self.next_request);
        self.next_request += 1;
        let handle = self
            .wired
            .as_ref()
            .map(|wired| &wired.handle)
            .ok_or_else(|| {
                CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
            })?;
        let submit = || match kind {
            SessionCommandKind::SelectVideo => handle.set_video_track(request, engine_selector),
            SessionCommandKind::SelectAudio => handle.set_audio_track(request, engine_selector),
            other => unreachable!("dispatch_set_track owns the two selector kinds, got {other:?}"),
        };
        submit().map_err(|error| session_error_wire(&error))?;
        let receipt = self.await_receipt(request)?;
        self.receipt_outcome(receipt)
    }

    /// Await one submitted command's receipt (bounded pipeline-only
    /// pumps).
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

    /// Translate one receipt into the truthful-UI outcome (the CLI's
    /// `receipt_outcome`).
    fn receipt_outcome(
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

    /// The `session.open` path (the CLI's `open_session` shape over
    /// the local source model).
    fn open_session(
        &mut self,
        source: SourceSpec,
        options: &OpenOptions,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        if self.engine.session_id().is_some() {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "a session is already open — close it first",
            ));
        }
        self.wired = Some(self.wire_source(source, options)?);
        // Pump the open to a settled state; an immediate open failure
        // (corrupt header, no decodable track) must surface in THIS
        // response.
        self.pump_pipeline_until(|host| host.state() != PlaybackState::Opening);
        if let Some(error) = self.last_fatal.take() {
            self.teardown_after_fatal();
            return Err(error);
        }
        Ok(CommandOutcome::Accepted {
            session: self.engine.session_id(),
            generation: self.engine.generation().unwrap_or(Generation(0)),
        })
    }

    /// Compose one session from a wire source spec: access → sniff →
    /// demux → provider/sink/clock → `SessionConfig` → engine open.
    fn wire_source(
        &mut self,
        source: SourceSpec,
        options: &OpenOptions,
    ) -> Result<Wired, CommandError> {
        if options.no_video || options.no_audio {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::UnsupportedForHost,
                "track filtering at open is the real host's composition \
                 (first vertical); this local host serves full A/V opens only",
            )
            .with_detail("owner", "apps/cli open path (2.D2)"));
        }
        match &source {
            SourceSpec::File { path } => {
                let uri = file_uri(path)?;
                let mut stream = input::FileAccess::new()
                    .open(&uri)
                    .map_err(|err| access_refusal(&err, &uri))?;
                let format = sniff_format(stream.as_mut());
                if !matches!(format, Ok("avi")) {
                    return Err(probe_refusal(format.unwrap_or("unknown")));
                }
                self.wire_avi(stream, uri, source.clone(), None)
            }
            SourceSpec::Url { url } => self.wire_scripted(url, source.clone()),
        }
    }

    /// Wire a real AVI stream (file or scripted) into a session.
    fn wire_avi(
        &mut self,
        stream: Box<dyn SourceStream>,
        uri: SourceUri,
        source: SourceSpec,
        script: Option<ScriptHandle>,
    ) -> Result<Wired, CommandError> {
        let mut demuxer = AviDemuxer::new(stream).map_err(|err| wire_error(&err))?;
        let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
        let facts = harvest_avi_facts(&demuxer, &tracks);
        let provider = Arc::new(RealProvider {
            pcm_facts: pcm_fact_map(&demuxer, &tracks),
        });
        let queues = self.paced_queues.take().unwrap_or_else(avi_queues);
        // The decode slice follows the track layout (see
        // `multitrack_budget`): ≥2 audio tracks ⇒ the coarse-packet
        // track-switch parents ⇒ one packet per limb per poll.
        let budget = if tracks
            .iter()
            .filter(|track| track.kind == MediaKind::Audio)
            .count()
            >= 2
        {
            multitrack_budget()
        } else {
            avi_budget()
        };
        self.finish_wiring(
            uri,
            Box::new(demuxer),
            provider,
            facts,
            script,
            source,
            queues,
            budget,
        )
    }

    /// Wire a scripted URL source: `script://avi-parent` (the parent
    /// fixture over the fault-scripted access fake) or
    /// `timeline://<count>x<step-us>` (the on-demand demux stage).
    /// Every other URL answers the real host's typed refusal.
    fn wire_scripted(&mut self, url: &str, source: SourceSpec) -> Result<Wired, CommandError> {
        if url == "script://avi-parent" {
            let parent = testkit::fixture_bytes(PARENT).map_err(|err| {
                CommandError::new(
                    Stage::Input,
                    ErrorCode::Inaccessible,
                    format!("cannot read the parent fixture: {err}"),
                )
            })?;
            let (stream, script) = ScriptedStream::new(Arc::new(parent));
            let uri = SourceUri::parse("file:///lumen-scenarios-b-scripted").must();
            self.wire_avi(Box::new(stream), uri, source, Some(script))
        } else if let Some(rest) = url.strip_prefix("timeline://") {
            let (count, step) = parse_timeline(rest).ok_or_else(|| {
                CommandError::new(
                    Stage::Input,
                    ErrorCode::InvalidArgument,
                    format!("bad timeline URL: {url} (want timeline://<count>x<step-us>)"),
                )
            })?;
            let demuxer = TimelineDemuxer::audio_timeline(count, step);
            let span_us = i64::try_from(count).must().checked_mul(step).must();
            let facts = SessionFacts {
                pcm: None,
                duration_us: (count > 0).then_some(span_us),
                container: "timeline".to_string(),
            };
            let uri = SourceUri::parse(&format!("memory://lumen-timeline-{count}x{step}")).must();
            self.finish_wiring(
                uri,
                Box::new(demuxer),
                Arc::new(FakeProvider { fail_select: None }),
                facts,
                None,
                source,
                timeline_queues(),
                timeline_budget(),
            )
        } else {
            Err(CommandError::new(
                Stage::Input,
                ErrorCode::UnsupportedForHost,
                format!("URL sources need the streaming crate (Phase 6): {url}"),
            )
            .with_detail("scheme", url.split(':').next().unwrap_or("?")))
        }
    }

    /// The shared wiring tail: clock, sink, config, engine open.
    /// (Bundled tail args keep clippy's fn-arity ceiling: the pieces
    /// always travel together.)
    #[allow(clippy::too_many_arguments)]
    fn finish_wiring(
        &mut self,
        uri: SourceUri,
        demuxer: Box<dyn Demuxer>,
        provider: Arc<dyn DecoderProvider>,
        facts: SessionFacts,
        script: Option<ScriptHandle>,
        source: SourceSpec,
        queues: QueuePlan,
        budget: TickBudget,
    ) -> Result<Wired, CommandError> {
        let base = TimeBase::new(1, 1_000_000).must();
        let clock = Arc::new(
            media_clock::MediaClock::system_monotonic(
                Arc::clone(&self.now) as Arc<dyn MonotonicSource>,
                Timestamp::new(0, base),
                media_types::Rate::ONE,
            )
            .map_err(|err| wire_error(&err))?,
        );
        let sink = ObservingSink::counting(
            Arc::clone(&self.now),
            Arc::clone(&clock),
            facts.pcm,
            self.drop_hooks.clone(),
        );
        let demuxer: Box<dyn Demuxer> = match self.drop_hooks.clone() {
            Some(hooks) => Box::new(DropCountingDemuxer {
                inner: demuxer,
                hooks,
            }),
            None => demuxer,
        };
        let mut config = SessionConfig::new(
            uri,
            demuxer,
            provider,
            Arc::clone(&sink) as Arc<dyn FrameConsumer>,
            Arc::clone(&clock),
        );
        config.container = Some(facts.container.clone());
        config.queues = queues;
        config.budget = budget;
        config.throttle = EventPolicy {
            tick: TimeTickPolicy::default_policy(),
            throttle: Arc::new(SharedNow(AtomicU64::new(0))),
        };
        let handle = self
            .engine
            .open(config)
            .map_err(|err| session_error_wire(&err))?;
        Ok(Wired {
            handle,
            sink,
            clock,
            facts,
            script,
            source,
        })
    }

    /// After a fatal failure the engine sits in `Error` with a dead
    /// session — close it so the host is reusable.
    fn teardown_after_fatal(&mut self) {
        if self.engine.state() == SessionState::Error {
            let _ = self.submit_and_await(SessionCommandKind::Close);
        }
        self.teardown_session_scoped();
    }

    /// Clear every session-scoped host half (after a settled Close or
    /// a fatal): the recordings too — bounded per session. The stale
    /// `last_fatal` dies with its session: a LATER open must not
    /// answer with the dead session's error (the reopen-after-fatal
    /// recovery shape).
    fn teardown_session_scoped(&mut self) {
        self.wired = None;
        self.pending_receipts.clear();
        self.events.clear();
        self.state_history.clear();
        self.tracks_wire.clear();
        self.last_fatal = None;
    }

    /// The class-S snapshot answers.
    fn media_info(&self) -> Result<CommandResult, CommandError> {
        let Some(wired) = self.wired.as_ref() else {
            return Err(not_ready());
        };
        Ok(CommandResult::MediaInfo {
            info: MediaInfo {
                source: wired.source.clone(),
                container: Some(wired.facts.container.clone()),
                title: None,
                duration: duration_wire(wired.facts.duration_us),
                seekable: true,
                is_live: false,
                metadata: std::collections::BTreeMap::new(),
                tracks: self.tracks_wire.clone(),
                chapters: Vec::new(),
            },
        })
    }

    fn duration(&self) -> Result<CommandResult, CommandError> {
        let Some(wired) = self.wired.as_ref() else {
            return Err(not_ready());
        };
        Ok(CommandResult::Duration {
            duration: duration_wire(wired.facts.duration_us),
        })
    }

    fn current_time(&self) -> Result<CommandResult, CommandError> {
        let Some(wired) = self.wired.as_ref() else {
            return Err(not_ready());
        };
        let position_us = self.position_us().max(0);
        let duration_us = wired.facts.duration_us;
        let fraction = match duration_us {
            Some(duration) if duration > 0 => {
                position_us.clamp(0, duration) as f32 / duration as f32
            }
            _ => 0.0,
        };
        let buffered = duration_us.map_or_else(Vec::new, |duration| {
            vec![TimeRange {
                start: WireTimestamp(0),
                end: WireTimestamp(duration),
            }]
        });
        Ok(CommandResult::CurrentTime {
            time: WireTimestamp(position_us),
            position: fraction,
            buffered,
        })
    }
}

fn not_ready() -> CommandError {
    CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
}

/// Parse `timeline://<count>x<step-us>`.
fn parse_timeline(rest: &str) -> Option<(u64, i64)> {
    let (count, step) = rest.split_once('x')?;
    Some((count.parse().ok()?, step.parse().ok()?))
}

/// A declared duration in wire form.
fn duration_wire(duration_us: Option<i64>) -> WireDuration {
    match duration_us {
        Some(us) => WireDuration::Known {
            time: WireTimestamp(us),
        },
        None => WireDuration::Unknown,
    }
}

/// A rejection's wire form (the CLI's `rejection_wire`).
fn rejection_wire(state: SessionState, reason: playback_engine::RejectionReason) -> CommandError {
    let stage = Stage::Session;
    let state_name = state.as_wire();
    match reason {
        playback_engine::RejectionReason::NoSession => {
            CommandError::new(stage, ErrorCode::NotReady, "no session open")
        }
        playback_engine::RejectionReason::NotReady => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!("the session is still opening (state: {state_name:?})"),
        ),
        playback_engine::RejectionReason::SettleFirst
        | playback_engine::RejectionReason::AlreadySettled => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "the state table refuses this command here (state: {state_name:?}, reason: {reason:?})"
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

// ---------------------------------------------------------------------------
// The dispatch entry (the local shape of doc 05 §5.1)
// ---------------------------------------------------------------------------

/// **The** dispatch entry of this suite — the local shape of doc 05
/// §5.1's `lumen_dispatch` (see the module docs for why it is local
/// and what is fake). Correlate, route through the engine, answer
/// the wire response.
#[must_use]
pub fn lumen_dispatch(host: &mut ScenarioHost, request: &WireRequest) -> WireResponse {
    let request_id = request.request_id;
    let result = route(host, &request.command);
    WireResponse {
        request_id,
        session: host.session_id().map(SessionId),
        generation: host.generation().map_or(Generation(0), Generation),
        result,
    }
}

/// One catalog command → its outcome: the served subset this suite's
/// scenarios need, mirroring the CLI's semantics; everything else
/// answers the honest typed refusal (doc 05 §5.2).
fn route(
    host: &mut ScenarioHost,
    command: &Command,
) -> Result<CommandOutcome<CommandResult>, CommandError> {
    match command {
        Command::SessionOpen { source, options } => {
            let outcome = host.open_session(source.clone(), options)?;
            if options.start_paused {
                host.settle();
                host.submit_and_await(SessionCommandKind::Pause)
                    .and_then(|receipt| host.receipt_outcome(receipt))?;
            }
            Ok(outcome)
        }
        Command::SessionClose => host
            .submit_and_await(SessionCommandKind::Close)
            .and_then(|receipt| host.receipt_outcome(receipt))
            .inspect(|_| host.teardown_session_scoped()),
        Command::PlaybackPlay => host
            .submit_and_await(SessionCommandKind::Play)
            .and_then(|receipt| host.receipt_outcome(receipt)),
        Command::PlaybackPause => host
            .submit_and_await(SessionCommandKind::Pause)
            .and_then(|receipt| host.receipt_outcome(receipt)),
        Command::PlaybackTogglePlayPause => {
            let kind = if host.state() == PlaybackState::Playing {
                SessionCommandKind::Pause
            } else {
                SessionCommandKind::Play
            };
            host.submit_and_await(kind)
                .and_then(|receipt| host.receipt_outcome(receipt))
        }
        Command::PlaybackStop => host
            .submit_and_await(SessionCommandKind::Stop)
            .and_then(|receipt| host.receipt_outcome(receipt)),
        Command::MediaGetInfo => Ok(CommandOutcome::Completed(host.media_info()?)),
        Command::MediaGetCurrentTime => Ok(CommandOutcome::Completed(host.current_time()?)),
        Command::MediaGetDuration => Ok(CommandOutcome::Completed(host.duration()?)),
        Command::TracksList => Ok(CommandOutcome::Completed(CommandResult::TrackList {
            tracks: host.tracks_wire.clone(),
        })),
        // The seek and track-switch compositions (3.E1's scenarios
        // 4/6/8): the CLI `dispatch_seek` shape minus its audio ring
        // (this host's sink is the direct seam), and card 3.A3's
        // selector kinds.
        Command::PlaybackSeek { target } => host.dispatch_seek(*target),
        Command::PlaybackSetPosition { fraction } => {
            host.dispatch_seek(commands::catalog::types::SeekTarget::Percent {
                fraction: *fraction,
            })
        }
        Command::TracksSetAudioTrack { selector } => {
            host.dispatch_set_track(SessionCommandKind::SelectAudio, *selector)
        }
        Command::TracksSetVideoTrack { selector } => {
            host.dispatch_set_track(SessionCommandKind::SelectVideo, *selector)
        }
        other => Err(unserved(other)),
    }
}

/// The seek terminal test (the first terminal after submission is the
/// live seek's — a superseded seek is silent by contract, doc 05
/// §4.2).
fn is_seek_terminal(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::TimingSeekCompleted { .. } | EventKind::TimingSeekFailed { .. }
    )
}

/// The typed refusal for a command this local host does not serve
/// (the CLI's `unserved` shape).
fn unserved(command: &Command) -> CommandError {
    let id = command_id_string(command);
    let owner = owner_of(&catalog_prefix(&id));
    CommandError::new(
        Stage::Control,
        ErrorCode::UnsupportedForHost,
        format!("`{id}` is not served by this build's command-line host ({owner})"),
    )
    .with_detail("command", id)
    .with_detail("owner", owner)
}

/// The catalog id of one command, read from its Debug form's first
/// token (the same stable names the wire's `cmd` tag carries;
/// serde_json is not on this crate's dependency set, and this is
/// error-path only).
fn command_id_string(command: &Command) -> String {
    let debug = format!("{command:?}");
    debug
        .split(['{', ' ', '('])
        .next()
        .unwrap_or("(unnamed)")
        .to_string()
}

/// The catalog-style dotted prefix of one Debug-form command token
/// (`AudioListDevices` → `audio.`, `TracksSetVideo` → `tracks.set-`):
/// the CLI reads the real `"cmd"` tag from serde — the authoritative
/// dotted id — while this host only has the Debug form; the tokens
/// share the catalog's word structure by construction, so the
/// prefix derives deterministically (lowercased leading words, the
/// set/step verbs hyphenated, `system.plugin` joined).
fn catalog_prefix(debug_token: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in debug_token.chars() {
        if ch.is_ascii_uppercase() && !current.is_empty() {
            words.push(current.clone());
            current.clear();
        }
        current.push(ch.to_ascii_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    match words.as_slice() {
        [first, second, ..] if first == "system" && second == "plugin" => {
            "system.plugin".to_string()
        }
        [first, second, ..] if second == "set" || second == "step" => {
            format!("{first}.{second}-")
        }
        [first, ..] => format!("{first}."),
        [] => String::new(),
    }
}

/// The owning-subsystem note (the CLI's `owner_of`, verbatim for the
/// rows this suite asserts).
fn owner_of(id: &str) -> &'static str {
    if id.starts_with("playlist.") {
        "the playlist model is a Phase-4 card"
    } else if id.starts_with("audio.") {
        "audio device control joins with the real backends (ADR-005)"
    } else if id.starts_with("video.") {
        "video output control joins with the windowed renderer (2.D3)"
    } else if id.starts_with("subtitle.") {
        "the subtitle path is a Phase-5 card"
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
    } else if id.starts_with("system.plugin") {
        "the plugin registry is a Phase-6 card"
    } else if id.starts_with("playback.set-") || id.starts_with("playback.step-") {
        "rate and frame stepping are card 3.A1"
    } else if id.starts_with("media.") {
        "chapters and their navigation are the chapter card"
    } else {
        "not in the first vertical's served set (see `lumen commands`)"
    }
}

// ---------------------------------------------------------------------------
// Scenario-side helpers
// ---------------------------------------------------------------------------

/// One dispatched step: build the wire request, dispatch, answer the
/// response — every scenario's every engine interaction goes through
/// here.
pub fn dispatch(host: &mut ScenarioHost, command: Command) -> WireResponse {
    let request_id = host.take_request_id();
    let request = WireRequest {
        request_id: RequestId(request_id),
        command,
    };
    lumen_dispatch(host, &request)
}

impl ScenarioHost {
    /// Take the next request id (test-side correlation only).
    fn take_request_id(&mut self) -> u64 {
        let id = self.next_request;
        self.next_request += 1;
        id
    }
}

/// The wire `playback.error` events, as `(stage.code, fatal,
/// message)` rows (the assertion surface).
pub fn error_events(host: &ScenarioHost) -> Vec<(String, bool, String)> {
    host.events()
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::PlaybackError {
                stage,
                code,
                message,
                fatal,
                ..
            } => Some((
                format!("{}.{}", stage.as_str(), code.as_str()),
                *fatal,
                message.clone(),
            )),
            _ => None,
        })
        .collect()
}

/// Count the wire events of one serde name (the stable id).
pub fn count_events(host: &ScenarioHost, name: &str) -> usize {
    host.events()
        .iter()
        .filter(|event| event_name(&event.kind) == name)
        .count()
}

/// The serde name of one event kind (the layer's own wire ids,
/// mirrored here for counting — no serde dependency in this crate).
fn event_name(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::SessionStateChanged { .. } => "session.state-changed",
        EventKind::SessionOpeningProgress { .. } => "session.opening-progress",
        EventKind::SessionBuffering { .. } => "session.buffering",
        EventKind::TimingTimeTick { .. } => "timing.time-tick",
        EventKind::TimingDurationKnown { .. } => "timing.duration-known",
        EventKind::TimingSeekCompleted { .. } => "timing.seek-completed",
        EventKind::TimingSeekFailed { .. } => "timing.seek-failed",
        EventKind::TracksListReady { .. } => "tracks.list-ready",
        EventKind::TracksChanged { .. } => "tracks.changed",
        EventKind::PlaybackEndOfTrack { .. } => "playback.end-of-track",
        EventKind::PlaybackEndOfMedia => "playback.end-of-media",
        EventKind::PlaybackError { .. } => "playback.error",
        EventKind::PlaybackRateChanged { .. } => "playback.rate-changed",
        EventKind::AudioDeviceLost { .. } => "audio.device-lost",
        EventKind::AudioDeviceChanged { .. } => "audio.device-changed",
        EventKind::VideoScreenshotSaved { .. } => "video.screenshot-saved",
        EventKind::VideoBackendChanged { .. } => "video.backend-changed",
        EventKind::PlaylistChanged { .. } => "playlist.changed",
        EventKind::ConfigSettingChanged { .. } => "config.setting-changed",
        EventKind::PluginsLoaded { .. } => "plugins.loaded",
        EventKind::PluginsFailed { .. } => "plugins.failed",
        EventKind::PluginsCrashed { .. } => "plugins.crashed",
        EventKind::SystemLog { .. } => "system.log",
        EventKind::MediaInfoReady { .. } => "media.info-ready",
        EventKind::DevicesVolumeAdded { .. } => "devices.volume-added",
        EventKind::DevicesVolumeRemoved { .. } => "devices.volume-removed",
    }
}

/// A temp-dir guard: the scenario's scratch space, removed on drop
/// (the "no temp state" assertions snapshot it).
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Create a unique scratch dir under the system temp root.
    pub fn new(tag: &str) -> TempDir {
        let path =
            std::env::temp_dir().join(format!("lumen-scenarios-b-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&path).must();
        TempDir { path }
    }

    /// The scratch dir's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The sorted list of entry names (the no-temp-state oracle).
    #[must_use]
    pub fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.path)
            .must()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Copy the parent fixture in and return the copy's path.
    pub fn copy_parent(&self, name: &str) -> PathBuf {
        let from = testkit::fixture_path(PARENT).must();
        let to = self.path.join(name);
        std::fs::copy(&from, &to).must();
        to
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Patch `len` bytes at `offset` with a deterministic non-zero
/// pattern (the documented byte-level corruption).
pub fn corrupt_bytes(bytes: &mut [u8], offset: usize, len: usize, seed: u8) {
    for (index, slot) in bytes[offset..offset + len].iter_mut().enumerate() {
        *slot = seed.wrapping_add(index as u8).rotate_left(3) ^ 0xA5;
    }
}

// ---------------------------------------------------------------------------
// Short-fixture surgery (scenario 14/15's well-formed tiny AVIs)
// ---------------------------------------------------------------------------

/// A little-endian u32 out of `bytes` at `at` (the RIFF walker's one
/// primitive; panics on a short slice — a broken parent fixture is a
/// broken test, never a product bug).
fn u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// A little-endian u32 into `bytes` at `at`.
fn put_u32_le(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// The chunk fourcc at `at`.
fn fourcc_at(bytes: &[u8], at: usize) -> [u8; 4] {
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}

impl TempDir {
    /// Build a **well-formed short AVI** from the class-A parent by
    /// byte surgery (scenario 14's "0.1 s clip" and 1-frame fixtures,
    /// scenario 15's reopen workhorse): keep the first `frames`
    /// video/audio chunk pairs of the `movi` list verbatim, patch the
    /// declared counts to the honest short numbers (`avih`
    /// `dwTotalFrames`, the video `strh` `dwLength`, the audio `strh`
    /// `dwLength` in samples), rebuild the `RIFF`/`movi` sizes, and
    /// truncate `idx1` to the kept entries. The result is a genuine
    /// tiny media file — declared spans match the payload, so it
    /// plays to `Ended` with **zero** damage (the corrupt-media
    /// matrix is scenario 9's business, not this one's).
    ///
    /// Determinism: a pure function of the committed parent bytes
    /// (the xtask generator's byte-stable contract); no clock, no
    /// entropy, no external encoder.
    pub fn build_short_avi(&self, name: &str, frames: usize) -> PathBuf {
        assert!(
            (1..=30).contains(&frames),
            "the surgery keeps the first 1..=30 frame pairs, asked for {frames}"
        );
        let parent = testkit::fixture_bytes(PARENT).must();

        // Top-level walk: RIFF('AVI ') { LIST hdrl, LIST movi, idx1 }.
        assert_eq!(fourcc_at(&parent, 0), *b"RIFF", "the parent is a RIFF file");
        let mut cursor = 12;
        assert_eq!(fourcc_at(&parent, cursor), *b"LIST");
        let hdrl_end = cursor + 8 + u32_le(&parent, cursor + 4) as usize;
        cursor = hdrl_end;
        assert_eq!(fourcc_at(&parent, cursor), *b"LIST");
        assert_eq!(fourcc_at(&parent, cursor + 8), *b"movi");
        let movi_data = cursor + 8 + 4;
        let movi_end = cursor + 8 + u32_le(&parent, cursor + 4) as usize;
        cursor = movi_end;
        assert_eq!(fourcc_at(&parent, cursor), *b"idx1");
        let idx1_data = cursor + 8;
        let idx1_entries = u32_le(&parent, cursor + 4) as usize / 16;

        // Walk the movi chunks, collecting the first 2*`frames`.
        let mut kept_end = movi_data;
        let mut kept = 0usize;
        let mut video = 0usize;
        let mut audio = 0usize;
        let mut first_audio_size = None;
        let mut at = movi_data;
        while at < movi_end && kept < 2 * frames {
            let size = u32_le(&parent, at + 4) as usize;
            match &fourcc_at(&parent, at) {
                b"00dc" => video += 1,
                b"01wb" => {
                    audio += 1;
                    first_audio_size.get_or_insert(size);
                }
                _ => unreachable!("the parent's movi is 00dc/01wb interleaved"),
            }
            kept += 1;
            kept_end = at + 8 + size + (size & 1);
            at = kept_end;
        }
        assert_eq!(kept, 2 * frames, "the parent has the frame pairs asked for");
        assert_eq!(video, frames);
        assert_eq!(audio, frames);
        assert!(kept <= idx1_entries, "idx1 covers the kept chunks");

        // The header patch: hdrl copied verbatim (offsets are stable —
        // the copy lands at the same file offset), avih/strh patched.
        let mut hdrl = parent[12..hdrl_end].to_vec();
        // The walk is HDRL-SLICE-LOCAL: the slice starts at the
        // `LIST` fourcc (file offset 12), so the first inner chunk
        // (`avih`) sits at slice offset 12 — NOT its file offset 24
        // (the original 12+8+4 walked past it and patched nothing,
        // leaving the parent's 60-frame declarations in place).
        let mut sub = 12;
        while sub + 8 <= hdrl.len() {
            let size = u32_le(&hdrl, sub + 4) as usize;
            match &fourcc_at(&hdrl, sub) {
                b"avih" => {
                    // dwTotalFrames is the 5th dword of the payload.
                    put_u32_le(&mut hdrl, sub + 8 + 16, frames as u32);
                }
                b"LIST" => {
                    // A strl: patch its strh's dwLength.
                    let strl_end = sub + 8 + size;
                    let mut inner = sub + 8 + 4;
                    while inner < strl_end {
                        let inner_size = u32_le(&hdrl, inner + 4) as usize;
                        if fourcc_at(&hdrl, inner) == *b"strh" {
                            let payload = inner + 8;
                            let fcc_type = u32_le(&hdrl, payload);
                            let length = match fcc_type {
                                // 'vids': dwLength counts frames.
                                0x7364_6976 => frames as u32,
                                // 'auds': dwLength counts samples
                                // (dwSampleSize-sized); scale the
                                // parent's per-chunk sample count.
                                0x7364_7561 => {
                                    let sample_size = u32_le(&hdrl, payload + 44);
                                    let chunk_bytes =
                                        u64::try_from(first_audio_size.unwrap_or(0)).must();
                                    let per_chunk = if sample_size > 0 {
                                        chunk_bytes / u64::from(sample_size)
                                    } else {
                                        1
                                    };
                                    u32::try_from(frames as u64 * per_chunk).must()
                                }
                                other => unreachable!("unknown stream fccType {other:#x}"),
                            };
                            // dwLength is the 9th dword of the payload.
                            put_u32_le(&mut hdrl, payload + 32, length);
                        }
                        inner += 8 + inner_size + (inner_size & 1);
                    }
                }
                _ => {}
            }
            sub += 8 + size + (size & 1);
        }

        // Assemble: RIFF header + patched hdrl + shortened movi +
        // truncated idx1; the RIFF size is patched last.
        let kept_bytes = kept_end - movi_data;
        let mut out = Vec::with_capacity(kept_bytes + hdrl.len() + kept * 16 + 32);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"AVI ");
        out.extend_from_slice(&hdrl);
        out.extend_from_slice(b"LIST");
        out.extend_from_slice(&(u32::try_from(4 + kept_bytes).must()).to_le_bytes());
        out.extend_from_slice(b"movi");
        out.extend_from_slice(&parent[movi_data..kept_end]);
        out.extend_from_slice(b"idx1");
        out.extend_from_slice(&(u32::try_from(kept * 16).must()).to_le_bytes());
        out.extend_from_slice(&parent[idx1_data..idx1_data + kept * 16]);
        let riff_size = u32::try_from(out.len() - 8).must();
        put_u32_le(&mut out, 4, riff_size);

        let path = self.path.join(name);
        std::fs::write(&path, out).must();
        path
    }
}

/// The open fds of this process (Linux; `None` elsewhere) — the
/// resource-release oracle for the reopen/exit scenarios.
#[must_use]
pub fn open_fds() -> Option<usize> {
    let entries = std::fs::read_dir("/proc/self/fd").ok()?;
    Some(entries.filter_map(|entry| entry.ok()).count())
}

/// The thread count of this process (Linux; `None` elsewhere).
#[must_use]
pub fn thread_count() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Threads:") {
            return rest.trim().parse().ok();
        }
    }
    None
}
