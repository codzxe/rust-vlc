# AUDIT: ui/src/screens/player/states/states.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 780 / 55 rule blocks (selectors) |
| File verdict | REAL (code) — UNREACHABLE from any shipped binary (W14) |
| Reachable from shipped entry point? | NO as product skin: compiled into `ui/dist` (baseline INDEX row 22, vite build OK) but the only loader is `apps/desktop-tauri` (`tauri.conf.json:7` `frontendDist: "../../ui/dist"`), which CANNOT COMPILE at HEAD (W14 SUMMARY l.19). Dev/vitest reach it: `states/index.ts:7` `import "./states.css"` → PlayerScreen. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg for `states.css` import (states/index.ts:7); rg every `lumen-state-*` class → all 54 base selectors consumed by StageOverlays.tsx/PlaylistBrowse.tsx/StateRouter.test.tsx; rg each `var(--…)` (30 tokens) → all defined in `ui/src/tokens/generated/tokens.css`; sed player.css:434-435 (verifies the cross-file claim); rg `--lumen-slider-fill/thumb` setters (Slider.tsx via states.md contract); read StateRouter.test.tsx:336-413 (CSS pin tests) |

## 1. One-paragraph verdict
Real, disciplined design CSS: every selector is consumed by the states/ components, every semantic token exists in the generated token sheet, the cross-file dependency it declares (player.css:435 `.lumen-player__canvas { position: relative }`) is present, and the W26 string-pinning tests in StateRouter.test.tsx:338-413 would fail if the skin regressed. The defects are maintenance debt, not fakeness: 20+ raw hex literals re-hard-code dark-theme token values for light-mode "picture-frame" pins (an admitted token-lane handoff), and the preview card re-implements the slider's internal geometry instead of sharing it. The overriding fact is reachability: this skin ships only inside `ui/dist`, whose sole consumer (desktop-tauri) does not compile at HEAD — no shipped binary can ever paint it.

## 2. What actually works (verified)
— verified by rg class-usage sweep → all 54 `lumen-state-*` base classes used in StageOverlays.tsx / PlaylistBrowse.tsx (+2 test files); no orphan selectors.
— verified by rg `--<token>:` in `ui/src/tokens/generated/tokens.css` → all 30 referenced custom properties defined (space-inset, color-surface-deep, radius-ring, motion-duration-fast, …).
— verified by `sed -n '430,440p' screens/player/player.css` → `.lumen-player__canvas { position: relative; …` present at 434-435, so the header comment's load-bearing claim (states.css:31-33) is true.
— verified by baseline INDEX rows 22-23 (`ui-build.log.txt` OK; `ui-vitest.log.txt` 1705 passed/1 skipped) → this sheet compiles and its pinning tests run green.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/states/states.css:291-330 (and 449-475, 602-625, 777-780)
Code:
```
:root[data-theme="light"] .lumen-state-hero {
  /* stylelint-disable-next-line color-no-hex -- frame: dark-mode --color-content-primary #c6c6c6; picture-frame rule (W25 spec §1) */
  color: #c6c6c6;
}
```
What is wrong: Light-mode "picture-frame" pins hard-code 20+ hex literals (`#c6c6c6`, `#eaeaea`, `#8f8f8f`, `#0e0e0e`, `#969696`, `#828282`, `#171717`, `#404040`, `#2b2b2b`) that DUPLICATE the dark theme's token values by literal instead of referencing a token. The comments themselves admit it: "Token-lane handoff (f15): a stays-dark canvas token would replace these pins" (l.313-314).
Why it matters: if the dark theme's token values ever change in `themes/tokens.json`, these literals silently drift — light mode would show stale colors with no lint, test, or build failure (the pin tests assert the same literals, so they lock the drift in). Value-duplication across 4 separate blocks multiplies the drift surface.
What correct looks like: a `--color-frame-*` (stays-dark canvas) token group generated once in `themes/tokens.json` and referenced here; the file's own comments already specify exactly this fix.
Confidence: HIGH — Verification: read (all four pin blocks) + token sheet cross-check.

### F-002 [S4]
Where: ui/src/screens/player/states/states.css:694-718
Code:
```
.lumen-state-preview__seek .lumen-slider__track {
  top: 0;
  height: 100%;
  background: var(--color-surface); /* frame: well #090909 */
  border-radius: 4.5px; /* frame: rx6 @1920 → ×0.75 — structural */
}
```
What is wrong: The preview card re-implements `.lumen-slider__track/__fill/__thumb` geometry (top/height/left/width formulas) in a second place (the first being `components/slider/slider.css`, including its own `--lumen-slider-fill` math at slider.css:75-91). Two independent copies of the fill/thumb positioning formula.
Why it matters: any change to the slider's internal layout contract (e.g. thumb size, inset math) must now be made twice; a miss produces a subtly broken mini-seekbar that no test renders visually.
What correct looks like: a shared modifier class (e.g. `.lumen-slider--mini`) inside slider.css, or a `--lumen-slider-*` custom-property API so consumers retheme without re-deriving geometry.
Confidence: HIGH — Verification: read both files; formulas compared (states.css:705 vs slider.css fill rule).

## 4. Per-function table (rule blocks; CSS has no functions — every selector gets a row)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-state-overlay` | 35-57 | REAL | — | scrim via color-mix 72%; hide-UI transition declared outside motion gate (correct: end-state for reduced-motion users) |
| `[data-chrome=hidden]` overlay fade | 65-70 | REAL | — | scoped to loading/buffering only; pin test asserts error/unsupported/hero excluded (StateRouter.test.tsx:383-388) |
| `__cluster` / `[data-card]` | 74-90 | REAL | — | panel grammar; all 4 tokens exist |
| `__icon/__title/__label/__body/__action` | 92-120 | REAL | — | token-only ink ramp |
| `__stagetag(-label/-value)` | 123-142 | REAL | — | used in StageOverlays |
| `__facts/__fact(-label/-value)` | 146-168 | REAL | — | Unsupported surface facts; consumers verified |
| `__progress/__spinner/__bar/__fill` | 172-206 | REAL | — | determinate-only bar; 28px/220px/4px structural px documented; `border-radius: 50%` functional circle |
| media no-preference / reduce + keyframes | 208-228 | REAL | — | spin + fill transition motion-gated; reduce kills overlay transition — reduced-motion safe |
| `.lumen-state-hero(+group/mark/title/body)` | 232-283 | REAL | — | 96px mark structural; display/regular grammar pinned by test |
| light-mode hero pins | 291-304 | REAL | — | F-001 literals |
| light-mode overlay pins | 315-330 | REAL | — | F-001 literals |
| `.lumen-state-home(+section/header/empty/row/cell)` | 342-396 | REAL | — | honest-empty third section confirmed in StageOverlays.tsx:485-490 |
| `__card(+album/hover/focus/art/title)` | 401-441 | REAL | — | focus-visible outline present; 237×133/150 structural |
| light-mode home pins | 449-475 | REAL | — | F-001 literals |
| `.lumen-state-browse(+head/title/count/exit/preview-toggle)` | 488-532 | REAL | — | progress bars deliberately absent (no resume seam — honest, per l.485-487) |
| `__grid/__cell` | 539-558 | REAL | — | `place-content: start start` prevents row stretch (documented, measured) |
| `__card/__card-art/__card-title/__empty` | 564-596 | REAL | — | 12px caption between tokens documented |
| light-mode browse pins | 602-625 | REAL | — | F-001 literals |
| `.lumen-state-preview` | 641-658 | REAL | — | 293px card; z-index 2 |
| `__video/__glyph` | 667-682 | REAL | — | placeholder art box (no second stream element — honest) |
| `__seek` + slider overrides | 687-723 | REAL | S4 | F-002 duplication |
| `__title[data-empty]` | 728-745 | REAL | — | honest null-title styling |
| `__meta/__chip(-text)/__hide` | 748-773 | REAL | — | consumed by PlaylistBrowse |
| light-mode `__video` pin | 777-780 | REAL | — | F-001 family |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| StateRouter.test.tsx:338 "overlays sit on a dark scrim…" | YES | regex-pins the exact color-mix/border/radius/shadow declarations in this file; deleting or altering the scrim breaks it |
| StateRouter.test.tsx:367 "hero… onboarding grammar" | YES | pins width:96px + token order; also negative-pins a forbidden `.lumen-button` radius override |
| StateRouter.test.tsx:374 "hide-UI… alerts never do" | YES | asserts the exact two-selector fade rule AND that error/unsupported/hero are NOT matched |
| StateRouter.test.tsx:392 "light mode keeps scrim dark" | YES (for drift from these literals) | pins the same hex literals — but this locks in F-001's duplicated values rather than guarding against token drift |
| StateRouter.test.tsx:405 "motion reduced-motion safe" | YES | pins both media-query blocks |
Mutation check (reasoned, not run): change `opacity: 0` to `opacity: 1` at l.67 or delete the `prefers-reduced-motion: reduce` block → at least 2 pin tests fail. These are string pins, not render tests — they prove the CSS text, not painted pixels; acceptable for a skin contract.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "W26 FIGMA REDESIGN … Semantic tokens only (§3.2); raw px appears solely for structural dimensions" (l.24-26) | file header | REAL | verified: only structural px (28/220/4/96/237/133/293/…) + the hex pins of F-001, each carrying a stylelint-disable + frame citation |
| "the overlays' absolute positioning relies on that one rule" (player.css:435) | l.31-33 | REAL | sed player.css:434-435 shows `.lumen-player__canvas { position: relative; … }` |
| "the canvas's mounted `<video>` surface is the one real media element" | l.660-661 | REAL (element exists) / MISLEADING (no shipped host) | MediaSurface.tsx:80 mounts `<video>`; but the desktop shell that renders it cannot compile (W14) and in a plain browser `FrameCanvasSurface` renders nothing (MediaSurface.tsx:106-108) |
| "the frame's … watched-progress bars have NO seam … deliberately absent" (l.485-487) | header comment | REAL | no progress-bar rules in this file; PlaylistBrowse renders art + title only |

## 7. Dead / duplicated / unreachable code
No dead selectors (54/54 consumed). Duplication: F-001 (token-value literals ×4 blocks), F-002 (slider geometry re-derivation). Unreachability: whole file ships only via desktop-tauri, which does not compile (W14 SUMMARY l.19-23).

## 8. Dependencies & flags
Imports nothing; depends on `ui/src/tokens/generated/tokens.css` (30 tokens, all present), `player.css` canvas rule, and the `.lumen-slider__*` class contract from `components/slider/slider.css`. No feature flags. stylelint `color-no-hex` suppressions: 16, each with a frame citation (checked individually).

## 9. Open questions for the next agent
- StageOverlays.tsx / PlaylistBrowse.tsx (owners of the data): do `session.opening-progress`/`session.buffering` ever deliver a real number from the engine bridge, or is `progress` always null in practice (indeterminate spinner forever)? That is the CSS-adjacent honesty seam this skin cannot answer.
- Who audits `useChromeAutoHide` (data-chrome writer)? The fade rules here are dead if the attribute never toggles.
