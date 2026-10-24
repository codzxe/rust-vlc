//! # playlists — M3U/M3U8 playlist reading/writing (Lumen cards 4.D1 + 5.O1a)
//!
//! Rust crate for parsing and writing M3U playlists (extended and plain)
//! plus the HLS `#EXT-X-*` media-playlist subset — the substrate for the
//! upcoming playlist panel and queue semantics (those land after 4.C2
//! merges — this crate is deliberately I/O-free, pure text-in/text-out,
//! so the UI layer owns file/dialog policy).
//!
//! ## Scope
//!
//! * [`parse`]/[`parse_detailed`]/[`parse_bytes`] — extended M3U
//!   (`#EXTM3U`, `#EXTINF`, `#EXT…` directives) and plain M3U (bare
//!   location lines).
//! * HLS media-playlist subset (5.O1a), **recorded, not resolved**:
//!   `#EXT-X-BYTERANGE` ([`ByteRange`]), `#EXT-X-DISCONTINUITY`,
//!   `#EXT-X-KEY` ([`HlsKey`] — method+URI), `#EXT-X-MAP` ([`HlsMap`]),
//!   `#EXT-X-ENDLIST` ([`Playlist::end_list`]); unknown `#EXT-X-*` tags
//!   stay verbatim directives. Attribute lists parse the HLS value
//!   types (quoted-string, decimal-integer, hexadecimal, enumerated).
//!   This is parsing only — the streaming scheduler is Phase 6.
//! * [`resolve_uri`] — RFC 3986 §5 relative-reference join (explicit,
//!   verbatim-locations-preserving): `../`, `./`, absolute-http,
//!   Windows-drive/UNC passthrough.
//! * [`write`] — canonical extended M3U output (HLS tags delta-encoded).
//! * Roundtrip law: `parse(write(parse(x))) == parse(x)` for every input
//!   `x` that parses (pinned by tests, including the dirty corpus,
//!   ffmpeg-generated HLS fixtures, and fuzz-lite).
//!
//! ## Design decisions (recorded for reviewers)
//!
//! * **Verbatim locations** — paths/URLs are kept exactly as written
//!   (after line-trim); no resolution, normalization, or scheme guessing.
//!   `-1` durations (the spec's unknown marker) are preserved as written.
//!   Resolution is a separate, explicit [`resolve_uri`] call.
//! * **No invented metadata** — unknown `#EXT…` tags are kept as ordered
//!   `(name, value)` directive pairs on the next entry; plain `#`
//!   comments (even with colons) are ignored, per the public spec. The
//!   HLS subset is *recorded*: keys name their method/URI, byte-range
//!   offsets stay as written, nothing is fetched or decrypted.
//! * **Stateful HLS tags are stamped** — `EXT-X-KEY`/`EXT-X-MAP` apply
//!   from their line onward; the parser records the effective value on
//!   each segment so consumers never replay the playlist to know what
//!   governs segment *i*.
//! * **Dirty-world tolerance (D-2)** — malformed `#EXTINF`, dangling
//!   metadata at EOF, truncated headers, malformed `#EXT-X-*` tags:
//!   typed [`ParseWarning`]s, line-level skip, the entry survives.
//!   Warnings never leak line text or paths (MP §12).
//! * **Bounded input (D-1)** — lines over [`MAX_LINE_BYTES`] (64 KiB)
//!   are rejected with a typed error; nothing unbounded is processed.
//! * **No panics on any byte input** — pinned by deterministic fuzz-lite
//!   harnesses (random bytes + mutations of valid playlists and of real
//!   HLS playlists; inputs bounded well under 200 KiB).
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
pub use m3u::{MAX_LINE_BYTES, Parsed, parse, parse_bytes, parse_detailed, resolve_uri, write};
pub use pls::{
    PlsError, PlsParsed, PlsWarning, parse as parse_pls, parse_bytes as parse_pls_bytes,
    parse_detailed as parse_pls_detailed,
};
pub use types::{ByteRange, EncryptionMethod, Entry, HlsKey, HlsMap, HlsSegment, Playlist};
pub use xspf::{
    MAX_XML_DEPTH, XspfError, XspfParsed, XspfWarning, parse as parse_xspf,
    parse_bytes as parse_xspf_bytes, parse_detailed as parse_xspf_detailed,
};

mod cue;

pub use cue::{
    CueFile, CueFileKind, CueFlag, CueIndex, CueSheet, CueTrack, CueTrackType, CueWarning,
    FRAMES_PER_SECOND, TimeContext, TrackWindow, frames_to_ns, parse_cue, parse_cue_bytes,
};

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
