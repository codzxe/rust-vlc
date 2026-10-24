//! The subtitle compositing contract (card 5.L1): placement, z-order,
//! timing and the reference blend over the [`ComposeTarget`] seam.
//!
//! Pipeline shape: a caller (engine glue, a later card) holds decoded
//! cues as [`CompositedCue`]s — an RGBA8 [`CueBitmap`] carrying its
//! baseline row, plus a [`CueLayout`] (anchor, alignment, safe margin,
//! reference extent) and a z-order key. Per presented video frame it
//! calls [`SubtitleCompositor::compose`] with the clock's `now()`;
//! the compositor filters by the half-open `[t0, t1)` interval
//! ([`timing`]), sorts by z, computes each cue's destination rectangle
//! ([`layout`]) and blends ([`target`]) onto the renderer's rows.
//!
//! Module map:
//! - [`layout`] — safe areas (5 % title / 10 % action), rational
//!   anchors, baseline alignment, uniform scaling with the video
//!   extent, safe-area clamping: [`place_cue`].
//! - [`bitmap`] — the cue payloads and their typed construction
//!   errors; ns conversion happens once, at build.
//! - [`timing`] — `active_at`: the `[t0, t1)` half-open rule.
//! - [`target`] — the [`ComposeTarget`] trait (R4 seam), the
//!   reference [`Rgba8Surface`], and the over-opaque blend.
//! - [`compose`] — [`SubtitleCompositor`]: the stateless-contract,
//!   persistent-scratch driver.

pub mod bitmap;
pub mod compose;
pub mod layout;
pub mod target;
pub mod timing;

pub use bitmap::{CompositedCue, CueBitmap, CueError};
pub use compose::SubtitleCompositor;
pub use layout::{
    Anchor, CueAlignment, CueLayout, HorizontalAlign, Rational, Rect, SafeMargin, VerticalAlign,
    place_cue,
};
pub use target::{ComposeError, ComposeTarget, Rgba, Rgba8Surface};
