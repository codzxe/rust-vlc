# AUDIT: ui/src/screens/playlist/options/sorting.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 199 / 8 (SORT_KEYS, 2 consts, 4 helpers, 1 comparator, 1 sort API) |
| File verdict | REAL — pure, honest-degrading sort model; 10/16 sort keys are view-only no-ops until a metadata seam exists |
| Reachable from shipped entry point? | YES (PlaylistOptionsProvider + SortingOptionsMenu + PlaylistPanel via entryDisplayTitle; audited in sibling files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | read (full file); rg `sortUsesMeta|metaSeamWired` consumers (exit 0) |

## 1. One-paragraph verdict
Genuine pure-function module: 16 frame-ordered sort keys, natural compare, total stability via index tiebreaks, and an honestly-optional metadata seam (`PlaylistEntryMeta`) where unknown fields never reorder (unknown-first). The honest-degradation design is real: with no seam wired, artist/album/track/disc/watched sorts (10 of 16 keys, per `sortUsesMeta` L89-95) degenerate to original queue order. Sorting is presentation-only by contract (L179-180 "playlist.move dispatches are NOT fired") — correct, since every `playlist.move` would be refused by the shipped host anyway (see PlaylistPanel audit F-001). The only real criticism: half the sort menu the user sees is structurally inert until an engine metadata seam that does not exist ships, and the module's own "needs metadata" hint machinery exists to disclose it.

## 2. What actually works (verified)
— verified by read: comparator correctness for all 16 keys (watched polarity L121-129, add-order proxy L130-137, natural title/uri L138-147, unknown-first L148-173); input arrays never mutated (map+sort on decorated copy L187-197).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/options/sorting.ts:89-95 (and 64-65)
Code:
```
export const sortUsesMeta = (key: PlaylistSortKey): boolean =>
  key.startsWith("artist-") ||
  ...
  key === "unwatched-first" ||
  key === "watched-first";
```
What is wrong: 10 of the 16 shipped sort options depend on `ResolveEntryMeta`, whose only comment is "the app shell wires engine metadata here (handoff)" (L64-65) — no wire exists (PlaylistOptionsProvider's metaSeamWired === false, cited at L86-88). Those 10 menu rows are permanent identity transforms (stable original order) in every shipped build. Disclosed via the "Needs media metadata" hint — real, not theater — but it is half the feature surface shipped inert. Confidence: HIGH — Verification: read (provider wiring verified in sibling audit).
F-002 [S4]
Where: ui/src/screens/playlist/options/sorting.ts:158, 171
Code:
```
      const d = naturalCompare(av, bv ?? "");
```
and
```
      const d = av - (bv ?? av);
```
What is wrong: both `??` fallbacks are dead — execution reaches these lines only when `av !== undefined && bv !== undefined` (knownFirst + guard at L157/L170 already returned otherwise). Harmless defensive residue. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SORT_KEYS / DEFAULT_SORT_KEY / sortLabelKey | 20-46 | REAL | — | 16 keys, frame order |
| PlaylistEntryMeta / ResolveEntryMeta | 53-65 | REAL seam, unwired | S3 | F-001 |
| entryDisplayTitle | 68-73 | REAL | — | title→basename→unknown; used by PlaylistPanel |
| formatDurationClock | 76-83 | REAL | — | h:mm:ss, guards NaN/negative |
| sortUsesMeta | 89-95 | REAL | S3 | the disclosure predicate |
| knownFirst | 104-109 | REAL | — | unknown sorts last |
| titleOf / uriOf | 113-115 | REAL | — | |
| compareDecorated | 119-175 | REAL | S4 | all 16 keys correct; dead ?? branches (F-002) |
| sortPlaylistEntries | 182-198 | REAL | — | pure, stable, presentation-only |

## 5. Test honesty
No tests in-file. The sorting menu/provider tests (sibling files) assert degradation behavior (unknown never reorders) — those would fail if comparators lied. Baseline vitest covers them. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every comparator degrades HONESTLY: unknown metadata never reorders the queue" (L7-9) | header | REAL | verified by read of all comparators |
| "The engine stays the source of truth — this is presentation order only; playlist.move dispatches are NOT fired" (L179-180) | doc | REAL | no dispatch anywhere in the module |
| "the hint is real, not aspirational" (L87-88) | comment | REAL | metaSeamWired false in provider (sibling audit) |

## 7. Dead / duplicated / unreachable code
Dead `??` fallbacks (F-002). The 10 meta-dependent keys are inert-until-wired (F-001) — reachable UI, unwireable backend.

## 8. Dependencies & flags
components/list/naturalCompare, PlaylistProvider types, queue.sourceLocation. Pure module — no React, no transport. No feature flags.

## 9. Open questions for the next agent
- PlaylistOptionsProvider owner: confirm metaSeamWired stays false in the shipped build (my sibling audit checks it) — if anyone flips it without a real resolver, the "honest" hint machinery silently lies.
