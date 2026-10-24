//! THE LUMEN SUBTITLES CRATE (cards 5.L1 + 5.L2, doc 04 §5).
//!
//! **Scaffold note (Lead-ratification required):** the parallel wave-5
//! cards 5.L1 (text-format parsers: SRT/WebVTT-family) and 5.L2 (cue
//! model) each create this crate in their own worktree; the Lead unifies
//! the scaffold at wave close. This branch (`wip/p5-l2`) lands card
//! 5.L2's [`model`] module; 5.L1's parser modules are file-disjoint.
//!
//! Card 5.L2 scope — the shared cue model:
//!
//! * [`model::Cue`] — identity ([`model::CueId`]), a half-open timing
//!   interval [`model::CueInterval`] (`[start, end)`, microseconds), a
//!   text payload ([`model::CueText`]: plain or styled), and
//!   [`model::Positioning`];
//! * positioning/alignment primitives — the 3×3 [`model::Alignment`]
//!   matrix, line/position [`model::Percent`]ages, and the
//!   relative→absolute resolution into [`model::ResolvedAnchor`] px;
//! * the style-token subset \[PROPOSAL\] — [`model::TextStyle`]:
//!   bold/italic/underline/color (RGBA8);
//! * a stable wire form ([`model::wire`]) whose roundtrip
//!   (cue → serialize → parse → equal) is the card's acceptance test.
//!
//! Everything is pure data + pure functions: no I/O, no threads, no
//! unsafe (workspace lint: `deny`), no panics on any input — failure is
//! typed ([`model::ModelError`]/[`model::WireError`]), per MP §8.

pub mod model;
