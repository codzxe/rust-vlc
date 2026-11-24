# AUDIT: ui/src/screens/player/dialogs/harness.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 65 / 2 (1 env-gated, explicitly skipped) |
| File verdict | REAL — dev-only vision-harness smoke + an env-gated HTML export whose skip is EXPLICIT in the runner output |
| Reachable from shipped entry point? | NO — vitest dev tooling; baseline ui-test.txt:156 `✓ …harness.test.tsx (2 tests | 1 skipped) 341ms` — this file IS the repo's single skipped test (baseline L179 `Tests 1705 passed | 1 skipped`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
A small, honest dev-tooling test: the always-run test mounts the real
ToolsDialogHarness on both tabs and asserts non-empty DOM + the 2-tab pair
(a smoke test named exactly as such); the export test is gated on
W26C5_HARNESS_OUT via `it.skipIf` — the skip is visible in the runner output
instead of an always-green no-op, and the baseline proves it: the repo's one
skipped test is exactly this one. The one hazard: missing CSS files in the
export path are silently substituted with a `/* missing: … */` comment, so a
renamed stylesheet degrades the exported screenshot HTML without any test
failure.

## 2. What actually works (verified)
— Baseline ui-test.txt:156 `✓ src/screens/player/dialogs/harness.test.tsx
(2 tests | 1 skipped) 341ms` and L179 `Tests 1705 passed | 1 skipped (1706)`
— the env-gated export is the single skipped test, exactly as designed.
— Mutation reasoning: a harness that crashes on mount fails L36; a tab-pair
regression fails L38.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/dialogs/harness.test.tsx:51-54
Code:
    const css = CSS_FILES.map((file) => {
      const path = resolve(uiRoot, file);
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
    }).join("\n");
What is wrong: a missing stylesheet silently degrades the exported HTML
(the comment leaves a trace but nothing fails) — a renamed CSS file yields
subtly-unstyled screenshot artifacts for the contract §5 comparison loop
rather than a loud error. Dev-only blast radius.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: :33-41 — the always-run test asserts `innerHTML.length > 0` + 2 tabs:
a shallow smoke; acceptable because the name says exactly what it does
("without crashing"), but it would pass through many real rendering
regressions.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| renders both tabs | 33-41 | REAL (smoke) | honest naming; tabs counted |
| env-gated export | 46-64 | REAL (skipped by design) | skipIf explicit; baseline-confirmed; F-001 |

## 5. Test honesty
The smoke test fails only on crash/empty render; the export test is the
env-gated lane and its non-run is REPORTED, not hidden (the in-file comment
L43-45 states the rationale). This is the correct alternative to an
always-passing no-op.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "skipIf makes the env-gated export EXPLICIT in the runner output instead of an always-passing no-op" | file L43-45 | REAL | baseline L156/L179 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
node:fs/path static imports; W26C5_HARNESS_OUT env gate (the only env
dependency); CSS_FILES list of 14 real sheets.

## 9. Open questions for the next agent
- Do the three sibling harness tests (video/visualizer/dynamics,
  ui-test.txt:110/120/130 — all green, none skipped) share this pattern?
  (Their env gates presumably never fire in CI — same explicit-skip posture.)
