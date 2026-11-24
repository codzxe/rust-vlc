# AUDIT: ui/src/screens/playlist/options/fileRowMenu.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 154 / 3 (playlistTargetKey, fileMenuResolveLabel, fileRowMenu) |
| File verdict | SHALLOW (5/12 rows wired; 7/12 disabled-honest; 2 "wired" rows lose their subject) |
| Reachable from shipped entry point? | YES — PlaylistScreen.tsx:55 imports, :502 builds the menu for the /playlist browse media view (lazy route App.tsx:319) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | full read; grep consumers; grep useSearchParams/useParams in ConvertScreen/MediaInfoScreen (no hits) |

## 1. One-paragraph verdict
The file-row context menu builder: 5 of 12 rows do something real (Mark as Watched -> provider override; Add to Playlist -> local store append with dynamic submenu; Add Subtitle Tracks -> dialog seam; Convert and Properties & Metadata -> route navigation), and 7 rows are permanently disabled with honest in-code reasons (Play, Favorite, Stream, Add to Queue, Show in File Explorer, Go to Source, Add Audio Tracks — all blocked by the names-only store). The two navigation rows are misleadingly shallow: they route to /convert and /media-info WITHOUT passing the file — neither screen reads params — so "Properties & Metadata" on a specific file shows the generic screen, not that file's properties.

## 2. What actually works (verified)
— verified by read + grep: onSetWatched/onAddToPlaylist/onAddSubtitles are real host callbacks (PlaylistScreen supplies them); resolver falls back t -> dynamic targets -> key. 7 disabled rows carry no onSelect (never theater).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/options/fileRowMenu.ts:94-100 and 146-152
Code:
```
    {
      id: "convert",
      labelKey: "playlist.file.menu.convert",
      icon: "convert",
      iconAccent: true,
      onSelect: () => navigate("/convert"),
    },
```
(and identically `onSelect: () => navigate("/media-info")` for id "properties")
What is wrong: the menu item implies a per-file action ("Convert this file" / "Properties of this file") but navigates to a generic screen with zero context — ConvertScreen.tsx and MediaInfoScreen.tsx have no useSearchParams/useParams (grep: no hits). Why it matters: user right-clicks file X, picks Properties & Metadata, and gets the media-info screen for whatever it independently shows — the clicked file is forgotten. What correct looks like: pass the entry (route state or param) or relabel the row. Confidence: HIGH — Verification: read + grep.
F-002 [S3]
Where: ui/src/screens/playlist/options/fileRowMenu.ts:76, 89, 91, 103, 122-128, 131, 143
Code:
```
    { id: "play", labelKey: "playlist.queue.menu.play", icon: "play", iconAccent: true, disabled: true },
```
What is wrong: 7 of 12 rows (58% of the menu) are permanently dead — the file context menu's first and most-expected action (Play) is disabled because saved items are names without sources. Root cause is the playlistsStore design (my playlistsStore audit F-002); the menu honestly disables rather than faking. Census data for the master question: over half this menu's surface is inert. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| playlistTargetKey | 43 | REAL-UNPROVEN | — | dynamic submenu key builder |
| fileMenuResolveLabel | 51-56 | REAL-UNPROVEN | — | 3-tier fallback; safe |
| fileRowMenu | 64-154 | SHALLOW | S3 | 5 wired / 7 disabled / 2 context-losing navigations |

## 5. Test honesty
No dedicated test file found for fileRowMenu.ts in options/ during scan — coverage via PlaylistScreen tests (UNVERIFIED depth).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Convert + Properties & Metadata (the real /convert + /media-info routes...)" | header L9-10 | PARTLY FALSE | routes real, per-file context lost (F-001) |
| "No-seam rows render disabled ... Never theater" | header L13-17 | REAL | 7 rows disabled, no fake onSelect |

## 7. Dead / duplicated / unreachable code
The 7 disabled rows (F-002) — census.

## 8. Dependencies & flags
MenuEntry type, SavedPlaylist/PlaylistEntry types. No feature flags.

## 9. Open questions for the next agent
- MediaList.tsx / PlaylistOptionsMenu.tsx (options lane): confirm the dialog seams fire as wired.
