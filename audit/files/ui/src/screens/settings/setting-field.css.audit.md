# AUDIT: ui/src/screens/settings/setting-field.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 449 / 12 rule groups (≈40 selectors) |
| File verdict | UNREACHABLE from shipped binaries; REAL as a stylesheet (all selectors emitted by live components, tokens verified, one disclosed pending-token fallback) |
| Reachable from shipped entry point? | NO at HEAD (host crate unbuildable, W14-A01 E0425). Imported via settings.css / SettingField.tsx / SettingWidgets.tsx / area sheets; compiled + tested in baseline #22/#23. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | per-class emission census: `rg -l "<class>" src --glob '!*.css'` for all 16 class families → every selector emitted (settings-field:28 files, settings-slider*→SettingWidgets.tsx:159-189, settings-values*→SettingWidgets.tsx:290-327, settings-field__info→10 files, path/action classes→BrowseField.tsx:54 etc.); `rg -c "color-slider-tick" themes/tokens.json ui/src/tokens/generated/tokens.css` → 0 (pending token); component contract checks: Dropdown.tsx:352 emits `lumen-dropdown--${density}`, NumberField.tsx:79,93 emits `lumen-number__control/__steppers`, Dropdown.tsx:254 rowPitch math; sed verification of quotes. |

## 1. One-paragraph verdict
A disciplined override sheet: every selector targets a class a live component actually emits (16/16 families verified), the specificity strategy (one class deeper than the primitive sheets) is coherent, the specificity-critical overrides (dropdown trigger, input well, slider, values rows, path rows) all match their components' DOM, and reduced-motion is handled. Two defects, both small: the slider-tick rule references `--color-slider-tick`, a token that exists nowhere in the token pipeline (themes/tokens.json: 0 matches) — the stylelint-disabled `#1a1611` fallback is the live value, contradicting the sheet header's "Colors map to semantic tokens"; and the sheet pins settings dropdown option rows at 44px min-height while Dropdown.tsx's own flip-measurement math assumes a 42px compact pitch, under-estimating open-list height by ~2px per row in the very screens this sheet styles. Unreachable from shipped binaries like the rest of ui/src.

## 2. What actually works (verified)
— verified by class census (commands above): no orphan selector in the sheet.
— verified by `rg -n "lumen-dropdown--\${density}" Dropdown.tsx:352` + NumberField.tsx:79,93 + BrowseField.tsx:54: the `--compact`/`__steppers`/`__action` override targets are real component classes.
— verified by baseline #23: SettingWidgets/SettingField tests (in the 1705-pass set) render these rows; #22 compiles the sheet.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/setting-field.css:255-263 (esp. 260)
Code:
```
.settings-field__control .settings-slider__tick {
  position: absolute;
  top: calc(50% - 3px); /* 6px mark centered on the track centerline */
  width: 2px;
  height: 6px;
  background: var(--color-slider-tick, #1a1611); /* stylelint-disable-line color-no-hex -- frame: Settings - {playback,video,audio,interface}.svg tick marks #1A1611 (warm, sw2); token seam: --color-slider-tick pending (token lane) */
```
What is wrong: `--color-slider-tick` is declared NOWHERE — `rg -c "color-slider-tick" themes/tokens.json ui/src/tokens/generated/tokens.css` → 0/0. The hex fallback is therefore the production value, and the sheet header's "Colors map to semantic tokens … raw px is structural" (lines 21-24) is contradicted at this rule (disclosed at the site as a pending token seam).
Why it matters: When the token lane eventually adds `--color-slider-tick` with any other value, every settings slider's tick ink silently changes away from the frame-pinned #1A1611; conversely a theme author reading the header believes ticks are token-driven today.
What correct looks like: add the token to themes/tokens.json (gen-tokens pipeline) and delete the fallback, or drop the var() and keep the documented hex.
Confidence: HIGH — Verification: rg over the whole token pipeline; read.

### F-002 [S4]
Where: ui/src/screens/settings/setting-field.css:99-103 vs ui/src/components/menu/Dropdown.tsx:254-255
Code:
```
.settings-field__control .lumen-dropdown--compact .lumen-dropdown__option {
  min-height: 44px; /* frame: 44px rows (~48px pitch) */
```
and Dropdown.tsx:254-255:
```
    const rowPitch = density === "compact" ? 42 : 46; /* 44+2 / 40+2 — Dropdown.svg grammar */
    const estimated = Math.min(options.length * rowPitch + 8, DROPDOWN_LIST_MAX_HEIGHT);
```
What is wrong: Every settings screen uses `density="compact"` dropdowns, which this sheet re-pins to 44px option rows; the component's open-direction estimate still assumes 42px per compact row. A 10-row settings dropdown is estimated 428px but renders ~448px — the flip/preferred-side decision runs on a ~20px under-estimate in exactly the screens this sheet overrides.
Why it matters: Estimation drift can pick "down" when the list actually overflows the clipping container — the bug class Dropdown.tsx:245-247 says the measurement exists to prevent (w26.i16 HDR repro).
What correct looks like: the component measures `rowPitch` from the rendered list (or the sheet documents its 44px override into the component's pitch table).
Confidence: MEDIUM — Verification: read both files; runtime flip behavior UNVERIFIED (no browser).
Severity: S4 (estimation-only input, clamped by DROPDOWN_LIST_MAX_HEIGHT).

## 4. Per-function table (rule groups)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| dropdown width/trigger/chevron/value/hover/open | 29-103 | REAL | S4 | chevron via data-URI mask (no hex); ring-in-ring rx6/rx4 geometry documented; F-002 pitch drift |
| open list + option rows | 92-103 | REAL | S4 | F-002 |
| input well (text/number) + states | 112-139 | REAL | — | focus = accent border; hover/active quiet steps; matches TextField's `lumen-field__control` |
| number wrapper/steppers-hidden | 146-162 | REAL | — | steppers display:none; radii restored — NumberField.tsx:93 target verified |
| ⓘ info affordance + hint text | 171-203 | REAL | — | 24px WCAG 2.5.8 hit; focus ring; hinttext used by 13 files |
| settings slider family | 211-241 | REAL | — | re-pins shared slider track/fill/thumb to §C0 frame |
| slider ticks | 249-263 | REAL | S4 | F-001 pending token |
| slider scale + marks | 267-280 | REAL | — | aria-hidden decorative labels (SettingWidgets.tsx:185-189) |
| values radio rows (startup volume) | 288-343 | REAL | — | UA button font kill (w26.gfix-B) documented; row/button split verified SettingWidgets.tsx:303-327 |
| values nested input | 345-390 | REAL | — | 182×32 + 0.4 disabled wrap; focus ring |
| path rows (input+Browse) | 401-428 | REAL | — | 208+11+100 frame arithmetic documented; BrowseField.tsx:54 target verified |
| visually-hidden field label + reduced-motion | 433-449 | REAL | — | clip-path sr pattern; prefers-reduced-motion honored |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| areas.test.tsx / audio.test.tsx / codecs.test.tsx (settings rows) | partial | assert structure/labels/behaviors in jsdom — jsdom applies no real layout, so the geometry this sheet pins (44px rows, 320px columns) is untested; only class presence is exercised |
| baseline #23 | — | executed green |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Colors map to semantic tokens …; raw px is structural" | setting-field.css:21-24 | FALSE at one rule | F-001: tick #1a1611 fallback is live, token absent |
| "Selectors are deliberately one class deeper … so the settings skin wins on specificity alone, independent of bundle import order" | setting-field.css:3-9 | REAL | every override is `.settings-field__control <primitive>` (0,2,0+) vs primitive sheets (0,1,0) — verified per rule |

## 7. Dead / duplicated / unreachable code
No orphan selectors (16/16 class families emitted). Duplication: the sr-only label block (433-443) duplicates the app-wide sr pattern (player.css `.lumen-player__sr-note` family) — acceptable local variance. Whole sheet unreachable from shipped binaries (W14-A01).

## 8. Dependencies & flags
Consumes shared component classes (dropdown, field, number, browse, slider) + design tokens. `--color-slider-tick` is a phantom dependency (F-001). No stylelint-unchecked block besides the two disclosed disables (line 62 uses %23-encoded mask, no literal hex).

## 9. Open questions for the next agent
- Token-lane owner: is `--color-slider-tick` on any roadmap doc? (variants.css has the same pending-token pattern for stays-dark surfaces — a consolidated list would prevent each sheet re-inventing the handoff.)
- Dropdown.tsx auditor (menu lane): does any test pin the 42px compact pitch against real layout (Playwright visual runs exist — playwright.visual.config.ts)? If yes, F-002's drift is already observable there.
