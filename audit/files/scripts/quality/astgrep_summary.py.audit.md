# AUDIT: scripts/quality/astgrep_summary.py
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 75 / 2 (is_test_path, main) |
| File verdict | REAL code, UNREACHABLE in its own pipeline (zero callers; classification logic itself executed and correct via patched sandbox copy) |
| Reachable from shipped entry point? | NO as shipped binary (dev tooling); worse: NOT reachable from the quality pipeline either — 99_run_all.sh (static+quality tiers) runs 03_astgrep.sh, which never invokes this script; audit.yml:88 references a .txt this script never writes |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | real script on real tree (exit 1, traceback); patched sandbox copy with 2 synthetic rule JSONs + 2 poison files (exit 0, table + summary.json verified); 6 is_test_path probes; rg pipeline wiring greps; sed quote verifications |

## 1. One-paragraph verdict
The classification logic is real and works — I executed it against synthetic
rule output and it produced the correct SHIPPED/tests split, per-file triage
queue, and a well-formed summary.json. But the script is an orphan: NOTHING in
the tree ever invokes it. The static/quality tiers (99_run_all.sh:31/:44) run
03_astgrep.sh, which writes the per-rule JSONs and then prints its OWN minimal
inline summary — no step calls astgrep_summary.py; audit.yml's Scoreboard step
cats `docs/audit/reports/astgrep-summary.txt`, an artifact NO tool produces
(the script writes .json). Net effect on the real pipeline: astgrep-summary.json
never exists, so 22_attack_surface.sh's panic-tier columns render ALL-ZERO
(false clean — cross-ref W17-A10 22_attack_surface F-001) and report.py's
FINDINGS reader gets nothing (cross-ref W17-A08 report.py F-006). A correct
summarizer that is never run is how a security dashboard lies with true data.

## 2. What actually works (verified)
- Patched sandbox copy (output redirected to audit/scratch; repo untouched):
  `python3 .../astgrep_summary_patched.py` -> exit 0; table:
  `todo-macros 3 2 1`, `panic-in-lib 4 1 3`; summary.json counts
  `{'panic-in-lib': {'total': 4, 'production': 1, 'test': 3}, 'todo-macros':
  {'total': 3, 'production': 2, 'test': 1}}` + production_hits rows with
  rule/file/line — exactly the synthetic inputs' correct split.
- is_test_path probes (direct import): 'crates/foo/tests/x.rs' -> True,
  'crates/foo/src/lib.rs' -> False, windows 'crates\\foo\\tests\\w.rs' ->
  True (backslash normalization works).
- BOM-tolerant read (encoding="utf-8-sig").

## 3. Findings
F-001 [S2]
Where: scripts/quality/astgrep_summary.py (whole file — its call graph)
Code:
```
for fn in sorted(os.listdir(D)):
    if not fn.endswith(".json") or fn == "summary.json":
```
What is wrong: the script has ZERO callers. rg over scripts/ + .github/ + tools/: 99_run_all.sh invokes 03_astgrep.sh (which generates $OUT/astgrep/*.json and prints its own inline PYEOF summary) but never this script; 22_attack_surface.sh:55 and report.py:144 only READ its would-be output; audit.yml:88 reads astgrep-summary.txt which nothing produces. Why it matters: the artifact this script exists to produce (astgrep-summary.json, the prod/test split that 22_attack_surface's panic tier and report.py's FINDINGS consume) is never generated on any real run — the security tier renders all-zero panic columns (false clean) and the findings table renders empty. The 2 downstream S3s already logged against 22_attack_surface.sh and report.py have their root cause HERE. What correct looks like: 03_astgrep.sh (or 99_run_all.sh after it) invokes astgrep_summary.py, and audit.yml:88 is changed to astgrep-summary.json or the script gains a .txt emitter. Confidence: HIGH — Verification: rg wiring greps + executed the script's logic.
F-002 [S3]
Where: scripts/quality/astgrep_summary.py:30
Code:
```
    for fn in sorted(os.listdir(D)):
```
What is wrong: no existence check on D (docs/audit/reports/astgrep); on the current tree the directory does not exist, and the script crashes with a raw FileNotFoundError traceback (I ran it: exit 1, "FileNotFoundError: ... docs/audit/reports/astgrep") instead of an actionable "run 03_astgrep.sh first" (the message style its sibling 05_deadcode.sh:34 gets right). Why it matters: a tool meant to be chained behind 03_astgrep.sh gives its operator a traceback when chained out of order. Confidence: HIGH — Verification: ran it.
F-003 [S3]
Where: scripts/quality/astgrep_summary.py:34-37
Code:
```
        try:
            hits = json.load(open(os.path.join(D, fn), encoding="utf-8-sig"))
        except Exception:
            continue
```
What is wrong: corrupt or non-list rule JSON is silently dropped from the summary — empirically proven: my poison files bad-rule.json (a dict) and corrupt.json (invalid JSON) simply vanished from the counts. Why it matters: false-clean direction — a truncated ast-grep run leaves a half-written rule JSON and the rule disappears from the triage queue without a whisper. What correct looks like: count and report skipped files (or die), like 03_astgrep.sh's own not_run() convention. Confidence: HIGH — Verification: ran it with poison inputs.
F-004 [S4]
Where: scripts/quality/astgrep_summary.py:16-24
Code:
```
TEST_PATH_MARKERS = (
    "/tests/", "/tests.rs", "/test/", "/benches/", "tests.rs", "_test.rs",
    "testkit/", "tests/common/",
)
```
What is wrong: filename-suffix markers ("tests.rs", "_test.rs") match ANY file whose name ends that way — probes: 'crates/foo/src/unit_tests.rs' -> True and 'apps/cli/src/main_tests.rs' -> True, both production-path files classified TEST; conversely inline #[cfg(test)] modules in src/*.rs count as SHIPPED (path-only heuristic, no cfg(test) awareness). The split is approximate in both directions while the docstring claims it "makes that split explicit". Confidence: HIGH — Verification: ran 6 probes via direct import.
F-005 [S4]
Where: .github/workflows/audit.yml:88 (consumer side of this file's contract)
Code:
```
            sed -n '1,40p' docs/audit/reports/astgrep-summary.txt 2>/dev/null || true
```
What is wrong: the workflow's Scoreboard consumes astgrep-summary.txt, but this script only ever writes astgrep-summary.json (:70) — a permanent extension mismatch, hidden by `2>/dev/null || true`. The scoreboard can never display this script's output even if F-001 is fixed by running it. Confidence: HIGH — Verification: read both files; no producer of the .txt exists (rg over tree).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| is_test_path | 22-24 | REAL but heuristic | S4 | executed (6 probes); suffix bias + no cfg(test) awareness |
| main | 27-71 | REAL code, never invoked | S2/S3 | executed via patched copy; crash-on-missing-input (F-002), silent skip (F-003), orphaned wiring (F-001) |

## 5. Test honesty
No tests for this script anywhere in the tree. Mutation reasoning: if
is_test_path always returned True, my synthetic run would have shown
SHIPPED=0 — nothing in the repo would catch it. The quality sweep itself
never exercises the script (F-001), so even integration coverage is absent.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "production hits already split by astgrep_summary.py (test code excluded)" | scripts/quality/22_attack_surface.sh:53 | FALSE-IN-PRACTICE | the split never happens — nothing runs the script; the .json it reads is absent (A10's 22_attack_surface F-001 corroborates) |

## 7. Dead / duplicated / unreachable code
The whole script is effectively dead in the pipeline (F-001): written,
correct, committed — and invoked by no one. Partial duplication:
03_astgrep.sh's inline PYEOF summary recomputes (less informative) totals —
the summarizer was never integrated to replace it.

## 8. Dependencies & flags
Pure stdlib (json/os/sys/collections). Input contract: docs/audit/reports/
astgrep/<rule>.json in ast-grep --json hit shape ({file, range.start.line,
lines}). Output contract: docs/audit/reports/astgrep-summary.json
({"counts", "production_hits"}) — consumed by 22_attack_surface.sh:55 and
report.py:144.

## 9. Open questions for the next agent
- Fix order matters: wire the invocation (F-001) BEFORE trusting any
  22_attack_surface panic-tier numbers produced so far — they were all-zero
  for a structural reason, not because the code is clean.
- If the .txt scoreboard format is wanted, add an emitter; else fix audit.yml:88
  to point at the .json.
