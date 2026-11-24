# AUDIT: ui/src/components/form/RadioGroup.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A10) |
| Date | 2026-11-24 |
| Lines / functions | 102 / 10 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:136 (10 tests, 163ms); subject mounted in ControlBar/SettingsPanel/SettingField |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | it-count 10 == baseline; consumer rg |

## 1. One-paragraph verdict
Solid native-radio suite: radiogroup semantics with checked state, click onChange, roving
tabindex, horizontal + vertical arrows with wrap, group-level native disabled, the
discriminating per-option disabled case (ArrowUp from the enabled-set's index 0 wraps to
"system", proving the disabled "light" is skipped — not-calledWith guard), id host wiring,
invalid aria. 10/10 pass in the baseline. Same single family defect as its siblings: one
self-generated DOM-snapshot golden.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:136 `✓ src/components/form/RadioGroup.test.tsx
(10 tests) 163ms` (1705-passed run); 10 it() blocks on disk match.

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/RadioGroup.test.tsx:98-101
Code:
  it("DOM snapshot", () => {
    const { container } = render(<RadioGroup label="Theme" options={options} value="dark" onChange={() => {}} />);
    expect(container.firstChild).toMatchSnapshot();
  });
What is wrong: self-generated golden — change-detector only; duplicates the named coverage of
the other nine tests. What correct looks like: drop it or label it a drift tripwire.
Confidence: HIGH — Verification: read.
No S0-S3: all other assertions bind to roles/labels/attributes/onChange payloads.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| radiogroup semantics (12) | REAL | — | checked/not-checked |
| click → onChange (19) | REAL | — | |
| roving tabindex (26) | REAL | — | selected-only tabbable |
| horizontal arrows (33) | REAL | — | |
| wrap-around (40) | REAL | — | |
| group disabled (47) | REAL | — | native disabled attr |
| per-option disabled skip (56) | REAL | — | discriminating wrap case + not-calledWith |
| id host (86) | REAL | — | j01-7 |
| invalid (93) | REAL | — | aria-invalid |
| DOM snapshot (98) | REAL | S4 | F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 9 of 10 | YES | e.g. skip logic broken → :81's not-calledWith("light") fails |
| DOM snapshot (98) | only on change | self-generated golden (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Native disabled on each input is the contract (browser-gated)" | :48-49 | REAL | toBeDisabled on each input |

## 7. Dead / duplicated / unreachable code
F-001's golden only. Subject live in 3+ screens.

## 8. Dependencies & flags
In-repo only. No mocks of the subject.

## 9. Open questions for the next agent
RadioGroup has no CSS-skin half (unlike Segmented/Swatch) — is its styling pinned anywhere
(form.css assertions in the siblings), or unguarded drift?
