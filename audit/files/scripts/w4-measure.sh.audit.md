# AUDIT: scripts/w4-measure.sh
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 9 / 2 blocks (harness run, speed bench) |
| File verdict | REAL (one-off battery, outputs committed) — machine-bound, cannot fail |
| Reachable from shipped entry point? | NO — orphan: no CI job, script, or doc invokes it (`rg -ln "w4-measure"` over docs/scripts/.github → only this file's own tree) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `bash -n` (exit 0); `ls`/`rg` over tools/corpus, docs/corpus, and the tools' source (exit 0). Script NOT executed — it would `cd /home/z/my-project` and drive heavy corpus runs (load discipline). |

## 1. One-paragraph verdict
A real wave-4 measurement battery whose committed outputs exist and parameter-match the script exactly (three `report-20261022-*-skip-suites.md` harness reports + `speed-baseline-20261022.md`, 36 files / best-of-2 / ffmpeg-only — all flags line up). Its two disqualifying properties: the hardcoded `cd /home/z/my-project` makes the script measure whatever tree lives at that absolute sandbox path — not the repo the script is committed in — and with no `set -e` plus an unconditional `MEASURE_DONE` echo it always exits 0, so on any machine lacking that path it silently "completes" while having run nothing.

## 2. What actually works (verified)
- Outputs committed and parameter-consistent: docs/corpus/speed-baseline-20261022.md header "corpus: `corpus-big` | files: 36 | best-of-2 | runners: ffmpeg" matches `--corpus corpus-big --count 36 --repeat 2`; speed_bench.py:162-165 generates exactly that header shape ("# Speed-parity benchmark (decode-to-null)" / `f"best-of-{args.repeat} | runners: …"`).
- The harness half's outputs also committed: docs/corpus/report-20261022-{1557,1610,2002}-skip-suites.md, whose "--skip-suites" naming run_harness.py:33 documents ("`--skip-suites` run writes *-skip-suites names instead").
- Both invoked tools exist: tools/corpus/run_harness.py, tools/corpus/speed_bench.py.
- The reports are honest about their gaps: report-20261022-2002-skip-suites.md:6 "mode: **--skip-suites** — the Rust suites were NOT driven (oracle-side validation run)" and :7 "suite exit codes: probe=skipped demux=skipped mjpeg=skipped pcm=skipped (skipped — not driven)".

## 3. Findings
F-001 [S3]
Where: scripts/w4-measure.sh:3
Code:
```
cd /home/z/my-project
```
What is wrong: an absolute, machine-specific path — the battery measures whatever repo is at /home/z/my-project, not the repository this script is committed in (on this sandbox both exist; on any other clone the `cd` fails). Why it matters: committed repo scripts must be tree-relative (`cd "$(dirname "$0")/.."`); as written, evidence gathered "for the repo" may have been measured against a sibling worktree of a different branch — unverifiable after the fact. What correct looks like: ROOT-relative resolution. Confidence: HIGH — Verification: read + `ls /home/z/my-project/tools/corpus` (a different tree exists there).

F-002 [S4]
Where: scripts/w4-measure.sh:1-8
Code:
```
#!/bin/bash
# W4 measurement battery: harness oracle report + speed baseline on corpus-big
cd /home/z/my-project
```
What is wrong: no `set -e/-u/-o pipefail` anywhere; both python invocations are piped to `tail`; the final `echo "MEASURE_DONE"` makes the exit code 0 unconditionally. What correct looks like: `set -euo pipefail` + per-step rc capture; MEASURE_DONE only on success. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: scripts/w4-measure.sh:7
Code:
```
python3 tools/corpus/speed_bench.py --corpus corpus-big --count 36 --stride 7 --repeat 2 --md docs/corpus/speed-baseline-20261022.md --json /tmp/speed.json 2>&1 | tail -45
```
What is wrong: the JSON half of the measurement is written to `/tmp/speed.json` — volatile, never committed; only the MD half survives. Half the battery's data is discard-by-design. Confidence: HIGH — Verification: read + `ls /tmp/speed.json` not checkable post-hoc (volatile by definition).

F-004 [S4]
Where: repo-wide
What is wrong: orphan script — nothing in CI, scripts/, or docs/ references w4-measure.sh; it is archived wave-4 tooling with no owner and no re-run path (its absolute-path defect guarantees it cannot be re-run elsewhere). Confidence: HIGH — Verification: `rg -ln "w4-measure"` → only the script itself.

## 4. Per-function table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cd + harness | 3-5 | REAL | S3 | absolute path; output committed |
| speed bench | 6-7 | REAL | S4 | output committed; JSON to /tmp |
| MEASURE_DONE | 8 | rubber | S4 | unconditional success marker |

## 5. Test honesty
No tests. Mutation reasoning: replacing both python invocations with `true` would still print MEASURE_DONE — the script's only consumer signal is a constant.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "W4 measurement battery: harness oracle report + speed baseline on corpus-big" | :2 | REAL | committed outputs parameter-match (Section 2) |
| "SPEED BENCH (ffmpeg baseline, 36-file stratified sample)" | :6 | REAL | speed-baseline-20261022.md: files 36, runners: ffmpeg; ratio-vs-ffmpeg column honestly "-" (no lumen side yet) |

## 7. Dead / duplicated / unreachable code
The script itself is unreachable dead tooling (no invokers) — retained as the provenance record for the committed corpus reports; that is a legitimate archival purpose, but it cannot serve that purpose off the original machine (F-001).

## 8. Dependencies & flags
python3, tools/corpus/{run_harness,speed_bench}.py, corpus-big corpus (data, not in scripts/), ffmpeg as oracle/baseline runner. Flags: --skip-suites, --count 36 --stride 7 --repeat 2.

## 9. Open questions for next agent
- Which tree did the 2026-10-22 measurements actually run against (/home/z/my-project's checkout vs a worktree)? Unverifiable from the committed artifacts — the reports carry corpus seeds but no commit hash.
- If the speed-parity story matures (27_speed_parity.sh is a JG gate), re-baseline from a tree-relative script; this file should then be deleted or fixed.
