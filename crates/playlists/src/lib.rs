//! # playlists — M3U playlist reading/writing (Lumen card 4.D1)
//!
//! Rust crate for parsing and writing M3U playlists (extended and plain),
//! the substrate for the upcoming playlist panel and queue semantics
//! (those land after 4.C2 merges — this crate is deliberately I/O-free,
//! pure text-in/text-out, so the UI layer owns file/dialog policy).
//!
//! ## Scope (v0)
//!
//! * [`parse`]/[`parse_detailed`]/[`parse_bytes`] — extended M3U
//!   (`#EXTM3U`, `#EXTINF`, `#EXT…` directives) and plain M3U (bare
//!   location lines).
//! * [`write`] — canonical extended M3U output.
//! * Roundtrip law: `parse(write(parse(x))) == parse(x)` for every input
//!   `x` that parses (pinned by tests, including the dirty corpus and
//!   fuzz-lite).
//!
//! ## Design decisions (recorded for reviewers)
//!
//! * **Verbatim locations** — paths/URLs are kept exactly as written
//!   (after line-trim); no resolution, normalization, or scheme guessing.
//!   `-1` durations (the spec's unknown marker) are preserved as written.
//! * **No invented metadata** — unknown `#EXT…` tags are kept as ordered
//!   `(name, value)` directive pairs on the next entry; plain `#`
//!   comments (even with colons) are ignored, per the public spec.
//! * **Dirty-world tolerance (D-2)** — malformed `#EXTINF`, dangling
//!   metadata at EOF, truncated headers: typed [`ParseWarning`]s, the
//!   entry survives. Warnings never leak line text or paths (MP §12).
//! * **Bounded input (D-1)** — lines over [`MAX_LINE_BYTES`] (64 KiB)
//!   are rejected with a typed error; nothing unbounded is processed.
//! * **No panics on any byte input** — pinned by a deterministic 10k-case
//!   fuzz-lite harness plus a 5k-case mutation fuzz in the test suite.
//! * Std + `thiserror` only (the workspace's existing error family); no
//!   `unsafe`, per the workspace lint policy.

mod error;
mod m3u;
mod types;

pub use error::{ParseError, ParseWarning};
pub use m3u::{MAX_LINE_BYTES, Parsed, parse, parse_bytes, parse_detailed, write};
pub use types::{Entry, Playlist};

// Card 5.O3 (append-only wiring): playlist probing + bounded directory
// loading. The crate keeps its I/O-free posture — every byte and every
// directory listing arrives through caller-supplied impls; see
// `probe`'s module docs for the R4 (directory-access trait) seam and
// the media-magic table note.
pub mod probe;

pub use probe::{
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_ENTRIES, DirEntryKind, DirSource, ListedEntry,
    PROBE_PREFIX_BYTES, PlaylistFormat, ProbedKind, WalkConfig, WalkError, WalkResult,
    WalkWarnings, probe, probe_path, walk,
};
