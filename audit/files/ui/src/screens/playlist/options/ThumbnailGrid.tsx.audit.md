# AUDIT: ui/src/screens/playlist/options/ThumbnailGrid.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 74 / 1 (ThumbnailGrid) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: PlaylistScreen:18 "Grid View → ThumbnailGrid" / player-dock options area → PlaylistOptionsToolbar.tsx:49 `viewMode === "grid" ? <ThumbnailGrid /> : <MediaList …/>` — the View Options grid mode) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read full file + ThumbnailGrid.test.tsx (97 lines) + PlaylistProvider.tsx:220-259 (the playItem seam) · `rg -n "ThumbnailGrid\|playItem" ui/src` (mount sites + seam consumers) · `rg -n "artwork\|thumbnail" src/generated/commands.ts` (ZERO matches — the "no artwork seam" claim verified) · `rg -n "entryDisplayTitle\|formatDurationClock" -A6 sorting.ts` (helper math verified) · `rg -n "playlist.options.grid-label" src/i18n/messages.ts` (key registered :2200) |

## 1. One-paragraph verdict
Zero findings — every claim this file makes is implemented, tested, and traced. The play card is wired to the real engine seam (`playlist.play-item` dispatch, PlaylistProvider.tsx:231-233, test-pinned at the wire-args level `{ id: 1 }`); the local-scope card degrades to honest presentation instead of a dead button (inert div, role=img, test-pinned twice); the artwork well is honestly empty because the engine genuinely has no artwork command (catalog grep: zero matches); unknown durations render "—" rather than a fabricated clock; the empty queue renders nothing because the panel owns the empty state (test-pinned); and the accessible-naming fix (w26.k15: cards named by title, never the duration text) is pinned in both scopes.

## 2. What actually works (verified)
— Play seam: one click → `playlist.playItem(entry.id)` → `dispatch({ cmd: "playlist.play-item", args: { id } })` — ThumbnailGrid.test.tsx:46-52 asserts the exact command + args through a recording transport.
— No-dead-affordance rule: without a PlaylistProvider ancestor the card is `div.lumen-plo-card--inert` with `role="img"` + aria-label — tests :76-96 assert zero buttons, two inert cards, and named img roles.
— Honest unknowns: `formatDurationClock(null|non-finite|negative)` → null → "—" (sorting.ts:76-83, math checked: floor, h/mm/ss zero-pad, the frame's 00-hours form pinned at :23).
— Title fallback chain: title → source basename (both path separators) → unknown-title string (sorting.ts:68-73, pinned :27-31).
— Empty honesty: empty sortedEntries → renders nothing; the panel owns the empty state (:54-57).
— The "no artwork seam yet" header claim is TRUE against the generated command catalog (grep zero matches for artwork/thumbnail).

## 3. Findings
None. Checked and how: fakeness (no stubbed seams — play dispatch verified at the wire level); correctness (duration math, basename split regex, key stability, empty-fragment return); a11y (aria-label beats content for naming — the exact regression the w26.k15 test guards; art well aria-hidden; role=img on the inert card); dead buttons (the local scope's entire design is the counter-example, tested); i18n (grid-label + unknown-title + unknown-duration keys registered); tests (8 tests, all behavioral, all would fail under the mutations they describe).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ThumbnailGrid | 23-74 | REAL | — | engine/local scope split at :47; card body shared; honest unknowns; empty → fragment |
| — dependency: entryDisplayTitle / formatDurationClock | sorting.ts:68-83 | REAL | — | verified math + fallback chain |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :13 labelled grid, one card per entry | YES | role=list name + card count |
| :20 duration clock forms incl. 00-hours + "—" | YES | exact strings |
| :27 title fallback via title attribute | YES | every card's title includes the basename |
| :33 cards named by title, never duration | YES | the w26.k15 regression (aria-label vs name-from-content) |
| :46 click → playlist.play-item {id:1} | YES | wire-level command + args assertion |
| :54 empty queue → no grid | YES | the panel-owns-empty-state contract |
| :66/:76/:86 local-scope trio | YES | no provider → inert cards, named, never buttons |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Cards are buttons: one click plays the item" | header :3-4 | REAL | playItem dispatch; test :46 |
| "the engine has no artwork seam yet, so the well renders honestly empty" | :6-7 | REAL | catalog grep zero; the glyph-only art span |
| "In the /playlist saved-playlist browse … NO play seam — the card renders as honest presentation, never a dead button" | :9-13 | REAL | tests :76-96 |
| "quiet centered h:mm:ss duration in the 37px footer" | :2-3 | REAL (format) | test :20-24 (the 37px is a CSS concern — options.css, audited separately as S4-clean) |

## 7. Dead / duplicated / unreachable code
- None.

## 8. Dependencies & flags
- Icon primitive, i18n seams (useMsg + useOptionsMsg), PlaylistProvider (optional), PlaylistOptionsProvider (sortedEntries), sorting helpers. Engine dependency: `playlist.play-item` (in the generated catalog). No feature flags.

## 9. Open questions for the next agent
- When an artwork seam lands, the art span's honest-empty contract (and its test) must be revisited — the header already says so.
- PlaylistOptionsToolbar's viewMode switch and MediaList's onPlay-optional mirror this file's scope split — their auditor should confirm the two stay in step.
