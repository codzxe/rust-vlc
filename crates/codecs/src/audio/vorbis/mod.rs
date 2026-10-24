//! Vorbis audio decoder (card 5.B2; doc 04 §3.1 row "Vorbis", doc 04
//! §2.2 row "Vorbis (in Ogg)" — the Phase-5 lossy core's Ogg native).
//!
//! # Format slice (what this module owns)
//!
//! **Vorbis-in-Ogg** (the `\x01vorbis` byte stream): the three-header
//! chain ([`headers`] — identification geometry, the split, the
//! `extra_data` seam), Vorbis comment exposure ([`comments`] —
//! our own parser, the card's metadata AC), block decode through the
//! pinned `symphonia-codec-vorbis` 0.6.1 engine ([`decoder`], the
//! dependency decision and its purity record documented there), and
//! the granule arithmetic ([`seek`] — the sample clock, the
//! shift measurement, the landing rules). Native-Vorbis-in-Matroska
//! reuses the same header chain verbatim (the `extra_data` seam is
//! container-agnostic); only the packet framing differs, which is the
//! demuxer's business.
//!
//! # Module map
//!
//! - [`VorbisDecoder`] — the codec-api [`Decoder`](codec_api::Decoder)
//!   implementation (negotiation + damage model documented on
//!   [`decoder`]).
//! - [`headers`] — [`is_vorbis_ident`], [`parse_identification`],
//!   [`VorbisHeaders`] (the chain split).
//! - [`comments`] — [`VorbisComments`] (vendor + user comments,
//!   case-insensitive lookup, repeatable tags).
//! - [`seek`] — [`measure_shift`], [`landing_first_sample`],
//!   [`sample_to_nanos`]/[`nanos_to_sample`].
//! - [`decode_block`] — the free surface: one packet + header chain →
//!   one decoded block with no contract state (the 5.A1
//!   `decode_frame` pattern; the [`fuzz`] entry).
//! - [`fuzz`] — the `fuzz_codec_vorbis` harness (`+FB`; wiring status
//!   documented there, honestly).
//!
//! # Conventions (crate-wide, unchanged)
//!
//! No `unsafe` on this module's own paths (the engine's crates are
//! `#![forbid(unsafe_code)]` / zero-unsafe per the dependency
//! record); no panic on any input — every decode/parse path is
//! bounds-checked, damage is a typed [`EngineError`] and the decoder
//! keeps decoding after a damaged packet with a one-block overlap
//! scar (the honest MDCT isolation boundary, doc 03 §4.5(d)), never a
//! crash.
//!
//! # Differential posture (the card AC)
//!
//! Vorbis is lossy: independent decoders are not sample-identical in
//! general, so the differential against ffmpeg runs on the same
//! granule-aligned window and asserts (1) the exact sample count
//! (the granule timeline, delay/padding trimmed — matching ffmpeg's
//! own decode length) and (2) a bounded per-sample float delta, with
//! our PCM MD5 pinned as the hard regression (the 5.A3 MP3 posture;
//! `tests/vorbis_bitexact.rs` records the verdicts, and a digital
//! **silence** fixture — where every conforming decoder must emit
//! exact zeros — gives the true byte-exact MD5 lane). The VLC
//! differential hook (`tests/vorbis_vlc_differential.rs`) is
//! registered and honestly pending infra (the 5.A1 FLAC pattern).

pub(crate) mod comments;
pub(crate) mod decoder;
pub(crate) mod error;
/// Fuzz harness (doc 04 §8.5 registry name `fuzz_codec_vorbis`):
/// `pub` because the registered target file and the in-tree survival
/// tests call it from outside the crate (the 2.B2/2.B4/5.A1
/// precedent).
pub mod fuzz;
pub(crate) mod headers;
pub(crate) mod seek;

pub use comments::VorbisComments;
pub use decoder::{DecodedVorbis, VorbisDecoder, decode_block};
pub use headers::{Identification, VorbisHeaders, is_vorbis_ident, parse_identification};
pub use seek::{
    cold_block_emission, join_emission, landing_first_sample, measure_shift, nanos_to_sample,
    sample_to_nanos,
};
