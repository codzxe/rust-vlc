# AUDIT: ui/src/screens/settings/settings.css#part2
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 444 (416–859 of 859) / 50 rule blocks |
| Chunk boundary | part2 = lines 416–859: sections `rows` (416), `action buttons` (520), `about / staged notes` (578), `advanced disclosure` (718), `floating bars` (762), `dirty dialog` (804), `responsive` (830). A08's part1 = 1–415 (stage, window, top bar, body, sidebar, content head). Equal split would be 430 — moved 14 lines so the `rows` section stays whole; flagged in log for A08. |
| File verdict | REAL-UNPROVEN (live grammar on all 13 settings-area routes with committed baselines; one 30-line dead rule family; no committed visual diff run) |
| Reachable from shipped entry point? | YES as web-UI source: `settings.css` imported by SettingsScreen.tsx; App.tsx routes all 13 settings areas (e2e/visual/manifest.ts "25-route truth"), and 13 `settings-*.png` baselines exist. Same W15 caveat: no GUI binary (W14; baseline #2 exit 101), no `<canvas>` (W0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "^/\* ---" settings.css` (section map → boundary); per-class producer counts over src/screens/settings (`settings-about__rows/__row → 0 files` → F-001); `rg -rn "about__row\|<dl" src scripts` (dl exists only in media-info/state overlays under other names); python3 WCAG recompute (claimed 1.75:1 exact; claimed 7.8:1 is actually 9.83:1 → F-003); `rg -n "lumen-segmented\|lumen-swatchgroup" components/form/form.css` (:378 — the width-fill targets exist); `rg -rn "settings-field__hint\b\|settings-field__head\b\|settings-screen__reset\b"` (k12's claimed deletions are real — only comment mentions remain); baseline #22/#23 |

## 1. One-paragraph verdict
The back half of the settings sheet — row grammar, action buttons, badges, dirty-edge, advanced disclosure, applybar, confirm dialog, responsive — is disciplined token work with unusually good in-file hygiene: three dead rules were deleted WITH grep-provenance comments (w26.k12, :494-498, :516-518, :683-685, :849-851), and I verified all three deletions are genuinely dead in TSX. Two real defects: a 30-line `.settings-about__rows/__row/dt/dd` family (687-716) that NO file produces while its comment claims a `<dl>` restructure in updates-about.tsx that file does not contain — dead rules plus false provenance in one block; and the "staged-inert" pattern that deliberately renders unwired buttons at full strength, which in a build where W11 proved settings never persist makes every staged control visually indistinguishable from a working one. As with the wave's other CSS: 13 settings-area baselines exist but no committed run ever diffed them.

## 2. What actually works (verified)
— verified by producer-count grep → 18 of 20 class families in this chunk have TSX producers (e.g. `settings-field__row` ×10 files, `settings-action` ×7, `settings-applybar` ×3); the 2 exceptions are the dead about-rows family (F-001).
— verified by `rg` for the three k12-deleted selectors → `settings-field__hint`, `settings-field__head`, `settings-screen__reset` appear in ZERO non-CSS files — the sheet's dead-code-deletion comments are truthful (rare; most sheets only claim).
— verified by python3 WCAG recompute → `.settings-badge--restart` light-mode problem is real (accent on light tint = 1.75:1, exactly as claimed) and the fix passes (#161616 on #d8ba9a = 9.83:1); dark badge accent-on-tint = 4.79:1 (AA pass).
— verified by baseline #22/#23 → sheet compiles; SettingsScreen + 13 area test files pass in the 1705-run.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/settings/settings.css:687-716
Code:
```
/* About panel rows (updates-about) — term/value pairs. The <dl> wrapper
 * (w26.d2 a11y restructure, updates-about.tsx) needs only the UA reset. */
.settings-about__rows {
  margin: 0;
}
```
What is wrong: the entire `.settings-about__rows` / `.settings-about__row` (+`__row + __row` border, `dt`, `dd`) rule family — 30 lines — has ZERO producers: `rg "about__row" src scripts` matches only this sheet; updates-about.tsx renders no `<dl>` and no `settings-about__row` (it uses `settings-field__row`/`__lead`/`__label`/`settings-about__value` divs, updates-about.tsx:37-53). The comment's claimed provenance ("<dl> wrapper … updates-about.tsx") is false at HEAD — the only `<dl>` elements in the app are media-info's `lumen-mi__rows` and the state overlay's facts list.
Why it matters: dead selectors with a false citation poison exactly the provenance system this sheet otherwise uses correctly (the k12 deletions); the next refactor will either preserve dead CSS "because updates-about uses it" or delete live-looking history.
What correct looks like: delete 687-716, or restore the `<dl>` grammar in updates-about.tsx if the d2 a11y restructure was supposed to land and regressed.
Confidence: HIGH — Verification: read + two independent greps (settings dir, whole src+scripts).

### F-002 [S4]
Where: ui/src/screens/settings/settings.css:564-567 and 600-604
Code:
```
.settings-action:disabled {
  opacity: 1; /* staged-inert: frame-full ink, never dimmed (Remotes pattern) */
  cursor: not-allowed; /* honest refusal (gfix-E disabled-thumb precedent) */
```
What is wrong: staged-inert renders unwired controls at full visual strength — indistinguishable from working buttons except by cursor and the (unrendered-until-hovered) title/aria-describedby reason. The sheet itself concedes the risk ("disabled stays un-focusable — pair the reason with a visible/row-level note where discovery matters", :562-563) — whether every staged row carries that visible note is an areas-lane question, and W11's finding that settings never persist widens the blast radius: staged rows are not the exception but the default outcome of Apply.
Why it matters: the visual language for "this does nothing" is deliberately erased at the exact layer (CSS) where users read affordance; keyboard users get nothing at all (unfocusable).
What correct looks like: a visible in-row staged marker (the `settings-area-staged-note` pattern at :590-594 exists — make it per-row mandatory), or a restrained dim (e.g. 0.85) the frames would still read as live.
Confidence: HIGH — Verification: read; W11 non-persistence cited from wave evidence (settings persistence layer absent).

### F-003 [S4]
Where: ui/src/screens/settings/settings.css:627-628
Code:
```
/* w26.d2 light sweep — accent ink on the flipped #d8ba9a tint is 1.75:1;
 * light keeps the tinted chip and swaps the ink to content-primary (7.8:1). */
```
What is wrong: the second number is wrong — content-primary #161616 on #d8ba9a measures 9.83:1, not 7.8:1 (first number 1.75:1 verified exact). The drift direction is safe (understates a passing value) but the figure is load-bearing in an a11y justification.
Why it matters: a11y math that cannot be reproduced on demand trains maintainers to trust comments over computation; the correct value makes the fix stronger, not weaker — the comment sells its own fix short.
What correct looks like: recompute-and-date the comment (9.8:1), or cite the tokens pair rather than a precomputed ratio.
Confidence: HIGH — Verification: python3 luminance recompute (both values).

### F-004 [S4]
Where: ui/src/screens/settings/settings.css:832
Code:
```
@media (width <= 64rem) {
```
What is wrong: Media Query Range Syntax — supported Chrome 104+/Safari 16.4+/Firefox 102+, but NOT by older WebKitGTK (the Tauri Linux webview on distros with pre-2.40 webkit2gtk). On such hosts the entire narrow-stage block (stage padding, content padding, topbar title reposition) silently no-ops — no error, just unadapted layout at ≤1024px. Correction during audit: this is NOT an isolated gamble — `rg "@media" src --include=*.css` shows the range syntax is the codebase convention (25 prefers-reduced-motion + 4 further `width <=`/`height <=` queries: 760px, 1120px, 1180px, 800px; zero legacy `max-width` queries). The finding therefore applies repo-wide: the UI's responsive layer as a whole has an untested WebKitGTK floor, and no committed run exercises it (F-001 of list.css — no visual run at all).
Why it matters: the shipped Linux desktop target is exactly the engine family where this syntax is newest; the failure is silent and systemic.
What correct looks like: either a documented minimum-webview policy (tauri.conf / README) with a CI lane on the floor version, or `@media (max-width: 64rem)` — identical semantics, universally parsed.
Confidence: HIGH — Verification: read + repo-wide media-query census (see Commands); WebKitGTK version floor UNVERIFIED (no build host — sandbox cannot compile tauri, baseline #2).

## 4. Per-function table (selector groups, part2)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.settings-field__row` | 422-429 | REAL | — | 320px control column, `min(320px,40vw)` narrow-stage yield |
| `[data-kind=switch/action/slider] __row` | 432-436 | REAL | — | 42px pitch variant; kinds produced by SettingField.tsx |
| `[data-kind=values] __row` | 440-442 | REAL | — | 16px gap variant |
| `__lead` / `__label` | 446-458 | REAL | — | wrap-safe, `overflow-wrap: anywhere` |
| `__control` / `__special` | 460-477 | REAL | — | right-anchored column; special stack |
| `__control .lumen-field/.lumen-radiogroup/__special` + `.lumen-segmented/.lumen-swatchgroup` + `.lumen-switch` | 479-492 | REAL | — | width-fill seams; form.css targets verified (:378) |
| (deleted `.settings-field__bare` — k12 note) | 494-498 | N/A | — | deletion verified true |
| `__control .lumen-radiogroup__options` (+special grid) | 501-514 | REAL | — | auto-fill 9rem grid for pickers |
| (deleted `__hint` — k12 note) | 516-518 | N/A | — | deletion verified true |
| `.settings-action` (+--wide/focus/disabled/hover/active) | 526-576 | REAL | S4 | F-002 staged-inert full-strength disabled |
| `.settings-about__value` | 581-586 | REAL | — | produced (updates-about.tsx:43/:53 + test) |
| `.settings-area-staged-note` | 590-594 | REAL | — | tertiary on panel 4.58:1 ✓ |
| `[data-staged] …:disabled` | 600-604 | REAL | S4 | F-002 companion (switch/dropdown/field) |
| `.settings-badge` (+--restart, light) | 608-631 | REAL | S4 | F-003 stale 7.8:1 figure; dark 4.79:1 ✓, light fix 9.83:1 ✓ |
| `__badges` / `__reset` (+hover/focus) | 633-665 | REAL | — | per-field reset: 4px 8px quiet affordance |
| `[data-dirty] __row` | 668-674 | REAL | — | 3px inset accent edge + tint; negative-margin compensation |
| `.settings-sections … :last-child` margin drop | 679-681 | REAL | — | frame's zero reserved gutter |
| (deleted `__head` — k12 note) | 683-685 | N/A | — | deletion verified true |
| `.settings-about__rows/__row/dt/dd` | 689-716 | STUB(dead) | S3 | F-001 zero producers, false `<dl>` citation |
| `.settings-advanced` (+__bar/chevron/rows) | 722-760 | REAL | — | 784×38 r2 disclosure; focus-visible present |
| `.settings-screen__restart` | 765-777 | REAL | — | 35% accent border strip; produced (SettingsScreen) |
| `.settings-applybar` (+__count) | 782-802 | REAL | — | sticky bottom + blur; "no frame shows it" — honest |
| `.settings-confirm` (+h2/p/__actions) | 806-828 | REAL | — | dirty-navigation dialog body |
| `@media (width <= 64rem)` | 832-846 | REAL | S4 | F-004 range syntax |
| `@media (prefers-reduced-motion)` | 848-859 | REAL | — | 5 transitions killed; k12 deletion note verified |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| SettingsScreen.test.tsx + 13 area tests (baseline 1705-pass) | DOM/contract only (data-setting, data-dirty, testids) | jsdom — no CSS application; class-name contracts, not style |
| updates-about.test.tsx:68 asserts `.settings-about__value` | YES for that one class | but nothing tests the dead `__row` family (F-001) — tests follow the live grammar, the sheet kept the dead one |
| e2e/visual 13 settings-area baselines | NO as committed evidence | no visual run log exists (list.css F-001 pattern) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The <dl> wrapper (w26.d2 a11y restructure, updates-about.tsx) needs only the UA reset" | settings.css:687-688 | FALSE | no `<dl>` in updates-about.tsx (or any settings file); family dead (F-001) |
| "accent ink on the flipped #d8ba9a tint is 1.75:1" | settings.css:627 | REAL | recompute = 1.75 exact |
| "swaps the ink to content-primary (7.8:1)" | settings.css:628 | SHALLOW (number wrong, direction safe) | actual 9.83:1 (F-003) |
| "staged-inert … the honesty stays in the disabled state + each button's title/aria-describedby reason" | settings.css:555-563 | REAL-UNPROVEN | pattern implemented; per-button reason coverage lives in areas/*.tsx (other agents) — spot-check controls.tsx/updates-about.tsx showed data-staged produced |

## 7. Dead / duplicated / unreachable code
Dead: `.settings-about__rows/__row/dt/dd` (F-001). Everything else in part2 has producers. The sheet's own deleted-rule comments (k12) verified truthful.

## 8. Dependencies & flags
Tokens: ~20 distinct, all defined (verified against tokens.css). Cross-sheet seams: form.css (`.lumen-field/.lumen-segmented/.lumen-swatchgroup/.lumen-switch/.lumen-radiogroup`), setting-field.css (control skins, sibling sheet), subtitles.css (scoped staged rule, noted at :599).

## 9. Open questions for the next agent
- F-001's dead family vs updates-about.tsx (this agent's file 18, audited next): if the `<dl>` grammar was the d2 a11y intent, updates-about regressed it; my file-18 audit will pin which side is current truth.
- Does every staged row actually carry the visible note F-002's justification relies on? Areas lanes (other agents) own the answer; if any staged row lacks it, F-002 upgrades to S3.
- The media range-syntax census (F-004) should be centralized: one policy doc for the webview floor, not 6 scattered queries.
