# AUDIT: ui/src/screens/player/tools/dynamics/harness.css (quick check)
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 46 / 4 rule blocks (stage, cell, caption, frame) |
| File verdict | REAL (dev-only vision harness skin — never in the app bundle) |
| Reachable from shipped entry point? | NO — imported only by dynamics/harness.tsx, which is rendered only by harness.test.tsx (W26C2_HARNESS_OUT static-HTML export); no app route imports it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read all 46 lines; Grep `w26c2-` → harness.tsx:26/29/30/38 consumes all four classes; Grep harness importers outside tools/ → only *.test.tsx files; tokens (--color-surface-deep, --font-family-sans, --font-size-caption, --color-content-secondary) verified present in tokens.css (read this session). Baseline: ui build #22 OK; harness.test green in ui-vitest. |

## 1. One-paragraph verdict
The cleanest of the harness skins: four flex/size rules, every dimension frame-measured and consistent (400px cells with 20px frame padding → the 360px tool-card width the header promises, so slider rows measure 328px), surface choice documented with the reason (between app surface and panel so both card states stay evaluable), tokens only, and — unlike the visualizer twin — no unscoped element selectors. Dev-only by import graph, verified.

## 2. What actually works (verified)
— verified by Grep: 4/4 w26c2-* classes consumed by harness.tsx; zero app-route importers of the harness module.
— verified by arithmetic: cell 400px − frame padding 2×20px = 360px card width ✓ (the header's "cards land at 360px wide" claim); stage padding/gap 40px are stage-level, distinct from the documented 20px gutters.

## 3. Findings
None. Checked and clean: selector usage (4/4), token existence, no raw colors, no unscoped selectors, no light-mode concerns (dev harness renders dark surface only, via token).

## 4. Per-function table
| Rule block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.w26c2-stage` | 6-21 | REAL | — | Flex stage, 40px pad/gap, surface-deep with documented rationale. |
| `.w26c2-cell` | 23-28 | REAL | — | 400px column cell. |
| `.w26c2-caption` | 30-34 | REAL | — | Caption scale + secondary ink. |
| `.w26c2-frame` | 38-45 | REAL | — | 400px, 20px padding, 20px state gap. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx (static-HTML vision export) | Indirectly | Structure only; dimensions evaluated in the export. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness styles … never imported by app routes" | harness.css:1-2 | REAL | Import graph verified. |
| "a 400px cell per tool with 20px gutters (cards land at 360px wide … slider rows measure 328px exactly)" | harness.css:2-4 | REAL | 400 − 2×20 = 360 ✓. |

## 7. Dead / duplicated / unreachable code
None dead; unreachable from the shipped app by design.

## 8. Dependencies & flags
Semantic tokens only; no flags.

## 9. Open questions for the next agent
None.
