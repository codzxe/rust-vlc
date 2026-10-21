//! The sink-facing buffer type and the trait output backends implement
//! (doc 01 §2.5 `audio-engine` interface sketch, verbatim shape).
//!
//! [`AudioSink`] is what the mixer feeds; `audio-output`'s
//! `AudioBackend` extends it with device enumeration, open/pause/
//! resume and the sample counter (doc 01 §2.6, doc 03 §4.9). Keeping
//! the write/latency half here means the engine can target any sink
//! without naming a device API (ARCH-6: no OS types upward).

use media_types::frame::AudioFormat;
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Stamp};

/// One block of processed audio headed for the output device
/// (doc 01 §2.5 `AudioBuffer`).
///
/// The bytes are always **interleaved** in `format`'s layout — the
/// planar mixer format is an internal engine state; sinks receive the
/// device-shaped payload (doc 03 §4.8 a, stage D).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioBuffer {
    /// Sample format, channels, layout, rate of `data`.
    pub format: AudioFormat,
    /// Interleaved PCM bytes (`format.frame_bytes() * frames`).
    pub data: SharedBytes,
    /// Presentation timestamp of the first frame in the buffer.
    pub pts: Stamp,
}

impl AudioBuffer {
    /// Whole sample-instants in this buffer, or `None` when the byte
    /// length is not a whole number of frames (data corruption — the
    /// pipeline never emits that shape; a sink seeing it treats it as
    /// malformed input).
    pub fn frames(&self) -> Option<u64> {
        self.format.frames_in(self.data.len())
    }
}

/// The mixer's output target (doc 01 §2.5 `AudioSink`).
///
/// Implementations: `audio-output` backends (null, and the platform
/// adapters behind ADR-005), capturing sinks in tests. `write` runs on
/// the pump side — it may block for backpressure (bounded queues are
/// the backend's business), but it must not deadlock against the
/// device callback: the real-time consumer never holds the same lock
/// the producer takes (doc 03 §4.9 c — the SPSC ring's wait-free index
/// swap is the only synchronization on the callback path).
pub trait AudioSink: Send {
    /// Accept one processed buffer. Errors are typed engine errors
    /// (a full bounded queue, a lost device — MP §8).
    ///
    /// # Errors
    /// [`media_types::EngineError`] at the sink's stage — e.g.
    /// [`ResourceLimit`](media_types::ErrorKind::ResourceLimit) for a
    /// full queue, [`Device`](media_types::ErrorKind::Device) for a
    /// lost output.
    fn write(&mut self, buf: &AudioBuffer) -> Result<(), media_types::EngineError>;

    /// The device's current output latency, when known. `None` is an
    /// honest "not measurable" (e.g. a discard sink), never a guess.
    fn latency(&self) -> Option<Duration>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::{AudioLayout, SampleFormat};

    fn stereo_f32() -> AudioFormat {
        AudioFormat {
            sample: SampleFormat::F32,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 44_100,
        }
    }

    #[test]
    fn buffer_frame_math() {
        let buf = AudioBuffer {
            format: stereo_f32(),
            data: SharedBytes::from_vec(vec![0u8; 2 * 2 * 4]),
            pts: Stamp::Missing,
        };
        assert_eq!(buf.frames(), Some(2));
        let bad = AudioBuffer {
            format: stereo_f32(),
            data: SharedBytes::from_vec(vec![0u8; 7]),
            pts: Stamp::Missing,
        };
        assert_eq!(bad.frames(), None, "partial frame is data, not rounded");
    }
}
