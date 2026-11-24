# AUDIT: scripts/quality/22_attack_surface.sh
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 123 / embedded python: 4 (fuzz glob+prefix map, astgrep hits merge, loc, report render) |
| File verdict | REAL-but-SILENTLY-ZEROED: the report generator runs, but its panic/ignored/unwrap columns read a file the security tier never produces |
| Reachable from shipped entry point? | N/A — quality tooling; run by 99_run_all.sh `security` tier (J reporter) + audit.yml security job (PR + nightly) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | extracted-python execution -> full matrix reproduced on this tree (all-zero hit columns); ls fuzz/fuzz_targets (35 targets, prefix classes confirmed); ls docs/audit/reports (absent) |

## 1. One-paragraph verdict
The matrix generator is real and I executed its embedded python on this tree:
10 boundaries, per-boundary fuzz-target counts (demux 14, codec 12, playlists
4, subtitles 3, streaming 2, and four boundaries with ZERO fuzz targets —
control protocol, settings TOML, source URI, wasm plugin ABI — correctly flagged
as "Gaps (a hacker starts here)"). THE DEFECT: the panic/ignored/unwrap
columns come from `docs/audit/reports/astgrep-summary.json` — WHICH DOES NOT
EXIST in the tree and is produced only by T03 (03_astgrep.sh), which the
SECURITY tier never runs (T03 lives in the `static` tier; audit.yml's security
job runs only the security tier). `if os.path.exists(j)` silently defaults to
zeros, so a fresh-checkout security run reports **0 panics on every boundary**
— false evidence of a clean attack surface, with no "run T03 first" error.
Verified by execution: my run shows all-zero columns precisely because the
JSON is absent. Second-order: the generated report hard-embeds the stale
"1,827 workspace-wide" unreachable! figure. As a J-reporter the script cannot
fail by design — acceptable — but its output can be silently WRONG.

## 2. What actually works (verified)
- Extracted python run (`sed -n '/^import glob/,/^PYEOF$/p'` -> scratch) -> full report rendered, exit path reached; matrix rows + gaps section produced as designed.
- Fuzz inventory: 35 .rs targets under fuzz/fuzz_targets/; prefix classes fuzz_demux_*/fuzz_codec_*/fuzz_sub_*/fuzz_playlist_*/fuzz_hls_*/fuzz_http_* all exist and map through PREFIX — verified by ls.
- The four no-fuzz-target gaps are honestly listed (control protocol, settings, source URI, wasm plugin ABI).
- Boundary->directory attribution is explicit and readable (BOUNDARIES table).

## 3. Findings
F-001 [S3]
Where: scripts/quality/22_attack_surface.sh:55-56
Code:
```
j = os.path.join(root, "docs", "audit", "reports", "astgrep-summary.json")
if os.path.exists(j):
```
What is wrong: silent zero-default — when astgrep-summary.json is absent
(it is ABSENT in this tree; `ls docs/audit/reports/` -> no such directory),
every panic/ignored/unwrap column renders 0. The security tier
(99_run_all.sh `security` = J 22 + JG 24 + JG 23) never runs T03/astgrep,
and audit.yml's security job runs only that tier on a fresh checkout — so the
PR-facing attack-surface summary asserts "no shipped panic" per boundary with
NO measurement behind it. Why it matters: this is the "attacker's map" — a
false clean map is worse than no map (a hacker starts here; the report says
don't worry). What correct looks like: `else: fail loudly ("run T03 first /
stale evidence")` or render "UNMEASURED" in the cells instead of 0.
Confidence: HIGH — Verification: ran (my execution produced the all-zero
columns exactly because the JSON is absent).
F-002 [S4]
Where: scripts/quality/22_attack_surface.sh:89-91
Code:
```
lines.append("`panics` counts `panic!`/`assert!` in shipped code only. `unreachable!`")
lines.append("(1,827 workspace-wide, the bigger DoS surface) is ratcheted separately by")
```
What is wrong: the generated report embeds the stale 1,827 figure (tree total
is 4,126 per loc-patterns.txt:28; shipped post-test-exclusion is ~37). The
report misstates the DoS surface to every reader. Confidence: HIGH —
Verification: ran (the number is in my reproduced output).
F-003 [S4]
Where: scripts/quality/22_attack_surface.sh:11 vs body
Code:
```
# No new scanning: this composes evidence already measured by T03/T05/T13.
```
What is wrong: the body consumes ONLY T03's astgrep-summary.json + the fuzz
glob. T05 (deadcode) and T13 (test_hygiene) evidence is composed NOWHERE in
this script — the header's composition claim overstates. Confidence: HIGH —
Verification: read the entire python.
F-004 [S4]
Where: scripts/quality/22_attack_surface.sh:121-122 (tail)
Code:
```
say ""
say "written: $OUT/attack/attack-surface.md"
```
What is wrong: the wrapper masks any python failure — moot here only because
the python has no failure exit at all (it cannot fail, even on a missing
evidence file — see F-001 for why that is exactly the problem). Listed for the
family pattern. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz inventory (glob+PREFIX) | 37-51 | REAL (executed; 35 targets classified) | — | prefix map correct |
| astgrep hits merge | 53-70 | REAL-code / SILENT-ZERO behavior | S3 | F-001: absence of JSON -> zeros, no error |
| loc | 72-82 | REAL (executed; counts include tests) | — | cosmetic |
| report render | 84-118 | REAL (executed) | S4 | embeds stale 1,827 (F-002); gaps section correct |

## 5. Test honesty
No tests for this script; my scratch execution is the test. Mutation
reasoning: the matrix WOULD show the DoS gaps only if the JSON existed with
hits — on this tree it cannot (F-001). The report's self-description ("from
evidence already measured by T03") is false on a fresh security-tier run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this composes evidence already measured by T03/T05/T13" | :11 | FALSE (T05/T13 not consumed; T03's file absent on fresh runs) | read + ls docs/audit/reports |
| "A boundary with zero fuzz targets and shipped `unreachable!` is a DoS" | :9 | TRUE (concept); the report cannot see unreachable! at all | the column set has no unreachable field |

## 7. Dead / duplicated / unreachable code
None dead; the tail masking is family-pattern (F-004).

## 8. Dependencies & flags
bash + python3 (common.sh PY), reads fuzz/fuzz_targets/*.rs and
docs/audit/reports/astgrep-summary.json (transitive T03 artifact — the
unproduced dependency is F-001), writes $OUT/attack/attack-surface.md.

## 9. Open questions for the next agent
- Does T03's astgrep write production_hits with the exact rule names
  (panic-in-lib / ignored-result / unwrap-outside-tests / no-op-test)?
  Verify when auditing 03_astgrep.sh — a rule-name mismatch would also silently
  zero the columns even WITH the JSON present.
- panic-in-lib.yml (my next-but-one file) is the ast-grep rule for
  panic-in-lib — the rule names here are its companion set.
