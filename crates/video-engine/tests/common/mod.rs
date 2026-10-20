//! Shared fixtures for the video-engine integration suites (card
//! 2.C4 AC). Integration tests cannot see the crate's cfg(test)
//! `test_support`, so this module carries the production-shaped
//! fakes: virtual clocks over media-clock's injected-truth traits
//! (doc 03 §8 — no sleeps, ever), synthetic frames, and a recording
//! presenter.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use media_clock::{MediaClock, MonotonicSource, NANO_BASE, PlaybackClock, SampleCounterSource};
use media_types::error::{DeviceCause, DeviceSlot, ErrorContext, ErrorKind};
use media_types::frame::{ColorDescription, FrameStorage, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::time::{Rate, Stamp, Timestamp};
use media_types::{EngineError, GenerationId, PixelFormat};
use testkit::Must;
use video_engine::{PresentInfo, PresentRequest, Presenter};

// ---------------------------------------------------------------------------
// Virtual clocks (media-clock's FakeCounter pattern; exact, no sleeps)
// ---------------------------------------------------------------------------

/// A deterministic monotonic source: plain nanoseconds, test-driven.
pub struct FakeMonotonic(AtomicU64);

impl MonotonicSource for FakeMonotonic {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The virtual clock most scheduling tests drive: a [`MediaClock`] in
/// system-monotonic master (the audio-less playback configuration,
/// doc 03 §4.7 d) over a settable nanosecond source — exact
/// nanosecond positions at rate 1.
pub struct VirtualClock {
    source: Arc<FakeMonotonic>,
    clock: Arc<MediaClock>,
}

impl VirtualClock {
    /// A clock at position zero, running, generation `FIRST`.
    pub fn new() -> VirtualClock {
        let source = Arc::new(FakeMonotonic(AtomicU64::new(0)));
        let clock = MediaClock::system_monotonic(
            source.clone(),
            Timestamp::new(0, NANO_BASE),
            Rate::ONE,
        )
        .must();
        VirtualClock {
            source,
            clock: Arc::new(clock),
        }
    }

    /// The scheduler-facing handle.
    pub fn clock(&self) -> Arc<dyn PlaybackClock> {
        self.clock.clone()
    }

    /// Move the virtual clock to an absolute position (monotonic-safe:
    /// never backwards — a regression would be a source discontinuity
    /// no scheduling test wants to inject by accident).
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

    /// Freeze the virtual position (media-clock pause: `now()` frozen).
    pub fn pause(&self) {
        self.clock.pause();
    }

    /// Continue from the frozen position (no discontinuity).
    pub fn resume(&self) {
        self.clock.resume();
    }
}

/// The production-shaped virtual clock: audio master over an injected
/// sample counter (media-clock's own FakeCounter test pattern). The
/// scheduler sees exactly the samples the test advances — the
/// configuration real playback uses (doc 01 §1.3 rule 8).
pub struct AudioMasterClock {
    counter: Arc<FakeCounter>,
    clock: Arc<MediaClock>,
}

/// A deterministic fake sample counter (48 kHz).
pub struct FakeCounter {
    sample: AtomicU64,
    rate_hz: u32,
}

impl SampleCounterSource for FakeCounter {
    fn sample_count(&self) -> u64 {
        self.sample.load(Ordering::Relaxed)
    }

    fn sample_rate_hz(&self) -> u32 {
        self.rate_hz
    }
}

impl AudioMasterClock {
    /// An audio-master clock at media position zero, 48 kHz, running.
    pub fn new() -> AudioMasterClock {
        let counter = Arc::new(FakeCounter {
            sample: AtomicU64::new(0),
            rate_hz: 48_000,
        });
        let clock = MediaClock::audio_master(
            counter.clone(),
            Timestamp::new(0, NANO_BASE),
            Rate::ONE,
        )
        .must();
        AudioMasterClock {
            counter,
            clock: Arc::new(clock),
        }
    }

    /// The scheduler-facing handle.
    pub fn clock(&self) -> Arc<dyn PlaybackClock> {
        self.clock.clone()
    }

    /// The pump consumed `samples` more samples; the clock follows.
    pub fn advance_samples(&self, samples: u64) {
        let _ = self.counter.sample.fetch_add(samples, Ordering::Relaxed);
    }

    /// Publish a session generation on the clock.
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
        color: ColorDescription::BT709_LIMITED,
        interlaced: false,
        generation,
    }
}

/// As [`frame`], but with a missing presentation timestamp.
pub fn frame_missing(generation: GenerationId) -> VideoFrame {
    VideoFrame {
        pts: Stamp::Missing,
        ..frame(0, generation)
    }
}

// ---------------------------------------------------------------------------
// Presenters
// ---------------------------------------------------------------------------

/// The presenter failure the scripted-failure tests use: a busy
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

/// One recorded presentation: the scheduled slot, the generation, and
/// the clock position the presenter echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentRecord {
    /// The request's scheduled slot (nanoseconds).
    pub when_ns: i64,
    /// The request's session generation.
    pub generation: GenerationId,
}

/// A presenter that records every request it is handed (doc 03 §8's
/// `FrameRecorder` shape), optionally failing at a scripted index.
pub struct RecordingPresenter {
    log: Vec<PresentRecord>,
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
        self.log.iter().map(|r| r.when_ns).collect()
    }

    /// The presented generations, in delivery order.
    pub fn gens_seen(&self) -> Vec<GenerationId> {
        self.log.iter().map(|r| r.generation).collect()
    }

    /// The full records, in delivery order.
    pub fn records(&self) -> &[PresentRecord] {
        &self.log
    }

    /// Assert the presented slots are non-decreasing (the doc 03
    /// §4.10 f in-order property) — with a caller-supplied context for
    /// the failure message.
    pub fn assert_in_order(&self, context: &str) {
        for pair in self.log.windows(2) {
            assert!(
                pair[0].when_ns <= pair[1].when_ns,
                "{context}: presented {} after {} — out of order",
                pair[1].when_ns,
                pair[0].when_ns
            );
        }
    }
}

impl Presenter for RecordingPresenter {
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        if self.fail_at == Some(self.log.len()) {
            return Err(presenter_failure());
        }
        self.log.push(PresentRecord {
            when_ns: request.when.ticks,
            generation: request.generation,
        });
        Ok(PresentInfo {
            presented_at: request.when,
        })
    }
}
