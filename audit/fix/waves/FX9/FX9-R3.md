<!--FIXV1
agent: FX9-R3
title: Replace nonexistent ModelColor::RED/BLUE constants with ModelColor::new opaque-red/blue in fx9_a05 test
verdict: DONE
changed:
- crates/subtitles/tests/fx9_a05_real_glyph_render.rs
tests:
- (EMPTY — remedial constant fix; central run is the oracle)
evidence_read:
- audit/fix/RULES.md
-->
## Approach
Read RULES.md, then verified all four sites (442, 447, 750, 755) with sed windows before touching anything. Confirmed the real API in `crates/subtitles/src/model/style.rs:29` exposes only `const fn new(r,g,b,a)`, `BLACK`, `WHITE` — no `RED`/`BLUE` — so the defect is exactly as scoped, with no drift. The intended colours are pure opaque red/blue: the styled-run assertions at :766/:767/:771 compare against `Rgba::RED` (r=255,g=0,b=0,a=255) and `Rgba::BLUE` (r=0,g=0,b=255,a=255) in `compositing::target`, and the first site (:442/:447) uses the same runs for free-vs-twin equivalence, so the constructor literals must be `new(255,0,0,255)` / `new(0,0,255,255)`.

## The fix
Replaced `ModelColor::RED` → `ModelColor::new(255, 0, 0, 255)` at :442 and :750, and `ModelColor::BLUE` → `ModelColor::new(0, 0, 255, 255)` at :447 and :755 — nothing else in the file changed (rg confirms no `ModelColor::RED`/`BLUE` remain and the constructor already appears elsewhere in the file at :294–:303 in the same style). rustfmt is not installed in this sandbox (no ~/.cargo/bin, no rustup toolchain), but the edits are single-line, same-indentation substitutions matching the file's existing formatting; longest line is 52 columns.

## Risks
No risk from the change itself — the literals mirror `Rgba::RED`/`Rgba::BLUE` channel-for-channel and the surrounding pixel assertions stay untouched. Cross-file observation for the orchestrator (outside my ownership, NOT changed by me): `crates/subtitles/src/compositing/target.rs:67` currently reads `#ust_use = "a computed color nobody reads is a test bug"]` — a mangled `#[must_use]` attribute that is a syntax error; re-derivable via `rg -n '#ust_use' crates/subtitles/src/compositing/target.rs`. If that line is live at central compile, the wave will fail there regardless of this fix.
