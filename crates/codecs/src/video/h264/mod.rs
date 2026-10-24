//! H.264/AVC **bitstream parser** (card 5.F1, doc 04 §4.2's "what we
//! can self-build cheaply regardless": NAL/SPS/PPS, slice headers,
//! SEI, Annex-B and AVC-length-prefixed unwrapping).
//!
//! # The honesty boundary (doc 04 §4.2, verbatim scope)
//!
//! There is **no software H.264 decode here** — this module parses and
//! classifies bitstream structure only. Software decode remains
//! `blocked`/`hw-only` in the feature matrix until ADR-001's hardware
//! path (Phase 7 adapters feed hardware decoders from these parsed
//! structures) or a spec-driven decoder project is approved. No
//! `Decoder` impl lives under this module tree.
//!
//! # Structure (wave-4 split, unified at Lead merge)
//!
//! [`parser`] owns the whole surface: NAL-unit framing (both the
//! Annex-B start-code form and the AVC length-prefixed form incl. the
//! `avcC` record — card 5.F1a), the SPS/PPS parameter sets (5.F1a),
//! and slice headers + SEI (sibling card 5.F1b). The halves meet at
//! the `parser::slice` context structs ([`parser::slice::SliceSpsInfo`],
//! [`parser::slice::SlicePpsInfo`]) — minimal seams that F1a's parsed
//! SPS/PPS populate.
//!
//! # Damage model
//!
//! Every entry point is total: any byte string yields either a parsed
//! structure or a typed [`parser::H264Error`] — no panics, no
//! unbounded work (the `fuzz_codec_h264` target's future guarantee,
//! doc 04 §8.5).

pub mod parser;
