//! Extended/plain M3U parsing and canonical writing (card 4.D1).
//!
//! # Format model
//!
//! * Line-based text (LF or CRLF terminators; a trailing `\r` is stripped).
//!   A leading UTF-8 BOM is tolerated and skipped.
//! * `#EXTM3U` — header; ignored wherever it appears (a headerless file is
//!   a legal *plain* M3U).
//! * `#EXTINF:<duration>[,<title>]` — metadata for the **next** location
//!   line. `duration` is seconds (`-1` = the spec's unknown marker,
//!   preserved verbatim); `title` is everything after the first comma.
//! * Any other `#EXT<NAME>:<value>` line (e.g. `#EXTVLCOPT:…`,
//!   `#EXTGRP:…`, unknown tags) is preserved as a directive on the next
//!   entry, in source order.
//! * Any other `#…` line is a comment and is ignored — including comments
//!   that happen to contain `:`, which are *not* directives.
//! * Blank (whitespace-only) lines are ignored.
//! * Everything else is a location line, kept verbatim after line-level
//!   whitespace trimming: URLs, Windows paths, Unix paths, relative paths.
//!
//! # Dirty-world behavior (directives D-1/D-2)
//!
//! * Malformed `#EXTINF` (bad/empty/non-finite duration, missing colon)
//!   degrades to a typed [`ParseWarning`]; the entry still parses with the
//!   damaged field absent — entry-not-death.
//! * Metadata still pending at EOF (no following location) is dropped with
//!   a [`ParseWarning::DanglingEntry`].
//! * A line longer than [`MAX_LINE_BYTES`] is fatal
//!   ([`ParseError::LineTooLong`]) — D-1 bounded input.
//! * Non-UTF-8 bytes are fatal ([`ParseError::InvalidUtf8`]) via
//!   [`parse_bytes`]; the parser never guesses an encoding.
//! * The parser never panics on any byte input (pinned by fuzz-lite).
//!
//! # Writer
//!
//! [`write`] emits canonical extended M3U: the `#EXTM3U` header, then per
//! entry its directives (in order), one `#EXTINF` line when duration or
//! title is present, then the location. The roundtrip law
//! `parse(write(parse(x))) == parse(x)` is pinned by tests for every
//! input that parses, including the dirty corpus.

use crate::error::{ParseError, ParseWarning};
use crate::types::{Entry, Playlist};

/// Maximum accepted line length in bytes, terminator excluded (D-1). A
/// longer line is rejected with [`ParseError::LineTooLong`]. 64 KiB is far
/// beyond any sane playlist line while bounding per-line work.
pub const MAX_LINE_BYTES: usize = 64 * 1024;

/// A successful parse: the playlist plus every dirty-world warning seen on
/// the way (warnings never alter the entry set — they only name dropped
/// metadata).
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// The parsed playlist.
    pub playlist: Playlist,
    /// Non-fatal findings, in source order.
    pub warnings: Vec<ParseWarning>,
}

/// Parses an M3U playlist, discarding warnings.
pub fn parse(input: &str) -> Result<Playlist, ParseError> {
    Ok(parse_detailed(input)?.playlist)
}

/// Parses an M3U playlist, reporting dirty-world warnings alongside it.
pub fn parse_detailed(input: &str) -> Result<Parsed, ParseError> {
    // BOM tolerance: exactly one leading U+FEFF. A BOM elsewhere is just
    // (weird) line content — kept, not guessed at.
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);

    let mut entries: Vec<Entry> = Vec::new();
    let mut warnings: Vec<ParseWarning> = Vec::new();

    // Metadata pending for the next location line.
    let mut duration: Option<f64> = None;
    let mut title: Option<String> = None;
    let mut directives: Vec<(String, String)> = Vec::new();
    let mut pending_since: Option<usize> = None;

    let mut rest: &str = input;
    let mut line_no: usize = 0;
    while !rest.is_empty() {
        line_no += 1;
        let (raw, tail): (&str, &str) = match rest.as_bytes().iter().position(|&b| b == b'\n') {
            Some(idx) => {
                let (line, after) = rest.split_at(idx);
                (line, &after[1..])
            }
            None => (rest, ""),
        };
        if raw.len() > MAX_LINE_BYTES {
            return Err(ParseError::LineTooLong {
                line: line_no,
                length: raw.len(),
            });
        }
        rest = tail;

        // CRLF tolerance, then line-level whitespace trim (locations keep
        // their internal bytes verbatim).
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(after_hash) = trimmed.strip_prefix('#') {
            if after_hash == "EXTM3U" {
                // Header (tolerated anywhere, tolerated repeated).
                continue;
            }
            if let Some(after_tag) = after_hash.strip_prefix("EXTINF") {
                let Some(body) = after_tag.strip_prefix(':') else {
                    warnings.push(ParseWarning::ExtinfMissingColon { line: line_no });
                    continue;
                };
                let (dur_text, title_text) = match body.find(',') {
                    Some(i) => (&body[..i], Some(&body[i + 1..])),
                    None => (body, None),
                };
                let dur = parse_duration(dur_text);
                if dur.is_none() {
                    warnings.push(ParseWarning::BadDuration { line: line_no });
                }
                duration = dur;
                title = title_text.map(str::to_string).filter(|t| !t.is_empty());
                if pending_since.is_none() {
                    pending_since = Some(line_no);
                }
            } else if after_hash.starts_with("EXT") {
                // Other (possibly unknown) directive: #EXT<NAME>:<value>.
                match after_hash.find(':') {
                    Some(i) if i > 0 => {
                        directives
                            .push((after_hash[..i].to_string(), after_hash[i + 1..].to_string()));
                        if pending_since.is_none() {
                            pending_since = Some(line_no);
                        }
                    }
                    Some(_) => {
                        // "#EXT:" — no name; treat as an ignored comment.
                    }
                    None => {
                        warnings.push(ParseWarning::DirectiveMissingColon { line: line_no });
                    }
                }
            }
            // Any other '#' line is a plain comment: ignored.
        } else {
            // Location line: flush pending metadata into a new entry.
            entries.push(Entry {
                location: trimmed.to_string(),
                duration,
                title,
                directives,
            });
            duration = None;
            title = None;
            directives = Vec::new();
            pending_since = None;
        }
    }

    if let Some(line) = pending_since {
        warnings.push(ParseWarning::DanglingEntry { line });
    }

    Ok(Parsed {
        playlist: Playlist { entries },
        warnings,
    })
}

/// Parses raw bytes as an M3U playlist. Non-UTF-8 input is a typed error;
/// every byte input either returns or errors — never panics (fuzz-pinned).
pub fn parse_bytes(input: &[u8]) -> Result<Parsed, ParseError> {
    let text = std::str::from_utf8(input).map_err(|_| ParseError::InvalidUtf8)?;
    parse_detailed(text)
}

/// Writes a playlist as canonical extended M3U (see the module docs).
/// Fields originating from [`parse`] contain no terminators, so the output
/// is well-formed by construction.
#[must_use]
pub fn write(playlist: &Playlist) -> String {
    let mut out = String::from("#EXTM3U\n");
    for entry in &playlist.entries {
        for (name, value) in &entry.directives {
            out.push('#');
            out.push_str(name);
            out.push(':');
            out.push_str(value);
            out.push('\n');
        }
        if entry.duration.is_some() || entry.title.is_some() {
            out.push_str("#EXTINF:");
            if let Some(d) = entry.duration {
                // Shortest round-trip decimal: `123.0` prints as `123`,
                // `123.5` as `123.5`; both re-parse exactly.
                out.push_str(&format!("{d}"));
            }
            if let Some(t) = &entry.title {
                out.push(',');
                out.push_str(t);
            }
            out.push('\n');
        }
        out.push_str(&entry.location);
        out.push('\n');
    }
    out
}

/// Duration text → finite seconds. Empty, malformed and non-finite
/// (`inf`, `nan`, overflow like `1e999`) all return `None` — the caller
/// turns that into a `BadDuration` warning (D-2: honest degradation).
fn parse_duration(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.parse::<f64>() {
        Ok(d) if d.is_finite() => Some(d),
        _ => None,
    }
}
