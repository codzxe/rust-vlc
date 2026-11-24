# AUDIT: ui/src/screens/network-stream/useNetworkStreamMsg.ts (compact, 31 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 31 / 1 (useNetworkStreamMsg) |
| File verdict | REAL (same verified seam pattern as usePlaylistMsg, single lane set) |
| Reachable from shipped entry point? | YES — NetworkStreamScreen.tsx:50 imports, :92 `const m = useNetworkStreamMsg()` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n useNetworkStreamMsg ui/src/screens/network-stream/` -> :50,:92; read of full file |

## 1. One-paragraph verdict
Identical mechanics to usePlaylistMsg (audited directly above): catalog
first, w26.b15 lane keys as fallback, union-typed keys, memoized resolver.
The same S4 applies — the "both paths return the same string" post-merge
promise is unenforced, so the lane copy can silently rot after
registration. Consumer confirmed live (NetworkStreamScreen:92).

## 2. What actually works (verified)
— Consumer chain (above). Lane key file w26.b15.ts exists.
— No literals: the eslint local/i18n-no-literals rule enforces the class.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/network-stream/useNetworkStreamMsg.ts:21-23
Code:
 * useNetworkStreamMsg — `useMsg` extended with this lane's pre-registration
 * keys. Lane keys win while they are unregistered; once the Lead merges them
 * into the catalog both paths return the same string.
What is wrong: same as usePlaylistMsg F-001 — no drift check between lane
copy and catalog copy after registration. Confidence: HIGH —
Verification: read.

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| useNetworkStreamMsg | 25-31 | REAL | S4 (F-001) |

## 5. Test honesty
Exercised via NetworkStreamScreen suites (baseline green); no seam-specific
test (same as the sibling).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Same pattern as the playlist/discovery/settings lanes" | :8-9 | REAL | pattern verified in 3 sibling files this wave |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
i18n/messages, keys/w26.b15. No flags.

## 9. Open questions for the next agent
- NetworkStreamScreen auditor: the dialog's frame copy promises multiple
  links — verify the open path actually enqueues multiple sources
  (playlist.enqueue-many) rather than only the first line.
