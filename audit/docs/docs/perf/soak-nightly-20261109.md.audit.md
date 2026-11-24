# CLAIM AUDIT: docs/perf/soak-nightly-20261109.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R5) |
| Claims extracted | 8 |
| REAL 6 / REAL-UNPROVEN 1 / UNVERIFIED 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The doc's self-description ("plumbing real, 4 h measurement NOT taken") is exactly the state
the tree confirms: the harness, env contract (SOAK_MINUTES=2 default, SOAK_LEAK_MB_PER_H=50
default, SOAK_REPORT), the nightly workflow (cron 0 3 * * *, 300-minute timeout, deliberate
no-continue-on-error, pinned toolchain, if:always artifact upload), the 12-test census, and
the ranked_by_growth assertion machinery all exist and match. No soak-4h artifact exists —
the pending status is honest. The single real run (2-min 12/12) is not re-executable here;
git-commit references are unverifiable under the no-git rule.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Harness = crates/playback-engine/tests/soak/main.rs — env-configured, not hardcoded | :14 | REAL | ls tests/soak -> main.rs + dispatch.rs memory.rs report.rs scenario.rs; main.rs:13-19 env table exactly as documented. |
| 2 | CEILING_MB_PER_H = 50.0 default, overridable via SOAK_LEAK_MB_PER_H; parse errors refuse loudly | :24-30/:17 | REAL | main.rs:81 `const CEILING_MB_PER_H: f64 = 50.0;`, :15 SOAK_LEAK_MB_PER_H default row, :104-111 `Err(format!("SOAK_MINUTES is not a number..."))` + range refusal (sed-verified). |
| 3 | Nightly workflow: cron 0 3 * * * UTC + workflow_dispatch, ubuntu-latest, timeout-minutes 300, no continue-on-error | :16/:32-37 | REAL | .github/workflows/soak-nightly.yml:76 `- cron: "0 3 * * *"`, :77 `workflow_dispatch:`, :102 `runs-on: ubuntu-latest`, :104 `timeout-minutes: 300`, :105 comment "NOT continue-on-error" (rg/sed-verified). |
| 4 | Toolchain pinned like ci.yml (dtolnay d124f86… 1.99.1); artifact pins v7.0.2 etc.; upload with if: always() | :38-45 | REAL | soak-nightly.yml:65-67 pinned actions table incl. `actions/upload-artifact v7.0.2`, :111 `uses: dtolnay/rust-toolchain@d124f86f28ff42855784ff3319f48ac182c248f8` (rg). |
| 5 | 12 tests = soak + config/memory/report/scenario unit tests | :61-62 | REAL | `rg -c "#\[test\]"` across soak modules: main 4 + memory 5 + dispatch 2 + scenario 1 = 12 exactly; main.rs:188 `fn soak_loops_stay_memory_bounded()`. |
| 6 | phases.ranked_by_growth names the top suspect in the assertion message | :85-86 | REAL | report.rs:230/:240 `self.phases.ranked_by_growth()` drives the report + worst-phase assertion (rg). |
| 7 | The REAL 2-min run: "12 passed; 0 failed ... finished in 120.03s" | :55-59 | REAL-UNPROVEN | point-in-time run not repeatable under load discipline (a 120 s release test would violate the budget); the 12-test census (#5) and defaults (2 min) make the log internally consistent. |
| 8 | 4 h baseline PENDING; first scheduled run produces the artifact; blocked-run record via `git show 1ff2c70^` | :19/:69-70/:91 | REAL (pending) / UNVERIFIED (git) | `ls docs/perf | grep soak` -> only soak-nightly-20261109.md (this file) — no soak-4h artifact exists, exactly as the doc states; the git-show citation cannot be checked (no-git rule). |

## 3. Contradictions
None. The doc aggressively de-values its own evidence ("a 2 min window extrapolates ~24×",
"every 4 h number in any doc would be fiction") — and the tree contains no such fiction.

## 4. Fabrication / inflation indicators
None — the entire doc is an anti-fabrication instrument (marking the 5 MB/h ceiling
[PROPOSAL] and the 4 h datum PENDING).

Commands run: ls tests/soak; rg SOAK_MINUTES/SOAK_LEAK_MB_PER_H/SOAK_REPORT/CEILING_MB_PER_H in main.rs; sed 104-111p main.rs; rg cron/workflow_dispatch/timeout-minutes/continue-on-error/ubuntu-latest/dtolnay/upload-artifact in soak-nightly.yml; rg -c "#\[test\]" per soak module (12 total); rg soak_loops_stay_memory_bounded, ranked_by_growth; ls docs/perf.
