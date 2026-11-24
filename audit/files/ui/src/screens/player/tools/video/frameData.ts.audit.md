# AUDIT: ui/src/screens/player/tools/video/frameData.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A05) |
| Date | 2026-11-24 |
| Lines / functions | 21 / 2 consts + 1 fn (isHexColor) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — ColorsTool.tsx:42-44,201-202 (initial hex state of both rows + swatch guard :168) → ToolsDialog.tsx:39 → PlayerScreen.tsx:1203 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg (consumers in ColorsTool + tests) -> cited; sed quotes verified; baseline ui-test.txt:77 |

## 1. One-paragraph verdict
Three tiny, correct data exports doing exactly what the comments claim: two frame-pinned seed colors kept as separate constants so the two hex fields stay independently editable (verified: two independent useState seeds, ColorsTool.tsx:201-202), and a `#RRGGBB` validator whose only consumer uses it to keep the swatch fill honest — an invalid hex renders no fill at all, and that behavior has a dedicated test.

## 2. What actually works (verified)
— verified by rg + read: FRAME_EXTRACTION_COLOR → `useState` seed (ColorsTool.tsx:201), FRAME_THRESHOLD_COLOR → second seed (:202) — the "independently editable" rationale (lines 13-15) is real: two constants, two states, no shared reference.
— verified by baseline ui-test.txt:77 (`✓ ColorsTool.test.tsx (16 tests)`), specifically :52-64 pin `#FF0000` input value + swatch fill, :131 hex typing updates both, :141 invalid hex → no swatch fill.

## 3. Findings
None. Checked: regex correctness (`/^#[0-9a-fA-F]{6}$/` — matches exactly `#` + 6 hex digits, rejects 3-digit shorthand, 8-digit alpha, missing `#`, empty string; anchored both ends, no global-flag reuse hazard), swatch safety (invalid → `style={undefined}`, never an invalid backgroundColor), duplicate-literal rationale (documented + verified as two independent states), token-discipline boundary claim (raw hex confined to this data module; ColorsTool applies it only as data-driven inline style with the comment at :162-163 explaining the exemption — consistent with the module header).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FRAME_EXTRACTION_COLOR | 11 | REAL | — | "#FF0000", seeds extraction row |
| FRAME_THRESHOLD_COLOR | 16 | REAL | — | same literal, separate const (independent editability — verified) |
| isHexColor | 19-21 | REAL | — | anchored #RRGGBB check, case-insensitive |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ColorsTool.test.tsx:52-64 (#FF0000 value + swatch) | YES | pins both seed constants' effect on DOM |
| ColorsTool.test.tsx:141 (invalid hex keeps swatch honest) | YES | deleting the isHexColor guard would paint the swatch with garbage → toHaveStyle fails |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "literal contents the frames show in editable fields … data, not theme" | frameData.ts:1-4 | REAL | used only as state seeds + data-driven fill, never as a themed color (ColorsTool.tsx:162-168) |
| "kept a separate constant so the two fields are independently editable" | :13-15 | REAL | two useStates at ColorsTool.tsx:201-202 |
| "`#RRGGBB` (the frames' format — 7 chars, uppercase hex)" | :18 | REAL (lenient) | regex accepts lowercase too — validation is a superset of the documented format; no defect |

## 7. Dead / duplicated / unreachable code
None in this file. (The barrel re-export of these three symbols has no external importer — recorded as F-002 in the video/index.ts audit; direct consumption via deep import is live.)

## 8. Dependencies & flags
None — pure data + one regex. No feature gates.

## 9. Open questions for the next agent
None.
