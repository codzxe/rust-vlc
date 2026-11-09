//! Typed error constructors for the output path (doc 01 §3.4
//! taxonomy), stage [`AudioOutput`](media_types::PipelineStage::AudioOutput).

use media_types::error::{
    CorruptDetail, DeviceCause, DeviceSlot, EngineError, ErrorContext, ErrorKind, PipelineStage,
};
use media_types::frame::AudioFormat;

const STAGE: PipelineStage = PipelineStage::AudioOutput;

/// An unsupported value or backend at the output boundary (shapes,
/// stub backends, unknown devices).
pub fn unsupported(code: &'static str) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new(code),
    )
}

/// The requested output format is not supported by the device (v0
/// devices speak interleaved f32 only — doc 03 §4.9's callback type).
pub fn format_unsupported(want: &AudioFormat) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Device {
            which: DeviceSlot::AudioOutput,
            cause: DeviceCause::FormatUnsupported,
        },
        ErrorContext::new("audio.format_unsupported"),
    )
    .with_source(std::io::Error::other(format!(
        "wanted {:?} {}ch @ {}Hz",
        want.sample, want.channels, want.rate
    )))
}

/// The output queue is full — the pump's backpressure signal
/// (bounded queues, MP §8). The whole buffer is refused; the caller
/// retries.
pub fn ring_full(budget: u64, attempted: u64) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::ResourceLimit {
            limit: media_types::error::LimitKind::QueueSize,
            budget,
            attempted,
        },
        ErrorContext::new("audio.ring_full"),
    )
}

/// The PCM pump's pacing refusal — the device is more than the
/// configured high-water mark behind (bounded-queue backpressure,
/// MP §8; the wave-14 `PcmPump`). The whole buffer is refused; the
/// caller waits for the playback clock to advance and retries.
pub fn pump_backpressure(high_water: u64, attempted: u64) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::ResourceLimit {
            limit: media_types::error::LimitKind::QueueSize,
            budget: high_water,
            attempted,
        },
        ErrorContext::new("audio.pump_backpressure"),
    )
}

/// A payload that is not a whole number of frames.
pub fn partial_frame(expected: u64, found: u64) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        ErrorContext::new("audio.partial_frame"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_stage_and_stable_codes() {
        let e = ring_full(64, 128);
        assert_eq!(e.stage, PipelineStage::AudioOutput);
        assert_eq!(e.context.code, "audio.ring_full");
        assert!(e.to_string().contains("[audio-output]"), "{}", e);
        assert!(e.to_string().contains("audio.ring_full"), "{}", e);

        let e = partial_frame(8, 3);
        assert_eq!(e.context.code, "audio.partial_frame");
        assert!(e.to_string().contains("truncated"), "{}", e);
    }
}
