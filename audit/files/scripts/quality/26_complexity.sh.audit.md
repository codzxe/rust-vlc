# AUDIT: scripts/quality/26_complexity.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 171 / 1 python program (scrub, scan_file, main) |
| File verdict | REAL logic, UNREACHABLE-as-gate in CI (ratchet baseline never committed) |
| Reachable from shipped entry point? | NO — quality tool; wired as a GATE in scripts/quality/99_run_all.sh `dynamic` tier, itself run by .github/workflows/audit.yml:171 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | python3 (extracted program) with scratch argv -> first run: "Baseline created: HARD=13" rc=0; second run vs scratch baseline: "GATE OK ... Floor tightened to 13" rc=0; ls docs/audit/ -> only docs-sweep-20261110.md + panic-baseline.json |

## 1. One-paragraph verdict
The scanner is real and I ran its Python core read-only against the live tree: it found 13 HARD quadratic-idiom sites (e.g. `events.remove(0)` in plugin-host-wasm) and 134 REVIEW hits, and both the baseline-creation and ratchet-OK paths execute correctly. But the gate it exists to be cannot function: `docs/audit/complexity-baseline.json` is not in the repository, so in the CI lane that invokes it as a GATE (audit.yml `dynamic` tier) every run is a "first run" that creates a baseline from the current tree and exits 0. The gate cannot fail in CI — precisely the "theatre" condition the runner's own header condemns. Locally it ratchets only against an untracked file.

## 2. What actually works (verified)
— `python3 audit/scratch/W17-A04/cplx.py $PWD <scratch-base> <scratch-out>` -> HARD=13, REVIEW=134, "Baseline created: HARD=13. The ratchet starts here.", rc=0 (no repo writes; scratch paths only).
— Re-run against the scratch baseline -> "Baseline HARD=13 -> now 13. GATE OK", rc=0 — ratchet comparison logic works.
— Reported HARD sites are real code, byte-checkable: `crates/plugin-host-wasm/src/state.rs:312` `remove0` — `self.import_trace.remove(0);` (site list printed by the run).

## 3. Findings
F-001 [S2] — The ratchet baseline is not committed; the CI gate can never fail.
Where: scripts/quality/26_complexity.sh:16-17 (claim) vs docs/audit/ (reality); consumer scripts/quality/99_run_all.sh:56 (`JG 26_complexity.sh`), .github/workflows/audit.yml:171
Code:
```
#   HARD count is RATCHETED against docs/audit/complexity-baseline.json:
#   a crate may not add an O(n) shift. First run creates the baseline.
```
and
```
  JG 26_complexity.sh
```
What is wrong: `ls docs/audit/` -> only `docs-sweep-20261110.md` and `panic-baseline.json`. The baseline the ratchet needs does not exist in the repo (and `.gitignore:121` only ignores `docs/audit/reports/`, so it is not a case of intentionally untracked baselines — the panic gate's baseline IS committed). Executed proof: with the baseline absent the program takes the L139-145 first-run path, writes the baseline, exits 0. In ephemeral CI that file evaporates with the job; the gate has no memory between runs, so "JG 26_complexity.sh" cannot fail — while 99_run_all.sh's own header insists "a CI that cannot fail is theatre". Why it matters: the D-1..D-5 "no naive-complexity" directive has zero working enforcement in CI despite a gate that claims to enforce it. What correct looks like: commit complexity-baseline.json (13 HARD sites as the floor), as panic-baseline.json was. Confidence: HIGH — Verification: ran (first-run path executed), read (99_run_all.sh/audit.yml wiring).
F-002 [S4] — HARD idioms counted regardless of loop context, contra the header.
Where: scripts/quality/26_complexity.sh:11-12 vs 75-77
Code:
```
#   HARD   idioms that are O(n) by construction and inside a loop become O(n^2):
```
vs
```
        for name, rx in HARD.items():
            if rx.search(line):
                hard.append((idx, name, line.strip()[:100]))
```
What is wrong: `for output in outputs.drain(..)` (apps/cli/src/host.rs:644, a full drain — not an O(n) shift inside a loop) is counted HARD anyway; the header scopes HARD to "inside a loop". Conservative, not dangerous — the ratchet semantics are self-consistent since the baseline uses the same counting — but the doc misdescribes the rule. Confidence: HIGH — Verification: ran (site list includes non-loop drains).
F-003 [S4] — scrub() string/comment order can eat code on unbalanced quotes in comments.
Where: scripts/quality/26_complexity.sh:55-62
Code:
```
    text = re.sub(r'r#*"(?:[^"]|"(?!#))*"#*', '""', text)
    text = re.sub(r'"(?:[^"\\]|\\.)*"', '""', text)
    text = re.sub(r"//[^\n]*", "", text)
```
What is wrong: strings are stripped BEFORE comments; a lone double-quote inside a `//` comment pairs with the next quote in real code and deletes the code between, potentially hiding a HARD site (false negative -> wrongly GATE OK). The stated rationale ("a `/*` inside a string eats shipped code") protects the opposite direction only. Edge case; blast radius is a miscount, and the ratchet would then also bake the wrong count into the baseline. Confidence: MEDIUM — Verification: read (regex reasoning; no triggering instance confirmed in-tree).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| scrub | 55-62 | REAL | S4 (F-003) | order flaw noted |
| scan_file | 65-85 | REAL | — | loop-depth tracking approximate (`LOOP.match` + `{` same-line; multi-line headers missed) |
| main | 88-167 | REAL | — | executed; both paths verified |

## 5. Test honesty
No tests; I executed the program directly (both paths) — stronger than a test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "a crate may not add an O(n) shift" (ratchet enforces) | 26_complexity.sh:16-17 | FALSE-in-CI (baseline uncommitted) | F-001 execution |
| "JG ... exit code fails the tier (and therefore the CI job)" | 99_run_all.sh:19-20 | REAL plumbing, but complexity gate cannot reach it | F-001 |
| "No static tool can ... tell you its O()" | 26_complexity.sh:4-5 | honest | — |

## 7. Dead / duplicated / unreachable code
None dead; REVIEW dicts are report-only by design (documented).

## 8. Dependencies & flags
bash + python3 (via scripts/quality/common.sh $PY); writes docs/audit/reports/complexity/complexity.txt (gitignored path) and the un-committed baseline.

## 9. Open questions for the next agent
- Was complexity-baseline.json ever committed and later dropped, or was 26_complexity.sh simply never run to completion in a lane that committed its outputs? (git history is out of my lane.)
