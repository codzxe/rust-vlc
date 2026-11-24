# AUDIT: scripts/quality/mutants_report.py
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 112 / 4 (find_outcomes, scenario_fields, main, _wall) |
| File verdict | REAL (with docstring contradictions) |
| Reachable from shipped entry point? | NO — dev quality script; invoked manually / by scripts/quality/12_mutants.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | `python3 scripts/quality/mutants_report.py /tmp/vta-audit/mut` (synthetic outcomes.json) -> correct report, rc=0; bare `python3 scripts/quality/mutants_report.py` -> "no outcomes.json at docs/audit/reports/mutants/codec-api", rc=1 |

## 1. One-paragraph verdict
A real, working cargo-mutants summarizer: I fed it a synthetic outcomes.json
and it correctly classified caught/missed/timeout/unviable, excluded
unviable from the score, computed 50.0% (2/4), and listed missed+timeout
survivors with file:line/function/replacement. Its own docstring lies in two
places: "Always exits 0" (a missing file exits 1 — executed and observed),
and its default argument points at docs/audit/reports/mutants/codec-api,
which does not exist anywhere in the tree. No doc or gate in the repo cites
this script's output, and no outcomes.json is committed — the mutation
scores it exists to produce exist nowhere as committed evidence.

## 2. What actually works (verified)
— Synthetic run (6 outcomes incl. one `{"summary": "Success"}` with no
scenario): output printed above; classification, score math, survivor
formatting, wall-time (3600s) all correct; rc=0.
— `_wall` handles the "Z" suffix and degrades to 0 on bad input.
— `scenario_fields` handles both the bare and `{"Mutant": ...}`-wrapped
scenario shapes and never raises on malformed entries (defaults to "?").

## 3. Findings
F-001 [S3]
Where: scripts/quality/mutants_report.py:16 and 36
Code:
```
Always exits 0: this is a report, not a gate.
```
```
    raise SystemExit("no outcomes.json at %s" % path)
```
What is wrong: `SystemExit("msg")` exits with code 1, not 0 — executed:
bare invocation printed "no outcomes.json at docs/audit/reports/mutants/
codec-api" and `echo rc=$?` showed 1. Malformed JSON would traceback and
exit nonzero too. A caller relying on "always exits 0" for CI arithmetic
is wrong.
What correct looks like: catch and print the error, then exit 0 (or drop
the claim).
Confidence: HIGH — Verification: ran.
F-002 [S3]
Where: scripts/quality/mutants_report.py:112
Code:
```
    main(sys.argv[1:] or ["docs/audit/reports/mutants/codec-api"])
```
What is wrong: the default path does not exist in the tree (docs/audit/
contains only docs-sweep-20261110.md and panic-baseline.json; no
outcomes.json is committed ANYWHERE — rg --files -g 'outcomes.json' finds
zero), so the no-arg mode can never succeed. The script's default is dead
on arrival, and none of the mutation-run raw data it summarizes is
committed as evidence.
Confidence: HIGH — Verification: ran + rg.
F-003 [S4]
Where: scripts/quality/mutants_report.py:78
Code:
```
        print("package : %s" % os.path.basename(os.path.dirname(find_outcomes(arg))) or arg)
```
What is wrong: `%` binds tighter than `or`, so this is `(print(...) or
arg)` — the `or arg` fallback is dead; an empty basename prints
"package : " with no fallback. Also find_outcomes() is called twice per
path (L55 and L78).
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: scripts/quality/mutants_report.py:70-71
Code:
```
            elif s == "Success":
                c["baseline"] += 1
```
What is wrong: the baseline counter is incremented but never printed
anywhere — Success rows are invisible in the report.
Confidence: HIGH — Verification: read + run (my Success row produced no
output line).
F-005 [S4]
Where: scripts/quality/mutants_report.py:99
Code:
```
    print("\nbands: >=80%% trustworthy | 60-79%% spotty | <60%% decorative")
```
What is wrong: plain string (no % formatting) — prints literal "80%%"
(observed in my run's output: "bands: >=80%% trustworthy").
Confidence: HIGH — Verification: ran (output shows %%).
F-006 [S4]
Where: scripts/quality/mutants_report.py:12-13 vs 66-68
Code:
```
plus every MISSED mutant - each is a line of production code whose behaviour no
```
vs
```
            elif s == "Timeout":
                c["timeout"] += 1
                survivors.append(o)
```
What is wrong: docstring promises only MISSED mutants in the survivor
list; timeouts are also listed. Defensible behavior, stale docstring.
Confidence: HIGH — Verification: read + run.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| find_outcomes | 28-36 | REAL | — | dir or file; checks 2 conventional paths; SystemExit(1) on miss (F-001) |
| scenario_fields | 39-49 | REAL | — | defensive, no panics on malformed |
| main | 52-99 | REAL | S3/S4 | correct math; dead `or arg`; invisible baseline; %% literal |
| _wall | 102-108 | REAL | — | Z-suffix aware, degrades to 0 |

## 5. Test honesty
No tests exist for this script. Mutation-reasoning: my synthetic run
proves classification and score math work; the script would misreport
nothing except the cosmetic defects above.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Always exits 0" | docstring :16 | FALSE | executed: rc=1 on missing default path |
| "Unviable mutants are excluded from the denominator" | :13-14 | REAL | run: unviable=1 excluded, score 2/4 |
| No repo doc cites this script or its scores | (rg) | REAL | rg 'mutants_report|MUTATION SCORE' over docs/ -> only my own audit log |

## 7. Dead / duplicated / unreachable code
The `or arg` fallback (L78), the `baseline` counter (never printed), and
the no-arg default path (L112) are all dead. Nothing else duplicates.

## 8. Dependencies & flags
stdlib only (json/os/sys/collections/datetime). No external deps.

## 9. Open questions for the next agent
Did any real cargo-mutants run ever feed this script (the 12_mutants.sh
lane)? No outcomes.json is committed, so any quoted mutation score in
worklog/PR text is UNVERIFIABLE from the tree — check the worklog claims
when auditing 12_mutants.sh.
