//! The real-time render path: the callback contract, its pure core,
//! and the per-call operation ledger (doc 03 §4.9 c — binding).
//!
//! # The contract, enforced by shape
//!
//! [`render_into`] is the entire callback path. It:
//!
//! - takes **only borrows** (`&SpscRing`, `&SampleCounter`,
//!   `&AtomicU64`, `&mut [f32]`) — it *cannot* allocate, because it
//!   constructs no owned value and calls nothing that does (the full
//!   call graph is this function, [`SpscRing::read`], one
//!   [`SampleCounter::advance`], one `fetch_add`, and slice writes);
//! - touches **no lock** (the ring is single-word atomics; the
//!   counters are single-word atomics);
//! - makes **no syscall** (nothing in the graph can);
//! - loops **boundedly**: one ring read of at most one capacity (≤ 2
//!   contiguous segments — the power-of-two wrap argument) plus one
//!   silence fill.
//!
//! # The measured budget
//!
//! [`RenderStats`] is the ledger: frames requested/read, silence
//! filled, ring segments visited. The callback-budget AC asserts, per
//! call: `ring_segments ≤ 2`, `silence_fills ≤ 1`, exactly one
//! counter advance, and (driver-side, outside the callback) zero
//! storage growth across any number of renders. What this v0 gate
//! cannot do is *trap* an allocation the way doc 03 §4.9 c's
//! fault-injecting allocator would — that hook needs a
//! `#[global_allocator]` (i.e. `unsafe`, platform-adapter territory
//! per doc 01 §2.7) and is `[PROPOSAL: tooling]` owed to testkit
//! hardening. Doc 03's own enforcement clause is "code review plus
//! these tests are the gate"; both halves of that sentence are
//! delivered here.
//!
//! # Underrun policy
//!
//! Doc 03 §4.8 d: *"audible glitch policy: output silence, never
//! repeat old audio"* — the renderer fills the unread tail of `out`
//! with zeros and counts **one underrun event per callback that could
//! not fill entirely from the ring**.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::counter::SampleCounter;
use crate::ring::SpscRing;

/// The device's render callback (doc 03 §4.9, verbatim shape): copy
/// interleaved frames from the ring into `out`. Runs on the device's
/// high-priority thread — implementors are bound by the no-blocking
/// contract (module docs).
pub trait RenderCallback: Send {
    /// Fill `out` with the next `out.len()` interleaved samples. The
    /// whole call is the real-time section: no locks (except
    /// wait-free index swaps), no allocation, no logging, no
    /// syscalls, no unbounded loops (doc 03 §4.9 c).
    fn render(&mut self, out: &mut [f32]);
}

/// One callback's operation ledger (the callback-budget evidence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderStats {
    /// Whole frames the device asked for (`out.len() / channels`).
    pub frames_requested: usize,
    /// Whole frames copied from the ring.
    pub frames_read: usize,
    /// Frames of silence filled (the underrun tail; 0 on a full read).
    pub silence_filled: usize,
    /// Contiguous ring segments visited (budget: ≤ 2 — one wrap).
    pub ring_segments: usize,
    /// Whether this call counted an underrun event.
    pub underrun: bool,
}

impl RenderStats {
    /// Whether the callback fully served the device from the ring.
    pub fn fully_served(&self) -> bool {
        self.frames_read == self.frames_requested
    }
}

/// The pure callback core (module docs): ring → `out`, silence on
/// underrun, counter advance, underrun count. This is the only code
/// that runs under the real-time rules.
///
/// `out.len()` must be a multiple of `channels` — a device buffer is
/// always whole frames; an unaligned tail is left **untouched** (the
/// driver-side wrapper validates and reports; the RT core stays
/// branch-cheap).
pub fn render_into(
    ring: &SpscRing,
    counter: &SampleCounter,
    underruns: &AtomicU64,
    out: &mut [f32],
) -> RenderStats {
    // `channels` is >= 1 by the ring's validated construction; a
    // device buffer is always whole frames, and any unaligned tail is
    // left untouched (the driver wrapper validates and reports).
    let channels = ring.channels() as usize;
    let frames_requested = out.len() / channels;
    let read = ring.read(&mut out[..frames_requested * channels]);
    let silence = frames_requested - read.frames;
    if silence > 0 {
        // Output silence, never repeat old audio (doc 03 §4.8 d).
        out[read.frames * channels..frames_requested * channels].fill(0.0);
    }
    // One counter advance: every rendered frame (data or silence)
    // was consumed by the device.
    counter.advance(frames_requested as u64);
    let underrun = silence > 0;
    if underrun {
        let _ = underruns.fetch_add(1, Ordering::Relaxed);
    }
    RenderStats {
        frames_requested,
        frames_read: read.frames,
        silence_filled: silence,
        ring_segments: read.segments,
        underrun,
    }
}

/// The production [`RenderCallback`]: a shared ring + counters view.
///
/// Constructed at open; moved into (or handed to) whatever drives the
/// device. The callback itself is the doc's `RenderCallback` shape —
/// the stats ledger is available through the driver-side wrapper that
/// calls [`render_into`] directly.
#[derive(Debug)]
pub struct RingRenderer {
    ring: std::sync::Arc<SpscRing>,
    counter: std::sync::Arc<SampleCounter>,
    underruns: std::sync::Arc<AtomicU64>,
    channels: u32,
}

impl RingRenderer {
    /// The renderer's channel count (whole-frame contract).
    pub fn channels(&self) -> u32 {
        self.channels
    }

    /// Access the shared ring (read-only view for introspection).
    pub fn ring(&self) -> &SpscRing {
        &self.ring
    }
}

impl RenderCallback for RingRenderer {
    fn render(&mut self, out: &mut [f32]) {
        // The trait shape returns nothing; the ledger is discardable
        // by design (the RT path stays allocation- and branch-cheap).
        render_into(&self.ring, &self.counter, &self.underruns, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::SampleCounter;
    use crate::ring::SpscRing;
    use testkit::Must;

    fn setup(channels: u32, frames: usize) -> (SpscRing, SampleCounter, AtomicU64) {
        (
            SpscRing::new(channels, frames).must(),
            SampleCounter::new(44_100).must(),
            AtomicU64::new(0),
        )
    }

    #[test]
    fn full_read_copies_everything_and_advances_once() {
        let (ring, counter, underruns) = setup(2, 8);
        ring.push(&[1.0, 2.0, 3.0, 4.0]); // 2 frames
        let mut out = vec![f32::NAN; 4];
        let stats = render_into(&ring, &counter, &underruns, &mut out);
        assert_eq!(
            stats,
            RenderStats {
                frames_requested: 2,
                frames_read: 2,
                silence_filled: 0,
                ring_segments: 1,
                underrun: false,
            }
        );
        assert_eq!(out, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(counter.count(), 2);
        assert_eq!(underruns.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn underrun_fills_silence_and_counts_one_event() {
        let (ring, counter, underruns) = setup(2, 8);
        ring.push(&[1.0, 2.0]); // 1 frame of 3 requested
        let mut out = vec![f32::NAN; 6];
        let stats = render_into(&ring, &counter, &underruns, &mut out);
        assert_eq!(stats.frames_read, 1);
        assert_eq!(stats.silence_filled, 2);
        assert!(stats.underrun);
        assert!(!stats.fully_served());
        assert_eq!(out, vec![1.0, 2.0, 0.0, 0.0, 0.0, 0.0]);
        // Both data and silence advanced the device counter.
        assert_eq!(counter.count(), 3);
        assert_eq!(underruns.load(Ordering::Relaxed), 1);
        // A second, fully-empty call: another event, all silence.
        let mut out = vec![f32::NAN; 4];
        let stats = render_into(&ring, &counter, &underruns, &mut out);
        assert_eq!(stats.frames_read, 0);
        assert_eq!(stats.silence_filled, 2);
        assert!(stats.underrun);
        assert_eq!(counter.count(), 5);
        assert_eq!(underruns.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn ring_segments_stay_within_the_power_of_two_wrap_budget() {
        // Fill across a wrap, then read the whole capacity: the read
        // crosses exactly one boundary — 2 segments, never more.
        let (ring, counter, underruns) = setup(1, 4);
        for round in 0..4 {
            let mut chunk = Vec::new();
            for i in 0..3 {
                chunk.push(round as f32 * 10.0 + i as f32);
            }
            let mut sink = [0.0f32; 1];
            let _ = ring.push(&chunk);
            let read = ring.read(&mut sink);
            let _ = read;
        }
        // Now a maximal read from a wrapped state.
        ring.push(&[1.0, 2.0, 3.0]);
        let mut out = [0.0f32; 4];
        ring.push(&[4.0]);
        let stats = render_into(&ring, &counter, &underruns, &mut out);
        assert_eq!(stats.frames_read, 4);
        assert!(stats.ring_segments <= 2, "budget violated: {}", stats.ring_segments);
        assert_eq!(stats.ring_segments, 2);
    }

    #[test]
    fn renderer_trait_callback_produces_the_same_output() {
        use std::sync::Arc;
        let ring = Arc::new(SpscRing::new(2, 8).must());
        let counter = Arc::new(SampleCounter::new(44_100).must());
        let underruns = Arc::new(AtomicU64::new(0));
        let mut renderer = RingRenderer {
            ring: ring.clone(),
            counter: counter.clone(),
            underruns: underruns.clone(),
            channels: 2,
        };
        ring.push(&[0.5, -0.5]);
        let mut out = vec![0.0; 4];
        renderer.render(&mut out);
        assert_eq!(out, vec![0.5, -0.5, 0.0, 0.0]);
        assert_eq!(counter.count(), 2);
        assert_eq!(underruns.load(Ordering::Relaxed), 1);
        assert_eq!(renderer.channels(), 2);
    }
}
