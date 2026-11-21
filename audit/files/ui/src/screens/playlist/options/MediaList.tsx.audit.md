# AUDIT: ui/src/screens/playlist/options/MediaList.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 230 / 4 (MEDIA_ROW_HEIGHT, MediaListProps, MediaList, openMenuAtKebab, MediaRow) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (App.tsx:319 `/playlist` → PlaylistScreen.tsx:54 imports MEDIA_ROW_HEIGHT; the list view renders MediaList through PlaylistOptionsViews; engine scope also mounts it around the player via PlaylistOptionsProvider). 18 behavioral tests executed in baseline (136/136 files, 1705 passed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n 'it\(' MediaList.test.tsx` (18 tests — every header claim pinned incl. :118 "renders nothing when the queue is empty — the panel owns the empty state", :101 "without onEntryMenu no dead ••• renders", :180 local-scope inert rows); Read options.css:250-345 (`.lumen-plo-mrow__play` = position:absolute inset:0 z-index:0 — the stretched-button painting-order claim is real; `.lumen-plo-mrow__menu` z-index:1 above it); `rg -n 'metaOf\|metaSeamWired\|watched' PlaylistOptionsProvider.tsx` (:55-60 — the watched override is client-local, in-memory, "nothing persists" documented); `rg -n 'empty' PlaylistScreen.tsx` (:339/:487 ListEmpty — the panel really owns the empty states, backing the empty-fragment design); Read Virtual.tsx:165-206 (grid semantics: row + gridcell wrappers around each MediaRow — the a11y duplication input). |

## 1. One-paragraph verdict
A small, honest, thoroughly-tested list view whose two clever parts are both real and verified: the stretched transparent play button (position:absolute inset:0 z-index:0 paints above the in-flow title per CSS painting order, so real-browser clicks anywhere on the row hit a native, keyboard-livable button — options.css:253-262 matches the code comment exactly, and the ••• lifts above it at z-index:1), and the honest-degradation ladder (no play seam → presentation-only inert rows; no menu seams → no ••• rendered at all; no metadata seam → honestly-unwatched rows — each rung pinned by a test). Watched state is client-local and non-persistent by documented design (the provider's own comment: "no engine watched command exists … and nothing persists"). The only findings are cosmetic: an empty-state `<></>` return where `null` is the convention, and a minor screen-reader duplication (the gridcell announces the title twice — once as the play button's "Play {title}" label, once as the row text).

## 2. What actually works (verified)
— verified by MediaList.test.tsx:19-183 (18 tests: row pitch, titles, watched states incl. partial bars, honest-unwatched without seam, play dispatch, keyboard-live rows, no dead •••, aria-current, empty-fragment, ContextMenu wrap + right-click + synthesized kebab contextmenu, per-entry entries, local-scope inertness) + baseline ui-vitest (136/136, 1705 passed).
— verified by Read options.css:253-262 + Virtual.tsx:165-181 — the z-index/painting-order architecture the k15 comment describes exists as written: absolute full-row button at z-index 0, kebab at z-index 1, gridcell wrapper.
— verified by `rg 'ListEmpty' PlaylistScreen.tsx` → :339/:487 — the empty-state ownership claim (test :118) is backed by the real panel empty states.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/playlist/options/MediaList.tsx:70
Code:
```tsx
  if (sortedEntries.length === 0) return <></>;
```
What is wrong: Returns an empty Fragment where `null` is the React idiom (and the file's own siblings — e.g. FrameCanvasSurface at MediaSurface.tsx:108 — use `null`). Functionally identical; stylistically inconsistent.
Why it matters: None functionally; noise for the next reader scanning for actual empty-state markup.
What correct looks like: `if (sortedEntries.length === 0) return null;`
Confidence: HIGH — Verification: read.
### F-002 [S4]
Where: ui/src/screens/playlist/options/MediaList.tsx:194 and 209 (structure per Virtual.tsx:181)
Code:
```tsx
        <button
          type="button"
          className="lumen-plo-mrow__play"
          aria-label={`${playLabel} ${title}`}
```
(plus, in the same gridcell)
```tsx
      <span className="lumen-plo-mrow__title">{title}</span>
```
What is wrong: The gridcell (Virtual.tsx:181 wraps each row) contains both the stretched button labeled "Play {title}" and the visible title text — a screen reader traversing the cell announces the title twice (button name, then cell text). The •••'s label adds a third string.
Why it matters: Minor verbosity per row for assistive-tech users; the file's a11y work (named focus targets, :focus-within reveal) is otherwise careful, so this is a gap against its own bar.
What correct looks like: Either the title span becomes `aria-hidden` (the button's label already carries it) or the button's label drops the title and relies on the cell text for context.
Confidence: MEDIUM — Verification: structure verified (Virtual.tsx:165-181 + this file); actual SR announcement order UNVERIFIED (no screen reader in sandbox).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MEDIA_ROW_HEIGHT` | 46 | REAL | — | 51px frame pitch; consumed by PlaylistScreen.tsx:54. |
| `MediaListProps` | 48-62 | REAL | — | Raw seam (onEntryMenu) + shared-menu seam (entryMenu + resolver + label); the both-required guard documented. |
| `MediaList` | 64-123 | REAL | S4 | Empty-fragment F-001 (pinned by test :118 as intended behavior); wrapMenu requires entries AND resolver (raw keys never render — test :145); scrollToIndex to the engine current item; play seam only in engine scope (playlist.playItem). |
| `openMenuAtKebab` | 131-141 | REAL | — | Synthesized contextmenu at the kebab's position; bubbles to the ContextMenu wrapper; pinned by test :137. |
| `MediaRow` | 143-230 | REAL | S4 | Stretched play button (native Enter/Space; stopPropagation); watched/progress rendering clamped 0..1; inert class in local scope; ••• only when a seam exists; F-002 double title. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :58 one click plays (play-item dispatch) | YES | Asserts the dispatched command through a fake transport. |
| :68 keyboard-live rows | YES | Asserts a focusable, named button per engine row. |
| :101/:145 no dead ••• / no raw keys | YES | Negative DOM assertions for each unwired shape. |
| :118 empty renders nothing | YES | Pins the empty-fragment contract (F-001's behavior is intentional). |
| :127/:137 ContextMenu wrap + kebab synthesis | YES | Right-click and click paths both asserted to open the same menu. |
| :170/:180 local scope | YES | No provider → inert presentation rows. |
| Gap: no clamp test for watchedProgress >1/<0 (the clamp exists at :205 but is unpinned). | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "with neither wired every row renders honestly unwatched" | MediaList.tsx:7-10 | REAL | Test :50; provider's resolveMeta default. |
| "never a dead click target (the 'no dead affordance' rule)" | MediaList.tsx:15-17 | REAL | Tests :101/:145/:180 — every unwired shape removes the affordance. |
| "positioned elements paint above the in-flow tile/title, so real-browser clicks land on [the button]" | MediaList.tsx:184-186 + options.css:250-252 | REAL | CSS verified: absolute inset-0 z-0 button; painting order guarantees. |
| "the engine queue itself is never reordered by views" | MediaList.tsx:10-11 | REAL | No dispatch in this file except playItem (plays, doesn't reorder); sorting lives in the provider's sortedEntries derivation. |

## 7. Dead / duplicated / unreachable code
Dead: none (all props and branches exercised by tests). Duplicated: none in-file. Unreachable: none.

## 8. Dependencies & flags
VirtualList (grid semantics), ContextMenu/MenuEntry (shared menu), PlaylistProvider (optional engine scope), PlaylistOptionsProvider (sortedEntries/metaOf), sorting.ts (entryDisplayTitle/meta types). No flags. The watched-override non-persistence is the provider's documented design (cross-noted: the W11 settings-never-persist family — watched marks share it, honestly labeled).

## 9. Open questions for the next agent
- PlaylistOptionsProvider's audit: does any test pin the watched override's session-only lifetime (reload loses marks — "nothing persists")? If a stakeholder demo shows Mark-as-Watched surviving a refresh, that's demo theater the comment already disclaims.
- VirtualList's gridcell + named-button pattern: is the double-title announcement (F-002) acceptable to the a11y lane (e2e/a11y.spec.ts encodes dialog semantics — does it cover grid cells)?
