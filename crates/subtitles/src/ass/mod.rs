//! ASS/SSA parsing — the plain-text subset (card 5.M2a, half of 5.M2).
//!
//! Scope decision (doc 04 §5, refined by feasibility §5 §2.2's tag
//! coverage matrix): v1 parses the ASS/SSA **syntax** faithfully —
//! sections, style lines, dialogue events with `H:MM:SS.CC` times —
//! and reduces every event's text to the model's **plain-text**
//! payload ([`crate::model::CueText::Plain`]). Override-tag blocks
//! `{\...}` are stripped and *counted* per tag name (the §2.2
//! `drop (counted)` disposition), never a parse failure; `\N`/`\n`
//! become newlines and `\h` a hard space. **Full styling is
//! DEFERRED to the Phase-5 styling-subset card** — fonts, colors
//! beyond the primary, karaoke, `\pos`/`\move` placement, rotation,
//! border/shadow, fades and vector drawing are recorded as dropped
//! tags here, not interpreted (R-09, tracked).
//!
//! The companion half of 5.M2 (parallel agent, file-disjoint) owns
//! the render path; this module is the parser core only.
//!
//! # Damage model
//!
//! The parser is infallible: [`parse`] always returns a document.
//! Malformed input degrades with *typed damage events*
//! ([`AssDamage`]) — a bad `Start`/`End` token skips that one cue,
//! everything else keeps going (doc 06 §7.8 tolerance; MP §8: no
//! panics on any input).

pub mod parser;

pub use parser::{AssDamage, AssDocument, AssStats, AssStyle, StyleFlavour, parse};
