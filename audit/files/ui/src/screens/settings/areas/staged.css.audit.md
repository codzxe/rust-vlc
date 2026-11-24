# AUDIT: ui/src/screens/settings/areas/staged.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 83 / 8 selector groups |
| File verdict | REAL (the "honest staged" grammar for schema-blocked settings areas; consumers verified carrying the markers) |
| Reachable from shipped entry point? | YES — imported by library.tsx, recording.tsx, privacy-network.tsx, staged.tsx; those render the /settings library/network/recording areas (data-testid markers verified in all three). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg data-staged / data-advanced-staged / data-staged-dimmed in the four consumer areas (all present); rg companion rules in settings.css (:564); var sweep earlier (3/3 resolve) |

## 1. One-paragraph verdict
Small, coherent sheet implementing the "staged surface" honesty pattern: schema-seed settings areas (library, network, recording — engine surface absent) render their frame rows full-strength but inert (`data-staged` un-dims disabled controls so they don't look broken), dim the frame's own disabled groups via `--opacity-muted`, pin the staged Advanced bar's collapsed looks, and carry meticulous per-section frame arithmetic (8.5/10.5px title margins to hit frame y-positions, with the ±2 SVG variance explicitly called out as not-a-bug). All selectors resolve against real markers in the consumer areas. Zero findings.

## 2. What actually works (verified)
— verified by rg: `data-advanced-staged="true"` present in library.tsx:51, recording.tsx:61, privacy-network.tsx:47; `data-staged`/`data-staged-dimmed` documented and used in all three areas; area testids match the selectors.

## 3. Findings
None. Checked: selector/marker agreement (all 8 groups), var resolution (3/3), the two 12px row-pitch rules' frame arithmetic (36+12=48 documented), no dead rules.

## 4. Per-function table
n/a (<100 LOC).

## 5. Test honesty
library/recording/areas tests (baseline PASS) assert the staged areas render with embedded markers (e.g. recording.test.tsx:59, library.test.tsx:50).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the honest-empty grammar for the schema-blocked settings areas" | staged.css:1-2 | TRUE | consumers carry data-staged="schema-seed" + honest tooltip (library.tsx:8, recording.tsx:9, privacy-network.tsx:8) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
settings.css grammar, token set. No flags.

## 9. Open questions for the next agent
- Which schema keys back these areas, and does the engine actually serve them? If the engine DOES serve config.* for library/network/recording keys, the staging is over-conservative; if not, these three settings areas are honest-unwired surfaces (master-question data point).
