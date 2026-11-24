# AUDIT: ui/src/screens/player/tools/visualizer/harness.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 130 / 2 describe blocks, 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:120 "visualizer/harness.test.tsx (5 tests)" ✓ PASS). The harness itself is dev-only by design. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg suite in baseline ui-test.txt |

## 1. One-paragraph verdict
A dev-harness crash test plus CSS contract pins: the first test honestly smoke-tests that both harness compositions render (innerHTML non-empty — an anti-crash check, not a behavior claim); the second is an env-gated HTML-snapshot exporter used by the w26 vision-lane screenshot loop (silently a no-op in the normal lane — by design, flagged below); the last three tests READ the committed visualizer.css and pin the frame-measured geometry (chevron -7.5px inset), the light-theme token remap (with an explicit negative assertion that no light ink override exists), and the shared-Dropdown composition (negative assertion on the dead lane-scoped grammar).

## 2. What actually works (verified)
- Baseline ui-test.txt:120: `✓ src/screens/player/tools/visualizer/harness.test.tsx (5 tests) 51ms`.
## 3. Findings
F-001 [S4]
Where: harness.test.tsx:53-55
Code:
```
  it("exports standalone HTML snapshots when W26C3_HARNESS_OUT is set", () => {
    const out = process.env.W26C3_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
```
What is wrong: in the default CI lane this test executes ZERO assertions (early return) — a vacuous green counted in the suite's pass total. The pattern is the documented w26.c1/c2 dev-lane convention (the export only matters for the screenshot loop), but a default-lane run reports "5 passed" while one test proved nothing. What correct looks like: `it.skipIf(!env)` so vitest reports it as skipped, or assert the early-return path explicitly. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: harness.test.tsx:45-51
Code:
```
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
```
What is wrong: smoke-level only ("renders without crashing" — no structure, no semantics asserted). Acceptable and honestly scoped for a dev harness; noted so nobody mistakes it for behavioral coverage. Confidence: HIGH — Verification: read.
## 4. Per-function table (test blocks)
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| renders both compositions | 45-51 | REAL (smoke) | F-002 |
| env-gated HTML export | 53-85 | REAL (dev-lane) | F-001; inlines 6 real CSS files; opens the second selector; writes 2 snapshots; asserts existence only when env set |
| chevron frame inset pin | 96-102 | REAL | regex over committed CSS, exact -7.5px |
| light-theme remap pins | 104-121 | REAL | positive remap rules + negative no-ink-override |
| shared Dropdown composition | 123-129 | REAL | positive composed selectors + negative dead-grammar absence |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crash smoke | YES only for crashes | render throws on a broken component |
| HTML export | NO in default lane | F-001 |
| CSS pins | YES | exact selectors/values; a geometry or remap regression fails |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "see the lane log for the loop record" | :5-6 | UNVERIFIED | the w26 vision-lane log not located in this pass (would live in docs/waves/w26/) |
## 7. Dead / duplicated / unreachable code
None beyond the env-gated branch.
## 8. Dependencies & flags
@testing-library/react, node:fs. Env knob W26C3_HARNESS_OUT (dev only). No feature flags.
## 9. Open questions for the next agent
- Convert the env-gated test to skipIf so the suite's pass count stays honest (F-001).
