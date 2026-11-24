# AUDIT: scripts/quality/24_panic_budget.sh
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 203 / embedded python: 6 (strip_test_regions, strip_comments_and_strings [DEAD], scrub, scan, total, main flow) |
| File verdict | SHALLOW-as-gate: the ratchet LOGIC is REAL and executed, but the shell wrapper CANNOT FAIL — the gate contract is false |
| Reachable from shipped entry point? | N/A — quality tooling; invoked by scripts/quality/99_run_all.sh `security` tier as a JG GATE (line 40) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | extracted-python runs (rc 1 on forced growth, rc 0 at baseline, counting reproduced 41=41); masktest.sh empirical rc=0 with inner exit 1; classification count of unreachable! (2177 tests-dir / 1909 cfg(test)-bearing / 11 src-only) |

## 1. One-paragraph verdict
The ratchet ENGINE works — I ran the embedded python against a scratch copy of
the committed baseline: current tree = 41 shipped panic sites = baseline
(RATCHET OK, rc 0), and a zeroed scratch baseline correctly produced "RATCHET
FAILED: 14 crate(s)" with python exit 1. The counting method is sound
(strings/comments scrubbed BEFORE brace-matched test-region removal, so braces
in literals cannot desync the matcher; the 4,126 total `unreachable!` in
loc-patterns.txt decomposes as 2,177 in tests/ dirs + 1,909 in cfg(test)-bearing
files vs ~37 shipped — the exclusion explains the delta, no undercount bug).
THE DEFECT: the shell wrapper swallows the python's exit code — `set -uo
pipefail` (no -e), and the last commands are two `say` lines, so
24_panic_budget.sh ALWAYS exits 0 even when the python says RATCHET FAILED.
Empirical proof: an exact-pattern scratch script (set -uo pipefail; python
heredoc exiting 1; trailing `say`) exits 0. 99_run_all.sh's JG gate plumbing
(`if [ "$rc" -ne 0 ]`) therefore NEVER fires for this script — the "security"
tier's panic gate is theatre, precisely the failure mode its own comment
("a CI that cannot fail is theatre") claims to have fixed.

## 2. What actually works (verified)
- Extracted python (`sed -n '/^import json/,/^sys.exit(rc)$/p'` -> audit/scratch/w17a10/panic_py.txt), run vs a scratch copy of docs/audit/panic-baseline.json: -> `Baseline grand total: 41 -> now 41.` `**RATCHET OK**` rc 0 — the committed baseline matches the current tree exactly.
- Forced-failure run (scratch baseline zeroed): -> `- **GREW** tools/xtask: 0 -> 10 (+10)` ... `**RATCHET FAILED**: 14 crate(s)` python rc **1** — the ratchet logic genuinely detects growth.
- Masking proof: `masktest.sh` (identical shell shape, inner `sys.exit(1)`) -> `script_rc=0`.
- Counting sanity: tree-wide `unreachable!` = 4,097 found by my classifier (2,177 tests-dir, 1,909 cfg(test)-bearing files, 11 src-only) vs loc-patterns.txt:28 `unreachable! 4126` — consistent; the script's 41-site shipped count is plausible, NOT an undercount bug.

## 3. Findings
F-001 [S2]
Where: scripts/quality/24_panic_budget.sh:196-202
Code:
```
PYEOF

say ""
say "written: $OUT/panic/panic-budget.txt"
```
What is wrong: no `exit $?` after the python heredoc — with `set -uo pipefail`
(no -e) the script's exit status is the last `say` (printf) = 0, ALWAYS. The
python's `sys.exit(rc)` (rc=1 on RATCHET FAILED) is discarded. Why it
matters: 99_run_all.sh declares this a GATE (`JG 24_panic_budget.sh`, whose
rc "fails the tier") — but the gate can never return non-zero, so panic-site
growth fails nothing. The header's own claim "FAILS any change that increases
it" is false at the process level. What correct looks like: `exit $?` after
the heredoc (as 27_speed_parity.sh does: its tail is `exit "$rc"`).
Confidence: HIGH — Verification: ran (masktest.sh + extracted python rc 1
with wrapper pattern proven to mask).
F-002 [S3]
Where: scripts/quality/24_panic_budget.sh:4
Code:
```
# 1,827 shipped `unreachable!` in a media engine are not style debt, they are
```
What is wrong: the motivating number matches nothing verifiable today:
shipped unreachable! on this tree is ~37 (my run: xtask 10, playlists 6, input
5, control-protocol 4, ...); total incl. tests is 4,126. The 1,827 figure is
stale narrative from an earlier tree state and now misstates the shipped
surface by ~50x. Confidence: HIGH — Verification: ran the counter.
F-003 [S4]
Where: scripts/quality/24_panic_budget.sh:79-84
Code:
```
def strip_comments_and_strings(text):
    text = re.sub(r"//[^\n]*", "", text)
```
What is wrong: dead function — never called (scrub() is the used path; scan()
line 121 calls strip_test_regions(scrub(txt))). Also its order
(comments-before-strings) is the exact anti-pattern scrub()'s docstring warns
about. Confidence: HIGH — Verification: read + rg call sites (0).

## 4. Per-function table (mandatory; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| strip_test_regions | 40-76 | REAL (exercised via scan; brace-matched, conservative on non-mod cfg(test)) | — | correct order (scrub first) |
| strip_comments_and_strings | 79-84 | DEAD | S4 | F-003 |
| scrub | 87-98 | REAL (exercised) | — | raw+normal strings before comments — the documented-bug-avoiding order |
| scan | 101-126 | REAL (executed; counts reproduce baseline 41) | — | skips tests/benches/examples/target/node_modules; per-crate src/ walk |
| total | 129-130 | REAL | — | |
| main flow | 133-198 | REAL (python) / MASKED (shell) | S2 | rc contract broken only at wrapper level (F-001) |

## 5. Test honesty
No tests in-repo for this script; my scratch runs are the test. Mutation
direction: zeroed baseline -> 14 GREW + rc 1 (would fail IF the wrapper
propagated). The auto-tighten (floor re-seed on OK) is a sound ratchet design;
the re-seed writes docs/audit/panic-baseline.json in-repo — I deliberately ran
only the extracted python against a SCRATCH baseline copy to avoid mutating
the repo file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "FAILS any change that increases it" | :7-8 | FALSE (process-level) | masktest.sh: exact pattern -> rc 0 despite inner rc 1 (F-001) |
| "1,827 shipped unreachable!" | :4 | STALE/FALSE-now | my count: ~37 shipped, 4,126 total (F-002) |
| "records the count per crate in docs/audit/panic-baseline.json" | :7 | TRUE | file exists, grand_total 41, crates 14; my run reproduces 41 |
| "brace-matched #[cfg(test)] excluded; tests/benches/examples skipped" | baseline json method | TRUE | scan() read + executed; test-region classifier corroborates the delta |

## 7. Dead / duplicated / unreachable code
strip_comments_and_strings (dead). Nothing else.

## 8. Dependencies & flags
bash + python3 (via common.sh's PY probe), writes $OUT/panic/panic-budget.txt
and mutates docs/audit/panic-baseline.json (side effect: OK runs tighten the
floor — by design). `--update` re-seeds. No external tools.

## 9. Open questions for the next agent
- The SAME masking tail exists in 22_attack_surface.sh, 16_fuzz.sh,
  23_alloc_budget.sh, 26_complexity.sh (all end in `say`); only
  27_speed_parity.sh ends with `exit "$rc"`. The JG-declared gates
  (24, 23, 26) in 99_run_all.sh are ALL theatre. Confirm each when auditing
  those files (16 and 22 are mine — see their audits).
- Was 99_run_all.sh `security` EVER run in CI? rg .github/workflows for
  99_run_all/panic — if absent, the ratchet is also not CI-gated at all.
