# AUDIT: docs/gates/logs/rebrand-acceptance-4h2.log (DATA AUDIT — log)
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines | 220 |
| File verdict | REAL-UNPROVEN (self-consistent run log; every repo-side cross-check passes; test execution not re-runnable here — no cargo on sandbox PATH) |
| Reachable from shipped entry point? | N/A (gate log; consumer: card 4.H2 / doc 07 §8.3) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg/sed cross-checks (test names, files, counts); arithmetic verification of all test tallies |

## 1. One-paragraph verdict
A well-formed acceptance log whose numbers add up and whose repo-side claims
all verify: the 7 rebrand_host test names exist verbatim in
tests/rebrand/rebrand_acceptance.rs (6 `surface_*` + `rebrand_acceptance_report`
= 7, exactly the run count); the vitest seam file exists; the BLOCKED(wiring)
disclosure is true (apps/cli/src/help.rs:101,113 hardcodes `lumen {}` / "The
Lumen command-line player" with the [PROPOSAL: identity wiring pending]
markers); the nova.toml fixture exists. Tallies: 7 + 108 (product-config
suites: 41+27+1+15+7+17+0) + 142 xtask-family (111+12+4+15) + 1 vitest, with
the 3 ignored tests each carrying a written reason pointing at the real proof.
Two stale-relative-to-HEAD notes: (1) the hygiene section claims a pre-existing
clippy `expect_used` failure at tools/xtask/src/cmd/visual.rs:91 — current HEAD
has no `expect(` in that file (the wip-branch failure has since been fixed;
consistent with the orchestrator baseline's clippy CLEAN); (2) the log's
"2 BLOCKED" posture remains accurate (the CLI banner is still hardcoded —
verified), but the log records it at a 2026-10-24 wip commit, so consumers
should not treat it as HEAD state.

## 2. What actually works (verified)
- `rg -c 'fn surface_' tests/rebrand/rebrand_acceptance.rs` -> 6; `rg -n 'fn rebrand_acceptance_report'` -> :224; 6+1 = 7 = "running 7 tests ... 7 passed". Test names in log (surface_about_facts_render_nova :162, surface_window_title_resolves_nova :104) exist verbatim.
- Arithmetic: Section 2 = 41+27+1+15+7+17+0 = 108 all-ok; Section 3 = 111+12+4+(12 passed+3 ignored of 15) — all sums check out; 0 failed anywhere in the log.
- `ls ui/src/screens/settings/areas/updates-about.nova.test.tsx` -> exists (the "1 passed" vitest seam is a real file).
- BLOCKED(wiring) honesty: `sed -n '99,113p' apps/cli/src/help.rs` -> hardcoded `lumen {}` banner + "The Lumen command-line player" with `lumen-allow-literal` markers and the [PROPOSAL: identity wiring pending] comment — the log's reason for BLOCKED is the truth.
- `ls tests/rebrand/nova.toml` -> exists.

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/rebrand-acceptance-4h2.log:213-215
Code:
```
PRE-EXISTING (not this card, file owned by 4.H1): cargo clippy -p xtask --all-targets fails on expect_used in tools/xtask/src/cmd/visual.rs:91 (test mod) — observed, left untouched.
```
What is wrong: stale relative to HEAD — `rg 'expect\(' tools/xtask/src/cmd/visual.rs`
returns zero matches today, so the claimed failure does not exist at HEAD
(fixed since the wip-branch run; the orchestrator baseline records clippy
CLEAN). The log was honest for its commit; a consumer reading it as current
state inherits a wrong hygiene fact. What correct looks like: gate logs should
carry their commit (they do: be4e4c6+dirty) — consumers must date-check; no
repo defect. Confidence: HIGH — Verification: rg at HEAD.
F-002 [S4]
Where: docs/gates/logs/rebrand-acceptance-4h2.log:1-2
Code:
```
# Run: 2026-10-24T00:02:29Z @ be4e4c6+dirty, branch wip/p4-h2
```
What is wrong: the log's product-state claims (BLOCKED rows) are anchored to a
dirty wip branch; the current product.toml identity is Lumen 0.1.3 and the CLI
banner is still hardcoded — the wiring BLOCKED is still true at HEAD, but the
log alone cannot prove that. UNVERIFIED: re-execution of the suites (no cargo
on this sandbox's PATH — environment limit, not a repo defect).
Confidence: HIGH — Verification: read + rg.

## 4. Data integrity checks
- All test-result lines parse as libtest output; every "test result: ok"
  line has 0 failed; the summary "7 PASS, 2 BLOCKED (reasons above), 0 FAIL"
  matches the per-check table (9 rows: 7 PASS + 2 BLOCKED).
- The 3 ignored xtask tests carry inline reasons naming the substituting proof
  (unit-tier test + vitest seam + BLOCKED register) — this is the honest form
  of #[ignore] the Wave-0 baseline (66 ignores) asks to see.
- No exit-code line for the overall gate is present in the log (the gate is
  the cargo test exit, 0 by the pasted results) — acceptable; the pasted
  summaries carry the verdict.

## 5. Consumer claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "7 PASS, 2 BLOCKED (reasons above), 0 FAIL" | :22 | REAL-UNPROVEN | counts internally consistent; test names/files exist; execution not re-run (no cargo) |
| "cli-identity(host-wiring) BLOCKED ... help.rs v0 hardcodes its banner" | :20 | REAL (still true at HEAD) | apps/cli/src/help.rs:101,113 |
| "installer-build BLOCKED(infra) no bundler/toolchain in this sandbox" | :19 | REAL-UNPROVEN | consistent with desktop-tauri gdk-sys sandbox failure in the Wave-0 baseline |
| "41 passed ... 17 passed" product-config regression guard | :37-49 | REAL-UNPROVEN | self-consistent; suite exists (product-config crate) |
| vitest "1 passed (1)" | :203-207 | REAL-UNPROVEN | file exists; npm not run here |
