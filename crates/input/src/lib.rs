//! The Lumen local byte-source layer (card 2.A1; doc 03 §4.1, doc 01 §2.4).
//!
//! Turns a validated [`SourceUri`](media_types::ids::SourceUri) into a
//! positioned, seekable byte stream over **local** sources: `file:` URIs
//! opened through `std::fs` only. Network protocols (HTTP(S), RTSP, …)
//! are the `streaming` crate's Phase-6 domain and are rejected here with
//! typed errors, never silently mis-opened.
//!
//! # What this crate owns
//!
//! - [`uri`] — `file:` URI resolution to a canonical local path (scheme
//!   rejection, authority rules, percent-decoding, symlink
//!   canonicalization).
//! - [`file`] — [`FileAccess`] (the `plugin_api::Access` implementation
//!   doc 01 §2.4 assigns to this crate) and the raw [`FileSource`] it
//!   wraps.
//! - [`buffered`] — [`BufferedReader`]: the fixed-capacity, bounded-memory
//!   reader. Its buffer is allocated once and never grows; small reads
//!   are served from the window with read-ahead locality, while requests
//!   at or above the capacity (the demux arena's payload class, 7-c
//!   part 2) bypass the window — leftover read-ahead drains first, then
//!   the source is asked directly for the remainder in caller-visible
//!   memory (one transfer instead of `want / capacity` window refills),
//!   and requests above a hard ceiling are refused with a typed
//!   `ResourceLimit` error (MP §12: bound memory).
//! - [`seekable`] — the [`Seekable`] trait (position/length/typed seek)
//!   and the [`RawRead`] chunk primitive the reader is generic over.
//! - [`contract`] — the scripted synthetic source + conformance checks
//!   (the codec-api `contract` pattern): every access implementation —
//!   here, in `streaming` (Phase 6), and in the conformance kit — can
//!   reuse the same executable contract.
//! - [`error`] — constructors that *mint* `media_types::EngineError`
//!   values with input-specific context codes. The taxonomy itself is
//!   media-types' (doc 01 §3.4) and is never duplicated here.
//!
//! # Design decisions (recorded for review)
//!
//! - **The landed contract wins.** Doc 03 §4.1's sketch (`ByteStream`
//!   with deadline/cancel parameters, `ReadErr`) was superseded by the
//!   card-1.D2 `plugin_api::Access`/`SourceStream` traits: pull-driven
//!   positional reads returning `Vec<u8>`, short reads as data, `Ok(0)`
//!   only at end of stream, cancellation by generation staleness (not
//!   tokens), blocking work confined to the caller-owned source thread
//!   (doc 01 §4.1). This crate implements the landed traits.
//! - **Truncation is a typed error, never a silent EOF** (card AC): the
//!   reader remembers the length observed at open; if the underlying
//!   source hits end-of-stream before that length is consumed, reads
//!   surface `ErrorKind::Corrupt` with `CorruptDetail::Truncated`.
//! - **No registry hook yet**: doc 01 §2.4's `register(&mut
//!   PluginRegistry)` waits on the registry itself (plugin-api scope
//!   fence: doc 06 §3 is the Phase-2 engine card).
//! - **Paths are UTF-8**: `file:` URI paths are percent-decoded and then
//!   required to be UTF-8, keeping the crate free of platform-specific
//!   `OsStr` byte APIs (ARCH-6). Non-UTF-8 filesystem names are a
//!   recorded v0 limitation.
//! - **Secrets never enter errors**: constructors take no path or URI
//!   text; `Display` stays structured (MP §12). OS detail rides the
//!   `source` chain of `EngineError`.

pub mod buffered;
pub mod contract;
pub mod error;
pub mod file;
pub mod seekable;
pub mod uri;

pub use buffered::{BufferedReader, DEFAULT_BUFFER_BYTES, MAX_BUFFER_BYTES, MAX_READ_BYTES};
pub use contract::{ScriptSource, SyntheticStream};
pub use file::{FileAccess, FileSource, FileStream};
pub use seekable::{RawRead, Seekable};
pub use uri::resolve_file;
