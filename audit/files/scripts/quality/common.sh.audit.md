# AUDIT: scripts/quality/common.sh
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / functions | 76 / 7 (PY detection, say, head1, stamp, not_run, need, sg_run, crates_list, log) |
| File verdict | REAL (executed a sandboxed copy: say/head1/log/need/not_run all behave; the honesty mechanism works) |
| Reachable from shipped entry point? | YES: sourced by 30 scripts/quality/*.sh runners, which .github/workflows/audit.yml runs via 99_run_all.sh tiers |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | bash -n -> OK; sandboxed source + helper exercises (say/head1/log/need/not_run) -> all behave; not-run.txt format verified |

## 1. One-paragraph verdict
The shared quality-sweep helpers do what they promise, and the load-bearing
honesty mechanism — `not_run()` appending a timestamped tab-separated record to
reports/not-run.txt instead of inventing a result — works as designed (I
exercised it: `need rpmbuild-` returned 1 and wrote
`2026-11-24T17:33:50Z\trpmbuild-\tnot installed (run
scripts/quality/00_install_tools.sh)` to not-run.txt). `set -uo pipefail`
without `-e` is the documented rule ("never abort the whole sweep because one
tool failed"), and every helper respects it. The Python-launcher detection
genuinely handles the Windows Store alias trap (validates the interpreter
executes before choosing it). One hygiene defect: if BOTH python3 and python
fail the execution check, PY is set back to `python3` anyway — a known-broken
interpreter — so downstream `$PY` calls die with the cryptic Store-alias error
instead of a clean `not_run` record. `crates_list` shells out to cargo
metadata (read-only) with a correct name\trelpath format.

## 2. What actually works (verified)
- `bash -n scripts/quality/common.sh` -> exit 0.
- Sandbox source + exercises: `say` -> prints; `head1` -> `=== X ===` banner; `echo | log test-dummy.log` -> tees to $OUT/test-dummy.log (file created); `need <missing-tool>` -> prints `NOT RUN [...]`, returns 1, appends the tab record to not-run.txt.
- `sg_run` launcher: ast-grep -> sg -> `npx.cmd --yes -p @ast-grep/cli@0.51.3` -> `npx ...` (pinned version, Windows .cmd shim handling — consistent with the repo's pin-everything posture).
- ROOT resolution `$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)` — correct for a SOURCED file (BASH_SOURCE, not $0).

## 3. Findings
F-001 [S4]
Where: scripts/quality/common.sh:22-26
Code:
```
if command -v python3 >/dev/null 2>&1 && python3 -c "pass" >/dev/null 2>&1; then
  PY=python3
elif command -v python >/dev/null 2>&1 && python -c "pass" >/dev/null 2>&1; then
  PY=python
else
  PY=python3
fi
```
What is wrong: the final `else` sets PY to `python3` — an interpreter the
guard just PROVED fails to execute (or is absent). Every later `$PY` call
(crates_list, per-script parsers) then fails with the raw error (e.g. the
Windows Store REGDB_E_CLASSNOTREG) instead of a recorded `NOT RUN [python]`
line. The file's own rule ("NEVER invent a result") is honored by accident
(hard failure ≠ invented result) but the failure is unattributed. What
correct looks like: `not_run python "no working interpreter" && PY=""` and a
guard in crates_list. Confidence: HIGH — Verification: read.
Note: on all real CI hosts python3 passes the check, so the path is dormant.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PY detection | 22-26 | REAL-FLAWED | S4 | F-001 dormant-broken fallback |
| say / head1 / stamp | 28-30 | REAL | — | executed |
| not_run | 33-37 | REAL | — | the honesty primitive; format verified (tab-separated, UTC stamp) |
| need | 40-47 | REAL | — | returns 1 on missing; not_run side effect verified |
| sg_run | 51-60 | REAL-UNPROVEN | — | ast-grep/sg absent in sandbox; npx fallback not executed here (network); pin @0.51.3 present |
| crates_list | 63-75 | REAL-UNPROVEN | — | needs cargo (absent on this sandbox PATH); heredoc parses cleanly; correct name\trel output contract |
| log | 76-79 | REAL | — | tee passthrough verified |

## 5. Test honesty
The helpers ARE the honesty mechanism for the whole quality program (not_run
+ need). Verified working. Mutation reasoning: if not_run silently dropped
the append, reports/not-run.txt would lose rows — my exercise shows the
append happens (file content pasted above).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Rule: never abort the whole sweep because one tool failed - record and continue" | :4 | REAL | set -uo pipefail (no -e); need/not_run design; exercised |
| "NEVER invent a result" | :34 | REAL | not_run records absence, not zeros |
| "the Windows Store `python3` App Execution Alias resolves but fails at exec time ... validate that the interpreter actually runs" | :15-17 | REAL | the `python3 -c pass` exec check exists (though F-001's fallback undermines the worst case) |

## 7. Dead / duplicated / unreachable code
None. All helpers have callers (30 sourcing scripts).

## 8. Dependencies & flags
bash (POSIX-ish; BASH_SOURCE use makes it bash-only despite "POSIX-ish" —
fine on Git-Bash/Linux CI as claimed at :3); stdlib python for crates_list;
pinned npx ast-grep fallback. Creates $OUT (docs/audit/reports) at source
time — a side effect consistent with its role.

## 9. Open questions for the next agent
- 00_install_tools.sh (my list, next) should install the tools `need` checks
  (ast-grep, cargo-nextest, typos, machete, ...) — verify the correspondence
  when audited.
