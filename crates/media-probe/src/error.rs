//! Typed error constructors (the input/demux pattern): every probe
//! failure is minted here with stage
//! [`PipelineStage::Probe`](media_types::PipelineStage::Probe) and a
//! stable dotted context code. The taxonomy itself is media-types' and
//! is never redefined.

use media_types::error::{ErrorContext, ErrorKind, FormatInfo, LimitKind};
use media_types::{EngineError, PipelineStage};

use crate::config::MAX_LOOKAHEAD_CEILING;
use crate::report::{ProbeHit, SourceMeta};

/// No signature reached the configured minimum score (doc 03 §4.2(d)).
///
/// The error carries whatever is known (MP §8 — never fail silently,
/// never present a guess as fact): the best-scoring hit's format name
/// when any signature matched at all, the first bytes that were
/// actually sniffed (capped at 16 by the L0 `Display`), and the
/// source's extension/size hints on the error's source chain.
pub fn no_demuxer(best: Option<&ProbeHit>, window: &[u8], meta: &SourceMeta) -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: best.map(|hit| hit.format.as_str().to_owned()),
                sniffed: window[..window.len().min(16)].to_vec(),
            }),
        },
        ErrorContext::new("probe.no-demuxer"),
    )
    .with_source(ProbeHints {
        extension: meta.extension.clone(),
        size: meta.size,
    })
}

/// A configured lookahead of zero bytes (nothing can be probed).
pub fn lookahead_zero() -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new("probe.lookahead-zero"),
    )
}

/// A configured lookahead above the doc 03 §4.2 hard cap — the probe
/// depth budget itself.
pub fn lookahead_over_ceiling(attempted: usize) -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::ResourceLimit {
            limit: LimitKind::ProbeDepth,
            budget: MAX_LOOKAHEAD_CEILING as u64,
            attempted: attempted as u64,
        },
        ErrorContext::new("probe.lookahead-over-ceiling"),
    )
}

/// A configured minimum score outside 0..=100.
pub fn min_score_out_of_range(attempted: u8) -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(attempted.to_string()),
                sniffed: Vec::new(),
            }),
        },
        ErrorContext::new("probe.min-score-out-of-range"),
    )
}

/// Probing a stream that cannot rewind (v0: probing is defined over
/// seekable sources so the window read is non-consuming, doc 03
/// §4.2(e); live-source probing is the `streaming` crate's Phase-6
/// peek-buffer design).
pub fn stream_not_rewindable(scheme: &str) -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(scheme.to_owned()),
                sniffed: Vec::new(),
            }),
        },
        ErrorContext::new("probe.stream-not-rewindable"),
    )
}

/// A track-hint codec string that violates the L0 descriptor grammar.
pub fn hint_codec_invalid() -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new("probe.hint-codec-invalid"),
    )
}

/// A format id string outside the grammar (the extension path).
pub fn format_id_invalid() -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new("probe.format-id.invalid"),
    )
}

/// The no-hit error's source-chain leaf: the advisory hints a caller
/// gave the probe (the demux DemuxSourceInfo precedent — structured
/// data, never secrets; extensions and sizes are not credentials).
#[derive(Debug)]
struct ProbeHints {
    extension: Option<String>,
    size: Option<u64>,
}

impl std::fmt::Display for ProbeHints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ext = self.extension.as_deref().unwrap_or("none");
        let size = match self.size {
            Some(n) => n.to_string(),
            None => "unknown".to_owned(),
        };
        write!(f, "probe hints: extension={ext}, size={size}")
    }
}

impl std::error::Error for ProbeHints {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{Confidence, FormatId, ProbeHit, TrackHint};
    use media_types::ids::MediaKind;

    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    #[test]
    fn no_demuxer_carries_sniffed_bytes_and_hints() {
        let meta = SourceMeta::new(Some(4096), Some("mp4"));
        let window = vec![0xAB; 300];
        let e = no_demuxer(None, &window, &meta);
        assert_eq!(e.stage, PipelineStage::Probe);
        assert_eq!(e.context.code, "probe.no-demuxer");
        match &e.kind {
            ErrorKind::Unsupported { format } => {
                let info = match format {
                    Some(i) => i,
                    None => unreachable!("format info present"),
                };
                assert!(info.name.is_none());
                assert_eq!(info.sniffed.len(), 16, "sniffed bytes capped at 16");
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
        assert!(std::error::Error::source(&e).is_some());
        assert!(e.to_string().contains("probe.no-demuxer"));
    }

    #[test]
    fn no_demuxer_names_the_best_subthreshold_hit() {
        let hit = ProbeHit {
            format: FormatId::mp4(),
            score: 50,
            track_hints: Vec::new(),
            prefix_consumed: 16,
        };
        let e = no_demuxer(Some(&hit), b"not-really-mp4", &SourceMeta::new(None, None));
        match &e.kind {
            ErrorKind::Unsupported { format } => {
                let info = match format {
                    Some(i) => i,
                    None => unreachable!("format info present"),
                };
                assert_eq!(info.name.as_deref(), Some("mp4"));
                assert_eq!(info.sniffed, b"not-really-mp4".to_vec());
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
    }

    #[test]
    fn limit_error_reports_the_doc_ceiling() {
        let e = lookahead_over_ceiling(2 * 1024 * 1024);
        match e.kind {
            ErrorKind::ResourceLimit {
                limit,
                budget,
                attempted,
            } => {
                assert_eq!(limit, LimitKind::ProbeDepth);
                assert_eq!(budget, 1_048_576);
                assert_eq!(attempted, 2_097_152);
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
        assert_eq!(
            e.to_string(),
            "[probe] resource limit: probe depth (budget 1048576, attempted 2097152) \
             (probe.lookahead-over-ceiling)"
        );
    }

    #[test]
    fn confidence_and_hit_construction_stay_in_range() {
        // The error constructors take validated vocabulary; the report
        // module owns range enforcement (pinned there). This pins the
        // constructor plumbing compiles against the real types.
        let hit = ProbeHit {
            format: FormatId::avi(),
            score: 70,
            track_hints: vec![ok(TrackHint::new(MediaKind::Video, "mjpeg"))],
            prefix_consumed: 12,
        };
        let c = match Confidence::from_score(hit.score) {
            Some(c) => c,
            None => unreachable!("70 <= 100"),
        };
        assert!(c.meets(60));
    }
}
