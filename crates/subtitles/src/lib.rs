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

pub mod compositing;
