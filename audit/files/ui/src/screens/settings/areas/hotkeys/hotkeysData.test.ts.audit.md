# AUDIT: ui/src/screens/settings/areas/hotkeys/hotkeysData.test.ts
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 47 / 1 runCheck helper + 1 describe block (2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:36 `✓ src/screens/settings/areas/hotkeys/hotkeysData.test.ts (2 tests) 55ms`; subject generated/bindings.ts ships in the ControlsArea hotkeys fold |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | baseline grep -> :36; read-through; the drift test itself executes `node scripts/gen-hotkeys-data.mjs --check` in-suite (child_process execFileSync, :20) — GREEN per baseline |

## 1. One-paragraph verdict
A real drift gate inside vitest: test 1 shells out to the committed generator script with `--check` against keymaps/default.toml (execFileSync, exit status propagated through the catch, :15-28) — this is the strongest form of generated-data honesty (regenerate-on-drift, executable in CI); test 2 pins the data's real shape (>100 bindings, the eight section names, a real command id, mode variants, macos platform overrides).

## 2. What actually works (verified)
— verified by baseline ui-test.txt:36 (2/2 pass; lane 1705 passed :178-179). Mutation reasoning: drifting generated/bindings.ts from keymaps/default.toml fails test 1 (the --check exit propagates); gutting the generator's section structure fails test 2's arrayContaining.

## 3. Findings
None. Checked for: tautologies (none — the shell-out result is a real exit status), swallowed errors (the catch converts to a status number which IS the assertion, :25-27 — correct pattern, not a swallow), unwired tests (2/2 run). Zero findings is the honest result.

## 4. Per-function table
File <100 LOC — runCheck (:15-28, execFileSync + status propagation) + 2 tests (:30-46). Both REAL.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| drift gate :31-35 | YES | real child-process --check exit asserted 0 |
| shape :37-46 | YES | structural pins on the real generated data |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 2 tests run | baseline ui-test.txt:36 | REAL | `✓ ... (2 tests) 55ms` |
| "regenerate: `npm run hotkeys:generate`" | header :1-2 | REAL-UNPROVEN | script referenced exists (scripts/gen-hotkeys-data.mjs invoked at :20); regen path itself not executed this run |

## 7. Dead / duplicated / unreachable code
None. The local NodeChildProcess/NodeProcess interfaces (:7-13) are minimal typing shims for the dynamic import — used.

## 8. Dependencies & flags
generated/bindings module (real, consumed by ControlsArea tests too); node:child_process dynamic import.

## 9. Open questions for the next agent
None.
