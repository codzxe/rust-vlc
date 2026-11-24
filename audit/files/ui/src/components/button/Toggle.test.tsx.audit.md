# AUDIT: ui/src/components/button/Toggle.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A01) |
| Date | 2026-06-14 |
| Lines / functions | 184 / 1 ruleOf helper + 2 describe blocks (17 tests incl. 2 snapshots) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:84 `✓ src/components/button/Toggle.test.tsx (17 tests) 171ms`; subject Toggle.tsx (40 L) ships via settings switches (switchCore consumers) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n Toggle audit/runs/baseline/ui-test.txt` -> :84; `wc -l` Toggle.tsx -> 40 L; `ls __snapshots__` -> Toggle.test.tsx.snap exists |

## 1. One-paragraph verdict
Strong behavioral suite for the switch control: real role=switch semantics, click-flip payloads both directions, disabled truthfulness (state visible, no event), ref forwarding, DOM structure, and an exemplary i18n-seam mock — the mocked catalog serves "AN"/"AUS", values that exist nowhere else, so a hardcoded "ON" literal cannot fake a pass (:11-18, :78-85). The second block pins the shared switch skin against button.css. One hygiene defect: those CSS pins are newline-sensitive regexes.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:84 (17/17 pass; lane 1705 passed :178-179). Mutation reasoning: wrong onChange payload fails :30/:37; event firing while disabled fails :57; a bypassed i18n seam fails :82-84 (literals would render "ON"/"OFF"); a drifted shared-skin rule fails :175-182.

## 3. Findings
F-001 [S4]
Where: ui/src/components/button/Toggle.test.tsx:101-106
Code:
```
  it("track: surface + control radius (68x32 rx6, #090909)", () => {
    const rule = ruleOf(/\.lumen-toggle,\n\.lumen-switch\s*\{([^}]*)\}/);
    expect(rule).toContain("background: var(--color-surface)");
```
What is wrong: the skin pins match raw CSS with `\n` literal newlines in the selector lists (`.lumen-toggle,\n\.lumen-switch`) — a formatter (prettier-style CSS formatting, selector reflow) breaks every pin at once without any semantic change. Why it matters: six tests' worth of shared-skin regression protection (j01-3/j01-9) evaporates under a whitespace-only reformat; conversely the pins cannot catch a semantic change expressed with different whitespace. What correct looks like: normalize whitespace before matching (e.g. `buttonCss.replace(/\s+/g," ")`) or assert computed styles. Confidence: HIGH — Verification: read (passing baseline proves the current formatting matches; fragility is structural).

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| i18n seam mock | 11-18 | REAL | — | unforgeable catalog values AN/AUS |
| semantics suite | 20-93 | REAL | — | 9 tests: checked state, flip payloads, keyboard, disabled, ref, structure, labels, seam, snapshots |
| skin pin suite | 95-184 | REAL | S4 | F-001; ruleOf helper resolves selector-list ambiguity |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| flip payloads :26-38 | YES | exact boolean args asserted |
| disabled no-change :48-58 | YES | spy not called |
| seam usage :78-85 | YES | mocked values unforgeable elsewhere |
| shared skin :174-183 | YES (text drift) | newline-anchored regex pins |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 17 tests run | baseline ui-test.txt:84 | REAL | `✓ ... (17 tests) 171ms` |

## 7. Dead / duplicated / unreachable code
None. Snapshot file exists and is exercised (:87-92).

## 8. Dependencies & flags
Imports resolve (Toggle, w26.k01 lane keys used as the not-rendered fallback check :84).

## 9. Open questions for the next agent
None.
