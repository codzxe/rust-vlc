//! Subtitle decoders (FX9-A04, wave FX9; FIX-PLAN §5 F3.1): bitmap
//! subtitle formats behind the [`codec_api::Decoder`] contract,
//! reduced to
//! [`Frame::Subtitle`](media_types::Frame::Subtitle) overlay cues.
//!
//! The crate's first vertical here is PGS (HDMV, Blu-ray) — the
//! embedded MPEG-TS subtitle track lane whose descriptor the TS
//! demuxer already mints (`"pgssub"`, `demux::ts::TsStream` for
//! stream type `0x90`). The reduce path is deliberately thin: the
//! `subtitles` crate (L2) owns the format grammar (epoch state
//! machine, RLE + palette decode, the compositing-cue builder) and
//! this lane only adapts it to the engine's packet → frame contract —
//! the same in-house-engine division the audio/video modules use
//! (grammar in a lower layer, the [`Decoder`] impl here).
//!
//! Layer note: this module is the reason `codecs` depends on
//! `subtitles` (L3 → L2, downward; sanctioned in FX9-A04 and recorded
//! in `boundaries.toml`'s `[crates.codecs]` row).

pub mod pgs;
