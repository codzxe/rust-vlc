# AUDIT: ui/src/screens/playlist/subtitles/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 44 / 0 (pure barrel: 13 re-exports + types) |
| File verdict | REAL-UNPROVEN (barrel is live; the exported flow dead-ends in product usage) |
| Reachable from shipped entry point? | YES — PlaylistScreen.tsx:50 imports AddSubtitlesDialog from this tree (both mount sites :391, :523); fileRowMenu row 10 opens the file-scoped one |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; grep consumers; read both mount sites + dialog header contract |

## 1. One-paragraph verdict
Clean public-surface barrel (dialog + match control + pairs list + pick list + glyphs + matching utils + lane msg hook). The tell is the header's own integration recipe: `onSave={(result) => { /* apply/record — no engine command yet */ }}` — and the SHIPPED usage is weaker still: both PlaylistScreen mount sites pass neither `candidates` (no folder-scan/OpenSubtitles seam exists -> honest empty list) nor `onSave` (the dialog contract says Save renders inert without it). So the Add-Subtitles flow, reachable from the playlist UI and file context menu, is a dialog that can be opened and closed but can never select or save anything: a complete dead-end flow, honestly labeled at every layer.

## 2. What actually works (verified)
— verified by grep + read: all 13 re-exports resolve to real modules in the same tree; matching utils (pairSubtitles/normalizeStem/...) are real pure functions; PlaylistScreen mounts the dialog (L391, L523).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/subtitles/index.ts:11-16 (recipe) + PlaylistScreen.tsx:391-396, 523-528 (shipped usage)
Code (recipe, index.ts):
```
 *     candidates={discoveredSubtitleFiles}   // host seam (folder scan / search)
 *     folderPath={["Videos", "Shows", "The Wand", "S01"]}
 *     onSave={(result) => { /* apply/record — no engine command yet */ }}
```
What is wrong: the flow the barrel exposes ships as open->look->close: no candidate seam (empty list by construction) and no onSave at either mount, so Save is inert (AddSubtitlesDialog.tsx:29-31 "without `onSave` Save renders" inert). Why it matters: census — another product flow with zero backend; the "Add Subtitle Tracks" context-menu row (fileRowMenu row 10) leads here. Honesty is maintained (empty state, inert Save) but the feature is a shell. Confidence: HIGH — Verification: read.

## 4. Per-function table
n.a. — barrel file (<100 LOC, compact audit).

## 5. Test honesty
Subtitle pieces have their own tests (AddSubtitlesDialog/matching lanes); would not catch the missing host wiring (F-001) since tests mount with fixtures.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no engine command yet" (onSave recipe) | L15 | REAL | subtitle.add-file exists in the catalog but nothing here calls it — the dialog reports through onSave, which no host passes |
| "the candidate list renders the honest empty state — never fabricated" | PlaylistScreen.tsx:388-389 | REAL | no candidates prop at either mount |

## 7. Dead / duplicated / unreachable code
None in the barrel; the dead-end is in the flow usage (F-001).

## 8. Dependencies & flags
None (re-exports only).

## 9. Open questions for the next agent
- AddSubtitlesDialog.tsx + matching.ts (subtitle lane): depth-audit the pairing logic and the inert-Save contract.
