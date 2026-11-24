# AUDIT: scripts/quality/12_mutants.sh
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 92 / 1 script (4 modes: list/crate/diff/full) |
| File verdict | REAL-UNPROVEN (logic sound; never executed anywhere verifiable) |
| Reachable from shipped entry point? | NO — dev quality tool; called by scripts/quality/99_run_all.sh:49 (J, non-gate) and .github/workflows/audit.yml:211 (CI, 8 shards) — CI never ran |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg over common.sh/00_install_tools.sh/99_run_all.sh/audit.yml/.gitignore; NOT executed — common.sh does `mkdir -p $ROOT/docs/audit/reports` and writes not-run.txt into the repo, which my read-only mandate forbids |

## 1. One-paragraph verdict
A coherent cargo-mutants driver with sensible modes (census / one-crate /
git-diff-scoped / sharded full), a real exclusion regex, and CI wiring that
installs the pinned tool (00_install_tools.sh:14 `cargo-mutants@27.1.0`,
matching the 27.x schema mutants_report.py parses). The systemic problem is
that the entire mutation-testing program — the script's stated purpose,
"Answers: are the tests real?" — has produced ZERO committed evidence:
outputs are gitignored (.gitignore:121 `docs/audit/reports/`), cargo-mutants
is not installed locally (`need cargo-mutants || exit 0`), and the CI lane
that would generate the shards has never executed (billing-blocked). The
tooling is real; the answer it exists to produce does not exist.

## 2. What actually works (verified)
— Static read: all four modes build correct cargo-mutants invocations
(--list --workspace; -p with --test-tool nextest --jobs --timeout --json
--output; --in-diff against a base; --workspace sharded with per-shard
output names).
— Cross-script consistency: `--output $OUT/mutants/$pkg/outcomes.json`
(common.sh OUT=docs/audit/reports) is exactly the path
mutants_report.py's find_outcomes() reads (dir/outcomes.json) — the two
scripts interlock correctly.
— The honest-skip machinery is real: common.sh need() records
NOT_RUN into not-run.txt and the file names the installer to run.

## 3. Findings
F-001 [S3]
Where: scripts/quality/12_mutants.sh:12 and .gitignore:121
Code:
```
need cargo-mutants || exit 0
```
What is wrong: on any machine without cargo-mutants (this sandbox: absent),
the whole T12 "primary task" exits 0 having done nothing; its outputs land
in a gitignored directory (.gitignore:121 `docs/audit/reports/`), and the
CI lane that installs the tool (.github/workflows/audit.yml:211) has never
run. Consequence: no mutation score, survivor list, or outcomes.json exists
anywhere in the repo — the question "are the tests real?" that this script
is the designated answer to has no committed answer.
What correct looks like: commit at least one reference run's outcomes.json
+ report under docs/quality/ (unignored), or run the CI lane once.
Confidence: HIGH — Verification: read + rg (.gitignore, audit.yml) + which
cargo-mutants (absent).
F-002 [S4]
Where: scripts/quality/12_mutants.sh:30
Code:
```
    total=$(grep -c ':' "$OUT/mutants/list.txt" 2>/dev/null || echo 0)
```
What is wrong: on zero matches grep -c prints "0" AND exits 1, so `|| echo 0`
appends a second "0" — total becomes the two-line string "0\n0" and the
report prints "total mutants: 0\n0". Cosmetic only.
Confidence: HIGH — Verification: read (grep -c semantics).
F-003 [S4]
Where: scripts/quality/12_mutants.sh:27-29
Code:
```
    cargo mutants --list --workspace \
      --exclude-re "$EXCLUDE_RE" \
      2>/dev/null | tee "$OUT/mutants/list.txt" | tail -5
```
What is wrong: stderr discarded — if `cargo mutants` fails (bad toolchain,
workspace error), the census silently reports 0 mutants and the script
exits 0. Fail-open census.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: scripts/quality/12_mutants.sh:16
Code:
```
EXCLUDE_RE='(^|/)(tests|benches|examples)/|_template|desktop-tauri|schemas/generated'
```
What is wrong: none functionally — but note `tools/` (vta harnesses, xtask,
sweep) is NOT excluded, so a full run would mutate dev tools too
(workspace-wide list includes them if they are members; vta is non-member,
xtask/sweep are members). Time-cost observation only.
Confidence: MEDIUM — Verification: read.

## 4. Per-mode table
| Mode | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| list | 25-35 | REAL-UNPROVEN | S4 | census; stderr swallowed (F-003); double-zero count (F-002) |
| crate | 36-46 | REAL-UNPROVEN | — | correct flags; writes outcomes.json where mutants_report.py reads |
| diff | 47-60 | REAL-UNPROVEN | — | requires .git (absent here — not_run + instructions, honest) |
| full | 61-76 | REAL-UNPROVEN | — | shard parsing manual but correct; per-shard output prevents clobber |
| usage | 77-81 | REAL | — | exit 2 on unknown mode |

## 5. Test honesty
No tests for the script itself; it is the test-quality instrument. Its own
gating logic (exit 0 when the tool is missing) means its absence is silent
in any rc-based consumer — see F-001.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T12 - MUTATION TESTING (primary task). Answers: are the tests real?" | :2 | PAPER-ONLY as an answer | no committed outcomes/scores anywhere (rg for outcomes.json: zero files) |
| CI aggregates "the number that answers 'are the tests real?'" | audit.yml mutation-score job | UNREACHABLE | CI never executed (billing); no artifacts exist |

## 7. Dead / duplicated / unreachable code
None dead in-file. The `how to read this` heredoc prints after every
non-usage mode — fine.

## 8. Dependencies & flags
cargo-mutants 27.1.0 (pinned installer), nextest test tool, MUTANTS_TIMEOUT
default 60s, MUTANTS_JOBS default 8. common.sh provides OUT/need/not_run.

## 9. Open questions for the next agent
Did ANY local mutation run ever happen (worklog mentions?) — check
worklog.md for "mutants" claims; any quoted score is unverifiable from the
tree (this file's outputs are gitignored).
