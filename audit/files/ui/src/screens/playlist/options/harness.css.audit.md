# AUDIT: ui/src/screens/playlist/options/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 52 / 6 blocks (stage, cell, caption, frame, frame--menu/--media, anchor + media height) |
| File verdict | REAL (dev-only harness layout) |
| Reachable from shipped entry point? | NO — imported only by harness.tsx (test-only, per harness.tsx audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; consumer chain established in harness.tsx audit |

## 1. One-paragraph verdict
Dev-only screenshot-stage layout, honestly labeled "Not part of the product CSS", token-only values, frame-measured widths (377px media.svg canvas, 228px row height) each cited. Zero findings.

## 2. What actually works (verified)
— verified by import chain (harness.tsx -> harness.test.tsx only).

## 3. Findings
None.

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| .w26b10-* | 5-52 | REAL (dev) | neutral backdrop, frame-sized cells |

## 5. Test honesty
N/A (layout artifact for the vision loop).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY harness layout ... Not part of the product CSS" (L1-3) | header | REAL | test-only import chain |

## 7. Dead / duplicated / unreachable code
None (dev tool).

## 8. Dependencies & flags
Semantic tokens only.

## 9. Open questions for the next agent
- None.
