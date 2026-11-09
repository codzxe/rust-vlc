//! DASH streaming (Phase 6, wave 6.C), built on the 6.C1a MPD
//! model in [`playlists::mpd`].
//!
//! * `resolve` (card 6.C1b) — **SegmentTemplate/SegmentList/
//!   SegmentBase URL resolution**: `$RepresentationID$`/`$Number$`/
//!   `$Time$`/`$Bandwidth$` substitution with printf-style format
//!   tags, SegmentTimeline `t+n×d` expansion (positive and negative
//!   `@r`), attribute-wise template inheritance
//!   (Representation ⊕ AdaptationSet ⊕ Period) and RFC 3986
//!   `<BaseURL>` folding — a pure function of the parsed model plus
//!   the manifest URL, bounded by typed plan budgets.
//! * `fuzz` (card 6.C1c) — the in-crate fuzz harness
//!   ([`fuzz::fuzz_demux_mpd`]) the `fuzz_demux_mpd` cargo-fuzz
//!   target shims: one full untrusted parse + bounded resolution
//!   pass, panic-free by construction.
//! * `abr` (card 6.C2) — **buffer-based representation selection**:
//!   the hls `AbrController` hysteresis pattern (6.B2b/6.D1
//!   watermarks — EWMA throughput, up/down margin bands, dwell,
//!   stall-forced down-steps) over DASH representation ladders.
//! * `sched` (card 6.C2) — **segment scheduling + live sync**: the
//!   segment walk over resolved plans, and availability-time
//!   arithmetic (ISO 8601 availabilityStartTime + Period start +
//!   presentation-time offset → live edge).

pub mod abr;
pub mod fuzz;
pub mod resolve;
pub mod sched;

pub use abr::{
    DEFAULT_DOWN_MARGIN_PERCENT, DEFAULT_HIGH_WATERMARK_S, DEFAULT_LOW_WATERMARK_S,
    DEFAULT_MAX_BANDWIDTH_BPS, DEFAULT_MIN_DWELL_SEGMENTS, DEFAULT_UP_MARGIN_PERCENT, DashAbr,
    DashAbrConfig, DashAbrError, DashAbrSnapshot, SWITCH_HISTORY_CAP, select_representation_index,
};
pub use resolve::{DashResolveError, RepresentationPlan, SegmentPlan, plan_representation};
pub use sched::{
    DashSchedError, DashScheduler, FetchStep, NextStep, SAFE_BEHIND_LIVE_EDGE_S,
    live_edge_presentation_seconds, parse_iso8601_epoch_seconds, segment_availability_epoch,
};
