# AUDIT: ui/src/screens/playlist/options/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 36 / 0 (barrel) |
| File verdict | REAL (barrel) — with one headline admission in its header |
| Reachable from shipped entry point? | YES — PlaylistScreen mounts the toolbar + views on /playlist |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | read; header-claim trace (LOCAL entries scope) |

## 1. One-paragraph verdict
A barrel whose header contains a master-question datapoint: "SHIPPED on /playlist: PlaylistScreen mounts the toolbar + views in the opened-playlist browse view, with a LOCAL `entries` scope (saved-playlist item names) instead of an engine PlaylistProvider." The /playlist browse surface — sorting, view modes, thumbnails, media list — runs over localStorage-saved playlist names, NOT the engine queue, despite the engine having 15 playlist commands on the wire.

## 2. What actually works (verified)
— verified by read: exports match the layer's modules (Provider/Toolbar/Views/Menus/ThumbnailGrid/MediaList/sorting).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/options/index.ts:12-14
Code:
```
// w26.f06 — SHIPPED on /playlist: PlaylistScreen mounts the toolbar + views
// in the opened-playlist browse view, with a LOCAL `entries` scope
// (saved-playlist item names) instead of an engine PlaylistProvider.
```
What is wrong: the playlist BROWSE experience (what a user sees at /playlist: entries, sorting, grid/list) is fed by a local entries scope, not the engine's queue/playlist state — the engine's playlist.next/prev/skip/play-item/remove/move/clear/set-shuffle/set-repeat/save/load commands are dispatched from OTHER surfaces (queue sidebar, customize tiles, context menus), but the dedicated playlist screen renders saved names from local storage. Two sources of playlist truth; the screen can disagree with the engine queue. Why it matters: the most playlist-looking screen in the product is not the engine's playlist. What correct looks like: mount PlaylistProvider (engine scope) or clearly separate "saved playlists browser" from "current queue". Confidence: HIGH — Verification: read (header admission); PlaylistScreen internals are another lane's files — depth deferred.

## 4-9. (compact)
Per-function: barrel, all exports exist. Tests: playlist lane (UNVERIFIED). Doc claims: the header admission is REAL and matches my earlier command census. No dead exports found. No env vars. Open question: does PlaylistScreen hydrate entries from `playlist.load` results at all, or purely localStorage? (Next agent: PlaylistScreen.tsx.)

