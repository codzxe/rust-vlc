# AUDIT: ui/src/test/setup.ts
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A06) |
| Date | 2026-11-23 |
| Lines / functions | 7 / 0 (test bootstrap) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — test infrastructure; wired via vite.config.ts:38 `setupFiles: ["./src/test/setup.ts"]` into every vitest run (baseline executed it: 136 files / 1705 tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sed vite.config.ts:30-46 (no `globals: true`); rg jest-dom/RTL deps (package.json:42-43); baseline ui-test.txt citation |

## 1. One-paragraph verdict
Correct minimal test bootstrap. Claim audit: vite.config.ts's `test` block sets
environment/setupFiles/include but NOT `globals: true` — so vitest globals
default to false, and the comment's premise (RTL auto-cleanup never registers
without the global afterEach) is TRUE; the explicit `afterEach(cleanup)` at
line 7 is the standard, correct remedy. The jest-dom/vitest side-effect import
(line 2) is proven live: tests assert `toBeInTheDocument()` (e.g.
ValueChip.test.tsx:12) and passed in the baseline run
(audit/runs/baseline/ui-test.txt — 136 files, 1705 passed | 1 skipped), which
would fail with "matcher not found" if this import were broken. Without the
cleanup line, RTL DOM would leak across tests in the same file and
getByText/getByTestId queries would collide — the 1705-test pass is evidence
it works.

## 2. What actually works (verified)
— verified by `sed -n '30,46p' vite.config.ts` -> no globals flag (default
  false); setupFiles points at this exact file.
— verified by `rg toBeInTheDocument` across src tests + baseline ui-test.txt
  pass -> jest-dom matchers are live in the real run.

## 3. Findings
None. Checked: setup file registered (vite.config.ts:38), globals premise
(true — no globals:true anywhere in the config), double-cleanup risk (none —
RTL does not auto-register when globals are off), stale import (none — both
deps pinned in ui/package.json:42-43, RTL 16.3.3 / jest-dom 7.0.1).

## 4. Per-function table
Not applicable — 7-line bootstrap, no functions.

## 5. Test honesty
This file IS test wiring; its honesty is proven by the baseline run executing
the whole suite through it (1705 passed, 1 skipped, 0 failed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "vitest runs with `globals: false`, so RTL's auto-cleanup ... never registers — do it explicitly" | setup.ts:5-6 | REAL | vite.config.ts test block has no `globals` key (default false); explicit afterEach(cleanup) at :7 |

## 7. Dead / duplicated / unreachable code
None. UNVERIFIED detail: RTL 16.3.3's internal auto-cleanup registration
source could not be inspected (node_modules absent in sandbox) — behavioral
evidence (baseline pass) substitutes.

## 8. Dependencies & flags
@testing-library/react 16.3.3, @testing-library/jest-dom 7.0.1 (both pinned
in ui/package.json). No feature gates.

## 9. Open questions for the next agent
None.
