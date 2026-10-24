//! AV1 decode via **rav1d (asm-off)** — card 5.E1, doc 04 §4.4's
//! flagship software video codec.
//!
//! # What landed (and the purity proof that gates it)
//!
//! [`Av1Decoder`] implements the [`codec_api::Decoder`] contract over
//! the rav1d 1.1.0 engine, admitted under the Lead's C5
//! pre-approval with the exact feature pin the record exists for:
//! `default-features = false, features = ["bitdepth_8",
//! "bitdepth_16"]` — **`asm` OFF** (it is default-on and compiles
//! shipped ARM `.S`/x86 nasm through `cc`/`nasm-rs`, doc 04 §4.4).
//! The full evidence trail — sparse-index query, sha256 match,
//! build.rs's `#[cfg(feature = "asm")]` no-op proof, the
//! sabotage build (`CC=/nonexistent-cc`), license and advisory
//! census — is `docs/dependencies/records/rav1d-1.1.0.md`.
//!
//! # Structure
//!
//! - [`obu`] — the minimal OBU temporal-unit framing validator (ours;
//!   typed `av1.obu.*` damage before the engine sees bytes). Sequence
//!   and frame *header parsing is the engine's job (dav1d-lineage
//!   code, battle-tested); we duplicate none of it.
//! - [`decoder`] — the engine seam (the repo's second sanctioned
//!   `unsafe` exception: rav1d 1.1.0's only public surface is its
//!   `dav1d_*` C ABI), the I420 copy-out, pooled output slots, and
//!   the `Decoder` impl.
//!
//! # Delivery plumbing
//!
//! The landed IVF demuxer (`demux::ivf`, card 5.I3b) is the
//! container path: `AV01` fourcc → descriptor `av1` → these packets
//! (one temporal unit each, byte-for-byte passthrough — its own
//! codec-honesty docs anticipated exactly this handoff). The
//! differential suite runs the full demux→decode composition. Ogg:
//! the Ogg demuxer carries Vorbis/Opus/FLAC/Theora/Kate/FFV1 lanes
//! only — **no AV1 mapping exists there today** (no `Av1` in its
//! codec table, verified this card), so there is no Ogg-AV1 plumbing
//! to wire; when an Ogg-AV1 mapping is ever admitted (Chromium
//! proposal-class, not shipped), the same one-TU-per-packet contract
//! applies. Matroska/WebM `AV01` tracks and MP4 `av01` follow in
//! their container cards (the same AV1 decoder consumes their
//! re-framed TUs unchanged).
//!
//! # Honest v0 boundary
//!
//! 8-bit 4:2:0 → [`media_types::PixelFormat::I420`]; 10/12-bit,
//! 4:2:2/4:4:4 and monochrome are typed `Unsupported` (`av1.bitdepth`
//! / `av1.layout`) — a 16-bit planar media-types vocabulary is the
//! recorded follow-up. asm-off costs decode speed (doc 04 §4.4's
//! benchmark ask is a doc 12 card); correctness is what this card
//! proves, differentially against ffmpeg.
//!
//! # Example
//!
//! ```
//! use codec_api::prelude::*;
//! use codecs::video::av1::Av1Decoder;
//!
//! // The descriptor gate speaks the IVF demuxer's vocabulary.
//! let mut decoder = Av1Decoder::new();
//! let generation = GenerationId::FIRST
//!     .next()
//!     .unwrap_or_else(|e| unreachable!("{e}"));
//! // A wrong codec is a typed refusal, never a mis-decode.
//! let refused = decoder.configure(&DecoderConfig::new(
//!     media_types::ids::CodecDescriptor::new("vp9")
//!         .unwrap_or_else(|e| unreachable!("{e}")),
//!     generation,
//!     ThreadBudget::default(),
//!     None,
//!     None,
//! ));
//! assert!(refused.is_err());
//! ```

pub mod decoder;
pub mod obu;

pub mod fuzz;

pub use decoder::Av1Decoder;
pub use obu::{
    MAX_OBUS_PER_TU, OBU_FRAME, OBU_FRAME_HEADER, OBU_METADATA, OBU_PADDING,
    OBU_REDUNDANT_FRAME_HEADER, OBU_SEQUENCE_HEADER, OBU_TEMPORAL_DELIMITER, OBU_TILE_GROUP,
    OBU_TILE_LIST, ObuInfo, TemporalUnit,
};
