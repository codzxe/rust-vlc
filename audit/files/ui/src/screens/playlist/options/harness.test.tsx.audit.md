# AUDIT: ui/src/screens/playlist/options/harness.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 84 / 2 `it` blocks (dev-only harness) |
| File verdict | SHALLOW by design (dev-only vision-harness smoke + env-gated HTML export) — honestly labeled, executed green in baseline |
| Reachable from shipped entry point? | N/A — dev-only harness; the exported HTML feeds the contract §5 visual comparison loop (headless screenshotting), not any shipped path |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
A dev-only harness test with two jobs: (1) "renders all frame states
without crashing" — a smoke test whose assertion is only
`innerHTML.length > 0` (weak; it would pass for a garbage render as long
as something mounts); (2) an env-gated standalone-HTML exporter
(`W26B10_HARNESS_OUT`) that inlines the real CSS sheets and writes three
snapshot files for headless screenshotting. The file is honest about being
dev-only, and the export path does real work (menu-open clicks before
serialization). This is tooling, not a product-contract test.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:152 (verbatim):
```
 ✓ src/screens/playlist/options/harness.test.tsx (2 tests) 58ms
```
- Exporter opens anchored menus before serializing (62-67):
```
      if (open !== undefined) {
        // Open the requested anchored menus before serializing.
        for (const label of open) fireEvent.click(screen.getByRole("button", { name: label }));
      }
```
- Missing-CSS is surfaced, not swallowed (57-59): `existsSync(path) ? readFileSync(path, "utf8") : \`/* missing: ${file} */\``.

## 3. Findings
F-001 [S4] — smoke assertion proves almost nothing
Where: ui/src/screens/playlist/options/harness.test.tsx:43-51
Code:
```
  it("renders all frame states without crashing", async () => {
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
```
What is wrong: the only assertion is non-empty HTML — a harness that
renders a single error string passes. Name promises "all frame states";
body proves "something mounted".
Why it matters: limited blast radius (dev-only harness; the real gate for
these surfaces is the visual baseline lane + the component's own tests),
but the test's green checkmark overstates coverage.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| CSS_FILES list | 18-27 | REAL | 8 real sheets |
| HARNESS_STATES | 36-40 | REAL | 3 states with menu-open triggers |
| crash-smoke test | 43-51 | SHALLOW | F-001 |
| HTML export test | 53-83 | REAL | env-gated; writes 3 files, existence-asserted |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crash smoke | barely | only on hard crash/empty render (F-001) |
| export | YES (when env set) | file existence asserted; skipped in normal runs by design |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "with W26B10_HARNESS_OUT set it additionally writes standalone HTML snapshots ... for headless screenshotting in the contract §5 comparison loop (see the lane log)" | header :1-5 | REAL-UNPROVEN | env-gate verified in code; the comparison-loop lane log not located by me (UNVERIFIED — likely docs/waves/w26/*) |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
vitest, node:fs/path. Dev-only env gate W26B10_HARNESS_OUT.

## 9. Open questions
- Which lane log records the §5 comparison loop consuming these exports?
  Next agent: rg W26B10_HARNESS_OUT in docs/waves/w26.
