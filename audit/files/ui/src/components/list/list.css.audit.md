# AUDIT: ui/src/components/list/list.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 491 / 62 rule blocks |
| File verdict | REAL-UNPROVEN (grammar live: playlist/media/settings/sidebar rows; rendered in committed baselines but no committed run ever diffed them) |
| Reachable from shipped entry point? | YES as web-UI source: `list.css` imported by Empty.tsx:2, Virtual.tsx:3, Tabs.tsx:3 (components/); screens: `PlaylistScreen.tsx:58`, `PlaylistPanel.tsx:47`, `NewPlaylistDialog.tsx:28` import playlist.css which composes over `lumen-lrow`/`lumen-vrow` classes; Tabs consumed by `ControlBar.tsx` + `ToolsDialog.tsx`. Same W15 caveat: no GUI binary (W14; baseline #2 exit 101), no `<canvas>` (W0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `rg -rho "lumen-(tabs|scrollarea|vlist|vtable|vrow|lrow|empty)[a-z_-]*" src/components/list/*.tsx \| sort -u` (two-way: every rule produced; TSX-only `lumen-tabs` bare + `lumen-scrollarea__content` → F-004); `sed -n '20,120p' Virtual.tsx` + render tree at :140-160 (spacer height=totalHeight inline, rows absolute top=i*itemHeight — CSS containing-block contract holds: `.lumen-vlist__spacer{position:relative}` :124-127); python3 WCAG recompute (empty-state comment's "3.2:1/2.45:1" NOT reproducible from current tokens — see F-002); `ls e2e/visual/baselines/linux/ \| wc -l` → 216 PNGs incl. playlist/player-music/settings-*; `ls audit/runs/baseline/` → no visual/playwright run log (F-001); baseline #22/#23 |

## 1. One-paragraph verdict
The list-primitive sheet (Tabs, ScrollArea, VirtualList/VirtualTable, ListRow, ListEmpty) is structurally sound and the virtualization contract is correctly encoded in CSS: the spacer is the relative containing block, rows are absolute with inline top/height, and the sticky-header rule that could never fire was already found and removed in-tree (w26.k03, :165-168). Rows are the best-covered CSS in the wave's sample so far — 216 committed baseline PNGs include the playlist and player-music routes where these rows render — but there is no committed run that ever diffed any of those PNGs (`audit/runs/baseline/` contains no playwright/visual log), so even this coverage is paper. Smaller rot: a radius comment contradicting its own token, a stale contrast figure describing a token value that no longer exists, a 20px kebab hit target below the WCAG 2.5.8 floor, and two hook-only classnames with no rules.

## 2. What actually works (verified)
— verified by render-tree read (`Virtual.tsx` :90-94, :140-160) → `.lumen-vlist` (overflow:auto) > `.lumen-vlist__spacer` (inline `height: range.totalHeight`, `position: relative` at list.css:124-127) > `.lumen-vrow` (absolute, inline `top: i*itemHeight; height: itemHeight` at :59-60) — the CSS/TSX geometry contract is coherent; `.lumen-vrow{left:0;right:0}` stretches rows to the spacer width.
— verified by `rg -n "lumen-lrow" Row.tsx` → all 15 `__lrow` rules have a producing site (density/selected classes at Row.tsx:74, kebab at :62/:69, handle at :79).
— verified by baseline #22/#23 → build + 1705 tests pass; List.test.tsx covers behavior.
— verified by `ls e2e/visual/baselines/linux/` → playlist, player-music, miniplayer, all 13 settings areas present — the `lumen-lrow`/`lumen-vrow` grammar renders in committed baselines (unexecuted — F-001).

## 3. Findings

### F-001 [S3]
Where: ui/src/components/list/list.css:6-12 (frame provenance header) vs audit/runs/baseline/ (absence)
Code:
```
 *   · playlist rows ~66px r10 — icon + title, quiet gray text that brightens
 *     on hover, hover fill, `•••` affordance revealed on hover
```
What is wrong: the row grammar is claimed frame-measured (66px, r10, 3px kebab dots at 6px pitch) and 216 baseline PNGs exist that render these rows (playlist/player-music/miniplayer/settings routes), but `audit/runs/baseline/` contains NO playwright/visual run log — `npm run visual` / `test:e2e` were never executed in the committed evidence set (INDEX.md #21-24 covers only npm ci/build/vitest/4×check). The baselines' generation provenance and current-diff status are both unevidenced.
Why it matters: the one CSS lane in this repo that could actually verify frame fidelity has never been shown to run; a 66px→60px regression or a broken hover fill passes every committed gate, and the committed PNGs cannot even be trusted as goldens (unknown generation date/method).
What correct looks like: a committed visual-run log (even one PR-lane pass) under audit/runs/, or a note in INDEX.md marking the visual lane UNVERIFIED-BUILD like the tauri lane.
Confidence: HIGH — Verification: read (INDEX.md table, baseline dir listing); no run log exists.

### F-002 [S4]
Where: ui/src/components/list/list.css:473-477
Code:
```
   * Lead): tertiary failed the axe color-contrast audit on every consumer
   * (3.2:1 dark / 2.45:1 light). One step up to the title's secondary tier —
```
What is wrong: the quoted failing ratios do not exist in today's tokens. Current `--color-content-tertiary` is #828282 dark / #474747 light → 4.58:1 and 5.44:1 on the panel (#191919/#c6c6c6) — both PASS AA. The 3.2:1 figure matches the pre-w26.f15 tertiary #6b6b6b (my recompute: 3.30:1 on #191919, per menu.css's own history note that tertiary was repinned to #828282 in w26.f15). The comment describes a historical token without saying so.
Why it matters: the stated justification for using secondary ink in the empty body is false as written; a future cleanup "restoring" tertiary per the passing math would still be fine, but anyone auditing a11y from this comment inherits wrong numbers.
What correct looks like: date the comment ("at the time, tertiary was #6b6b6b") or delete the numbers and keep the intent (calm by size, not ink).
Confidence: HIGH — Verification: read + python3 recompute (4.58/5.44 vs claimed 3.2/2.45); #6b6b6b→3.30:1 matches "3.2:1".

### F-003 [S4]
Where: ui/src/components/list/list.css:399-411
Code:
```
.lumen-lrow__kebab {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px; /* structural hit area around the 15px dot cluster */
  height: 20px;
```
What is wrong: 20×20 hit target is below WCAG 2.5.8 (AA, 24×24 CSS px minimum). The row's other affordances clear 24px (kebab is the sole sub-24 target in the sheet). The spacing exception (24px center-to-center to adjacent targets) plausibly saves it — the kebab is the last element of a 66px row — but nothing documents that defense, and compact 40px rows shrink the effective separation.
Why it matters: the smallest, most-used per-row control (every playlist/media row) is the hardest to hit; on touch it fails outright.
What correct looks like: 24×24 (the visual dots stay 15px), or a comment invoking the 2.5.8 spacing exception with the measured pitch.
Confidence: HIGH — Verification: read; spacing-exception analysis UNVERIFIED (no rendered measurement).

### F-004 [S4]
Where: ui/src/components/list/list.css (absence) vs Tabs.tsx:40, Virtual.tsx:16
Code:
```
      <div className={`lumen-tabs ${className}`.trim()}>{children}</div>
```
What is wrong: `lumen-tabs` (bare, Tabs.tsx:40) and `lumen-scrollarea__content` (Virtual.tsx:16) are emitted with no rules in this sheet — hook-only classnames (the panel/list/tab children carry all styling). Same inventory-rot as menu.css F-004.
Why it matters: grep-for-rule maintenance misses; `__content` particularly suggests a styling seam that does not exist.
What correct looks like: drop them, or a sheet-top list of hook-only classes.
Confidence: HIGH — Verification: two-way grep; zero hits for either selector.

### F-005 [S4]
Where: ui/src/components/list/list.css:34
Code:
```
  border-radius: var(--radius-panel); /* selected category rx2 */
```
What is wrong: comment contradicts the declaration — `--radius-panel` is 12px (tokens.css), while "selected category rx2" reads as a 2px frame rounding. Either the token is wrong for a 28px-tall tab (12px on 28px ≈ pill) or the comment is stale; no frame citation resolves it.
Why it matters: the file's own provenance convention ("raw px is structural, frame-measured where marked") is violated by a token whose comment implies a different frame value; the next person cannot tell which side is the typo.
What correct looks like: pick one — `border-radius: 2px` with the frame citation, or a corrected comment naming the 12px source.
Confidence: HIGH — Verification: read; token value 12px verified in tokens.css `--radius-panel: 12px`.

## 4. Per-function table (selector groups)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-tabs__list` | 19-22 | REAL | — | flex row |
| `__tab` (+hover/selected/focus/disabled) | 24-57 | REAL | S4 | radius comment mismatch F-005; focus ring present |
| `__panel` (+focus-visible) | 59-67 | REAL | — | tabpanel focusable — correct ARIA |
| `.lumen-scrollarea` (+focus) | 71-80 | REAL | — | honest native scroll; focusable region |
| scrollbar group (standard + webkit) | 84-109 | REAL | — | dual-engine thin scrollbars, token-colored; correct |
| `.lumen-vlist` (+focus) | 113-122 | REAL | — | scroll container |
| `__spacer` | 124-127 | REAL | — | relative containing block for absolute rows — verified against Virtual.tsx:94 |
| `.lumen-vrow` (+`__cell`, :hover) | 129-156 | REAL | — | absolute row; rx10 raw documented |
| `.lumen-vtable` | 158-163 | REAL | — | flex column |
| `__head` | 169-171 | REAL | — | dead sticky removed in-tree (w26.k03) — honest |
| `__body` | 173-177 | REAL | — | scroll body |
| `__row` / `__cell` (+head) | 179-200 | REAL | — | grid rows absolute; head cell chrome bg |
| `.lumen-lrow` (+compact/comfortable) | 208-237 | REAL | — | 66/40px rows; rx10 vs r6 by density |
| `:hover/:focus-within`, `--selected(:hover)` | 242-259 | REAL | — | selected beats hover — no flicker |
| `__icon` (+selected accent) | 264-274 | REAL | — | orange icon on selected |
| `:focus-visible` | 276-279 | REAL | — | -2px inset ring |
| `__handle` (+pseudo bars, reveal states) | 284-317 | REAL | — | 12px bars 2×2px; hover/focus/selected reveal |
| `[draggable]` cursors | 319-325 | REAL | — | grab/grabbing |
| `__main` / `__title` / `__meta` | 329-352 | REAL | — | title owns width, ellipsis both lines |
| compact title | 356-359 | REAL | — | menu-item look |
| `__duration` | 364-370 | REAL | — | tabular-nums |
| `__trailing` | 374-379 | REAL | — | tertiary ink on panel = 4.58:1 ✓ |
| `__actions` (+reveal) | 384-397 | REAL | — | hover reveal |
| `__kebab` (+hover/focus) / `__kebab-dots`(+pseudo) | 399-447 | REAL | S4 | F-003 20px target; dots decorative aria-hidden (Row.tsx:69) |
| `.lumen-empty` (+title/body/action) | 454-482 | REAL | S4 | F-002 stale comment; ink itself passes |
| `@media prefers-reduced-motion` | 484-490 | REAL | — | kills 3 transitions |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| List.test.tsx (baseline 1705-pass) | Only DOM/behavior | jsdom; asserts roles/classes/keyboard, not styles |
| e2e/visual baselines (216 PNGs) | NO as committed evidence | no visual run log exists (F-001); PNGs never diffed in any committed run |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only. Raw px is structural, frame-measured where marked" | list.css:14-15 | REAL | verified: raw values only at :26/:34/:140/:212/:215/:222/:229-236/:287-310/:333/:424-446, each marked structural/frame |
| "tertiary failed the axe color-contrast audit (3.2:1 dark / 2.45:1 light)" | list.css:474-475 | FALSE (stale) | current tertiary measures 4.58:1/5.44:1 — numbers describe the retired #6b6b6b token (F-002) |
| "focus-visible accent ring (W25 §2 focus floor)" | list.css:11 | REAL | every interactive group carries :focus-visible (7 sites) |

## 7. Dead / duplicated / unreachable code
Hook-only: `lumen-tabs` (bare), `lumen-scrollarea__content` (F-004). The rx10 value is duplicated at :140 and :222 with a documented no-token rationale — acceptable duplication, cross-referenced.

## 8. Dependencies & flags
17 semantic tokens, all defined in generated tokens.css. No flags. Scrollbar group uses both standard (`scrollbar-width/color`) and webkit pseudo-element rails — correct for the two webview engines.

## 9. Open questions for the next agent
- F-001 generalizes to every CSS file in this wave: does ANY committed artifact prove `scripts/visual-run.mjs` ever executed? If the orchestrator can run `npm run visual:pr` once, most REAL-UNPROVEN CSS verdicts in W15 upgrade or downgrade decisively. REQUEST-ORCH-RUN: `cd ui && npm run visual:pr` (needs playwright browsers; sandbox may lack them).
- Tabs.tsx `TabsList` keyboard handler reads `tab.dataset.value` — empty string for valueless tabs silently breaks roving; Row.tsx/Virtual.tsx consumers unaffected but worth a guard (owner: list-lane wave).
