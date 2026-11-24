# AUDIT: ui/src/components/form/ValueChip.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A01) |
| Date | 2026-11-24 |
| Lines / functions | 33 / 4 tests (2 behavioral + 1 snapshot + 1 CSS-skin) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:176 (4 tests, 38ms); subject rendered in tools/equalizer/GraphicEQ.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | it-count 4 == baseline; consumer rg |

## 1. One-paragraph verdict
Tiny and appropriate to a presentational chip: content-as-accessible-text, composition class
merge, the form.css skin pin (deep well + control radius + bright tabular-nums value). 4/4
pass in the baseline. Same family soft spot: one self-generated DOM-snapshot golden. The
component is genuinely consumed by the EQ readout rows.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:176 `✓ src/components/form/
ValueChip.test.tsx (4 tests) 38ms` (1705-passed run); 4 it() blocks on disk match; skin test
reads the real form.css (:7).

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/ValueChip.test.tsx:20-23
Code:
  it("DOM snapshot", () => {
    const { container } = render(<ValueChip value="0.0 dB" />);
    expect(container.firstChild).toMatchSnapshot();
  });
What is wrong: self-generated golden — change-detector only (checklist 6.4), duplicating the
two named assertions. Confidence: HIGH — Verification: read.
No S0-S3: the other assertions bind to rendered text/query results or the committed CSS.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| value as content (10) | REAL | — | |
| composition class (15) | REAL | — | merged selector |
| DOM snapshot (20) | REAL | S4 | F-001 |
| skin: deep well + tabular (27) | REAL | — | form.css regex |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 3 of 4 | YES | text/class/CSS-token drift all fail |
| DOM snapshot (20) | only on change | self-generated golden |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "EQ value readout" role | header | REAL | consumer GraphicEQ.tsx |

## 7. Dead / duplicated / unreachable code
F-001's golden only.

## 8. Dependencies & flags
In-repo + node:fs CSS read (dev-only).

## 9. Open questions for the next agent
None — smallest file of the form family, coverage matches its surface.
