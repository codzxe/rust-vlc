# AUDIT: ui/src/components/form/ValueChip.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 21 / 1 (ValueChip) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — EqualizerTool/GraphicEQ (player tools dialog) + form barrel + own test + snapshot |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers |

## 1. One-paragraph verdict
A 3-line presentational span. Correct a11y reasoning documented (row supplies
label context; the value is the accessible content). No findings.

## 2. What actually works (verified)
- 6 consumers including production EQ tools.
- Class filter(Boolean).join pattern handles empty className.

## 3. Findings
None. Checked: dead code (none), styling ownership (consumer CSS owns
layout, chip owns the well via form.css), duplication (the tools lane's
ToolsValueChip is the dialog-family twin — separate grammar, documented
there).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| ValueChip | 17-21 | REAL | |

## 5. Test honesty
ValueChip.test.tsx + BrowseField/SegmentedControl snapshots exist in the
form family (the snapshot files are separate assignments).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A plain span — the surrounding row supplies the label context" | ValueChip.tsx:14-15 | REAL | read |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
form.css. None.

## 9. Open questions for the next agent
- None.
