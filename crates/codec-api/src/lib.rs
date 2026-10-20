//! The Lumen codec contract (doc 01 §2.4, card 1.D1/1.D1-R): the
//! `Decoder`/`Encoder` traits, the frame and packet sinks they push to,
//! capability descriptors, negotiated configurations, and the
//! backend-neutral hardware abstraction bounded by ADR-001
//! (`docs/adr/ADR-001-hardware-decode.md`).
//!
//! Layer L1 (doc 01 §2 registry), depends only on `media-types`
//! `[PROPOSAL per doc 01 §2.2(c) — std-only besides]`. This crate is a
//! **contract**: it contains no codec implementations, names no platform
//! types (hardware stays abstract, doc 01 §2.4(d)), and carries no
//! selection policy — choosing between implementations by capability and
//! priority is the plugin registry's job (MP §6). What it does define:
//!
//! - [`Decoder`] / [`Encoder`] — the pipeline-stage traits, with the
//!   binding threading/no-blocking contract in their trait docs;
//! - [`DecodeOutcome`] / [`EncodeOutcome`] — what one bounded call did,
//!   including typed recovery reports;
//! - [`FrameSink`] / [`PacketSink`] — the only emission paths;
//! - [`DecoderConfig`] / [`EncoderConfig`] — the negotiated result the
//!   registry hands an implementation (codec descriptor, generation,
//!   thread budget, optional accelerator);
//! - [`CodecCapability`] — what an implementation offers (profiles and
//!   levels as opaque strings, dimension limits, ADR-001 hardware
//!   posture);
//! - [`HwAccelerator`] — the opaque hardware seam (ADR-001);
//! - [`contract`] — the reusable test suite every codec in `codecs` and
//!   the conformance kit runs against its implementations (doc 01
//!   §2.4(e), MP §6/§8).
//!
//! # Example
//!
//! A minimal contract-conforming decoder (the [`contract`] module is the
//! executable version of these rules):
//!
//! ```
//! use codec_api::prelude::*;
//! use media_types::packet::SharedBytes;
//! use media_types::{GenerationId, Stamp, TrackId};
//!
//! struct NopDecoder {
//!     generation: GenerationId,
//! }
//!
//! impl Decoder for NopDecoder {
//!     fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
//!         self.generation = cfg.generation;
//!         Ok(())
//!     }
//!
//!     fn decode(
//!         &mut self,
//!         packet: Packet,
//!         _out: &mut dyn FrameSink,
//!     ) -> Result<DecodeOutcome, EngineError> {
//!         // Cancellation is generational: stale packets never reach
//!         // decode logic.
//!         match drop_stale_packet(&packet, self.generation) {
//!             Some(outcome) => Ok(outcome),
//!             None => Ok(DecodeOutcome::NeedMoreInput),
//!         }
//!     }
//!
//!     fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
//!         Ok(DecodeOutcome::Drained)
//!     }
//! }
//!
//! struct Discard;
//!
//! impl FrameSink for Discard {
//!     fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
//!         Ok(())
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let current = GenerationId::FIRST.next()?;
//!     let mut decoder = NopDecoder {
//!         generation: GenerationId::FIRST,
//!     };
//!     decoder.configure(&DecoderConfig::new(
//!         CodecDescriptor::new("mock.nop")?,
//!         current,
//!         ThreadBudget::default(),
//!         None,
//!         None,
//!     ))?;
//!
//!     // A packet demuxed under the previous generation is dropped on
//!     // sight and reported as NeedMoreInput with zero frames.
//!     let stale = Packet {
//!         track: TrackId::new(1),
//!         pts: Stamp::Missing,
//!         dts: Stamp::Missing,
//!         duration: None,
//!         data: SharedBytes::from_vec(vec![0x01]),
//!         keyframe: true,
//!         discontinuity: false,
//!         generation: GenerationId::FIRST,
//!     };
//!     let outcome = decoder.decode(stale, &mut Discard)?;
//!     assert_eq!(outcome, DecodeOutcome::NeedMoreInput);
//!     Ok(())
//! }
//! ```

pub mod capability;
pub mod config;
pub mod contract;
pub mod decode;
pub mod encode;
pub mod hw;

// Headline re-exports: flat access to the contract surface.
pub use capability::{
    CodecCapability, DimensionLimits, HwPreference, LevelDescriptor, ProfileDescriptor,
};
pub use config::{DecoderConfig, EncoderConfig, ThreadBudget};
pub use decode::{DecodeOutcome, Decoder, FrameSink, RecoverReason, drop_stale_packet};
pub use encode::{EncodeOutcome, Encoder, PacketSink, drop_stale_frame};
pub use hw::HwAccelerator;

/// Prelude-style alias: the whole contract plus the `media-types`
/// vocabulary its signatures speak, in one `use`.
///
/// `use codec_api::prelude::*;` is sufficient to implement [`Decoder`] or
/// [`Encoder`]; frame-construction detail types (planes, sample formats)
/// come from `media_types::prelude` as usual.
pub mod prelude {
    pub use crate::capability::{
        CodecCapability, DimensionLimits, HwPreference, LevelDescriptor, ProfileDescriptor,
    };
    pub use crate::config::{DecoderConfig, EncoderConfig, ThreadBudget};
    pub use crate::decode::{DecodeOutcome, Decoder, FrameSink, RecoverReason, drop_stale_packet};
    pub use crate::encode::{EncodeOutcome, Encoder, PacketSink, drop_stale_frame};
    pub use crate::hw::HwAccelerator;
    pub use media_types::ids::CodecDescriptor;
    pub use media_types::packet::SharedBytes;
    pub use media_types::{EngineError, ErrorKind, Frame, GenerationId, Packet};
}
