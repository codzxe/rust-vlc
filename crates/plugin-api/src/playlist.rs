//! The playlist kinds (doc 06 §1.2 rows 12–13): playlist parsing
//! (URI/drop → entries) and export (`playlist.save`, doc 05 §2.7).
//!
//! The domain crate `playlists` (L2) owns the rich
//! `playlists::PlaylistParser` / `PlaylistExporter` traits per doc 06
//! §1.2; the minimal-but-real host traits staged here [PROPOSAL:
//! placement, doc 06 §10 reconciliation] speak only L0 types, carry
//! the same calling-context contract as the other host traits, and
//! define the shared [`Playlist`] vocabulary both directions use.
//! Both kinds may load as WASM extensions in v1 (§1.4).

use std::fmt;

use media_types::ids::SourceUri;
use media_types::packet::SharedBytes;
use media_types::{Duration, EngineError, PipelineStage};

/// A parsed playlist (doc 05 §2.7 `playlist.*` command family's data):
/// ordered entries, stable ids, no playback state — the playlist is a
/// document, the player is its consumer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Playlist {
    /// Entries in order; positions are meaningful (reorder commands
    /// are user-visible, doc 07 §5).
    pub entries: Vec<PlaylistEntry>,
}

/// One playlist row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistEntry {
    /// The media address. Playlists may point at other playlists;
    /// the engine expands lazily and guards cycles (doc 06 §2.2's
    /// dependency-cycle posture transposed to playlist refs).
    pub uri: SourceUri,
    /// Entry title from the playlist document, when present
    /// (data, not UI copy — localization never rewrites it).
    pub title: Option<String>,
    /// Declared duration, when the format carries one
    /// (`--:--` when unknown — a value and its absence are different
    /// types, MP §8).
    pub duration: Duration,
}

impl Playlist {
    /// An empty playlist.
    pub fn new() -> Playlist {
        Playlist {
            entries: Vec::new(),
        }
    }

    /// Append one entry (the only construction path — entries are
    /// ordered by construction).
    pub fn push(&mut self, entry: PlaylistEntry) {
        self.entries.push(entry);
    }

    /// Entry count (the `playlist.count` query's data).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty playlist?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A playlist format parser (doc 06 §1.2 row 12). Instance scope:
/// per job (§1.3).
///
/// # Calling context
///
/// Worker thread ([`Send`]); bounded work per call — one chunk per
/// [`next_entries`](PlaylistParser::next_entries), never an internal
/// parse-the-whole-file loop; malformed lines are skipped and
/// counted (playlists are user-editable text; a bad line is data,
/// not a failed document).
pub trait PlaylistParser: Send {
    /// Feed one chunk of the playlist document (`None` = end).
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Open`]: playlist
    /// parsing happens at open/drop time) when the bytes are not the
    /// claimed format at all.
    fn next_entries(
        &mut self,
        chunk: Option<SharedBytes>,
        sink: &mut dyn FnMut(PlaylistEntry) -> Result<(), EngineError>,
    ) -> Result<ParseOutcome, EngineError>;
}

/// One playlist parse step's outcome (shared bounded-recovery
/// vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseOutcome {
    /// Chunk accepted; entries (if any) went to the sink.
    Accepted,
    /// Document fully parsed.
    Drained,
    /// Damage overcome; `skipped` counts entries lost.
    Recovered {
        /// Entries skipped.
        skipped: u32,
        /// Why.
        reason: &'static str,
    },
}

/// A playlist exporter (doc 06 §1.2 row 13 [PROPOSAL kind]): a
/// [`Playlist`] → document writer for `playlist.save`. Instance
/// scope: per job.
///
/// # Errors
/// All methods return [`EngineError`] (stage
/// [`PipelineStage::Open`]); I/O belongs to the engine — the exporter
/// produces document bytes, the engine writes the file (the same
/// split as Access: capability, not implementation, doc 06 §2.1).
pub trait PlaylistExporter: Send {
    /// Begin a document for `playlist`.
    fn begin(&mut self, playlist: &Playlist) -> Result<(), EngineError>;

    /// Serialize one more entry into the document.
    fn write_entry(&mut self, entry: &PlaylistEntry) -> Result<(), EngineError>;

    /// Finish; the document bytes leave the exporter here.
    fn finish(&mut self) -> Result<SharedBytes, EngineError>;
}

/// Typed playlist error: an entry line's URI did not parse.
pub fn bad_entry_uri(line: &str) -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new("plugin.playlist.bad-entry-uri"),
    )
    .with_source(BadUriLine(line.to_owned()))
}

#[derive(Debug)]
struct BadUriLine(String);

impl fmt::Display for BadUriLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "playlist line is not a URI: `{}`", self.0)
    }
}
impl std::error::Error for BadUriLine {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A toy `path|title|seconds` line parser.
    struct LineParser {
        buf: String,
    }

    impl PlaylistParser for LineParser {
        fn next_entries(
            &mut self,
            chunk: Option<SharedBytes>,
            sink: &mut dyn FnMut(PlaylistEntry) -> Result<(), EngineError>,
        ) -> Result<ParseOutcome, EngineError> {
            match chunk {
                None => Ok(ParseOutcome::Drained),
                Some(bytes) => {
                    self.buf
                        .push_str(&String::from_utf8_lossy(bytes.as_slice()));
                    let buf = std::mem::take(&mut self.buf);
                    let mut lines: Vec<&str> = buf.split('\n').collect();
                    self.buf = match lines.pop() {
                        Some(last) => last.to_string(), // partial line stays buffered
                        None => String::new(),
                    };
                    let mut skipped = 0u32;
                    for line in lines {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        let mut parts = line.splitn(3, '|');
                        let (uri_s, title, secs) =
                            (parts.next().unwrap_or(""), parts.next(), parts.next());
                        match SourceUri::parse(uri_s) {
                            Ok(uri) => {
                                let duration = match secs.and_then(|s| s.parse::<u64>().ok()) {
                                    Some(ns) => Duration::Known(media_types::Nanoseconds(ns)),
                                    None => Duration::Unknown,
                                };
                                let entry = PlaylistEntry {
                                    uri,
                                    title: title.map(|t| t.to_string()),
                                    duration,
                                };
                                match sink(entry) {
                                    Ok(()) => {}
                                    Err(e) => return Err(e),
                                }
                            }
                            Err(_) => skipped += 1,
                        }
                    }
                    if skipped > 0 {
                        return Ok(ParseOutcome::Recovered {
                            skipped,
                            reason: "unparsable entry URI",
                        });
                    }
                    Ok(ParseOutcome::Accepted)
                }
            }
        }
    }

    /// A toy exporter writing one `URI\n` per entry.
    struct LineExporter {
        out: Vec<u8>,
    }

    impl PlaylistExporter for LineExporter {
        fn begin(&mut self, _playlist: &Playlist) -> Result<(), EngineError> {
            self.out.clear();
            Ok(())
        }
        fn write_entry(&mut self, entry: &PlaylistEntry) -> Result<(), EngineError> {
            // The document carries the real address — redaction is a
            // logging concern (MP §12); `.raw()` is the explicit
            // raw-text accessor on SourceUri.
            self.out
                .extend_from_slice(format!("{}\n", entry.uri.raw()).as_bytes());
            Ok(())
        }
        fn finish(&mut self) -> Result<SharedBytes, EngineError> {
            Ok(SharedBytes::from_vec(std::mem::take(&mut self.out)))
        }
    }

    #[test]
    fn parse_then_export_round_trips_uris() {
        let doc = "file:///a/a.flac|Song A|1000\nfile:///b/b.flac|Song B|2000\n";
        let mut parser = LineParser { buf: String::new() };
        let mut playlist = Playlist::new();
        let mut sink = |entry: PlaylistEntry| -> Result<(), EngineError> {
            playlist.push(entry);
            Ok(())
        };
        // Feed in two chunks (mid-line split).
        let (head, tail) = doc.split_at(28);
        let (a, b) = (head.as_bytes(), tail.as_bytes());
        for chunk in [Some(a), Some(b), None] {
            let chunk = chunk.map(|b| SharedBytes::from_vec(b.to_vec()));
            match parser.next_entries(chunk, &mut sink) {
                Ok(_) => {}
                Err(e) => unreachable!("{e}"),
            }
        }
        assert_eq!(playlist.len(), 2);
        assert_eq!(playlist.entries[0].title.as_deref(), Some("Song A"));
        assert_eq!(
            playlist.entries[1].duration,
            Duration::Known(media_types::Nanoseconds(2000))
        );
        assert_eq!(
            playlist.entries[0].duration,
            Duration::Known(media_types::Nanoseconds(1000))
        );

        let mut exporter = LineExporter { out: Vec::new() };
        match exporter.begin(&playlist) {
            Ok(()) => {}
            Err(e) => unreachable!("{e}"),
        }
        for entry in &playlist.entries {
            match exporter.write_entry(entry) {
                Ok(()) => {}
                Err(e) => unreachable!("{e}"),
            }
        }
        let bytes = match exporter.finish() {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(
            String::from_utf8_lossy(bytes.as_slice()),
            "file:///a/a.flac\nfile:///b/b.flac\n"
        );
    }

    #[test]
    fn bad_lines_are_counted_not_fatal() {
        let doc = "not a uri at all\nfile:///ok.flac|T|1\n";
        let mut parser = LineParser { buf: String::new() };
        let mut count = 0;
        let mut sink = |_entry: PlaylistEntry| -> Result<(), EngineError> {
            count += 1;
            Ok(())
        };
        let status = match parser.next_entries(
            Some(SharedBytes::from_vec(doc.as_bytes().to_vec())),
            &mut sink,
        ) {
            Ok(s) => s,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(
            status,
            ParseOutcome::Recovered {
                skipped: 1,
                reason: "unparsable entry URI",
            }
        );
        assert_eq!(count, 1, "good line survived");
    }

    #[test]
    fn bad_uri_error_is_typed() {
        let err = bad_entry_uri("|oops");
        assert_eq!(err.stage, PipelineStage::Open);
        assert!(matches!(err.kind, media_types::ErrorKind::Corrupt { .. }));
    }
}
