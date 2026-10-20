//! The filter kinds (doc 06 §1.2 rows 6–7): post-decode audio and
//! video chains.
//!
//! The **domain traits** `audio_engine::AudioFilter` and
//! `video_engine::VideoFilter` live in their L3 domain crates (doc 06
//! §1.2 trait-ownership note: keeps L1 free of L3/L4 deps). What this
//! module stages for the plugin surface [PROPOSAL: placement, flag at
//! the doc 06 §10 reconciliation]:
//!
//! - [`FilterOutcome`] — the push-result vocabulary both filter
//!   domains share (mirrors codec-api's `DecodeOutcome` shape:
//!   emitted / need-input / drained / recovered, so the engine's
//!   bounded-recovery accounting is uniform, MP §8);
//! - minimal [`AudioFilter`] / [`VideoFilter`] **host traits** for the
//!   registry's v1 builtin + light-WASM filter story (§1.4), expressed
//!   purely in L0 frame types;
//! - [`FrameSource`] — where filter input comes from (the engine's
//!   chain walk, not a queue the filter owns).

use media_types::{AudioFormat, EngineError, Frame, PixelFormat};

/// What one filter push accomplished (mirrors
// [`codec_api::DecodeOutcome`]; MP §8 bounded-recovery
/// observability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterOutcome {
    /// The call pushed exactly `frames` frames downstream.
    Emitted {
        /// Frames pushed during this call.
        frames: u32,
    },
    /// Input consumed, no output yet (buffering inside the filter is
    /// bounded and declared at registration).
    NeedMoreInput,
    /// No output now or ever again without chain renegotiation.
    Drained,
    /// The filter overcame damage; counted against the filter-stage
    /// recovery budget.
    Recovered {
        /// Output units lost to the damage.
        skipped: u32,
        /// What kind of recovery happened.
        reason: &'static str,
    },
}

/// Where filter input comes from: the engine walks the chain and hands
/// frames in (the filter never pulls from a queue it does not own).
///
/// Created and consumed on the filter worker thread — deliberately no
/// `Send` bound (the codec-api sink precedent).
pub trait FrameSource {
    /// Hand one input frame to the filter's processing step.
    ///
    /// # Errors
    /// The source's own typed error, expressed as an [`EngineError`].
    fn next_frame(&mut self) -> Result<Option<Frame>, EngineError>;
}

/// The v1 audio-filter host trait (doc 06 §1.2 row 6: post-decode
/// audio chain → mixer → sink; builtin + light WASM, §1.4).
///
/// # Calling context
///
/// Dedicated filter worker thread ([`Send`]); bounded work per call —
/// one input frame's worth of processing, never an internal
/// whole-buffer loop; cancellation is generation staleness (stale
/// frames dropped on sight,
/// [`Frame::is_stale`](media_types::Frame::is_stale)).
pub trait AudioFilter: Send {
    /// The output format this filter will produce for the given input
    /// format — negotiated at chain build, before any frame flows.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::AudioFilter`]) when the
    /// input format is not servable.
    fn negotiate(&self, input: &AudioFormat) -> Result<AudioFormat, EngineError>;

    /// Push one frame through the filter step.
    ///
    /// # Errors
    /// [`EngineError`] on unrecoverable processing failure (stage
    /// [`PipelineStage::AudioFilter`]).
    fn push(
        &mut self,
        frame: Frame,
        out: &mut dyn FnMut(Frame) -> Result<(), EngineError>,
    ) -> Result<FilterOutcome, EngineError>;
}

/// The v1 video-filter host trait (doc 06 §1.2 row 7: post-decode
/// video chain → presentation scheduler; builtin + light WASM).
///
/// Same calling-context contract as [`AudioFilter`]; stage label
/// [`PipelineStage::VideoFilter`].
pub trait VideoFilter: Send {
    /// The output pixel format for a given input (chain-build time).
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::VideoFilter`]) when the
    /// input format is not servable.
    fn negotiate(&self, input: &PixelFormat) -> Result<PixelFormat, EngineError>;

    /// Push one frame through the filter step.
    ///
    /// # Errors
    /// [`EngineError`] on unrecoverable processing failure (stage
    /// [`PipelineStage::VideoFilter`]).
    fn push(
        &mut self,
        frame: Frame,
        out: &mut dyn FnMut(Frame) -> Result<(), EngineError>,
    ) -> Result<FilterOutcome, EngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::{AudioLayout, SampleFormat};
    use media_types::ids::GenerationId;
    use media_types::time::Stamp;

    /// A gain-less pass-through audio filter: negotiation is identity,
    /// push is a copy.
    struct Passthrough;

    impl AudioFilter for Passthrough {
        fn negotiate(&self, input: &AudioFormat) -> Result<AudioFormat, EngineError> {
            Ok(*input)
        }
        fn push(
            &mut self,
            frame: Frame,
            out: &mut dyn FnMut(Frame) -> Result<(), EngineError>,
        ) -> Result<FilterOutcome, EngineError> {
            match out(frame) {
                Ok(()) => Ok(FilterOutcome::Emitted { frames: 1 }),
                Err(e) => Err(e),
            }
        }
    }

    fn audio_frame(generation: GenerationId) -> Frame {
        Frame::Audio(media_types::frame::AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: 48_000,
            },
            samples: media_types::packet::SharedBytes::from_static(&[0; 4]),
            pts: Stamp::Missing,
            duration: None,
            generation,
        })
    }

    #[test]
    fn passthrough_negotiates_and_emits() {
        let fmt = AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 48_000,
        };
        let mut filter = Passthrough;
        let out_fmt = match filter.negotiate(&fmt) {
            Ok(f) => f,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(out_fmt, fmt, "identity negotiation");
        let mut pushed = 0u32;
        let mut sink = |frame: Frame| -> Result<(), EngineError> {
            assert!(matches!(frame, Frame::Audio(_)));
            pushed += 1;
            Ok(())
        };
        let outcome = match filter.push(audio_frame(GenerationId::FIRST), &mut sink) {
            Ok(o) => o,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(outcome, FilterOutcome::Emitted { frames: 1 });
        assert_eq!(pushed, 1, "sink and outcome agree");
    }

    #[test]
    fn sink_errors_propagate() {
        let mut filter = Passthrough;
        let mut sink = |_frame: Frame| -> Result<(), EngineError> {
            Err(EngineError::new(
                media_types::PipelineStage::AudioFilter,
                media_types::ErrorKind::ResourceLimit {
                    limit: media_types::error::LimitKind::QueueSize,
                    budget: 0,
                    attempted: 1,
                },
                media_types::error::ErrorContext::new("test.sink.full"),
            ))
        };
        let result = filter.push(audio_frame(GenerationId::FIRST), &mut sink);
        assert!(result.is_err(), "sink errors are never swallowed");
    }
}
