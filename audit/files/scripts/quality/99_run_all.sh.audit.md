# AUDIT: scripts/quality/99_run_all.sh
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 105 / 2 (J reporter-runner, JG gate-runner) + 7 tier branches |
| File verdict | REAL (with one real aggregation bug in `all` mode) |
| Reachable from shipped entry point? | YES — `.github/workflows/audit.yml` runs it 5x: static (:80, PRs), quality (:127, nightly), dynamic (:171, nightly), security (:302, PR+nightly), report (:446). Not part of the release binary lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | bash -n -> OK; grep wiring in audit.yml (tier->job map, continue-on-error map); existence check of all 25 referenced scripts + report.py -> all present. NOT executed (common.sh writes to docs/audit/reports; tools heavy). |

## 1. One-paragraph verdict
The runner's core design is right and self-aware: J records reporter failures without failing the tier, JG gates fail the tier via the end-of-run `exit 1`, and the header comment explicitly calls a CI that cannot fail "theatre" — the failure-plumbing intent is real, and every one of the 25 scripts it references exists. Two defects: (1) the `all` mode runs the four tiers as separate `bash` invocations and `exit $?` — only the LAST tier's exit code propagates, so gate failures in static/security/quality are swallowed when dynamic happens to pass; (2) the PR-level `static` tier contains zero JG gates AND its audit.yml job carries `continue-on-error: true`, so the every-PR audit can never redden CI — the PR lane of this program is advisory by construction.

## 2. What actually works (verified)
— verified by `bash -n scripts/quality/99_run_all.sh` -> exit 0.
— verified by file-existence loop over all 25 referenced scripts + report.py -> all present (no dangling tier references).
— verified by grep of `.github/workflows/audit.yml`: static/quality/dynamic/security/report tiers wired at :80/:127/:171/:302/:446; quality/dynamic/security jobs are BLOCKING (no continue-on-error), static/formal/speed/sanitizers are marked `continue-on-error: true` (:63, :262, :371, :409).
— verified by read: `JG`-marked gates exist and are reachable: 26_complexity (dynamic), 24_panic_budget + 23_alloc_budget (security), 27_speed_parity (speed) — four real gates wired through the tier exit code (L101-103).

## 3. Findings
F-001 [S2]
Where: scripts/quality/99_run_all.sh:68-73
Code:
```
  all)
    bash "$HERE/99_run_all.sh" static
    bash "$HERE/99_run_all.sh" security
    bash "$HERE/99_run_all.sh" quality
    bash "$HERE/99_run_all.sh" dynamic
    exit $?
```
What is wrong: `exit $?` propagates only the dynamic tier's exit code. Gate failures recorded by the static/security/quality sub-invocations (each of which would `exit 1` on GATES) are discarded when the last tier passes. An overnight `all` run can report green while three of its four tiers had gate failures. Why it matters: the script's own header says a CI that cannot fail is theatre; this makes the aggregate `all` invocation a partial no-op gate. What correct looks like: capture each rc (`rcs+=($?)`) and exit non-zero if any sub-run failed. Confidence: HIGH — Verification: read (control flow is unambiguous). Confidence note: dynamic normally ends the chain, and its `26_complexity` gate is real, so this is not a constant-zero bug — it is a last-tier-wins bug.
F-002 [S3]
Where: scripts/quality/99_run_all.sh:28-41 (static tier) + .github/workflows/audit.yml:59-65
Code:
```
  static)
    : > "$OUT/not-run.txt"
    J 01_clippy.sh
    ...
    J 28_format_coverage.sh
```
(and audit.yml:) `continue-on-error: true`
What is wrong: the every-PR tier has zero JG gates (all 11 are reporters) and its CI job is continue-on-error — the PR-level audit can never fail a build, by double design. The tier's findings land only in the FAILED list + BASELINE/FINDINGS reports. Why it matters: reviewers may assume the PR audit is a gate; it is advisory. The honest part: the runner comment (L16-19) explains exactly this distinction, and nightly quality/dynamic/security jobs ARE blocking. What correct looks like: label the static tier as advisory in audit.yml's header (it half-does: "-> every PR" without saying non-blocking). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: scripts/quality/99_run_all.sh:29
Code:
```
    : > "$OUT/not-run.txt"
```
What is wrong: the NOT-RUN ledger is cleared only in the static tier; running `quality`/`security`/`dynamic` directly appends to a stale ledger from an earlier static run (not-run entries from other tiers intermix). Minor evidence-hygiene defect. Confidence: MEDIUM — Verification: read.
F-004 [S4]
Where: scripts/quality/99_run_all.sh:9
Code:
```
set -uo pipefail
```
What is wrong: no `-e` — intentional and REQUIRED for the J/JG plumbing (a failing reporter must not abort the sweep; rc is captured manually). Recorded as a checked non-defect, not a finding against the design. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory >100 LOC)
| Function/branch | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `J()` | 22-23 | REAL | — | runs reporter, records rc in FAILED, returns 0 — matches documented contract |
| `JG()` | 24-25 | REAL | — | records rc in FAILED+GATES; tier exits 1 at L101-103 — real gate plumbing |
| `static` branch | 28-41 | REAL | S3 | 11 reporters, zero gates (F-002); not-run.txt cleared here only (F-003) |
| `quality` branch | 42-50 | REAL | — | 7 reporters incl. `12_mutants.sh list` (list mode) |
| `dynamic` branch | 51-57 | REAL | — | 4 reporters + JG 26_complexity — blocking tier |
| `security` branch | 58-63 | REAL | — | 22_attack_surface reporter + JG 24_panic_budget, JG 23_alloc_budget — blocking tier |
| `speed` branch | 64-67 | REAL | — | JG 27_speed_parity with forwarded args; audit.yml marks it continue-on-error until baseline seeded (:371) |
| `all` branch | 68-73 | REAL | S2 | last-tier exit only (F-001) |
| `report` branch | 75-76 | REAL | — | regenerates BASELINE/FINDINGS from current reports |
| summary + exit | 83-104 | REAL | — | FAILED list printed, GATES printed + exit 1 — the honest epilogue |

## 5. Test honesty
No tests for the runner itself. Its honesty instrument is the FAILED/GATES printout plus report.py; the design comment (L16-19) accurately describes the implemented behavior (verified by reading J/JG).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "JG = run a GATE: its exit code fails the tier (and therefore the CI job)" | 99_run_all.sh:17-18 | REAL for single-tier invocations; FALSE for `all` (F-001) | code L24-25 + L101-103; `all` L73 |
| "Without this, every tier exited 0 no matter what a script reported - a CI that cannot fail is theatre" | 99_run_all.sh:18-19 | REAL (the fix is implemented for single tiers) | J/JG plumbing as read |
| audit.yml: "bash scripts/quality/99_run_all.sh <tier>" per-tier contract | .github/workflows/audit.yml:6, 80, 127, 171, 302, 446 | REAL | grep wiring verified |

## 7. Dead / duplicated / unreachable code
None. All 25 referenced scripts exist. `dynamic`/`security`/`quality` tiers reachable via nightly cron `0 2 * * *` (:31) and dispatch.

## 8. Dependencies & flags
bash + common.sh ($PY launcher, $OUT=docs/audit/reports — WRITES to the repo tree by design); report.py for BASELINE/FINDINGS regeneration.

## 9. Open questions for the next agent
- audit.yml CI is billing-blocked (phase7-exit.md §6) — none of these tiers has ever executed on a runner; all evidence is local. UNVERIFIED.
- Does `report.py` fail if no reports exist (fresh clone + `report` tier)? Not executed (writes). UNVERIFIED.
