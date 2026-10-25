//! Raw AAC file access (card 5.I3a): the `.aac` extension is the
//! landed ADTS/ADIF demuxer (card 5.C2b) — consumed, not duplicated.
//!
//! The framing truth is identical (ADTS frames are self-delimited;
//! ADIF carries one CBR header), so this module is the registry
//! surface: it names the format, documents the extensions, and points
//! at the real implementation. Every behavior note (ID3 skip, bounded
//! resync, the truncation divergence, ADIF VBR/multi-program
//! refusals) lives in [`crate::adts`].

/// File extensions this demuxer claims (the R3 registry row; the
/// landed implementation claims the superset `aac`/`adts`/`adif`).
pub const EXTENSIONS: &[&str] = &["aac"];

/// The raw-AAC demuxer: the landed [`crate::adts::AdtsDemuxer`].
pub use crate::adts::AdtsDemuxer as RawAacDemuxer;

/// The probe: the landed implementation's own.
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    crate::adts::probe(header)
}

/// Open a raw AAC stream (`.aac`): the landed constructor.
///
/// # Errors
/// The landed implementation's typed `Unsupported` set (empty /
/// unseekable / unknown-length sources; no ADTS syncword or `ADIF`
/// magic within the open-scan window).
pub fn open(
    stream: Box<dyn plugin_api::SourceStream>,
) -> Result<RawAacDemuxer, media_types::EngineError> {
    RawAacDemuxer::new(stream)
}
