# AUDIT: ui/src/screens/player/controls/controls.css#part1 (lines 1-382)
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = 1-382 (row, clusters, timecodes, play pill, seekbar, volume wedge — through the volume thumb suppression). Part2 = 383-837 (right-cluster popovers, app menu popup, error chip, motion) — W15-A03. Seam at the `right-cluster popovers` section header; no overlap. |
| Lines / functions | 382 / 27 selector rules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): ControlBar.tsx (mounted by PlayerScreen's bottom region, my part2 audit) and Volume.tsx consume these classes (`rg` hits in ControlBar.tsx:316/412/720, Volume.test.tsx:97). Baseline #22 build OK, #23 vitest 1705 passed incl. ControlBar.test.tsx + Volume.test.tsx. No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `Read` controls.css 1-382 (part1; section map via `rg -n "^/\* -+ "`); `rg -n "lumen-slider__thumb" -B8 -A10 slider.css` (thumb formula parity check — identical to the Lead-fix formula, documented); `rg -n "lumen-slider-fill" Slider.tsx` (custom property is a NUMBER — the `* 0.71%` calc is valid); `for t in …` token-existence loop (all 11 part1 tokens exist in themes/generated/tokens.css); `rg -c "--gradient-volume-level" ../themes/generated/tokens.css src/tokens/generated/tokens.css` (MISSING — F-001); `rg -n "stylelint-disable" src/ -g '*.css'` (20+ inline disables tree-wide vs the config's "ZERO" claim); `Read stylelint.config.js` (exception policy + rules); `rg -n "lumen-controls__output" src -g '*.tsx'` (used, ControlBar.tsx:720); `sed -n` verification of quoted ranges. |

## 1. One-paragraph verdict
Frame-faithful, mathematically careful control-bar styling: the seek-row shrink floor, the playhead flush formula (byte-identical to slider.css's Lead-fix math), the volume wedge's full-track gradient reveal (a width-scaled fill WOULD compress the ramp — the comment explains why the clip-path is right, and the `* 0.71%` calc is valid because `--lumen-slider-fill` is a number), and a specificity-escalation story for the play pill's on-accent ink that is documented to three past defects. One real process defect: the volume gradient is raw hex behind an inline stylelint disable whose recorded token handoff (`--gradient-volume-level`) never landed — the token does not exist in the generated output while slider.css:275 already consumes it with an accent fallback — and the stylelint config's own exception policy claims hand-written CSS carries "ZERO inline disables," flatly contradicted by 20+ disables tree-wide. Everything else verified: all tokens exist, all classes consumed, formulas check out.

## 2. What actually works (verified)
— Verified by read + arithmetic: the volume level reveal `clip-path: inset(0 calc(100% - var(--lumen-slider-fill, 0) * 0.71%) 0 0)` (371) reveals the gradient to `fill × 0.71%` of the track — at 100% volume, 71% of 89px ≈ 63px, matching the measured "100% = 63px" comment; the number × percentage calc is valid CSS (Slider.tsx:99 sets the var to a bare number).
— Verified by read: playhead formula (322) `left: calc((100% - 4px) * var(--lumen-slider-thumb, 0) / 100 - 6px)` is the same Lead-fix formula as slider.css's EQ thumb (slider.css:97-101) — right edge flush with the 2px-inset fill; no drift between the two sheets.
— Verified by token loop: all 11 custom properties used in part1 (`--space-inset`, `--space-grid`, `--space-track-inset`, `--font-family-time`, `--font-size-time`, `--color-content-quiet`, `--color-accent-bright`, `--color-accent-strong`, `--opacity-emphasis`, `--opacity-disabled`, `--color-content-on-accent`, `--color-stroke-icon`) exist in the committed generated tokens.
— Verified by `rg`: every part1 class is consumed (transport/right/volume/output clusters, time buttons, play pill, seek, volume-slider) by ControlBar.tsx / Volume.tsx.
— Verified by baseline #23: ControlBar.test.tsx and Volume.test.tsx run green (1705 passed total), including the volume-slider class-passing test (Volume.test.tsx:97-100).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/player/controls/controls.css:361-372; cross-file: ui/stylelint.config.js:13-19 (exception policy), ui/src/components/slider/slider.css:275 (token consumer), themes/generated/tokens.css (token absent)
Code:
```css
  /* stylelint-disable-next-line color-no-hex -- frame: volume.svg — the measured signal gradient #95D318→#B2A918(57.9%)→#D37918(79.2%)→#FF3737(100%). Green/red are volume/EQ-only signal colors (NOT theme accents — analysis §B8 note), so they deliberately do not map to accent tokens; handoff: --gradient-volume-level (slider.css seam, token lane). */
  background: linear-gradient(90deg, #95d318 0%, #b2a918 57.9%, #d37918 79.2%, #ff3737 100%);
```
What is wrong: three-part drift. (1) The inline disable's recorded handoff — "handoff: --gradient-volume-level (slider.css seam, token lane)" — never landed: `rg -c -- "--gradient-volume-level" themes/generated/tokens.css src/tokens/generated/tokens.css` finds NOTHING; slider.css:275 already consumes `var(--gradient-volume-level, var(--color-accent))` and silently gets the accent fallback. (2) ui/stylelint.config.js's exception policy states "Hand-written CSS carries ZERO inline disables — every raw color in src/**/*.css is a lint error" and "New genuinely-unexpressible values do NOT get inline disables: they get semantic tokens" — this line (and 20+ other disables across src/**: form.css ×4, onboarding.css ×3, playlist.css ×3, options.css, subtitles.css, surface.css, appshell.css, list.css, icon.css ×3) contradicts the policy the config itself documents. (3) The token-discipline gate (`npm run lint:css`) was not run in the baseline (INDEX #21-24 list neither lint nor lint:css), so the "gate" status is UNVERIFIED while its documented policy is FALSE.
Why it matters: the gradient renders correctly today, but the repo's color-discipline story ("semantic tokens only" — this file's own header, CONTRACT §4.4) is enforced by a config whose written policy is already false, with a consumer (slider.css) waiting on a token that was promised in a comment and never delivered — classic comment-driven development with no follow-through. The volume/EQ signal colors are also theme-invariant raw hex, so a theme that retunes accents will leave this gradient untouched by design — but that design lives only in a lint-disable comment, not in the token system where it belongs.
What correct looks like: add `--gradient-volume-level` to themes/tokens.json (the parse.rs audit shows gradient tokens are a supported category), regenerate, delete the inline disable — and either enforce the "ZERO disables" policy mechanically (stylelint `reportNeedlessDisables`/comment rule) or rewrite the policy to admit the documented-disable pattern.
Confidence: HIGH — Verification: read (token absence verified by rg on both generated files; policy text read; consumer read). lint:css not run — REQUEST-ORCH-RUN: `cd ui && npm run lint:css` (expect: passes only because of the inline disables; would confirm).

### F-002 [S4]
Where: ui/src/screens/player/controls/controls.css:339-355
Code:
```css
/* volume.svg: 89px wedge track (left cap ≈11px tall → right cap ≈28px) on
 * surface; the level fill is the measured signal gradient anchored to the
 * FULL track, revealed to the level — no thumb, hard fill edge. */
.lumen-controls__volume-slider {
```
What is wrong: the wedge's left cap is drawn with `clip-path: polygon(0 calc(50% - 5.5px), …)` (355) — hard 5.5px math inside a clip-path means the silhouette's aliasing and the track's `border-radius: 2px` (352) interact (the clip cuts the corners the radius rounds). Cosmetic-only; the frame's wedge is itself a hard-edged silhouette.
Why it matters: hygiene note; no functional defect found in the geometry (11px cap = ±5.5px from center, arithmetic correct).
What correct looks like: nothing required; if the frame is ever re-measured, keep the cap math and the radius in one comment.
Confidence: LOW — Verification: read (cosmetic interaction reasoned, not rendered — UNVERIFIED visually).

## 4. Per-function (selector) table
| Selector | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-controls` | 34-45 | REAL | — | Row anatomy; ~44px single-row invariant; anchors floating children. |
| `__transport` | 49-54 | REAL | — | Pill pitch via space tokens. |
| `__right`/`__volume`/`__output` | 56-64 | REAL | — | All consumed (ControlBar.tsx:412/720, Volume.tsx). |
| `__seek` | 74-80 | REAL | — | e18 F1 fix: `min-width: max-content` floor; documented overflow tradeoff below floor. |
| `.lumen-controls__time` | 88-102 | REAL | — | Tabular timecodes; 4.5ch floor; constant padding (no layout jump on hover chip). |
| `button.__time` (+focus/hover/active/disabled/[data-editing]) | 110-152 | REAL | — | gfix-C real-button faces; edit caret ::after 1px accent — frame Vector 76. Correct. |
| `.__play` (+[data-glyph] states) | 158-200 | REAL | — | Play/pause pill: accent vs content-quiet (frame #6B6B6B); one-step hover/active progressions documented. |
| `.__play:disabled` (+pause variant) | 207-214 | REAL | — | Opacity-carried inertness. |
| `[data-playback-state="idle"]` overrides | 224-243 | REAL | — | Empty-state saturated bar (Player.png); ghost transport (0,6,0) with `:not(.__play)` — the g04 P1-3 2.5:1 contrast fix, documented. |
| `(0,4,0)/(0,5,0)` on-accent ink pins | 265-274 | REAL | — | k07 consolidated pin; specificity story vs icon.css/button.css documented to three past defects. |
| `[data-playback-state="buffering"]` | 277-279 | REAL | — | Truthful buffering hint (opacity-emphasis). |
| `__seek .lumen-seekbar` + `__track`/`__fill` | 287-307 | REAL | — | 28px r6 track; fill `calc((100% - inset·2)·p/100)` — correct inset math. |
| `__seek .lumen-slider__thumb` | 318-326 | REAL | — | 8×24 bar; Lead-fix flush formula, parity with slider.css verified. |
| `__seek .lumen-seekbar__buffer` | 330-335 | REAL | — | Engine-reported buffer sub-regions, quiet 16% mix. |
| `__volume-slider` (+track/gradient/fill/thumb) | 342-381 | REAL | F-001, F-002 | 89px wedge; full-track gradient revealed to level (math verified); fill/thumb suppressed; raw hex behind never-landed token handoff. |

## 5. Test honesty (ControlBar.test.tsx / Volume.test.tsx — behavior tests over this bar; ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Volume.test.tsx:97-100 (class passing) | YES | Asserts the volume-slider class reaches the slider root — the wedge/gradient selectors' hook. |
| ControlBar.test.tsx transport/cluster assertions (class hooks at 316/412/720) | YES | DOM structure pins the class contract styled here. |
| CSS itself has no visual regression test in this suite (Playwright visual configs live elsewhere — w15-g003, another agent) | — | The gradient/wedge/pill visuals are comment-documented only; pixel parity is UNVERIFIED in baseline. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (CONTRACT §4.4); raw px below are structural" (header 26-27) | this file | FALSE (partially) | The volume gradient is raw HEX colors (367), not just raw px geometry — exempted only by the inline disable F-001 documents. |
| "The ONE raw color stop set (the volume signal gradient) carries an inline stylelint disable + frame citation — token-lane handoff recorded" (27-30) | this file | REAL (the disable) / FALSE (the handoff) | Disable present at 366; the `--gradient-volume-level` token it hands off to does not exist (rg on both generated tokens files). |
| "Hand-written CSS carries ZERO inline disables — every raw color in src/**/*.css is a lint error" | stylelint.config.js:13-19 | FALSE | 20+ inline disables in src/**/*.css (rg list in F-001); this file's own 366 among them. |
| "100% volume ends at ≈71% of the track … the level never reaches the red zone" (369-370) | this file | REAL | Clip-path arithmetic verified: reveal = fill × 0.71%. |

## 7. Dead / duplicated / unreachable code
- No dead selectors in part1 (all classes consumed).
- Duplicated formula (deliberate, documented): the thumb flush formula mirrors slider.css — the comment cross-references it; acceptable duplication with an explanatory record.
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Depends on the shared slider API (`--lumen-slider-fill`/`--lumen-slider-thumb` number-valued custom properties, verified in Slider.tsx:99-100), icon button grammar, and 12 semantic tokens (all exist). One stylelint disable (F-001).

## 9. Open questions for the next agent
- Part2 (W15-A03): the popovers/app-menu/error-chip sections — check the popover `z-index` layering against MenuPopup's and the error chip's placement vs `.lumen-controls { position: relative }` (35).
- Token lane owner: F-001's `--gradient-volume-level` — also consumed with fallback at slider.css:275; landing the token fixes both sites.
- REQUEST-ORCH-RUN: `cd ui && npm run lint:css` — baseline never ran it; result would evidence whether the "gate" passes only via the documented disables.
