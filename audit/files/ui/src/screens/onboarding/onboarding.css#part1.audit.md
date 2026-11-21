# AUDIT: ui/src/screens/onboarding/onboarding.css#part1 (lines 1-481)
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / rule groups | 481 (chunk part1 of 925; part2 = lines 482-925 → A07) / 26 selector groups |
| File verdict | REAL |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (OnboardingScreen.tsx:64 `import "./onboarding.css"`; /onboarding is a full-bleed route, App.tsx; fresh-profile boot gate per e2e/smoke.spec.ts:8-43). SHIPPED DESKTOP BINARY: NO (tauri host unbuildable — baseline #2 exit 101; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | rg class-usage per selector (24 classes + 6 modifiers → all present in OnboardingScreen.tsx, incl. dynamic `onboarding__theme-card--${choiceId}` with choiceId ∈ light/dark/system verified at OnboardingScreen.tsx:167-172,530); token-definition check: 18/18 var(--…) used in part1 defined in tokens/generated/tokens.css or theme/lumen.css; rg prefers-reduced-motion (global guard lumen.css:171-177); rg onboarding in e2e/ (smoke.spec.ts, a11y.spec.ts, visual/manifest.ts:85,131); baseline INDEX.md (no lint:css / playwright run) |

## 1. One-paragraph verdict
Part1 (stage, window, progress dots, step anatomy, library open-form,
shortcuts tour keycaps, appearance theme cards) is a real, fully-wired
stylesheet: every selector class and modifier is referenced by
OnboardingScreen.tsx (including the dynamically composed theme-card and
keycap modifiers), every custom property resolves to a defined token, and
frame-measured raw px values carry inline `frame:` provenance comments with
one acknowledged exception family. Nothing here is dead, fabricated, or
orphaned. The findings are hygiene: a browser-default-margin subtraction
hack, two unexplained +2px drifts from the file's own stated frame
measurements, raw px values that violate the header's marking contract, and
a documented-but-fragile equal-specificity coupling to form.css. Stylelint
and the visual-fidelity suite were NOT run in baseline (UNVERIFIED).

## 2. What actually works (verified)
— verified by rg per-class usage: 24/24 base classes + modifiers
(--done/--current/--center/--text/--light/--dark/--system) consumed by
OnboardingScreen.tsx (lines 250, 530, 787-788, 811 + static hits); no dead
rules in part1.
— verified by token grep: --radius-card-lg/-window/-panel/-ring/-menu,
--stroke-ring, --motion-duration-fast, --font-size-display/-label,
--color-content-bright, --color-surface-chrome/-deep/-panel,
--space-grid/-inset, --opacity-disabled, --color-stroke-icon,
--shadow-overlay, --font-weight-regular — all defined (1-2 definitions each).
— verified by read: `@media (prefers-reduced-motion: reduce)` global guard
(lumen.css:171-177) covers this sheet's three transitions (lines 102, 430,
436) — no per-sheet guard needed.
— verified by rg: /onboarding is in the e2e smoke flow (smoke.spec.ts:42-55)
and the visual manifest (e2e/visual/manifest.ts:85,131) — the CSS has
runtime coverage paths, though neither ran in baseline.

## 3. Findings
### F-001 [S4] Stage height subtracts browser-default body margins — latent coupling to a reset that does not exist
Where: ui/src/screens/onboarding/onboarding.css:34-39
Code:
```
  /* The app still ships the browser-default 8px body margins (no global
   * reset yet) — subtract them so the stage fits the first paint exactly
   * at 1440×900 and 1024×768 with nothing clipped (same note as the
   * previous sheet; longer step content scrolls honestly). */
  min-height: calc(100vh - 16px);
  min-height: calc(100dvh - 16px);
```
What is wrong: The magic 16px is the UA default `body { margin: 8px }` baked
into a screen-level calc. The comment honestly flags it, but the day a global
reset lands (every design system eventually adds one), every onboarding
stage becomes 16px too short and this rule silently mis-fits.
Why it matters: Time-bomb constant with no test pinning the body margin
assumption.
What correct looks like: `body { margin: 0 }` in lumen.css reset section +
plain `min-height: 100dvh` here.
Confidence: HIGH — Verification: read (no `body {` reset found in
theme/lumen.css reset section — checked).
### F-002 [S4] Frame-fidelity drift: measured 590px/1090px pinned as 592px/1092px
Where: ui/src/screens/onboarding/onboarding.css:63-64,171-175
Code:
```
  /* frame: Onboarding.svg window 1090 wide on its 1440 stage */
  width: min(1092px, 100%);
```
and
```
  /* Fixed measure (frame: Onboarding.svg body column 590px) so the
```
(then line 175: `width: 592px;`)
What is wrong: Both structural widths deviate +2px from the file's own stated
frame measurements with no comment explaining the rounding.
Why it matters: This sheet's entire authority is frame measurement
provenance; unexplained deviations erode the convention the rest of the file
religiously follows (every other value matches its comment).
What correct looks like: Either 590/1090 with a note, or a comment stating
the +2px rationale (e.g. even-pixel snapping).
Confidence: HIGH — Verification: read, comments and values compared.
### F-003 [S4] Raw px values without the header's `frame:` marking contract
Where: ui/src/screens/onboarding/onboarding.css:97-99,109-110,281,331
Code:
```
.onboarding__progress-dot {
  width: 6px;
  height: 6px;
```
and line 110 `width: 18px;`, line 281 `gap: 8px 32px;`, line 331 `gap: 16px;`
What is wrong: The header contract (lines 21-24) permits raw px "only for
frame-measured structural values, each marked `frame:`". These structural
values (dot geometry, hint gaps, row gap) are unmarked; the step gap at 123
(`gap: 48px`) even admits in-comment it is a rhythm compromise, not a frame
value. None map to the existing --space-* tokens.
Why it matters: Self-declared convention violated in its own first 300 lines;
future token sweeps will not know which raw values are load-bearing.
What correct looks like: `frame:` markers where measured; --space-* tokens
where the value is system rhythm.
Confidence: HIGH — Verification: read.
### F-004 [S4] Equal-specificity restatement of form.css state rules — cross-file cascade fragility
Where: ui/src/screens/onboarding/onboarding.css:230-244
Code:
```
/* An inset field stays inset — hover/active lift the STROKE, never the fill
 * (the raised-field fills belong to form.css's anatomy; the rest shorthand
 * above sits at equal specificity with form.css's state rules and would
 * otherwise swallow them — §7's hover/focus floor needs the lift restated). */
.onboarding__open-form .lumen-field__control:hover,
.onboarding__open-form .lumen-field__control:active {
```
What is wrong: The 224-228 shorthand overrides form.css state rules by source
order, so this sheet must restate hover/focus to restore them — a coupling
that breaks silently if form.css reorders or renames state selectors. The
comment documents the hazard instead of removing it.
Why it matters: Any form.css state-rule change (e.g. adding :focus-visible
border grammar) can silently regress the onboarding field.
What correct looks like: A scoped modifier class on the field (e.g.
`.lumen-field--inset`) owned by form.css, or `:where()` to drop the
shorthand's specificity so form.css wins naturally.
Confidence: HIGH — Verification: read; form.css state rules presence
confirmed by the comment's own claim (cross-file order not executed —
stylelint/playwright not run in baseline, UNVERIFIED).

## 4. Per-rule-group table (part1)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .onboarding (stage) | 31-50 | REAL | S4 (F-001) | dvh fallback present; token-clean |
| .onboarding__window | 54-83 | REAL | S4 (F-002) | +2px drift; margin:auto scroll-not-clip rationale documented |
| .onboarding__progress / -dot / --done / --current | 87-112 | REAL | S4 (F-003) | unmarked 6/18px; transition covered by global reduced-motion |
| .onboarding__step / --center | 116-144 | REAL | — | min-height floor 296px unmarked raw (F-003 family) |
| .onboarding__title (+center variant) | 153-166 | REAL | — | raw 24px but frame-marked; token gap (18↔32) honestly noted |
| .onboarding__body (+center) | 168-191 | REAL | S4 (F-002) | 592 vs measured 590; justify only on center step |
| .onboarding__cone | 194-198 | REAL | — | 107px frame-marked |
| .onboarding__open-form family | 206-262 | REAL | S4 (F-004) | inset-field grammar; light-mode remap 251-255 present |
| .onboarding__open-action / -notice / -hints / -hint | 264-291 | REAL | — | 114×35 frame-marked |
| .onboarding__shortcut-grid (+1180px fallback) | 301-315 | REAL | — | range syntax `width <= 1180px`; measured 357/392/186 frame-marked |
| .onboarding__shortcut-column / -row | 317-332 | REAL | — | 40px pitch frame-derived |
| .onboarding__shortcut-label / -keys / -pair / -sep | 334-357 | REAL | — | consumed (rg 1-2 hits each) |
| .onboarding__key / --text | 368-394 | REAL | — | two-layer inset shadow geometry documented; 35/42/55px frame-marked |
| .onboarding__theme-cards / -card (+disabled) | 398-422 | REAL | — | disabled path used when theme store unavailable (OnboardingScreen.tsx:533) |
| .onboarding__theme-swatch / -label | 424-437 | REAL | — | transitions reduced-motion covered |
| theme depiction hex (+stylelint disable) | 439-454 | REAL | — | stylelint-disable scoped and re-enabled; frame-measured art per header exemption |
| selected-ring / hover / pressed | 456-480 | REAL | — | aria-pressed ring, hover floor, active dip — all consumed |

## 5. Test honesty
No unit tests target this stylesheet directly; OnboardingScreen.test.tsx
(component) and e2e/smoke.spec.ts + visual manifest cover the rendered tree.
Baseline executed vitest only (ui-vitest.log.txt — component tests pass);
playwright e2e/visual and `npm run lint:css` were NOT run in baseline —
stylelint cleanliness and visual fidelity are UNVERIFIED, flagged here.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Colors/fonts/radii strictly via semantic tokens; raw px only for frame-measured values, each marked `frame:`" | header lines 21-24 | SHALLOW | F-003: multiple unmarked raw px; F-002: +2px drifts |
| "no global reset yet — subtract 8px body margins" | lines 34-37 | REAL | read; no body reset in lumen.css |
| frame provenance per rule (Onboarding*.svg measurements) | throughout | REAL-UNPROVEN | measurements not re-derivable in sandbox (no frame render); internal consistency checked only |

## 7. Dead / duplicated / unreachable code
None in part1 — every selector group has a consumer (per-class rg proof,
§2). No duplicate declarations found within part1.

## 8. Dependencies & flags
Depends on generated token sheet (tokens/generated/tokens.css) and
form.css class anatomy (.lumen-field__control) — the latter is the F-004
coupling. No feature flags.

## 9. Open questions for the next agent (part2, A07)
- Does part2 (footer, accent dots) keep the negative-margin pattern
  (`margin-inline: -12px -2px` starts at 598 in part2's territory)? If both
  halves use magic negatives, the reset time-bomb (F-001) doubles.
- The visual manifest claims onboarding coverage — REQUEST-ORCH-RUN
  candidate: `npx playwright test e2e/visual/visual.spec.ts -g onboarding`
  to convert frame-fidelity claims from UNVERIFIED to executed.
