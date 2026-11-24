# AUDIT: ui/src/screens/playlist/index.ts
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A05) |
| Date | 2026-11-23 |
| Lines / functions | 10 / 0 (barrel) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — main.tsx -> App.tsx:34 `import("./screens/playlist").then((m) => ... m.PlaylistScreen)` (lazy /playlist route) + PlayerScreen.tsx:100-101 imports PlaylistPanel/PlaylistProvider/usePlaylist via this barrel |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | test -f on 5 target modules (all OK); rg export declarations (all 12 value exports + 6 types resolve); rg barrel consumers (exit 0) |

## 1. One-paragraph verdict
Clean barrel: 10 value re-exports + 6 type re-exports, every target verified a
real declaration — PlaylistPanel.tsx:95, PlaylistScreen.tsx:283,
PlaylistProvider.tsx:84/305, all six queue.ts pure functions (:9/:25/:35/:42/
:47/:58), enqueueSources.ts:19/:47, and the six types (PlaylistProvider.tsx:
39/46/60/80, enqueueSources.ts:12). Two shipped consumers import through it
(App lazy route, PlayerScreen). No dangling exports, no logic, no lies.

## 2. What actually works (verified)
— verified by `rg -n "export (function|const|type|interface) ..." across the
  5 target files` -> all 16 re-exported symbols exist at cited lines.
— verified by `rg -n "screens/playlist" src/App.tsx src/screens/player` ->
  App.tsx:34 + PlayerScreen.tsx:100-101 real consumers.
— Baseline ui-test.txt: PlaylistScreen/PlaylistPanel/PlaylistProvider/
  enqueueSources test files all in the 136-file passing run.

## 3. Findings
None. Checked: dangling re-exports (none), name mismatches (none), circular
imports (none — file imports nothing), dead barrel (none — 2 consumers),
shadowing (none).

## 4. Per-function table
Not applicable — barrel, < 100 LOC, zero functions.

## 5. Test honesty
No tests in this file; the re-exported modules carry their own test files
(queue.test.ts, PlaylistPanel/Screen/Provider.test.tsx, enqueueSources.test.tsx)
— all executed in baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "w26.k14: the shared enqueue seam (the i16 generalization...)" | index.ts:7-8 | REAL | enqueueEntries/useEnqueueSources re-exported; consumers verified in enqueueSources' own audit lane |

## 7. Dead / duplicated / unreachable code
`usePlaylistOptional` (PlaylistProvider.tsx:318) is deliberately NOT
re-exported here — toolbarItems.tsx:49-51 documents the choice and imports the
deep path. Documented omission, not a defect.

## 8. Dependencies & flags
None. Plain re-export module.

## 9. Open questions for the next agent
None.
