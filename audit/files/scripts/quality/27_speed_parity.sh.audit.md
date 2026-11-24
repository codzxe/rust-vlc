# AUDIT: scripts/quality/27_speed_parity.sh
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / functions | 201 / 5 stages (arg-parse, corpus discovery, per-corpus bench, aggregator python, exit plumbing) |
| File verdict | REAL-UNPROVEN (logic reviewed; gate plumbing verified vs 99_run_all/audit.yml; no ffmpeg/cargo/lumen build in this sandbox to execute) |
| Reachable from shipped entry point? | YES (CI: audit.yml:383 speed job; scripts/quality/99_run_all.sh:66 speed tier as a JG GATE) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | sed/rg quotes + callers — exit 0; execution of the script itself impossible here (no ffmpeg, no cargo) |

## 1. One-paragraph verdict
A real, carefully-built harness: discovers EVERY fixture directory holding media
(python os.walk over the repo), benches each corpus via `tools/corpus/speed_bench.py`
with repeat/median math in the embedded aggregator, classifies every
corpus::class by ratio (ffmpeg/lumen), writes a human table, and exits 1 only on
REGRESSED (vs baseline) or BELOW PARITY (floor). The ratchet baseline design
(keep best-ever ratio per class, only rewrite when no fails) is sound — for a
persistent filesystem. Its two real weaknesses: (a) in CI the baseline can never
persist (fresh checkout + file absent from repo + no download step), so the gate is
structurally dead there (S2, filed with audit.yml F-001); (b) total failure is
GREEN: if lumen decodes nothing, every class lands in `dnf`, `results` is empty,
"Nothing comparable" is printed and the script exits 0 (S3).

## 2. What actually works (verified)
- Callers: `99_run_all.sh:66` `JG 27_speed_parity.sh "${@:2}"` (a GATE — failures fail the tier, which exits 1 via the GATES array) and `audit.yml:383` (speed job).
- Exit plumbing: `sys.exit(1 if fails else 0)` -> `rc=$?` -> `exit "$rc"` (L194-200) — gate failures propagate; quoted exit, no swallowed rc.
- Corpus discovery (L48-64) prunes target/node_modules/.git; extension set covers the tree's fixture formats; verified against `ls docs/audit/` (no speed/ dir exists — the lane has never produced committed output here).
- Baseline ratchet (L183-189): merges prior classes, raises floor only where faster, never rewrites on a failing run — a regression can't launder itself into a new baseline.
- `need ffmpeg` / `need "$PY"` / empty-corpora / empty-JSON all `not_run` + `exit 0` (honest, recorded in docs/audit/reports/not-run.txt).

## 3. Findings
F-001 [S2]
Where: scripts/quality/27_speed_parity.sh:24,140-177
Code:
```
BASE="$ROOT/docs/audit/speed-baseline.json"
...
try:
    baseline = json.load(open(base_path, encoding="utf-8"))
except (OSError, ValueError):
    baseline = None
prior = (baseline or {}).get("classes", {})
```
What is wrong: the gate's memory lives in a file that (i) does not exist in the repo
(`ls docs/audit/` -> docs-sweep-20261110.md, panic-baseline.json only), (ii) is
written fresh on every CI run (L172-177 seed branch) because runners are ephemeral
and the workflow neither commits nor downloads the prior baseline. So "a >20%
regression fails" (the file's headline claim, L12-13) can only ever fire on a
developer laptop where the file persists — never in the CI lane that the audit
workflow defines for it. Why it matters: this is the repo's only performance
regression gate and its CI wiring makes it inert. What correct looks like: commit
the baseline (docs/audit/speed-baseline.json checked in) or have audit.yml's speed
job download the previous run's artifact before running. Confidence: HIGH —
Verification: read + ls (cross-file with audit.yml F-001).
F-002 [S3]
Where: scripts/quality/27_speed_parity.sh:121-131,164-171
Code:
```
for cls, row in agg.items():
    if not row["ffmpeg"] or not row["lumen"]:
        dnf.append(cls)
        continue
...
if not results:
    L.append("**Nothing comparable.** Every lumen run failed or decoded nothing.")
```
What is wrong: every class where LUMEN fails to decode (ffmpeg succeeds, lumen
produces no wall_best) is DNF — informational only; and if ALL classes are DNF,
`results` is empty, `fails` stays empty, exit code 0. A build whose decoder
regressed to decoding NOTHING passes the speed gate green (with a prose line no
machine consumer reads). Why it matters: "never had teeth" was the complaint this
script exists to fix (L5-6); total decode failure is exactly the scenario with the
most teeth needed. What correct looks like: fail (or at minimum count DNF classes
as failures when nonzero and results empty). Confidence: HIGH — read.
F-003 [S4]
Where: scripts/quality/27_speed_parity.sh:26-34
Code:
```
    --count)  COUNT="$2"; shift 2 ;;
    --parity) PARITY="$2"; shift 2 ;;
    -*)       shift ;;
    *)        CORPORA+=("$1"); shift ;;
```
What is wrong: unknown `-`-flags are silently discarded (`-*) shift`) — a typo like
`--Count` eats the flag and its value becomes a corpus path; `--count` as the last
arg dies on unset `$2` under set -u (loud, but cryptic). Confidence: HIGH — read.
F-004 [S4]
Where: scripts/quality/27_speed_parity.sh:36,80-83
Code:
```
LUMEN_CMD="${LUMEN_CMD:-cargo run -q --release -p lumen-cli -- decode-bench}"
...
  $PY "$ROOT/tools/corpus/speed_bench.py" \
      --corpus "$c" --count "$COUNT" --repeat 3 \
      --runner "lumen=$LUMEN_CMD {file}"
```
What is wrong: the default runner recompiles/checks via `cargo run -q --release`
per invocation — fine locally, but the CI job sets LUMEN_CMD to the prebuilt binary
(audit.yml:380-381), so the default is only a laptop convenience; and `$LUMEN_CMD`
containing `=` beyond the first would still split correctly (name=cmd split at
first '='), but a value containing `{file}` collides with the template placeholder.
Cosmetic given current usage. Confidence: HIGH — read.

## 4. Per-function table
| Stage | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| arg-parse | 26-36 | REAL | S4 (F-003) | set -u kills missing option values; unknown flags swallowed |
| corpus discovery | 47-71 | REAL | — | python os.walk, pruned dirs; prints the discovered list |
| per-corpus bench | 73-96 | REAL | — | `--repeat 3` + per-corpus JSON+MD sidecars; "no result (exit=...)" honest logging; F-002's data source |
| aggregator (python) | 98-195 | REAL | — | median per class, DNF classification, ratchet baseline merge, REGRESSED/BELOW-PARITY fails; F-001/F-002 |
| exit plumbing | 196-201 | REAL | — | rc propagated verbatim to JG gate |

## 5. Test honesty
N/A (gate script). Mutation reasoning: deleting the `if b and ratio < b*(1-regress)`
test (L154) makes REGRESSED impossible — the gate would be toothless and nothing
would catch that except code review; deleting `sys.exit(1 if fails...)` is caught by
99_run_all.sh's JG contract only in a real CI run — which is exactly the lane that
cannot reach a `fails` state today (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "BASELINE: docs/audit/speed-baseline.json remembers the ratio per class, so a regression is detectable" | L10-12 | SHALLOW in CI | file absent from repo + never re-read in CI (F-001); locally true |
| "GATES: fail on >SPEED_REGRESS (20%) regression" | L12-13 | REAL-UNPROVEN | logic verified by reading; never observed failing (no baseline ever existed) |
| "promote to blocking once the baseline is seeded" (audit.yml:371) | audit.yml | FALSE-in-CI | baseline cannot become seeded in CI (F-001); the promotion precondition is unreachable there |
| "runs every fixture directory that contains media" | L8-9 | REAL | discovery code verified (L48-64) |

## 7. Dead / duplicated / unreachable code
None dead. The `--parity` floor path is exercised only via workflow_dispatch with a
nonzero parity input (default "0") — reachable, rarely used.

## 8. Dependencies & flags
tools/corpus/speed_bench.py (exists), ffmpeg + python (both absent in audit sandbox),
LUMEN_CMD env (audit.yml sets the prebuilt binary). SPEED_REGRESS env default 0.20.

## 9. Open questions for the next agent
- Who seeds docs/audit/speed-baseline.json? If nobody, F-001 is permanent. If a
  laptop run seeds it, commit it.
- tools/corpus/speed_bench.py owner: confirm the `--runner` template handles paths
  with spaces (shlex vs naive split) — my read shows split("=", 1) then shell
  template; the file is another agent's assignment.
