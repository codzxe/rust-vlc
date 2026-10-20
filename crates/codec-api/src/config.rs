//! Negotiated configurations (card 1.D1-R): what the registry hands a
//! [`Decoder`](crate::Decoder) or [`Encoder`](crate::Encoder) after
//! selection — codec descriptor, stream generation, thread budget, and
//! the optional hardware accelerator instance.
//!
//! These types carry the *result* of negotiation; the negotiation itself
//! (matching capabilities against a track under user policy) is the
//! registry's job and lives elsewhere (MP §6). Every field is a
//! self-validating type, so a `DecoderConfig` that exists is a config
//! the codec may trust.

use crate::hw::HwAccelerator;
use media_types::error::ErrorContext;
use media_types::ids::CodecDescriptor;
use media_types::packet::SharedBytes;
use media_types::{EngineError, ErrorKind, GenerationId, PipelineStage};
use std::fmt;
use std::sync::Arc;

/// Upper bound on helper threads a codec implementation may spawn
/// (card 1.D1-R: "thread budget").
///
/// The engine owns the one worker thread that calls the trait methods;
/// this budget bounds any *additional* parallelism the implementation
/// creates (slice-parallel video decode, for example). Invariant:
/// `1..=MAX_THREADS` — never zero (that would be a stalled worker),
/// never unbounded (MP §12 resource discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadBudget {
    threads: u32,
}

impl ThreadBudget {
    /// Largest budget a configuration may carry `[PROPOSAL]` — 256
    /// helper threads exceeds any real codec's useful parallelism while
    /// keeping a misconfiguration visible at configure time.
    pub const MAX_THREADS: u32 = 256;

    /// Validate a budget.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] (code `codec.threads.out_of_range`)
    /// when `threads` is zero or above
    /// [`MAX_THREADS`](Self::MAX_THREADS).
    pub fn new(threads: u32) -> Result<ThreadBudget, EngineError> {
        if (1..=Self::MAX_THREADS).contains(&threads) {
            Ok(ThreadBudget { threads })
        } else {
            Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("codec.threads.out_of_range"),
            ))
        }
    }

    /// The bounded thread count.
    pub fn threads(&self) -> u32 {
        self.threads
    }
}

impl Default for ThreadBudget {
    /// One thread — the conservative default for codecs without
    /// frame/slice parallelism.
    fn default() -> Self {
        ThreadBudget { threads: 1 }
    }
}

/// The negotiated decoder configuration (card 1.D1-R: codec descriptor +
/// generation + thread budget + optional accelerator).
///
/// Built by the registry/engine after selection and installed via
/// [`Decoder::configure`](crate::Decoder::configure). The `generation`
/// is the demux generation the decoder must compare every packet
/// against ([`drop_stale_packet`](crate::drop_stale_packet)).
#[derive(Clone)]
pub struct DecoderConfig {
    /// The codec to decode (equality-matches the track's descriptor).
    pub codec: CodecDescriptor,
    /// Current demux generation — the staleness watermark.
    pub generation: GenerationId,
    /// Helper-thread budget (the calling worker is not counted).
    pub threads: ThreadBudget,
    /// The negotiated hardware accelerator, if any. `None` means
    /// software decode. H.264/HEVC receive one per ADR-001 §1 when the
    /// platform provides an audited adapter.
    pub accelerator: Option<Arc<dyn HwAccelerator>>,
    /// Out-of-band codec initialization bytes (AVC config records, AAC
    /// ASC, …) when the container separates them from the stream
    /// `[PROPOSAL]` — not in the card's field list, but without it
    /// Phase 2 would have to smuggle extradata through fake packets.
    pub extra_data: Option<SharedBytes>,
}

impl DecoderConfig {
    /// Assemble a negotiated decoder config. All inputs are
    /// self-validating types, so this cannot fail.
    pub fn new(
        codec: CodecDescriptor,
        generation: GenerationId,
        threads: ThreadBudget,
        accelerator: Option<Arc<dyn HwAccelerator>>,
        extra_data: Option<SharedBytes>,
    ) -> DecoderConfig {
        DecoderConfig {
            codec,
            generation,
            threads,
            accelerator,
            extra_data,
        }
    }
}

impl fmt::Debug for DecoderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The accelerator prints as its backend id (ADR-001's truthful
        // reporting label), not as a pointer — trait objects cannot
        // derive Debug and diagnostics should say *what was negotiated*.
        let accelerator = match &self.accelerator {
            Some(accel) => accel.backend_id(),
            None => "software",
        };
        f.debug_struct("DecoderConfig")
            .field("codec", &self.codec)
            .field("generation", &self.generation.counter())
            .field("threads", &self.threads.threads())
            .field("accelerator", &accelerator)
            .field("extra_data", &self.extra_data)
            .finish()
    }
}

/// The negotiated encoder configuration — the symmetric counterpart of
/// [`DecoderConfig`] (card 1.D1-R).
///
/// No `extra_data`: encoders *emit* header bytes into their packet
/// stream (the first packets out of `configure`d state) rather than
/// consume them; seeding input-side config blobs is deferred to the
/// Phase 7 transcode cards `[PROPOSAL]`.
#[derive(Clone)]
pub struct EncoderConfig {
    /// The codec to encode.
    pub codec: CodecDescriptor,
    /// Current generation — the staleness watermark for input frames.
    pub generation: GenerationId,
    /// Helper-thread budget.
    pub threads: ThreadBudget,
    /// The negotiated hardware encoder, if any.
    pub accelerator: Option<Arc<dyn HwAccelerator>>,
}

impl EncoderConfig {
    /// Assemble a negotiated encoder config (cannot fail — see
    /// [`DecoderConfig::new`]).
    pub fn new(
        codec: CodecDescriptor,
        generation: GenerationId,
        threads: ThreadBudget,
        accelerator: Option<Arc<dyn HwAccelerator>>,
    ) -> EncoderConfig {
        EncoderConfig {
            codec,
            generation,
            threads,
            accelerator,
        }
    }
}

impl fmt::Debug for EncoderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let accelerator = match &self.accelerator {
            Some(accel) => accel.backend_id(),
            None => "software",
        };
        f.debug_struct("EncoderConfig")
            .field("codec", &self.codec)
            .field("generation", &self.generation.counter())
            .field("threads", &self.threads.threads())
            .field("accelerator", &accelerator)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAccelerator;

    impl HwAccelerator for MockAccelerator {
        fn backend_id(&self) -> &str {
            "hw.mock"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn thread_budget_bounds_are_enforced() {
        assert_eq!(
            ThreadBudget::default().threads(),
            1,
            "conservative default: one thread"
        );
        for good in [1, 2, 8, ThreadBudget::MAX_THREADS] {
            let budget = ThreadBudget::new(good).unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(budget.threads(), good);
        }
        for bad in [0, ThreadBudget::MAX_THREADS + 1, u32::MAX] {
            match ThreadBudget::new(bad) {
                Err(e) => {
                    assert!(matches!(e.kind, ErrorKind::Unsupported { .. }), "{bad}");
                    assert_eq!(e.context.code, "codec.threads.out_of_range");
                    assert_eq!(e.stage, PipelineStage::Decode);
                }
                Ok(_) => unreachable!("{bad} must be rejected"),
            }
        }
    }

    #[test]
    fn decoder_config_assembles_and_debugs_its_negotiation() {
        let codec = CodecDescriptor::new("h264").unwrap_or_else(|e| unreachable!("{e}"));
        let generation = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e}"));
        let software = DecoderConfig::new(
            codec.clone(),
            generation,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(vec![0x01, 0x02])),
        );
        assert_eq!(software.codec, codec);
        assert_eq!(software.generation, generation);
        assert_eq!(software.threads.threads(), 1);
        assert!(software.accelerator.is_none());
        assert_eq!(software.extra_data.as_ref().map(|d| d.len()), Some(2));
        let rendered = format!("{software:?}");
        assert!(rendered.contains("software"), "{rendered}");
        assert!(rendered.contains("h264"), "{rendered}");

        let accelerated = DecoderConfig::new(
            codec,
            generation,
            ThreadBudget::new(4).unwrap_or_else(|e| unreachable!("{e}")),
            Some(Arc::new(MockAccelerator)),
            None,
        );
        let rendered = format!("{accelerated:?}");
        assert!(rendered.contains("hw.mock"), "{rendered}");
        assert!(rendered.contains("4"), "{rendered}");
    }

    #[test]
    fn encoder_config_mirrors_decoder_config() {
        let codec = CodecDescriptor::new("av1").unwrap_or_else(|e| unreachable!("{e}"));
        let config = EncoderConfig::new(
            codec.clone(),
            GenerationId::FIRST,
            ThreadBudget::new(2).unwrap_or_else(|e| unreachable!("{e}")),
            None,
        );
        assert_eq!(config.codec, codec);
        assert_eq!(config.generation, GenerationId::FIRST);
        assert_eq!(config.threads.threads(), 2);
        let rendered = format!("{config:?}");
        assert!(rendered.contains("EncoderConfig"), "{rendered}");
        assert!(rendered.contains("software"), "{rendered}");
    }
}
