# AUDIT: ui/src/screens/playlist/NewPlaylistDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 217 / 8 |
| File verdict | REAL (genuine locally-persisted create flow with honest disabled surfaces — but the artifact it creates is a display-only playlist whose items are file NAMES with no play seam) |
| Reachable from shipped entry point? | YES (web app build): AppShell.tsx:88/852 mounts it (the shell sidebar's create row) and PlaylistScreen's header CTA opens the same dialog (PlayerScreen's PlaylistsDock path is part3's range; the dock opens the same dialog per PlayerScreen.tsx:102/19-20 comment). Baseline #22 build OK, #23 vitest 1705 passed incl. PlaylistScreen.test.tsx's create-dialog suites (9 cases) + the k14 reset suite. No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `Read` NewPlaylistDialog.tsx 1-217 (complete); `Read playlistsStore.ts 1-60` + `rg -n "createPlaylist|localStorage|itemNames" playlistsStore.ts` (persistence is REAL: `lumen.playlists.v1` localStorage, useSyncExternalStore, corrupt-store fold-back to seed — never crashes); `rg -n "NewPlaylistDialog" src --glob '*.tsx'` (mounts: AppShell:852 + PlaylistScreen); `sed -n '315,410p' PlaylistScreen.test.tsx` (9 behavioral cases incl. save-persists-through-seam, disabled-honest thumbnail, drop-zone label seam); `rg -n "describe|it(" …` (k14 close-without-onClose reset suite at 410-428); `sed -n` quote verification. |

## 1. One-paragraph verdict
A real, carefully-tested dialog: Save genuinely creates and persists a playlist through the localStorage-backed store (`lumen.playlists.v1`), the title gate, focus management (macrotask refocus documented against StrictMode effect ordering), the k14 state-reset-on-close-edge fix, disabled-honest thumbnail, and the drop-zone-as-label seam are all test-pinned. The one substantive gap is the artifact itself: "Select files" records file NAMES ONLY — no paths, no sources — so the created playlist is a display object that can never play anything (the store's own header: "Rows never claim engine playback: the context-menu Play entry stays disabled-honest until a catalog command exists"). The dialog's "N items" feedback implies a playable playlist; the disclosure lives one screen away in a disabled Play entry. Secondary: name-based dedup collapses distinct same-named files and records dropped folders as one "file".

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): the create-dialog suites — open/close/✕, initial focus on the title input, Save disabled until non-empty title, "Save creates the playlist, closes and persists through the seam" (asserts the captured commit contains the new playlist), thumbnail disabled-honest with the titled reason, drop-zone↔input label wiring, honest empty state, and the k14 reset-on-open=false-without-onClose case.
— Verified by read: `playlistsStore` is a real store — localStorage persistence with a memory test seam, corrupt/absent store folds back to the seed fixture (parse at 114-117 filters non-strings), user removals persist (seed never resurrects).
— Verified by read: the dialog's reset logic runs on BOTH edges (close() at 75-80 and the open=false effect at 66-71 — the j14 #7 fix); `onFileChange` clears `event.target.value` so re-picking the same file still fires a change event (99).
— Verified by read: the drop zone is a `<label htmlFor={fileInputId}>` wrapping the hidden file input — clicking anywhere opens the picker natively (test-pinned).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/playlist/NewPlaylistDialog.tsx:82-95 (names-only recording), 108-113 (save); cross-file: playlistsStore.ts:31 ("itemNames: readonly string[]" — "File names recorded by the create dialog (names only, no paths)"), playlistsStore.ts:12-14 ("the context-menu Play entry stays disabled-honest until a catalog command exists")
Code:
```tsx
  const recordFiles = (files: FileList | null) => {
    if (files === null || files.length === 0) return;
    setFileNames((prev) => {
      const seen = new Set(prev);
```
What is wrong: the "Select files" flow (click-to-browse or drag-and-drop) records ONLY `file.name` strings. The saved playlist (`SavedPlaylist.itemNames`) carries no paths and no SourceSpecs — by construction it cannot enqueue or play anything. The store's own honesty note keeps the downstream Play entry disabled "until a catalog command exists" (which, per my effects.tsx audit pattern, does not exist in the catalog). The dialog's feedback — "{N} items" after a drop — presents the artifacts as playlist items; nothing in the dialog says they are labels.
Why it matters: a user who drags five movies into "New Playlist", titles it, and hits Save has created an entry that looks like a playlist, persists like a playlist, and can never play a single file. The honesty apparatus exists but is split across screens (the disabled Play lives in the browse/library views), so the create flow itself oversells the result. Not fraud — the store's docs and the disabled Play disclose it — but a user-visible gap in a core-shaped flow (S3).
What correct looks like: at minimum an inline hint on the drop zone ("file names are recorded for the browse view; playback arrives with the engine playlist seam"), or record `file.path` where the host exposes it (the desktop shell's file dialog) behind the existing seam, so the future play command has data to act on.
Confidence: HIGH — Verification: read (dialog + store traced; the disabled-Play disclosure verified in the store header; tests confirm persistence but assert nothing about playability).

### F-002 [S4]
Where: ui/src/screens/playlist/NewPlaylistDialog.tsx:84-93
Code:
```tsx
    setFileNames((prev) => {
      const seen = new Set(prev);
      const added: string[] = [];
      for (const file of Array.from(files)) {
        if (!seen.has(file.name)) {
```
What is wrong: dedup keys on the bare file NAME. Two distinct files with the same name (`/a/movie.mp4` and `/b/movie.mp4`) collapse to one entry; a dropped FOLDER appears as a single "file" named after the directory (browsers expose folder drops as one entry unless webkitGetAsEntry traversal is implemented — it is not).
Why it matters: edge-case item loss in a flow whose items are already only labels (F-001), so the blast radius is cosmetic today — but the moment real paths are recorded (F-001's fix), this dedup silently drops distinct media.
What correct looks like: dedup on a path/identity key when paths exist; ignore or explicitly traverse directory entries.
Confidence: HIGH — Verification: read (logic traced; folder-drop behavior is standard browser behavior, not executed — UNVERIFIED by run).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `NewPlaylistDialog` (component) | 30-216 | REAL | — | See sub-rows. |
| — focus effect | 54-58 | REAL | — | Macrotask refocus on every open; StrictMode ordering documented; test-pinned. |
| — reset effect | 66-71 | REAL | — | j14 #7: reset on the close EDGE (open=false without onClose); test-pinned (410-428). |
| — `close` | 75-80 | REAL | — | Reset + onClose. |
| — `recordFiles` | 82-95 | REAL | F-001, F-002 | Names-only recording; name-keyed dedup. |
| — `onFileChange` | 99 | REAL | — | Value reset keeps re-pick a change event. Correct. |
| — `onDrop` | 102-106 | REAL | — | preventDefault + dropOver clear + record. Correct. |
| — `save` | 108-113 | REAL | — | Trim + empty gate (Save disabled anyway — double guard); createPlaylist + close. |
| — render | 115-215 | REAL | — | Dialog grammar; labelled input + helper; disabled-honest thumbnail (titled reason, test-pinned); label-wrapped drop zone; count feedback; disabled-until-title Save. |

## 5. Test honesty (PlaylistScreen.test.tsx 315-428, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| opens/closes (Cancel + ✕) | YES | Role/dialog assertions. |
| opening focuses the title input | YES | activeElement assertion with the documented macrotask wait. |
| Save disabled until non-empty title | YES | Disabled-state assertions. |
| Save creates + persists through the seam | YES | Captures the store commit and asserts the new playlist's presence — real persistence oracle (via the test seam, not the mock of the subject: the store's memory double mirrors the localStorage shape). |
| thumbnail disabled-honest | YES | Asserts disabled AND the titled reason. |
| drop zone ↔ input label seam | YES | htmlFor wiring asserted. |
| k14 reset (open=false without onClose) | YES | Re-open asserts clean state. |
| NOT covered: name-only recording semantics (F-001), same-name dedup (F-002), folder drops | — | No test feeds files — the recording path's only coverage is indirect (no test drags FileList entries through the drop zone). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Save creates a LOCAL saved playlist (playlistsStore — no engine command exists)" (header 11-12) | this file | REAL | createPlaylist → localStorage `lumen.playlists.v1` (store read); no playlist.create command in the catalog (rg during effects.tsx audit). |
| "the thumbnail tile is disabled-honest (no backend, the a10 create-row rule)" (12-13) | this file | REAL | disabled + titled hint; test-pinned. |
| "file picks/drops record NAMES only (the media-rows browse view is the wave-b thumbnail/media lane's)" (13-14) | this file | REAL | recordFiles traced; SavedPlaylist.itemNames is string[]. |
| "Playlists you create are saved on this device" (empty-state copy) | PlaylistScreen | REAL | localStorage persistence verified. |

## 7. Dead / duplicated / unreachable code
- No dead code. `fileInputRef` is assigned but never read after mount (the label handles activation) — an unused ref (S4-grade, folded into hygiene; not counted separately).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Button/IconButton, Icon, Dialog/Separator, playlistsStore (localStorage + test seam), usePlaylistMsg lane seam; cross-lane CSS imports (form.css for `lumen-field__control`, playlist.css) — documented Vite-dedupe precedent. No feature flags.

## 9. Open questions for the next agent
- playlistsStore.ts owner (not in my list): does the SEED fixture ("Cool Playlist"/"Cool Songs"/"Anime to Watch" with fake item names) ship in production localStorage on first run? The fold-back-to-seed on corrupt stores means users may see fixture playlists resurrect if their store is corrupted — confirm the seed only fires when the key is ABSENT (comment claims this; line-level verification belongs to that file's auditor).
- PlayerScreen part3 (W15-A04): the dock's create row opens this dialog — confirm the same disabled-Play disclosure reaches the dock's browse face.
