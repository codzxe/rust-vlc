# AUDIT: ui/src/screens/playlist/options/ViewOptionsMenu.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 39 / 3 (ViewOption interface, VIEW_MODES const, ViewOptionsMenu) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the dev/test shell — PlaylistScreen.tsx:481 mounts PlaylistOptionsToolbar → ViewOptionsMenu (toolbar:33) on the /playlist route (route mounted: App.tsx, cross-ref W15-A08 file 1). No shipped binary exists (W14 pin). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | PlaylistOptionsMenu.tsx read in full (generic entries, measure/flip/cap, APG keyboard); PlaylistOptionsProvider.tsx:29-152 read (viewMode state, no persistence); PlaylistOptionsToolbar.tsx:47-50 (the view switch); OptionsMenus.test.tsx:128-177 read (view-menu tests); g19 findings re-checked (View_Options verdict MATCH; g19-P1-1 clipping fixed by w26.gfix-E measure() — verified present at PlaylistOptionsMenu.tsx:179-222); rg for persistence in provider (none) |

## 1. One-paragraph verdict
Small, genuine, fully wired: the two-row view menu is frame-true (rows, icons, default-checked List View), the selection round-trips through the provider to a real body switch (MediaList ↔ ThumbnailGrid, verified in code AND by an end-to-end test), and the shared PlaylistOptionsMenu it rides now carries the g19-P1-1 fix (open-measure/flip/scrollport-cap + resize re-measure) — irrelevant for this 2-row menu but present. Three hygiene nits only: an identity `.map` that rebuilds equal objects every render, an unchecked `id as PlaylistViewMode` cast that erases the union the entries define, and view-mode state that resets on provider remount (no persistence — consistent with the repo-wide W11 pin, but user-visible).

## 2. What actually works (verified)
- Wiring chain traced end to end: ViewOptionsMenu → setViewMode → PlaylistOptionsProvider (viewMode state) → PlaylistOptionsViews (`viewMode === "grid" ? <ThumbnailGrid /> : <MediaList …/>`, toolbar:49) → mounted at PlaylistScreen.tsx:500. The menu changes what the user sees.
- Frame truth: g19's extracted record — "View Options.svg — 265×104; row 1 'Grid View' (2×2 square icon, #8F8F8F stroke1.5) transparent; row 2 'List View' (list icon) #1F1F1F + orange check" — matches VIEW_MODES (grid/list ids, icons) and DEFAULT_VIEW_MODE = "list" (provider:32), so the checked row is row 2 as in the frame. g19 per-screen verdict: View_Options.png **MATCH**.
- Tests (OptionsMenus.test.tsx:128-177): 2 menuitemradio rows with exact labels + leading icon svgs; exactly one aria-checked row = "List View"; and the switch test — clicking "Grid View" replaces the 3 media rows with the "Playlist thumbnails" list (roles asserted both before and after). These would fail on any wiring break.
- The trigger mirrors the active mode's icon (line 25: `viewMode === "grid" ? "grid" : "list"`) as the header claims; icon-only IconButton gets an accessible name via triggerLabel.
- Baseline vitest 1705/1705 (INDEX #23); k15 measurement pass recorded "View menu: 2×48px rows … 'List View' checked" 9/9 PASS (w26.k15.md:179-184).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/playlist/options/ViewOptionsMenu.tsx:28
Code:
```tsx
      entries={VIEW_MODES.map(({ id, labelKey, icon }) => ({ id, labelKey, icon }))}
```
What is wrong: The map destructures each entry and rebuilds an identical object — a no-op transformation allocating a fresh array on every render. `entries={VIEW_MODES}` is exactly equivalent (ViewOption is structurally assignable to OptionsMenuEntry; entries is a module constant).
Why it matters: Noise sold as mapping; a reader assumes the transform does something (drops a field? reorders?) and must diff the shapes to learn it does not. Minor garbage per render.
What correct looks like: `entries={VIEW_MODES}`.
Confidence: HIGH — Verification: read; both interfaces compared field-by-field.

### F-002 [S4]
Where: ui/src/screens/playlist/options/ViewOptionsMenu.tsx:30
Code:
```tsx
      onSelect={(id) => setViewMode(id as PlaylistViewMode)}
```
What is wrong: `PlaylistOptionsMenu` types entry ids as plain `string` (OptionsMenuEntry.id: string, onSelect: (id: string)), so the `PlaylistViewMode` union defined by VIEW_MODES does not survive the round-trip — the cast re-asserts it without checking. `setViewMode` does no validation (provider:87 is a bare useState setter).
Why it matters: The generic exists for labelKey (`K extends string`) but not for id — the one value that flows back out. A future entry with a typo'd id would compile, render, and on click set `viewMode` to a string that matches neither "grid" nor "list" (falling through to MediaList by the `=== "grid"` test — silently wrong rather than crashing).
What correct looks like: Make OptionsMenuEntry/Props generic on the id type too (`K extends string` for labelKey, `Id extends string` for id/onSelect/value), or validate in the sink.
Confidence: HIGH — Verification: read; PlaylistOptionsMenu.tsx:26-70 generic signatures checked.

### F-003 [S4]
Where: ui/src/screens/playlist/options/ViewOptionsMenu.tsx:24 (with PlaylistOptionsProvider.tsx:86)
Code:
```tsx
  const { viewMode, setViewMode } = usePlaylistOptions();
```
What is wrong: viewMode is bare component state — no localStorage, no config.set-setting, no engine store — so the user's Grid View choice resets to List View whenever the provider remounts (route change away and back, app restart).
Why it matters: A view preference is exactly the kind of thing users expect to stick; it also mirrors the W11 settings-persistence pin (nothing persists in the real host), so this is a second, smaller instance of the same class. Design intent unverified — no doc found requiring persistence, hence S4 not S3.
What correct looks like: Persist via the settings store (config.set-setting surface, once the engine side actually persists — cross-ref W11-A02) or localStorage in the dev shell, keyed like the onboarding seed.
Confidence: HIGH on the behavior (state traced, no persistence code), LOW on intent (no requirement doc) — Verification: read; rg for storage in the provider: none.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ViewOption` | 11-15 | REAL | — | id typed PlaylistViewMode (the union F-002 erases downstream) |
| `VIEW_MODES` | 17-20 | REAL | — | module const, stable identity (good for the shared menu's measurement deps: entries.length) |
| `ViewOptionsMenu` | 22-39 | REAL | S4 | pure wrapper over PlaylistOptionsMenu; identity map (F-001); unchecked cast (F-002); align prop pass-through (start/end) honored by the shared menu's `--${align}` class |

## 5. Test honesty (if tests exist in/for this file)
OptionsMenus.test.tsx:128-177 (wrapper tests, honest header "frame data (View Options.svg)").
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "lists the two frame options with their icons" (:129-138) | YES | exact labels, count 2, leading icon svg presence per row |
| "frame default: List View is the checked row; trigger shows the mode icon" (:140-146) | YES (checked-row half) | exactly one aria-checked=true and it is "List View"; the trigger-icon half of the name is not asserted (assertion list ends at the checked row) |
| "selecting Grid View switches the shared body to the thumbnail grid" (:148-159) | YES | the full round-trip: rows present before, click, thumbnails list after, rows gone — a wiring break anywhere in the chain fails this |
| Mutation check (reasoned): swap VIEW_MODES icons (grid↔list) | PARTIALLY caught | icons asserted present, not which icon is on which row — a swapped pair passes the icon assertion but changes the trigger glyph (untested) |
| Mutation check (reasoned): cast corruption (F-002) | NOT caught | only valid ids ever flow in tests |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "two rows — grid icon 'Grid View', list icon 'List View' (checked in the frame)" | ViewOptionsMenu.tsx:1-3 | REAL | g19 frame extraction (View Options.svg 265×104, row 2 checked) + DEFAULT_VIEW_MODE "list" + tests |
| "The trigger mirrors the active mode's icon (list while in list view)" | ViewOptionsMenu.tsx:3 | REAL | line 25; default list → list glyph |
| (adjacent, g19-P1-1) "PlaylistOptionsMenu is a bespoke popup with no measuring at all" | w26 findings g19 | FIXED | w26.gfix-E measure()/flip/cap present (PlaylistOptionsMenu.tsx:179-222), resize-aware; irrelevant for this 2-row menu but the defect is closed |

## 7. Dead / duplicated / unreachable code
- No dead code; the identity map (F-001) is dead computation, not dead code.

## 8. Dependencies & flags
Imports: PlaylistOptionsMenu (shared bespoke menu), PlaylistOptionsProvider (context), useOptionsMsg (lane i18n seam), IconName type. No engine/bridge imports — the view switch is pure client state (correct: view layout is a UI concern). No feature flags. The honest-degradation `hint` prop of the shared menu is unused here (both rows are live) — correct absence.

## 9. Open questions for the next agent
- The shared PlaylistOptionsMenu's id-typing hole (F-002) affects SortingOptionsMenu too (same `as PlaylistSortKey`-style sink) — a one-generic fix covers both.
- Whether view-mode should persist is a product question the options lane never recorded; if the Lead says yes, it joins the W11 persistence seam list.
