# AUDIT: .github/workflows/ci-full.yml
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / jobs | 169 / 3 jobs (check, clippy, test) × 3-OS matrix, fail-fast: false, test `needs: [check]` |
| File verdict | REAL (honest non-blocking nightly matrix; one latent defect inherited-from-history in the test job) |
| Reachable from shipped entry point? | N/A — CI infra; triggers = schedule (cron 17 3 * * *) + workflow_dispatch ONLY (parsed) — never runs on PR/push, by design |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 yaml parse (triggers/jobs/needs); byte scan (no continue-on-error / no paths filter); rg cargo build (absent) |

## 1. One-paragraph verdict
This workflow CANNOT gate merges and does not pretend to: parsed triggers are
`schedule` + `workflow_dispatch` only (the pull_request/push mentions in the
header are the promotion-path plan, not live triggers). Structure is clean —
3 jobs x 3 OS with fail-fast:false (one OS failing cannot hide the others),
test gated behind check (cheap-first ordering), no continue-on-error, no paths
filter, least-privilege permissions, pinned actions byte-identical to ci.yml,
and the same toolchain 3-way assert per job. The command strings for clippy
and test ARE identical to ci.yml's gate steps as the header claims. ONE REAL
latent defect: the `test` job omits the `cargo build --workspace
--all-features --locked` pre-step that ci.yml added specifically because
"first-vertical's evidence suite shells out to target/debug/lumen — run
43378741286 gate step 3 died without it" (ci.yml:227-233). ci-full's test
matrix will hit that same documented failure mode on first-vertical. The
"CURRENT REALITY" comment about .cargo/config.toml is stale (same as ci.yml:
the alias is restored and no target-dir is committed). Given the baseline's
2 first-vertical failures, the nightly test matrix would be red on ALL THREE
OSes anyway.

## 2. What actually works (verified)
- `python3 yaml.safe_load` -> `on:` = schedule (cron "17 3 * * *") + workflow_dispatch ONLY; jobs check/clippy/test; `test.needs=[check]`; matrices [ubuntu-latest, windows-latest, macos-latest]; `fail-fast: false` on all three.
- Byte scan: `continue-on-error: False`, `paths: False`.
- Parity claim: clippy string `cargo clippy --workspace --all-targets --all-features -- -D warnings` (line 136) == ci.yml:223; test string `cargo test --workspace --all-features` (line 169) == ci.yml:234. IDENTICAL as claimed (ci-full.yml:14-17).
- Action pins: checkout@3d3c42e…, dtolnay@d124f86f…, rust-cache@6323deb… — byte-identical to ci.yml's pins (lines 46-48 vs ci.yml:90-92).

## 3. Findings
F-001 [S3]
Where: .github/workflows/ci-full.yml:151-169 (test job steps) vs ci.yml:225-234
Code:
```
      - name: cargo test — incl. scenarios_b + soak (doc 10 §2.1 #4)
        run: cargo test --workspace --all-features
```
What is wrong: no `cargo build --workspace --all-features --locked` before
the test run. ci.yml added exactly that pre-step after a live failure:
"first-vertical's evidence suite shells out to target/debug/lumen — the
failure text says exactly this. ... run 43378741286 gate step 3 died without
it" (ci.yml:227-233). The header of this file claims gate/CI parity
("IDENTICAL ... strings"), but the JOB SHAPE is not identical — the build
ordering rule that ci.yml learned the hard way is missing here, so the
first-vertical evidence suite can die on the 3-OS nightly exactly the way the
gate job once did. Why it matters: nightly 3-OS signal polluted by a known,
already-fixed-elsewhere failure mode; three red OS lanes for one avoidable
reason. What correct looks like: the same two-line build+test block ci.yml's
gate step 3 uses. Confidence: MEDIUM — Verification: read + rg (no `cargo
build` anywhere in this file); the actual runner behavior is UNVERIFIED.
F-002 [S4]
Where: .github/workflows/ci-full.yml:40-43
Code:
```
# "CURRENT REALITY" RECONCILIATION (see ci.yml header for the full story):
# the committed .cargo/config.toml hard-codes build.target-dir to the Lead
# sandbox path /home/z/my-project/target (and lost its xtask alias) —
```
What is wrong: stale — .cargo/config.toml now carries the restored `[alias]
xtask = "run -p xtask --locked --"` and deliberately NO target-dir. Same
comment-rot as ci.yml:82-86. Confidence: HIGH — Verification: cat .cargo/config.toml.
F-003 [S4]
Where: .github/workflows/ci-full.yml:93-104, 122-133, 155-166
Code:
```
          pin="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
```
What is wrong: the 12-line toolchain-assert block triplicated across the three
jobs (same duplication family as ci.yml's 5x). Confidence: HIGH — Verification:
rg "toolchain mismatch" -c = 3.

## 4. Per-job table (mandatory; file >100 LOC)
| Job | Lines | Steps | Verdict | This tree's expected result | Notes |
|---|---|---|---|---|---|
| check | 80-107 | checkout/toolchain-assert/cache/cargo check | REAL | GREEN (workspace check compiles; baseline clippy CLEAN implies compile health) — UNVERIFIED on runners | -- |
| clippy | 109-136 | + cargo clippy -D warnings | REAL | GREEN per baseline clippy CLEAN (ubuntu); windows/macos UNVERIFIED | identical string to ci.yml gate step 2 |
| test | 138-169 | + cargo test --workspace --all-features | REAL | RED: the 2 first-vertical failures reproduce anywhere (committed-evidence drift); plus F-001's missing-binary risk | needs: [check] |

## 5. Test honesty
N/A (workflow). The header honestly pre-declares "the windows/macos lanes have
NEVER executed on real runners" (:28-33) — a limitation claim that is
consistent with the sandbox-validated status.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NON-BLOCKING ... nightly + manual dispatch" | :1,7,20,24 | TRUE | parsed triggers: schedule + workflow_dispatch only |
| "gate/CI parity: IDENTICAL clippy and test strings" | :13-17 | TRUE for strings, FALSE for job shape (F-001) | byte compare of the two command lines |
| "windows/macos lanes have NEVER executed" | :28-31 | UNVERIFIED | run history needs repo access |
| ".cargo/config.toml hard-codes build.target-dir ... lost its xtask alias" | :40-43 | STALE/FALSE-now | config.toml has alias restored, no target-dir |
| "do NOT add CI-Full jobs to required checks — a skipped check cannot block" | :24-26 | TRUE | it never runs on PRs, so it cannot block even if required-checks misconfigured... (a skipped run of a required check = neutral/skip, not block) |

## 7. Dead / duplicated / unreachable code
- Triplicated toolchain assert (F-003). No dead steps; every step runs something real.

## 8. Dependencies & flags
Same pins as ci.yml (verified byte-identical). `CARGO_TARGET_DIR: target` env
defends against a config.toml poison that no longer exists (comment-stale but
harmless). No external tool downloads (no actionlint/cargo-deny steps here).

## 9. Open questions for the next agent
- Has ci-full's nightly EVER run green on windows/macos? Header says record
  first three green runs in phase3-exit.md §4 gap 1 — check that doc when
  auditing it.
- Does `cargo test --workspace --all-features` alone build target/debug/lumen
  on a cold runner? ci.yml's run-43378741286 evidence says NO — confirm F-001
  at first 3-OS observation.
