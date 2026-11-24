# AUDIT: ui/src/screens/playlist/usePlaylistMsg.ts (compact, 37 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 37 / 1 (usePlaylistMsg) |
| File verdict | REAL (working i18n seam hook; consumers confirmed) |
| Reachable from shipped entry point? | YES — NewPlaylistDialog.tsx:27 imports it; the playlist lane screens (PlaylistPanel family) render through it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln usePlaylistMsg ui/src` -> NewPlaylistDialog.tsx:27 + lane key files; read of full file |

## 1. One-paragraph verdict
The standard lane-i18n seam (useMsg first, lane catalog fallback), carrying
two lane key sets (w26.b09 + w26.f06). Mechanics are correct: the union
type prevents unresolvable keys, useCallback memoizes on the catalog
function, and the no-undefined claim holds for typed callers. The one
documentation inaccuracy is shared with its siblings: the comment says
"Lane keys win while they are unregistered" — accurate only in the sense
that unregistered keys fall through; once registered the CATALOG wins and
a drifting lane copy is silently ignored (no drift check between the two
copies).

## 2. What actually works (verified)
— Consumer exists (NewPlaylistDialog.tsx:27; PlaylistPanel.tsx documents
  the same shape). Lane key files exist (w26.b09.ts, w26.f06.ts).
— Type-level exhaustiveness: PlaylistLaneKey is a union of both files'
  keys — typos fail typecheck (build gate, baseline green).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/usePlaylistMsg.ts:27-29
Code:
 * usePlaylistMsg — `useMsg` extended with the playlist lane's
 * pre-registration keys. Lane keys win while they are unregistered; once the
 * Lead merges them into the catalog both paths return the same string.
What is wrong: "both paths return the same string" is an unenforced
promise — after registration, the lane copy can drift from the catalog
copy and nothing (no test, no lint rule found) compares them. The user
always sees the catalog string; the lane file becomes misleading dead
data. What correct looks like: an l10n:check drift assertion lane-file vs
catalog. Confidence: HIGH — Verification: read (rg found no comparison
beyond existence checks).
## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| usePlaylistMsg | 31-37 | REAL | S4 doc/promise gap (F-001) |

## 5. Test honesty
No dedicated test; the seam is exercised by every consumer suite
(playlist lane, vitest baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "none renders `undefined` while unregistered" | :7-8 | REAL | fallback chain + union typing |
| "once the Lead merges them ... both paths return the same string" | :28-29 | REAL-UNPROVEN (unenforced) | F-001 |

## 7. Dead / duplicated / unreachable code
None (post-merge lane files are the redundancy — see F-001).

## 8. Dependencies & flags
i18n/messages (useMsg), two lane key files. No flags.

## 9. Open questions for the next agent
- l10n-check.mjs auditor: does the check script cover lane-key vs catalog
  drift, or only locale completeness? (Determines whether F-001 is already
  mitigated.)
