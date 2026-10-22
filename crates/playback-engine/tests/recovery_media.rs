//! The media-level recovery regressions (card 3.C2): one named test
//! per defect in the worklog's W2-19 §C2.0 gap list, each pinning the
//! FIXED shape plus the never-terms the card asserts (no panic, no
//! fabricated output).
//!
//! Defects covered here (the demux-level halves live in
//! `demux`'s suites; the engine halves here):
//!
//! - **G1** — `NoDecodableTrack` dropped the known-format info: the
//!   error must now name the container (when the opening host
//!   declared one) and the codec ids of every track the demuxer
//!   reported. Regression: the `no_decodable_track_*` tests below.
//! - **G3** — audio per-frame damage was fatal: the decode slice now
//!   applies bounded skip-and-report to audio limbs (drop the packet,
//!   count it, NEVER synthesize silence), with the budget shared with
//!   the video concealment lane. Regression: the
//!   `audio_per_frame_damage_*` tests below.
//!
//! The fakes are deliberately local (the session_flow.rs fakes are
//! that suite's; 3.C1's planned hoist to `tests/common` will unify
//! them — this file keeps its own so the two waves cannot collide).

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use media_clock::MonotonicSource;
use media_types::error::{ErrorKind, FormatInfo};
use media_types::frame::{AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet, PipelineStage,
    SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId, TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::state::SessionState;
use playback_engine::{
    DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput, FrameConsumer, QueuePlan,
    SessionConfig, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

// ---------------------------------------------------------------------------
// Fakes (session_flow.rs's shapes, fault-armed for the recovery paths)
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps).
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The collecting sink: every frame the engine delivered, with its
/// track, in delivery order (audio frames are what G3 must never
/// fabricate — the collector is the proof surface).
#[derive(Default)]
struct Collector {
    frames: Mutex<Vec<(TrackId, Frame)>>,
}

impl FrameConsumer for Collector {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((track, frame));
        Ok(())
    }
}

/// The scripted demux stage (session_flow.rs's shape): a fixed track
/// list and a fixed event script.
struct FakeDemuxer {
    tracks: Vec<Track>,
    script: VecDeque<DemuxEvent>,
}

impl Demuxer for FakeDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        match self.script.pop_front() {
            Some(event) => Ok(event),
            None => Ok(DemuxEvent::EndOfStream),
        }
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.not-implemented"),
        ))
    }
}

/// The fault-armed decode stage: decodes normally (one audio frame per
/// packet) EXCEPT on the armed packet indices / when `fail_all` is
/// set, where it answers the armed error — the scripted-decoder fault
/// seam the G3 design names (PCM cannot produce per-frame damage from
/// real bytes; any bytes are valid PCM).
struct FaultDecoder {
    generation: GenerationId,
    /// Decode failures for these packet ordinals (0-based, in decode
    /// order for this limb).
    fail_indices: Vec<u64>,
    /// Fail EVERY packet (the budget-exhaustion shape).
    fail_all: Option<EngineError>,
    seen: u64,
}

impl Decoder for FaultDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let index = self.seen;
        self.seen += 1;
        if let Some(error) = &self.fail_all {
            return Err(shallow_clone(error));
        }
        if self.fail_indices.contains(&index) {
            // The armed fault: a FRESH error per call (the engine
            // stores the last one in the budget terminal's source).
            return Err(fault_error());
        }
        if packet.is_stale(self.generation) {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        out.accept(audio_frame(packet.pts, self.generation))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The per-frame damage fault the tests arm: `Corrupt { Truncated }`
/// is a per-frame kind for BOTH limbs (doc 03 §4.5(d) `CorruptPacket`
/// — "affects only the current packet").
fn fault_error() -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::Truncated {
                expected: 64,
                found: 12,
            },
        },
        media_types::error::ErrorContext::new("decode.packet.corrupt"),
    )
}

/// Clone without the boxed `source` chain (the fake's shared provider
/// needs per-call copies; the engine never clones real errors).
fn shallow_clone(error: &EngineError) -> EngineError {
    EngineError::new(
        error.stage,
        error.kind.clone(),
        media_types::error::ErrorContext {
            code: error.context.code.clone(),
        },
    )
}

/// The decoder seam: one [`FaultDecoder`] per track, armed per test.
struct FaultProvider {
    fail_indices: Vec<u64>,
    fail_all: Option<EngineError>,
}

impl DecoderProvider for FaultProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(Box::new(FaultDecoder {
            generation: config.generation,
            fail_indices: self.fail_indices.clone(),
            fail_all: self.fail_all.as_ref().map(shallow_clone),
            seen: 0,
        }))
    }
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

fn base() -> TimeBase {
    TimeBase::new(1, 1_000_000).must()
}

fn track(id: u32, kind: MediaKind, codec: &str) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new(codec).must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: base(),
            gaps: Vec::new(),
        },
    }
}

fn audio_frame(pts: Stamp, generation: GenerationId) -> Frame {
    Frame::Audio(media_types::frame::AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 48_000,
        },
        samples: SharedBytes::from_static(&[0u8; 4]),
        pts,
        duration: None,
        generation,
    })
}

fn tiny_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

fn tiny_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

fn virtual_clock() -> Arc<media_clock::MediaClock> {
    Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(SharedNow(AtomicU64::new(0))),
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    )
}

/// A frozen virtual throttle source (the session_flow doctrine): the
/// damage coalescer never windows mid-test, so the terminal
/// `flush_all` is the only RecoveredDamage emission.
fn frozen_throttle() -> playback_engine::EventPolicy {
    playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    }
}

/// Everything one test needs: the engine, the collecting sink, and
/// the container declaration the host would make.
struct Fixture {
    engine: Engine,
}

fn open_with(
    container: Option<&str>,
    tracks: Vec<Track>,
    script: Vec<DemuxEvent>,
    provider: FaultProvider,
) -> Fixture {
    let collector = Arc::new(Collector::default());
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-recovery.media").must(),
        Box::new(FakeDemuxer {
            tracks,
            script: VecDeque::from(script),
        }),
        Arc::new(provider),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        virtual_clock(),
    );
    config.container = container.map(str::to_string);
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    let mut engine = Engine::new();
    let _handle = engine.open(config).must();
    Fixture { engine }
}

/// Drive polls until `predicate` holds on the engine state, or fail
/// loudly after `limit` polls (a hang is a failed test — doc 03
/// §4.6(f); the bound IS the never-hang term).
fn poll_until(
    fixture: &mut Fixture,
    predicate: impl Fn(SessionState) -> bool,
    limit: usize,
    context: &str,
) {
    for _ in 0..limit {
        if predicate(fixture.engine.state()) {
            return;
        }
        fixture.engine.poll();
    }
    assert!(
        predicate(fixture.engine.state()),
        "{context}: state is {} after {limit} polls",
        fixture.engine.state()
    );
}

/// Every event output the engine has produced so far.
fn take_events(fixture: &mut Fixture) -> Vec<EngineEvent> {
    fixture
        .engine
        .take_outputs()
        .into_iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => Some(event),
            _ => None,
        })
        .collect()
}

/// The first fatal error on the stream, if any.
fn first_fatal(events: &[EngineEvent]) -> Option<&EngineError> {
    events.iter().find_map(|event| match &event.kind {
        EngineEventKind::Fatal { error } => Some(error),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// G1 — NoDecodableTrack carries the known-format info (defect G1)
// ---------------------------------------------------------------------------

/// **G1 regression (container + fourccs).** A demuxer that reports
/// only non-AV tracks (the engine-level shape of "nothing decodable":
/// `wire()` counts zero limbs) with a host-declared container fails
/// with `Unsupported` NAMING the container and listing every reported
/// codec id in the error's source leaf — the card's known-format-info
/// AC. Before the fix the error was `Unsupported { format: None }`
/// with no source: it named neither the container nor the codecs.
#[test]
fn no_decodable_track_names_container_and_codecs() {
    let mut fixture = open_with(
        Some("avi"),
        vec![
            track(4, MediaKind::Subtitle, "subp"),
            track(5, MediaKind::Data, "txts"),
        ],
        vec![DemuxEvent::EndOfStream],
        FaultProvider {
            fail_indices: Vec::new(),
            fail_all: None,
        },
    );
    poll_until(&mut fixture, |s| s == SessionState::Error, 16, "open fails");
    let events = take_events(&mut fixture);
    let error = first_fatal(&events).expect("a fatal error is on the stream");
    assert_eq!(error.context.code, "session.no-decodable-track");
    // The known-format info: the container name, empty sniff (the
    // engine never sniffed — honesty, not laziness).
    assert_eq!(
        error.kind,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("avi".to_string()),
                sniffed: Vec::new(),
            })
        }
    );
    // The fourcc list rides the source leaf (the DemuxSourceInfo
    // chaining pattern).
    let source = std::error::Error::source(error).expect("the codec-list source leaf");
    let text = source.to_string();
    assert!(
        text.contains("container `avi`"),
        "source leaf names the container: {text}"
    );
    assert!(
        text.contains("subtitle `subp`") && text.contains("data `txts`"),
        "source leaf lists every reported codec: {text}"
    );
    // Display renders the enrichment (the kind summary carries the
    // container name).
    let display = error.to_string();
    assert!(display.contains("avi"), "display names avi: {display}");
}

/// **G1 regression (production shape).** The all-disabled open (the
/// CLI's `--no-video --no-audio`: the host demuxer answers an EMPTY
/// track list) still names the container — the reachable production
/// case for the enrichment — and says "none reported" for the codec
/// list rather than inventing fourccs (never fabricated).
#[test]
fn no_decodable_track_with_empty_track_list_names_container_only() {
    let mut fixture = open_with(
        Some("avi"),
        Vec::new(),
        vec![DemuxEvent::EndOfStream],
        FaultProvider {
            fail_indices: Vec::new(),
            fail_all: None,
        },
    );
    poll_until(&mut fixture, |s| s == SessionState::Error, 16, "open fails");
    let events = take_events(&mut fixture);
    let error = first_fatal(&events).expect("a fatal error is on the stream");
    assert_eq!(error.context.code, "session.no-decodable-track");
    assert_eq!(
        error.kind,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("avi".to_string()),
                sniffed: Vec::new(),
            })
        }
    );
    let source = std::error::Error::source(error).expect("the codec-list source leaf");
    assert!(
        source.to_string().contains("(none reported)"),
        "no codecs were reported and none are invented: {}",
        source.to_string()
    );
}

/// **G1 back-compat pin.** A host that declares no container keeps
/// the historical un-enriched shape (`format: None`, no source leaf)
/// — the enrichment is additive, never a fabricated guess.
#[test]
fn no_decodable_track_without_container_stays_unenriched() {
    let mut fixture = open_with(
        None,
        vec![track(9, MediaKind::Subtitle, "subp")],
        vec![DemuxEvent::EndOfStream],
        FaultProvider {
            fail_indices: Vec::new(),
            fail_all: None,
        },
    );
    poll_until(&mut fixture, |s| s == SessionState::Error, 16, "open fails");
    let events = take_events(&mut fixture);
    let error = first_fatal(&events).expect("a fatal error is on the stream");
    assert_eq!(error.context.code, "session.no-decodable-track");
    assert_eq!(error.kind, ErrorKind::Unsupported { format: None });
    // The codec list still rides the leaf (the container half alone
    // is absent — the tracks half is always known).
    let source = std::error::Error::source(error).expect("the codec-list source leaf");
    assert!(
        source.to_string().contains("subtitle `subp`"),
        "codec list present without a container: {}",
        source.to_string()
    );
}
