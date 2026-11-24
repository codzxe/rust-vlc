# AUDIT: ui/src/screens/playlist/playlistsStore.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 203 / 11 (coercePlaylist, hydrate, commit, subscribe, getSnapshot, usePlaylists, nextId, createPlaylist, removePlaylist, addItemToPlaylist, resetPlaylistsForTests) |
| File verdict | SHALLOW (sound local store; fixture seed shipped as user data; zero engine seam by design) |
| Reachable from shipped entry point? | YES — usePlaylists consumed by /playlist (PlaylistScreen) and AppShell.tsx:487 (sidebar rows); NewPlaylistDialog Save -> createPlaylist (AppShell.tsx:852 mounts the dialog) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | full read; grep usePlaylists/createPlaylist consumers; NewPlaylistDialog.tsx read (Save path, name-only recording) |

## 1. One-paragraph verdict
A correctly-built localStorage-backed store (`lumen.playlists.v1`, useSyncExternalStore, corrupt-store fallback, validated coercion, test seam) — but it is the entire backend for the saved-playlists feature: no engine command exists, so saved playlists never reach the player, and the create dialog throws away the real file sources it reads, persisting display NAMES only. Worse for the master question: the store SEEDS three fabricated playlists ("Cool Playlist", "Cool Songs" with three fake tracks, "Anime to Watch" with four fake episodes) into every first-run profile, rendered in the app shell sidebar and the /playlist library with no "sample" marking — fixture data baked into production code and presented as the user's library.

## 2. What actually works (verified)
— verified by read: create/remove/addItem all commit + persist; hydrate folds corrupt JSON to seed without crashing; empty stored list stays empty (never resurrects seed); ids collision-safe. Consumers confirmed by grep (AppShell.tsx:487, PlaylistScreen, NewPlaylistDialog).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/playlistsStore.ts:57-77
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
What is wrong: first-run users get three fake playlists with fabricated media names ("The Wand_S01E01.mp4"...) presented in the shell sidebar and the library screen as their own saved playlists. No UI marks them as samples; the disclosure lives only in code comments ("structural fixture precedent"). Why it matters: fixtures baked into production code (spec checklist 6.1); a user who deletes "Cool Songs" believes they deleted something real; a user who tries to play "Anime to Watch" hits an inert grid (see PlaylistBrowse F-001). What correct looks like: an honest empty state, or sample rows labeled "Sample". Confidence: HIGH — Verification: read (no sample marker in AppShell row rendering, AppShell.tsx:799-830).
F-002 [S3]
Where: ui/src/screens/playlist/playlistsStore.ts:21-22 and 34-35
Code:
```
 * context-menu Play entry stays disabled-honest until a catalog command
 * exists. File names recorded by the create dialog are names only — the
```
and
```
  /** File names recorded by the create dialog (names only, no paths). */
  readonly itemNames: readonly string[];
```
What is wrong: the entire saved-playlist feature is display-only by construction: no engine seam (no playlist.list / create / play commands for saved playlists), and itemNames cannot ever be played because sources are discarded at save time. The store is honest about it in comments, and Play stays disabled — but this means a headline feature (playlists library, create dialog, browse grid, shell sidebar) is a local list of strings wired to nothing downstream. Confidence: HIGH — Verification: read + engine command catalog cross-check (commands.ts has playlist.* for the QUEUE only: enqueue/play-item/next/prev/...; no saved-playlist persistence commands).

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| coercePlaylist | 116-127 | REAL-UNPROVEN | — | field/type validation; bad records dropped |
| hydrate | 131-140 | REAL-UNPROVEN | — | corrupt -> seed; empty list respected |
| commit | 145-149 | REAL-UNPROVEN | — | save + notify |
| subscribe / getSnapshot | 151-158 | REAL-UNPROVEN | — | correct useSyncExternalStore plumbing |
| usePlaylists | 161-163 | REAL-UNPROVEN | — | reactive read; used by shell + screen |
| nextId | 166-168 | REAL-UNPROVEN | — | time+random id |
| createPlaylist | 170-184 | REAL-UNPROVEN | — | non-empty gate; persists locally |
| removePlaylist | 186-190 | REAL-UNPROVEN | — | no-op on unknown id |
| addItemToPlaylist | 192-205 | REAL-UNPROVEN | — | dedupe; no-op on unknown id |
| resetPlaylistsForTests | 207-212 | REAL-UNPROVEN | — | test seam, labeled |
| (SEED fixture) | 57-77 | FALSE-ish | S2 | fabricated user data (F-001) |

## 5. Test honesty
No test file in this module; consumers' tests (AppShell.test.tsx "Save persists through the shared playlistsStore", PlaylistScreen tests) exercise create/persist honestly against the memory seam. Would they catch the fixture-as-user-data problem? No — tests seed expectations around the fixture.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no `playlist.list` and no create/remove commands for SAVED playlists" | header L7-9 | REAL | generated/commands.ts playlist.* = queue commands only |
| "Seeded only when the store is absent; user edits (removals) persist, so the seed never resurrects" | L52-54 | REAL | hydrate() logic L131-140 |
| "the rows read the REAL saved-playlists store ... no dead chrome, no second data source" (AppShell header) | AppShell.tsx:20-23 | PARTLY FALSE | the "REAL store" is this localStorage fixture-seeded seam — one source, yes; real backend, no |

## 7. Dead / duplicated / unreachable code
None; memoryPlaylistsPersistence is a labeled test double.

## 8. Dependencies & flags
react useSyncExternalStore; localStorage; no transport import at all (by design). No feature flags.

## 9. Open questions for the next agent
- Does the engine roadmap carry saved-playlist persistence (config area? playlist.save exists for the QUEUE — could save into this store with sources)? playlist.save command exists in commands.ts (`{ args: { format...; path: string }; cmd: "playlist.save" }`) — an engine seam the store ignores. Worth a cross-lane note.
