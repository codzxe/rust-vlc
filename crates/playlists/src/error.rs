//! Typed parse errors and warnings (card 4.D1).
//!
//! Redaction rule (MP §12, matching `media-types`' error discipline): the
//! `Display` of every variant names the *kind* and the *line number* only.
//! Line text, locations, URLs and filesystem paths never enter an error or
//! warning message — the caller already holds the input.

use crate::m3u::MAX_LINE_BYTES;

/// Fatal parse failures. Dirty input usually degrades to a
/// [`ParseWarning`] instead (D-2); only the D-1 line-cap violation and
/// non-UTF-8 input are fatal, because both mean we cannot honestly
/// tokenize the file at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// A single line exceeds [`MAX_LINE_BYTES`] (D-1: bounded input — the
    /// parser refuses to process unbounded lines instead of stalling or
    /// allocating per-line). `line` is 1-based; `length` is the line's byte
    /// length excluding the terminator.
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

    /// The byte input is not valid UTF-8. M3U is a text format; we do not
    /// guess an encoding.
    #[error(
        "playlists: input is not valid UTF-8 (M3U is a text format; encoding is the caller's policy)"
    )]
    InvalidUtf8,
}

/// Non-fatal dirty-world findings (D-2): the entry still parses, with the
/// affected field left absent. Line numbers are 1-based and no line
/// content is embedded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseWarning {
    /// `#EXTINF` duration text was empty, not a number, or non-finite
    /// (`inf`/`nan`). The entry survives with `duration: None`.
    #[error("playlists: line {line}: malformed #EXTINF duration (kept entry, duration dropped)")]
    BadDuration {
        /// 1-based line number.
        line: usize,
    },

    /// A line starting `#EXTINF` had no `:` after the tag.
    #[error("playlists: line {line}: #EXTINF without ':' (line ignored)")]
    ExtinfMissingColon {
        /// 1-based line number.
        line: usize,
    },

    /// A line starting `#EXT…` (other than `#EXTINF`) had no `:`, so no
    /// name/value split exists. This also covers truncated headers such as
    /// a lone `#EXTM`.
    #[error("playlists: line {line}: #EXT directive without ':' (line ignored)")]
    DirectiveMissingColon {
        /// 1-based line number.
        line: usize,
    },

    /// `#EXTINF`/directive metadata was still pending when the file ended,
    /// with no location line to attach it to. The metadata is dropped.
    #[error("playlists: line {line}: entry metadata with no following location (metadata dropped)")]
    DanglingEntry {
        /// 1-based line number of the first pending metadata line.
        line: usize,
    },

    /// An `#EXT-X-*` tag from the HLS subset (card 5.O1) was malformed —
    /// broken attribute-list syntax, a badly-typed or missing required
    /// attribute, a non-numeric byte-range, or junk on a valueless tag.
    /// The line is skipped; segments already parsed are unaffected and
    /// later lines still parse (D-2 line-level skip).
    #[error("playlists: line {line}: malformed #EXT-X-* tag (line skipped, entry survives)")]
    BadExtXTag {
        /// 1-based line number.
        line: usize,
    },
}
