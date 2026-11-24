# AUDIT: ui/src/screens/playlist/options/PlaylistOptionsToolbar.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 50 / 2 (PlaylistOptionsToolbar, PlaylistOptionsViews) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — mounted on /playlist (PlaylistScreen.tsx, header comment "w26.f06 — SHIPPED on /playlist") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln` mounts (PlaylistScreen); read of PlaylistOptionsProvider.tsx:1-60 (state scope + honest-degradation disclosures) |

## 1. One-paragraph verdict
Two small composition mounts over the real option menus: the toolbar (sort + view triggers) and the body switch (grid→ThumbnailGrid, else MediaList). Underneath, the provider is honest about scope: sort/view are session-local useState (no persistence claimed), the engine-metadata seam degrades visibly when unwired (metaSeamWired flag + "needs metadata" hints), and Mark-as-Watched is disclosed as a client-local, non-persisted override because no engine command exists. The grid view dropping the per-entry context menu is documented at the switch (j15 P3-5).

## 2. What actually works (verified)
— verified by mount trace (PlaylistScreen) and provider read; sorting menus/thumbnail grid covered by their own tests (options test files present in the dir listing; baseline vitest 1705 passed).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsToolbar.tsx:38-49
Code:
```
 * `thumbnail.svg` cards carry no kebab, so switching to Grid view drops the
 * per-entry file context menu (Mark as Watched / Add Subtitles / Convert /
 * Properties) by design.
```
What is wrong: documented behavior gap, not a code bug — grid view silently removes context-menu functionality that list view has. Disclosed in-comment; users discover it by mode switch. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlaylistOptionsToolbar | 29-36 | REAL | — | align forwarding |
| PlaylistOptionsViews | 47-50 | REAL | S4 | F-001 grid drops entry menu (documented) |

## 5. Test honesty
Covered via options/harness test files (dir listing); the provider's honest-degradation behaviors (metaSeamWired) have their own assertions per the w26.k15 citations.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "SHIPPED on /playlist: the toolbar row of the opened-playlist browse view" (L12-13) | header | REAL | PlaylistScreen mount (rg) |
| "No engine command exists (doc 05 §2.9 has no watched command) and nothing persists" (provider L54-56) | comment | REAL | provider read; catalog has no watched cmd |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
SortingOptionsMenu/ViewOptionsMenu/MediaList/ThumbnailGrid + options provider. No transport — the /playlist library page is local-scope by design (playlistsStore names).

## 9. Open questions for the next agent
- Sort/view preferences not persisting across sessions is session-scope by design here; if a doc claims persisted playlist preferences, that's a doc-side FALSE.
