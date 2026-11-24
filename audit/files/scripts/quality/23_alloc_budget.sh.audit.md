# AUDIT: scripts/quality/23_alloc_budget.sh
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 136 / 7 logical blocks (sampling, run, parse, K-estimate, gate check, report, tail) |
| File verdict | SHALLOW — real measurement, gate that cannot fail |
| Reachable from shipped entry point? | YES (CI): .github/workflows/audit.yml:294-302 `security` job → `bash scripts/quality/99_run_all.sh security` → L62 `JG 23_alloc_budget.sh` |
| Findings | S0: 1 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `bash -c` exit-swallow repro (exit 0, output below); `rg` over tools/mem-audit/src/report.rs (exit 0); `rg` over .github/workflows/audit.yml + 99_run_all.sh (exit 0). Script itself NOT executed — `cargo run -p lumen-mem-audit` compiles the workspace (cargo-build-budget). |

## 1. One-paragraph verdict
The measurement half is real and thoughtful: it samples committed fixtures deterministically, runs the in-tree lumen-mem-audit instrument, and replaces the naive peak/size ratio with a median-fixed-cost K so constant parser overhead does not fake "amplification". The gate half is theatre: when the analysis finds over-threshold files the Python heredoc exits 1, but the script runs two `say` lines afterwards and exits 0 — always. The CI tier that invokes it as a `JG` gate ("its exit code fails the tier (and therefore the CI job)", 99_run_all.sh:17) is contradicted by this script; a security gate wired into `audit.yml`'s PR job that can never return non-zero.

## 2. What actually works (verified)
- Parser keys match the instrument's output format: tools/mem-audit/src/report.rs:12 defines `MEMAUDIT file=… verdict=… size=… container=… peak_B=…` — exactly what `re.findall(r"(\w+)=(\S*)", line)` extracts.
- Sampling block is genuine: extension-breadth-first deterministic walk (`out[:n]` after one-per-ext ordering), size filter `0 < getsize(p) <= 4_000_000`, excludes `target/node_modules/.git`, and handles Windows CRLF (`tr -d '\r'`).
- K-estimate design: `statistics.median` of peaks for `size <= 8192` — sound way to separate fixed open cost from scaling amplification.
- Exit-swallow proof (executed): `bash -c 'python3 -c "import sys; sys.exit(1)"; say(){ printf "%s\n" "$*"; }; say ""; say "written: x"; echo "script-exit=$?"'` → `script-exit=0`.

## 3. Findings
F-001 [S0]
Where: scripts/quality/23_alloc_budget.sh:127-135
Code:
```
if bad:
    print("\nOVER THRESHOLD:")
    for r in bad:
        print("  %.2fx  %s" % (r["excess"], r["file"]))
    sys.exit(1)
PYEOF

say ""
say "written: $OUT/alloc/alloc-budget.txt"
```
What is wrong: the Python gate check exits 1 on violations, but nothing captures it; the script continues to `say ""`/`say "written: …"` and its exit code is that of the last `say` — 0. Why it matters: 99_run_all.sh:24-25 `JG` treats this script's exit code as a GATE that "fails the tier (and therefore the CI job)", and audit.yml:294-302 runs the security tier on every PR + nightly. A confirmed allocation-amplification violation therefore prints `OVER THRESHOLD` into a log and the CI job still passes. The gate cannot fail; the JG comment is false for this script. What correct looks like: capture `rc=$?` after the heredoc (and after `cargo run`), `exit $rc` at the end. Confidence: HIGH — Verification: ran (bash -c repro above) + read.

F-002 [S2]
Where: scripts/quality/23_alloc_budget.sh:64-65
Code:
```
cargo run -q -p lumen-mem-audit -- "${FILES[@]}" > "$RAW" 2>&1
say "mem-audit exit=$?"
```
What is wrong: the instrument's exit code is printed and discarded. If `cargo run` fails (compile error, tool panic), `$RAW` contains the error text, zero `MEMAUDIT` lines parse, `rows == []`, `bad == []`, Python exits 0, script exits 0. Why it matters: the gate reports "0 files" and passes green on total instrument failure — a broken instrument is indistinguishable from a compliant codebase. What correct looks like: check the exit code, and fail (or not_run with a loud reason) when the raw file has zero MEMAUDIT rows while `${#FILES[@]} > 0`. Confidence: HIGH — Verification: read.

F-003 [S3]
Where: scripts/quality/23_alloc_budget.sh:24
Code:
```
if ! need cargo; then exit 0; fi
```
What is wrong: when cargo is absent the script exits 0. `not_run` does append a line to `docs/audit/reports/not-run.txt` (common.sh:30-34), so it is recorded — but as far as the `JG` gate/CI exit code is concerned, a missing tool is a passing gate. A security gate that silently "passes" when its engine is missing is a rubber stamp by omission. What correct looks like: a missing engine on a GATE tier should fail the tier or at minimum force the tier result to "not run". Confidence: HIGH — Verification: read (common.sh:36-43 need → returns 1 only records).

F-004 [S4]
Where: scripts/quality/23_alloc_budget.sh:92-93
Code:
```
small = [r for r in rows if r["size"] <= 8192]
K = int(statistics.median([r["peak"] for r in small])) if small else 0
```
What is wrong: if the sampled fixture set contains no file ≤ 8 KB, K=0 and `excess` silently degenerates into the naive `peak/size` ratio the report itself calls "not the DoS metric" (L104) — while still printing "Measured K (fixed open cost) = 0 B". No guard, no warning when `small == []`. Confidence: MEDIUM — Verification: read.

F-005 [S4]
Where: scripts/quality/23_alloc_budget.sh:75 and mem-audit's error format
Code:
```
kv = dict(re.findall(r"(\w+)=(\S*)", line))
```
What is wrong: `error=<verbatim to end of line>` (report.rs:12) contains spaces; `(\S*)` truncates the error value at the first space, and `kv` dict-overwrites duplicate keys (one MEMAUDIT line per file per rep — later reps silently replace earlier ones in the report table). Cosmetic/report-fidelity only. Confidence: HIGH — Verification: read.

## 4. Per-function table (>100 LOC, script blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header/env (set -uo, source common) | 1-24 | REAL | — | `set -uo pipefail`, no `-e` by design (common.sh:4) |
| fixture sampling (python) | 26-60 | REAL | — | deterministic, breadth-first, ≤4MB, CRLF handled |
| instrument run | 63-65 | SHALLOW | S2 | exit code printed, not enforced |
| MEMAUDIT parse | 67-85 | REAL | — | keys match report.rs:12; dup-key overwrite S4 |
| K estimate + excess | 87-99 | REAL | S4 edge | K=0 fallback degenerates metric |
| report writer | 101-126 | REAL | — | honest methodological preamble |
| gate decision + tail | 127-136 | STUB as gate | S0 | exit 1 swallowed by trailing `say`s |

## 5. Test honesty
No tests exist for this script (no self-test, no fixture asserting the OVER THRESHOLD path exits non-zero). Mutation reasoning: if the entire excess computation were replaced with `rows=[]`, the script would still exit 0 and write a valid-looking report — nothing would catch it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "JG = run a GATE: its exit code fails the tier (and therefore the CI job)" | 99_run_all.sh:17 | **FALSE for this script** | bash -c repro: python exit 1 → trailing `say` → `script-exit=0`; script has no other non-zero exit path except `not_run`/usage |
| "This turns that instrument into a RATIO GATE" (23_alloc_budget.sh:7-8) | the script itself | **SHALLOW** | the ratio is computed and reported; the GATE half (exit code) never fires |

## 7. Dead / duplicated / unreachable code
None; every line executes on the security tier path.

## 8. Dependencies & flags
Depends on: `cargo`, workspace tool `lumen-mem-audit` (exists: tools/mem-audit), `$PY` (validated in common.sh:17-23), `mapfile` (bash ≥4 — Git-Bash OK). Env knobs: `ALLOC_RATIO_MAX` (default 64), arg1 sample count (default 60). No feature flags involved.

## 9. Open questions for the next agent
- 24_panic_budget.sh is also a `JG` gate — same tail pattern? If yes, the whole security tier's gate plumbing is inert (S0 cluster).
- Did any committed `docs/audit/reports/alloc/alloc-budget.txt` ever show "over threshold" while CI stayed green? Check the reports dir.
