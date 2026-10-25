# Subtitle record — ASS/SSA (plain-text subset parse + render)

Status: **partial** — plain-text subset per the doc 04 §5 decision
(styling DEFERRED, recorded; the R-09 tag-coverage gap). Cards 5.M2a
(parser half) + 5.M2b (render half), merged by the Lead. Feature-
matrix row `subtitle-ass-ssa` was `not_started` ×4 with empty
evidence at wave-9 open — backfilled to `partial` by the 5.Q2-V
records audit (this file is that evidence). Front-matter keys per
doc 04 §6:

- id = `subtitle-ass-ssa` (`docs/feature-matrix.toml`)
- status = partial (plain-text parse + placeholder-glyph render;
  working subset described below)
- phase = 5 (landed, wave 6)
- fuzz_target = `fuzz_sub_ass` (doc 04 §8.5 planned name; in-crate
  fuzz-lite harnesses only — cargo-fuzz absent, Lead-owned wiring)
- crates = [] — zero external deps
- self_built = [`crates/subtitles/src/ass/**` (mod/parser/layout/
  render)]
- last_audit = 2026-10-25 (5.Q2-V; engine evidence 5.M2a + 5.M2b)

## Working subset (what "partial" means)

- **[Script Info]** key/value (BOM/CRLF/`;`-comment tolerant);
  **[V4+ Styles]/[V4 Styles]** Format+Style — Name/Fontname/
  Fontsize (exact i64 thousandths, no floats), PrimaryColour
  (`&HAABBGGRR`/short-hex/decimal/negative-wrap → straight RGBA8,
  ASS alpha inverted), Bold/Italic/Underline (-1 convention),
  Alignment BOTH conventions (ASS numpad 1–9; legacy SSA 1–11),
  MarginL/R/V; **[Events]** Format+Dialogue in ANY field order
  (Text absorbs commas only as last declared field).
- **Times**: `H:MM:SS.CC` → µs exact (1 cs = 10 000 µs); fractions
  1–6 digits; lenient M/S>59 (libass-style linear); overflow/
  garbage → typed BadTime skip. 0:01:02.75 → 62 750 000 µs pinned.
- **Plain text**: `{\...}` stripped with per-tag-name drop counters
  (the R-09 conformance artifact); `{}` preserved (VSFilter quirk);
  unterminated `{` literal (libass); `\N`/`\n` → newline, `\h` →
  space; UTF-8 scalar walk.
- **Render** (5.M2b): 1–9 numpad grid → compositor `CueLayout` with
  EXACT rationals (margins = anchor fractions of PlayRes 384×288,
  libass-style scaling); TitleSafe clamp on top of explicit margins
  (documented Lumen deviation, test-pinned); PrimaryColour → style
  token; placeholder block glyphs (monospace-widest-char metrics:
  advance 3/5 em, box 6/5 em, ascent 80% — all integer ratios, no
  floats); 64 MiB bitmap cap; typed `AssRenderError` (12 variants).

## Corpus / differential evidence (5.M2a)

| Sample | Origin | Result |
|---|---|---|
| basic multiline | ffmpeg 7.1.5 `-c:s ass` from hand-written SRT | exact counts/times/texts PASS |
| unicode + literal `{\b1}` passthrough | ffmpeg 7.1.5 | PASS (`{\b1}html{\b0}` → "html", dropped_tags["b"]=2) |
| 6-cue ladder | ffmpeg 7.1.5 | PASS + style facts (Arial/16000 milli/alignment 2/margins 10) |

- 5.M2b: 9/9 alignment-grid positions pixel-exact through the
  compositor's own `place_cue` (hand-derived 24×24 cue @ 384×288);
  margin-0 TitleSafe clamp engaged; 2× extent scaling; e2e
  render→place→blend draws expected pixels on a real surface.
- Fuzz-lite: 2000 deterministic runs (5.M2a junk+mutations incl.
  `\pos`/`\k10`/日本語/`{}`) + 2048 splitmix64 junk (5.M2b) — zero
  panics, ids dense, intervals valid, accepted cues place in-extent.
- Gates at cards: 85/0 (M2a) and 81/0 (M2b); fmt + clippy clean.
  Current tree (5.N2 session, re-verified 5.Q2-V): 230/0 with
  `ass_ffmpeg_ac` 4 green.

## Deferred (honest, per doc 04 §5 / feasibility §5 §2.2)

ALL override-tag interpretation (fonts, colors beyond primary,
karaoke, `\pos`/`\move`, rotation, borders, fades, drawing — dropped
+ counted); Layer z-order, Name/Effect, margin overrides counted
unmapped; WrapStyle reflow; PlayRes-aware margin math → the Phase-5+
styling card. libass cross-differential IMPOSSIBLE (C — forbidden by
§9); no real fansub-grade corpus lane exists. Placeholder glyphs
under-estimate CJK wide-width (one cell per scalar). libass mid-
alignment (4–6) MarginV policy unverified (conservative center,
test-pinned).

## Next steps

Styling card consumes `AssStyle` raw fields + `AssStats::dropped_tags`;
real font stack replaces GlyphMetrics + fill loop only.
