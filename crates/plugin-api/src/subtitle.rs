//! The subtitle-parser contract (doc 06 §1.2 kind 10): subtitle
//! format → timed [`SubtitleCue`]s. Impl crate: `subtitles` (L2).
//!
//! Instance scope: per playback session (§1.3). Input is the raw
//! subtitle file bytes the engine hands over (a sidecar file, an
//! embedded track's packets, or an external-load command); output is
//! cue batches through a caller-owned [`SubtitleSink`]. Styled and
//! bitmap payloads are Phase 5 vocabulary extensions (doc 04; the L0
//! `SubtitleCue` is plain text in v1) — the trait speaks L0 types
//! only.

use media_types::frame::SubtitleCue;
use media_types::packet::SharedBytes;
use media_types::{EngineError, PipelineStage};

/// A subtitle format parser (doc 06 §1.2 row 10).
///
/// # Calling context
///
/// Worker thread ([`Send`]); bounded work per call — **one chunk per
/// [`next_cues`](SubtitleParser::next_cues)** (the engine chunks the
/// input), never an internal parse-the-whole-file loop; panics are
/// contract violations. Cancellation is generation staleness: cue
/// batches carry the demux generation of the session segment they
/// belong to.
pub trait SubtitleParser: Send {
    /// Feed one chunk of the subtitle document (the engine chunks;
    /// `None` = end of document, the flush signal).
    ///
    /// Returns the cues that became complete from this chunk (a
    /// subtitle line typically needs its end-timestamp line before it
    /// is complete — early returns are correct, not lossy).
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Demux`]: subtitle
    /// parsing is a demux-adjacent parse step) when the document is
    /// not the claimed format or fails unrecoverably; recoverable
    /// damage skips the cue and is reported as data (see
    /// [`Recovered`](ParseStatus::Recovered)).
    fn next_cues(
        &mut self,
        chunk: Option<SharedBytes>,
        sink: &mut dyn SubtitleSink,
    ) -> Result<ParseStatus, EngineError>;
}

/// What a subtitle parse step accomplished (the shared
/// bounded-recovery vocabulary; MP §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseStatus {
    /// The call accepted the chunk; cues (if any) went to the sink.
    Accepted,
    /// The document is fully parsed — the flush endpoint.
    Drained,
    /// The parser overcame damage (resynchronized past it); `skipped`
    /// counts the cues lost.
    Recovered {
        /// Cues skipped by the resynchronization.
        skipped: u32,
        /// Why (observability).
        reason: &'static str,
    },
}

/// Where parsed cues go: the engine's subtitle channel, created and
/// consumed on the parse worker thread (no `Send` bound — the
/// codec-api sink precedent).
pub trait SubtitleSink {
    /// Take one parsed cue.
    ///
    /// Returning `Err` aborts the in-flight call: the parser
    /// propagates or chains the error and remains usable.
    ///
    /// # Errors
    /// The sink's own typed error, expressed as an [`EngineError`].
    fn accept(&mut self, cue: SubtitleCue) -> Result<(), EngineError>;
}

/// Typed subtitle error: the bytes are not the claimed format.
pub fn not_this_format(sniffed: &[u8]) -> EngineError {
    let cap = &sniffed[..sniffed.len().min(16)];
    EngineError::new(
        PipelineStage::Demux,
        media_types::ErrorKind::Unsupported {
            format: Some(media_types::error::FormatInfo {
                name: None,
                sniffed: cap.to_vec(),
            }),
        },
        media_types::error::ErrorContext::new("plugin.subtitle.not-this-format"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::SubtitleCue;
    use media_types::time::{Stamp, TimeSpan};

    /// A parser for a toy line format: each line is `text|end_ticks`.
    struct LineParser {
        pending: Option<String>,
        done: bool,
    }

    impl SubtitleParser for LineParser {
        fn next_cues(
            &mut self,
            chunk: Option<SharedBytes>,
            sink: &mut dyn SubtitleSink,
        ) -> Result<ParseStatus, EngineError> {
            match chunk {
                None => {
                    self.done = true;
                    Ok(ParseStatus::Drained)
                }
                Some(bytes) => {
                    let text = match self.pending.take() {
                        Some(p) => format!("{p}{}", String::from_utf8_lossy(bytes.as_slice())),
                        None => String::from_utf8_lossy(bytes.as_slice()).into_owned(),
                    };
                    let (cue_text, rest) = match text.split_once('|') {
                        Some((a, b)) if !b.is_empty() => (a.to_string(), b.to_string()),
                        _ => {
                            // No complete pair yet — buffer everything.
                            self.pending = Some(text);
                            return Ok(ParseStatus::Accepted);
                        }
                    };
                    self.pending = Some(rest.clone());
                    let end = match rest.trim().parse::<i64>() {
                        Ok(t) => t,
                        Err(_) => {
                            return Ok(ParseStatus::Recovered {
                                skipped: 1,
                                reason: "unparsable end stamp",
                            });
                        }
                    };
                    let end_ts = match media_types::time::Timestamp::from_parts(end, 1, 1_000) {
                        Ok(ts) => ts,
                        Err(e) => unreachable!("{e}"),
                    };
                    let cue = SubtitleCue {
                        span: TimeSpan {
                            start: Stamp::Missing,
                            end: Stamp::Present(end_ts),
                        },
                        text: cue_text,
                    };
                    match sink.accept(cue) {
                        Ok(()) => Ok(ParseStatus::Accepted),
                        Err(e) => Err(e),
                    }
                }
            }
        }
    }

    struct Collector {
        cues: Vec<SubtitleCue>,
    }

    impl SubtitleSink for Collector {
        fn accept(&mut self, cue: SubtitleCue) -> Result<(), EngineError> {
            self.cues.push(cue);
            Ok(())
        }
    }

    #[test]
    fn parses_in_chunks_and_drains() {
        let mut parser = LineParser {
            pending: None,
            done: false,
        };
        let mut sink = Collector { cues: Vec::new() };
        // Split mid-line: the parser must buffer across calls.
        let s1 = match parser.next_cues(Some(SharedBytes::from_static(b"hello sub|")), &mut sink) {
            Ok(s) => s,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(s1, ParseStatus::Accepted);
        assert!(sink.cues.is_empty(), "incomplete cue not emitted early");
        let s2 = match parser.next_cues(Some(SharedBytes::from_static(b"9000")), &mut sink) {
            Ok(s) => s,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(s2, ParseStatus::Accepted);
        assert_eq!(sink.cues.len(), 1);
        assert_eq!(sink.cues[0].text, "hello sub");
        let s3 = match parser.next_cues(None, &mut sink) {
            Ok(s) => s,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(s3, ParseStatus::Drained);
        assert!(parser.done);
    }

    #[test]
    fn damage_is_data_not_silence() {
        let mut parser = LineParser {
            pending: None,
            done: false,
        };
        let mut sink = Collector { cues: Vec::new() };
        let status =
            match parser.next_cues(Some(SharedBytes::from_static(b"x|not-a-number")), &mut sink) {
                Ok(s) => s,
                Err(e) => unreachable!("{e}"),
            };
        assert_eq!(
            status,
            ParseStatus::Recovered {
                skipped: 1,
                reason: "unparsable end stamp",
            }
        );
        assert_eq!(sink.cues.len(), 0, "the damaged cue was skipped, counted");
    }

    #[test]
    fn format_mismatch_error_shape() {
        let err = not_this_format(&[0x1a, 0x45, 0xdf, 0xa3]);
        assert_eq!(err.stage, PipelineStage::Demux);
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Unsupported { .. }
        ));
    }
}
