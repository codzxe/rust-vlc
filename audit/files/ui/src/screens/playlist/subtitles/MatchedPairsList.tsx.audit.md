# AUDIT: ui/src/screens/playlist/subtitles/MatchedPairsList.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 69 / 1 (MatchedPairsList) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — AddSubtitlesDialog.tsx + barrel -> Add Subtitles flow |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumers; read SubtitlePair (matching.ts:105-108) |

## 1. One-paragraph verdict
Presentational matched-pairs review list (media name · dotted leader ·
subtitle name) with an honest empty state and correct aria (role=list with
label, decorative glyphs hidden). The data (pairing, natural order) comes
from matching.pairSubtitles — this component only renders. One React-key
fragility.

## 2. What actually works (verified)
- Consumers: AddSubtitlesDialog (production) + own test + barrel.
- Empty state renders honest i18n copy (empty-title/empty-body) instead of nothing.

## 3. Findings
F-001 [S4]
Where: MatchedPairsList.tsx:48
Code:
```
      {pairs.map((pair) => (
        <li key={pair.mediaName} className="lumen-plsubs-pair" role="listitem">
```
What is wrong: key is mediaName — two media files with identical names
(same name in different folders, if the scan source ever widens) collide and
React drops/mismatches rows. Today's source is one folder scan where names
are unique, so latent. Confidence: HIGH — Verification: read.
What correct looks like: index-composite key or a stable pair id from the
matcher.

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| MatchedPairsList | 30-69 | REAL | see F-001 |

## 5. Test honesty
MatchedPairsList.test.tsx exists (consumer scan); row/empty-state rendering
assertable there (not read in full — its own assignment if listed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Rows arrive paired + natural-ordered from matching.pairSubtitles" | MatchedPairsList.tsx:15 | REAL-UNPROVEN | matching.ts is another lane's file; the contract is typed (SubtitlePair) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
SubtitleGlyphs, useSubtitlesMsg, matching types. None.

## 9. Open questions for the next agent
- matching.ts auditor: are media names guaranteed unique (single-folder
  scan)? If yes, F-001 stays latent forever.
