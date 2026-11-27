<!--FIXV1
agent: FX9-R2
title: Remedial compile fixes for the FX9-A03 subtitle lane in apps/cli/src/pipeline_host.rs (4 errors from the central build)
verdict: DONE
changed:
- apps/cli/src/pipeline_host.rs
tests:
- (EMPTY — remedial compile fixes; central build is the oracle)
evidence_read:
- /tmp/fx9-cli3.log
-->
# FX9-R2 — remedial compile fixes (pipeline_host.rs, 4 errors)

## Approach
Read /tmp/fx9-cli3.log first, then the four error sites in
apps/cli/src/pipeline_host.rs plus the compositing crate's real API
(crates/subtitles/src/compositing/layout.rs — `Rational::new` at :49,
`Anchor::new` at :98) and the `EngineError` conversion surface
(`From<TimeError>` only, crates/media-types/src/time.rs:700; no
`From<CueError> for EngineError` anywhere in the workspace — verified by
rg). Applied the four minimal fixes below; no seam signatures, no
compositor call order, no test expectations touched. rustfmt was
permitted but not present on this box
(/home/z/my-project/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
missing), so formatting follows the file's existing hand-rustfmt style.

## Fix per error
1. **:88 unused import `SubtitleOverlay` (denied).** Grep showed the only
   other mention is the prose doc comment at :1939 (`A04's
   `SubtitleOverlay` payload`), and `reconstruct_embedded` reaches the
   overlay via `cue.overlay` field inference — the import is genuinely
   unused. Removed it from the `media_types::frame` use list; the other
   three names in that line stay (used).

2. **:1387-1388 E0308 (binding `cues` with two types across one
   arm-pair).** Split the or-pattern into two separate match arms so each
   arm's `cues` gets its own concrete type
   (`Text { cues } => cues.len()` and `Bitmap { cues } => cues.len()`).
   Both arms compute the identical `cues.len()`, so `cue_count()`
   semantics (the empty-load refusal's truth, callers at :1503/:1606)
   are preserved exactly.

3. **:1981-1991 E0308 (`Rational` vs `Result<Rational, CueError>`).**
   Drift note: the task brief said ":1983 already has a `?`" — in the
   actual file AND the log BOTH args were bare
   `Rational::new(x, extent_w)` / `Rational::new(y, extent_h)` (the
   compiler notes flag 1982:9 and 1983:9). The compiler's bare-`?` help
   is also infeasible here: `reconstruct_embedded` returns
   `Result<Option<CompositedCue>, EngineError>` and no
   `From<CueError> for EngineError` exists (rg-verified; the subtitles
   crate only has `From<CueError> for AssRenderError`,
   crates/subtitles/src/ass/render.rs:208). Fix: each `Rational::new`
   argument now unwraps via the file's own error-mapping pattern —
   `.map_err(|err| host_error(PipelineStage::Present,
   "cli.subtitle_overlay", "the embedded overlay's placement refused:
   {err}"))?` — matching the adjacent `Anchor::new` mapping already on
   the call, so both args arrive as `Rational` per layout.rs:98. Error
   surface unchanged (same stage/code/message family), and since
   `x <= extent_w` / `y <= extent_h` are pre-clamped with
   `extent >= 1`, the mapping is unreachable in practice anyway.

4. **:2047-2059 E0521 (borrowed data escapes the closure).** The
   file-local `TextFragment<'a>` held `text: &'a str`; the
   `push_fragment` closure (param `text: &str`, higher-ranked) pushed
   fragments borrowed from its own parameter into `lines`, declared
   outside — a mutable `Vec<Vec<TextFragment<'_>>>` is invariant, so
   the borrow cannot escape. Fix per the task's first suggestion:
   `TextFragment` now owns its text (`text: String`, struct no longer
   generic), the closure pushes `piece.to_string()`, and `lines`
   becomes `Vec<Vec<TextFragment>>` (no lifetime parameter, nothing
   borrowed escapes). Line splitting, colour/deco handling, the
   retain-empty-lines rule (:2090) and the rasterize loop
   (`fragment.text.chars()` — String has `chars()`) are byte-for-byte
   the same behaviour; only ownership changed. `TextFragment` is
   private to this file (only uses at :2014/:2047/:2055, rg-verified).

## Risks & followups
- No cargo/rustc available to me (wave rules), so the central build
  remains the sole oracle; the fixes were type-checked by hand against
  the real signatures in crates/subtitles/src/compositing/layout.rs.
- The triple-repeated `placement refused` map_err block at :1982-2004
  is verbose but matches the file's dominant style; a shared local
  closure would deduplicate it if a later wave wants polish.
- The doc comment at :1939 still mentions `SubtitleOverlay` in prose —
  intentional (it documents the payload type by name, no import
  needed).
