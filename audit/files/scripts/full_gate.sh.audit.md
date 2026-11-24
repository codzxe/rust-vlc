# AUDIT: scripts/full_gate.sh
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 85 / 11 gated sections + result |
| File verdict | REAL (honest failing gate — verified via orchestrator baseline, exit code propagates) |
| Reachable from shipped entry point? | YES (dev/release lane): not wired into .github/workflows directly — ci.yml replicates it as "PARITY TARGET" (ci.yml:13,32,36); docs/release-checklist.md references it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `bash -n` -> OK; `grep -rn full_gate .github/...` -> ci.yml parity references; baseline citation audit/runs/baseline/full_gate.txt tail (GATE RESULT: FAIL); sed quote verification |

## 1. One-paragraph verdict
This gate is the honest one the repo rarely gets right elsewhere: every one of
the 11 sections runs inside `if <cmd>; then ... else ...; fail=1; fi`, there
is no `|| true`, no `continue-on-error`, no pipe-masking (pipefail set), and
the script exits with the aggregated `$fail`. Proof it can fail: the
orchestrator baseline run ends `=== GATE RESULT: FAIL ===` with sections 1,
3, 4, 7, 8, 9, 10 red. The defects are all hygiene: a hardcoded
`/home/z/my-project` default directory (orchestrator machine leaked into a
repo-committed script), fixed `/tmp/*.log` collision paths, and an undeclared
`rg` dependency for the summary lines.

## 2. What actually works (verified)
- `bash -n scripts/full_gate.sh` -> exit 0.
- Exit propagation read line-by-line: every failure branch sets `fail=1`;
  L84-85 `echo "=== GATE RESULT: $([ $fail -eq 0 ] && echo PASS || echo FAIL) ==="` / `exit $fail`.
- Baseline execution (audit/runs/baseline/full_gate.txt, tail): `=== GATE RESULT: FAIL ===` — the gate failed on fmt/test/arch/lint-literals/identity/sbom/web-budget and still exited non-zero; honesty claim (header L2) is REAL.
- §9 sbom --check is what catches the stale SBOM (F-001 of my cyclonedx.json audit): baseline L69-75 `sbom: FAILED ... result: DRIFT`.

## 3. Findings
F-001 [S4] — Orchestrator machine path baked into repo script
Where: scripts/full_gate.sh:3,6
Code:
```
# Usage: bash /home/z/my-project/scripts/full_gate.sh [worktree-dir]
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "${1:-/home/z/my-project}"
```
What is wrong: the default target dir is the orchestrator's absolute home
path. On any other machine `cd` fails; with no `-e`, execution continues in
the caller's cwd and the gate then audits whatever directory that is. If
that cwd is another Cargo project, the gate happily reports on the wrong
tree. Why it matters: a gate that can silently target the wrong directory.
Blast radius small (fresh-checkout users run it from repo root anyway).
What correct looks like: `cd "${1:-$(dirname "$0")/..}"`.
Confidence: HIGH — Verification: read + bash semantics.

F-002 [S4] — Fixed /tmp log paths, no mktemp
Where: scripts/full_gate.sh:14,21,29,36,43,50,57,64,71,78
Code:
```
if cargo clippy --workspace --all-targets --all-features -- -D warnings > /tmp/clippy.log 2>&1; then
```
What is wrong: every section writes a fixed `/tmp/<name>.log`. Two
concurrent runs (or two worktrees) clobber each other's evidence; the logs
also persist after exit. Why it matters: a FAIL section's diagnostic tail
can come from another run's log. What correct looks like: `mktemp -d`.
Confidence: HIGH — Verification: read.

F-003 [S4] — Undeclared `rg` dependency degrades silently
Where: scripts/full_gate.sh:22,25
Code:
```
  rg "test result:" /tmp/test.log | awk '{p+=$4; f+=$6; i+=$8} END {print "tests: "p" passed / "f" failed / "i" ignored"}'
```
What is wrong: if ripgrep is absent, `rg` fails (command-not-found does not
abort under `set -u`), awk prints `tests:  passed /  failed /  ignored`
(zeros), and the gate still prints `test: CLEAN`. The gate verdict itself is
still driven by cargo's exit code (honest), so this is cosmetic — but the
summary can lie while the verdict doesn't.
Confidence: HIGH — Verification: read; `rg` presence here confirmed (this
audit used rg).

## 4. Section table
| Section | Lines | Command | Failure path honest? |
|---|---|---|---|
| 1 fmt | 10-11 | cargo fmt --all --check | yes, fail=1 |
| 2 clippy | 13-18 | cargo clippy --workspace --all-targets --all-features -D warnings | yes |
| 3 test | 20-26 | cargo test --workspace --all-features | yes (baseline: FAILED, 2 tests) |
| 4 check-architecture | 28-33 | cargo xtask check-architecture | yes |
| 5 fixtures --check | 35-40 | cargo xtask fixtures --check | yes |
| 6 fuzz-registry | 42-47 | bash scripts/fuzz_registry_check.sh | yes |
| 7 lint-literals | 49-54 | cargo xtask lint-literals | yes (baseline FAILED) |
| 8 check-identity | 56-61 | cargo xtask check-identity | yes (baseline FAILED) |
| 9 sbom --check | 63-68 | cargo xtask sbom --check | yes (baseline FAILED/DRIFT — the stale-SBOM catcher) |
| 10 web-budget --check | 70-75 | cargo xtask web-budget --check | yes (baseline FAILED, wasm target absent) |
| 11 packaging | 77-82 | bash scripts/packaging/gate.sh | yes (baseline PASS) |

## 5. Test honesty
Not a test file. Its own honesty is the subject: verified above (all 11
sections aggregate into exit code; baseline shows FAIL result propagating).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "HONEST exit-code propagation (no pipe-masking)" (L2) | header | REAL | read every section + `exit $fail` L85; baseline FAIL |
| ci.yml is "full_gate.sh parity" | .github/workflows/ci.yml:13,32 | REAL-UNPROVEN | step names say "full_gate.sh #N"; whether CI misses a section is ci.yml's auditor's job (not my file) |

## 7. Dead / duplicated / unreachable code
None. Note: `set -uo pipefail` deliberately omits `-e` — correct here because
every command is guarded by `if`; adding `-e` would abort on first red section
and HIDE later results (the keep-going aggregation is the point).

## 8. Dependencies & flags
cargo (fmt/clippy/test), xtask subcommands (check-architecture, fixtures,
lint-literals, check-identity, sbom, web-budget), rg, awk, bash. Heavy
commands NOT re-run by me (load discipline) — baseline cited instead.

## 9. Open questions for the next agent
- ci.yml "parity": does CI replicate section 5 (fixtures) and 11 (packaging)? If not, parity claim is inflated — belongs to the ci.yml auditor.
- The repo ships with this gate RED (7 sections) — a process-level S2 for the orchestrator's synthesis, not this file.
