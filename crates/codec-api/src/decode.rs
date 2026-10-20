//! The decode-side contract (doc 01 §2.4): [`Decoder`], the outcome of
//! one bounded call ([`DecodeOutcome`]), the recovery vocabulary
//! ([`RecoverReason`]), and the single emission path ([`FrameSink`]).

use crate::config::DecoderConfig;
use media_types::{EngineError, Frame, GenerationId, Packet};

/// A codec decoder (doc 01 §2.4 sketch, verbatim surface).
///
/// Implementations live in the `codecs` crate (or plugins behind the
/// registry, MP §6); this trait is the whole contract between them and
/// the engine's decode worker. Configure once per stream generation,
/// then feed packets; frames leave **only** through the [`FrameSink`]
/// handed to each call.
///
/// # Threading and no-blocking contract (binding)
///
/// * **Worker-thread execution.** Every method is called from a
///   dedicated decode worker owned by the engine — never from UI,
///   audio-callback or demux threads. The `Send` supertrait exists for
///   exactly this hand-off; a type that cannot cross threads cannot be
///   a decoder:
///
///   ```compile_fail
///   use std::rc::Rc;
///   use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
///   use media_types::{EngineError, Packet};
///
///   // Thread-local scratch makes the type !Send, so it cannot
///   // implement Decoder (E0277: `Rc<Vec<u8>>` cannot be sent between
///   // threads safely; the trait bound `ThreadLocalDecoder: Send` is
///   // not satisfied).
///   struct ThreadLocalDecoder {
///       scratch: Rc<Vec<u8>>,
///   }
///
///   impl Decoder for ThreadLocalDecoder {
///       fn configure(&mut self, _cfg: &DecoderConfig) -> Result<(), EngineError> {
///           Ok(())
///       }
///
///       fn decode(
///           &mut self,
///           _packet: Packet,
///           _out: &mut dyn FrameSink,
///       ) -> Result<DecodeOutcome, EngineError> {
///           Ok(DecodeOutcome::NeedMoreInput)
///       }
///
///       fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
///           Ok(DecodeOutcome::Drained)
///       }
///   }
///   ```
///
/// * **Bounded work per call.** `decode` processes the one packet it was
///   given (or buffers it); `flush` drains already-buffered state. No
///   method may wait indefinitely: no blocking I/O, no unbounded lock
///   waits, no device-event waits. "Waiting for more input" is expressed
///   as [`DecodeOutcome::NeedMoreInput`], never by blocking. Helper
///   threads are permitted within the configured
///   [`ThreadBudget`](crate::ThreadBudget) and must be owned and joined
///   by the implementation.
/// * **Cancellation is generational.** The engine cancels by bumping the
///   generation; a packet that is stale
///   ([`Packet::is_stale`](media_types::Packet::is_stale) against the
///   configured generation) is dropped **on sight** and reported as
///   [`DecodeOutcome::NeedMoreInput`] with zero frames — no error, no
///   decode work, no state mutation. Implementations should run
///   [`drop_stale_packet`] as their first statement.
/// * **Panics are contract violations.** A panicking decoder is a
///   decoder defect (the engine treats it as an
///   [`ErrorKind::Decoder`](media_types::ErrorKind::Decoder) fault and
///   discards the instance); corrupt media is
///   [`DecodeOutcome::Recovered`] or a typed `Err`, never a panic.
/// * **Errors are fatal per generation.** After returning `Err`, the
///   instance may still be asked to `flush` (which must answer, not
///   panic); the engine then reconfigures or discards it.
///
/// # Misuse
///
/// Calling `decode`/`flush` before a successful `configure` must fail
/// with a typed [`ErrorKind::Decoder`](media_types::ErrorKind::Decoder)
/// error (`BadState`), never panic — see the contract suite
/// ([`crate::contract`]) for the executable rules.
pub trait Decoder: Send {
    /// Install the negotiated configuration for this stream generation.
    ///
    /// Called once before decoding (and again, on a fresh generation,
    /// after seeks/source switches). Rejecting the config is a typed
    /// error — typically [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported).
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError>;

    /// Decode one packet, pushing any frames to `out`.
    ///
    /// Bounded work: this call consumes at most the given packet. All
    /// emitted frames go through `out`; an [`EngineError`] returned by
    /// `out.accept` stops the call and is propagated to the caller
    /// (verbatim or via
    /// [`with_source`](media_types::EngineError::with_source)).
    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError>;

    /// Signal end-of-input and drain everything buffered.
    ///
    /// Returns [`DecodeOutcome::Drained`] once nothing remains; further
    /// `flush` calls stay `Drained`. Also the drain path after seek
    /// generation switches.
    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError>;
}

/// What one bounded `decode`/`flush` call did (doc 01 §2.4, verbatim
/// surface).
///
/// Outcome/sink agreement is part of the contract (see
/// [`contract::decode_outcome_agrees_with_sink`]): `Emitted { frames: n }`
/// means exactly `n` `accept` calls happened during this call (and `n`
/// is at least 1); `NeedMoreInput` and `Drained` mean zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use = "ignoring a decode outcome loses recovery and drain information"]
pub enum DecodeOutcome {
    /// `frames` frames were pushed to the sink during this call
    /// (`frames >= 1`; nothing to emit is `NeedMoreInput`).
    Emitted {
        /// Number of frames pushed to the [`FrameSink`] in this call.
        frames: u32,
    },
    /// Input was consumed (or dropped as stale) with nothing to emit
    /// yet; the decoder waits for the next packet or `flush`.
    NeedMoreInput,
    /// No buffered state remains; the stream is fully drained.
    Drained,
    /// Damage was survived: `skipped` units were dropped while
    /// recovering, for reason `reason`. The decoder remains usable.
    Recovered {
        /// How many packets/frames were dropped to resynchronize.
        skipped: u32,
        /// Typed recovery cause.
        reason: RecoverReason,
    },
}

/// Typed recovery causes for [`DecodeOutcome::Recovered`] (card 1.D1-R).
///
/// A recovery is *not* an error: the decoder stayed within its bounded
/// work, kept its state usable, and reports what it had to drop or
/// paper over. Variants marked *decode-side* or *encode-side* name where
/// a reason can arise; the enum is shared so reporting stays one
/// vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoverReason {
    /// Corrupt input was detected and resynchronized past; at least one
    /// unit was skipped (decode-side for packets, encode-side for
    /// malformed frames).
    CorruptInput,
    /// A damaged frame was concealed rather than dropped — output was
    /// produced from partial data (decode-side).
    ConcealedFrame,
    /// The hardware path failed and the call fell back to (or re-tried
    /// in) software (ADR-001 hw-first fallback, decode- and encode-side).
    HwFallback,
    /// A discontinuity (seek, splice, stream switch) forced a resync
    /// with skipped units (both sides).
    Discontinuity,
}

/// The only emission path for decoded frames (doc 01 §2.4, verbatim
/// surface).
///
/// Deliberately **not** `Send`: the sink is created and consumed on the
/// same worker thread that owns the decoder, so it may hold
/// thread-affine state (a queue front, a callback closure). Engines that
/// need frames elsewhere wrap the sink around a channel of their
/// choosing.
///
/// # Error propagation (contract)
///
/// An `Err` from `accept` means the downstream consumer refuses more
/// output (backpressure, teardown, device loss). The decoder must stop
/// the current call and return that error — propagated verbatim or
/// chained via [`with_source`](media_types::EngineError::with_source) —
/// never swallow it, never keep pushing.
pub trait FrameSink {
    /// Take delivery of one decoded frame.
    ///
    /// # Errors
    /// Whatever the downstream consumer fails with, as an
    /// [`EngineError`]; see the propagation contract above.
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError>;
}

/// The canonical stale-packet guard (contract: "dropped on sight").
///
/// Every `Decoder` implementation runs this as the first statement of
/// [`Decoder::decode`]: when `packet` is stale against `current` (the
/// generation the decoder was configured for), the answer is
/// [`DecodeOutcome::NeedMoreInput`] with zero frames and no decode work;
/// otherwise `None` and decoding proceeds.
#[must_use = "the guard's answer is the call's answer when it fires"]
pub fn drop_stale_packet(packet: &Packet, current: GenerationId) -> Option<DecodeOutcome> {
    if packet.is_stale(current) {
        Some(DecodeOutcome::NeedMoreInput)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::packet::SharedBytes;

    fn packet_at(generation: GenerationId) -> Packet {
        Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(vec![0x01]),
            keyframe: true,
            discontinuity: false,
            generation,
        }
    }

    #[test]
    fn stale_guard_fires_only_for_past_generations() {
        let current = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e}"));
        // Stale (past) generation: dropped on sight, reported as
        // NeedMoreInput with zero frames.
        match drop_stale_packet(&packet_at(GenerationId::FIRST), current) {
            Some(DecodeOutcome::NeedMoreInput) => {}
            other => unreachable!("stale drop must be NeedMoreInput, got {other:?}"),
        }
        // Current generation: not stale, decoding proceeds.
        assert!(drop_stale_packet(&packet_at(current), current).is_none());
        // A packet from the future is an ordering bug caught elsewhere,
        // never "stale" (packet.rs owns that rule; the guard inherits it).
        assert!(
            drop_stale_packet(
                &packet_at(current.next().unwrap_or_else(|e| unreachable!("{e}"))),
                current
            )
            .is_none()
        );
    }

    #[test]
    fn outcome_is_small_and_comparable() {
        // Outcomes are tiny value reports: Copy + Eq, so workers can
        // forward them without cloning and tests can assert equality.
        let outcomes = [
            DecodeOutcome::Emitted { frames: 1 },
            DecodeOutcome::NeedMoreInput,
            DecodeOutcome::Drained,
            DecodeOutcome::Recovered {
                skipped: 2,
                reason: RecoverReason::CorruptInput,
            },
        ];
        assert_eq!(outcomes[0], DecodeOutcome::Emitted { frames: 1 });
        assert_ne!(outcomes[1], outcomes[2]);
        let reasons = [
            RecoverReason::CorruptInput,
            RecoverReason::ConcealedFrame,
            RecoverReason::HwFallback,
            RecoverReason::Discontinuity,
        ];
        for (i, reason) in reasons.iter().enumerate() {
            let following = reasons[(i + 1) % reasons.len()];
            assert_ne!(reason, &following, "reasons are distinct");
        }
    }
}
