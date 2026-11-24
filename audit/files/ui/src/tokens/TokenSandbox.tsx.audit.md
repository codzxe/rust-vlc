# AUDIT: ui/src/tokens/TokenSandbox.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 76 / 1 (TokenSandbox) + 2 const tables |
| File verdict | REAL (dev fixture, honestly labeled) |
| Reachable from shipped entry point? | YES but dev-only — route /dev/tokens (App.tsx:84 "and /dev/tokens (dev fixture) render ungated"); not a product screen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; cross-check ACCENTS import (generated tokens) |

## 1. One-paragraph verdict
A transparent dev fixture: mode/accent switchers driving the real ThemeProvider, six semantic-token swatches resolving through generated tokens.css. Labeled NOT a product screen in its own header; button labels are union values under a documented technical-identifier exemption. Does exactly what it claims — proving the token pipeline end-to-end.

## 2. What actually works (verified)
— verified by read: setModeSetting/setAccent are the provider's real actions; swatches are data-role'd spans styled only by TokenSandbox.css tokens.

## 3. Findings
Zero findings. Checked: no product copy bypass (headings via useMsg), no engine claims, no dead branches.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| MODE_SETTINGS/SWATCH_ROLES | 17-27 | REAL | data |
| TokenSandbox | 29-76 | REAL | fixture render |

## 5. Test honesty
Not test-covered (dev fixture) — acceptable; it exists to be looked at.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NOT a product screen" | header L1 | REAL | route ungated + dev-prefixed |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Generated tokens, useTheme. No env vars.

## 9. Open questions for the next agent
- None.
