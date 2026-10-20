//! The codec kinds (doc 06 §1.2 rows 4–5): decoder and encoder.
//!
//! **These traits are NOT redefined here.** They are
//! [`codec_api::Decoder`] and [`codec_api::Encoder`] — owned by the
//! codec contract crate (card 1.D1, doc 01 §2.4) and re-exported so a
//! plugin author has one import surface. Kind 4 impls live in
//! `codecs` (L3) with hardware paths via
//! [`codec_api::HwAccelerator`] in `platform-*` (L5, ADR-001); kind 5
//! is Phase 7 (transcode jobs only).
//!
//! What this module adds on top of the re-exports:
//!
//! - the [`CodecPlugin`] marker: how a codec-kind plugin *presents*
//!   itself to the registry (factory surface), keeping the decode
//!   contract itself untouched (doc 06 §1.2 separates the trait from
//!   the plugin registration seam);
//! - typed stage labels for registry-side codec errors.

pub use codec_api::decode::{DecodeOutcome, Decoder, FrameSink, RecoverReason};
pub use codec_api::encode::{EncodeOutcome, Encoder, PacketSink as EncodePacketSink};
pub use codec_api::{DecoderConfig, EncoderConfig};

/// Factory surface of a decoder-kind plugin (doc 06 §1.2 row 4: the
/// registry instantiates one decoder **per track**).
///
/// The [`Decoder`] trait itself stays the pure decode contract
/// (configure / decode / flush — doc 01 §2.4); this marker adds only
/// the plugin registration seam: given a negotiated
/// [`DecoderConfig`], construct the instance that will serve it, or
/// refuse honestly (ADR-001's `unsupported_software_decode_pending`
/// posture — a machine without an OS decoder for a hardware-first
/// codec says so, never fakes success).
///
/// # Calling context
///
/// `instantiate` runs on the engine's codec-instantiation path
/// (worker thread, bounded work: construction, no I/O). The returned
/// decoder is owned by the decode worker.
pub trait CodecPlugin: Send + Sync {
    /// The codec this plugin decodes (equality-matches the track's
    /// [`CodecDescriptor`](media_types::ids::CodecDescriptor) — the
    /// registry checks it against the manifest's `codecs` capability
    /// claim, doc 06 §7.2).
    fn codec(&self) -> media_types::ids::CodecDescriptor;

    /// Construct a decoder for a negotiated config.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Decode`]) when the
    /// plugin cannot serve this config — unsupported profile, a
    /// refused accelerator, or the honest ADR-001
    /// software-decode-pending refusal.
    fn instantiate(
        &self,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, media_types::EngineError>;
}

/// Typed registry-side error: the plugin's claimed codec does not
/// match what it serves (a manifest lie — doc 06 §7.2 blocks shipping
/// on it).
pub fn codec_claim_mismatch(claimed: &str, served: &str) -> media_types::EngineError {
    media_types::EngineError::new(
        media_types::PipelineStage::Plugin,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new("plugin.codec.claim-mismatch"),
    )
    .with_source(ClaimMismatch {
        claimed: claimed.to_string(),
        served: served.to_string(),
    })
}

#[derive(Debug)]
struct ClaimMismatch {
    claimed: String,
    served: String,
}

impl std::fmt::Display for ClaimMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "manifest claims codec `{}` but the plugin serves `{}`",
            self.claimed, self.served
        )
    }
}
impl std::error::Error for ClaimMismatch {}

#[cfg(test)]
mod tests {
    use super::*;
    use codec_api::config::ThreadBudget;
    use media_types::ids::{CodecDescriptor, GenerationId};
    use media_types::packet::SharedBytes;
    use media_types::time::Stamp;
    use media_types::{EngineError, Frame, Packet, PipelineStage};

    /// A pass-through decoder proving the factory → trait wiring.
    struct CopyDecoder;

    impl Decoder for CopyDecoder {
        fn configure(&mut self, _cfg: &DecoderConfig) -> Result<(), EngineError> {
            Ok(())
        }
        fn decode(
            &mut self,
            packet: Packet,
            out: &mut dyn FrameSink,
        ) -> Result<DecodeOutcome, EngineError> {
            let frame = Frame::Audio(media_types::frame::AudioFrame {
                format: media_types::AudioFormat {
                    sample: media_types::frame::SampleFormat::S16,
                    channels: 1,
                    layout: media_types::frame::AudioLayout::Interleaved,
                    rate: 48_000,
                },
                samples: packet.data,
                pts: packet.pts,
                duration: packet.duration,
                generation: packet.generation,
            });
            match out.accept(frame) {
                Ok(()) => Ok(DecodeOutcome::Emitted { frames: 1 }),
                Err(e) => Err(e),
            }
        }
        fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
            Ok(DecodeOutcome::Drained)
        }
    }

    struct FlacPlugin;

    impl CodecPlugin for FlacPlugin {
        fn codec(&self) -> CodecDescriptor {
            match CodecDescriptor::new("flac") {
                Ok(c) => c,
                // The descriptor grammar validated the literal at its
                // own test site; here it is a constant we control.
                Err(_) => unreachable!("constant `flac` is a valid descriptor"),
            }
        }
        fn instantiate(&self, _config: &DecoderConfig) -> Result<Box<dyn Decoder>, EngineError> {
            Ok(Box::new(CopyDecoder))
        }
    }

    #[test]
    fn factory_constructs_a_working_decoder() {
        let plugin = FlacPlugin;
        let config = DecoderConfig::new(
            match CodecDescriptor::new("flac") {
                Ok(c) => c,
                Err(e) => unreachable!("{e}"),
            },
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_static(&[])),
        );
        let mut decoder = match plugin.instantiate(&config) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        match decoder.configure(&config) {
            Ok(()) => {}
            Err(e) => unreachable!("{e}"),
        }
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_static(&[1, 2, 3, 4]),
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
        let mut counting = CountingSink { frames: 0 };
        let outcome = match decoder.decode(packet, &mut counting) {
            Ok(o) => o,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
        assert_eq!(counting.frames, 1, "sink and outcome agree");
    }

    struct CountingSink {
        frames: u32,
    }

    impl FrameSink for CountingSink {
        fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
            self.frames += 1;
            Ok(())
        }
    }

    #[test]
    fn claim_mismatch_is_a_plugin_stage_error() {
        let err = codec_claim_mismatch("flac", "vorbis");
        assert_eq!(err.stage, PipelineStage::Plugin);
        // Display is redacted/structured by construction (MP §12): the
        // claim detail rides the source chain, not the summary line.
        let src = match std::error::Error::source(&err) {
            Some(s) => s.to_string(),
            None => unreachable!("source carries the claim detail"),
        };
        assert!(src.contains("flac"), "names the claim: {src}");
        assert!(src.contains("vorbis"), "names reality: {src}");
    }
}
