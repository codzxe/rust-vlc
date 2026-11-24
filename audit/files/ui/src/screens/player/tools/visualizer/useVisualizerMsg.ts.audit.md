# AUDIT: ui/src/screens/player/tools/visualizer/useVisualizerMsg.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 34 / 1 (useVisualizerMsg) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — VisualizerTool/ModeSelect/Display (player tools dialog) resolve all copy through this seam |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | node copy-equality check of both lane sources (w26.c3: 10/10, w26.a18: 3/3 — all registered, identical); rg consumers |

## 1. One-paragraph verdict
The visualizer's i18n seam: useMsg widened with two lane files, catalog-first
with a `?? key` last resort (the good pattern shared.tsx lacks). Both lane
sources are fully registered with identical copy (verified: w26.c3 10/10,
w26.a18 3/3), so the fallback is dormant-but-consistent. No findings.

## 2. What actually works (verified)
- node equality: 13/13 lane keys registered in messages.ts, byte-identical.
- Fallback chain total: m(key) ?? laneCatalog[key] ?? key — a visible string
  even if both sources go missing (better than the areas' shared seam).
- All three visualizer components consume the seam (rg).

## 3. Findings
None. Checked: divergence (none), ordering (catalog-first as documented),
last-resort (present), literals (none — comment claim consistent with the
resolver design).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| laneCatalog | 16-19 | REAL | c3 over a18 (a18 keys shadowed correctly — disjoint sets verified by registration scan) |
| useVisualizerMsg | 27-34 | REAL | total fallback chain |

## 5. Test honesty
VisualizerTool.test.tsx consumes the components (icon audit's consumer scan);
copy rendering is pinned by the tool tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every user-visible visualizer string flows through here — no literals" | useVisualizerMsg.ts:5-6 | REAL-UNPROVEN | the three consumer components resolve via this seam (rg); exhaustive literal scan of those files not done (their audit owner's scope) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
i18n messages + two lane key files. None.

## 9. Open questions for the next agent
- None.
