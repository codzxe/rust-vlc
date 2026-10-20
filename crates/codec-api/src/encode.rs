//! The encode-side contract (card 1.D1-R: symmetric to [`crate::decode`]):
//! [`Encoder`], [`EncodeOutcome`], and the single packet emission path
//! [`PacketSink`]. Encoding serves the Phase-7 transcode/record paths
//! (doc 01 registry row `transcode`); the trait is landed now so the
//! vocabulary is stable before any implementation exists.

use crate::config::EncoderConfig;
use media_types::{EngineError, Frame, GenerationId, Packet};

/// A codec encoder — the mirror of
/// [`Decoder`](crate::Decoder) over the same worker model.
///
/// Configure once per stream generation, then feed decoded [`Frame`]s;
/// compressed packets leave **only** through the [`PacketSink`] handed to
/// each call.
///
/// # Threading and no-blocking contract (binding)
///
/// Identical to the [`Decoder`](crate::Decoder) contract, with frames in
/// place of packets:
///
/// * called from a dedicated worker thread (`Send` supertrait);
/// * bounded work per call — no indefinite waits, buffering is reported
///   as [`EncodeOutcome::NeedMoreInput`], helper threads only within the
///   configured [`ThreadBudget`](crate::ThreadBudget);
/// * cancellation is generational: a frame that is stale
///   ([`Frame::is_stale`](media_types::Frame::is_stale) against the
///   configured generation) is dropped on sight and reported as
///   [`EncodeOutcome::NeedMoreInput`] with zero packets — run
///   [`drop_stale_frame`] first;
/// * panics are contract violations; a malformed input frame is
///   [`EncodeOutcome::Recovered`] or a typed `Err`, never a panic;
/// * `Err` is fatal per generation, but `flush` must still answer.
pub trait Encoder: Send {
    /// Install the negotiated configuration for this stream generation.
    ///
    /// # Errors
    /// Typed [`EngineError`] — typically
    /// [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
    /// when the implementation cannot serve the requested codec/profile.
    fn configure(&mut self, cfg: &EncoderConfig) -> Result<(), EngineError>;

    /// Encode one frame, pushing packets to `out`.
    ///
    /// Bounded work: consumes at most the given frame. Sink errors stop
    /// the call and propagate (verbatim or chained) — same rule as
    /// [`FrameSink`](crate::FrameSink).
    fn encode(
        &mut self,
        frame: Frame,
        out: &mut dyn PacketSink,
    ) -> Result<EncodeOutcome, EngineError>;

    /// Signal end-of-input and drain everything buffered; stays
    /// [`EncodeOutcome::Drained`] once drained.
    fn flush(&mut self, out: &mut dyn PacketSink) -> Result<EncodeOutcome, EngineError>;
}

/// What one bounded `encode`/`flush` call did — the mirror of
/// [`DecodeOutcome`](crate::DecodeOutcome).
///
/// Outcome/sink agreement applies symmetrically (see
/// [`contract::encode_outcome_agrees_with_sink`]): `Emitted { packets: n
/// }` means exactly `n` `accept` calls in this call; `NeedMoreInput` and
/// `Drained` mean zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use = "ignoring an encode outcome loses recovery and drain information"]
pub enum EncodeOutcome {
    /// `packets` compressed packets were pushed to the sink during this
    /// call (`packets >= 1`).
    Emitted {
        /// Number of packets pushed to the [`PacketSink`] in this call.
        packets: u32,
    },
    /// The frame was consumed (or dropped as stale/buffered) with no
    /// complete packet yet.
    NeedMoreInput,
    /// No buffered state remains; the stream is fully drained.
    Drained,
    /// Damage was survived with `skipped` units dropped; see
    /// [`RecoverReason`](crate::RecoverReason).
    Recovered {
        /// How many frames/packets were dropped to recover.
        skipped: u32,
        /// Typed recovery cause.
        reason: crate::RecoverReason,
    },
}

/// The only emission path for encoded packets — the mirror of
/// [`FrameSink`](crate::FrameSink) (not `Send`, same reasoning).
///
/// # Error propagation (contract)
///
/// An `Err` from `accept` stops the current call and is propagated to
/// the caller (verbatim or via
/// [`with_source`](media_types::EngineError::with_source)); never
/// swallowed, never pushed past.
pub trait PacketSink {
    /// Take delivery of one encoded packet.
    ///
    /// # Errors
    /// Whatever the downstream consumer fails with, as an
    /// [`EngineError`].
    fn accept(&mut self, packet: Packet) -> Result<(), EngineError>;
}

/// The canonical stale-frame guard, mirror of
/// [`drop_stale_packet`](crate::drop_stale_packet): stale frames are
/// dropped on sight and reported as [`EncodeOutcome::NeedMoreInput`]
/// with zero packets.
#[must_use = "the guard's answer is the call's answer when it fires"]
pub fn drop_stale_frame(frame: &Frame, current: GenerationId) -> Option<EncodeOutcome> {
    if frame.is_stale(current) {
        Some(EncodeOutcome::NeedMoreInput)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::{AudioFrame, SampleFormat};
    use media_types::packet::SharedBytes;
    use media_types::{AudioFormat, Stamp};

    fn audio_frame_at(generation: GenerationId) -> Frame {
        Frame::Audio(AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: media_types::frame::AudioLayout::Interleaved,
                rate: 48_000,
            },
            samples: SharedBytes::from_vec(vec![0; 4]),
            pts: Stamp::Missing,
            duration: None,
            generation,
        })
    }

    #[test]
    fn stale_frame_guard_fires_only_for_past_generations() {
        let current = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e}"));
        match drop_stale_frame(&audio_frame_at(GenerationId::FIRST), current) {
            Some(EncodeOutcome::NeedMoreInput) => {}
            other => unreachable!("stale drop must be NeedMoreInput, got {other:?}"),
        }
        assert!(drop_stale_frame(&audio_frame_at(current), current).is_none());
        assert!(
            drop_stale_frame(
                &audio_frame_at(current.next().unwrap_or_else(|e| unreachable!("{e}"))),
                current
            )
            .is_none()
        );
    }

    #[test]
    fn encode_outcome_mirrors_decode_outcome() {
        assert_eq!(
            EncodeOutcome::Emitted { packets: 3 },
            EncodeOutcome::Emitted { packets: 3 }
        );
        assert_ne!(EncodeOutcome::NeedMoreInput, EncodeOutcome::Drained);
        assert_eq!(
            EncodeOutcome::Recovered {
                skipped: 0,
                reason: crate::RecoverReason::HwFallback
            },
            EncodeOutcome::Recovered {
                skipped: 0,
                reason: crate::RecoverReason::HwFallback
            }
        );
    }
}
