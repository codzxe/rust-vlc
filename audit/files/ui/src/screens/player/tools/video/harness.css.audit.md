# AUDIT: ui/src/screens/player/tools/video/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 48 / 6 selectors |
| File verdict | REAL (dev-only harness styles) |
| Reachable from shipped entry point? | NO by design — imported only by harness.tsx (sole consumer harness.test.tsx, verified in that audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; consumer chain from harness.tsx audit |

## 1. One-paragraph verdict
Dev-only presentation scaffolding for the video-tools vision loop. The one
product-leaking rule is the global `body { margin: 0 }` (line 10) — in a
standalone harness render that is the point, and the module never loads on
app routes, so no shipped surface is affected. Notable honesty: the canvas
background is deliberately stepped (surface-deep) so the collapsed card
state remains visible — a documented harness finding, not a styling whim.

## 2. What actually works (verified)
- Import chain: harness.tsx:18 -> harness.test.tsx only (verified in the
  harness.tsx audit); "Not part of the product CSS" claim holds.
- Frame arithmetic documented per rule (400/20 insets, 406/22 HDR).

## 3. Findings
None. (The global `body` selector would be an S3 in a product sheet; here it
is scoped by the module's dev-only loading.)

## 4. Per-function table (<100 LOC)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| body reset | 10-12 | REAL | harness-only |
| .w26c4-stage (+--hdr) | 14-30 | REAL | canvas sizing |
| .w26c4-caption | 32-36 | REAL | |
| .w26c4-frame | 38-42 | REAL | |
| .w26c4-hdr-open | 44-48 | REAL | |

## 5. Test honesty
N/A (styles for the vision loop).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never imported by app routes" | harness.css:1 | REAL | verified via harness.tsx consumer chain |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Design tokens (imported transitively by the tools). Dev-only.

## 9. Open questions for the next agent
- None.
