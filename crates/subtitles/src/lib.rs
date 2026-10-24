//! Lumen subtitles: the compositing contract (card 5.L1, doc 04 §5
//! "compositing contract"; doc 01 §2.2 registry row `subtitles`, L2).
//!
//! # What this crate owns in v0 (the 5.L1 half)
//!
//! The overlay pipeline's CONTRACT, not the rendering: placement math
//! (safe areas, anchor/baseline alignment, scaling with the video
//! extent), z-ordering, sample-exact `[t0, t1)` timing against the
//! shared clock, and one reference alpha-blend implementation over a
//! renderer-implemented [`ComposeTarget`] seam. Text shaping, fonts and
//! the GPU path stay in `renderer-wgpu` (doc 04 §5: "text rendering
//! itself is a renderer concern"); the parser/cue-model half (5.L2)
//! lands beside this module. Doc 04 §5's "bitmap compositing
//! dependency" tracking item and risk R-27's overlay-contract milestone
//! are satisfied BY this contract: PGS/DVB/VobSub bitmaps and any
//! renderer-rasterized text cue reduce to an RGBA8 [`CueBitmap`] plus a
//! [`CueLayout`], and the blend itself never touches video-engine.
//!
//! # The seam (R4)
//!
//! [`ComposeTarget`] is the one trait a renderer implements to receive
//! composited subtitle pixels: `extent()` + one `row_rgba()` call per
//! touched destination row. The blend math lives HERE, once — no
//! video-engine or renderer-wgpu edit is needed or allowed by this
//! card; the renderer adapter (a later card) forwards its rows.
//!
//! # Honesty notes
//!
//! * Scaling is point-sampled nearest-neighbor with pixel-center
//!   mapping — no gamma-domain conversion, the same honesty as
//!   renderer-wgpu's software compositor (bilinear belongs to the
//!   linear-light shader path).
//! * Blending composites straight-alpha RGBA8 over an OPAQUE video
//!   plane, in byte space, rounding to nearest. This is associative
//!   for cue-over-cue-over-opaque-video (z stacking) and is what
//!   subtitle overlays do in practice; linear-light blending is the
//!   GPU path's concern.
//! * All placement math is exact integer/rational arithmetic — no
//!   floats anywhere near a pixel (MP §8: determinism over
//!   approximation). The AC tests are pixel-exact by construction.
//!
//! # Never
//!
//! I/O, device APIs, UI, font stacks (doc 01 §2.2 d — the registry
//! row's "Never: rendering (that is presentation)" is honored by
//! owning the contract and the reference blend, never a window).

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

pub mod compositing;
pub mod model;

// 5.M2b (wave 6): the ASS plain-text render path — `ass/layout.rs`
// (numpad alignment grid + margins) and `ass/render.rs` (plain-text
// cue -> CompositedCue). The parallel 5.M2a parser half is
// file-disjoint; see `ass/mod.rs` for the [LEAD SEAM] note.
pub mod ass;
