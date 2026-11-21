# AUDIT: ui/src/screens/playlist/subtitles/subtitles.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 486 / 38 selector blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle, via /playlist → PlaylistScreen.tsx:391,523 + PlaylistPanel.tsx:293, all mounting AddSubtitlesDialog (open via file-row menu, F2 flow). W14 caveat: no shipped desktop binary (tauri host fails to build, build-desktop-tauri.log.txt exit 101); browser/dev + vitest only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg -l 'lumen-plsubs' src --glob '*.tsx' (9 consumers: AddSubtitlesDialog, SubtitlePickRow/List, MatchedPairsList, FilenameMatchControl, harness); rg 'plsubs-row:hover|transition' across all css (NO hover rule, NO transition on rows); sed -n '285,305p' PlaylistPanel.tsx + '385,400p;515,535p' PlaylistScreen.tsx (all three mount sites pass NO onSave); AddSubtitlesDialog.tsx rg onSave (Save disabled without onSave, tsx:29-34,315); token check (all consumed vars exist). |

## 1. One-paragraph verdict
Frame-exact, densely-cited overlay stylesheet whose grammar is fully rendered by the subtitle-flow components — and whose central interactive surface (the candidate-row list, lines 240-320) is dead in practice: every production mount of AddSubtitlesDialog passes no `onSave`, the comment chain says "No folder-scan / OpenSubtitles seam exists yet, so candidates render the honest empty state", and AddSubtitlesDialog.tsx:61 states "no engine command exists" — so no real subtitle file can ever be listed, selected, or saved by the flow this CSS dresses. The stylesheet itself is competent (selector scoping via `.lumen-plsubs` compounds, honest frame-vs-token deltas with a recorded Lead ruling), but it carries a dead reduced-motion guard and styles interactive rows that have no hover affordance.

## 2. What actually works (verified)
— All 38 selector blocks' classes rendered by the 9 subtitle-flow components (rg). Consumed tokens all exist in tokens.css (incl. --shadow-overlay, --radius-window). Baseline: `npm test` 1705 passed incl. AddSubtitlesDialog/SubtitlePickList/MatchedPairsList suites (ui-vitest.log.txt); `npm run build` OK (ui-build.log.txt).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/subtitles.css:240-320 (candidate rows) + PlaylistPanel.tsx:291-298 + PlaylistScreen.tsx:518-524
Code:
```
      {/* Add-Subtitles flow (w26.f06, e09 F2): scoped to the queue's media
       * names. No folder-scan / OpenSubtitles seam exists yet, so candidates
       * render the honest empty state — never fabricated. */}
```
What is wrong: the entire candidate-row grammar (`.lumen-plsubs-list`, `.lumen-plsubs-row` + `--selected`, `__icon/__name/__check/__cycle`, 80 lines of frame-measured CSS) can never render a real row in any shipped mount: all three production mounts (PlaylistScreen.tsx:391, :523, PlaylistPanel.tsx:293) pass only `open/onClose/mediaNames/initialDestination` — no candidate source exists — and no mount passes `onSave`, so Save is permanently disabled (AddSubtitlesDialog.tsx:315: `onSave === undefined || …` disables it).
Why it matters: dead-button evidence at the wave level — the overlay is UI chrome over a feature with no backend seam; ~80 lines of this file are exercised only by tests and the visual harness.
What correct looks like: either wire a real candidate source (folder scan / OpenSubtitles / engine subtitle track command) + an onSave consumer, or cut the candidate grammar until the seam exists.
Confidence: HIGH — read all three mount sites; rg onSave across the dialog.

### F-002 [S4]
Where: ui/src/screens/playlist/subtitles/subtitles.css:481-486
Code:
```
@media (prefers-reduced-motion: reduce) {
  .lumen-plsubs-row,
  .lumen-plsubs-pair {
    transition: none;
  }
}
```
What is wrong: neither `.lumen-plsubs-row` nor `.lumen-plsubs-pair` declares any `transition` anywhere (rg across all CSS) — the guard disables nothing.
Why it matters: dead rule; signals an intent (rows should transition) that was never implemented, or copy-paste from a sheet whose rows do transition.
What correct looks like: delete the block, or add the intended hover/selection transitions it implies.
Confidence: HIGH — rg 'transition' in this file returns only this block.

### F-003 [S4]
Where: ui/src/screens/playlist/subtitles/subtitles.css:253-264
Code:
```
.lumen-plsubs-row {
  display: flex;
  align-items: center;
  gap: 17px; /* frame: glyph x31–49, text starts x66.7 */
  height: 36px; /* frame: selected band h36 */
```
```
  cursor: pointer;
```
(line 263)
What is wrong: candidate rows are interactive (`cursor: pointer`, click-to-select via optionProps) but have NO `:hover` rule in this sheet or any other (rg: 0 hits) — the only hover affordance in the grammar is the `--multiple` cycle-glyph reveal (lines 317-320). A pointer user gets no feedback on a plain candidate row.
Why it matters: standard state-matrix gap (the repo's own doc 08 §4.4 "every interactive primitive" gets hover); blast radius currently zero only because of F-001 (rows never render in production mounts).
What correct looks like: `.lumen-plsubs-row:hover { background: var(--color-surface-hover); }` or equivalent.
Confidence: HIGH — rg evidence.

## 4. Per-function table (selector blocks)
| Selector block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-dialog.lumen-plsubs` | 30-44 | REAL | — | color-mix 95% alpha; shadow/radius tokens |
| `.lumen-plsubs__window` | 46-54 | REAL | — | |
| `.lumen-plsubs__head` | 64-69 | REAL | — | 96px band pin documented |
| `.lumen-plsubs__titlebar` | 71-79 | REAL | — | 14+28+11=53 math checks |
| `.lumen-plsubs__title` | 82-87 | REAL | — | |
| `.lumen-plsubs__close` | 89-91 | REAL | — | |
| `.lumen-plsubs__controls` | 97-102 | REAL | — | nowrap fix documented |
| `.lumen-plsubs .lumen-dropdown.lumen-plsubs__pill` | 109-111 | REAL | — | compound specificity override, documented |
| `.lumen-plsubs .lumen-plsubs__pill .lumen-dropdown__trigger` | 113-120 | REAL | — | |
| `…__value` / `…__option` | 122-129 | REAL | — | 44px rows |
| `.lumen-plsubs__crumb` (+svg, -path) | 133-156 | REAL | — | |
| `.lumen-plsubs__search` (+svg, -input, placeholder, :focus-within) | 159-196 | REAL | — | focus-within ring correct |
| `.lumen-plsubs .lumen-plsubs-match` (trigger/option) | 203-214 | REAL | — | 362px dropdown |
| `.lumen-plsubs .lumen-dropdown__option:hover/:focus/[aria-selected]` | 220-224 | REAL | — | scoped label-ink pin, documented |
| `.lumen-plsubs__body` | 228-236 | REAL | — | |
| `.lumen-plsubs-list` (+focus-visible) | 241-251 | REAL | S3 (F-001) | dead in production mounts |
| `.lumen-plsubs-row` (+--selected, :focus-visible) | 253-276 | REAL | S3/S4 (F-001/F-003) | no hover rule; rows never render live |
| `.lumen-plsubs-row__icon` | 278-286 | REAL | — | Lead ruling Δ23 documented |
| `.lumen-plsubs-row__name` | 288-294 | REAL | — | |
| `.lumen-plsubs-row__check` (+ .lumen-icon) | 297-307 | REAL | — | |
| `.lumen-plsubs-row__cycle` (+multiple hover/focus) | 311-320 | REAL | — | the only hover affordance |
| `.lumen-plsubs-pairs` | 329-336 | REAL | — | gap-0 fix documented (44px pitch bug) |
| `.lumen-plsubs-pair` | 338-347 | REAL | S4 (F-002) | renders real media names (pairs reachable) |
| `.lumen-plsubs-pair__icon` / `__name` | 349-365 | REAL | — | |
| `.lumen-plsubs-pair__side` | 372-378 | REAL | — | 472px column |
| `.lumen-plsubs-pair__leader` | 383-391 | REAL | — | radial-gradient dot pitch 8.15px |
| `.lumen-plsubs-empty` (+title/body) | 395-413 | REAL | — | the state that ALWAYS renders |
| `.lumen-plsubs__foot` (+ .lumen-button) | 431-444 | REAL | — | 72px min-height pin, math documented |
| `:root:not([data-theme=light]) …__cancel` | 450-454 | REAL | — | raw #1b1b1b, stylelint-disabled, Δ7 documented, dark-only |
| `…__foot .lumen-button--tinted:disabled` (dark) | 461-468 | REAL | — | |
| `…__foot .lumen-button--tinted:disabled` (light) | 470-477 | REAL | — | |
| `@media reduced-motion` | 481-486 | DEAD | S4 (F-002) | guards nonexistent transitions |

## 5. Test honesty
No tests in this file. Colocated suites (AddSubtitlesDialog.test.tsx, SubtitlePickList.test.tsx, MatchedPairsList.test.tsx, FilenameMatchControl.test.tsx, harness.test.tsx) belong to other agents; they render the row grammar with fixtures, which is the only thing keeping F-001's dead CSS "alive".

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens ONLY; raw px appears solely for frame-measured structural dimensions" | subtitles.css:12-14 | FALSE (one exception) | line 452: raw #1b1b1b button fill (documented, no token) |
| "candidates render the honest empty state — never fabricated" | PlaylistPanel.tsx:293-294 | REAL | no candidate source exists; empty state is the only production render |
| "Save reports the selection through onSave and never fakes persistence — without onSave Save renders [disabled]" | AddSubtitlesDialog.tsx:29-31 | REAL | no mount passes onSave (3 sites verified) → Save disabled in every shipped flow |

## 7. Dead / duplicated / unreachable code
F-001 (candidate grammar dead in practice), F-002 (dead reduced-motion guard). The `.lumen-plsubs-empty` state is the only body state that renders in production.

## 8. Dependencies & flags
Pure CSS. Depends on shared Dropdown grammar (`.lumen-dropdown__trigger/__option` — another agent's file) and Button primitives (`.lumen-button`, `--tinted`). All tokens consumed exist.

## 9. Open questions for the next agent
- Does any roadmap row promise "subtitle download/scan" as implemented? If a matrix row claims it, F-001 upgrades to S0 FALSE (check docs/feature-matrix.toml subtitle rows).
- The `:root:not([data-theme="light"])` dark-only raw hex (#1b1b1b) is the same un-tokenized delta logged for the netstream dialog (g20-P3-3) — a third instance would justify a token.
