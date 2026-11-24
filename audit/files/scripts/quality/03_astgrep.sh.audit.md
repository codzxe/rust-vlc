# AUDIT: scripts/quality/03_astgrep.sh
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 55 / 1 flow (rule-file loop + python summary) |
| File verdict | REAL (reporter-tier tool by design: counts hits, never fails — documented J/JG split in 99_run_all.sh) |
| Reachable from shipped entry point? | YES as tooling — static/quality tiers of scripts/quality/99_run_all.sh (J = reporter, does not fail the tier) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |

## 1. One-paragraph verdict
A real reporter that runs ast-grep over 11 rule files and prints per-rule hit counts plus a python summary with per-file top-3. The §8.17-style question ("does this quality script ever fail?") has a documented answer here: it is a REPORTER (tier J), and the runner's own comment records that this distinction was ADDED after "every tier exited 0 no matter what a script reported" — the unfailable-CI bug class was caught and fixed at the framework level; only 26_complexity, 23_alloc_budget, 24_panic_budget, 27_speed_parity are gates. The tool never fabricates results: missing rule files and failed scans are recorded via not_run (appended to docs/audit/reports/not-run.txt with a timestamp + reason), never invented. Three S4-grade report-hygiene notes.

## 2. What actually works (verified)
— Read in full + common.sh read: `sg_run` prefers a local ast-grep binary, falls back to PINNED `@ast-grep/cli@0.51.3` via npx (no floating tool version) — supply-chain posture consistent with the release lane's pin discipline.
— The J/JG split verified in 99_run_all.sh:24-26 + L38-45 (03_astgrep is J in both static and quality tiers; the comment at L16-18 documents the past every-tier-exits-0 bug).
— Python launcher hardening read (common.sh): the Windows Store python3 App Execution Alias is probe-validated (`python3 -c pass`) before use — a real Windows CI failure class handled.
— NOT EXECUTED deliberately: running it would write `docs/audit/reports/*` (repo files) — prohibited by this audit's read-only rule; analysis is read-based, noted as such.

## 3. Findings

F-001 [S4]
Where: scripts/quality/03_astgrep.sh:26-29
Code:
```
  if [ $rc -ne 0 ] && [ ! -s "$OUTD/$id.json" ]; then
    not_run "ast-grep:$id" "scan failed rc=$rc: ..."
    continue
  fi
```
What is wrong: a scan that exits nonzero but leaves a NON-empty (partial) json is neither failed nor flagged — its partial hits are counted as if complete. Under-reporting with no marker. Confidence: MEDIUM (read; ast-grep's failure modes not exercised). What correct looks like: flag rc!=0 regardless of json size.

F-002 [S4]
Where: scripts/quality/03_astgrep.sh:30-31
Code:
```
  n=$($PY -c "import json,sys;print(len(json.load(open(sys.argv[1],encoding='utf-8'))))" "$OUTD/$id.json" 2>/dev/null || echo "?")
  say "rule=$id hits=?"
```
What is wrong: an unparseable json prints `hits=?` and moves on — no not_run record for the parse failure (the `|| echo "?"` swallows it). A "?" row is easy to skim past. Confidence: HIGH (read).

F-003 [S4]
Where: scripts/quality/03_astgrep.sh:45-46
Code:
```
    if not isinstance(hits, list):
        continue
```
What is wrong: the summary silently skips non-list json (an ast-grep error object would be dropped with no PARSE ERROR line, unlike the exception path at L43-44 which does print). Report-hygiene asymmetry. Confidence: HIGH (read).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| rule loop | 21-32 | REAL | S4 | explicit rule ids -> files; missing rule -> not_run; F-001/F-002 |
| summary | 34-52 | REAL | S4 | per-rule count + top-3 files + TOTAL; F-003 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (reporter; no tests) | n/a | reporter contract: never claims a result it didn't get (not_run discipline); hits are data, not verdicts |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Rule: never abort the whole sweep because one tool failed - record and continue" | common.sh L5 | REAL | not_run + continue shapes in this script (L23, L27) |
| "J = reporter: exit code recorded but does not fail the tier ... Without this, every tier exited 0" | 99_run_all.sh L16-18 | REAL | read — the unfailable-tier bug was found and structurally fixed (J vs JG) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
ast-grep (local or pinned npx @ast-grep/cli@0.51.3); python for counts; writes reports under docs/audit/reports/ (the suite's committed-report design — which is why this audit did not execute it).

## 9. Open questions for the next agent
1. Which of the 11 rules produce current hits? (docs/audit/reports/astgrep/*.json — committed outputs; check they match the current tree.)
2. The two assigned rule files (sleep-in-test.yml, allow-attr.yml) — audited next in my queue.
