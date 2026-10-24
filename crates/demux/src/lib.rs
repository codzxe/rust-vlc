//! # demux — container demultiplexers (card 2.B2; doc 03 §4.3, doc 01
//! §2.2 registry row: layer L2)
//!
//! v0 carries the AVI 1.0 demuxer of the first vertical (Lead decision
//! D-018, variant A: AVI + MJPEG video + PCM audio) in [`avi`]. The
//! crate implements the landed [`plugin_api::Demuxer`] contract over
//! any seekable [`plugin_api::SourceStream`] — the `input` crate's file
//! access is the first vertical's byte source, consumed at the trait
//! seam (no compile-time edge, the 2.C4 note on doc 13's Consumes
//! lists).
//!
//! ## Module map
//!
//! - [`avi`] — the demuxer itself: RIFF chunk walk, `hdrl`/`strl`
//!   track discovery, `idx1` seek table, MJPEG+PCM packet extraction,
//!   seek-to-time.
//! - [`wav`] — the RIFF/WAVE demuxer (card 5.A2): `fmt ` (plain /
//!   extensible / float), `fact`, `data` PCM framing, RF64/BW64
//!   `ds64`, LIST/INFO, odd-chunk word-align.
//! - [`aiff`] — the AIFF/AIFC demuxer (card 5.A2): COMM (incl. the
//!   extended-80-bit float sample rate), SSND, skip-tolerant chunks.
//! - [`error`] — typed error constructors at stage
//!   [`PipelineStage::Demux`](media_types::PipelineStage::Demux); the
//!   *taxonomy* is media-types' (never redefined here).
//!
//! ## House rules (workspace-wide, enforced by lints + review)
//!
//! Std-only, zero external dependencies (C5); no `unsafe`; no
//! `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!`/`dbg!` in
//! non-test code; bounded memory (MP §12): every read is
//! chunk-bounded and declared sizes are ceiling-checked before they are
//! trusted into an allocation.

pub mod aiff;
pub mod avi;
pub mod error;
pub mod ogg;
pub mod wav;
