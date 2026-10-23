//! Deterministic fault injection for recovery testing (doc 03 §8, card
//! 3.D2) — the driver behind the MP §13 scenario rows "corrupt / missing
//! files", "interrupted network" and "device loss", and the kit half of
//! cards 3.C2 (media-level recovery) and 3.C3 (environment recovery).
//!
//! Three injectors, one discipline: **everything is scripted, nothing is
//! random.** No RNG anywhere — a fault fires at an exact packet offset or
//! an exact virtual-time stamp, so a failing recovery test replays
//! byte-for-byte and shrinks to the scripted line that caused it.
//!
//! * [`FlakySource`] wraps any [`PacketSource`] (synthetic or a real
//!   demuxer adapter) and applies a [`Fault`] script at exact packet
//!   offsets: drop, delay by virtual ticks, corrupt payload bytes
//!   (flip / zero), cut a payload short mid-stream (the "truncate"
//!   corruption mode, no EOF), clean-EOF truncation at a packet
//!   boundary and mid-packet truncation (dirty EOF).
//! * [`DeviceLossSimulator`] emits `Lost` / `Recovered` signals for the
//!   audio device and the renderer surface on a virtual-time schedule
//!   (hot-unplug, surface loss).
//! * [`ClockJumpInjector`] wraps any [`VirtualClock`] and warps its
//!   readout by scripted deltas at scripted raw times — suspend/resume
//!   gaps, NTP-style corrections, buggy timers.
//!
//! # Composition contract with 3.D1 (synthetic sources)
//!
//! The flaky wrapper is generic over the *consumer's* packet type: the
//! source seam is [`PacketSource`], the payload seam is
//! [`PacketPayload`]. Card 3.D1's sources (and any real demuxer adapter)
//! join with one `impl PacketSource for …` block each, without this
//! module knowing their packet shape. [handoff: those impls land with
//! 3.D1's own files; deliberately not written here — ownership]
//!
//! # D-1: the per-packet check is the hot path
//!
//! [`FlakySource::poll`] is allocation-free per packet by construction:
//! the fault lookup is a bounded cursor walk over the pre-sorted script
//! (integer compares only, O(1) amortized, no collection growth),
//! corruption is
//! an in-place byte write, a delay parks the packet by moving it (no
//! clone), a drop is a plain drop, and the outcome enum is returned by
//! value. No `format!`, no boxing, no closure capture crosses the path.
//! Construction-time work (sorting the script, copying schedules) is
//! *not* hot path and allocates freely. The measured-allocator proof of
//! this belongs in a dedicated integration binary — the house pattern of
//! `tests/alloc_count_e2e.rs` (the global allocator is process-global;
//! the lib unit-test binary cannot be exclusively serialized against the
//! crate's other unit tests). [PROPOSAL: add
//!  `crates/testkit/tests/faults_zero_alloc.rs` post-merge — Lead /
//!  W5-03 decision, ~20 lines on `CountingGlobalAlloc` +
//!  `serialize_tests`; not addable under this card's file ownership]
//!
//! No `unsafe`, no dependencies (the crate is std-only by design).

use crate::VirtualClock;

// ---------------------------------------------------------------------------
// Source and payload seams (composition with 3.D1 sources)
// ---------------------------------------------------------------------------

/// A pull-mode packet source: synthetic (3.D1) or a real demuxer adapter.
pub trait PacketSource {
    /// The packet type this source yields.
    type Packet: PacketPayload;

    /// Pull the next packet, or `None` when the source is exhausted.
    fn next_packet(&mut self) -> Option<Self::Packet>;
}

/// The payload seam: how fault injection reaches packet bytes.
///
/// Deliberately minimal — exactly the two operations the [`Fault`]
/// script needs; nothing an implementor must write goes unused.
pub trait PacketPayload {
    /// Corrupt the payload byte at `offset` per `how`. Returns `false` —
    /// without panicking — when `offset` is out of bounds: the fault is
    /// a tolerated no-op (D-3 dirty-world: scripts may target short
    /// packets; a panic here would turn a script typo into an abort).
    fn corrupt_byte(&mut self, offset: usize, how: PayloadCorrupt) -> bool;

    /// Keep only the first `keep` payload bytes, returning whether the
    /// payload actually shrank. A `keep` at or beyond the length is a
    /// tolerated no-op that changes nothing — the cut models a stream
    /// that ends at a byte this packet never reached.
    fn truncate_payload(&mut self, keep: usize) -> bool;
}

/// How [`Fault::Corrupt`] rewrites the target byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadCorrupt {
    /// XOR the byte with `0xFF` — a bit flip (the corpus "flip" op).
    FlipBits,
    /// Write zero — a clobbered byte.
    Zero,
}

// ---------------------------------------------------------------------------
// The fault script
// ---------------------------------------------------------------------------

/// One scripted fault, aimed at an exact **source packet offset**: the
/// N-th packet the inner source yields, 0-based, counted *before* any
/// fault drops or delivers it (so offsets stay stable no matter what the
/// rest of the script does).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Pull packet `at` from the source and discard it — the consumer
    /// never sees it (a lost network packet, a demux skip). `Drop` wins
    /// over `Delay` / `Corrupt` / `Cut` scripted for the same offset
    /// (the packet is gone, so they are moot); a same-offset clean
    /// [`Truncate`](Fault::Truncate) outranks `Drop` — the stream ends
    /// before that packet is read at all.
    Drop { at: u64 },
    /// Hold packet `at` back for `ticks` virtual ticks, counted from the
    /// moment it is pulled: polls return
    /// [`Pending`](SourcePoll::Pending) naming the exact wake time until
    /// then. FIFO is preserved — nothing overtakes a parked packet.
    /// `ticks: 0` is a documented no-op (no Pending round-trip).
    Delay { at: u64, ticks: u64 },
    /// Corrupt the payload byte at `byte` of packet `at` per `how`. An
    /// out-of-bounds `byte` is a tolerated no-op, not counted by
    /// [`applied_faults`](FlakySource::applied_faults).
    Corrupt {
        at: u64,
        byte: usize,
        how: PayloadCorrupt,
    },
    /// Cut packet `at`'s payload to its first `keep` bytes and keep
    /// streaming — the card's "truncate" corruption mode *without* an
    /// EOF: a torn packet from a corrupted length field, not an ended
    /// stream (that is [`TruncateMid`](Fault::TruncateMid)). A `keep`
    /// at or beyond the payload length leaves the packet whole and is
    /// not counted as applied.
    Cut { at: u64, keep: usize },
    /// End the stream cleanly at the boundary *before* packet `at`:
    /// packets `0..at` are delivered, then `Ended { dirty: false }`,
    /// sticky. Packet `at` is **never read from the source** (a file
    /// truncated on a packet boundary stops reading at the boundary)
    /// and outranks every other fault at the same offset — they are
    /// consumed as moot. Models a well-formed, just shorter, stream.
    Truncate { at: u64 },
    /// Cut packet `at`'s payload to its first `byte` bytes, deliver the
    /// partial packet, then end dirty: `Ended { dirty: true }`. The cut
    /// is best-effort (`byte` beyond the payload leaves it whole) but
    /// the dirty EOF lands regardless — the stream was torn mid-packet.
    /// Beats a same-offset `Truncate` only when scripted first (the
    /// first terminal fault in script order wins).
    TruncateMid { at: u64, byte: usize },
}

impl Fault {
    /// The source packet offset this fault aims at.
    #[must_use = "a script offset nobody aims is a script bug"]
    pub fn at(&self) -> u64 {
        match self {
            Fault::Drop { at }
            | Fault::Delay { at, .. }
            | Fault::Corrupt { at, .. }
            | Fault::Cut { at, .. }
            | Fault::Truncate { at }
            | Fault::TruncateMid { at, .. } => *at,
        }
    }
}

/// The outcome of one [`FlakySource::poll`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcePoll<P> {
    /// A packet — possibly corrupted or truncated by the script.
    Packet(P),
    /// A scripted delay holds the next packet; retry at (or after)
    /// `ready_at_ns` of virtual time. Only a [`Fault::Delay`] produces
    /// this.
    Pending { ready_at_ns: u64 },
    /// End of stream — sticky, every later poll repeats it. `dirty`
    /// marks a mid-packet end (scripted or, one day, real).
    Ended { dirty: bool },
}

// ---------------------------------------------------------------------------
// 1. The flaky source wrapper
// ---------------------------------------------------------------------------

/// A source wrapper that applies a [`Fault`] script at exact packet
/// offsets — the "interrupted network" / damaged-media driver of doc 03
/// §8, scripted so recovery tests are deterministic.
///
/// Offsets are indices into the *source* stream (see [`Fault`]). The
/// script is sorted by offset at construction (stable: same-offset
/// faults keep script order); each fault is consumed at most once.
pub struct FlakySource<S: PacketSource> {
    inner: S,
    script: Vec<Fault>,
    cursor: usize,
    tick_ns: u64,
    next_index: u64,
    pending: Option<(S::Packet, u64)>,
    pending_dirty: bool,
    ended: Option<bool>,
    applied: u64,
}

impl<S: PacketSource> FlakySource<S> {
    /// Wrap `inner` with a fault script. `tick_ns` is the nanosecond
    /// weight of one *virtual tick* for [`Fault::Delay`] — the unit the
    /// script is written in, kept explicit so tests read exactly.
    #[must_use = "a wrapper nobody polls proves nothing"]
    pub fn new(inner: S, script: Vec<Fault>, tick_ns: u64) -> FlakySource<S> {
        let mut script = script;
        script.sort_by_key(|fault| fault.at());
        FlakySource {
            inner,
            script,
            cursor: 0,
            tick_ns,
            next_index: 0,
            pending: None,
            pending_dirty: false,
            ended: None,
            applied: 0,
        }
    }

    /// Poll one outcome at virtual time `now_ns`.
    ///
    /// Deterministic ordering rules: a clean `Truncate` aimed at the
    /// next unpulled packet ends the stream at the boundary before it —
    /// the packet is never read, and same-offset faults are consumed as
    /// moot. Otherwise the packet is pulled and its faults apply in
    /// script order: `Drop` discards it (later same-offset faults are
    /// consumed as no-ops); `Delay` parks it (FIFO — nothing passes a
    /// parked packet); `Corrupt` rewrites a byte; `Cut` shortens the
    /// payload. Between the terminal faults, the first in script order
    /// at an offset wins: `TruncateMid` delivers a partial packet then
    /// ends dirty, and a `Truncate` behind it is moot. A pull loop that
    /// only drops is bounded by the finite script (O(1) per packet,
    /// D-2).
    pub fn poll(&mut self, now_ns: u64) -> SourcePoll<S::Packet> {
        // Ended is sticky: every poll after the end reports the end.
        if let Some(dirty) = self.ended {
            return SourcePoll::Ended { dirty };
        }
        // A parked (delayed) packet gates the stream (FIFO: nothing
        // passes it), so its ready time is the only answer until due.
        if let Some((_, ready_at)) = self.pending.as_ref()
            && now_ns < *ready_at
        {
            return SourcePoll::Pending {
                ready_at_ns: *ready_at,
            };
        }
        if let Some((packet, _)) = self.pending.take() {
            // The parked packet was mid-truncated by the script: the
            // dirty end becomes visible once it is delivered.
            if self.pending_dirty {
                self.pending_dirty = false;
                self.ended = Some(true);
            }
            return SourcePoll::Packet(packet);
        }
        loop {
            let index = self.next_index;
            // Terminal scan, BEFORE pulling: the first terminal fault in
            // script order among those aimed at offsets <= index decides
            // the ending. A clean Truncate ends the stream at the
            // boundary — packet `at` is never read from the source — and
            // every other fault at that offset is consumed as moot. The
            // scan is a few integer compares on the pre-sorted script:
            // no allocation, no growth (D-1).
            let mut scan = self.cursor;
            let mut clean_cut = false;
            while let Some(fault) = self.script.get(scan).copied()
                && fault.at() <= index
            {
                match fault {
                    Fault::Truncate { .. } => {
                        clean_cut = true;
                        break;
                    }
                    // A mid-packet cut needs its packet: pull below.
                    Fault::TruncateMid { .. } => break,
                    _ => scan += 1,
                }
            }
            if clean_cut {
                while self.cursor < self.script.len() && self.script[self.cursor].at() <= index {
                    self.cursor += 1;
                }
                self.applied += 1;
                self.ended = Some(false);
                return SourcePoll::Ended { dirty: false };
            }
            let Some(mut packet) = self.inner.next_packet() else {
                // The real source ended on its own: a clean EOF.
                self.ended = Some(false);
                return SourcePoll::Ended { dirty: false };
            };
            self.next_index = index.saturating_add(1);
            let mut dropped = false;
            let mut parked_until: Option<u64> = None;
            let mut dirty_after = false;
            // Consume every fault scripted for offsets <= index. The
            // cursor walk is aligned (offsets are visited in order and
            // each pull advances the index by exactly one), so this is
            // an O(1) integer compare on the hot path — allocation-free
            // by construction (D-1; see the module docs).
            while self.cursor < self.script.len() && self.script[self.cursor].at() <= index {
                let fault = self.script[self.cursor];
                self.cursor += 1;
                if dropped {
                    // Consumed, but the packet is gone: a no-op.
                    continue;
                }
                match fault {
                    Fault::Drop { .. } => {
                        dropped = true;
                        self.applied += 1;
                    }
                    Fault::Delay { ticks, .. } => {
                        if ticks > 0 {
                            parked_until =
                                Some(now_ns.saturating_add(ticks.saturating_mul(self.tick_ns)));
                            self.applied += 1;
                        }
                        // ticks == 0: a documented no-op.
                    }
                    Fault::Corrupt { byte, how, .. } => {
                        // Out-of-bounds byte: tolerated no-op, not
                        // counted as applied.
                        if packet.corrupt_byte(byte, how) {
                            self.applied += 1;
                        }
                    }
                    // Moot by construction: a clean Truncate that would
                    // have won was handled by the pre-pull scan, and one
                    // scripted behind a same-offset TruncateMid cannot
                    // end a mid-cut stream clean.
                    Fault::Truncate { .. } => {}
                    Fault::TruncateMid { byte, .. } => {
                        // The cut is best-effort; the dirty EOF is the
                        // fault, and it lands regardless.
                        packet.truncate_payload(byte);
                        dirty_after = true;
                        self.applied += 1;
                    }
                    Fault::Cut { keep, .. } => {
                        // A no-op cut (keep beyond the payload) is not
                        // counted: nothing observable landed.
                        if packet.truncate_payload(keep) {
                            self.applied += 1;
                        }
                    }
                }
            }
            if dropped {
                if dirty_after {
                    // The partial packet was dropped by script: the cut
                    // still ends the stream dirty.
                    self.ended = Some(true);
                    return SourcePoll::Ended { dirty: true };
                }
                // An invisible skip: keep pulling toward the next
                // deliverable packet (bounded by the finite script).
                continue;
            }
            if let Some(ready_at) = parked_until {
                self.pending = Some((packet, ready_at));
                self.pending_dirty = dirty_after;
                return SourcePoll::Pending {
                    ready_at_ns: ready_at,
                };
            }
            if dirty_after {
                // Deliver the partial packet now; the dirty end is what
                // the *next* poll reports.
                self.ended = Some(true);
            }
            return SourcePoll::Packet(packet);
        }
    }

    /// Faults whose effects landed on a live packet so far — the count a
    /// recovery test asserts to prove the script actually fired.
    #[must_use = "a counter nobody asserts is decoration"]
    pub fn applied_faults(&self) -> u64 {
        self.applied
    }

    /// Whether the stream has ended (cleanly or dirty).
    #[must_use = "observable evidence"]
    pub fn is_ended(&self) -> bool {
        self.ended.is_some()
    }

    /// The next source packet offset [`poll`](Self::poll) will pull —
    /// the source-stream position. Drops advance it too (the packet was
    /// pulled, then discarded), which is exactly what a test wants to
    /// prove a drop happened.
    #[must_use = "observable evidence"]
    pub fn next_offset(&self) -> u64 {
        self.next_index
    }
}

// ---------------------------------------------------------------------------
// 2. The device-loss simulator
// ---------------------------------------------------------------------------

/// Which device a loss/recovery signal concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// The audio output device (hot-unplug — doc 03's
    /// suspend-on-device-loss default).
    Audio,
    /// The renderer surface (window destroyed, GPU surface lost).
    Surface,
}

/// One scripted device transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSignal {
    /// The device went away.
    Lost(DeviceKind),
    /// The device came back.
    Recovered(DeviceKind),
}

/// A device transition fired at its scripted virtual time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceEvent {
    /// The **scheduled** time, not the poll time — a late poll still
    /// reports the schedule's stamp, so exactness stays assertable.
    pub at_ns: u64,
    /// What happened.
    pub signal: DeviceSignal,
}

/// Emits `Lost` / `Recovered` events for the audio device and the
/// renderer surface on a scripted virtual-time schedule — the 3.C3
/// hot-unplug / surface-loss driver. The consumer paces virtual time
/// and polls; each event fires at most once, in schedule order
/// (same-time entries in script order). An unpaired `Recovered` is
/// tolerated (dirty-world: drivers misbehave) — it simply clears state.
pub struct DeviceLossSimulator {
    schedule: Vec<DeviceEvent>,
    cursor: usize,
    audio_lost: bool,
    surface_lost: bool,
    emitted: u64,
}

impl DeviceLossSimulator {
    /// Build from a `(virtual time, signal)` schedule. The schedule is
    /// sorted by time at construction (stable); construction is not hot
    /// path.
    #[must_use = "a simulator nobody polls proves nothing"]
    pub fn new(schedule: &[(u64, DeviceSignal)]) -> DeviceLossSimulator {
        let mut events: Vec<DeviceEvent> = schedule
            .iter()
            .map(|(at_ns, signal)| DeviceEvent {
                at_ns: *at_ns,
                signal: *signal,
            })
            .collect();
        events.sort_by_key(|event| event.at_ns);
        DeviceLossSimulator {
            schedule: events,
            cursor: 0,
            audio_lost: false,
            surface_lost: false,
            emitted: 0,
        }
    }

    /// Fire the next due event (scheduled at or before `now_ns`), if
    /// any. At most one event per call — the consumer drives the clock
    /// and polls until `None`. Allocation-free: a copy out of the owned
    /// schedule, the cursor does the rest (D-1).
    pub fn poll(&mut self, now_ns: u64) -> Option<DeviceEvent> {
        let event = self.schedule.get(self.cursor).copied()?;
        if event.at_ns > now_ns {
            // Not due yet — the cursor stays put.
            return None;
        }
        self.cursor += 1;
        self.emitted += 1;
        match event.signal {
            DeviceSignal::Lost(DeviceKind::Audio) => self.audio_lost = true,
            DeviceSignal::Lost(DeviceKind::Surface) => self.surface_lost = true,
            DeviceSignal::Recovered(DeviceKind::Audio) => self.audio_lost = false,
            DeviceSignal::Recovered(DeviceKind::Surface) => self.surface_lost = false,
        }
        Some(event)
    }

    /// Whether `device` is currently lost, as of the events **emitted so
    /// far** — state advances only through [`poll`](Self::poll), so the
    /// test paces it; an un-polled due event has not happened yet.
    #[must_use = "observable evidence"]
    pub fn is_lost(&self, device: DeviceKind) -> bool {
        match device {
            DeviceKind::Audio => self.audio_lost,
            DeviceKind::Surface => self.surface_lost,
        }
    }

    /// Events emitted so far.
    #[must_use = "observable evidence"]
    pub fn emitted(&self) -> u64 {
        self.emitted
    }

    /// Scripted events not yet emitted.
    #[must_use = "observable evidence"]
    pub fn remaining(&self) -> usize {
        self.schedule.len() - self.cursor
    }
}

// ---------------------------------------------------------------------------
// 3. The clock-jump injector
// ---------------------------------------------------------------------------

/// One scripted clock warp, armed at a **raw** inner-clock time. The
/// trigger timeline is monotonic (raw time never runs backwards), so
/// every jump fires at most once, in schedule order — even when the
/// warped readout itself runs backwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockJump {
    /// The raw inner time that arms this jump (inclusive).
    pub at_ns: u64,
    /// The warp applied to every subsequent readout: positive = forward
    /// leap (suspend/resume gap), negative = backward step (NTP-style
    /// correction, buggy timer).
    pub delta_ns: i64,
}

impl ClockJump {
    /// A forward jump — the clock leaps ahead (suspend/resume).
    #[must_use = "pure constructor"]
    pub const fn forward(at_ns: u64, delta_ns: u64) -> ClockJump {
        ClockJump {
            at_ns,
            delta_ns: delta_ns as i64,
        }
    }

    /// A backward jump — the clock steps back (correction, buggy timer).
    #[must_use = "pure constructor"]
    pub const fn backward(at_ns: u64, delta_ns: u64) -> ClockJump {
        ClockJump {
            at_ns,
            delta_ns: -(delta_ns as i64),
        }
    }
}

/// A [`VirtualClock`] whose readout is warped by scripted jumps — the
/// clock-realism fault (D-9): engines must survive suspend/resume gaps,
/// stepped corrections and timers that lie, so tests inject exactly
/// those. **This deliberately violates the trait's monotonicity
/// convention when a backward jump wins — that violation is the fault
/// under test.**
///
/// Semantics: jumps arm at raw inner times and fire inside
/// [`advance`](VirtualClock::advance) when the raw clock reaches them
/// (a zero advance still fires jumps armed at the current raw time);
/// between advances the readout is stable and no jump double-fires. The
/// warped readout saturates at 0 — a backward jump never wraps to a
/// huge positive time (D-9 arithmetic honesty).
pub struct ClockJumpInjector<C> {
    inner: C,
    jumps: Vec<ClockJump>,
    cursor: usize,
    offset_ns: i64,
    fired: u64,
}

impl<C: VirtualClock> ClockJumpInjector<C> {
    /// Wrap `inner` with a jump schedule (sorted by `at_ns`, stable —
    /// same-time jumps apply in script order).
    #[must_use = "a clock nobody reads proves nothing"]
    pub fn new(inner: C, jumps: &[ClockJump]) -> ClockJumpInjector<C> {
        let mut jumps = jumps.to_vec();
        jumps.sort_by_key(|jump| jump.at_ns);
        ClockJumpInjector {
            inner,
            jumps,
            cursor: 0,
            offset_ns: 0,
            fired: 0,
        }
    }

    /// The unwarped inner time — the timeline the schedule is written
    /// against.
    #[must_use = "observable evidence"]
    pub fn raw_now_ns(&self) -> u64 {
        self.inner.now_ns()
    }

    /// The cumulative warp applied so far.
    #[must_use = "observable evidence"]
    pub fn offset_ns(&self) -> i64 {
        self.offset_ns
    }

    /// Jumps fired so far.
    #[must_use = "observable evidence"]
    pub fn fired_jumps(&self) -> u64 {
        self.fired
    }

    /// Unwrap into the inner clock.
    #[must_use = "the inner clock is still yours"]
    pub fn into_inner(self) -> C {
        self.inner
    }

    /// Fire every jump armed at or before the current raw time.
    /// Called from `advance` only — time moves only through advances.
    fn fire_due(&mut self) {
        let raw = self.inner.now_ns();
        while self.cursor < self.jumps.len() && self.jumps[self.cursor].at_ns <= raw {
            self.offset_ns = self
                .offset_ns
                .saturating_add(self.jumps[self.cursor].delta_ns);
            self.cursor += 1;
            self.fired += 1;
        }
    }
}

impl<C: VirtualClock> VirtualClock for ClockJumpInjector<C> {
    fn advance(&mut self, by_ns: u64) {
        self.inner.advance(by_ns);
        self.fire_due();
    }

    fn now_ns(&self) -> u64 {
        // i128 intermediate: raw + offset cannot overflow, and the
        // warped readout saturates at 0 instead of wrapping (a backward
        // jump past the session epoch clamps, never lies with a huge
        // positive time).
        let warped = i128::from(self.inner.now_ns()) + i128::from(self.offset_ns);
        if warped <= 0 {
            0
        } else {
            warped.min(u64::MAX as i128) as u64
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — every injector proven at exact offsets / times (card 3.D2
// acceptance). Deterministic by construction; no RNG anywhere.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StepClock, VirtualClock};
    use std::collections::VecDeque;

    /// A deterministic test packet: identity plus an owned payload.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestPacket {
        id: u64,
        data: Vec<u8>,
    }

    impl PacketPayload for TestPacket {
        fn corrupt_byte(&mut self, offset: usize, how: PayloadCorrupt) -> bool {
            let Some(byte) = self.data.get_mut(offset) else {
                return false;
            };
            match how {
                PayloadCorrupt::FlipBits => *byte ^= 0xFF,
                PayloadCorrupt::Zero => *byte = 0,
            }
            true
        }

        fn truncate_payload(&mut self, keep: usize) -> bool {
            if keep >= self.data.len() {
                false
            } else {
                self.data.truncate(keep);
                true
            }
        }
    }

    /// An ordered source over pre-built packets. `pop_front` moves a
    /// packet out without allocating — the shape a zero-alloc
    /// measurement of the wrapper would demand of the source too.
    struct VecSource {
        packets: VecDeque<TestPacket>,
    }

    impl VecSource {
        /// `count` packets of `payload_len` bytes of `0xA5`, ids 0..
        fn with_ids(count: u64, payload_len: usize) -> VecSource {
            VecSource {
                packets: (0..count)
                    .map(|id| TestPacket {
                        id,
                        data: vec![0xA5; payload_len],
                    })
                    .collect(),
            }
        }
    }

    impl PacketSource for VecSource {
        type Packet = TestPacket;

        fn next_packet(&mut self) -> Option<TestPacket> {
            self.packets.pop_front()
        }
    }

    /// Drive a delay-free script to its end at one fixed virtual time,
    /// collecting the delivered packets (moved) and the end's dirtiness.
    fn drain_at(source: &mut FlakySource<VecSource>, now_ns: u64) -> (Vec<TestPacket>, bool) {
        let mut delivered = Vec::new();
        loop {
            match source.poll(now_ns) {
                SourcePoll::Packet(packet) => delivered.push(packet),
                SourcePoll::Pending { .. } => {
                    unreachable!("a fixed-time drain cannot clear a delay")
                }
                SourcePoll::Ended { dirty } => return (delivered, dirty),
            }
        }
    }

    #[test]
    fn drop_removes_exactly_the_scripted_packet() {
        let mut flaky = FlakySource::new(
            VecSource::with_ids(8, 16),
            vec![Fault::Drop { at: 3 }],
            1_000_000,
        );
        let (delivered, dirty) = drain_at(&mut flaky, 0);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        // Packet 3 is gone; everything else flows, untouched.
        assert_eq!(ids, vec![0, 1, 2, 4, 5, 6, 7]);
        assert!(delivered.iter().all(|p| p.data.len() == 16));
        assert!(!dirty);
        assert_eq!(flaky.applied_faults(), 1);
        assert_eq!(flaky.next_offset(), 8);
        // Ended is sticky.
        match flaky.poll(1_000_000) {
            SourcePoll::Ended { dirty: false } => {}
            other => unreachable!("sticky end, got {other:?}"),
        }
    }

    #[test]
    fn delay_parks_exactly_v_ticks_from_pull_time() {
        const TICK: u64 = 1_000_000; // one virtual tick = 1 ms
        let mut flaky = FlakySource::new(
            VecSource::with_ids(6, 8),
            vec![Fault::Delay { at: 2, ticks: 3 }],
            TICK,
        );
        // t = 0: packets 0 and 1 flow...
        match flaky.poll(0) {
            SourcePoll::Packet(p) => assert_eq!(p.id, 0),
            other => unreachable!("{other:?}"),
        }
        match flaky.poll(0) {
            SourcePoll::Packet(p) => assert_eq!(p.id, 1),
            other => unreachable!("{other:?}"),
        }
        // ...the third poll pulls packet 2 and parks it; the pending
        // answer names the exact wake time (pulled at 0 + 3 ticks).
        match flaky.poll(0) {
            SourcePoll::Pending { ready_at_ns } => assert_eq!(ready_at_ns, 3 * TICK),
            other => unreachable!("{other:?}"),
        }
        // One nanosecond early is still pending, same wake time.
        match flaky.poll(3 * TICK - 1) {
            SourcePoll::Pending { ready_at_ns } => assert_eq!(ready_at_ns, 3 * TICK),
            other => unreachable!("{other:?}"),
        }
        // At exactly the wake time the packet flows, FIFO preserved.
        match flaky.poll(3 * TICK) {
            SourcePoll::Packet(p) => assert_eq!(p.id, 2),
            other => unreachable!("{other:?}"),
        }
        let (delivered, dirty) = drain_at(&mut flaky, 3 * TICK);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![3, 4, 5]);
        assert!(!dirty);
        assert_eq!(flaky.applied_faults(), 1);

        // The anchor is the PULL time: a delay first touched at t = 5 ms
        // with 2 ticks wakes at 7 ms, not at 2 ms.
        let mut late = FlakySource::new(
            VecSource::with_ids(3, 8),
            vec![Fault::Delay { at: 1, ticks: 2 }],
            TICK,
        );
        match late.poll(5 * TICK) {
            SourcePoll::Packet(p) => assert_eq!(p.id, 0),
            other => unreachable!("{other:?}"),
        }
        match late.poll(5 * TICK) {
            SourcePoll::Pending { ready_at_ns } => assert_eq!(ready_at_ns, 7 * TICK),
            other => unreachable!("{other:?}"),
        }
        match late.poll(6 * TICK) {
            SourcePoll::Pending { ready_at_ns } => assert_eq!(ready_at_ns, 7 * TICK),
            other => unreachable!("{other:?}"),
        }
        match late.poll(7 * TICK) {
            SourcePoll::Packet(p) => assert_eq!(p.id, 1),
            other => unreachable!("{other:?}"),
        }
    }

    #[test]
    fn corrupt_rewrites_the_exact_offset_byte_only() {
        let mut flaky = FlakySource::new(
            VecSource::with_ids(6, 8),
            vec![
                Fault::Corrupt {
                    at: 1,
                    byte: 2,
                    how: PayloadCorrupt::FlipBits,
                },
                Fault::Corrupt {
                    at: 4,
                    byte: 0,
                    how: PayloadCorrupt::Zero,
                },
                // Out-of-bounds byte: a tolerated no-op, not applied.
                Fault::Corrupt {
                    at: 5,
                    byte: 999,
                    how: PayloadCorrupt::FlipBits,
                },
            ],
            1_000_000,
        );
        let (delivered, _) = drain_at(&mut flaky, 0);
        assert_eq!(delivered.len(), 6);
        // Packet 1: byte 2 flipped 0xA5 -> 0x5A, the rest untouched.
        assert_eq!(
            delivered[1].data,
            [0xA5, 0xA5, 0x5A, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5]
        );
        // Packet 4: byte 0 zeroed only.
        assert_eq!(
            delivered[4].data,
            [0x00, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5]
        );
        // Packet 5: the out-of-bounds corrupt left it whole.
        assert_eq!(delivered[5].data, vec![0xA5; 8]);
        assert_eq!(delivered[0].data, vec![0xA5; 8]);
        assert_eq!(flaky.applied_faults(), 2);
    }

    #[test]
    fn truncate_ends_clean_at_the_packet_boundary() {
        let mut flaky = FlakySource::new(
            VecSource::with_ids(10, 16),
            vec![Fault::Truncate { at: 5 }],
            1_000_000,
        );
        let (delivered, dirty) = drain_at(&mut flaky, 0);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        // Packets 0..=4 delivered; 5 onward never surfaces.
        assert_eq!(ids, vec![0, 1, 2, 3, 4]);
        assert!(!dirty);
        assert_eq!(flaky.applied_faults(), 1);
        assert_eq!(flaky.next_offset(), 5);
        assert!(flaky.is_ended());
    }

    #[test]
    fn truncate_mid_delivers_partial_packet_then_dirty_end() {
        let mut flaky = FlakySource::new(
            VecSource::with_ids(10, 16),
            vec![Fault::TruncateMid { at: 5, byte: 4 }],
            1_000_000,
        );
        let (delivered, dirty) = drain_at(&mut flaky, 0);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![0, 1, 2, 3, 4, 5]);
        // The packet before the cut is whole; the cut packet keeps its
        // FIRST four bytes; the end is dirty and sticky.
        assert_eq!(delivered[4].data.len(), 16);
        assert_eq!(delivered[5].data, vec![0xA5; 4]);
        assert!(dirty);
        assert_eq!(flaky.applied_faults(), 1);
        match flaky.poll(0) {
            SourcePoll::Ended { dirty: true } => {}
            other => unreachable!("{other:?}"),
        }
    }

    #[test]
    fn cut_shortens_payload_and_the_stream_continues() {
        // The card's third corruption mode: a payload cut (a torn packet
        // / corrupted length field) that is NOT an EOF.
        let mut flaky = FlakySource::new(
            VecSource::with_ids(5, 16),
            vec![
                Fault::Cut { at: 2, keep: 4 },
                // keep beyond the payload: a tolerated no-op, not applied.
                Fault::Cut { at: 3, keep: 999 },
            ],
            1_000_000,
        );
        let (delivered, dirty) = drain_at(&mut flaky, 0);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        // All five packets still flow — a cut never ends the stream.
        assert_eq!(ids, vec![0, 1, 2, 3, 4]);
        assert_eq!(delivered[1].data.len(), 16);
        assert_eq!(delivered[2].data, vec![0xA5; 4]); // cut to its first 4
        assert_eq!(delivered[3].data, vec![0xA5; 16]); // no-op cut left whole
        assert!(!dirty);
        assert_eq!(flaky.applied_faults(), 1);
        assert_eq!(flaky.next_offset(), 5);
    }

    #[test]
    fn clean_truncate_never_pulls_and_outranks_same_offset_faults() {
        let mut flaky = FlakySource::new(
            VecSource::with_ids(8, 16),
            vec![
                // Both moot: the boundary wins before the packet exists.
                Fault::Drop { at: 4 },
                Fault::Corrupt {
                    at: 4,
                    byte: 0,
                    how: PayloadCorrupt::Zero,
                },
                Fault::Truncate { at: 4 },
            ],
            1_000_000,
        );
        let (delivered, dirty) = drain_at(&mut flaky, 0);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![0, 1, 2, 3]);
        assert!(!dirty);
        // The aimed-at packet was never read: the pull position stops at
        // the boundary (a Drop would have advanced it past the packet).
        assert_eq!(flaky.next_offset(), 4);
        assert_eq!(flaky.applied_faults(), 1);
    }

    #[test]
    fn dropped_partial_packet_still_ends_dirty() {
        // TruncateMid scripted first wins the terminal slot; a later
        // same-offset Drop discards the partial packet — but the stream
        // was cut mid-packet, so the end is dirty with nothing after.
        let mut flaky = FlakySource::new(
            VecSource::with_ids(6, 16),
            vec![Fault::TruncateMid { at: 2, byte: 8 }, Fault::Drop { at: 2 }],
            1_000_000,
        );
        let (delivered, dirty) = drain_at(&mut flaky, 0);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![0, 1]);
        assert!(dirty);
        assert_eq!(flaky.applied_faults(), 2);
        // Pulled, truncated, dropped — the pull position advanced.
        assert_eq!(flaky.next_offset(), 3);
    }

    #[test]
    fn mixed_script_composes_and_replays_identically() {
        let script = vec![
            Fault::Drop { at: 1 },
            Fault::Corrupt {
                at: 3,
                byte: 0,
                how: PayloadCorrupt::Zero,
            },
            Fault::Delay { at: 5, ticks: 2 },
            Fault::TruncateMid { at: 8, byte: 8 },
        ];
        // (id, payload len, first payload byte) per delivered packet,
        // plus the poll-kind trace and the end's dirtiness.
        type PacketTrace = Vec<(u64, usize, u8)>;
        type PollTrace = Vec<&'static str>;
        fn run(script: &[Fault]) -> (PacketTrace, PollTrace, bool) {
            let mut flaky =
                FlakySource::new(VecSource::with_ids(12, 16), script.to_vec(), 1_000_000);
            let mut packets = Vec::new();
            let mut kinds = Vec::new();
            let mut now = 0u64;
            loop {
                match flaky.poll(now) {
                    SourcePoll::Packet(p) => {
                        packets.push((p.id, p.data.len(), p.data.first().copied().unwrap_or(0)));
                        kinds.push("packet");
                    }
                    SourcePoll::Pending { .. } => {
                        kinds.push("pending");
                        now += 1_000_000; // one tick per pending answer
                    }
                    SourcePoll::Ended { dirty } => {
                        kinds.push("ended");
                        break (packets, kinds, dirty);
                    }
                }
            }
        }
        let first = run(&script);
        let second = run(&script);
        // Determinism: the whole timeline replays identically.
        assert_eq!(first, second);
        let (packets, kinds, dirty) = first;
        // Exact expectation: 1 dropped, 3's first byte zeroed, 5 parked
        // exactly two ticks, 8 delivered as an 8-byte partial, dirty end.
        assert_eq!(
            packets,
            vec![
                (0, 16, 0xA5),
                (2, 16, 0xA5),
                (3, 16, 0x00),
                (4, 16, 0xA5),
                (5, 16, 0xA5),
                (6, 16, 0xA5),
                (7, 16, 0xA5),
                (8, 8, 0xA5),
            ]
        );
        assert_eq!(
            kinds,
            vec![
                "packet", "packet", "packet", "packet", "pending", "pending", "packet", "packet",
                "packet", "packet", "ended",
            ]
        );
        assert!(dirty);
    }

    #[test]
    fn delayed_packet_can_also_be_the_mid_truncated_one() {
        let mut flaky = FlakySource::new(
            VecSource::with_ids(4, 16),
            vec![
                Fault::Delay { at: 1, ticks: 2 },
                Fault::TruncateMid { at: 1, byte: 4 },
            ],
            1_000_000,
        );
        match flaky.poll(0) {
            SourcePoll::Packet(p) => assert_eq!(p.id, 0),
            other => unreachable!("{other:?}"),
        }
        // Pulled at t=0, parked 2 ticks, payload already cut to 4 bytes.
        match flaky.poll(0) {
            SourcePoll::Pending { ready_at_ns } => assert_eq!(ready_at_ns, 2_000_000),
            other => unreachable!("{other:?}"),
        }
        match flaky.poll(2_000_000) {
            SourcePoll::Packet(p) => {
                assert_eq!(p.id, 1);
                assert_eq!(p.data.len(), 4);
            }
            other => unreachable!("{other:?}"),
        }
        // The dirty end surfaces only after the partial packet delivers.
        match flaky.poll(2_000_000) {
            SourcePoll::Ended { dirty: true } => {}
            other => unreachable!("{other:?}"),
        }
        assert_eq!(flaky.applied_faults(), 2);
    }

    #[test]
    fn unsorted_script_is_sorted_at_construction() {
        let mut flaky = FlakySource::new(
            VecSource::with_ids(4, 8),
            vec![
                Fault::Drop { at: 2 },
                Fault::Corrupt {
                    at: 0,
                    byte: 0,
                    how: PayloadCorrupt::FlipBits,
                },
            ],
            1_000_000,
        );
        let (delivered, _) = drain_at(&mut flaky, 0);
        let ids: Vec<u64> = delivered.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![0, 1, 3]);
        assert_eq!(delivered[0].data[0], 0x5A); // 0xA5 flipped
        assert_eq!(flaky.applied_faults(), 2);
    }

    #[test]
    fn edge_scripts_are_tolerated_dirty_world() {
        // Empty source, empty script: immediate clean end.
        let mut flaky = FlakySource::new(VecSource::with_ids(0, 8), vec![], 1_000_000);
        match flaky.poll(0) {
            SourcePoll::Ended { dirty: false } => {}
            other => unreachable!("{other:?}"),
        }
        // Zero-tick delay: a documented no-op — no Pending round-trip.
        let mut flaky = FlakySource::new(
            VecSource::with_ids(2, 8),
            vec![Fault::Delay { at: 0, ticks: 0 }],
            1_000_000,
        );
        match flaky.poll(0) {
            SourcePoll::Packet(p) => assert_eq!(p.id, 0),
            other => unreachable!("{other:?}"),
        }
        assert_eq!(flaky.applied_faults(), 0);
        // Mid-truncate beyond the payload length: the packet stays whole
        // and the end is still dirty (the stream was cut mid-packet).
        let mut flaky = FlakySource::new(
            VecSource::with_ids(2, 4),
            vec![Fault::TruncateMid { at: 0, byte: 999 }],
            1_000_000,
        );
        match flaky.poll(0) {
            SourcePoll::Packet(p) => assert_eq!(p.data.len(), 4),
            other => unreachable!("{other:?}"),
        }
        match flaky.poll(0) {
            SourcePoll::Ended { dirty: true } => {}
            other => unreachable!("{other:?}"),
        }
    }

    #[test]
    fn device_loss_schedule_fires_in_order_at_exact_times() {
        let mut sim = DeviceLossSimulator::new(&[
            (100, DeviceSignal::Lost(DeviceKind::Audio)),
            (250, DeviceSignal::Lost(DeviceKind::Surface)),
            (400, DeviceSignal::Recovered(DeviceKind::Audio)),
            (400, DeviceSignal::Recovered(DeviceKind::Surface)),
        ]);
        assert!(!sim.is_lost(DeviceKind::Audio));
        assert!(!sim.is_lost(DeviceKind::Surface));
        assert_eq!(sim.poll(0), None);
        assert_eq!(sim.poll(99), None);
        // Exact boundary: the event scheduled at t=100 fires at t=100.
        assert_eq!(
            sim.poll(100).map(|e| (e.at_ns, e.signal)),
            Some((100, DeviceSignal::Lost(DeviceKind::Audio)))
        );
        assert!(sim.is_lost(DeviceKind::Audio));
        assert!(!sim.is_lost(DeviceKind::Surface));
        // No double fire at the same instant.
        assert_eq!(sim.poll(100), None);
        assert_eq!(sim.poll(150), None);
        // A late poll still reports the *scheduled* stamp.
        assert_eq!(
            sim.poll(300).map(|e| (e.at_ns, e.signal)),
            Some((250, DeviceSignal::Lost(DeviceKind::Surface)))
        );
        assert!(sim.is_lost(DeviceKind::Surface));
        // Same-time entries fire in script order, one per poll.
        assert_eq!(
            sim.poll(400).map(|e| (e.at_ns, e.signal)),
            Some((400, DeviceSignal::Recovered(DeviceKind::Audio)))
        );
        assert!(!sim.is_lost(DeviceKind::Audio));
        assert_eq!(
            sim.poll(400).map(|e| (e.at_ns, e.signal)),
            Some((400, DeviceSignal::Recovered(DeviceKind::Surface)))
        );
        assert!(!sim.is_lost(DeviceKind::Surface));
        // The schedule is exhausted.
        assert_eq!(sim.poll(u64::MAX), None);
        assert_eq!(sim.emitted(), 4);
        assert_eq!(sim.remaining(), 0);
    }

    #[test]
    fn unpaired_recovery_is_tolerated() {
        // Dirty-world: a driver that reports recovery without a loss
        // must not panic the kit; state simply clears.
        let mut sim = DeviceLossSimulator::new(&[(10, DeviceSignal::Recovered(DeviceKind::Audio))]);
        assert_eq!(
            sim.poll(10).map(|e| e.signal),
            Some(DeviceSignal::Recovered(DeviceKind::Audio))
        );
        assert!(!sim.is_lost(DeviceKind::Audio));
        assert_eq!(sim.emitted(), 1);
    }

    #[test]
    fn clock_jumps_fire_at_exact_raw_times_and_warp_exactly() {
        let mut clock = ClockJumpInjector::new(
            StepClock::new(),
            &[
                ClockJump::forward(1_000_000_000, 5_000_000_000), // +5 s at raw t=1 s
                ClockJump::backward(3_000_000_000, 2_000_000_000), // -2 s at raw t=3 s
            ],
        );
        clock.advance(500_000_000);
        assert_eq!(clock.now_ns(), 500_000_000); // nothing armed yet
        assert_eq!(clock.fired_jumps(), 0);
        clock.advance(500_000_000); // raw 1 s: the forward jump fires
        assert_eq!(clock.now_ns(), 6_000_000_000);
        assert_eq!(clock.fired_jumps(), 1);
        clock.advance(1_000_000_000); // raw 2 s
        assert_eq!(clock.now_ns(), 7_000_000_000);
        clock.advance(1_000_000_000); // raw 3 s: the backward jump fires
        // NON-MONOTONIC readout — the modeled fault, exactly warped.
        assert_eq!(clock.now_ns(), 6_000_000_000);
        assert_eq!(clock.fired_jumps(), 2);
        assert_eq!(clock.offset_ns(), 3_000_000_000);
        assert_eq!(clock.raw_now_ns(), 3_000_000_000);
        clock.advance(2_000_000_000); // raw 5 s
        assert_eq!(clock.now_ns(), 8_000_000_000);
        // Schedule exhausted: further advances are warp-free.
        clock.advance(1_000_000_000);
        assert_eq!(clock.now_ns(), 9_000_000_000);
        assert_eq!(clock.fired_jumps(), 2);
    }

    #[test]
    fn clock_jump_backward_saturates_at_zero_and_forward_recovers() {
        let mut clock = ClockJumpInjector::new(
            StepClock::new(),
            &[
                ClockJump::backward(0, 10_000_000_000), // -10 s armed at raw 0
                ClockJump::forward(2_000_000_000, 20_000_000_000), // +20 s at raw 2 s
            ],
        );
        // A zero advance still fires jumps armed at the current raw
        // time (at_ns = 0 here).
        clock.advance(0);
        assert_eq!(clock.fired_jumps(), 1);
        assert_eq!(clock.now_ns(), 0); // raw 0 - 10 s clamps at 0
        clock.advance(1_000_000_000);
        assert_eq!(clock.now_ns(), 0); // still saturated, never wrapped
        clock.advance(1_000_000_000); // raw 2 s: +20 s fires
        assert_eq!(clock.now_ns(), 12_000_000_000); // (2 s - 10 s) + 20 s
        assert_eq!(clock.fired_jumps(), 2);
    }

    #[test]
    fn injector_is_a_virtual_clock_drop_in() {
        // Engine tests take `impl VirtualClock`; the injector must slot
        // in without any wrapper ceremony.
        fn read_one<C: VirtualClock>(clock: &mut C) -> u64 {
            clock.advance(1_000_000);
            clock.now_ns()
        }
        let mut clock = ClockJumpInjector::new(
            StepClock::new(),
            &[ClockJump::forward(1_000_000, 9_000_000)],
        );
        assert_eq!(read_one(&mut clock), 10_000_000); // 1 ms raw + 9 ms warp
        // The inner clock is recoverable (composition, not captivity).
        let inner = clock.into_inner();
        assert_eq!(inner.now_ns(), 1_000_000);
    }
}
