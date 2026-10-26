//! VP9 decode via **rusty_vp9** — card 6.M-vp9, the M-wave VP9
//! admission (a top blocked-format, YouTube/WebM).
//!
//! # What landed (and the purity proof that gates it)
//!
//! [`Vp9Decoder`] implements the [`codec_api::Decoder`] contract over
//! the rusty_vp9 0.1.3 engine, admitted under the Lead's decision
//! with the exact pin the record exists for:
//! `rusty_vp9 = "=0.1.3"`, **default features (an empty set)** — the
//! `prometheus-telemetry` feature stays OFF. The full evidence
//! trail — sparse-index query, sha256 match, vendored-source scans,
//! the six-lane bit-exactness differential against ffmpeg
//! `libvpx-vp9`, and the hostile-input no-panic battery — is
//! `docs/dependencies/records/rusty_vp9-0.1.3.md`.
//!
//! # Structure
//!
//! - [`decoder`] — the engine seam (all **safe** Rust: unlike the
//!   rav1d C-ABI seam of [`super::av1`], rusty_vp9's decode surface
//!   is safe end to end), the pooled tight-I420 copy-out, the v0
//!   output boundary and the `Decoder` impl.
//! - [`error`] — the typed `vp9.*` error taxonomy mapping onto the
//!   public [`media_types::EngineError`] kinds.
//!
//! # Delivery plumbing
//!
//! The landed IVF demuxer (`demux::ivf`) is the container path:
//! `VP90` fourcc → descriptor `vp9` → these packets (one coded frame
//! or superframe each, byte-for-byte passthrough). Matroska/WebM
//! `V_VP9` tracks map to the same descriptor — the end-to-end mkv
//! and ivf decode lanes are the parallel container cards' surface,
//! reconciled by the Lead.
//!
//! # Honest v0 boundary
//!
//! 8-bit 4:2:0 → [`media_types::PixelFormat::I420`]; 10/12-bit
//! (profile 2/3) and 4:2:2/4:4:4 (profile 1) are typed `Unsupported`
//! (`vp9.bitdepth` / `vp9.layout`) — a 16-bit planar media-types
//! vocabulary is the av1 module's recorded follow-up, shared here.
//! The engine decodes those profiles fine (the record's lane F
//! measured 10-bit bit-exact at the engine level); the wrapper
//! refuses to reinterpret what it cannot name. The engine is
//! single-threaded (record honest gap).
//!
//! # Example
//!
//! ```
//! use codec_api::prelude::*;
//! use codecs::video::vp9::Vp9Decoder;
//!
//! // The descriptor gate speaks the demuxers' vocabulary.
//! let mut decoder = Vp9Decoder::new();
//! let generation = GenerationId::FIRST
//!     .next()
//!     .unwrap_or_else(|e| unreachable!("{e}"));
//! // A wrong codec is a typed refusal, never a mis-decode.
//! let refused = decoder.configure(&DecoderConfig::new(
//!     media_types::ids::CodecDescriptor::new("av1")
//!         .unwrap_or_else(|e| unreachable!("{e}")),
//!     generation,
//!     ThreadBudget::default(),
//!     None,
//!     None,
//! ));
//! assert!(refused.is_err());
//! ```

pub mod decoder;
mod error;

pub use decoder::Vp9Decoder;
