# AUDIT: ui/src/components/form/__snapshots__/Checkbox.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A08) |
| Date | 2026-11-24 |
| Lines / functions | 19 / 1 snapshot entry (forwards the ref + DOM snapshot) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — the pinned component is dead product-wise: exhaustive rg shows `Checkbox` is imported ONLY by its own test (form/index.ts:7-8 exports it; every form-barrel consumer imports Switch/TextField/SegmentedControl/NumberField/ValueChip — never Checkbox). The snapshot is reachable only from the vitest run. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (snapshot-producing test Checkbox.test.tsx:34-37, impl useId/htmlFor, all importers incl. form-barrel import statements) -> zero production consumers; sed -n '3,7p;13,17p' snap -> quotes verified; baseline ui-test.txt:174 |

## 1. One-paragraph verdict
The snapshot itself is legitimate and would fail on any DOM regression: it pins the `lumen-choice--checkbox` wrapper grammar, the native checked input, and the useId-driven `label[for]`↔`input[id]` pairing (Checkbox.tsx:19,34). The problem is what it pins: a component nothing in the product mounts. Checkbox is exported through the form barrel, tested (5 tests green), and referenced by no screen, dialog, or tool — a fully-equipped dead island.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:174 `✓ src/components/form/Checkbox.test.tsx (5 tests) 50ms` — snapshot matched live output.
— verified by rg: the pinned attrs are the component's real wiring (Checkbox.tsx:1,19,34 — useId, htmlFor) — not test fixtures.

## 3. Findings
F-001 [S3]
Where: ui/src/components/form/index.ts:7-8 (export) + this snapshot's subject
Code: (verbatim, sed-verified snapshot head)
```
exports[`Checkbox > forwards the ref + DOM snapshot 1`] = `
<span
  class="lumen-choice lumen-choice--checkbox"
>
```
What is wrong: the Checkbox component has zero production consumers — rg over ui/src excluding components/form finds no import and no `<Checkbox` JSX outside Checkbox.test.tsx; the "checkbox" hits elsewhere are the menu grammar's `kind: "checkbox"` (PlaylistPanel.tsx:417, fileRowMenu.ts) and CSS/docs text. Why it matters: a tested, exported, snapshotted component that ships nowhere is dead code dressed as a living primitive — its 5 tests and this snapshot create an illusion of product coverage. What correct looks like: either mount it (a settings row needs it — e.g. the staged effects rows use StagedToggleRow instead) or delete component+tests+snapshot+barrel export. Confidence: HIGH — Verification: read (exhaustive importer scan).
F-002 [S4]
Where: snap lines 10, 14 (`id="_r_4_"` / `for="_r_4_"`)
What is wrong: same React useId-value brittleness as the TextField snapshot — pins React-internal id values that shift on React upgrades while the actual contract is the for↔id pairing. Confidence: HIGH — Verification: read.

## 4. What the snapshot pins (mutation reasoning)
| Component change | Snapshot fails? | Why |
|---|---|---|
| label htmlFor decoupled from input id | YES | both pinned to `_r_4_` |
| `checked` not reflected on the native input | YES | `checked=""` (line 8) |
| choice grammar renamed / modifier dropped | YES | line 5 |
| input type changed | YES | line 11 |
| label text duplicated | YES | single "Snapshot" node (line 16) |
| invalid/disabled styling | NO | attributes absent in this case; covered by tests :23,:28 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Checkbox.test.tsx:34-37 (produces this snap) | YES | full-DOM diff of the real render |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| states.md:39 "sets `aria-invalid` (Checkbox per-input…)" | form/states.md | REAL (code) / UNREACHABLE (product) | invalid prop tested (Checkbox.test.tsx:23) but no product mount exists |

## 7. Dead / duplicated / unreachable code
The entire Checkbox island (component, barrel export, 5 tests, this snapshot) is production-dead — F-001. The snapshot entry itself is live within the test run (no orphans).

## 8. Dependencies & flags
None. Vitest snapshot v1; no env/config keys.

## 9. Open questions for the next agent
Is Checkbox scheduled for a shipped surface (e.g. a future settings row), or is it a Wave-26 leftover? Nothing in the tree answers this — the component's own header makes no staging claim (unlike the honestly-staged tools).
