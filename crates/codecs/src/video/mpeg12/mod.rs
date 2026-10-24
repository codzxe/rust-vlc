//! MPEG-1 (ISO/IEC 11172-2) / MPEG-2 (ITU-T H.262, ISO/IEC
//! 13818-2) video **sequence layer** (card 5.H1, doc 04 §4 row
//! "MPEG-1/2 video": the [VERIFY-P0] pure-Rust availability
//! question, answered in `docs/formats/video-mpeg12.md`).
//!
//! # The honesty boundary (the h264::parser precedent, verbatim
//! scope)
//!
//! There is **no software MPEG-1/2 decode here** — this module
//! parses and classifies bitstream structure only: start codes,
//! sequence headers, GOP/picture headers, the MPEG-2 extension
//! family (sequence/picture-coding), and slice headers with
//! macroblock addressing. Software decode remains `blocked` in the
//! feature matrix: no validated pure-Rust engine exists (the
//! 5.H1 crates.io survey — two pre-1.0 candidates below the
//! admission bar, both recorded), and a self-build is an L-sized
//! VLC/ffmpeg-class project queued Phase 6+. The typed refusal the
//! codec-api registry surfaces is [`error::decode_refusal`]
//! (`mpeg12.decode.blocked`); no `Decoder` impl lives under this
//! module tree.
//!
//! # What the sequence layer is for
//!
//! - probe/track-info facts: resolution, frame rate,
//!   profile/level, chroma, progressive/interlaced, GOP timecode,
//!   picture-type census, motion-vector syntax presence;
//! - the hw-feed seam: ADR-001's Phase-7 adapters will hand
//!   hardware decoders exactly these parsed structures;
//! - the future self-build's substrate: the start-code walk and
//!   header geometry are the front half of any conforming decoder.
//!
//! # Module map
//!
//! - [`startcode`] — the `00 00 01` scanner (Table 6-1
//!   classification, zero-copy borrowed units).
//! - [`bitreader`] — the MSB-first header reader (the spec's
//!   `uimsbf`/`bslbf` convention).
//! - [`headers`] — sequence/GOP/picture/slice syntax + extension
//!   headers + the Table B-1 macroblock-addressing VLC.
//! - [`facts`] — [`facts::Mpeg12Facts::scan`]: the one-pass
//!   total stream scan probe/track-info consume, plus the
//!   descriptor gate ([`facts::descriptor_gate`]).
//! - [`error`] — the typed parse failure vocabulary and the
//!   decode-refusal constructor.
//!
//! # Damage model
//!
//! Individual header parsers are total in the H.264 sense: typed
//! `Result`, no panics, no damage smoothing. The stream-level scan
//! is damage-**tolerant**: a bad header is counted and the walk
//! continues at the next start code (broadcast-capture posture);
//! a buffer with no start codes yields an empty fact set, never a
//! guess. Fuzz-lite pins this in `facts`; the registered
//! `fuzz_codec_mpeg2` target (doc 04 §8.5) formalizes it later.

pub mod bitreader;
pub mod error;
pub mod facts;
pub mod headers;
pub mod startcode;
