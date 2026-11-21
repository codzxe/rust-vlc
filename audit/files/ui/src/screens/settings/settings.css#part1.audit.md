# AUDIT: ui/src/screens/settings/settings.css#part1 (lines 1-415)
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 415 (of 859; chunk boundary at the `rows` banner: part1 = 1-415 [stage/window/top bar/body/sidebar/content], part2 = 416-859 [rows, action buttons, about/staged, advanced disclosure, floating bars, dirty dialog, responsive] — A06) / 22 selector groups |
| File verdict | REAL (token-disciplined layout sheet; all selectors consumed; no dead rules found in range) |
| Reachable from shipped entry point? | NO (ui-wide fact: no GUI binary at HEAD). SPA-internal: imported by SettingsScreen.tsx (`import "./settings.css"`), routed /settings/:area App.tsx:322. Style application verified indirectly by SettingsScreen tests asserting geometry contracts (baseline run #23). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Read 1-500 (chunk + boundary context); `rg -n '^/\* -{20,}'` (section banners → boundary choice); per-selector consumer check via `rg -l` (all 22 part1 selectors have ≥1 TSX consumer — settings-screen:1, settings-window:3, settings-section:24, settings-section__title:18 …); raw-color scan `rg '#[0-9A-Fa-f]{3,8}'` on 1-415 → hits ONLY inside comments (zero raw colors in declarations); `rg '"scripts"' -A20 package.json` (lint:css = scripts/lint-css.mjs → stylelint 18 local — NOT in baseline runs); read scripts/lint-css.mjs head (offline launcher contract). |

## 1. One-paragraph verdict
The cleanest file audited so far: a frame-measured, token-disciplined stylesheet where every raw pixel carries a `frame:` provenance comment, zero raw hex colors appear in declarations, every selector in the range has a live TSX consumer, and the tricky bits (light-theme placeholder contrast, focus-visible rings, script-focus ring suppression, specificity ordering) are each solved and explained in place. The only defects are maintenance-shaped: three live-measured per-area section-gap constants (26/23/9px) coupled to row inventories with an in-code admission that the global area still misses its frame by 33.5px, a stage-padding comment whose arithmetic cannot be reconstructed, and the fact that stylelint (`npm run lint:css`) has no committed baseline run despite the file's comments leaning on stylelint rules.

## 2. What actually works (verified)
— Selector liveness: all 22 part1 selector groups have consumers (rg -l counts above; `settings-nav__item--active`, `data-mount-focus`, `settings-screen__results/empty` all in SettingsScreen.tsx).
— Token discipline verified: `sed -n '1,415p' | rg '#hex'` → every hit is inside a comment; declarations use var(--…) / color-mix over tokens exclusively, matching the header's contract (:17-18).
— Light-theme a11y fix verified present: `:root[data-theme="light"] .settings-topbar__search input::placeholder` (:150-152) — the documented invisible-placeholder hazard addressed.
— Contract markers the header promises (data-settings-area, settings-search-*) exist in TSX (SettingsScreen.tsx:385/389, SettingsSections.tsx).
— Baseline run #23 (vitest) covers SettingsScreen geometry contract tests per the header's claim (:12-14).

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/settings/settings.css:385-395
Code:
```
.settings-sections[data-settings-area="subtitles"] .settings-section {
  margin-bottom: 26px; /* live: 2×2px reclaimed across the 2 boundaries */
}

.settings-sections[data-settings-area="interface"] .settings-section {
  margin-bottom: 23px;
}

.settings-sections[data-settings-area="global"] .settings-section {
  margin-bottom: 9px; /* live: sections end y587.5, bar y611.5 (see above) */
}
```
What is wrong: Three area-scoped magic constants hand-calibrated against in-browser measurements, explicitly coupled to "the area's CURRENT row inventory — re-measure when an area's rows change" (:369-370). The global area's calibration comment further admits a 33.5px unreconciled residual vs its frame ("the bar sits at window-local y611.5 … The frame draws the bar at y645") deferred to the Lead.
Why it matters: Highest-maintenance rules in the sheet: any row added/removed in subtitles/interface/global silently breaks the fold calibration the constants exist to preserve; the global area knowingly ships 33.5px off its frame. Disclosed debt, limited blast radius (spacing only).
What correct looks like: Row-pitch derivation from the frame metrics (the comment itself says "a row-pitch pass … not a gap tweak") or a single data-driven spacing table per area.
Confidence: HIGH — Verification: read (all three blocks + their :362-384 rationale comment).

### F-002 [S4]
Where: ui/src/screens/settings/settings.css:26-30
Code:
```
 * w26.gfix-B (g13-P1-1 + the verifiers' 837-vs-865 note): stage padding
 * 16→2px — at 1440×900 the window now measures the frames' exact
 * 1048×865 (previously 837: the 14px lost to padding pushed 3 subtitles
 * rows below the fold). */
```
What is wrong: The arithmetic does not reconstruct. With 16px stage padding at a 900px stage, the window's `height: min(865px, 100%)` still resolves to 865 (900−32=868 ≥ 865), so "previously 837" cannot follow from the padding alone; "the 14px lost to padding" matches neither 16 nor 2 nor the 28px (865−837) delta. Some other sizing rule must have been in play and is not named.
Why it matters: Provenance comments that fail arithmetic cannot be trusted for the next calibration pass — exactly the kind of number this file's discipline depends on.
What correct looks like: A comment that states the prior rule (e.g. `height: min(865px, calc(100% - 28px))`) or drops the unreconstructable history.
Confidence: MEDIUM — Verification: read + arithmetic; prior CSS revision not inspected (git history out of scope this session).

### F-003 [S4]
Where: ui/src/screens/settings/settings.css:289-292, 397-401 (and stylelint reliance generally)
Code:
```
/* Active row — frame: #222222 raised fill, ORANGE icon at stroke 2
 * (Icon variant="strong"), #EAEAEA label. Active rules come BEFORE the
 * hover rules so the orange icon/bright label stay specific enough on an
 * active row (stylelint no-descending-specificity). */
```
What is wrong: The file's rule ordering is engineered around stylelint's `no-descending-specificity`, but no baseline run executes `npm run lint:css` (baseline INDEX runs #21-24: npm ci/build/test/4 check-scripts — lint:css absent; scripts/lint-css.mjs exists precisely because the sandbox had no stylelint for a while).
Why it matters: The lint contract the comments rely on is UNVERIFIED in the committed evidence; a regression in this sheet's ordering discipline would surface only on a machine that runs the un-run script.
What correct looks like: `npm run lint:css` added to the baseline/gate matrix.
Confidence: HIGH — Verification: read of baseline INDEX.md command table + package.json scripts.
(REQUEST-ORCH-RUN candidate — recorded in log, not essential.)

## 4. Per-function table (CSS: selector rows)
| Selector | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.settings-screen` | 31-44 | REAL | — | Flex center stage; 2px padding (F-002 comment issue); token colors/fonts. |
| `.settings-vh` | 48-58 | REAL | — | Visually-hidden (clip-path inset 50%) — correct modern pattern; consumed by areas/* (staged.tsx, extensions.tsx, subtitles.tsx). |
| `.settings-window` | 62-80 | REAL | — | 1048×865 min() sizing; token color-mix 95%; blur; shadow via color-mix; radius token. |
| `.settings-topbar` | 86-91 | REAL | — | 50px strip via grid row (window :64). |
| `.settings-topbar__search` (+focus-within, icon, input, ::placeholder, light override) | 95-152 | REAL | — | Pill geometry frame-annotated; input focus ring for e2e computed-outline audit; light placeholder contrast fix. |
| `.settings-topbar__title` | 157-170 | REAL | — | x240 anchor; 15/85 color mix ≈ #979797 (token-lane gap disclosed). |
| `.settings-topbar__close` (+hover/focus) | 173-201 | REAL | — | 24px hit centered on 25px frame inset (13+12=25 ✓); hover/focus states. |
| `.settings-window__body` | 207-211 | REAL | — | 216px sidebar grid column. |
| `.settings-sidebar` / `__panel` | 217-231 | REAL | — | 8px inset; panel r6 token; 95% surface-panel mix. |
| `.settings-sidebar__group-label` / `__group` (+ul/li) | 233-252 | REAL | — | 48px pitch via 40px rows + 8px margins. |
| `.settings-nav__item` (+icon/label/active/hover/focus/data-mount-focus) | 256-319 | REAL | — | r2 rows; active before hover (stylelint note F-003); mount-focus ring suppression (reasoned, cleared on first keydown by SettingsScreen). |
| `.settings-content` | 329-336 | REAL | — | 33px top padding frame-annotated; no bottom padding (frame-faithful, reasoned). |
| `.settings-screen__results` / `__empty` | 338-347 | REAL | — | Search-results containers (SettingsScreen.tsx:385-389). |
| `.settings-sections` / `.settings-section` (+area overrides + :last-child) | 349-404 | REAL | F-001 | Calibrated magic constants. |
| `.settings-section__title` | 406-414 | REAL | — | 7px margin frame-annotated. |

## 5. Test honesty
No tests in this file. Contract tests referenced by the header live in SettingsScreen.test.tsx / areas tests (other agents' files) and passed in baseline run #23. The e2e tab audit asserting the input's computed outline (comment :132-135) is in ui/e2e — not run in baseline (UNVERIFIED).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only; raw px is structural … marked with `frame:` comments" | settings.css:17-18 | REAL | Raw-color scan: zero hex in declarations (1-415); px values annotated. |
| "Contracts kept: data-setting, data-dirty, data-settings-area, testids …" | settings.css:12-14 | REAL (part1 subset) | data-settings-area + settings-search-* verified in TSX; applybar/restart-notice testids live in part2's selectors (A06 to verify). |
| "1048×865 r14, #0E0E0E@95%" frame fidelity | settings.css:4-5 | REAL-UNPROVEN | Values coded (:69-79); frame Settings - global.svg exists in design/figma (not pixel-diffed here — visual harness is `npm run visual`, not in baseline). |
| global-area bar at frame y645 | settings.css:379-383 | FALSE (self-admitted) | In-code: bar actually at y611.5, residual 33.5px "flagged for the Lead". |

## 7. Dead / duplicated / unreachable code
None in part1: all 22 selector groups consumed (verified). The k12 comment (:494-498) documents dead rules DELETED in a prior pass — verified absent.

## 8. Dependencies & flags
Tokens from ui/src/tokens/generated/tokens.css (my file 9). No feature flags. stylelint 18 + config-standard 41 via devDependencies (lint:css launcher scripts/lint-css.mjs — offline contract).

## 9. Open questions for the next agent
- A06 (part2): do the floating-bars/dirty-dialog/responsive selectors keep the same liveness? (`rg -l` each.)
- Visual harness (`npm run visual`, scripts/visual-run.mjs — my file 19) never ran in baseline: every `frame:` fidelity claim in this sheet is UNVERIFIED by committed evidence. REQUEST-ORCH-RUN: `cd ui && npm run lint:css` (cheap, proves the specificity-ordering contract).
