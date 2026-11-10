//! `apps/web/src/io` — the web I/O pillar (card 7.H4, issue #48; doc
//! 09 §6).
//!
//! # What this module tree is
//!
//! The browser byte-transport lane, mirroring `crates/streaming`'s
//! proven Range/seekability design (card 6.A2a/6.A2b:
//! `RangedResource`, `crates/streaming/src/ranged.rs`) for the
//! fetch-shaped browser path:
//!
//! - [`fetch_port`] — the typed fetch seam ([`FetchRequest`]/
//!   [`FetchResponse`]/[`FetchPort`]): the browser twin of streaming's
//!   `HttpFetch` port, shaped for what `fetch()` actually answers
//!   (status + headers + whole body, redirects followed by the
//!   browser, `final_url`/`redirected` observable).
//! - [`seekability`] — the seekability *detection* (doc 09 §6.1:
//!   "detected, not assumed"): one ranged probe `GET` answered by a
//!   `206 Partial Content` ⇒ seekable; a `200` full body ⇒ the
//!   no-range server (forward-only, VLC-style); `416` at offset 0 ⇒
//!   the empty resource. Mirrors `ranged.rs`'s probe semantics and
//!   its typed protocol-violation refusals.
//! - [`staged_source`] — the [`SourceStream`] the engine reads
//!   network media through: fetch responses are **staged** into
//!   bounded windows (the message-transport adaptation — the browser
//!   `fetch()` is async and lives in the Worker's JS glue, so bytes
//!   cross the boundary as `FetchResponse` messages correlated by
//!   request id; the pump gates engine polls on the source's
//!   readiness so a synchronous demuxer read never sees an unstaged
//!   byte).
//! - [`mock_fetch`] — the mock fetch/Response test double (test-only,
//!   `#[cfg(test)]`): the mirror of streaming's `mockhttp` harness
//!   pattern — scripted routes (range-honoring, no-range, truncating,
//!   redirecting, fixed-status), a request log tests assert on, and
//!   the `deliver` harness that answers the host's outstanding
//!   `FetchRequest`s through the REAL message path.
//!
//! # The transport split (why a port AND messages)
//!
//! In a browser, the Worker's JS glue performs `fetch()` — the wasm
//! module has no imports and no sockets (doc 09 §6.3). The Rust side
//! therefore *models* the transport: it mints typed
//! [`FetchRequest`]s (surfaced upstream for the glue to serve) and
//! absorbs typed [`FetchResponse`]s. [`FetchPort`] is the in-process
//! seam that keeps that contract testable: the mock implements it,
//! and the same request/response types ride the port contract — one
//! vocabulary, no drift.
//!
//! # CORS note (doc 09 §6.1/§9.1)
//!
//! `Range` is not a CORS-safelisted header: every ranged fetch on
//! cross-origin media triggers a preflight, and the server must
//! answer `OPTIONS` with `Access-Control-Allow-Headers: Range`. The
//! transport (browser or mock) enforces CORS; this layer only
//! records it in the docs, because the wasm side never sees origins.
//!
//! # Evidence labels
//!
//! Everything here is proven headless-native against the mock double
//! (the seekability matrix, the bounded-memory proofs, and the
//! fetch-lane playback proof over the REAL first-vertical fixture).
//! Real-`fetch()` behavior — CORS, preflight, redirect following,
//! streaming bodies — is `[VERIFY]` (no browser in this sandbox;
//! card 7.I1's matrix owns the browser rows).

pub mod fetch_port;
#[cfg(test)]
pub mod mock_fetch;
pub mod seekability;
pub mod staged_source;

pub use fetch_port::{
    DEFAULT_CHUNK_BYTES, DEFAULT_WINDOW_BYTES, FetchError, FetchPort, FetchRequest, FetchResponse,
    RangeRequest,
};
pub use seekability::{Seekability, detect_seekability};
pub use staged_source::{MAX_READ_BYTES, StagedConfig, StagedSource, StagedStats, WindowRequest};
