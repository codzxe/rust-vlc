//! The AudioWorklet sink model (card 7.H3, doc 09 §5.1) — the
//! arithmetic twin of the hand-written JS processor
//! (`apps/web/audio-worklet.js`).
//!
//! Doc 09 §5.1 maps the native traits to the browser: `AudioSink` →
//! a host-owned `AudioWorkletNode` + `AudioContext`;
//! `RenderCallback::render(out)` → the processor's
//! `process(inputs, outputs)` on the browser's real-time audio
//! thread; `sample_counter()` → the processor's played-sample
//! counter. The worklet module itself is "a tiny, fixed,
//! hand-audited JS shim (ring read, fill, silence-on-underrun,
//! counter bump — a direct transcription of doc 03 §4.9(c)'s allowed
//! operations); no engine code runs inside it".
//!
//! This module is that transcription's **spec**: the same constants
//! ([`WORKLET_QUANTUM`], the bounded queue), the same fill and
//! counter semantics, proven headless-native. The JS file transcribes
//! it; when either changes, both change (the file headers say so).
//!
//! # The single-threaded-mode transport (v1)
//!
//! No `SharedArrayBuffer` in v1 (`threading: single`): chunks cross
//! as transferred `Float32Array` messages routed into the node's
//! port, and the shim keeps a **bounded queue** — the doc's
//! "bounded prebuffer … reports underruns identically" rule. The
//! queue bound refuses pushes **typed** ([`QueueFull`]): audio is
//! never silently dropped (a push past the bound is the page's
//! routing bug, and it must be visible, not a silent gap).
//!
//! # The counter semantics (the clock's truth)
//!
//! The played counter counts **rendered instants** — filled AND
//! silent (silence-on-underrun still bumps the counter: the device
//! clock keeps running through an underrun, exactly like the native
//! null device and doc 03 §4.9(c)'s counter rule). Pause freezes it
//! (a suspended context renders nothing — the autoplay gate's twin
//! rule in [`crate::audio::autoplay`]).
//!
//! # Evidence labels
//!
//! The arithmetic here is proven by this crate's native tests
//! (**simulation** — no browser, no real audio device in this
//! sandbox). The processor's browser behavior (the real-time thread,
//! the browser's watchdog, `AudioWorkletNode` port routing) is
//! `[VERIFY]` — card 7.I1's matrix owns those rows.

use std::collections::VecDeque;

/// The WebAudio render quantum (frames per `process()` call) — fixed
/// by the platform: 128. Both halves (this model and the JS
/// processor) are pinned to it.
pub const WORKLET_QUANTUM: usize = 128;

/// The default queue bound (frames): absorbs the v1 ship granularity
/// (the pump's 1470-frame slices) with headroom while staying a
/// bounded ~170 ms at 48 kHz. `[PROPOSAL — number; doc 09 §5.1's
/// prebuffer target is separately ~2× the quantum, see
/// [`WorkletQueue::PREBUFFER_TARGET`].]`
pub const DEFAULT_QUEUE_FRAMES: usize = 8_192;

/// One `process()` call's outcome (the report the shim posts back
/// through the port — the model's twin carries it as data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessReport {
    /// Frames taken from the queue.
    pub filled: usize,
    /// Frames of silence rendered (the underrun's extent).
    pub silent: usize,
}

impl ProcessReport {
    /// Whether this quantum underran (any silence rendered).
    #[must_use]
    pub fn underran(&self) -> bool {
        self.silent > 0
    }
}

/// The typed queue-full refusal (a push past the bound — the page's
/// routing bug, visible, never a silent drop).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull {
    /// The frames the queue held.
    pub queued: usize,
    /// The frames the refused chunk carries.
    pub chunk: usize,
    /// The bound.
    pub capacity: usize,
}

impl std::fmt::Display for QueueFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the worklet queue is full ({} queued + {} chunk > {} bound)",
            self.queued, self.chunk, self.capacity
        )
    }
}

/// The processor's queue + counter state (the shim's model).
///
/// Interleaved f32 frames, `channels` wide. `process` never
/// allocates (the `VecDeque` pops are O(1); D-1's hot-path rule for
/// what is, on the browser, the real-time audio thread).
#[derive(Debug)]
pub struct WorkletQueue {
    channels: usize,
    capacity_frames: usize,
    queue: VecDeque<f32>,
    played: u64,
    underruns: u64,
    silent_frames: u64,
    overflowed: u64,
}

impl WorkletQueue {
    /// The prebuffer target (frames below which the producer should
    /// ship more): doc 09 §5.1's "~2× the worklet quantum
    /// `[PROPOSAL — number]`".
    pub const PREBUFFER_TARGET: usize = 2 * WORKLET_QUANTUM;

    /// A queue for `channels`-wide interleaved audio with the
    /// [`DEFAULT_QUEUE_FRAMES`] bound.
    ///
    /// # Errors
    /// [`QueueFull`]-free construction refuses a zero channel count
    /// by contract instead: a zero-channel queue is useless but
    /// harmless (every push is a zero-frame push, every process is
    /// pure silence) — construction therefore cannot fail, and the
    /// channel count is validated by the caller that knows the
    /// geometry.
    #[must_use]
    pub fn new(channels: usize, capacity_frames: usize) -> WorkletQueue {
        WorkletQueue {
            channels: channels.max(1),
            capacity_frames: capacity_frames.max(WORKLET_QUANTUM),
            queue: VecDeque::new(),
            played: 0,
            underruns: 0,
            silent_frames: 0,
            overflowed: 0,
        }
    }

    /// The channel count.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// The queue bound (frames).
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity_frames
    }

    /// Frames the queue holds (whole frames; a partial trailing
    /// frame is impossible — pushes validate).
    #[must_use]
    pub fn queued_frames(&self) -> usize {
        self.queue.len() / self.channels
    }

    /// Whether the queue is below the prebuffer target (the
    /// producer's ship-more signal).
    #[must_use]
    pub fn needs_frames(&self) -> bool {
        self.queued_frames() < Self::PREBUFFER_TARGET
    }

    /// The played-sample counter (rendered instants — filled and
    /// silent alike; the clock's truth source).
    #[must_use]
    pub fn played(&self) -> u64 {
        self.played
    }

    /// Quanta that rendered any silence (the underrun count).
    #[must_use]
    pub fn underruns(&self) -> u64 {
        self.underruns
    }

    /// Silent frames rendered so far (the underrun extent).
    #[must_use]
    pub fn silent_frames(&self) -> u64 {
        self.silent_frames
    }

    /// Pushes refused by the bound (the routing-bug evidence).
    #[must_use]
    pub fn overflowed_pushes(&self) -> u64 {
        self.overflowed
    }

    /// Append one interleaved chunk. A partial trailing frame is
    /// refused whole (never a torn frame into the queue).
    ///
    /// # Errors
    /// [`QueueFull`] when the chunk would cross the bound — the
    /// whole chunk is refused (the page retries after the queue
    /// drains; audio is never silently dropped).
    pub fn push(&mut self, chunk: &[f32]) -> Result<(), QueueFull> {
        if !chunk.len().is_multiple_of(self.channels) {
            // A torn chunk is a routing bug: count it as an overflow
            // refusal (the typed path) rather than inventing a
            // partial-frame semantic the real port never produces.
            self.overflowed = self.overflowed.saturating_add(1);
            return Err(QueueFull {
                queued: self.queued_frames(),
                chunk: chunk.len() / self.channels.max(1),
                capacity: self.capacity_frames,
            });
        }
        let frames = chunk.len() / self.channels;
        if self.queued_frames() + frames > self.capacity_frames {
            self.overflowed = self.overflowed.saturating_add(1);
            return Err(QueueFull {
                queued: self.queued_frames(),
                chunk: frames,
                capacity: self.capacity_frames,
            });
        }
        self.queue.extend(chunk);
        Ok(())
    }

    /// One `process()` call: fill `out` (one quantum,
    /// `WORKLET_QUANTUM × channels` interleaved samples) from the
    /// queue, silence for whatever the queue cannot supply, and
    /// bump the counter by the FULL quantum either way (the device
    /// clock runs through underruns).
    ///
    /// # Panics
    /// Never in production shapes: `out` shorter than one quantum
    /// processes the prefix it was given (the model stays total);
    /// the real shim always receives exactly one quantum.
    pub fn process(&mut self, out: &mut [f32]) -> ProcessReport {
        let wanted = out.len() / self.channels.max(1);
        let mut filled = 0;
        for sample in out.iter_mut() {
            match self.queue.pop_front() {
                Some(value) => {
                    *sample = value;
                    filled += 1;
                }
                None => {
                    *sample = 0.0;
                }
            }
        }
        let filled_frames = filled / self.channels.max(1);
        let silent_frames = wanted - filled_frames;
        let report = ProcessReport {
            filled: filled_frames,
            silent: silent_frames,
        };
        if report.underran() {
            self.underruns = self.underruns.saturating_add(1);
            self.silent_frames = self.silent_frames.saturating_add(silent_frames as u64);
        }
        // The counter bump: rendered instants, silence included.
        self.played = self.played.saturating_add(wanted as u64);
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    /// A 10-second playback simulation (native-cfg, real arithmetic,
    /// no browser): 48 kHz stereo, 375 quanta per second, every
    /// quantum filled — the counter lands exactly on 480 000
    /// instants per simulated second (u64 arithmetic, no drift by
    /// construction; the wallclock comparison is [`crate::audio::clock`]'s
    /// simulation).
    #[test]
    fn ten_second_filled_playback_simulation_counter_is_exact() {
        let channels = 2;
        let mut queue = WorkletQueue::new(channels, DEFAULT_QUEUE_FRAMES);
        // Ship 1470-frame chunks (the pump's slice unit) as they fit.
        let ship: Vec<f32> = vec![0.5; 1_470 * channels];
        let mut out = vec![0.0_f32; WORKLET_QUANTUM * channels];
        let mut quanta = 0_u64;
        for _ in 0..10 {
            for _ in 0..375 {
                if queue.needs_frames() || queue.queued_frames() < WORKLET_QUANTUM {
                    queue.push(&ship).must();
                }
                let report = queue.process(&mut out);
                assert!(!report.underran(), "no underrun in the filled lane");
                quanta += 1;
            }
        }
        assert_eq!(quanta, 3_750);
        // 3 750 quanta × 128 frames = 480 000 instants = exactly ten
        // seconds at 48 000 Hz (48 000 × 10).
        assert_eq!(queue.played(), 480_000, "exactly ten seconds");
        assert_eq!(queue.underruns(), 0);
        assert_eq!(queue.silent_frames(), 0);
        assert!(queue.queued_frames() <= DEFAULT_QUEUE_FRAMES);
    }

    /// The underrun discipline (simulation): an empty queue still
    /// renders whole quanta of silence, bumps the counter by the
    /// full quantum, and counts one underrun per silent quantum —
    /// the clock never stalls (audio-master with silence, the native
    /// null-device semantics).
    #[test]
    fn underruns_render_silence_and_keep_the_counter_running() {
        let channels = 2;
        let mut queue = WorkletQueue::new(channels, DEFAULT_QUEUE_FRAMES);
        let mut out = vec![0.25_f32; WORKLET_QUANTUM * channels];
        for _ in 0..5 {
            let report = queue.process(&mut out);
            assert!(report.underran());
            assert_eq!(report.filled, 0);
            assert_eq!(report.silent, WORKLET_QUANTUM);
            assert!(out.iter().all(|&sample| sample == 0.0), "silence");
        }
        assert_eq!(queue.played(), 5 * WORKLET_QUANTUM as u64);
        assert_eq!(queue.underruns(), 5);
        assert_eq!(queue.silent_frames(), 5 * WORKLET_QUANTUM as u64);
        // A partial refill: the quantum fills what it can, silences
        // the rest — ONE underrun (any silence in the quantum), the
        // counter still bumps by the full quantum.
        let half: Vec<f32> = vec![0.5; (WORKLET_QUANTUM / 2) * channels];
        queue.push(&half).must();
        let report = queue.process(&mut out);
        assert_eq!(report.filled, WORKLET_QUANTUM / 2);
        assert_eq!(report.silent, WORKLET_QUANTUM / 2);
        assert_eq!(queue.played(), 6 * WORKLET_QUANTUM as u64);
        assert_eq!(queue.underruns(), 6);
    }

    /// The bounded queue refuses whole chunks typed past the bound —
    /// never a silent drop — and counts the refusals.
    #[test]
    fn the_bound_refuses_typed_and_counts() {
        let channels = 2;
        let mut queue = WorkletQueue::new(channels, 2 * WORKLET_QUANTUM);
        let chunk: Vec<f32> = vec![0.5; WORKLET_QUANTUM * channels];
        queue.push(&chunk).must();
        queue.push(&chunk).must();
        // Full: the third push is refused whole.
        let refusal = assert_err(queue.push(&chunk), "full");
        assert_eq!(refusal.queued, 2 * WORKLET_QUANTUM);
        assert_eq!(refusal.chunk, WORKLET_QUANTUM);
        assert_eq!(refusal.capacity, 2 * WORKLET_QUANTUM);
        assert_eq!(queue.overflowed_pushes(), 1);
        // The queue is intact: exactly what was accepted is held.
        assert_eq!(queue.queued_frames(), 2 * WORKLET_QUANTUM);
        // A torn chunk is refused too (never a torn frame in).
        let torn: Vec<f32> = vec![0.5; 3];
        assert_err(queue.push(&torn), "torn");
        assert_eq!(queue.overflowed_pushes(), 2);
        assert_eq!(queue.queued_frames(), 2 * WORKLET_QUANTUM);
    }

    /// The prebuffer target: below 2× the quantum the producer ships
    /// more (doc 09 §5.1's `[PROPOSAL]` number, pinned as data).
    #[test]
    fn the_prebuffer_target_is_two_quanta() {
        assert_eq!(WorkletQueue::PREBUFFER_TARGET, 256);
        let mut queue = WorkletQueue::new(1, DEFAULT_QUEUE_FRAMES);
        assert!(queue.needs_frames());
        let chunk: Vec<f32> = vec![0.5; 255];
        queue.push(&chunk).must();
        assert!(queue.needs_frames(), "255 < 256");
        queue.push(&[0.5]).must();
        assert!(!queue.needs_frames(), "256 is at target");
    }

    /// The constants this model and the JS processor share (the
    /// both-halves-change-together pin).
    #[test]
    fn the_shared_constants() {
        assert_eq!(WORKLET_QUANTUM, 128);
        assert_eq!(DEFAULT_QUEUE_FRAMES, 8_192);
        assert_eq!(WorkletQueue::new(0, 0).channels(), 1, "zero clamps");
        assert_eq!(WorkletQueue::new(2, 1).capacity(), WORKLET_QUANTUM);
    }
}
