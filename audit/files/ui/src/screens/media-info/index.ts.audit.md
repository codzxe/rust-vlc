# AUDIT: ui/src/screens/media-info/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 (recovered by W15-R3) |
| Date | 2026-11-24 |
| Lines / functions | 6 / barrel (3 export statements, 5 symbols) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: App.tsx:37 lazy-imports this barrel for the /media-info route (App.tsx:320) and PlayerScreen.tsx:99 imports MediaInfoPanel/MediaInfoProvider from "../media-info" (mounted PlayerScreen.tsx:1545-1547) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg export-source checks (exit 0); rg consumers of MediaInfoState outside module (exit 1 = none); sed quote verification (exit 0) |

## 1. One-paragraph verdict
A clean 3-statement barrel. All 5 re-exported symbols exist at their sources (MediaInfoPanel.tsx:70, MediaInfoProvider.tsx:20/39/92, MediaInfoScreen.tsx:16). Both barrel consumers resolve: the App lazy route seam and the PlayerScreen shell region. The header comment's three claims are all true. One dead re-export: the `MediaInfoState` type has zero consumers outside the module.

## 2. What actually works (verified)
— rg "export (function|const|type|interface)" per source -> all 5 symbols present at cited lines.
— Route body claim: `sed -n '318,322p' ui/src/App.tsx` -> `<Route path="/media-info" element={<MediaInfoScreen />} />` and App.tsx:37 lazy-imports the barrel's named export.
— Player shell claim: PlayerScreen.tsx:1543-1546 mounts `<MediaInfoProvider><MediaInfoPanel /></MediaInfoProvider>` for region.id === "media-info".

## 3. Findings
F-001 [S4]
Where: ui/src/screens/media-info/index.ts:5
Code:
```
export { MediaInfoProvider, useMediaInfo, type MediaInfoState } from "./MediaInfoProvider";
```
What is wrong: `MediaInfoState` is re-exported but has zero consumers outside screens/media-info (rg across ui/src, ui/scripts, ui/tools: only the definition site MediaInfoProvider.tsx:20 and this barrel line). What correct looks like: either a consumer imports the type or the re-export is dropped; as a public-seam type on a frozen barrel this is tolerable hygiene debt, not a lie. Confidence: HIGH — Verification: ran (rg exit 1 = no matches).

## 4. Per-function table
N/A (<100 LOC barrel): each of the 5 symbols verified against its source and consumer.

## 5. Test honesty
No tests target the barrel itself. The modules it re-exports carry MediaInfoPanel.test.tsx and MediaInfoScreen.test.tsx (W15-A08 audited MediaInfoScreen.tsx: REAL, provider sends media.get-info over the real bridge).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the /media-info route body" (L1) | index.ts:1 | TRUE | App.tsx:320 route + App.tsx:37 lazy import of this barrel |
| "the shared panel (player shell region content)" (L1-2) | index.ts:1-2 | TRUE | PlayerScreen.tsx:99 + 1545-1547 (region.id === "media-info") |
| "the state seam" (L2) | index.ts:2 | TRUE | MediaInfoProvider.tsx:39/92, consumed by MediaInfoPanel.tsx:40,81 |

## 7. Dead / duplicated / unreachable code
`type MediaInfoState` re-export (F-001). fixtures.ts and format.ts are deliberately NOT re-exported (internal + test/demo imports only: demo-transport.ts:31, PlayerScreen.test.tsx:15) — correct scoping, not a defect.

## 8. Dependencies & flags
None. No side effects, no cycles (sources import siblings directly, never via the barrel).

## 9. Open questions for the next agent
- None.
