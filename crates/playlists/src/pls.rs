//! PLS playlist parsing (card 5.O1b) — the INI-style sibling of M3U.
//!
//! # Format model (per the public PLS spec, as emitted by Winamp/XMMS/ffmpeg
//! `-f pls`)
//!
//! * Line-based text (LF or CRLF; a trailing `\r` is stripped), one leading
//!   UTF-8 BOM tolerated. `;` and `#` lines are comments; blank lines are
//!   ignored.
//! * A `[playlist]` section header (case-insensitive, surrounding
//!   whitespace tolerated) opens the playlist. Other `[section]` headers
//!   follow INI semantics: keys under them are ignored until the next
//!   `[playlist]`.
//! * Inside the section, `Key=Value` pairs (split at the **first** `=`,
//!   key/value whitespace-trimmed, keys ASCII-case-insensitive):
//!   * `FileN` — the media location, verbatim. `N` is 1-indexed and
//!     **sparse-tolerant**: indexes 2..4 may be absent; entries are emitted
//!     in index order.
//!   * `TitleN` — the display title (empty value normalizes to `None`).
//!   * `LengthN` — duration in seconds; `-1` is the unknown/live marker and
//!     is preserved as written, exactly like M3U's `#EXTINF:-1`.
//!   * `NumberOfEntries` — cross-checked against the parsed entry count; a
//!     lying count is a damage note ([`PlsWarning::EntryCountMismatch`]),
//!     never fatal — the parsed entries win.
//!   * `Version` — `2` is the only real version; any other value is noted
//!     ([`PlsWarning::UnexpectedVersion`]) and tolerated.
//! * Unknown keys (anything not matching the above shapes) are ignored per
//!   INI convention — this parser is a deliberate subset, and PLS has no
//!   per-entry extension vocabulary worth preserving.
//!
//! # Dirty-world behavior (directives D-1/D-2)
//!
//! * `File0`/`Filex` (bad or zero index) → [`PlsWarning::BadIndex`]; the
//!   key is ignored. Malformed `LengthN` → [`PlsWarning::BadLength`], the
//!   entry survives with `duration: None`. Duplicate keys for one index →
//!   [`PlsWarning::DuplicateKey`], last value wins. `FileN=` (empty value)
//!   → [`PlsWarning::EmptyLocation`], the entry is kept with an empty
//!   location. `TitleN`/`LengthN` with no `FileN` for that index →
//!   [`PlsWarning::OrphanMetadata`], the metadata is dropped. Keys before
//!   any section header still parse (with a one-time
//!   [`PlsWarning::MissingSectionHeader`]) — entry-not-death throughout.
//! * A line over [`MAX_LINE_BYTES`](crate::m3u::MAX_LINE_BYTES) is fatal
//!   ([`PlsError::LineTooLong`]) — D-1 bounded input, same cap as M3U.
//! * Non-UTF-8 bytes are fatal ([`PlsError::InvalidUtf8`]) via
//!   [`parse_bytes`]; no encoding is guessed.
//! * The parser never panics on any byte input (pinned by fuzz-lite).
//!
//! # Mapping onto the shared vocabulary
//!
//! `FileN`/`TitleN`/`LengthN` map 1:1 onto [`Entry::location`],
//! [`Entry::title`], [`Entry::duration`]. PLS has no extra per-entry
//! metadata in the subset, so `directives` is always empty; a parsed PLS
//! therefore bridges losslessly through [`Playlist::to_m3u_string`].

use std::collections::BTreeMap;

use crate::m3u::MAX_LINE_BYTES;
use crate::types::{Entry, Playlist};

/// Fatal parse failures. Dirty input degrades to [`PlsWarning`]s; only the
/// D-1 line-cap violation and non-UTF-8 input are fatal (same policy, and
/// the same redaction rule, as the M3U parser: line numbers and counts
/// only — never key names, values, locations or paths).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlsError {
    /// A single line exceeds the 64 KiB line cap (D-1 bounded input).
    /// `line` is 1-based; `length` is the line's byte length excluding the
    /// terminator.
    #[error(
        "playlists: line {line} is {length} bytes, over the {}-byte line cap (D-1 bounded input)",
        MAX_LINE_BYTES
    )]
    LineTooLong {
        /// 1-based line number.
        line: usize,
        /// Byte length of the offending line (without terminator).
        length: usize,
    },

    /// The byte input is not valid UTF-8.
    #[error(
        "playlists: input is not valid UTF-8 (PLS is a text format; encoding is the caller's policy)"
    )]
    InvalidUtf8,
}

/// Non-fatal dirty-world findings (D-2). Line numbers are 1-based; no key
/// names, values, locations or titles are ever embedded (MP §12).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlsWarning {
    /// A PLS key appeared before any section header. Parsed anyway
    /// (tolerant of a missing `[playlist]` line); warned once.
    #[error("playlists: line {line}: PLS key before any [playlist] section header (parsed anyway)")]
    MissingSectionHeader {
        /// 1-based line number.
        line: usize,
    },

    /// A `[…` line with no closing `]`. The line is ignored.
    #[error("playlists: line {line}: malformed section header (line ignored)")]
    MalformedSection {
        /// 1-based line number.
        line: usize,
    },

    /// A content line with no `=` (PLS has no bare-location lines).
    #[error("playlists: line {line}: not a key=value pair (line ignored)")]
    NotAKeyValuePair {
        /// 1-based line number.
        line: usize,
    },

    /// A known File/Title/Length key whose index suffix is zero, non-numeric
    /// or overflowing. The key is ignored.
    #[error("playlists: line {line}: known PLS key with a bad index suffix (key ignored)")]
    BadIndex {
        /// 1-based line number.
        line: usize,
    },

    /// `FileN` with an empty value. The entry is kept with an empty
    /// location rather than dropped.
    #[error("playlists: line {line}: File key with an empty value (entry kept, empty location)")]
    EmptyLocation {
        /// 1-based line number.
        line: usize,
    },

    /// The same key was set twice for one index. The last value wins.
    #[error("playlists: line {line}: duplicate key for one entry index (last value wins)")]
    DuplicateKey {
        /// 1-based line number.
        line: usize,
    },

    /// `LengthN` was empty, not a number, or non-finite. The entry survives
    /// with `duration: None`.
    #[error("playlists: line {line}: malformed Length value (kept entry, duration dropped)")]
    BadLength {
        /// 1-based line number.
        line: usize,
    },

    /// `NumberOfEntries` was not a parseable count. Ignored.
    #[error("playlists: line {line}: NumberOfEntries is not a count (ignored)")]
    BadEntryCount {
        /// 1-based line number.
        line: usize,
    },

    /// `Version` was present but not `2`. Tolerated.
    #[error("playlists: line {line}: Version is not 2 (tolerated)")]
    UnexpectedVersion {
        /// 1-based line number.
        line: usize,
    },

    /// `TitleN`/`LengthN` existed for an index with no `FileN`. The
    /// metadata is dropped (reported at the first such line).
    #[error(
        "playlists: line {line}: Title/Length metadata with no File for that index (metadata dropped)"
    )]
    OrphanMetadata {
        /// 1-based line number of the first orphaned metadata line.
        line: usize,
    },

    /// `NumberOfEntries` disagrees with the number of parsed entries. The
    /// parsed entries win. Counts only — no content is embedded.
    #[error(
        "playlists: NumberOfEntries says {declared} but {found} entries were parsed (entries kept)"
    )]
    EntryCountMismatch {
        /// The declared count.
        declared: usize,
        /// The actually parsed count.
        found: usize,
    },
}

/// A successful PLS parse: the playlist plus every dirty-world warning seen
/// on the way (warnings never alter the entry set — they only name dropped
/// or damaged metadata and lying headers).
#[derive(Debug, Clone, PartialEq)]
pub struct PlsParsed {
    /// The parsed playlist (entries in index order).
    pub playlist: Playlist,
    /// Non-fatal findings (line-scan findings in source order, then the
    /// finalize-time orphan/count findings).
    pub warnings: Vec<PlsWarning>,
}

/// Parses a PLS playlist, discarding warnings.
pub fn parse(input: &str) -> Result<Playlist, PlsError> {
    Ok(parse_detailed(input)?.playlist)
}

/// Parses a PLS playlist, reporting dirty-world warnings alongside it.
pub fn parse_detailed(input: &str) -> Result<PlsParsed, PlsError> {
    // BOM tolerance: exactly one leading U+FEFF.
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);

    let mut entries: BTreeMap<usize, PartialEntry> = BTreeMap::new();
    let mut warnings: Vec<PlsWarning> = Vec::new();
    let mut declared_count: Option<usize> = None;
    let mut in_playlist = false;
    let mut section_seen = false;
    let mut warned_missing_section = false;

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
            return Err(PlsError::LineTooLong {
                line: line_no,
                length: raw.len(),
            });
        }
        rest = tail;

        // CRLF tolerance, then whitespace trim (values keep their internal
        // bytes verbatim).
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let first = trimmed.chars().next().unwrap_or(' ');
        if first == ';' || first == '#' {
            // INI-style comments.
            continue;
        }

        if first == '[' {
            match trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                Some(inner) => {
                    in_playlist = inner.trim().eq_ignore_ascii_case("playlist");
                    section_seen = true;
                }
                None => warnings.push(PlsWarning::MalformedSection { line: line_no }),
            }
            continue;
        }

        let Some(eq) = trimmed.find('=') else {
            warnings.push(PlsWarning::NotAKeyValuePair { line: line_no });
            continue;
        };
        let key = trimmed[..eq].trim();
        let value = trimmed[eq + 1..].trim();
        if key.is_empty() {
            warnings.push(PlsWarning::NotAKeyValuePair { line: line_no });
            continue;
        }
        let kind = classify_key(&key.to_ascii_lowercase());

        if !section_seen {
            // Tolerate a missing [playlist] header: known keys still parse,
            // with a one-time damage note. Unknown keys are just ignored.
            if matches!(
                kind,
                KeyKind::File(_)
                    | KeyKind::Title(_)
                    | KeyKind::Length(_)
                    | KeyKind::Count
                    | KeyKind::Version
            ) {
                if !warned_missing_section {
                    warnings.push(PlsWarning::MissingSectionHeader { line: line_no });
                    warned_missing_section = true;
                }
            } else {
                continue;
            }
        } else if !in_playlist {
            // Keys under a foreign section: INI semantics, ignored.
            continue;
        }

        match kind {
            KeyKind::BadIndex => warnings.push(PlsWarning::BadIndex { line: line_no }),
            KeyKind::Unknown => {} // INI-convention ignore of unknown keys.
            KeyKind::Count => match value.parse::<usize>() {
                Ok(n) => declared_count = Some(n),
                Err(_) => warnings.push(PlsWarning::BadEntryCount { line: line_no }),
            },
            KeyKind::Version => {
                if value != "2" {
                    warnings.push(PlsWarning::UnexpectedVersion { line: line_no });
                }
            }
            KeyKind::File(idx) => {
                let e = entries.entry(idx).or_default();
                if e.file_set {
                    warnings.push(PlsWarning::DuplicateKey { line: line_no });
                }
                e.file_set = true;
                if value.is_empty() {
                    warnings.push(PlsWarning::EmptyLocation { line: line_no });
                } else {
                    e.file = Some(value.to_string());
                }
            }
            KeyKind::Title(idx) => {
                let e = entries.entry(idx).or_default();
                if e.title_set {
                    warnings.push(PlsWarning::DuplicateKey { line: line_no });
                }
                e.title_set = true;
                e.title = (!value.is_empty()).then(|| value.to_string());
                if e.first_meta_line.is_none() {
                    e.first_meta_line = Some(line_no);
                }
            }
            KeyKind::Length(idx) => {
                let e = entries.entry(idx).or_default();
                if e.length_set {
                    warnings.push(PlsWarning::DuplicateKey { line: line_no });
                }
                e.length_set = true;
                if e.first_meta_line.is_none() {
                    e.first_meta_line = Some(line_no);
                }
                if value.is_empty() {
                    warnings.push(PlsWarning::BadLength { line: line_no });
                } else {
                    match value.parse::<f64>() {
                        Ok(d) if d.is_finite() => e.length = Some(d),
                        _ => warnings.push(PlsWarning::BadLength { line: line_no }),
                    }
                }
            }
        }
    }

    // Finalize: entries in index order (sparse indexes tolerated), then the
    // cross-checks.
    let mut playlist = Playlist::new();
    for (_, part) in entries {
        if !part.file_set {
            if let Some(line) = part.first_meta_line {
                warnings.push(PlsWarning::OrphanMetadata { line });
            }
            continue;
        }
        playlist.push(Entry {
            location: part.file.unwrap_or_default(),
            duration: part.length,
            title: part.title,
            directives: Vec::new(),
        });
    }
    if let Some(declared) = declared_count
        && declared != playlist.len()
    {
        warnings.push(PlsWarning::EntryCountMismatch {
            declared,
            found: playlist.len(),
        });
    }

    Ok(PlsParsed { playlist, warnings })
}

/// Parses raw bytes as a PLS playlist. Non-UTF-8 input is a typed error;
/// every byte input either returns or errors — never panics (fuzz-pinned).
pub fn parse_bytes(input: &[u8]) -> Result<PlsParsed, PlsError> {
    let text = std::str::from_utf8(input).map_err(|_| PlsError::InvalidUtf8)?;
    parse_detailed(text)
}

/// One sparse index slot, filled by whichever keys arrived for it.
#[derive(Default)]
struct PartialEntry {
    file: Option<String>,
    file_set: bool,
    title: Option<String>,
    title_set: bool,
    length: Option<f64>,
    length_set: bool,
    first_meta_line: Option<usize>,
}

/// Key classification over the ASCII-lowercased key text.
enum KeyKind {
    File(usize),
    Title(usize),
    Length(usize),
    Count,
    Version,
    BadIndex,
    Unknown,
}

fn classify_key(lower: &str) -> KeyKind {
    if lower == "numberofentries" {
        return KeyKind::Count;
    }
    if lower == "version" {
        return KeyKind::Version;
    }
    if let Some(r) = split_index(lower, "file") {
        return match r {
            Ok(n) => KeyKind::File(n),
            Err(()) => KeyKind::BadIndex,
        };
    }
    if let Some(r) = split_index(lower, "title") {
        return match r {
            Ok(n) => KeyKind::Title(n),
            Err(()) => KeyKind::BadIndex,
        };
    }
    if let Some(r) = split_index(lower, "length") {
        return match r {
            Ok(n) => KeyKind::Length(n),
            Err(()) => KeyKind::BadIndex,
        };
    }
    KeyKind::Unknown
}

/// `file<N>` → `Some(Ok(N))` for a valid 1-based index, `Some(Err(()))` for
/// a known-prefix key with a bad/zero/overflowing suffix, `None` when the
/// key does not carry this prefix at all.
fn split_index(lower: &str, prefix: &str) -> Option<Result<usize, ()>> {
    let rest = lower.strip_prefix(prefix)?;
    if rest.is_empty() {
        return None; // bare "file"/"title"/"length": an unknown key, not damage
    }
    if !rest.bytes().all(|b| b.is_ascii_digit()) {
        return Some(Err(()));
    }
    match rest.parse::<usize>() {
        Ok(n) if n >= 1 => Some(Ok(n)),
        _ => Some(Err(())), // zero or overflowing index
    }
}
