<!--FIXV1
agent: FX9-R1
title: Rewrite FX9-A02's guessed swash 0.2.10 call sites against the real crate API (10 compile errors, contract unchanged)
verdict: DONE
changed:
- crates/subtitles/src/font/swash.rs
tests:
- (EMPTY — remedial API fix; central build is the oracle)
evidence_read:
- /tmp/fx9-sub2.log (the orchestrator's error log for this task)
- audit/fix/RULES.md
- audit/fix/waves/FX9/FX9-A02.md (the original assignment this remediates)
- crates/subtitles/tests/fx9_a02_font_stack.rs (the behavioral oracle suite)
- crates/subtitles/src/font/pipeline.rs (the frozen GlyphPipeline seam)
- swash-0.2.10 sources at /home/z/my-project/.cargo-fresh/registry/src/index.crates.io-1949cf8c6b5b557f/swash-0.2.10/src/: lib.rs, font.rs, metrics.rs, scale/mod.rs, scale/image.rs, shape/mod.rs, shape/cluster.rs, Cargo.toml
- zeno-0.3.3 src/geometry.rs (Placement field types)
-->

# FX9-R1 — swash.rs rewritten against the REAL swash 0.2.10 API

## Approach

The 10 errors were all guessed-API call sites in
`crates/subtitles/src/font/swash.rs` (FX9-A02's code half was written
without the crate source available; the log at `/tmp/fx9-sub2.log`
lists them). The real swash 0.2.10 source is now unpacked in the
registry, so I read the actual API surface first (lib.rs re-exports,
`scale/mod.rs`, `shape/mod.rs`, `shape/cluster.rs`, `metrics.rs`,
`scale/image.rs`, `font.rs`, zeno's `Placement`), then rewrote only the
wrong call sites inside swash.rs. Nothing outside swash.rs was touched;
the `GlyphPipeline` contract (name/metrics/advance/rasterize/overhang
signatures and behavior), the `new(&'static [u8], u32) -> Option<Self>`
constructor seam consumed by `font/mod.rs`, and all decoration/mask
geometry (untouched code below `shape_char`/`render_glyph`) are exactly
as FX9-A02 wrote them — the FX9-A02 test suite
(`tests/fx9_a02_font_stack.rs`) remains the behavioral oracle and ran
against only the public `subtitles::font` API, so it is unaffected by
the internal restructuring. Deny lints respected (no `.unwrap()`, no
new imports unused, no TODO markers); rustfmt does not exist in this
sandbox's toolchains (verified — consistent with FX9-A02's risk #1),
so the file is hand-formatted to the repo's ≤100-column convention
(machine-checked here: no line > 100, no tabs, no trailing whitespace).

## What the real API looks like (one line per corrected call)

- `scale::RenderContext` does not exist in 0.2.10 — import dropped
  with the whole `render_ctx` field; rendering goes through
  `swash::scale::Render`, a builder that takes `&[Source]` in priority
  order and a `&mut Scaler` built from the `ScaleContext`.
- `FontRef::from_index` takes `usize` (font.rs:172), not `u32` —
  `index as usize` at the seam boundary (u32→usize is lossless on
  every target the workspace builds for).
- `ScaleContext` has no `with_font`; the builder is
  `ScaleContext::builder(font).size(px).build() -> Scaler` (and
  `Scaler` has no `metrics()` method at all in 0.2.10) — the
  construction-time face ratios now come from the crate-root
  `FontRef::metrics(&[])` + `Metrics::scale(REFERENCE_EM_PX)`
  (metrics.rs:291), the same linear scaling `Shaper::metrics()` applies
  internally.
- the construction mismatch (`scale_ctx: ScaleContext` vs the
  `RefCell<ScaleContext>` field) — both contexts are now built
  inline as `RefCell::new(ScaleContext::new())` /
  `RefCell::new(ShapeContext::new())`.
- `ShapeContext` likewise has no `with_font`; the shaper comes from
  `ShapeContext::builder(font).size(em).build() -> Shaper`
  (shape/mod.rs:313, 440, 518), borrowed through the existing
  `RefCell` (`ctx.builder(self.font)`, `FontRef` is `Copy`).
- `CharCluster::push` is `pub(super)` and there is no `glyphs()`
  method on `CharCluster` (text/cluster/cluster.rs:241) — feeding a
  single character now uses the crate's own documented simple-run path:
  `Shaper::add_str(ch.encode_utf8(&mut scratch))` (which runs the real
  parser + charmap), then `Shaper::shape_with(|gc| ...)` and the first
  `gc.glyphs.first()` (`Glyph { id: GlyphId, advance: f32 }`,
  shape/cluster.rs:22-35) — `None` when the cluster is empty (control
  chars / dropped ignorables), preserving the old fallback contract.
- `scale::Source` has no `Box` variant — the render source list is
  `&[Source::Outline, Source::Bitmap(StrikeWith::BestFit)]` (outline
  first, an embedded bitmap strike as the honest fallback;
  `StrikeWith` is now actually used, resolving the unused-import
  error).
- `GlyphId` is a plain `pub type GlyphId = u16;` alias at the crate
  root (lib.rs:95), not a tuple constructor — the `.notdef` fallback is
  `.unwrap_or((0, metrics.advance_px as f32))`.
- the render call itself: `Render::new(&[...]).render(&mut scaler,
  gid) -> Option<Image>` with `Image { placement: zeno::Placement
  {left: i32, top: i32, width: u32, height: u32}, data: Vec<u8> }`
  (default `Format::Alpha` → `Content::Mask`, `width*height` samples —
  exactly what the unchanged blit loop consumes; `render` feature is on
  via swash's default features, per the admitted dependency row).

## Risks & followups

1. **Compile-reviewed, not compiled** — this sandbox forbids cargo
   (RULES.md) and has no rustc/rustfmt, so every corrected call above
   was verified by reading the real source signatures, not by the
   compiler. The orchestrator's central build is the oracle; if
   anything remains, it will be a signature-level detail, not a design
   one (the whole flow is now the crate's own documented pattern from
   scale/mod.rs's and shape/mod.rs's doc examples).
2. **Behavioral deltas are intentional-but-narrow**: metrics now come
   from `FontRef::metrics` (hhea/OS/2) rather than a scaler object
   (which doesn't expose metrics in 0.2.10) — same `Metrics` source
   `Shaper::metrics()` would materialize, so the FX9-A02 face-derived
   inequality tests keep their wide margins; the `Source::Box`
   fallback is replaced by `Source::Bitmap(StrikeWith::BestFit)`
   (DejaVu Sans has no embedded bitmaps, so the outline source always
   serves — the fallback is a safety net, not a behavior change).
3. **The module/field doc comments were updated** where they named the
   guessed API ("render strike"/"session") — doc-only, no code
   semantics behind them.
