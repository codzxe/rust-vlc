//! The H.264 raw elementary-stream demuxer (card 5.I3a): the `.264`
//! file-access surface over the shared Annex-B engine in
//! [`super::annexb`]. See that module's docs for the access-unit
//! boundary rule (IDR/SPS/PPS/SEI-aware, `first_mb_in_slice` probe)
//! and the corpus-pinned packet shape.

use super::annexb::{AnnexbCodec, AnnexbVideoDemuxer};

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["h264", "264", "avc"];

/// The probe: do the leading bytes carry an Annex-B start code with a
/// plausible H.264 NAL type (VCL, SEI, SPS, PPS or AUD)?
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    super::annexb::probe(AnnexbCodec::H264, header)
}

/// Open a raw H.264 Annex-B stream (`.264`).
///
/// # Errors
/// Typed `Unsupported` (stage `Open`) for unknown-length, unseekable
/// or empty sources; the same for a stream whose open-scan window
/// holds no start code / plausible NAL.
pub fn open(
    stream: Box<dyn plugin_api::SourceStream>,
) -> Result<AnnexbVideoDemuxer, media_types::EngineError> {
    AnnexbVideoDemuxer::new(AnnexbCodec::H264, stream)
}
