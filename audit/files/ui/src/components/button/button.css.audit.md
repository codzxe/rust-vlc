# AUDIT: ui/src/components/button/button.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 512 / 34 rule blocks |
| File verdict | REAL (code) with one undeliverable state claim — UNREACHABLE from any shipped binary (W14) |
| Reachable from shipped entry point? | NO as product: sole host `apps/desktop-tauri` (frontendDist `../../ui/dist`) cannot compile at HEAD (W14 SUMMARY l.19). Dev/test reach: `Button.tsx:2`, `IconButton.tsx:2`, `Toggle.tsx`/`switchCore.tsx`, `form/Switch.tsx` all `import "./button.css"`; baseline rows 22-23 (build OK, vitest 1705 pass). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg `button.css` importers (5 components + 2 form files); rg `lumen-switch`/`lumen-toggle` producers (Switch.tsx, switchCore.tsx, Toggle.tsx); rg every `var(--…)` → all 27 tokens present in `tokens/generated/tokens.css`; read `components/icon/icon.css:10-33` (glyph color mechanism); python WCAG contrast computation for the 7 ratios cited in comments; rg aria-busy setters (IconButton.tsx:41, Button.tsx:63, MiniPlayer.tsx:265, PlaylistBrowse.tsx:203, StageOverlays.tsx:122) |

## 1. One-paragraph verdict
A genuinely engineered control skin: variant/specificity interactions are consciously handled (the (0,2,0) restatement at l.115-119 proves the author understood the cascade), all tokens exist, all selectors have producers, busy/disabled/focus/reduced-motion states are complete, and the switch core genuinely unifies two class grammars. But two claims do not survive tracing: the IconButton :active accent ink can never reach a `.lumen-icon` glyph (icon.css's direct (0,2,0) rule beats the inherited `color`), and the light-mode IconButton remap is justified by a token value (`#9a9a9a`) that is NOT the light `--color-stroke-icon` (`#6b6b6b`, 4.68:1 — already WCAG-passing), so the "fix" overrides a compliant design token on a false premise. As with all ui/src: nothing shipped can render it (W14).

## 2. What actually works (verified)
— verified by rg token sweep → all 27 referenced custom properties defined in `ui/src/tokens/generated/tokens.css` (incl. --opacity-emphasis, --color-content-on-accent, --radius-control).
— verified by rg producers → `.lumen-button*` (Button.tsx), `.lumen-icon-button*` (IconButton.tsx), `.lumen-toggle*` (switchCore.tsx/Toggle.tsx), `.lumen-switch*` (form/Switch.tsx) — no orphan selectors.
— verified by python WCAG computation → dark accent-on-tinted 4.79:1 (claim "4.7:1" ✓), light accent-on-tinted 1.75:1 (claim ✓) — the w26.d2 dark-side analysis is honest.
— verified by geometry arithmetic → switch track 68 = 3px pad + 26 knob + 3 gap + 33 segment + 3 pad in BOTH on/off arrangements (l.356-456); no layout shift between states.
— verified by baseline INDEX rows 22-23 → the sheet compiles and Button/IconButton/Toggle tests run green.

## 3. Findings
### F-001 [S3]
Where: ui/src/components/button/button.css:262-264
Code:
```
.lumen-icon-button:active {
  color: var(--color-accent-icon); /* Icon.svg: active glyphs run accent */
}
```
What is wrong: The accent ink is declared on the BUTTON's `color`, but every real IconButton child is an `.lumen-icon` glyph, and icon.css:11 sets `color: var(--color-stroke-icon)` directly on the glyph, with icon.css:27-29 overriding it on host-active:
```
.lumen-icon-host:active .lumen-icon {
  color: var(--color-content-bright);
}
```
A direct declaration at (0,2,0) on the glyph element always beats an inherited `color`, regardless of the button rule's specificity. The pressed IconButton glyph therefore renders content-bright, never accent — the comment's claim ("Icon.svg: active glyphs run accent") describes behavior this cascade cannot produce. The button's own `color` only reaches non-`.lumen-icon` children (e.g. the busy spinner's currentcolor ring).
Why it matters: The documented active-state design (accent feedback on press) is silently not delivered; nobody notices because no test pins it (Button.test.tsx pins :active for `.lumen-button`, IconButton.test.tsx never asserts the icon-button active rule) and no visual test runs in CI. Dead rule + lying comment is exactly the drift class this audit exists to catch.
What correct looks like: `--color-stroke-icon: var(--color-accent-icon);` on `.lumen-icon-button:active` (the exact technique the same sheet uses at l.74-80, l.98, l.129), or an `.lumen-icon-host:active .lumen-icon` accent rule in icon.css.
Confidence: HIGH — Verification: read icon.css:10-33; cascade reasoning on direct-vs-inherited declarations.

### F-002 [S3]
Where: ui/src/components/button/button.css:284-298
Code:
```
 * the light icon token (W25 §1 light table,
 * --color-stroke-icon #9a9a9a) puts the ghost rest glyphs at 2.1–2.3:1 on
 * the light chrome/panel surfaces (#f0f0f0/#efefef) and 2.0:1 on the
 * hover chip — under WCAG 1.4.11's 3:1 for interactive iconography
 ...
:root[data-theme="light"] .lumen-icon-button {
  --color-stroke-icon: var(--color-content-secondary);
}
```
What is wrong: The justification cites light `--color-stroke-icon: #9a9a9a`. The generated token sheet says otherwise — `tokens/generated/tokens.css:129` (light block): `--color-stroke-icon: #6b6b6b;` (#9a9a9a is light `--color-stroke-border`, line 128). Computed: #6b6b6b on #f0f0f0 = 4.68:1, on #efefef = 4.63:1 — already above the 3:1 the comment says is violated. The remap therefore fires on a false premise and pushes every light-mode IconButton's rest glyph from the design's 4.68:1 token ink to content-secondary (9.98:1) — darker than any frame specifies.
Why it matters: A stale comment licenses a global override of a compliant design token; the next maintainer "fixing" contrast again has no accurate record of what the token actually is. It also proves the comment was never re-validated against the generated tokens after a tokens change (or the token was fixed and the compensating CSS was never retired).
What correct looks like: delete the remap (token passes), or if the design really wants darker icons in light mode, say so without the false WCAG emergency; regenerate the comment from `themes/tokens.json`.
Confidence: HIGH — Verification: read tokens.css light block; python contrast computation.

### F-003 [S4]
Where: ui/src/components/button/button.css:152-157
Code:
```
 * the accent label on the tinted fill is 4.7:1 in
 * dark (#d37918 on #371f06) but 1.75:1 in light (#d37918 on the flipped
 * #d8ba9a tint) — the CTA reads washed-out/disabled (vision-flagged on
 * /open-network-stream + /keyboard-shortcuts). Light keeps the tinted fill
 * and swaps the ink to content-primary (7.8:1); hover walks to bright.
```
What is wrong: The two emergency ratios are accurate (4.79, 1.75 — verified), but the fix's ratio is not: content-primary (#161616) on #d8ba9a computes to 9.83:1, not "7.8:1".
Why it matters: Numeric claims in load-bearing a11y comments must be recomputable; this one is off by 26%, showing the ratios are transcribed by hand, not derived.
What correct looks like: 9.8:1 (or a script that emits them).
Confidence: HIGH — Verification: python WCAG computation with token values.

### F-004 [S4]
Where: ui/src/components/button/button.css:257-260
Code:
```
.lumen-icon-button:hover {
  color: var(--color-content-bright);
  background: var(--color-surface-hover);
}
```
What is wrong: Same mechanism as F-001: the `color` declaration is dead for `.lumen-icon` children (icon.css:24-26 `.lumen-icon-host:hover .lumen-icon` delivers the tint directly). The hover chip (`background`) is the live half. Unlike :active, the dead value coincides with what icon.css delivers, so no visual bug — only a redundant declaration that misleads readers into thinking button.css owns hover ink.
Why it matters: Two sources both claiming to own glyph hover ink is how F-001 happened; remove the ambiguity.
What correct looks like: drop `color` from this rule (or own glyph ink exclusively via `--color-stroke-icon` remaps like Button does).
Confidence: HIGH — Verification: read icon.css:24-26.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-button` base | 36-68 | REAL | — | token-only; `--color-stroke-icon` in-subtree remap (correct technique) |
| `:hover/:active/:focus-visible` | 70-88 | REAL | — | hover/active remap stroke-icon; focus ring 2px accent |
| `--primary` + states + (0,2,0) restatement | 92-119 | REAL | — | cascade-correct; the restatement is necessary and commented |
| `--tinted` + states | 124-150 | REAL | — | accent-on-tinted; focus keeps accent glyph |
| light `--tinted` fixes | 158-170 | REAL | — | F-003 ratio citation |
| `--ghost` + states | 174-189 | REAL | — | transparent border prevents layout shift |
| `--sm/--lg` | 193-199 | REAL | — | 28/36 heights, structural |
| `:disabled/[aria-disabled]` | 203-212 | REAL | — | pointer-events:none + remap |
| `[aria-busy]` + spinner + keyframes | 215-235 | REAL | — | 14px ring, currentcolor; consumers verified (5 setters) |
| `.lumen-icon-button` base | 239-255 | REAL | — | 32px hit, transparent border |
| icon-button `:hover/:active` | 257-264 | REAL / dead-half | S3/S4 | F-001, F-004 |
| icon-button focus/disabled | 266-282 | REAL | — | remap technique correct here |
| light icon-button remap + disabled companion | 284-306 | REAL (applies) / premise FALSE | S3 | F-002 |
| icon-button busy + glyph hide + sizes | 308-344 | REAL | — | visibility (not display) avoids resize; pinned by IconButton.test.tsx:128-141 |
| `.lumen-toggle,.lumen-switch` core | 356-375 | REAL | — | one core, two grammars; geometry verified |
| `__fill/__off/__text` segments | 390-427 | REAL | — | 33×26 rx3; overflow clips locale labels |
| `__knob/__fill` + on/off state swaps + hovers | 430-468 | REAL | — | OFF hover color-mix knob; ON hover accent-bright |
| `:disabled` | 470-475 | REAL | — | opacity token |
| light ON-label fix | 482-485 | REAL | — | moved from form.css (provenance kept) |
| reduced-motion ×2 | 487-512 | REAL | — | transitions and both spinners gated |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Button.test.tsx:85-93 "loading means inert" | YES | renders + asserts aria-busy + disabled + no handler fires |
| Button.test.tsx:138/169/193 cssRule pins | YES (text-level) | string-pin the exact declarations; would NOT catch a rule that is cascade-dead (F-001 class of bug) — they assert text, not computed styles |
| IconButton.test.tsx:66-74 "loading means inert" | YES | behavioral |
| IconButton.test.tsx:128-141 spinner pin | YES (text-level) | pins busy rules incl. reduced-motion companion; same text-vs-cascade caveat |
| Toggle.test.tsx (not read in full — same sheet) | UNVERIFIED | assigned to another agent (W15-A10 has Toggle.tsx) |
Mutation check (reasoned): delete the `[aria-busy]::before` block → 2 pin tests fail. Break the switch's `[aria-checked="true"]` display swap → Toggle/Switch tests that render and assert segments fail. Nothing tests the :active accent ink — that gap is F-001's enabler.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "active glyphs run accent" (Icon.svg) | l.263 | FALSE (undeliverable) | icon.css:27-29 direct (0,2,0) rule paints content-bright; F-001 |
| "light icon token #9a9a9a … 2.1–2.3:1 … under 3:1" | l.285-288 | FALSE at HEAD | tokens.css:129 light --color-stroke-icon = #6b6b6b → 4.68:1; F-002 |
| "4.7:1 in dark / 1.75:1 in light" (tinted CTA) | l.152-155 | REAL | computed 4.79 / 1.75 |
| "content-primary (7.8:1)" | l.156 | FALSE (number) | computed 9.83:1; F-003 |
| "68x32 rx6 … 33x26 … 26x26 rx3" (Toggle.svg) | l.354-355, 398-436 | REAL | geometry arithmetic checks (68=3+26+3+33+3) |
| "Labels are localized (i18n keys switch.on/off)" | l.28 | REAL-UNPROVEN | keys asserted by switchCore consumers; i18n wiring audited by W11 (localization orphaned crate) — not re-verified here |

## 7. Dead / duplicated / unreachable code
Dead-in-effect: `.lumen-icon-button:active { color }` (F-001), `.lumen-icon-button:hover { color }` (F-004). Duplicated: hover/active glyph ink declared in both button.css and icon.css (two owners). Unreachable: entire sheet ships only via desktop-tauri (does not compile, W14).

## 8. Dependencies & flags
Consumes 27 semantic tokens (all verified present). Class contract with icon.css (`.lumen-icon-host` protocol) — the seam where F-001/F-004 live. No feature flags. stylelint suppressions: none in this file (verified by reading; the sheet is clean of disables).

## 9. Open questions for the next agent
- icon.css owner (not in my list): confirm whether `.lumen-icon-host:active .lumen-icon` should be accent (Icon.svg frames) — if yes, F-001 fixes itself there and button.css:262-264 can be deleted.
- Toggle.tsx / switchCore.tsx / Switch.tsx owners: do any settings rows disable the switch (opacity path) and does `switch.on/off` i18n exist in the en-US bundle (W11 says UI hardcodes en-US)?
