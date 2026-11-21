# AUDIT: ui/src/theme/lumen.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 259 / 8 sections (root type, numerals, scrollbars, selection, focus floor, separator primitive, reduced motion, light-mode role corrections 8a-8f) — 20 rule blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (main.tsx:32 `import "./theme/lumen.css"` — after tokens.css, before a11y; also re-imported by ThemeProvider, Vite-deduped, per main.tsx:29-31 comment). Every mount of the app applies it. Product caveat: no desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n 'lumen.css' src/main.tsx` (:32, chain order verified); `rg -ln 'lumen-separator'` (consumer: Surface.tsx:406 Separator component + Surface.test.tsx:481-488); `rg -ln '"tabular"\|"timecode"'` (Row.tsx, ValueChip.tsx, PlaylistPanel.tsx, GotoTimeDialog.tsx); Read Slider.tsx:82-111 (root = `.lumen-slider` + merged className, track INSIDE — the decisive fact for F-001) + ControlBar.tsx:368/477-484 (`.lumen-controls__seek` WRAPS the Seekbar; VolumeSlider root carries BOTH `lumen-slider` and `lumen-controls__volume-slider`); `rg -n 'hover' slider.css` (:306 `.lumen-slider:hover .lumen-slider__track` — the correct grammar the dead rule inverted); `rg -n 'app-shell__appdot\|app-shell__rail' appshell.css` (:68 — override targets exist); `rg -n '\-\-color-surface:|\-\-color-focus-ring:|\-\-color-stroke-icon:' tokens.css` (dark #090909/#d37918/#8f8f8f, light #c2c2c2/#824505/#6b6b6b — every value the 8a-8f comments cite matches the generated pipeline). |

## 1. One-paragraph verdict
A disciplined base sheet that keeps every promise its header makes except one: colors/fonts are all `var(--…)` (raw values are geometry only — verified line by line), the token pipeline is composed never overridden (every light-mode section-8 value matches the generated tokens.css exactly), the specificity math in the comments is correct (8a's (0,3,0) beats component focus rules), and the two deliberate duplications (body margin with appshell.css, focus floor with a11y/focus.css) are documented with owners. The one real defect is section 8b's hover rule: its selectors are compound-inverted (`.lumen-slider:hover .lumen-controls__seek .lumen-slider__track` requires the slider to be an ANCESTOR of the seek cluster; the actual DOM is the reverse — ControlBar.tsx:368 wraps the Seekbar), so the documented light-mode hover lift for the seek and volume tracks matches NOTHING and light mode silently falls back to slider.css's dark-frame hover value. Plus one stale header phrase ("#090909 once the W25 token set lands" — it landed) and the acknowledged body-margin duplicate.

## 2. What actually works (verified)
— verified by `rg -n '\-\-color-surface:…' tokens.css` → dark `--color-surface:#090909`, light `--color-focus-ring:#824505`, light `--color-stroke-icon:#6b6b6b` — the exact values the 8a/8f comments cite; the "composes, never overrides" claim holds for every section-8 rule (all re-point ROLES to existing tokens).
— verified by Surface.tsx:406 + Row.tsx/ValueChip.tsx/PlaylistPanel.tsx/GotoTimeDialog.tsx → both primitive classes (.lumen-separator, .tabular/.timecode) have real consumers; no dead primitives (the never-adopted .lumen-panel/.lumen-row were already deleted per j18 F7, header 10-12).
— verified by main.tsx:25-37 → import chain: tokens.css → lumen.css → a11y/focus.css → a11y/page.css; the 8a comment's "regardless of import order" is backed by the (0,3,0) specificity, independently true.
— verified for 8c/8d/8e/8f: every overridden selector (`.app-shell__rail`, `.app-shell__sidebar`, `.lumen-player__sidebar`, `.app-shell__appdot`, `.lumen-controls__play[data-glyph="pause"]`) matches a real class/attribute in AppShell.tsx (651/698/543) and ControlBar.tsx (329-331) — cross-file overrides that actually target living elements.

## 3. Findings
### F-001 [S3]
Where: ui/src/theme/lumen.css:217-220
Code:
```css
html[data-theme="light"] .lumen-slider:hover .lumen-controls__seek .lumen-slider__track,
html[data-theme="light"] .lumen-slider:hover .lumen-controls__volume-slider .lumen-slider__track {
  background: color-mix(in srgb, var(--color-surface-raised) 70%, var(--color-surface-hover));
}
```
What is wrong: The compound is inverted. The selector requires `.lumen-slider` to be an ANCESTOR of `.lumen-controls__seek` (and of `.lumen-controls__volume-slider`). The real DOM is the opposite shape: `.lumen-controls__seek` (ControlBar.tsx:368) WRAPS the `<Seekbar>` whose root IS `.lumen-slider` (Slider.tsx:86, track at :106); the volume case puts `lumen-slider` and `lumen-controls__volume-slider` on the SAME element (ControlBar.tsx:483 + Slider.tsx:86 merge), and an element is never its own ancestor. Both selectors match nothing — dead CSS. The base 8b rule (212-215, correct descendant shape) applies; the hover lift documented at 211 ("Hover keeps slider.css's lift grammar, raised-based") never does, so light-mode hover falls through to slider.css:306's dark-frame hover value.
Why it matters: A shipped light-mode interaction state (hover feedback on the two most-touched controls — seek and volume) silently doesn't exist; the comment claims it does. Exactly the class of doc-vs-reality drift this audit hunts, in the one sheet whose job is to be the truth layer for light mode.
What correct looks like: `html[data-theme="light"] .lumen-controls__seek .lumen-slider:hover .lumen-slider__track, html[data-theme="light"] .lumen-controls__volume-slider.lumen-slider:hover .lumen-slider__track { … }` (descendant-then-hover, mirroring slider.css:306's grammar).
Confidence: HIGH — Verification: read Slider.tsx:82-111 + ControlBar.tsx:368/477-484 (the DOM shape is static); selector semantics are CSS certainties. Not runtime-verified (no browser), but a non-matching selector cannot be rescued by load order or specificity.
### F-002 [S4]
Where: ui/src/theme/lumen.css:4-5
Code:
```css
 * * (design/figma-fidelity-spec.md): near-black app surface painted from
 * --color-surface (#090909 once the W25 token set lands), Inter-led
```
What is wrong: "#090909 once the W25 token set lands" — the token set HAS landed: tokens.css:54 `--color-surface: #090909`. The conditional phrasing is stale.
Why it matters: Trivial comment rot, but in the header of the sheet whose contract is precision about tokens.
What correct looks like: "--color-surface (#090909, tokens.css)".
Confidence: HIGH — Verification: tokens.css value read.
### F-003 [S4]
Where: ui/src/theme/lumen.css:67-69 (duplicate at appshell.css:33-35)
Code:
```css
  /* The base sheet owns the body — the margin reset lives here (k05 P3-4
   * ask); appshell.css carries a duplicate until its owner drops it. */
  margin: 0;
```
What is wrong: The acknowledged duplicate is real: appshell.css:33-35 `body { margin: 0; }` still exists next to this one. Two owners of one reset; harmless today (identical declarations), a drift risk tomorrow (one gets "fixed" alone).
Why it matters: Acknowledged debt with a named owner and no deadline — the standard pattern this audit keeps finding (token-lane handoffs, menu-lane fix-needed).
What correct looks like: appshell.css drops its copy (its own comment at :27-30 explains why it added one — that rationale expired when this sheet landed).
Confidence: HIGH — Verification: both sites read.

## 4. Per-function table (rule blocks)
| Rule | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `html` root type + color-scheme | 47-56 | REAL | — | Dark default; light flips scheme. |
| `body` paint + margin | 58-70 | REAL | S4 | F-003 documented duplicate. |
| `h1-h6` | 73-81 | REAL | — | Semibold, -0.01em tracking. |
| `.tabular`/`.timecode` | 89-92 | REAL | — | Consumers verified (4 files). |
| `*` scrollbar-width + `::-webkit-scrollbar*` | 100-120 | REAL | — | Firefox pair + Chromium pair; token-driven thumb. |
| `::selection` | 128-131 | REAL | — | color-mix accent tint; modern-CSS floor consistent with the Chromium/webview target. |
| `:focus-visible` floor | 141-144 | REAL | — | Deliberate duplicate of a11y/focus.css (documented, embed rationale). |
| `.lumen-separator` | 158-163 | REAL | — | Consumer: Surface.tsx:406. |
| reduced-motion | 171-180 | REAL | — | Standard 0.01ms kill; complements per-sheet transitions. |
| 8a light focus ring | 202-204 | REAL | — | Correct specificity (0,3,0); value verified in tokens.css:127. |
| 8b light track base | 212-215 | REAL | — | Correct descendant shape — matches both DOMs. |
| 8b light track hover | 217-220 | REAL | S3 | F-001 inverted compound — dead rule. |
| 8c light rail | 225-227 | REAL | — | Target verified (appshell.css:68 exists; AppShell.tsx:651). |
| 8d light sidebars | 238-241 | REAL | — | Both carriers verified; the comment's width note cross-references player.css's 283px pin accurately. |
| 8e light app dot | 246-248 | REAL | — | Target verified (AppShell.tsx:543). |
| 8f light pause pill | 257-259 | REAL | — | `[data-glyph="pause"]` matches ControlBar.tsx:331's data attribute; value verified (light --color-stroke-icon #6b6b6b). |

## 5. Test honesty
No unit tests target this sheet (standard for base CSS). Indirect pins: Surface.test.tsx:481-488 asserts the separator primitive renders; ThemeProvider/theme tests exercise data-theme wiring; the visual harness would be the only thing that could catch F-001 (a hover-state light-mode screenshot of the seek bar) — no committed light-hover seek shot exists in the baseline evidence (visual lane is not part of the committed baseline; INDEX #22-24 cover build/tests/checks only). F-001 is unpinned by any committed evidence.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "this file COMPOSES them, never overrides or redefines them" (tokens) | lumen.css:14-17 | REAL | Every color is var()/color-mix(var()); all section-8 values exist in tokens.css with the cited values. |
| "Raw values here are geometry only" | lumen.css:16-17 | REAL | 1px/8px/1.5/2px/0.01ms/-0.01em only. |
| "Hover keeps slider.css's lift grammar, raised-based" | lumen.css:211 | FALSE | F-001 — the hover rule matches nothing. |
| "#090909 once the W25 token set lands" | lumen.css:5 | STALE | F-002 — landed (tokens.css:54). |
| "appshell.css carries a duplicate until its owner drops it" | lumen.css:67-68 | REAL (still true) | appshell.css:33-35 verified present. |

## 7. Dead / duplicated / unreachable code
Dead: F-001's two hover selectors (match no DOM). Duplicated: body margin (F-003, documented); focus floor (documented, deliberate). Unreachable: none.

## 8. Dependencies & flags
Consumes the generated semantic tokens (tokens.css) — verified value-for-value for every section-8 citation. Cross-file dependencies: appshell.css (rail/sidebar/appdot targets, margin duplicate), player.css (`__sidebar` target), controls.css + slider.css (8b/8f targets and the hover grammar F-001 inverted). No flags. No stylelint disables in this file (the only ones in my assignment live in player.css, with rationale).

## 9. Open questions for the next agent
- Who owns the light-mode visual regression gap? F-001 survived because no committed screenshot exercises a light-mode hover on the seek/volume tracks — the dev-shots/visual lane's coverage list should be checked by its owner.
- appshell.css's owner should confirm the margin-reset drop is tracked; if the k05 P3-4 ask is closed, delete the duplicate.
