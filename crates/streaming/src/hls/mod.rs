//! HLS streaming (Phase 6, wave 6.B).
//!
//! * `media` (card 6.B1a) — **VOD media playlists**: fetch the
//!   playlist over the [`HttpFetch`](crate::fetch::HttpFetch) port,
//!   type it through the playlists crate's HLS model, and serve the
//!   segment concatenation as one continuous
//!   [`SourceStream`](plugin_api::SourceStream) so the existing
//!   container/probe/decode machinery is reused, not duplicated.
//!
//! Later cards land here on top: live windows + discontinuity
//! (6.B1b), master playlists/variant selection (6.B2a), records +
//! conformance (6.B3).

pub mod media;
