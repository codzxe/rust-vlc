# AUDIT: ui/src/screens/playlist/NewPlaylistDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 218 / 1 component (+6 inner handlers) |
| File verdict | REAL local UI — Save writes a localStorage-only playlist that can never be played; the "playlist" it creates is a title plus bare FILE NAMES |
| Reachable from shipped entry point? | YES (AppShell.tsx:88/852 mounts it; the /playlist page and the shell sidebar create-row both open it — AppShell.tsx:22, 778) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | Read playlistsStore.ts (full, 204 lines); rg `NewPlaylistDialog` src (exit 0 — AppShell consumer); rg `Cool Playlist|Anime` (exit 0 — seeds render in PlaylistScreen tests) |

## 1. One-paragraph verdict
A well-built, accessible create dialog whose Save calls `createPlaylist` (playlistsStore.ts:162-173) — a localStorage-backed local store with, by its own header's admission, "no engine command" behind it. Two product-level deceptions survive the in-code honesty: (1) the file picker/drop zone records only `file.name` strings (L82-95, `itemNames`), so a "saved playlist" contains no paths/sources and can NEVER be opened or played by anything — the user completes a real-looking flow whose output is inert; (2) on first run the same store seeds the library with three fake playlists containing fake episode names ("The Wand_S01E01.mp4" etc., playlistsStore.ts:57-76) that render as if they were the user's data. The dialog itself never claims engine playback and disables the thumbnail tile honestly — but the USER-facing result (a playlist that cannot play, next to seeded demo playlists) is theater at the flow level.

## 2. What actually works (verified)
— verified by Read playlistsStore.ts: create/remove/add-item genuinely mutate a persisted local store (`lumen.playlists.v1`, corrupt-store fallback, useSyncExternalStore reactivity — real code, real persistence in both browser and tauri webviews).
— Dialog mechanics verified by read: focus management (L54-58 macrotask refocus), state reset on close edge (L66-71), Save gated on non-empty title (L211), duplicate-name dedupe (L84-94).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/NewPlaylistDialog.tsx:82-95, 108-113 (and playlistsStore.ts:30-31)
Code:
```
  const recordFiles = (files: FileList | null) => {
    if (files === null || files.length === 0) return;
    setFileNames((prev) => {
      ...
        if (!seen.has(file.name)) {
```
What is wrong: the drop zone/file input record ONLY `file.name` — no path, no File handle, no source. `save()` persists `{title, itemNames}`. Nothing in the app can ever resolve those names back to media (the store comment admits "names only, no paths" and "no play seam"). The dialog's entire "Select files" section produces data with zero playback capability — the user believes they built a playlist; they built a string list. Combined with Play being disabled-honest on library rows, the create-playlist feature is a dead end dressed as a core flow (VLC's New Playlist equivalent). Why it matters: highest-traffic hostile-focus pattern — mock-shaped data collected as if functional. What correct looks like: record `File.path`/handle where the host exposes it, or defer the file section until a play seam exists. Confidence: HIGH — Verification: read (dialog + store).
F-002 [S3]
Where: ui/src/screens/playlist/NewPlaylistDialog.tsx:26 (import) vs playlistsStore.ts:57-76
Code:
```
import { createPlaylist } from "./playlistsStore";
```
and (store, L57-63):
```
const SEED: readonly SavedPlaylist[] = [
  { id: "cool-playlist", title: "Cool Playlist", createdAt: 0, itemNames: [] },
  {
    id: "cool-songs",
    title: "Cool Songs",
```
What is wrong: the store this dialog writes into seeds first-run state with three fabricated playlists ("Cool Playlist", "Cool Songs", "Anime to Watch" + invented episode names) that render in the /playlist library as user data (PlaylistScreen.test.tsx:77/99-123 confirm they render and are removable). A first-run user cannot distinguish demo seeds from their own playlists; the only marker is that nothing plays. Charged here because the dialog is the store's write path and the seeds' visual sibling. Confidence: HIGH — Verification: read + rg.
F-003 [S3]
Where: ui/src/screens/playlist/NewPlaylistDialog.tsx:148-160
Code:
```
          {/* Thumbnail picker — disabled-honest: no thumbnail backend
              exists (title hint carries the reason, the a10 create rule). */}
          <button
            type="button"
            className="lumen-plnew__thumb"
            disabled
```
What is wrong: a permanently-disabled thumbnail control in the primary create flow — honest, but shipped dead chrome in the frame's most prominent position (110×110 tile next to the title input). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NewPlaylistDialog | 30-217 | REAL | S2 | local-only output (F-001) |
| focus effect / reset effect | 54-71 | REAL | — | StrictMode-proof focus; reset on close edge |
| close | 75-80 | REAL | — | |
| recordFiles | 82-95 | REAL code / inert output | S2 | names only (F-001) |
| onFileChange / onDrop | 97-106 | REAL | — | drop zone works |
| save | 108-113 | REAL | — | gated, trims, persists locally |

## 5. Test honesty
No tests in this file; PlaylistScreen.test.tsx asserts the SEED rows render (L77) — the test suite pins the mock-seed behavior as expected output rather than flagging it. Would fail if seeds were removed — a test protecting the deception pattern, not the user.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "HONESTY: Save creates a LOCAL saved playlist (playlistsStore — no engine command exists)" (L11-12) | module header | REAL | accurate; but see F-001 — local-only is disclosed in code, not to the user |
| "file picks/drops record NAMES only" (L13-14) | module header | REAL | verified L82-95 |

## 7. Dead / duplicated / unreachable code
Thumbnail button (F-003) dead by design. The created playlists are unreadable by any engine path (F-001).

## 8. Dependencies & flags
components/{button,icon,surface}, form.css classes, playlistsStore (localStorage), usePlaylistMsg lane seam. No feature flags.

## 9. Open questions for the next agent
- playlistsStore.ts owner: who decided seeds are acceptable product data? (F-002 — recommend empty first-run or explicit "Sample" badge.)
- When `playlist.save`/`playlist.list` ever land in the catalog, the names-only `itemNames` shape must migrate to sources — a schema break hidden inside a "v1" localStorage key.
