# AUDIT: .github/workflows/ci.yml
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / jobs / steps | 775 / 7 jobs (lint, gate, schema-validation, config-propagation, cargo-deny, wasm-build, tauri-check) / ~40 steps |
| File verdict | REAL (workflow with real, unpinned-by-skip gating; no continue-on-error, no paths filter) |
| Reachable from shipped entry point? | N/A — CI infrastructure; triggers on push/PR/merge_group to main (verified byte-level) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | python3 codepoint dump (triggers); rg continue-on-error/paths (absent); python3 re-run of the inline feature-matrix v0 check (exit 0); baseline citations (audit/runs/baseline/full_gate.txt, cargo-test-workspace.txt, cargo-fmt.txt) |

## 1. One-paragraph verdict — THE HOSTILE ANSWER
**CI on this tree is RED — decisively, in at least 4 of 7 jobs — and there are no
skip mechanisms hiding it.** `rg` proves the file contains NO `continue-on-error`
and NO `paths:` filter; the only `if: always()` (gate steps 2-6) is keep-going
aggregation that still fails the job. Triggers are `push`/`pull_request` on
`branches: [main]` + `merge_group` (byte-verified below — the header's claim that
the `ain]` corruption is a display artifact is TRUE; my own sed output reproduced
the swallowed `[m` and the codepoint dump proved the bytes are `91 109 97 105 110
93`). Mapping the orchestrator baseline onto this workflow's exact commands:
`lint` job step 1 runs `cargo fmt --all -- --check` → baseline exit 1 → RED; `gate`
step 3 runs `cargo test --workspace --all-features` → baseline 2 first-vertical
failures (committed transcript + final-frame.png byte drift) → RED; `gate` step 4
runs check-architecture → baseline arch FAILED → RED; `config-propagation` runs
lint-literals + check-identity → baseline 1 violation each → RED; `cargo-deny` job
runs sbom --check → baseline DRIFT → RED. The three WORST baseline failures —
feature-audit (2 missing evidence paths), parity-report (0/252), config-validator
(4 missing brand assets) — are NOT run by ANY workflow (rg over .github/workflows:
zero hits for feature-audit/parity-report/config-validator as commands), so they
cannot redden CI; CI is red anyway. Whether branch protection actually REQUIRES
this workflow for merge is UNVERIFIED (repo settings not in tree, git forbidden).

## 2. What actually works (verified)
- Trigger filter bytes: `python3` codepoint dump of line 102 -> `[32,32,32,32,98,114,97,110,99,104,101,115,58,32,91,109,97,105,110,93]` = `branches: [main]`. The header's "not corrupt on disk" claim (lines 21-30) is TRUE; the sandbox's own output renderer swallows `[m` — reproduced via sed.
- Inline feature-matrix v0 structural check (lines 284-398): re-executed the extracted Python on this tree -> `feature-matrix: 252 rows, 21 categories, schema_version 1 - v0 structural check passed`, exit 0. Step is REAL and green.
- Skip-mechanism scan: `python3` byte scan -> `has continue-on-error: False`, `has paths filter: False`; job list = lint/gate/schema-validation/config-propagation/cargo-deny/wasm-build/tauri-check.
- Toolchain assert steps (5x): real 3-way pin==inst==eff check with `exit 1` — logic read, not executed on a runner.

## 3. Findings
F-001 [S2]
Where: .github/workflows/ci.yml:284-398, 289-292
Code:
```
          # vocabulary, R4/R5/R6 conditionals. NOT covered (needs
          # tools/feature-audit): R1 category-prefix map, R2 P0 rule, R4
          # evidence resolution, R7 config cross-check, R8 inventory
```
What is wrong: the merge-blocking feature-matrix check is structure-only; it does
not resolve a single evidence path. I re-ran it: 252 rows PASS while the
orchestrator baseline feature-audit FAILURE shows 2 evidence paths MISSING and
parity-report shows 0/252 verified. Why it matters: a fabricated matrix ("verified"
+ evidence path that doesn't exist) is green in CI; the only tools that catch it
(feature-audit/parity-report) are run by nobody (rg: no workflow invokes them).
The blind spot is honestly documented here, but documentation is not enforcement.
What correct looks like: `cargo run -p xtask --locked -- feature-audit` (or the
evidence-path subset) as a CI step. Confidence: HIGH — Verification: ran the
extracted check + rg over all workflows.

F-002 [S3]
Where: .github/workflows/ci.yml:740-743
Code:
```
          # Pinned binary identical to the cargo-deny job's (G3/G4):
          # reuses the same download; see that job's pin block.
          "${DENY_BIN:-cargo-deny}" --version >/dev/null 2>&1 || cargo install cargo-deny --version 0.20.2 --locked
```
What is wrong: the comment is FALSE. This step does not reuse the SHA-256-pinned
release tarball from the cargo-deny job (lines 510-518); it falls back to
`cargo install` from crates.io — version-pinned but hash-floating, and `${DENY_BIN}`
is never defined anywhere in the workflow (dead escape hatch). This contradicts
the header's own posture claim (lines 94-96: "no floating tool installs in CI").
Why it matters: the tauri-check lane's deny results come from a different,
unverified-binary path than the main lane; supply-chain posture is inconsistent.
What correct looks like: replicate the SHA-256 download block, or set DENY_BIN to
the pinned artifact path. Confidence: HIGH — Verification: read + rg DENY_BIN
(1 occurrence).

F-003 [S3]
Where: .github/workflows/ci.yml:32-38
Code:
```
# PARITY TARGET: scripts/full_gate.sh — the *implemented* local gate. The
# `gate` job's cargo commands are the gate script's commands VERBATIM (its
# six steps: fmt #1 lives in the `lint` job here, clippy/test/arch/
# fixtures/fuzz-registry #2-#6 below), in the same order, ...
```
What is wrong: full_gate.sh now has ELEVEN steps (fmt, clippy, test, arch,
fixtures, fuzz-registry, lint-literals, check-identity, sbom, web-budget,
packaging gate — scripts/full_gate.sh:10-82). The "six steps VERBATIM, same
order" parity claim is stale/false for the current tree: steps 7-8 run in a
different job (config-propagation), 9 in cargo-deny, 10 in wasm-build, and step
11 (scripts/packaging/gate.sh) runs in NO ci.yml job (rg: only release.yml:39
mentions it). Why it matters: "CI = the local gate" is the file's central claim
and it is 10/11 with scattered placement and one step missing entirely — a
reader auditing parity from the comment alone is mislead. What correct looks
like: update the comment to the 11-step map, or add the packaging gate step.
Confidence: HIGH — Verification: read both files, rg for gate.sh in workflows.

F-004 [S3]
Where: .github/workflows/ci.yml:82-86 vs .cargo/config.toml:1-14
Code:
```
#   2. The committed .cargo/config.toml hard-codes build.target-dir to the
#      Lead sandbox path /home/z/my-project/target — on a hosted runner the
```
What is wrong: stale "CURRENT REALITY" comment — the committed .cargo/config.toml
now contains the restored `[alias] xtask = "run -p xtask --locked --"` and
deliberately NO target-dir (its own header documents the fix). Both
"reconciliations" this header presents as current reality (lines 71-86) are fixed
in the tree. Why it matters: the file carries dated claims presented as present-
tense fact. What correct looks like: past-tense/archive the comment.
Confidence: HIGH — Verification: cat .cargo/config.toml.

F-005 [S4]
Where: .github/workflows/ci.yml:219
Code:
```
        run: sudo apt-get update && sudo apt-get install -y --no-install-recommends ffmpeg
```
What is wrong: unpinned external oracle — apt resolves whatever ffmpeg the
runner image ships; differential-oracle test outcomes drift with the image.
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: .github/workflows/ci.yml:159-163
Code:
```
          curl --proto '=https' --tlsv1.2 -sSf -o /tmp/download-actionlint.bash \
            https://raw.githubusercontent.com/rhysd/actionlint/v1.7.12/scripts/download-actionlint.bash
```
What is wrong: the installer script and resulting binary are fetched over TLS
with no checksum, vs the cargo-deny SHA-256 posture claimed as uniform policy
(lines 94-96). Confidence: HIGH — Verification: read.

F-007 [S4]
Where: .github/workflows/ci.yml:142-153, 196-207, 262-273, 424-435, 556-567
Code:
```
          if [ "${pin}" != "${inst}" ] || [ "${pin}" != "${eff}" ]; then
            echo "::error::toolchain mismatch - bump the dtolnay/rust-toolchain ref in .github/workflows/ci.yml to match rust-toolchain.toml, and check for a RUSTUP_TOOLCHAIN override on the runner"
            exit 1
          fi
```
What is wrong: identical 12-line block copy-pasted into 5 jobs (5x duplication,
~60 lines); a composite action or workflow-level job template would deduplicate.
Confidence: HIGH — Verification: rg "toolchain mismatch" -c = 5.

## 4. Per-job table (mandatory; file >100 LOC)
| Job | Lines | Steps | Verdict | This tree's result | Notes |
|---|---|---|---|---|---|
| lint | 134-178 | fmt / actionlint / metadata --locked / deps-tree upload | REAL | RED (fmt exit 1: baseline full_gate.txt:29 `fmt: FAILED`; cargo-fmt.txt shows diff in crates/plugin-api/tests/testkit_wiring.rs:40) | actionlint/deny-tree steps UNVERIFIED (need runner) |
| gate | 188-246 | ffmpeg install / clippy / build+test / arch / fixtures / fuzz-registry | REAL | RED: test 2 first-vertical failures (baseline cargo-test-workspace.txt:2764-2791, committed evidence drift left!=right); arch FAILED (full_gate.txt:42, `lumen-cli`->`streaming` edge violation); fixtures+fuzz-registry PASS; clippy CLEAN | `if: always()` keep-going is honest |
| schema-validation | 254-407 | product-config test / schema-gen --check / codegen --check / fixtures --check / feature-matrix v0 | REAL | feature-matrix step GREEN (I ran it: 252 rows pass); product-config/schema-gen/codegen steps UNVERIFIED (cargo build budget) | the v0 check's R4 blind spot = F-001 |
| config-propagation | 416-473 | localization test / rebrand no-op + cmp / lint-literals / check-identity | REAL | RED: lint-literals 1 violation + check-identity 1 violation (full_gate.txt:60-66, both `apps/cli/src/host.rs:1799: identity`) | rebrand no-op step has real `cmp` byte control |
| cargo-deny | 490-528 | SHA-256 cargo-deny install / deny check / sbom --check | REAL | sbom RED (full_gate.txt:70 `sbom: FAILED` — DRIFT vs docs/security/sbom/cyclonedx.json); deny check itself UNVERIFIED | SHA-256 pin verified by reading; binary not executed |
| wasm-build | 548-670 | rustup target / canary build / size report / web-budget / upload | REAL | UNVERIFIED on runner: baseline web-budget FAILED only because wasm32-unknown-unknown std is absent in the sandbox (full_gate.txt:77 `error[E0463]: can't find crate for std`) — CI installs the target first, so this job's true color is UNKNOWN | size report has real magic-byte check (od -An -tx1 -N4 == 0061736d) |
| tauri-check | 680-775 | apt webkit / npm build / test / clippy / deny / ldd smoke | REAL-UNPROVEN | UNVERIFIED: sandbox cannot build gdk-sys (spec §13 environment limit); deny install path is F-002; `--exclude-dev` flag existence on cargo-deny 0.20.2 UNVERIFIED | ldd smoke is honest no-DISPLAY boundary |

## 5. Test honesty
No tests in this file; the workflow IS the test harness. The `Assert toolchain pin`
step would fail if the dtolnay ref drifted from rust-toolchain.toml (mutation
reasoning: change channel to 1.99.0 -> pin!=eff -> exit 1). Honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`branches: ain]` ... does NOT exist on disk — both triggers read `branches: [main]`" | ci.yml:21-30 | TRUE | codepoint dump: line 102 = `[...58,32,91,109,97,105,110,93]`; my sed reproduced the `[m` swallow |
| "gate job's cargo commands are the gate script's commands VERBATIM (its six steps ... in the same order)" | ci.yml:32-38 | FALSE (stale) | full_gate.sh has 11 steps; step 11 absent from ci.yml (F-003) |
| "no floating tool installs in CI (doc 11 §3.7 posture)" | ci.yml:94-96 | FALSE (partial) | tauri-check line 742 `cargo install cargo-deny` (F-002); actionlint script fetch (F-006) |
| "Pinned binary identical to the cargo-deny job's: reuses the same download" | ci.yml:740-741 | FALSE | code installs from crates.io, no reuse, DENY_BIN undefined (F-002) |
| "cargo xtask DOES NOT RESOLVE in a fresh clone" (current reality) | ci.yml:71-81 | STALE/FIXED | .cargo/config.toml now has `[alias] xtask = "run -p xtask --locked --"` |
| "run 43378741286 gate step 3 died without it" (build-before-test) | ci.yml:230-233 | UNVERIFIED | no git access; run ID cannot be checked from tree |

## 7. Dead / duplicated / unreachable code
- `${DENY_BIN:-cargo-deny}` — DENY_BIN defined nowhere (dead default).
- 5x duplicated toolchain-assert block (F-007).
- `CARGO_TARGET_DIR: target` env (line 126) defends against a config.toml poison that no longer exists — harmless belt-and-braces, but its motivating comment (lines 82-86) is stale (F-004).

## 8. Dependencies & flags
- Actions pinned by full SHA (checkout 3d3c42e..., dtolnay d124f86..., rust-cache 6323deb..., upload-artifact cf430e0..., setup-node 2028fbc...). Cannot ls-remote (no git) — comment's claimed re-verification UNVERIFIED.
- Toolchain pin: rust-toolchain.toml `channel = "1.99.1"` + components rustfmt/clippy — matches the dtolnay branch claim.
- No paths filter, no continue-on-error, no workflow-level `if:` skips. `permissions: contents: read`. Concurrency: per-ref with PR-only cancel-in-progress.

## 9. Open questions for the next agent
- Does the repo's branch protection actually make this workflow required? (UNVERIFIED — needs repo settings/git.)
- Does `cargo deny --manifest-path apps/desktop-tauri/Cargo.toml --exclude-dev check ...` parse on cargo-deny 0.20.2? Flag existence UNVERIFIED.
- On a real runner with wasm32 std installed, does `web-budget --check` pass? Sandbox cannot answer (E0463 artifact).
- release.yml (not in my assignment) is claimed to run the packaging gate (release.yml:39) — verify there.
