//! F5 (issue #16): a video-only session's end-of-media must wait for
//! the media to play out.
//!
//! The measured defect (worklog 2026-10, finding F5): mp4 corpus 0115
//! (fps-extremes, mjpeg, 4.0 s container duration) reported
//! end-of-media at 0.111043 s — 2.8% of the declared extent — through
//! `advance 512`. Root cause: the engine's drain condition
//! (`is_drained`: demux EOF + queues empty + decoders flushed) is pure
//! state inspection with no relation to the presentation clock. A
//! video-only session has no audio renders to move the clock, so the
//! bounded decode/flush loop outruns the clock and `Drained` fires at
//! drain-completion time, not at media end — the eom position is a
//! wall-time artifact, not a media fact. (Audio limbs are exempt: the
//! render-driven clock and the device-ring drain gate keep them
//! honest; see the `DrainGatedDecoder` note in the CLI host.)
//!
//! The regression here is engine-level and deterministic: a video-only
//! session whose clock has NOT reached the last decoded frame's pts
//! must stay `Playing` after the script, queues and decoders are all
//! exhausted; end-of-media lands only once the clock presents the
//! final frame.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use media_clock::MonotonicSource;
use media_types::frame::{Frame, FrameStorage, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::{
    Duration, EngineError, MediaKind, Nanoseconds, Packet, PixelFormat, Rate, SourceUri, Stamp,
    TimeBase, TimeSpan, Track, TrackFlags, TrackId, TrackTiming,
};
use playback_engine::state::SessionState;
use playback_engine::{DecoderProvider, Engine, EngineEventKind, EngineOutput, FrameConsumer};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

fn micros_base() -> TimeBase {
    TimeBase::new(1, 1_000_000).must()
}

fn at_micros(us: i64) -> Stamp {
    Stamp::Present(media_types::Timestamp::new(us, micros_base()))
}

/// A virtual monotonic source the test advances by hand — the frozen
/// clock is the defect's stage (drain completes, clock never moves).
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// One video track (the fps-extremes family: video-only, no audio
/// limb to render the clock forward).
fn video_track() -> Track {
    Track {
        id: TrackId::new(1),
        stream: media_types::StreamId::new(1),
        kind: MediaKind::Video,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("mjpeg").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: micros_base(),
            gaps: Vec::new(),
        },
    }
}

/// `count` video packets at 1 ms spacing (the high-fps shape: the
/// whole media decodes in far fewer ticks than it plays).
fn video_script(count: u64) -> VecDeque<DemuxEvent> {
    let mut script: VecDeque<DemuxEvent> = (0..count)
        .map(|index| {
            DemuxEvent::Packet(Packet {
                track: TrackId::new(1),
                pts: at_micros((index * 1_000) as i64),
                dts: Stamp::Missing,
                duration: Some(Duration::Known(Nanoseconds(1_000_000))),
                data: SharedBytes::from_vec(vec![0xA5u8; 96]),
                keyframe: true,
                discontinuity: false,
                generation: media_types::GenerationId::FIRST,
            })
        })
        .collect();
    script.push_back(DemuxEvent::EndOfStream);
    script
}

struct ScriptDemuxer {
    tracks: Vec<Track>,
    script: Arc<Mutex<VecDeque<DemuxEvent>>>,
}

impl Demuxer for ScriptDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        Ok(self
            .script
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop_front()
            .unwrap_or(DemuxEvent::EndOfStream))
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("test.no-seek"),
        ))
    }

    fn seekability(&self) -> SeekSupport {
        SeekSupport::None
    }
}

/// A stateless per-frame video decoder: one frame per packet, at the
/// packet's pts; flush drains immediately (the mjpeg shape).
struct VideoDecoder {
    generation: media_types::GenerationId,
}

impl Decoder for VideoDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        out.accept(Frame::Video(VideoFrame {
            format: PixelFormat::I420,
            size: (8, 8),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0u8; 8 * 8 * 3 / 2])),
            pts: packet.pts,
            color: media_types::frame::ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation: self.generation,
        }))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

struct VideoProvider;

impl DecoderProvider for VideoProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(Box::new(VideoDecoder {
            generation: config.generation,
        }))
    }
}

/// Collects delivered frames (the consumer seam).
#[derive(Default)]
struct Collector {
    frames: Mutex<u64>,
}

impl FrameConsumer for Collector {
    fn accept(&self, _frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        let mut frames = self
            .frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *frames = frames.saturating_add(1);
        Ok(())
    }
}

/// Open a video-only session over `count` 1 ms frames with a frozen
/// virtual clock the test advances by hand.
fn open_video_frozen(count: u64) -> (Engine, Arc<SharedNow>) {
    let now = Arc::new(SharedNow(AtomicU64::new(0)));
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::clone(&now) as Arc<dyn MonotonicSource>,
            media_types::Timestamp::new(0, micros_base()),
            Rate::ONE,
        )
        .must(),
    );
    let collector = Arc::new(Collector::default());
    let script = Arc::new(Mutex::new(video_script(count)));
    let mut config = playback_engine::SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-f5-fps-extremes.mp4").must(),
        Box::new(ScriptDemuxer {
            tracks: vec![video_track()],
            script: Arc::clone(&script),
        }),
        Arc::new(VideoProvider),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        clock,
    );
    config.throttle = playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    };
    let mut engine = Engine::new();
    let _handle = engine.open(config).must();
    (engine, now)
}

fn has_end_of_media(engine: &mut Engine) -> bool {
    engine.take_outputs().iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(event)
                if matches!(event.kind, EngineEventKind::EndOfMedia)
        )
    })
}

#[test]
fn video_only_end_of_media_waits_for_the_clock_to_present_the_last_frame() {
    // 100 frames at 1 ms: the last frame's pts is 99 ms. The script,
    // queues and decoder all exhaust long before 99 ms of clock.
    let (mut engine, now) = open_video_frozen(100);

    // Reach Playing (priming), then drain everything: enough polls
    // that the demuxer hits EOF, the queues empty and the decoder
    // flushes — the exact state in which the pre-fix engine fired
    // end-of-media at drain time (F5: position 0.111 s of a 4.0 s
    // media — 2.8%).
    let mut playing = false;
    for _ in 0..400 {
        engine.poll();
        if engine.state() == SessionState::Playing {
            playing = true;
        }
        if playing && engine.state() != SessionState::Playing {
            break;
        }
    }
    assert!(
        playing,
        "the fixture must reach Playing before the drain window"
    );
    // The F5 assertion: still Playing — the media has NOT played out
    // (the clock is frozen at 0, the last frame's pts is 99 ms).
    assert_eq!(
        engine.state(),
        SessionState::Playing,
        "F5 regression: end-of-media fired before the video played out \
         (drain-completion is not media end)"
    );
    assert!(
        !has_end_of_media(&mut engine),
        "no end-of-media event before play-out"
    );

    // Present the media: advance the clock past the last frame's pts
    // (99 ms) — a single hand-move, then the drain translates.
    now.0.store(200_000_000, Ordering::Relaxed);
    let mut ended = false;
    for _ in 0..40 {
        engine.poll();
        if engine.state() == SessionState::Ended {
            ended = true;
            break;
        }
    }
    assert!(
        ended,
        "end-of-media must land once the clock has presented the last frame"
    );
    assert!(
        has_end_of_media(&mut engine),
        "the end-of-media event follows the Ended transition"
    );
}

#[test]
fn a_short_video_media_whose_first_frames_cover_the_clock_ends_at_drain() {
    // The honest complement: when the frozen clock already sits at or
    // past the last frame's pts (here: one frame at pts 0), the drain
    // translates immediately — the gate never wedges a fully
    // presented media.
    let (mut engine, _now) = open_video_frozen(1);
    let mut ended = false;
    for _ in 0..400 {
        engine.poll();
        if engine.state() == SessionState::Ended {
            ended = true;
            break;
        }
    }
    assert!(
        ended,
        "a media whose last frame is already presented ends at drain"
    );
}
