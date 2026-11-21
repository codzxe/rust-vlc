# AUDIT: ui/src/screens/onboarding/onboarding.css#part2 (lines 464–925 of 925)
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 462 (chunk part2) / 41 selector blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — /onboarding is the first-run gate route (App.tsx:296, RequireOnboarding wraps every shell route); CSS imported by OnboardingScreen.tsx:64. W14 caveat: no shipped desktop binary (tauri host fails to compile, build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg -o 'onboarding__[a-z-]+' OnboardingScreen.tsx vs sed -n '464,925p' onboarding.css — comm: ZERO part2 classes unrendered; rg -- '--stroke-ring:' tokens.css (exists); rg -n 'transition' onboarding.css (all four reduced-motion targets have real transitions: progress-dot:102, theme-swatch:430, theme-label:436, accent:665); sed quotes verified 588-600, 613-616. |

## 1. One-paragraph verdict
Chunk boundary: part1 (lines 1–463) belongs to A10; this audit covers 464–925 (theme-card states, playback/subtitle settings rows, privacy rows, footer, accent-dot radiogroup, dark ink pins, light AA block, sr-only utility, and three media-query blocks). Every one of the 41 selector blocks' classes is rendered by OnboardingScreen.tsx (comm cross-check: zero orphans in this range), the reduced-motion guard disables four real transitions, and the 9 accent hexes are byte-identical to form.css's swatch table and to the generated per-accent tokens. Debt is documentation-heavy but real: one raw typography size, five raw dark-mode ink/fill hexes, and a duplicated 9-entry accent table.

## 2. What actually works (verified)
— class↔TSX comm check (48 rendered onboarding classes, part2 subset fully consumed); token existence (--stroke-ring, all others verified in the form.css pass); transition↔reduced-motion cross-check; baseline `npm test` 1705 passed incl. OnboardingScreen suites (ui-vitest.log.txt), `npm run build` OK (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/onboarding/onboarding.css:613-615
Code:
```
  /* frame: Onboarding-2.svg note ≈13px (g01-P3-4 / h02 #3 — the caption
   * token's 11px was two sizes under the frame). */
  font-size: 13px; /* frame: ≈13px */
```
What is wrong: a raw typography size. The file's own header (part1, lines 26-28) restricts raw px to "frame-measured structural dimensions" — this is a font size, i.e. the type ramp, not geometry. Documented (the caption token is 11px, frame wants 13px), so the deviation is honest — but it means the token ramp is two sizes short of the frames and every consumer must choose between lying tokens and raw px.
Why it matters: a second raw size anywhere breaks the ramp's consistency story; the token lane owes a 13px step.
What correct looks like: add the missing type step to the generated token set and consume it.
Confidence: HIGH — read.

### F-002 [S4]
Where: ui/src/screens/onboarding/onboarding.css:758-784
Code:
```
:root:not([data-theme="light"]) .onboarding__step:not(.onboarding__step--center) .onboarding__title {
  color: #969696; /* frame: Onboarding-1/-2.svg working-step headings */
}
```
```
:root:not([data-theme="light"]) .onboarding__back,
:root:not([data-theme="light"]) .onboarding__open-action {
  background: #1b1b1b; /* frame: Onboarding-1.svg Back / Onboarding.svg Browse fill */
```
What is wrong: five raw hex pins for INK and FILL (not structural geometry) in dark mode: #969696 (title, body, shortcut labels/seps), #1b1b1b (keycap fill, back/open-action fill+border), #373737 (keycap inset stroke). Each carries a frame citation and stylelint-disable, and the light block (798-809) remaps to tokens — but the dark theme (the DEFAULT theme) reads text color from frame literals, outside the token cascade.
Why it matters: theme-wide ink changes (e.g. dark secondary content token re-derivation) will never reach these five surfaces; also `:not([data-theme="light"])` means any future third theme silently inherits the dark pins.
What correct looks like: dark tokens at (or a documented deviation from) the frame values, consumed like light does.
Confidence: HIGH — read.

### F-003 [S4]
Where: ui/src/screens/onboarding/onboarding.css:709-744 vs ui/src/components/form/form.css:502-536
Code:
```
/* stylelint-disable color-no-hex -- frame: Onboarding-1.svg (9 accent dot primaries, frame-measured) */
.onboarding__accent[data-accent="orange"] {
  background: #d37918;
}
```
What is wrong: the 9-entry accent-primary table exists twice in the codebase (identical hexes: here and form.css's SwatchGroup), both times justified by the same "generated per-accent blocks are :root[data-accent]-scoped and cannot be consumed here" rationale.
Why it matters: two hand-maintained copies of a color table that must track the generated tokens — a future accent change updates tokens.css and at most one copy; the other silently lies.
What correct looks like: a generated static class list (e.g. tokens gen emits `.lumen-accent-primary[data-accent=…]` background utilities) consumed by both.
Confidence: HIGH — read both files; hexes verified identical.

## 4. Per-function table (selector blocks in 464–925)
| Selector block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.onboarding__theme-card:hover` (label/swatch) + pressed | 466-480 | REAL | — | :not(:disabled) guards correct |
| `.onboarding__settings` | 484-491 | REAL | — | |
| `.onboarding__setting-row` (+last-child) | 493-504 | REAL | — | |
| `.onboarding__setting-label` | 506-509 | REAL | — | |
| `.onboarding__setting-field.lumen-field` (+label/number) | 512-528 | REAL | — | composes form.css grammar |
| `.onboarding__setting-segmented .lumen-segmented__segment` | 532-534 | REAL | — | 88px override of 104px min |
| `.onboarding__privacy-list` | 538-545 | REAL | — | |
| `.onboarding__privacy-row` (+last-child) | 547-558 | REAL | — | |
| `.onboarding__privacy-text` / `__label` / `__description` | 560-576 | REAL | — | |
| `.onboarding__privacy-note` | 578-584 | REAL | — | |
| `.onboarding__footer` | 594-601 | REAL | — | negative margins frame-pinned |
| `.onboarding__footer-aside` / `__footer-note` | 603-621 | REAL | S4 (F-001) | raw 13px |
| `.onboarding__footer-actions` | 623-628 | REAL | — | |
| `.onboarding__back, .footer-actions .lumen-button--primary` | 631-641 | REAL | S4 (F-002 dark pin) | panel fill rest-state |
| `.onboarding__hidden` | 643-645 | REAL | — | visibility for geometry stability |
| `.onboarding__accents` | 650-655 | REAL | — | 80px pitch |
| `.onboarding__accent` (+unchecked/disabled/hover/checked-ring/pressed) | 657-703 | REAL | — | aria-checked selectors (w26.gfix-D fixed dead aria-pressed — documented) |
| 9 × `[data-accent]` dot fills | 708-744 | REAL | S4 (F-003) | hexes match tokens exactly |
| dark ink pins (title/body/shortcut/key/back/open-action) | 753-785 | REAL | S4 (F-002) | stylelint-disabled, cited |
| light AA ink block | 796-810 | REAL | — | 9 selectors → content-primary ≈8:1, documented |
| `.onboarding .visually-hidden` | 815-825 | REAL | — | screen-local sr-only |
| `@media (height <= 800px)` | 829-880 | REAL | — | range syntax needs modern browsers — fine for webview target |
| `@media (width <= 760px)` | 884-914 | REAL | — | |
| `@media (prefers-reduced-motion)` | 918-925 | REAL | — | all 4 targets verified transitioning |

## 5. Test honesty
No tests in this chunk. OnboardingScreen tests belong to the OnboardingScreen.tsx owner (not in my list).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "identical set to the generated per-accent blocks" | onboarding.css:705-707 | REAL | hex-by-hex match with tokens.css (verified in form.css audit) |
| "the radiogroup marks selection with aria-checked, w26.gfix-D fixes the dead aria-pressed selector" | onboarding.css:688-690 | REAL | selectors at 672/691 use aria-checked |
| "raw px just for frame-measured structural dimensions" (part1 header, governs this file) | onboarding.css:26-28 (part1) | FALSE (3 exceptions in part2) | F-001 (13px font), F-002 (5 raw hex inks/fills) |
| "a token, never a hardcode (contract §4.4)" (light block) | onboarding.css:793 | REAL | 798-809 uses only var(--color-content-primary) |

## 7. Dead / duplicated / unreachable code
No dead selectors in this chunk (comm check zero orphans). Duplication: F-003 (accent table ×2).

## 8. Dependencies & flags
Depends on form.css grammar (`.lumen-field`, `.lumen-number`, `.lumen-segmented__segment`, `.lumen-button--primary`) and tokens.css (all consumed vars exist; --stroke-ring verified).

## 9. Open questions for the next agent
- Part1 (A10): lines 38-39 `min-height: calc(100vh - 16px)` compensates a body margin that lumen.css:69 reset globally — 16px-short wizard window? (Also flagged in my appshell.css audit.)
- Do the playback/subtitle "real settings rows" (482-534) actually persist anything, or does the wizard's settings state evaporate on finish (W11 settings-never-persist)? The CSS cannot answer; the OnboardingScreen.tsx owner must.
