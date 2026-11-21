# AUDIT: ui/src/components/form/form.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 622 / 47 selector blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — imported by 8 form components (TextField.tsx:3, BrowseField.tsx:3, NumberField.tsx:3, Checkbox.tsx:3, RadioGroup.tsx:4, SegmentedControl.tsx:4, SwatchGroup.tsx:5, ValueChip.tsx:2) plus screens (NewPlaylistDialog.tsx:25, NetworkStreamScreen.tsx:47); routes /playlist, /open-network-stream, /settings/* mount them (App.tsx:302-322). CAVEAT: the tauri host binary that would ship this UI cannot compile at HEAD (W14; baseline build-desktop-tauri.log.txt exit 101), so no shipped desktop binary reaches it; browser/dev runs do. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg -c -- '--<token>:' src/tokens/generated/tokens.css (every var form.css consumes — all exist EXCEPT --color-surface-input, --color-stroke-input); rg -n 'lumen-field__control|lumen-browse__|lumen-number__|lumen-choice--radio|lumen-segmented__|lumen-swatchgroup__|lumen-valuechip' src (all classes consumed); sed -n '62,68p;573,576p;103,104p' (quote verification); sed -n '120,265p' tokens.css (accent hex cross-check) |

## 1. One-paragraph verdict
A real, densely-documented component stylesheet: every selector class is rendered by a real form component, every semantic token it consumes exists in src/tokens/generated/tokens.css except the two input-well seams it explicitly documents as pending, and the nine hardcoded accent-swatch hexes exactly match the generated per-accent `--color-accent` primaries (verified red #e15032, yellow #c49e23, green #19a234, cyan #18aad3, blue #185dd3, violet #8e42ff, pink #f052b3, gray #afafaf). The state matrix (hover/active/focus-visible/disabled + prefers-reduced-motion + light-mode remediation) is present for every interactive primitive. The two token seams never landed, so raw dark-only hexes are the live values in every dark build; the file is honest about it (states.md, stylelint-disable markers), which caps severity at S3.

## 2. What actually works (verified)
— rg over tokens.css: 30 of 32 consumed custom properties exist; swatch hexes match generated accent blocks. All 14 class grammars (`lumen-field`, `lumen-browse`, `lumen-number`, `lumen-choice`, `lumen-radiogroup`, `lumen-segmented`, `lumen-swatchgroup`, `lumen-valuechip` + element parts) are rendered by components (rg results above). Baseline: `npm test` 1705 passed incl. the colocated state-matrix suites (audit/runs/baseline/ui-vitest.log.txt), `npm run build` OK (ui-build.log.txt).

## 3. Findings
### F-001 [S3]
Where: ui/src/components/form/form.css:62,68,205
Code:
```
  border: 1px solid var(--color-stroke-input, #3a3a3a); /* stylelint-disable-line color-no-hex -- frame: W26 input border #3a3a3a; token seam: --color-stroke-input pending (token lane) */
```
```
  background-color: var(--color-surface-input, #2a2a2a); /* stylelint-disable-line color-no-hex -- frame: W26 input fill #2a2a2a */
```
What is wrong: `--color-surface-input` and `--color-stroke-input` do not exist anywhere in src/tokens/generated/tokens.css (rg count 0 for both), so the "pending token lane" comment has been true since w26 and the raw frame hexes are the live values in every dark-mode build — the header's contract "Semantic tokens only (contract §4)" (form.css:26) is violated by the file itself at three sites (62, 68, 205).
Why it matters: the documented seam is not temporary debt that resolves automatically; any theme that is not dark-remediated (the light block at 603-617 covers it) or future token rename silently keeps #2a2a2a/#3a3a3a. A generated token rename audit (`tokens:check`) will not catch it because the token never existed.
What correct looks like: land `--color-surface-input`/`--color-stroke-input` in the generated token set (dark + light values) and delete the hex fallbacks, or delete the var() indirection and state plainly that the frame hex IS the spec.
Confidence: HIGH — Verification: rg -c over tokens.css = 0 for both names; read the file.

### F-002 [S4]
Where: ui/src/components/form/form.css:103-104
Code:
```
/* error state: --color-record stays the only red semantic token (dedicated
 * --color-danger is a recorded token-seam request, states.md). */
```
What is wrong: error affordances (aria-invalid border, error caption, error focus ring) reuse `--color-record`, a token whose semantic name means "recording indicator".
Why it matters: naming that lies — a future theme tweak of the record color (e.g. pulsing red) mutates every form error affordance; consumers can't distinguish "error" from "recording".
What correct looks like: a `--color-danger` token; the file itself records this request but it has not landed.
Confidence: HIGH — read; token absent from tokens.css.

### F-003 [S4]
Where: ui/src/components/form/form.css:573
Code:
```
  background: var(--color-surface-deep); /* frame: dark #141414 chip ≈ audio-value well */
```
What is wrong: comment admits the frame value is #0e0e0e/#090909 (line 553-554) but ships `--color-surface-deep` with an "≈" approximation note; in light mode `--color-surface-deep` flips to #bababa (tokens.css light block), so the "dark recessed value chip" design becomes a light chip — intentional per the light sweep (589-602) but the ValueChip is also used in the EQ overlay on the near-black player surface, where a #bababa chip is a deliberate inversion, untested by any light-mode visual on that surface.
Why it matters: documented approximation drift; minor.
What correct looks like: a dedicated `--color-surface-well` token for recessed readouts (same seam family as F-001).
Confidence: MEDIUM — read; light-mode visual on the player surface UNVERIFIED (no baseline run covers it).

## 4. Per-function table (selector blocks)
| Selector block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-field` | 33-40 | REAL | — | consumed TextField/BrowseField/NumberField |
| `.lumen-field__label` | 42-45 | REAL | — | rendered by all three field components |
| `.lumen-field__control` (+hover/active/focus/focus-visible/placeholder/disabled/aria-invalid) | 53-111 | REAL | S3 (F-001) | token seams raw; state matrix complete |
| `.lumen-field__error` | 113-118 | REAL | S4 (F-002) | record-red reuse |
| `.lumen-browse` | 125-129 | REAL | — | |
| `.lumen-browse__control` | 131-134 | REAL | — | composes field control |
| `.lumen-browse__action` (+states) | 136-173 | REAL | — | 100×32 frame-marked |
| `.lumen-number` | 179-181 | REAL | — | |
| `.lumen-number__control` | 183-186 | REAL | — | radius surgery composes with field control |
| `.lumen-number__steppers` | 188-191 | REAL | — | |
| `.lumen-number__stepper` (+first/last/hover/active/focus-visible/disabled) | 193-236 | REAL | S3 (F-001 at 205) | 2×17px=34px math correct, border-box |
| `.lumen-choice` (+hover/active) | 242-259 | REAL | — | |
| `.lumen-choice__input` (+focus-visible) | 261-273 | REAL | — | native accent-color |
| `.lumen-choice--disabled` | 275-278 | REAL | — | |
| `.lumen-choice--error .lumen-choice__input` | 280-282 | REAL | S4 (F-002) | outline-color only |
| `.lumen-radiogroup` | 288-300 | REAL | — | fieldset reset |
| `.lumen-radiogroup__options` | 306-314 | REAL | — | |
| `.lumen-choice--radio` (+label/hover/checked/hover-unchecked/input/focus-ring) | 316-363 | REAL | — | `:has()` selectors — WebKit 15.4+/GTK webkit2gtk 2.40+ OK for tauri target |
| Switch section (comment only) | 365-371 | REAL | — | correctly documents skin moved to button.css; Switch.test.tsx:98 enforces absence |
| `.lumen-segmented` | 378-385 | REAL | — | |
| `.lumen-segmented__segment` (+checked/hover/active/focus/disabled) | 387-429 | REAL | — | |
| `.lumen-choice--error .lumen-segmented__segment:focus-visible` | 436-438 | REAL | S4 (F-002) | |
| `.lumen-swatchgroup` | 446-453 | REAL | — | |
| `.lumen-swatchgroup__swatch` (+checked/active/focus/disabled) | 455-487 | REAL | — | |
| `.lumen-swatchgroup__dot` | 489-494 | REAL | — | 20×20 rx1 |
| 9 × `data-accent` dot fills | 501-538 | REAL | — | hexes match tokens.css accent primaries exactly (verified) |
| `.lumen-swatchgroup__swatch[aria-checked="false"] .dot` | 543-545 | REAL | — | --opacity-swatch-rest: 0.7 exists (tokens.css:92) |
| `.lumen-swatchgroup__swatch:hover .dot` | 547-549 | REAL | — | |
| `.lumen-valuechip` | 559-576 | REAL | S4 (F-003) | |
| `@media (prefers-reduced-motion)` | 578-587 | REAL | — | covers all 6 transitioned primitives |
| light-mode block | 603-622 | REAL | — | contrast values documented (4.7:1/1.75:1/7.8:1) |

## 5. Test honesty
No tests in this file, but colocated suites (TextField.test.tsx:65-79, BrowseField.test.tsx:70-79, SegmentedControl.test.tsx:107-126, SwatchGroup.test.tsx:113-134, ValueChip.test.tsx:29-31, Switch.test.tsx:98) regex-match this stylesheet's text. These are literal-text assertions against the CSS source, not rendering assertions — they pass while the CSS is syntactically present even if a cascade conflict breaks the visual. They DO fail if a rule is deleted/mutated (mutation: change `--color-surface-hover` to any other token in the hover rule → TextField.test.tsx:65 fails). Honest but shallow coverage style; the a11y/render tests in the same files carry the real weight. Those test files belong to other agents' waves.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (contract §4)" | form.css:26 header | FALSE (self-referentially) | F-001: two raw hexes live because tokens never landed |
| Switch skin "lives in button.css" | form.css:4-7, 365-371 | REAL | rg: no `.lumen-switch` rules in form.css; Switch.test.tsx:98 enforces |
| "9 accent swatches, frame-measured" | form.css:496-500 | REAL | hex-by-hex match with tokens.css accent blocks |
| labels localized via i18n keys switch.on/off | form.css:11 | REAL-UNPROVEN (other agents' files) | useSwitchLabels.ts is in asset-group w15-g003; not executed here |

## 7. Dead / duplicated / unreachable code
None found: every selector grammar has a rendering consumer (rg evidence in header). The `.lumen-field__label--hidden` rule removal (47-51) is documented as gone; verified absent.

## 8. Dependencies & flags
Pure CSS, no imports. Depends on src/tokens/generated/tokens.css cascade (all 32 vars enumerated; 30 exist). stylelint-config-standard (color-no-hex disables are inline and scoped).

## 9. Open questions for the next agent
- Who owns the "token lane" that owes --color-surface-input/--color-stroke-input/--color-danger/--color-slider-tick? (setting-field.css:247 references the same debt.) If that lane is paper-only, F-001 upgrades to S2 across all consumers.
- Does any light-mode visual baseline cover the ValueChip on the player surface (F-003)?
