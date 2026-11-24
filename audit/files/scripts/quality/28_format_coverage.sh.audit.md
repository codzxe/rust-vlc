# AUDIT: scripts/quality/28_format_coverage.sh
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 137 / 1 bash wrapper + 1 embedded python program (3 phases: claims, fixtures, tests-text) |
| File verdict | SHALLOW (report generation works and is honest; the advertised --strict gate cannot fail) |
| Reachable from shipped entry point? | YES — scripts/quality/99_run_all.sh:40 invokes `J 28_format_coverage.sh` (WITHOUT --strict); also runnable standalone. Not in the release binary lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `bash scripts/quality/28_format_coverage.sh --strict` -> SCRIPT_EXIT=0 while reporting 3 unbacked claims (codec:hevc, container:mpd, container:ps) — proof of F-001; fuzz-target inventory (12 codec + 14 demux = header numbers verified); cleanup of run artifacts |

## 1. One-paragraph verdict
The matrix generator is real and its evidence model is sound (claims = fuzz-target filenames, evidence = committed fixtures, static walk, seconds-fast), and its own report honestly flags 3 unbacked claims today. But the advertised `--strict` contract is a lie: python computes the right exit (1 on gaps) and the bash wrapper throws it away, because neither this script nor common.sh uses `set -e` and the last command is a `say` that always succeeds. I proved it by execution: `--strict` run with 3 gaps returned SCRIPT_EXIT=0. Anyone wiring this gate into CI as documented ("exit 1 when a claimed format/codec has no fixture") gets a green that cannot fail — the textbook cannot-fail step.

## 2. What actually works (verified)
— verified by execution: `bash scripts/quality/28_format_coverage.sh --strict > /tmp/28fc.out; echo $?` -> `SCRIPT_EXIT=0`, output tail:
```
Unbacked claims:
- codec:hevc
- container:mpd
- container:ps
written: /home/z/my-project/audit-workspace/docs/audit/reports/coverage-matrix/coverage.txt
```
— verified by `ls fuzz/fuzz_targets/`: 12 `fuzz_codec_*` + 14 `fuzz_demux_*` — the header's "12 codecs and 14 containers" (L4) is exact today.
— verified by read: claims are derived from the tree's own fuzz-target names (L34-42), fixtures from a real os.walk with pruned target/node_modules/.git (L56-57), sizes >0 only.

## 3. Findings
F-001 [S2]
Where: scripts/quality/28_format_coverage.sh:15-18, 133-137
Code:
```
#   --strict: exit 1 when a claimed format/codec has no fixture
set -uo pipefail
```
and the tail:
```
say ""
say "written: $OUT/coverage-matrix/coverage.txt"
```
What is wrong: The strict gate cannot fail. The embedded python exits 1 on gaps (`sys.exit(1 if (strict and gaps) else 0)`, L132), but with only `set -uo pipefail` (no `-e`, here or in common.sh:6) execution continues to the `say` epilogue, and the script's exit status is `say`'s 0. Proven by execution: `--strict` with 3 unbacked claims (hevc, mpd, ps) -> SCRIPT_EXIT=0. Why it matters: the usage contract invites CI use of `--strict`; a wired-in strict gate would be permanently green — fake evidence by construction. Current automation is unaffected (99_run_all.sh:40 calls it WITHOUT --strict, report-only), which is why this is S2 and not S0. What correct looks like: capture `$PY`'s status (`rc=$?`) and `exit $rc` after the epilogue, or run the python under `set -e` scope. Confidence: HIGH — Verification: ran.
F-002 [S3]
Where: scripts/quality/28_format_coverage.sh:104
Code:
```
    named = ("fuzz_codec_" + c) in blob or (c in blob)
```
What is wrong: the second disjunct makes "named in tests" a trivial substring match over ALL .rs text in crates/apps — `c in blob` is true for e.g. "aac" appearing in any comment. The column is near-always "yes", so the "a test names it?" evidence half (header L9) is weak for short codec names. Why it matters: the matrix's second evidence column overstates test backing. What correct looks like: match only the fuzz-target identifier or a word-bounded regex. Confidence: HIGH — Verification: read (blob is all .rs text, L69-81).
F-003 [S4]
Where: scripts/quality/28_format_coverage.sh:131
Code:
```
print("\n".join(L[-12 - len(gaps):]))
```
What is wrong: the console tail-slice arithmetic is opaque and drops arbitrarily many header rows (only the last 12+gaps lines print) — the full truth goes only to coverage.txt. Cosmetic. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory >100 LOC)
| Function/phase | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| arg parse (--strict) | 21-22 | REAL | S2 | flag works in python; wrapper swallows exit (F-001) |
| claims from fuzz targets | 33-42 | REAL | — | 12/14 verified against fuzz_targets/ listing |
| ALIAS map | 44-50 | REAL | — | es->mp3/mp2/aac etc.; documented, quirky but consistent |
| fixture walk | 53-65 | REAL | — | prune target/node_modules/.git; size>0; corpus/ dirs absent in this workspace |
| tests-text blob | 67-81 | REAL | S3 | substring weakness (F-002) |
| find_fixtures | 84-92 | REAL | — | path-token match with aliases; `_k.`/`-k.` patterns reasonable |
| matrix render + gaps | 95-130 | REAL | — | markdown table to coverage.txt |
| console print + exit | 131-132 | REAL | S4 | tail-slice (F-003); python exit correct |
| wrapper epilogue | 135-136 | REAL | S2 | `say` last -> exit 0 always (F-001) |

## 5. Test honesty
No test harness for this script in-repo. Its own output is the evidence: today's run flags codec:hevc / container:mpd / container:ps as unbacked — the gate's *reporting* half is honest; its *enforcement* half (strict exit) is broken (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The tree claims support for 12 codecs and 14 containers through its own fuzz targets" | 28_format_coverage.sh:4-5 | REAL | ls fuzz/fuzz_targets -> 12 fuzz_codec_*, 14 fuzz_demux_* |
| "--strict: exit 1 when a claimed format/codec has no fixture" | 28_format_coverage.sh:17 | FALSE | execution: 3 gaps + --strict -> exit 0 (F-001) |
| "It fails when a claimed format has NO fixture" (PR usage) | 28_format_coverage.sh:11-13 | FALSE-as-stated (only true inside python) | same execution proof |

## 7. Dead / duplicated / unreachable code
None dead. ALIAS "av1": ["av1","ivf"] also feeds container claims (ivf demux target exists) — cross-use is intended.

## 8. Dependencies & flags
bash + python3 (common.sh PY launcher, Windows Store-alias-safe); no cargo needed. Writes only to docs/audit/reports/coverage-matrix/coverage.txt (repo-internal report dir, referenced by .github/workflows/audit.yml).

## 9. Open questions for the next agent
- INCIDENT NOTE (auditor): my execution created docs/audit/reports/coverage-matrix/coverage.txt (the script's designed output); I removed it + the empty coverage-matrix/ and reports/ dirs afterward to restore READ-ONLY state. docs/audit/ pre-existing content (docs-sweep-20261110.md, panic-baseline.json) untouched.
- Does any future CI plan wire `--strict`? If yes, F-001 becomes an S0 fake-gate at wire time.
