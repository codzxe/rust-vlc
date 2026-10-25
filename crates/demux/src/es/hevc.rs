//! The HEVC raw elementary-stream demuxer (card 5.I3a): the `.265`
//! file-access surface over the shared Annex-B engine in
//! [`super::annexb`]. See that module's docs for the access-unit
//! boundary rule (VPS/SPS/PPS types 32/33/34-aware,
//! `first_slice_segment_in_pic_flag` probe) and the corpus-pinned
//! packet shape.

use super::annexb::{AnnexbCodec, AnnexbVideoDemuxer};

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["h265", "265", "hevc"];

/// The probe: do the leading bytes carry an Annex-B start code with a
/// plausible HEVC NAL type (VCL 0..=31 or parameter sets 32..=34)?
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    super::annexb::probe(AnnexbCodec::Hevc, header)
}

/// Open a raw HEVC Annex-B stream (`.265`).
///
/// # Errors
/// Typed `Unsupported` (stage `Open`) for unknown-length, unseekable
/// or empty sources; the same for a stream whose open-scan window
/// holds no start code / plausible NAL.
pub fn open(
    stream: Box<dyn plugin_api::SourceStream>,
) -> Result<AnnexbVideoDemuxer, media_types::EngineError> {
    AnnexbVideoDemuxer::new(AnnexbCodec::Hevc, stream)
}
