//! Shared fixtures for the crate's unit tests (cfg(test)-gated; the
//! integration suites under `tests/` carry their own copy — dev-only
//! types can never leak into the public API, ARCH-7).

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use media_clock::{MediaClock, NANO_BASE, PlaybackClock};
use media_types::error::{DeviceCause, DeviceSlot, ErrorContext, ErrorKind};
use media_types::frame::{FrameStorage, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::time::{Rate, Stamp, Timestamp};
use media_types::{EngineError, GenerationId, PixelFormat};
use testkit::Must;

use crate::sink::{PresentInfo, PresentRequest, Presenter};

// ---------------------------------------------------------------------------
// Virtual time (media-clock's injected-truth pattern: exact-ns control,
// no sleeps — doc 03 §8)
// ---------------------------------------------------------------------------

/// A deterministic monotonic source: plain nanoseconds, test-driven.
pub struct FakeMonotonic(AtomicU64);

impl media_clock::MonotonicSource for FakeMonotonic {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The virtual clock the scheduling tests drive: a [`MediaClock`] in
/// system-monotonic master over a settable nanosecond source. The
/// master choice is deliberate — it is the audio-less playback
/// configuration (doc 03 §4.7 d), and it gives exact nanosecond
/// positions (`now().ticks` equals the source's nanoseconds at rate 1).
pub struct VirtualVideoClock {
    source: Arc<FakeMonotonic>,
    clock: Arc<MediaClock>,
}

impl VirtualVideoClock {
    /// A clock at position zero, running, generation `FIRST`.
    pub fn new() -> VirtualVideoClock {
        let source = Arc::new(FakeMonotonic(AtomicU64::new(0)));
        let clock =
            MediaClock::system_monotonic(source.clone(), Timestamp::new(0, NANO_BASE), Rate::ONE)
                .must();
        VirtualVideoClock {
            source,
            clock: Arc::new(clock),
        }
    }

    /// The scheduler-facing handle.
    pub fn clock(&self) -> Arc<dyn PlaybackClock> {
        self.clock.clone()
    }

    /// Move the virtual clock to an absolute position (monotonic-safe:
    /// never backwards — a regression would be a source discontinuity,
    /// which no scheduling test wants to inject by accident).
    pub fn set_ns(&self, at: u64) {
        let _ = self.source.0.fetch_max(at, Ordering::Relaxed);
    }

    /// Advance the virtual clock by a delta.
    pub fn advance_ns(&self, by: u64) {
        let _ = self.source.0.fetch_add(by, Ordering::Relaxed);
    }

    /// The current virtual position in nanoseconds.
    pub fn now_ns(&self) -> u64 {
        self.source.0.load(Ordering::Relaxed)
    }

    /// Publish a session generation on the clock (the engine's seek /
    /// track-switch / stop mint — doc 03 §5).
    pub fn set_generation(&self, generation: GenerationId) {
        self.clock.set_generation(generation);
    }
}

// ---------------------------------------------------------------------------
// Synthetic frames and requests
// ---------------------------------------------------------------------------

/// A minimal 2×2 NV12 frame stamped at `pts_ns` on the clock's
/// nanosecond base.
pub fn frame(pts_ns: i64, generation: GenerationId) -> VideoFrame {
    VideoFrame {
        format: PixelFormat::Nv12,
        size: (2, 2),
        data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0u8; 6])),
        pts: Stamp::Present(Timestamp::new(pts_ns, NANO_BASE)),
        color: media_types::frame::ColorDescription::BT709_LIMITED,
        interlaced: false,
        generation,
    }
}

/// As [`frame`], but stamped in an arbitrary stream base (conversion
/// coverage: the scheduler normalizes at push).
pub fn frame_stream_base(ticks: i64, num: u32, den: u32, generation: GenerationId) -> VideoFrame {
    VideoFrame {
        pts: Stamp::Present(Timestamp::from_parts(ticks, num, den).must()),
        ..frame(0, generation)
    }
}

/// As [`frame`], but with a missing presentation timestamp.
pub fn frame_missing(generation: GenerationId) -> VideoFrame {
    VideoFrame {
        pts: Stamp::Missing,
        ..frame(0, generation)
    }
}

/// A present request for slot `when_ns` under `generation`.
pub fn request(when_ns: i64, generation: GenerationId) -> PresentRequest {
    PresentRequest {
        frame: frame(when_ns, generation),
        when: Timestamp::new(when_ns, NANO_BASE),
        generation,
    }
}

// ---------------------------------------------------------------------------
// Presenters
// ---------------------------------------------------------------------------

/// The presenter failure every scripted-failure test uses: a busy
/// display (stage Present, structured, no secrets).
pub fn presenter_failure() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Present,
        ErrorKind::Device {
            which: DeviceSlot::Display,
            cause: DeviceCause::Busy,
        },
        ErrorContext::new("video.presenter.refused"),
    )
}

/// A presenter that records every request it is handed (doc 03 §8's
/// `FrameRecorder` shape, local to this crate until testkit grows it),
/// optionally failing at a scripted index.
pub struct RecordingPresenter {
    log: Vec<(i64, GenerationId)>,
    fail_at: Option<usize>,
}

impl RecordingPresenter {
    /// A healthy presenter.
    pub fn new() -> RecordingPresenter {
        RecordingPresenter {
            log: Vec::new(),
            fail_at: None,
        }
    }

    /// A presenter that refuses the `fail_at`-th request (0-based).
    pub fn failing_at(fail_at: usize) -> RecordingPresenter {
        RecordingPresenter {
            log: Vec::new(),
            fail_at: Some(fail_at),
        }
    }

    /// The presented slots, in delivery order.
    pub fn slots_seen(&self) -> Vec<i64> {
        self.log.iter().map(|(when, _)| *when).collect()
    }

    /// The presented generations, in delivery order.
    pub fn gens_seen(&self) -> Vec<GenerationId> {
        self.log.iter().map(|(_, generation)| *generation).collect()
    }
}

impl Presenter for RecordingPresenter {
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        if self.fail_at == Some(self.log.len()) {
            return Err(presenter_failure());
        }
        self.log.push((request.when.ticks, request.generation));
        Ok(PresentInfo {
            presented_at: request.when,
        })
    }
}
