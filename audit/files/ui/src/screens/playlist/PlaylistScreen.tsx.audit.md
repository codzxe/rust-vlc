# AUDIT: ui/src/screens/playlist/PlaylistScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 609 / 9 |
| File verdict | UNREACHABLE (genuine, vitest-proven; no shipped binary reaches the SPA — see reachability) |
| Reachable from shipped entry point? | NO. SPA-internal chain: `App.tsx:33-34` lazy-imports → `App.tsx:319` `<Route path="/playlist" element={<PlaylistScreen />} />`. But no shipped binary mounts the SPA: `lumen` CLI is headless; `lumen-tauri` cannot compile at HEAD — `apps/desktop-tauri/src/app.rs:301` calls `spawn_session_pump` while the only definition is `spawn_event_pump` (`app.rs:190`; E0425 re-verified by W14-A01/A02/A03). Executed only under vitest (baseline run #23, `ui-vitest.log.txt`, 1705 passed). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 2 |
| Commands run | `rg -n "PlaylistScreen" ui/src` (router mount App.tsx:319); `rg -n "Route\|path=" ui/src/App.tsx` (route table); read `ui/src/screens/playlist/playlistsStore.ts` full (SEED, persistence); read `options/fileRowMenu.ts` full (12 rows, 7 disabled); `sed -n '2995,3060p' ui/src/i18n/messages.ts` (useMsg fallback `catalog[key] ?? messages[key]`); `rg -n "aria-rowindex\|role=\|tabIndex" ui/src/components/list/Virtual.tsx` (role=grid + aria-rowindex + tabIndex=0 pins verified :165-208); `rg -n "MEDIA_ROW_HEIGHT\|lumen-plo-mrow__menu" options/MediaList.tsx` (both exist :46/:213); `rg -n "spawn_session_pump\|spawn_event_pump" apps/desktop-tauri/src/app.rs` (:301 vs :190); `rg -n "storage" playlistsStore.ts` (no cross-tab listener); 31 tests inventoried in PlaylistScreen.test.tsx. |

## 1. One-paragraph verdict
This is a competently built, honestly-commented two-level screen (library → browse) whose data plane is entirely client-local: saved playlists are name-only records in `localStorage` (`lumen.playlists.v1`), seeded with fake demo rows, and NOTHING in the file can play, queue, or inspect a single media file through the engine — Play is disabled-honest in both menus, 7 of the 12 file-row menu rows are disabled, and the engine's real playlist (PlaylistProvider/`playlist.changed`) is deliberately not mounted. The code itself is real (arrow-key grid controller, context menus, dialogs all function under vitest), but it is unreachable from any shipped binary because the tauri host crate does not compile at HEAD, and even in a dev browser the screen manages a parallel universe of localStorage names divorced from the engine queue it visually imitates.

## 2. What actually works (verified)
— Router mount verified: `rg -n "PlaylistScreen" ui/src` → App.tsx:33 (lazy import) + App.tsx:319 (`/playlist` route inside GatedShell). Verified (static chain, not executed here).
— Baseline execution: `audit/runs/baseline/INDEX.md` run #23 (`npm test`, vitest) → 1705 passed / 1 skipped, 136 files — includes `PlaylistScreen.test.tsx` (31 tests) covering menus, sort, watched-toggle, persistence, arrow-key grid.
— Disabled-honesty verified: `fileRowMenu.ts` rows 1/3/4/6/8/9/11 carry `disabled: true` with per-row reasons; LibraryRow Play at PlaylistScreen.tsx:573 `disabled: true`. No theater buttons that do nothing.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/playlist/playlistsStore.ts:57-76 (rendered via PlaylistScreen.tsx:294, 355-370)
Code:
```
const SEED: readonly SavedPlaylist[] = [
  { id: "cool-playlist", title: "Cool Playlist", createdAt: 0, itemNames: [] },
  {
    id: "cool-songs",
    title: "Cool Songs",
    createdAt: 0,
    itemNames: ["The Wand (Main Theme).flac", "Voltage.mp3", "Paper Planes.mp3"],
  },
```
What is wrong: Production code seeds every fresh user's playlist library with three fabricated demo playlists containing fabricated media names ("The Wand_S01E01.mp4" etc.). `hydrate()` (playlistsStore.ts:123-130) returns `SEED` whenever the store is absent, and `PlaylistScreen` renders it as the user's real library. The test suite enshrines it (`PlaylistScreen.test.tsx:74` "renders … the seeded frame playlists as rows").
Why it matters: Fixtures baked into production code. A real user's first run of the media player shows media that does not exist on their machine, presented with no visual distinction from real data — the exact "demo content as truth" pattern a hostile auditor flags. VLC does not ship fake playlists.
What correct looks like: Empty library on first run (the `ListEmpty` + CTA state already exists and is tested at test.tsx:393); demo content only behind an explicit dev/profile flag.
Confidence: HIGH — Verification: read (playlistsStore.ts full read; PlaylistScreen.test.tsx:74, 393).

### F-002 [S2]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:19-22, 250-262, 348-354 (with fileRowMenu.ts:74-131)
Code:
```
//    the saved item names — an engine PlaylistProvider is deliberately NOT
//    mounted here: saved items are names-only client data, engine
//    playlist.changed events must not contaminate the view).
```
```
  return playlist.itemNames.map((name, index) => ({
    id: -(index + 1),
    title: name,
    source: null,
    durationSeconds: null,
  }));
```
What is wrong: The entire /playlist screen has zero engine integration. Saved items carry `source: null`, `durationSeconds: null`, negative fake ids; the engine's actual playlist (PlaylistProvider, `playlist.changed`, enqueue) is deliberately not mounted; Play/Stream/Add-to-Queue/Show-in-File-Explorer/Go-to-Source/Add-Audio are all disabled (fileRowMenu.ts:76-143). The screen named "playlist" in a media player cannot enqueue, play, or reveal one single file.
Why it matters: Architectural drift with user-visible impact: the app maintains TWO disjoint playlist concepts — an engine queue that plays and a localStorage name list that renders. A user who "adds" their files to "Cool Songs" by typing names in the create dialog gets a list that can never play anything. The honest-disabled rows are disclosed in comments, but comments don't reach the user; the shipped capability is a notepad.
What correct looks like: Saved playlists persisting resolvable sources (paths/URLs) with Add-to-Queue/Play dispatching the existing engine commands once a catalog seam exists; or the screen labeled as local-notes until then.
Confidence: HIGH — Verification: read (fileRowMenu.ts full; PlaylistProvider exists in same dir with engine wiring — cross-lane; disabled rows enumerated).

### F-003 [S3]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:174-178
Code:
```
      if (activeIndex === null) {
        grid.removeAttribute("aria-activedescendant");
      } else if (found) {
        grid.setAttribute("aria-activedescendant", `${gridId}-row-${activeIndex}`);
      }
```
What is wrong: When `activeIndex !== null` and the row is not found (all 30 `GRID_SYNC_RETRIES` exhausted), a previously-set `aria-activedescendant` is never cleared — the `else` branch that would remove it does not exist. The grid then permanently points at a nonexistent DOM id.
Why it matters: Stale `aria-activedescendant` is an APG violation; screen readers announce a row that doesn't exist. Blast radius is limited because the empty-dataset case unmounts the whole grid (ListEmpty swap), so it needs retries exhausted with the grid still mounted (slow device + fast count shrink).
What correct looks like: `else { grid.removeAttribute("aria-activedescendant"); }` — clearing whenever the active row is not in the rendered window.
Confidence: MEDIUM — Verification: read (logic traced; no runtime repro — jsdom tests at test.tsx:442-520 don't cover retry exhaustion).

### F-004 [S3]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:552-564
Code:
```
    const rect = el.getBoundingClientRect();
    el.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: rect.right - 24,
        clientY: rect.bottom + 4,
      }),
    );
```
What is wrong: The kebab click synthesizes a fake `contextmenu` event at hardcoded pixel offsets (`right - 24`, `bottom + 4`) instead of reading the actual kebab button's position. The menu anchor is geometry guessed from the row rect.
Why it matters: Any layout change (density, padding, icon size, RTL) silently opens the menu at the wrong screen position. It also dispatches on the row `el`, relying on ContextMenu's bubbling listener — a second implicit coupling.
What correct looks like: `ListRow`'s kebab callback carrying the real event/target (the comment admits "kebab callback carries no pointer event" — fix the primitive, don't fake the event), or `el.querySelector('.kebab')?.getBoundingClientRect()`.
Confidence: HIGH — Verification: read (ListRow kebab seam not audited here — primitive owned by list lane).

### F-005 [S4]
Where: ui/src/screens/playlist/playlistsStore.ts:134-153 (consumed by PlaylistScreen.tsx:286)
Code:
```
let persistence: PlaylistsPersistence = localStoragePlaylistsPersistence;
let state: PlaylistsState = hydrate();
const listeners = new Set<() => void>();
```
What is wrong: No `storage` event listener: two windows/tabs of the same app desync silently (each keeps its own in-memory `state`, last writer wins on `localStorage`).
Why it matters: Minor for a single-window desktop shell, but the SPA runs fine in multiple browser tabs today (it's the only shipped-ish surface), and edits made in one tab are lost/overwritten in the other.
What correct looks like: `window.addEventListener("storage", …)` → re-hydrate + notify listeners.
Confidence: HIGH — Verification: read (`rg -n "storage" playlistsStore.ts` → no listener).

### F-006 [S4]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:446-449
Code:
```
  // the saved-playlist scope (rows are presentation; the ••• is the action). In
  // the thumbnail grid view there is no role=grid — the controller no-ops.
```
What is wrong: Keyboard users lose all row navigation when the browse view is switched to Grid View (ThumbnailGrid has no `role="grid"`), while List View has the full arrow-key model. Disclosed in comment, still an inconsistent a11y contract for the same data.
Why it matters: Same dataset, two different keyboard realities depending on an unrelated view toggle; grid-view rows are keyboard-dead apart from tab stops.
What correct looks like: ThumbnailGrid mounting the same `semantics="grid"` VirtualList (or the composite owning a shared roving layer).
Confidence: HIGH — Verification: read (ThumbnailGrid owned by options lane; claim taken from comment + MediaList.tsx:82 uses VirtualList; ThumbnailGrid not re-verified — UNVERIFIED detail).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `requestFrame` | 89-92 | REAL | — | rAF with jsdom fallback; correct typeof guard. |
| `cancelFrame` | 93-96 | REAL | — | Cancels rAF else timeout; symmetrical with requestFrame. |
| `useGridRoving` (hook: sync effect + onScroll + onKeyDown/move) | 110-247 | REAL | F-003 | APG activedescendant model; rowindex read from primitive's aria-rowindex (verified Virtual.tsx:176); bounded 30-frame retry; scroll listener re-attached per effect run; stale-attr gap F-003. |
| `savedPlaylistEntries` | 255-262 | REAL | F-002 | Negative fake ids, source/duration null — honest about being non-engine data. |
| `NavigationSeam` | 270-276 | REAL | — | useInRouterContext guard prevents useNavigate crash in bare mounts; correct seam. |
| `RoutedNavigation` | 278-281 | REAL | — | Thin useNavigate render-prop. |
| `PlaylistScreen` | 283-399 | REAL | F-001, F-002 | Library level: sorted memo, open/create/subs state, dialogs; renders SEED fixture rows on fresh install (F-001). |
| `PlaylistBrowse` | 407-531 | REAL | F-002, F-006 | Browse level; entryMenu wiring real (verified against fileRowMenu.ts + PlaylistOptionsProvider seam); grid no-ops in Grid View (F-006). |
| `LibraryRow` (+`openMenuAtKebab`) | 534-609 | REAL | F-004 | Menu entries honest (Play disabled); kebab fake-event geometry F-004. |

## 5. Test honesty (PlaylistScreen.test.tsx, 31 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders one page heading and the seeded frame playlists as rows" (:74) | YES | Asserts concrete seeded titles in DOM — mutation of render breaks it (and enshrines F-001). |
| "kebab opens the row context menu; Play is disabled-honest, Remove deletes" (:106) | YES | Clicks kebab, asserts menu open + disabled Play + store removal. |
| "Mark as Watched toggles…" (:232), "Add to Playlist appends…" (:246), "Add Subtitle Tracks opens the dialog…" (:263) | YES | Behavior-level assertions through provider + store seams. |
| "library grid: arrows/Home/End move aria-activedescendant…" (:446-520) | YES | Asserts DOM attributes + focus; would catch roving regressions (not retry-exhaustion F-003). |
| "a stored empty library renders the quiet empty state" (:393) | YES | Swaps persistence seam to empty; asserts ListEmpty. |
All 31 inventoried via `rg -n "it\(" `; suite passed in baseline run #23. No tautologies found; no mocks of the subject (store seam doubles are the dependency, not the subject).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every row action is honest — Play … disabled until a catalog command exists" | PlaylistScreen.tsx:32-36 (self-claim) | REAL | fileRowMenu.ts:76/89/91/103/127/131/143 `disabled: true`; LibraryRow :573. |
| "Convert + Properties & Metadata (routes)" as "real seams" | PlaylistScreen.tsx:29-30 | REAL-UNPROVEN | Routes exist (App.tsx:309 `/convert`, :320 `/media-info`); target screens audited by other agents (ConvertScreen has engine-probe seams per header comments; not verified here). |
| Saved playlists "persisted" (playlistsStore) | PlaylistScreen.tsx:32 | REAL | localStorage `lumen.playlists.v1` save/load verified by read + tests (:359). |

## 7. Dead / duplicated / unreachable code
No dead exports in this file. `selectedId` state is live (row selection + back-restore test :169). Whole screen unreachable from shipped binaries (see header) — SPA-internal only.

## 8. Dependencies & flags
React 18 hooks, react-router-dom, i18n lane seams (usePlaylistMsg/useOptionsMsg — verified fallback chain `catalog[key] ?? messages[key] ?? laneCatalog[key]`), list primitives, playlistsStore (localStorage). No feature flags. `useOptionsMsg` return type claims `string` but can return `undefined` at runtime for unknown keys (cast at :439 handles it) — noted for the options-lane owner, not a defect here.

## 9. Open questions for the next agent
- ThumbnailGrid (options lane): confirm it truly lacks `role="grid"` (F-006 UNVERIFIED detail).
- /convert and /media-info screens: are they engine-wired or scaffold placeholders? (Affects F-002's "real routes" framing.)
- Who owns playlistsStore.ts? Its SEED (F-001) and missing storage listener (F-005) deserve a primary finding in that file's own audit.
