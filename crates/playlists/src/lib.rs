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
//! * **Card 5.O1b** — [`parse_pls`] and [`parse_xspf`]: the two sibling
//!   playlist formats join M3U on the same [`Playlist`] vocabulary. PLS
//!   is read as the INI-style `[playlist]`/`FileN`/`TitleN`/`LengthN`
//!   triples (1-indexed, sparse-tolerant, `NumberOfEntries` cross-check
//!   with a damage note on a lying count). XSPF is a hand-rolled XML
//!   subset pull parser (no new dependency): the five named entities
//!   plus numeric references, CDATA, namespace/prefix-tolerant
//!   local-name matching, an attribute subset, and a 64-element depth
//!   guard; unclosed elements are salvaged, never fatal. XSPF
//!   `creator`/`annotation` ride the entry directive pairs
//!   (`("creator", …)` then `("annotation", …)`); both parsers carry
//!   the same D-1/D-2 damage discipline and are fuzz-pinned like the
//!   M3U one.

mod error;
mod m3u;
mod pls;
mod types;
mod xspf;

pub use error::{ParseError, ParseWarning};
pub use m3u::{MAX_LINE_BYTES, Parsed, parse, parse_bytes, parse_detailed, write};
pub use pls::{
    PlsError, PlsParsed, PlsWarning, parse as parse_pls, parse_bytes as parse_pls_bytes,
    parse_detailed as parse_pls_detailed,
};
pub use types::{Entry, Playlist};
pub use xspf::{
    MAX_XML_DEPTH, XspfError, XspfParsed, XspfWarning, parse as parse_xspf,
    parse_bytes as parse_xspf_bytes, parse_detailed as parse_xspf_detailed,
};
