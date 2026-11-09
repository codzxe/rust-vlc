//! HLS streaming (Phase 6, wave 6.B).
//!
//! * `media` (card 6.B1a) — **VOD media playlists**: fetch the
//!   playlist over the [`HttpFetch`](crate::fetch::HttpFetch) port,
//!   type it through the playlists crate's HLS model, and serve the
//!   segment concatenation as one continuous
//!   [`SourceStream`](plugin_api::SourceStream) so the existing
//!   container/probe/decode machinery is reused, not duplicated.
//!   Since 6.B1b it also records `#EXT-X-DISCONTINUITY` seams
//!   instead of refusing them (typed boundaries + stats).
//! * `live` (card 6.B1b) — **live media playlists** (no
//!   `#EXT-X-ENDLIST`): the sliding-window session — live-edge join,
//!   demand-driven reload under a stalled-request budget, `ENDLIST`
//!   termination, typed live-timeout / lost-window refusals, and the
//!   same recorded-discontinuity contract.
//! * `master` (cards 6.B2a/6.B2b) — **master playlists + variant
//!   switching**: the variant ladder selected by a bandwidth cap,
//!   and an adaptive session that switches rungs at segment
//!   boundaries under a hysteresis band (timeline continuity via
//!   absolute media sequence; seams recorded as discontinuities).
//!
//! Later cards land here on top: records + conformance (6.B3).

pub mod live;
pub mod master;
pub mod media;
