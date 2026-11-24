# AUDIT: ui/src/screens/playlist/PlaylistScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 609 / 7 (useGridRoving, savedPlaylistEntries, NavigationSeam, RoutedNavigation, PlaylistScreen, PlaylistBrowse, LibraryRow) |
| File verdict | REAL (client-local only; zero backend wiring) |
| Reachable from shipped entry point? | YES (App.tsx:319 `<Route path="/playlist" element={<PlaylistScreen />} />`, lazy-imported App.tsx:33) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | rg playlistsStore persistence (localStorage); rg AddSubtitlesDialog onSave/candidates; sed quote verification; test-list rg |

## 1. One-paragraph verdict
A real, well-built React screen with a genuine a11y arrow-key grid model, natural
sort, context menus, and a working two-level library/browse flow — but every
byte of data is client-local: playlists persist to `window.localStorage`
(playlistsStore.ts:92, key `lumen.playlists.v1`), saved items are name-only
strings with no play seam, "Play" is permanently disabled, and the Add-Subtitles
flow opens a dialog whose Save button can never be enabled (no `onSave` prop is
passed by this screen). Answer to the master question for /playlist: NO UI
element on this screen is wired to a real backend; everything either works
against localStorage or is honestly dead.

## 2. What actually works (verified)
— rg persistence: `window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state))`
(playlistsStore.ts:92) — create/remove/rename persist locally (test at
PlaylistScreen.test.tsx:359 covers the seam).
— useGridRoving (lines 110-247): real APG activedescendant implementation —
row id stamping from `aria-rowindex` (line 163-166), clamped movement, scroll
into rendered window, bounded rAF retry (GRID_SYNC_RETRIES=30), scroll listener
with cleanup (lines 191-196). No leak found on read.
— Route registration verified: App.tsx:319.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:388-396, 523-528 (mounts) and 477-480 (entry)
Code:
```
      <AddSubtitlesDialog
        open={subsPlaylist !== null}
        onClose={() => setSubsId(null)}
        mediaNames={subsPlaylist?.itemNames ?? []}
        initialDestination="playlist"
      />
```
What is wrong: Neither mount passes `candidates` or `onSave`. AddSubtitlesDialog
defaults candidates to `[]` (line 96) and disables Save when `onSave ===
undefined` (AddSubtitlesDialog.tsx:315). So the ENABLED "Add subtitles" toolbar
button (lines 477-480), the library-row menu entry (575-579), and the file-row
menu's "Add Subtitle Tracks" (508) all open a dialog that can never list a
candidate and can never save. Dead flow behind a live button. Why it matters:
a user-visible feature (subtitle management) is 100% non-functional in the
shipped UI; the honesty lives in code comments the user never sees. What
correct looks like: disable/hide the entry points, or wire them to the host
seam the dialog's mount recipe documents.
Confidence: HIGH — Verification: read both files + prop defaults.

F-002 [S3]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:573
Code: `    { id: "play", labelKey: "playlist.library.menu.play", icon: "play", disabled: true },`
What is wrong: Permanently disabled "Play" entry in every library row's context
menu — no catalog command exists. Honest (comment lines 566-569 says so), but it
is a dead menu item shipped in the product. Dead-surface stat: 1.
Confidence: HIGH — Verification: read.

F-003 [S3]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:255-262
Code:
```
function savedPlaylistEntries(playlist: SavedPlaylist): PlaylistEntry[] {
  return playlist.itemNames.map((name, index) => ({
    id: -(index + 1),
    title: name,
    source: null,
    durationSeconds: null,
  }));
}
```
What is wrong: Browse rows are name-only presentation objects — `source: null`
means no row can ever be played, enqueued, or resolved to media. The whole
browse level (grid view, list view, sort options, watched toggles) operates on
strings. Watched state and metadata live in the options provider (client-local).
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: ui/src/screens/playlist/PlaylistScreen.tsx:560-561
Code: `        clientX: rect.right - 24,
        clientY: rect.bottom + 4,`
What is wrong: Magic numbers for the synthesized contextmenu position — the
kebab's real position is measurable (the button is in the row) but is
reconstructed with hard-coded offsets; brittle if ListRow geometry changes.
Quality defect only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| requestFrame/cancelFrame helpers | 87-96 | REAL | - | jsdom-safe rAF fallback |
| useGridRoving | 110-247 | REAL | - | solid a11y controller; idempotent DOM writes; bounded retries |
| savedPlaylistEntries | 255-262 | REAL | F-003 | names-only view model, source null |
| NavigationSeam/RoutedNavigation | 270-281 | REAL | - | router-safe navigate seam |
| PlaylistScreen | 283-399 | REAL | F-001 | localStorage-backed library; Add-Subtitles dead flow |
| PlaylistBrowse | 407-531 | REAL | F-001 | sort/view real; rows presentation-only |
| LibraryRow | 534-609 | REAL | F-002, F-004 | context menu w/ disabled Play |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "kebab opens menu; Play is disabled-honest, Remove deletes" (106) | YES | asserts disabled state + real store delete |
| "Save creates the playlist ... persists through the seam" (359) | YES | exercises localStorage seam |
| "the thumbnail picker is disabled-honest (no backend)" (373) | YES | pins another dead element |
| "Add Subtitle Tracks opens the dialog scoped to the one file" (263) | NO (weak) | asserts the dialog OPENS, never that it can save — masks F-001 |
Tests run: not by me; UI vitest suite 1705 passed / 1 skipped per orchestrator baseline (audit/runs/baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every row action is honest — Play ... disabled until a catalog command exists" | header comment 32-36 | REAL | line 573 disabled: true |
| "candidates arrive through props (the host wires the seams)" | AddSubtitlesDialog.tsx:28-29 | TRUE-BUT-DEAD | no host in this screen passes them; flow unreachable-to-completion |

## 7. Dead / duplicated / unreachable code
Dead UI elements reachable from this screen: Play menu entry (permanently
disabled), Add-Subtitles Save (permanently disabled), Add-Subtitles candidate
list (permanently empty), NewPlaylistDialog thumbnail picker (disabled-honest,
test line 373). No dead code in the TS sense.

## 8. Dependencies & flags
react-router-dom, i18n messages, components (Button/Icon/Card/ContextMenu/List/
VirtualList), playlistsStore (localStorage), options/*, subtitles/* — all
in-repo. No feature flags.

## 9. Open questions for the next agent
AddSubtitlesDialog.tsx owner: confirm no other mount site passes onSave/candidates
(rg showed only PlaylistScreen mounts in screens/; verify shell/app-level mounts).

### Addendum (verified after writing)
rg confirms the dead flow is systemic: PlaylistPanel.tsx:293 mounts
AddSubtitlesDialog with NO onSave/candidates either; only the dev harness
(subtitles/harness.tsx:59,74,89) passes `onSave={() => {}}`. No production
mount in the tree can ever save a subtitle selection. F-001 blast radius
covers both screens.
