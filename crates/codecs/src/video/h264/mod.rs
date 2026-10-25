//! H.264/AVC **bitstream parser + hardware decode-path seam** (the
//! parser is card 5.F1, doc 04 §4.2's "what we can self-build cheaply
//! regardless": NAL/SPS/PPS, slice headers, SEI, Annex-B and
//! AVC-length-prefixed unwrapping; the decode-path integration is card
//! 5.F2).
//!
//! # The honesty boundary (doc 04 §4.2; ADR-001 §1)
//!
//! There is **no software H.264 decode here** — the module parses and
//! classifies bitstream structure, and routes the parsed result to a
//! **hardware session** ([`hw`]). Software decode remains
//! `blocked`/`hw-only` in the feature matrix until ADR-001's
//! conformance-gated software track produces a validated decoder. The
//! one `Decoder` impl under this tree ([`decode::H264Decoder`]) feeds
//! hardware adapters only: with no accelerator negotiated it fails
//! configure with the typed `h264.hw.required` `Unsupported` error —
//! the honest matrix row, never a fake success, never a silent skip.
//!
//! # Structure (wave-4 split, unified at Lead merge; F2 additions)
//!
//! [`parser`] owns the whole surface: NAL-unit framing (both the
//! Annex-B start-code form and the AVC length-prefixed form incl. the
//! `avcC` record — card 5.F1a), the SPS/PPS parameter sets (5.F1a),
//! and slice headers + SEI (sibling card 5.F1b). The halves meet at
//! the `parser::slice` context structs ([`parser::slice::SliceSpsInfo`],
//! [`parser::slice::SlicePpsInfo`]) — minimal seams that F1a's parsed
//! SPS/PPS populate. Card 5.F2 adds the consumption side:
//! [`hw`] (the adapter seam: [`hw::H264HwDecoder`] session contract,
//! [`hw::H264HwBridge`] plug, [`hw::H264HwPath`] consumption point)
//! and [`decode`] (the codec-api [`Decoder`](codec_api::Decoder)
//! adapter: extradata configure, §7.4.1.2.4 access-unit assembly,
//! typed-unsupported fallback).
//!
//! # Damage model
//!
//! Every entry point is total: any byte string yields either a parsed
//! structure or a typed [`parser::H264Error`] — no panics, no
//! unbounded work (the `fuzz_codec_h264` target's future guarantee,
//! doc 04 §8.5). The decode path inherits the discipline: in-stream
//! damage is skipped-and-counted recovery
//! ([`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered)),
//! never a panic.

pub mod decode;
pub mod fuzz;
pub mod hw;
pub mod parser;

pub use decode::H264Decoder;
pub use hw::{H264HwBridge, H264HwDecoder, H264HwPath};
