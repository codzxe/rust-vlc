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

mod error;
mod m3u;
mod types;

pub use error::{ParseError, ParseWarning};
pub use m3u::{MAX_LINE_BYTES, Parsed, parse, parse_bytes, parse_detailed, resolve_uri, write};
pub use types::{ByteRange, EncryptionMethod, Entry, HlsKey, HlsMap, HlsSegment, Playlist};
