# AUDIT: ui/src/screens/media-info/media-info.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 191 / 17 class selectors |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: media-info panel (PlayerScreen.tsx:1544-1547 media-info region) + /media-info route (App.tsx:320) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python class cross-reference (exit 0) |

## 1. One-paragraph verdict
Clean token-driven styling for the media-info panel and route: all 17 class tokens are live in TSX (python cross-reference, zero unused), no raw hex, no `!important`, tabular numerals on values, and an honest-unknown ink treatment (`[data-unknown]` renders tertiary so an absent value never reads as reported). The two usage-side overrides (SectionHeader 15px pin, light-mode card step) are each justified against a named primitive-lane drift with the pending token named. Zero findings.

## 2. What actually works (verified)
— python cross-reference: 17/17 classes used.
— `[data-unknown]` selector matches the panel's honest-unknown contract (L94-97); test contract selectors (`data-mi-section`, `data-mi-stream`, etc.) are set by MediaInfoPanel, not this sheet.

## 3. Findings
None. Checked: selector liveness (17/17), token discipline (zero raw values), light-mode override correctness (`--color-surface-panel-raised` with fallback), grid responsiveness (auto-fit minmax, no media query), hairline rhythm consistency.

## 4. Per-function table (selector groups)
| Selector group | Lines | Verdict | Notes |
|---|---|---|---|
| `.lumen-mi` root + SectionHeader 15px pin | 19-33 | REAL | documented primitive drift |
| `__title/__section/__rows/__row/__term/__value` + `[data-unknown]` | 35-97 | REAL | honest-unknown ink |
| `__stream*` sub-blocks | 99-128 | REAL | hairline separators |
| `__note/__state-card` honest states | 130-150 | REAL | empty/not-exposed share card grammar |
| `.lumen-mi-screen*` route page | 152-180 | REAL | 680px column |
| light-mode card step | 182-191 | REAL | fallback token chain correct |

## 5. Test honesty
MediaInfoPanel.test.tsx pins DOM contracts (baseline ui-test.txt pass); style claims UNVERIFIED visually (no renderer).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (§3.2)" | L16-17 | TRUE | read: zero raw values |
| "an absent value never reads as a reported one" | L92-93 | TRUE | L94-97 tertiary + regular weight |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Tokens; SectionHeader/Card/ListEmpty primitives; pending `--color-surface-panel-raised` token (theme lane) with safe fallback.

## 9. Open questions for the next agent
- None for this file.
