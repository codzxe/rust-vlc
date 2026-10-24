//! # demux::es — raw elementary-stream file access (card 5.I3a)
//!
//! The headerless "container" family of doc 04 §2.2's registry: files
//! that are one bare codec bitstream with no container grammar. Four
//! lanes, one honest posture each:
//!
//! - [`h264`] — H.264 Annex-B (`.264`): start-code scan →
//!   NAL-type-aware access-unit boundaries (IDR/SPS/PPS/SEI-aware;
//!   the `first_mb_in_slice` one-bit probe), via [`annexb`].
//! - [`hevc`] — HEVC Annex-B (`.265`): the same engine with VPS/SPS/
//!   PPS (types 32/33/34) awareness and the
//!   `first_slice_segment_in_pic_flag` probe.
//! - [`mp3`] — raw MPEG-audio (`.mp3`/`.mp2`): ID3v2 skip + frame
//!   walk over the published ISO tables (a thin local twin of the
//!   codecs-side mp3 module's tables — layer ordering forbids sharing
//!   the crate; the decode-domain surfaces stay with the decoder).
//! - [`aac`] — raw AAC (`.aac`): the landed [`crate::adts`]
//!   demuxer, consumed as-is (this module is the registry surface).
//!
//! ## House rules
//!
//! Std-only, zero external dependencies (C5); no `unsafe`; no
//! `unwrap`/`expect`/`panic!` in non-test code; bounded memory
//! (MP §12): every scan is windowed, every pool slot is
//! ceiling-checked, recovery budgets are typed refusals. Timing
//! honesty: raw ES carries no stamps — video packets carry
//! [`Stamp::Missing`](media_types::time::Stamp::Missing) (the L0
//! vocabulary's own raw-stream case) and audio lanes use exact sample
//! arithmetic, never a guessed bit rate.
//!
//! ## Error paths (typed, all unit-pinned)
//!
//! Empty/short sources refuse at construction (the shared
//! [`SourceReader`](crate::avi::riff::SourceReader) contract); a
//! no-sync window is the typed `Unsupported`-after-scan
//! (`demux.es.no-sync`) carrying the sniffed bytes; a frame that
//! overruns end-of-stream is the typed sticky truncation
//! (`demux.es.truncated`); contract-order violations are
//! `demux.es.not-started`.

pub mod aac;
pub mod annexb;
pub mod error;
pub mod h264;
pub mod hevc;
pub mod mp3;

#[cfg(test)]
mod tests;
