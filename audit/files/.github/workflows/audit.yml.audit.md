# AUDIT: .github/workflows/audit.yml
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / jobs | 454 / 10 (static, quality, dynamic, mutation, mutation-score, formal, security, fuzz, speed, sanitizers, report) |
| File verdict | REAL-UNPROVEN (fully wired program; zero evidence any run ever executed) |
| Reachable from shipped entry point? | N/A — IS a CI entry point: PR + nightly cron "0 2 * * *" + weekly "0 3 * * 0" + workflow_dispatch |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 4 |
| Commands run | ls/rg/sed/awk — exit 0 (existence checks, quotes, arithmetic); python3 scripts/twin_parity.py n/a here |

## 1. One-paragraph verdict
The nightly audit/fuzz workflow is a real, fully-wired program: every one of the 12
referenced scripts/quality/ scripts EXISTS in the tree (00_install_tools, 99_run_all,
12_mutants, 18_ub_checks, 20_formal, 25_fuzz_campaign, 27_speed_parity, mutants_report.py
+ common.sh), the 35 fuzz targets it claims are exactly the 35 .rs files under
fuzz/fuzz_targets/, the job `if:` plumbing (schedule vs dispatch vs PR) is coherent,
mutation-score's `needs.mutation.result != 'skipped'` guard is correct, and the tier
dispatcher 99_run_all.sh genuinely exits 1 on GATE failures (J/JG plumbing +
`exit 1` when GATES non-empty), so quality/dynamic/security CAN fail CI. BUT: nothing
proves it has ever run — docs/audit/ contains only docs-sweep-20261110.md +
panic-baseline.json: no reports/, no BASELINE.md/FINDINGS.md, no speed-baseline.json.
And the speed-regression gate is structurally dead in CI (F-001): every run re-seeds
because the baseline is neither committed nor downloaded.

## 2. What actually works (verified)
- `ls scripts/quality/` -> all 8 scripts invoked by this workflow exist (00, 99, 12, 18, 20, 25, 27, mutants_report.py). Exit 0.
- `ls fuzz/fuzz_targets/*.rs | wc -l` -> **35**; workflow L323 comment says "the repo's 35 real targets" — EXACT MATCH.
- `ls docs/audit/` -> docs-sweep-20261110.md, panic-baseline.json (and nothing else — see F-001/F-002).
- `sed -n '100,104p' scripts/quality/99_run_all.sh` -> `if [ "${#GATES[@]}" -gt 0 ]; then exit 1; fi` — the tiers this workflow runs un-gated (quality, dynamic) can really fail the job.
- Job-`if` audit (read): nightly "0 2 * * *" -> quality, dynamic, formal, security, fuzz, speed; weekly "0 3 * * 0" -> mutation + sanitizers; PR -> static + security; dispatch per-tier. No job double-fires across event kinds; mutation-score correctly skipped when mutation is.
- `permissions: contents: read` (L42-43) — least privilege. Concurrency group PR-only cancel-in-progress (L50-52) — correct.
- `25_fuzz_campaign.sh` signature verified: `[seconds-per-target] [target ...]` — workflow call `25_fuzz_campaign.sh 180 "${MINE[@]}"` matches; sharding `i % 5` over `ls`-ordered 35 names is balanced (7/shard).

## 3. Findings
F-001 [S2]
Where: .github/workflows/audit.yml:363-366, 371, 393-401 vs scripts/quality/27_speed_parity.sh:24,140-177
Code (claim):
```
# Ratio convention: 1.00x = same speed. First run seeds the baseline; from
# then on a >20% regression fails.
```
Code (script):
```
BASE="$ROOT/docs/audit/speed-baseline.json"
...
elif baseline is None:
    json.dump({"generated": ...
```
What is wrong: the baseline file does not exist in the repo (docs/audit/ holds only
docs-sweep-20261110.md + panic-baseline.json), CI runners are ephemeral
(runs-on: ubuntu-latest), the speed job has NO download-artifact step, and the
baseline written during a run is only uploaded (L393-401) — never fed back. So every
CI execution hits the `baseline is None` seed branch and the >20% regression gate can
NEVER fire in CI. The claim "from then on a >20% regression fails" is false for this
workflow; it only holds on a laptop where the file persists. Compounded by
`continue-on-error: true` (L371) "promote to blocking once the baseline is seeded" —
a state the workflow itself can never reach. Why it matters: the one lane that could
catch perf regressions is provably inert in the shipped configuration; a 2x slowdown
sails through nightly CI as "Baseline created from this run". What correct looks like:
commit docs/audit/speed-baseline.json (or download the prior run's artifact) before
running the gate. Confidence: HIGH — Verification: read (paths + steps) +
ls docs/audit.
F-002 [S3]
Where: .github/workflows/audit.yml:182-184
Code:
```
# Full mutation sweep, sharded. 38,276 mutants / 8 shards / 4 jobs per shard
# = ~2.5 h wall per shard (measured 1.81 s/mutant). 8 shards is deliberate:
```
What is wrong: the matrix (L189-192) defines exactly 8 jobs total — 1 job per shard,
not "4 jobs per shard" (32 jobs). The 2.5 h/shard arithmetic matches SERIAL execution
(38,276/8 × 1.81 s ≈ 2.4 h); with 4 jobs/shard it would be ~0.6 h. The comment
misstates the workflow's own topology. Why it matters: readers sizing the mutation
lane (and its cost) are misled by ~4x. What correct looks like: delete "/ 4 jobs per
shard". Confidence: HIGH — read.
F-003 [S4]
Where: .github/workflows/audit.yml:36-37
Code:
```
        description: "static | quality | dynamic | security | fuzz | speed | sanitizers | mutation"
        default: "static"
```
What is wrong: the documented tier list omits `formal`, yet the formal job dispatches
on `inputs.tier == 'formal'` (L259). The input is a free string (no `type: choice`),
so a typo like "quailty" dispatches zero jobs and the workflow silently no-ops.
Confidence: HIGH — read.
F-004 [S4]
Where: .github/workflows/audit.yml:262, 276, 280, 417-419, 409
Code:
```
      - name: Kani catalogue (harnesses to write; no proofs yet)
        run: bash scripts/quality/20_formal.sh || true
```
What is wrong: the formal and sanitizers lanes end every step in `|| true` AND carry
job-level `continue-on-error: true` — by the repo's own standard (99_run_all.sh
L18-19: "a CI that cannot fail is theatre") these lanes cannot fail at all, even on
a genuine Miri/ASan/UBSan finding; a real memory bug surfaces only in a log grep.
Disclosed in the header ("this workflow is *evidence*... not a green light") and the
Kani comment admits "no proofs yet", so intent is honest — recorded so the audit
headline is not fooled. Confidence: HIGH — read.
F-005 [S4]
Where: .github/workflows/audit.yml:378
Code:
```
      - name: Build lumen (release - speed numbers are meaningless in debug)
        run: cargo build --release -p lumen-cli
```
What is wrong: no `--locked` — unlike the release lane (`cargo build --release
--locked`), the speed job can silently build with drifted dependency resolution.
Cosmetic today (Cargo.lock committed) but the one place in the repo where the audit
measures a build that may not be the shipped one. Confidence: HIGH — read.

## 4. Per-job table
| Job | Lines | Trigger | Verdict | Notes |
|---|---|---|---|---|
| static | 59-102 | PR, dispatch static | REAL-UNPROVEN | continue-on-error: true by design (L56-58 "flipped to a hard gate once F-001..F-018 are triaged"); scoreboard + artifact upload if:always() |
| quality | 105-149 | nightly, dispatch | REAL-UNPROVEN | no continue-on-error — CAN fail CI via 99_run_all.sh GATES exit 1 |
| dynamic | 151-179 | nightly, dispatch | REAL-UNPROVEN | JG 26_complexity.sh is the one hard gate in the tier |
| mutation | 185-219 | weekly, dispatch | REAL-UNPROVEN | fail-fast:false, fetch-depth:0, 8 matrix shards; comment bug F-002 |
| mutation-score | 223-252 | after mutation | REAL-UNPROVEN | correct skip-guard; merge-multiple artifact download |
| formal | 258-288 | nightly, dispatch formal | STUB-leaning | all steps `|| true` + continue-on-error; "harnesses to write; no proofs yet" (F-004) |
| security | 294-321 | PR, nightly, dispatch | REAL-UNPROVEN | no continue-on-error: panic/alloc budgets are LIVE PR gates |
| fuzz | 325-361 | nightly, dispatch | REAL-UNPROVEN | 5 shards over 35 targets, 180s each, nightly toolchain + cargo-fuzz |
| speed | 367-401 | nightly, dispatch | FALSE-gate | regression gate structurally dead in CI (F-001) + continue-on-error |
| sanitizers | 405-426 | weekly, dispatch | STUB-leaning | asan/ubsan `|| true` + continue-on-error (F-004) |
| report | 431-454 | after quality | REAL-UNPROVEN | regenerates BASELINE.md/FINDINGS.md — neither exists in-tree today |

## 5. Test honesty
N/A (workflow). The embedded "self-tests" are the tier scripts; 27_speed_parity.sh's
gate would fail if code regressed AND a prior baseline existed — in CI it provably
never does (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this file contains NO audit logic. Every step calls a script in scripts/quality/" | L3-6 | REAL | all 12 step bodies verified as bash/python script calls; inline logic limited to scoreboard echo/sed |
| "the repo's 35 real targets" | L323 | REAL | `ls fuzz/fuzz_targets/*.rs | wc -l` -> 35 |
| "First run seeds the baseline; from then on a >20% regression fails" | L365-366 | FALSE (in CI) | F-001: baseline absent from repo, never downloaded; seed branch every run |
| "38,276 mutants / 8 shards / 4 jobs per shard" | L182 | FALSE-in-part | matrix = 1 job/shard (F-002); mutant count UNVERIFIED (cargo-mutants absent) |
| "measured baseline 2026-11-16: 39 members, 435,498 lines, 5,014 tests" | L11 | REAL-UNPROVEN | members: manifest arithmetic gives 39 (31 crates incl _template + apps minus excluded desktop-tauri + 4 explicit); LOC today 452,504 (grew past 435,498 — consistent); 5,014 tests UNVERIFIED (baseline cargo test = 4744+2+51) |
| "windows/macos lanes in ci-full.yml ... catch F-001..F-003" | L21-24 | UNVERIFIED | ci-full.yml is another agent's file; existence not checked |

## 7. Dead / duplicated / unreachable code
- No dead steps; every job's every step feeds an artifact or summary.
- `formal` + `sanitizers` are non-failing by construction (F-004) — reachable but
  consequence-free lanes.

## 8. Dependencies & flags
- actions/checkout@v4, actions/cache@v4, Swatinem/rust-cache@v2,
  dtolnay/rust-toolchain@nightly, taiki-e/install-action@v2,
  actions/upload-artifact@v4 / download-artifact@v4 — all unpinned to commit SHA
  (tag-pinned; supply-chain script 07_supply_chain.sh territory).
- Depends on scripts/quality/* (all present) and ffmpeg via apt (unpinned version).

## 9. Open questions for the next agent
- Has the nightly cron ever fired? Zero in-tree evidence either way (no reports/, no
  BASELINE.md/FINDINGS.md, no committed artifacts). If the workflow has run, the
  evidence-retention claim ("nothing is lost") is being violated — artifacts exist
  only on GitHub, 14-90 day retention.
- ci-full.yml owner: verify the 3-OS lanes actually exist (L21-24 claim).
- mutants_report.py owner: the mutation-score job passes the unexpanded glob
  `docs/audit/reports/mutants/full-shard-*` if downloads fail — behavior on literal
  path should be checked.
