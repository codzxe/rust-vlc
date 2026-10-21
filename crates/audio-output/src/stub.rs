//! The typed not-supported stub for real-device backends (card 2.C3's
//! fallback: "real-device adapters stubbed with typed not-supported
//! errors, honestly recorded").
//!
//! Doc 01 §2.7 places the real OS audio backends in `platform-*` (L5)
//! crates behind ADR-005 (audio output's E2 exception slot — the
//! cpal-vs-direct-API decision; the cpal P0 record lives at
//! `docs/dependencies/records/cpal-0.18.2.md`). Until those cards
//! land, this stub occupies the selection slot truthfully: it
//! enumerates **no devices** and every operation answers with a
//! typed [`Unsupported`](media_types::ErrorKind::Unsupported) error
//! naming the platform and the pending decision — never a fake
//! success, never a panic.

use std::sync::Arc;

use audio_engine::AudioSink;
use audio_engine::buffer::AudioBuffer;
use media_clock::SampleCounterSource;
use media_types::frame::AudioFormat;
use media_types::time::Duration;
use media_types::{EngineError, ErrorKind};

use crate::device::{AudioBackend, DeviceDesc, DeviceId};

/// The placeholder for a real (OS) audio backend. Construct via
/// [`UnsupportedBackend::for_current_os`] or
/// [`UnsupportedBackend::named`].
#[derive(Debug, Clone)]
pub struct UnsupportedBackend {
    platform: &'static str,
}

impl UnsupportedBackend {
    /// The stub for the compiled target's OS (a compile-time name —
    /// no OS type leaks, ARCH-6).
    pub fn for_current_os() -> UnsupportedBackend {
        let platform = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "other"
        };
        UnsupportedBackend { platform }
    }

    /// The stub for a named platform (tests + future selection
    /// tables).
    pub fn named(platform: &'static str) -> UnsupportedBackend {
        UnsupportedBackend { platform }
    }

    /// The platform this stub stands in for.
    pub fn platform(&self) -> &'static str {
        self.platform
    }

    fn pending(&self) -> EngineError {
        // The stable, honest refusal: this backend's implementation
        // is pending ADR-005 + the platform-* cards.
        EngineError::new(
            media_types::PipelineStage::AudioOutput,
            ErrorKind::Unsupported {
                format: Some(media_types::error::FormatInfo {
                    name: Some(format!("os-audio-backend/{}", self.platform)),
                    sniffed: Vec::new(),
                }),
            },
            media_types::error::ErrorContext::new("audio.os_backend_pending"),
        )
    }
}

impl AudioSink for UnsupportedBackend {
    fn write(&mut self, _buf: &AudioBuffer) -> Result<(), EngineError> {
        Err(self.pending())
    }

    fn latency(&self) -> Option<Duration> {
        None
    }
}

impl AudioBackend for UnsupportedBackend {
    fn enumerate(&self) -> Vec<DeviceDesc> {
        // Honest: no devices until a real backend exists.
        Vec::new()
    }

    fn open(
        &mut self,
        _device: DeviceId,
        _format: &AudioFormat,
        _buffer_frames: usize,
    ) -> Result<(), EngineError> {
        Err(self.pending())
    }

    fn pause(&mut self) -> Result<(), EngineError> {
        Err(self.pending())
    }

    fn resume(&mut self) -> Result<(), EngineError> {
        Err(self.pending())
    }

    fn clock_source(&self) -> Option<Arc<dyn SampleCounterSource>> {
        None
    }

    fn underruns(&self) -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn every_operation_answers_typed_not_supported() {
        let mut stub = UnsupportedBackend::for_current_os();
        assert_eq!(stub.enumerate().len(), 0);
        assert!(stub.clock_source().is_none());
        assert_eq!(stub.underruns(), 0);
        assert_eq!(stub.latency(), None);

        let err = stub.pause().err().must();
        assert_eq!(err.context.code, "audio.os_backend_pending");
        assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));

        // The refusal names the platform and the pending decision.
        let rendered = err.to_string();
        assert!(rendered.contains("os-audio-backend/"), "{rendered}");

        let err = stub
            .open(
                DeviceId::new(0),
                &AudioFormat {
                    sample: media_types::frame::SampleFormat::F32,
                    channels: 2,
                    layout: media_types::frame::AudioLayout::Interleaved,
                    rate: 44_100,
                },
                64,
            )
            .err()
            .must();
        assert_eq!(err.context.code, "audio.os_backend_pending");

        // A named platform round-trips.
        let mut stub = UnsupportedBackend::named("plan9");
        assert_eq!(stub.platform(), "plan9");
        let err = stub.resume().err().must();
        assert!(
            err.to_string().contains("os-audio-backend/plan9"),
            "{}",
            err
        );
    }
}
