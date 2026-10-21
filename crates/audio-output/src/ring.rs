//! The output ring: a wait-free SPSC ring of interleaved f32 frames
//! (doc 03 §4.9 c — "the SPSC ring's wait-free index swap is the only
//! synchronization on the callback path").
//!
//! # Design
//!
//! - Capacity is a **power-of-two number of frames**; each frame is
//!   `channels` slots of `AtomicU32` holding f32 bits (std has no
//!   `AtomicF32`; bit-casting through `AtomicU32` is exact and
//!   wait-free).
//! - Two monotonic cursors (`write`, `read`, in frames): the producer
//!   is the only writer of `write`, the consumer the only writer of
//!   `read` — the SPSC discipline. Index math is `cursor & mask`, so
//!   a read or write of at most one capacity crosses **at most one**
//!   wrap boundary: ≤ 2 contiguous segments per operation (the
//!   bounded-loop guarantee the no-blocking contract cites).
//! - Ordering: slot stores are `Relaxed`, cursor publishes are
//!   `Release`, cursor loads are `Acquire` — the standard
//!   release/acquire pairing that makes the producer's slots visible
//!   to the consumer and the consumer's reads visible to the
//!   producer. Nothing else synchronizes; there are no locks.
//!
//! The ring never allocates after construction (open time), never
//! blocks, and never distinguishes "empty" from "full" by luck:
//! `available = write - read`, `free = capacity - available`.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use media_types::EngineError;

use crate::error;

/// The outcome of one consumer-side read: whole frames moved, and how
/// many contiguous ring segments they came from (the op-count ledger
/// entry for the callback budget — bounded by 2 per call, see the
/// module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RingRead {
    /// Whole frames copied into the output buffer.
    pub frames: usize,
    /// Contiguous segments visited (≤ 2 for any single read).
    pub segments: usize,
}

/// A single-producer single-consumer ring of interleaved f32 frames.
///
/// **SPSC discipline (binding):** exactly one producer calls
/// [`SpscRing::push`], exactly one consumer calls [`SpscRing::read`].
/// The type is `Sync` because the halves are handed out separately
/// ([`crate::NullStream`]); violating the discipline is a logic bug
/// the type cannot catch — it is the documented contract (same as
/// every SPSC ring in production audio).
#[derive(Debug)]
pub struct SpscRing {
    channels: usize,
    mask: usize,
    slots: Box<[AtomicU32]>,
    write: AtomicUsize,
    read: AtomicUsize,
}

impl SpscRing {
    /// Build a ring of `buffer_frames` frames (must be a power of
    /// two) × `channels` (must be ≥ 1).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.ring_shape_invalid`: zero channels, zero or non-power-
    /// of-two frame capacity, or a byte size that overflows `usize`.
    pub fn new(channels: u32, buffer_frames: usize) -> Result<SpscRing, EngineError> {
        if channels == 0 || buffer_frames == 0 || !buffer_frames.is_power_of_two() {
            return Err(error::unsupported("audio.ring_shape_invalid"));
        }
        let slots = buffer_frames
            .checked_mul(channels as usize)
            .ok_or_else(|| error::unsupported("audio.ring_shape_invalid"))?;
        let mut storage = Vec::with_capacity(slots);
        storage.resize_with(slots, || AtomicU32::new(0));
        Ok(SpscRing {
            channels: channels as usize,
            mask: buffer_frames - 1,
            slots: storage.into_boxed_slice(),
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        })
    }

    /// The channel count each frame carries.
    pub fn channels(&self) -> u32 {
        self.channels as u32
    }

    /// The frame capacity (power of two by construction).
    pub fn capacity_frames(&self) -> usize {
        self.mask + 1
    }

    /// Frames currently readable (`write - read`; producer and
    /// consumer both may call — a monotonic-cursor snapshot).
    pub fn available_frames(&self) -> usize {
        self.write.load(Ordering::Acquire) - self.read.load(Ordering::Acquire)
    }

    /// Frames currently free for writing.
    pub fn free_frames(&self) -> usize {
        self.capacity_frames() - self.available_frames()
    }

    /// **Producer side.** Push whole frames from interleaved `data`
    /// (`data.len()` must be a multiple of `channels`), at most
    /// `free_frames()`. Returns the frames actually written (0 when
    /// full — the pump's backpressure signal; never blocks).
    pub fn push(&self, data: &[f32]) -> usize {
        let channels = self.channels;
        if !data.len().is_multiple_of(channels) {
            // A whole-frames contract violation: refuse rather than
            // tear a frame. (Callers validate and type this; the ring
            // itself is the last line of defense.)
            return 0;
        }
        let wanted = data.len() / channels;
        let free = self.free_frames();
        let count = wanted.min(free);
        let mut write = self.write.load(Ordering::Relaxed);
        let mut offset = 0;
        let mut remaining = count;
        while remaining > 0 {
            // One contiguous segment: from the write cursor to the
            // next wrap boundary.
            let start = write & self.mask;
            let run = remaining.min(self.mask + 1 - start);
            for f in 0..run {
                let base = ((start + f) & self.mask) * channels;
                for c in 0..channels {
                    let bits = data[offset + f * channels + c].to_bits();
                    self.slots[base + c].store(bits, Ordering::Relaxed);
                }
            }
            offset += run * channels;
            write += run;
            remaining -= run;
        }
        if count > 0 {
            self.write.store(write, Ordering::Release);
        }
        count
    }

    /// **Consumer side.** Read whole frames into interleaved `out`
    /// (`out.len()` must be a multiple of `channels`), at most
    /// `min(available, out frames)`. Returns the move + segment
    /// count. Never blocks; a short read is the underrun signal the
    /// renderer answers with silence.
    pub fn read(&self, out: &mut [f32]) -> RingRead {
        let channels = self.channels;
        if !out.len().is_multiple_of(channels) {
            // Same whole-frames contract as push: read nothing rather
            // than tear a frame; the driver validates and reports.
            return RingRead::default();
        }
        let wanted = out.len() / channels;
        let available = self.available_frames();
        let count = wanted.min(available);
        let mut read = self.read.load(Ordering::Relaxed);
        let mut offset = 0;
        let mut remaining = count;
        let mut segments = 0;
        while remaining > 0 {
            let start = read & self.mask;
            let run = remaining.min(self.mask + 1 - start);
            segments += 1;
            for f in 0..run {
                let base = ((start + f) & self.mask) * channels;
                for c in 0..channels {
                    let bits = self.slots[base + c].load(Ordering::Relaxed);
                    out[offset + f * channels + c] = f32::from_bits(bits);
                }
            }
            offset += run * channels;
            read += run;
            remaining -= run;
        }
        if count > 0 {
            self.read.store(read, Ordering::Release);
        }
        RingRead {
            frames: count,
            segments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn shape_validation_is_typed() {
        let err = SpscRing::new(0, 8).err().must();
        assert_eq!(err.context.code, "audio.ring_shape_invalid");
        let err = SpscRing::new(2, 0).err().must();
        assert_eq!(err.context.code, "audio.ring_shape_invalid");
        let err = SpscRing::new(2, 7).err().must();
        assert_eq!(err.context.code, "audio.ring_shape_invalid");
        assert!(SpscRing::new(2, 8).is_ok());
    }

    #[test]
    fn push_read_roundtrip_in_fifo_order() {
        let ring = SpscRing::new(2, 8).must();
        let data: Vec<f32> = (0..16).map(|i| i as f32 * 0.25 - 2.0).collect();
        assert_eq!(ring.push(&data), 8);
        assert_eq!(ring.free_frames(), 0);
        let mut out = vec![f32::NAN; 16];
        let read = ring.read(&mut out);
        assert_eq!(read.frames, 8);
        assert_eq!(read.segments, 1, "one contiguous run under capacity");
        assert_eq!(out, data);
        assert_eq!(ring.available_frames(), 0);
    }

    #[test]
    fn full_ring_refuses_without_blocking_or_tearing() {
        let ring = SpscRing::new(1, 4).must();
        assert_eq!(ring.push(&[1.0, 2.0, 3.0, 4.0]), 4);
        // Full: further pushes write nothing.
        assert_eq!(ring.push(&[9.0]), 0);
        // Drain two, push two more (wrap exercised).
        let mut out = [0.0f32; 2];
        assert_eq!(ring.read(&mut out).frames, 2);
        assert_eq!(out, [1.0, 2.0]);
        assert_eq!(ring.push(&[5.0, 6.0]), 2);
        let mut out = [0.0f32; 6];
        let read = ring.read(&mut out);
        assert_eq!(read.frames, 4);
        // 3..4 from the tail, then 5..6 across the wrap: two segments.
        assert_eq!(read.segments, 2, "wrap boundary splits the run");
        assert_eq!(out[..4], [3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn short_read_reports_what_was_available() {
        let ring = SpscRing::new(2, 8).must();
        ring.push(&[1.0, 2.0, 3.0, 4.0]); // 2 frames
        let mut out = vec![f32::NAN; 16]; // wants 8 frames
        let read = ring.read(&mut out);
        assert_eq!(read.frames, 2);
        assert_eq!(read.segments, 1);
        assert_eq!(&out[..4], &[1.0, 2.0, 3.0, 4.0]);
        // The untouched tail is the renderer's silence to fill.
        assert!(out[4..].iter().all(|v| v.is_nan()));
    }

    #[test]
    fn unaligned_buffers_are_refused_whole() {
        let ring = SpscRing::new(2, 8).must();
        assert_eq!(
            ring.push(&[1.0, 2.0, 3.0]),
            0,
            "odd sample count tears a frame"
        );
        let mut out = [0.0f32; 3];
        assert_eq!(ring.read(&mut out).frames, 0);
    }

    #[test]
    fn long_stream_wrap_conservation() {
        // Many wrap cycles: every frame pushed is read back exactly
        // once, in order — the conservation property the clock's
        // sample counter depends on. The run interleaves 6-frame
        // pushes with 2-frame reads, so the ring saturates after a
        // few iterations and every push is partial (the backpressure
        // shape): conservation is `pushed == read + drained +
        // still-buffered`, closed exactly by the final drain.
        let ring = SpscRing::new(2, 8).must();
        let mut pushed = 0usize;
        let mut read_out = 0usize;
        let mut expected = Vec::new();
        let mut value = 0u32;
        for _ in 0..100 {
            let mut chunk = Vec::new();
            for _ in 0..6 {
                // 6 frames per iteration: capacity 8 forces wraps.
                let l = f32::from_bits(value);
                value = value.wrapping_add(1);
                let r = f32::from_bits(value);
                value = value.wrapping_add(1);
                chunk.push(l);
                chunk.push(r);
            }
            let wrote = ring.push(&chunk);
            pushed += wrote;
            expected.extend_from_slice(&chunk[..wrote * 2]);
            let mut out = vec![0.0f32; 4];
            let got = ring.read(&mut out);
            read_out += got.frames;
            assert_eq!(
                &out[..got.frames * 2],
                &expected[read_out * 2 - got.frames * 2..read_out * 2]
            );
        }
        // Drain the remainder: everything pushed comes back exactly
        // once, in order, and the accounts close to the frame.
        let mut drained = Vec::new();
        while ring.available_frames() > 0 {
            let mut out = vec![0.0f32; 8];
            let got = ring.read(&mut out);
            drained.extend_from_slice(&out[..got.frames * 2]);
        }
        assert_eq!(drained, expected[read_out * 2..], "FIFO order to the end");
        assert_eq!(
            pushed,
            read_out + drained.len() / 2,
            "conservation: pushed == read + drained"
        );
        assert_eq!(ring.available_frames(), 0);
    }
}
