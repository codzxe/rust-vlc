# AUDIT: ui/src/screens/playlist/options/PlaylistOptionsToolbar.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 50 / 2 components (+1 interface) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — PlaylistScreen.tsx:481 (`<PlaylistOptionsToolbar align="end" />`) and :500 (`<PlaylistOptionsViews …>`) inside the /playlist opened-playlist browse view, under the PlaylistOptionsProvider at :376. W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Grep 'PlaylistOptionsToolbar\|PlaylistOptionsViews\|lumen-plo-toolbar' src (production mounts PlaylistScreen.tsx:481,500; CSS options.css:22; harness/tests otherwise). |

## 1. One-paragraph verdict
Two tiny composition components that do exactly what their 20-line spec says: the toolbar row (Sorting + View menus, align forwarded) and the body switch (grid → ThumbnailGrid, else MediaList with the MediaListProps seam forwarded). Both are shipped on /playlist. The one documented tradeoff is user-silent: switching to Grid view drops the per-entry context menu (Mark as Watched / Add Subtitles / Convert / Properties) because thumbnail cards carry no kebab — a by-design limitation recorded only in code comments, invisible to the user flipping the view.

## 2. What actually works (verified)
— Both mounts real (PlaylistScreen.tsx:481,500); `.lumen-plo-toolbar` styled (options.css:22); align prop threaded to both menus. Baseline: OptionsMenus.test.tsx exercises both components (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsToolbar.tsx:40-46
Code:
```
 * `thumbnail.svg` cards carry no kebab, so switching to Grid view drops the
 * per-entry file context menu (Mark as Watched / Add Subtitles / Convert /
 * Properties) by design. A grid-card context menu is a next-wave surface
 * (needs its own frame); hosts that need the seam in both views should keep
 * the list view until then. */
```
What is wrong: a capability regression that is documented for developers only. The user toggles "Grid View" and silently loses four entry actions; no UI hint exists that the list view carries features the grid lacks.
Why it matters: feature-by-view drift users can discover only by loss; the comment even prescribes the workaround ("keep the list view") for hosts — users get no such advice.
What correct looks like: a one-line hint on the view toggle (or a grid context menu when the frame lands).
Confidence: HIGH — code comment read; no hint string found in the view menu's keys (ViewOptionsMenu is another agent's file — UNVERIFIED whether it carries a hint; grep of its header shows none).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PlaylistOptionsToolbarProps` (interface) | 24-26 | REAL | — | align union |
| `PlaylistOptionsToolbar` | 29-36 | REAL | — | pure composition |
| `PlaylistOptionsViews` | 47-50 | REAL | S4 (F-001) | viewMode switch; props forwarded to list only |

## 5. Test honesty
OptionsMenus.test.tsx (other agent) mounts both; would fail if the switch or toolbar composition broke. The grid-drops-menu behavior is asserted as DESIGN (the k15 note) — a test pinning a capability regression as correct.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "w26.f06 — SHIPPED on /playlist: the toolbar row of the opened-playlist browse view" | PlaylistOptionsToolbar.tsx:12-13 | REAL | PlaylistScreen.tsx:481,500 |
| "an engine PlaylistProvider ancestor is only needed when the data source is the engine queue" | PlaylistOptionsToolbar.tsx:8-10 | REAL | provider reads usePlaylistOptional (PlaylistOptionsProvider.tsx:84) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
MediaList/SortingOptionsMenu/ThumbnailGrid/ViewOptionsMenu + usePlaylistOptions — all exist.

## 9. Open questions for the next agent
- ViewOptionsMenu owner: does the menu carry any hint about F-001's grid limitation (my grep says no — confirm)?
- ThumbnailGrid owner: does the grid render watched-state from metaOf (making the grid LIST-only seam inconsistent — meta shown but not editable)?
