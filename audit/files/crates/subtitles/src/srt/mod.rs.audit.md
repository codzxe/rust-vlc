# AUDIT: crates/subtitles/src/srt/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 67 / 0 prod (module facade: 6 pub mods, 4 re-export groups, 1 doctest) |
| File verdict | REAL-UNPROVEN in product — accurate facade over real, executed child modules; UNREACHABLE from shipped entry points (crate unlinked, parser.rs F-001) |
| Reachable from shipped entry point? | NO — no app/engine links `subtitles`; only fuzz/ (fuzz_sub_srt target exists: fuzz/fuzz_targets/fuzz_sub_srt.rs, verified by ls) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg '^pub (fn|struct|enum|const)' over render.rs/damage.rs/encoding.rs (exit 0 — re-export list verified); rg doctest in baseline (exit 0) |

## 1. One-paragraph verdict
A documentation-and-re-export facade; all substance lives in the child modules I audited
(parser.rs) or read for context (encoding, tags, damage, render, fuzz). Every concrete claim
in this file's docs checks out: the never-error/never-abort/never-panic contract is executed
truth (my probes + 2000-round mutation loop + 37-fixture corpus), the tolerance list here
("`.`-vs-`,` milliseconds, glued `-->`, rolled hours, 1–3-digit ms") omits parser.rs's false
"decimal-fraction scaling" sentence and is accurate, and the re-export lists match the
child modules' real public items. No findings of its own; the crate-level wiring defect and
parser defects are filed under parser.rs.

## 2. What actually works (verified)
— Doctest at lines 35-48 passes: baseline cargo-test-workspace.txt:6165
  (`test crates/subtitles/src/srt/mod.rs - srt (line 35) ... ok`); its assertions
  (clean 1-cue parse, 1_000_000 µs start, garbage-block tolerance) were re-proven by my
  direct execution in the parser.rs audit (probe_srt).
— Re-exports verified present in children: SrtRenderError/TextBoxLayout/TextMetrics/
  cue_layout/place_plain_text/plain_text (render.rs:56-282), MAX_DAMAGE_EVENTS/SrtDamage/
  SrtDamageKind (damage.rs:17-121), SourceEncoding (encoding.rs:35), SrtDocument/parse/
  parse_str (parser.rs) — rg, exit 0.

## 3. Findings
None of its own. Zero-findings justification — checked and how:
- Contract claims ("never returns an error / never aborts / never panics"): executed in the
  parser.rs audit (5 fixtures + 8 hostile inputs + baseline 2000-round mutation loop) — all hold.
- Tolerance list accuracy: cross-checked against executed behavior (parser.rs F-002 shows the
  false wording lives in parser.rs's doc, not here).
- Re-export completeness/accuracy: rg over child modules (above).
- Registry claim ("registry fuzz target fuzz_sub_srt"): fuzz/fuzz_targets/fuzz_sub_srt.rs
  exists and delegates to srt::fuzz::fuzz_sub_srt (read during parser audit).
- "Never: Font stacks, rasterization, GPU paths — geometry only": srt::render uses only
  geometry + the crate's placeholder metrics (read for context) — consistent.

## 4. Per-function table
Not applicable (0 functions; facade). Items: pub mod damage/encoding/fuzz/parser/render/tags
(lines 55-60); pub use groups (lines 62-67) — all verified against children.

## 5. Test honesty
The doctest (lines 35-48) would fail if the parser regressed on the clean path, the garbage
path, or the µs conversion — real assertions, not tautologies.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "parse … never returns an error, never aborts … never panics" | lines 26-31 | REAL | executed (parser.rs audit §2) |
| tolerances "`.`-vs-`,` ms, glued `-->`, rolled hours, 1–3-digit ms" | lines 12-15 | REAL | parser tests + my probes (note: parser.rs's own doc misstates `,5` semantics — parser.rs F-002) |
| "registry fuzz target `fuzz_sub_srt`" | lines 2-3 | REAL | fuzz/fuzz_targets/fuzz_sub_srt.rs exists (ls) |
| "the renderer owns fonts" | lines 19-20 | REAL | srt::render consumes crate::font placeholder metrics only |

## 7. Dead / duplicated / unreachable code
None in-file. render.rs's `text_box` (line 243) and `MAX_PROBE_PIXELS` (line 50) are pub but
not re-exported — still reachable via `srt::render::` (pub mod), not dead.

## 8. Dependencies & flags
Std-only; no features; no unsafe. Child module sizes: parser 567, render ~300+, encoding 390,
tags ~500+, damage ~170, fuzz 47 — the facade exposes all of them publicly.

## 9. Open questions for the next agent
- srt/render.rs and srt/tags.rs have not been audited by me beyond context reads — their
  layout/box math and entity handling are open (note vtt/text.rs F-001's quadratic scan
  pattern; tags.rs shares the `while let Some(at) = rest.find(...)` shape — check it).
