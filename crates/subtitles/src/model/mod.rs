//! The cue model (card 5.L2): the shared vocabulary every subtitle
//! format parser (5.L1) projects into and every renderer consumes.
//!
//! Modules:
//!
//! * [`time`] — [`CueInterval`], half-open `[start, end)` microseconds
//!   with saturating arithmetic and overlap semantics;
//! * [`position`] — [`Alignment`], [`Percent`], [`LinePlacement`],
//!   [`Positioning`] and the relative→absolute resolution primitives;
//! * [`style`] — [`TextStyle`] (bold/italic/underline/color subset)
//!   and [`Color`];
//! * [`cue`] — [`Cue`], [`CueId`], [`CueText`], [`StyledSegment`];
//! * [`wire`] — the stable serialize/parse roundtrip form;
//! * [`error`] — the typed failure taxonomy.
//!
//! No panics anywhere in this module tree: every fallible operation
//! returns [`Result`] with a typed error (MP §8).

pub mod cue;
pub mod error;
pub mod position;
pub mod style;
pub mod time;
pub mod wire;

pub use cue::{Cue, CueId, CueText, StyledSegment};
pub use error::{ModelError, WireError};
pub use position::{
    Alignment, HorizontalAlign, LinePlacement, Percent, Positioning, ResolvedAnchor, VerticalAlign,
    Viewport,
};
pub use style::{Color, TextStyle};
pub use time::CueInterval;
pub use wire::{parse_document, serialize_document, write_document};
