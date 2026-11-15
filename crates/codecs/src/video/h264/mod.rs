//! H.264/AVC **bitstream parser + decode paths** (the parser is card
//! 5.F1, doc 04 §4.2's "what we can self-build cheaply regardless":
//! NAL/SPS/PPS, slice headers, SEI, Annex-B and AVC-length-prefixed
//! unwrapping; the decode-path integration is card 5.F2).
//!
//! # The decode paths (ADR-001, updated 2026-11-15)
//!
//! Two decode paths live under this tree:
//!
//! * [`decode::H264Decoder`] — the **hardware** path: parser →
//!   access-unit assembler → hardware session ([`hw`]). With no
//!   accelerator negotiated it fails configure with the typed
//!   `h264.hw.required` `Unsupported` error (the honest matrix row,
//!   never a fake success, never a silent skip).
//! * [`sw::H264SwDecoder`] — the **software** path (`codec-h264-sw`,
//!   default-on): the Lead-admitted `rust_h264` 0.6.1 engine behind
//!   the same contract, fed through the in-house parser's framing/
//!   EPB bridge with the project's damage accounting. Admission
//!   record: `docs/dependencies/records/rust_h264-0.6.1.md` (built on
//!   the issue-#2 audits); differential evidence:
//!   `tests/h264_sw_differential.rs` (bit-exact vs ffmpeg 7.1.5 on
//!   six lanes, both entry forms). The `blocked` era of the
//!   `codec-video-h264-software` matrix row is closed.
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
//! typed-unsupported fallback). The software track (2026-11-15) adds
//! [`sw`] behind `codec-h264-sw`.
//!
//! # Damage model
//!
//! Every entry point is total: any byte string yields either a parsed
//! structure or a typed [`parser::H264Error`] — no panics, no
//! unbounded work (the `fuzz_codec_h264` target's future guarantee,
//! doc 04 §8.5). Both decode paths inherit the discipline: in-stream
//! damage is skipped-and-counted recovery
//! ([`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered)),
//! never a panic.

pub mod decode;
pub mod fuzz;
pub mod hw;
pub mod parser;
#[cfg(feature = "codec-h264-sw")]
pub mod sw;

pub use decode::H264Decoder;
pub use hw::{H264HwBridge, H264HwDecoder, H264HwPath};
#[cfg(feature = "codec-h264-sw")]
pub use sw::H264SwDecoder;
