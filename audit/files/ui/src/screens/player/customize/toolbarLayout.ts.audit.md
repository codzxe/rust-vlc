# AUDIT: ui/src/screens/player/customize/toolbarLayout.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 129 / 7 (toolbarLayoutEqual, parseToolbarLayout, loadToolbarLayout, persistToolbarLayout, clearPersistedToolbarLayout, bandTiles, useToolbarLayout) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — useToolbarLayout consumed by toolbarItems.tsx:135 (ControlBar) ; persist via the customize modal's Save; contract strings pinned |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; cross-check CustomizePanels.test.tsx:463-487 (parse/equal/persist tests); rg useToolbarLayout consumers |

## 1. One-paragraph verdict
Clean localStorage-backed persistence for the UI-owned toolbar arrangement,
with defensive parsing (unknown/duplicate ids dropped, missing catalog ids
healed in frame order), never-throw host degradation, and the correct
same-tab CustomEvent + cross-tab `storage` re-read pattern. The header's claim
"no engine command exists for it (verified against the 1.D4 catalog)" matches
the generated command union (no toolbar command there — checked while auditing
toolbarItems). Tests cover the parser's junk/heal paths.

## 2. What actually works (verified)
- CustomizePanels.test.tsx:463 "parseToolbarLayout rejects junk, heals omissions and duplicates", :476 structural equality, :483 "persistToolbarLayout broadcasts the lumen:toolbar-changed seam" — the parser's nasty paths are test-pinned.
- Consumer chain: useToolbarLayout -> toolbarItems.tsx:135 -> ControlBar (verified in that file's audit).
- Degrade paths: all three storage ops wrapped, comments name the failure mode (unavailable host / corrupted JSON / non-window host).

## 3. Findings
F-001 [S4]
Where: toolbarLayout.ts:52-59
Code:
```
  const bandIds = band.filter((id): id is string => typeof id === "string" && known.has(id));
  if (orderIds.length === 0 && bandIds.length === 0) return null;
  // Missing catalog ids (a tile added post-persist) append in frame order —
  // the arrangement stays a deduped permutation of the catalog.
  const appended = CUSTOMIZE_TILE_IDS.filter((id) => !seenOrder.has(id));
```
What is wrong: a persisted record whose `order` is entirely junk but whose
`band` is valid heals to order = FULL catalog (appended = everything since
seenOrder is empty) rather than being treated as unusable — the palette
arrangement silently resets while the band survives. Asymmetric healing;
harmless (order is cosmetic palette ordering) but undocumented.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TOOLBAR_STORAGE_KEY / TOOLBAR_CHANGED_EVENT | 20-27 | REAL | - | frozen contract strings |
| toolbarLayoutEqual | 30-37 | REAL | - | |
| parseToolbarLayout | 40-61 | REAL | S4 | see F-001 |
| loadToolbarLayout | 64-75 | REAL | - | corrupt JSON -> default |
| persistToolbarLayout | 79-90 | REAL | - | write + broadcast, both guarded |
| clearPersistedToolbarLayout | 93-99 | REAL | - | |
| bandTiles | 102-108 | REAL | - | unknown ids dropped |
| useToolbarLayout | 117-129 | REAL | - | same-tab event + cross-tab storage |

## 5. Test honesty
Covered in CustomizePanels.test.tsx (see §2). Mutation reasoning: breaking
the dedupe/heal logic fails :463; breaking the broadcast fails :483. Honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The layout stays a pure UI concern: no engine command exists for it" | toolbarLayout.ts:11-13 | REAL | generated CommandRequest union has no toolbar cmd (checked in toolbarItems audit) |
| "never throwing on unavailable hosts" | toolbarLayout.ts:4-5 | REAL | all ops guarded (read) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
react hooks, ./panels catalog. No gates.

## 9. Open questions for the next agent
- None.
