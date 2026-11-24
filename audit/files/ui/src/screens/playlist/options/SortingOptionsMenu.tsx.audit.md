# AUDIT: ui/src/screens/playlist/options/SortingOptionsMenu.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 42 / 1 component |
| File verdict | REAL — thin honest composition; the "Needs media metadata" hint is genuinely wired to the provider's truth flag |
| Reachable from shipped entry point? | YES (mounted by the /playlist options bar — PlaylistScreen tree, same provider audited) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read full file; prior Grep/rg evidence for metaSeamWired (provider audit) |

## 1. One-paragraph verdict
Correct 40-line composition: 16 frame-ordered sort rows, each metadata-dependent row carrying the honest "Needs media metadata" hint exactly while `metaSeamWired` is false (provider audit verified it IS false in every production mount until the user hand-marks watched). No defects. The disclosure this menu implements is the only reason 10 inert sort options are not theater. Zero findings.

## 2. What actually works (verified)
— verified by read + provider-audit evidence: hint condition (`sortUsesMeta(key) && !metaSeamWired`, L27) reflects real provider state; selection writes the provider's sortKey (L31-32) which reorders the presentation list (sorting.ts, pure).

## 3. Findings
None. Checked: key typing (template-literal key union), hint gating, default selection (frame row 1 via DEFAULT_SORT_KEY), label resolution seam, no fabricated state.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| SortingOptionsMenu | 18-41 | REAL | thin over PlaylistOptionsMenu |

## 5. Test honesty
Options-lane tests assert hint presence/absence per metaSeamWired (provider harness) — honest disclosure is test-pinned. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "metadata-dependent rows render a quiet honest-degradation hint ... while nothing can influence their order" (L5-8) | header | REAL | provider audit: no production resolveMeta wiring |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
PlaylistOptionsMenu (sibling), sorting.ts, provider, useOptionsMsg seam. No feature flags.

## 9. Open questions for the next agent
- None.
