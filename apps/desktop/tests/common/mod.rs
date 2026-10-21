//! Shared fixtures for the desktop integration suites (card 2.D3
//! AC). Test code only: a wrong shape here is a broken *fixture*, so
//! the `unreachable!`s live behind the testkit `must` vocabulary
//! (AGENTS §3.5, the 2.D1 common convention).
//!
//! - a **virtual timeline**: one `AtomicU64` nanosecond source
//!   serving BOTH the media clock (scheduler ticks) and the bridge's
//!   emit clock — so end-to-end latency is measurable in one
//!   coordinate system (the thing a desktop run needs the
//!   `[PROPOSAL]` origin handshake for);
//! - synthetic frame builders over `renderer_wgpu::synthetic` with
//!   a per-frame **marker pixel** (the 2.D1 convention: pixel (5,5)
//!   carries the frame index, so every claim is checked against
//!   *content*, not counters).

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use desktop::EmitClock;
use media_clock::{MediaClock, MonotonicSource, NANO_BASE, PlaybackClock};
use media_types::GenerationId;
use media_types::frame::{
    ColorDescription, ColorMatrix, ColorPrimaries, ColorRange, FrameStorage,
    TransferCharacteristics, VideoFrame,
};
use media_types::packet::SharedBytes;
use media_types::time::{Rate, Stamp, Timestamp};
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::synthetic;
use testkit::Must;
use video_engine::PresentRequest;

/// The settable monotonic source (media-clock's contract).
struct FakeMonotonic(Arc<AtomicU64>);

impl MonotonicSource for FakeMonotonic {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// One nanosecond timeline shared by the scheduler and the bridge
/// (see the module docs — the single-coordinate-system trick).
pub struct Timeline {
    ns: Arc<AtomicU64>,
}

impl Timeline {
    /// A timeline at zero.
    pub fn new() -> Timeline {
        Timeline {
            ns: Arc::new(AtomicU64::new(0)),
        }
    }

    /// The scheduler-facing clock.
    pub fn media_clock(&self) -> Arc<dyn PlaybackClock> {
        let clock = MediaClock::system_monotonic(
            Arc::new(FakeMonotonic(Arc::clone(&self.ns))),
            Timestamp::new(0, NANO_BASE),
            Rate::ONE,
        )
        .must();
        Arc::new(clock)
    }

    /// The bridge-facing emit clock (the same timeline).
    pub fn emit_clock(&self) -> TimelineEmitClock {
        TimelineEmitClock {
            ns: Arc::clone(&self.ns),
        }
    }

    /// Move to an absolute position (monotonic-safe).
    pub fn set_ns(&self, at: u64) {
        let _ = self.ns.fetch_max(at, Ordering::Relaxed);
    }

    /// Advance by a delta.
    pub fn advance_ns(&self, by: u64) {
        let _ = self.ns.fetch_add(by, Ordering::Relaxed);
    }

    /// The current position.
    pub fn now_ns(&self) -> u64 {
        self.ns.load(Ordering::Relaxed)
    }
}

/// The bridge's emit clock on the shared timeline.
pub struct TimelineEmitClock {
    ns: Arc<AtomicU64>,
}

impl EmitClock for TimelineEmitClock {
    fn emit_ns(&self) -> u64 {
        self.ns.load(Ordering::Relaxed)
    }
}

/// The synthetic pattern with pixel (5,5) overwritten by the frame
/// index (the marker slot — clear of the corner markers and grid).
pub fn marked_bytes(extent: Extent2D, index: u8) -> Vec<u8> {
    let mut rgba = synthetic::synthetic_pattern(extent);
    let at = (5 * extent.width as usize + 5) * 4;
    rgba[at..at + 4].copy_from_slice(&[index, 0xEE, 0x77, 255]);
    rgba
}

/// A synthetic frame carrying `index`'s marker, PTS-stamped for the
/// `pts_ns` slot, sRGB full-range (the synthetic pattern's color).
pub fn marked_frame(extent: Extent2D, pts_ns: i64, index: u8) -> VideoFrame {
    VideoFrame {
        format: media_types::PixelFormat::Rgba8,
        size: (extent.width, extent.height),
        data: FrameStorage::Cpu(SharedBytes::from_vec(marked_bytes(extent, index))),
        pts: Stamp::Present(Timestamp::new(pts_ns, NANO_BASE)),
        color: ColorDescription {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Srgb,
            matrix: ColorMatrix::Bt709,
            range: ColorRange::Full,
        },
        interlaced: false,
        generation: GenerationId::FIRST,
    }
}

/// A marked request scheduled for the `when_ns` slot.
pub fn marked_request(extent: Extent2D, when_ns: i64, index: u8) -> PresentRequest {
    PresentRequest {
        frame: marked_frame(extent, when_ns, index),
        when: Timestamp::new(when_ns, NANO_BASE),
        generation: GenerationId::FIRST,
    }
}

/// Read a frame payload's marker pixel — *which frame these bytes
/// carry*.
pub fn payload_marker(extent: Extent2D, payload: &[u8]) -> [u8; 4] {
    let at = (5 * extent.width as usize + 5) * 4;
    [
        payload[at],
        payload[at + 1],
        payload[at + 2],
        payload[at + 3],
    ]
}
