//! ASS (Advanced SubStation Alpha) support — split across two
//! file-disjoint cards (wave 6):
//!
//! * **5.M2a (parallel worktree):** the parser (`ass/parser.rs`) —
//!   ASS event/style text → the cue model. NOT in this worktree.
//! * **5.M2b (this worktree):** the plain-text render path —
//!   [`layout`] (the 1–9 numpad alignment grid + margins → the model
//!   and compositing positioning vocabularies) and [`render`]
//!   (plain-text cue → [`crate::compositing::CompositedCue`] with
//!   monospace-widest-char glyph metrics and the `&HAABBGGRR`
//!   PrimaryColour mapping).
//!
//! # [LEAD SEAM]
//!
//! This `mod.rs` is the 5.M2b half: it declares ONLY the render-path
//! modules, and the 5.M2a worktree owns `parser.rs` plus its own
//! `mod.rs` declaring `pub mod parser;`. At merge the Lead unions the
//! declarations (append `pub mod parser;` + any re-exports) — the
//! halves are file-disjoint by construction, and [`render`]'s entry
//! types (`AssRenderCue`/`AssRenderStyle`) are standalone for exactly
//! this reason: if the parser half grew same-named types, the Lead
//! deduplicates at the union, not in either worktree.
pub mod layout;
pub mod render;
