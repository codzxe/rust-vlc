# AUDIT: .github/workflows/rust-clippy.yml
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / jobs | 55 / 1 (rust-clippy-analyze) |
| File verdict | REAL but NON-GATING — stock GitHub template SARIF reporter; the clippy step itself can never fail the workflow |
| Reachable from shipped entry point? | N/A (CI); triggers push/PR main + Sunday 22:37 cron. The real clippy merge gate is ci.yml's `gate` job ("identical clippy+test strings", ci.yml:47) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | python3 yaml parse (steps + continue-on-error confirmed True); grep ci.yml clippy wiring |

## 1. One-paragraph verdict
This is GitHub's stock "rust-clippy analyze" template committed near-verbatim: it runs clippy with `--all-features` (but no `-D warnings`, no `--all-targets`), pipes to SARIF, and uploads — with `continue-on-error: true` on the analysis step, so NO lint finding can ever fail this workflow. It is a reporter lane, not a gate; the merge gate for clippy lives in ci.yml's `gate` job. Two repo-convention violations stand out: it pins the toolchain to `stable` with `override: true`, bypassing the repo's 1.99.1 pin that every other workflow asserts, and it `cargo install`s unpinned clippy-sarif/sarif-fmt from crates.io on each run.

## 2. What actually works (verified)
— `python3 yaml.safe_load(...)` -> 5 steps parsed; `continue-on-error` on the clippy step = True (parsed from the YAML).
— The SARIF pipeline is coherent: `cargo clippy --all-features --message-format=json | clippy-sarif | tee ... | sarif-fmt` (plain-multiline run scalar folds to one pipeline — valid).
— Triggers live (push/PR main + cron Sun 22:37); uploads with `wait-for-processing: true`.

## 3. Findings
F-001 [S3] — The analysis step cannot fail the workflow; this lane cannot gate anything.
Where: .github/workflows/rust-clippy.yml:44-49
Code:
```
      - name: Run rust-clippy
        run:
          cargo clippy
          --all-features
          --message-format=json | clippy-sarif | tee rust-clippy-results.sarif | sarif-fmt
        continue-on-error: true
```
What is wrong: `continue-on-error: true` (template default) means clippy warnings/errors never fail the job; combined with no `-D warnings`, this workflow's only failure modes are infra steps (checkout, toolchain install, `cargo install`, upload). Anyone treating "rust-clippy analyze" as the lint gate is looking at a lane that structurally cannot go red from code quality. The actual gate is ci.yml's `gate` job (ci.yml:47 "gate runs the IDENTICAL clippy+test strings"). Confidence: HIGH — Verification: read + yaml parse (continue-on-error True).
F-002 [S3] — Toolchain drift: `stable` + `override: true` bypasses the repo's 1.99.1 pin.
Where: .github/workflows/rust-clippy.yml:33-39
Code:
```
      - name: Install Rust toolchain
        uses: actions-rs/toolchain@16499b5e05bf2e26879000db0c1d13f7e13fa3af #@v1
        with:
          profile: minimal
          toolchain: stable
          components: clippy
          override: true
```
What is wrong: The repo pins 1.99.1 in rust-toolchain.toml, and every other workflow asserts the pin (soak-nightly.yml:112-123 asserts rust-toolchain.toml == action == effective rustc and exits 1 on mismatch). This workflow deliberately overrides to `stable`, so its clippy findings come from a different compiler than the merge gate's — SARIF triage noise and a convention violation. Also `actions-rs/toolchain` is a deprecated/archived action. Confidence: HIGH — Verification: read + convention comparison.
F-003 [S4] — `cargo install clippy-sarif sarif-fmt` unpinned.
Where: .github/workflows/rust-clippy.yml:41-42
Code:
```
      - name: Install required cargo
        run: cargo install clippy-sarif sarif-fmt
```
What is wrong: compiles whatever the latest crates.io release is at run time — non-reproducible lane, 5+ minutes of runner compute per run, and a supply-chain surface (new upstream release changes the tool silently). Plus floating tags `actions/checkout@v4` and `upload-sarif@v3` (only the actions-rs ref is SHA-pinned — the opposite of the repo's all-pinned convention). Confidence: HIGH — Verification: read.
F-004 [S4] — No `--all-targets`: SARIF coverage ≠ gate coverage.
Where: .github/workflows/rust-clippy.yml:46-47 vs the local sweep's scope
What is wrong: `cargo clippy --all-features` covers lib/bin targets only; the repo's own sweep (scripts/quality/01_clippy.sh:9) uses `--workspace --all-targets --all-features` (minus platform exclusions). Test-code lints never reach this lane's SARIF. Confidence: HIGH — Verification: read + script comparison.

## 4. Per-job table
| Job | Lines | Can it fail? | Verdict | Notes |
|---|---|---|---|---|
| rust-clippy-analyze | 21-56 | only on infra steps (checkout/toolchain/cargo install/upload); NEVER on lint findings | REAL (SARIF reporter) | worklog "clippy green" claims refer to this lane's green + the ci.yml gate |

## 5. Test honesty
N/A (workflow). Cross-evidence: worklog.md:1714/1893/1951 record "clippy green" in the round sets — consistent with a reporter that cannot go red from findings; the green that matters is ci.yml's gate lane.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| (template text) "rust-clippy is a tool that runs a bunch of lints..." | L5-8 | boilerplate | no repo-specific claim made |
| "clippy green" (round-11/15, tag v0.1.0 set) | worklog.md:1714,1893,1951 | REAL-UNPROVEN | CI state not directly verifiable here |

## 7. Dead / duplicated / unreachable code
None; every step runs. Coverage duplication with ci.yml's gate (intentional: evidence vs gate split per audit.yml's design philosophy).

## 8. Dependencies & flags
actions/checkout@v4 (floating), actions-rs/toolchain@16499b5e (SHA-pinned, deprecated action), github/codeql-action/upload-sarif@v3 (floating), cargo install of 2 crates (unpinned).

## 9. Open questions for the next agent
- Does anyone consume the uploaded rust-clippy SARIF (code-scanning alerts UI)? Zero in-repo references to clippy-sarif beyond this file — if the sarif is never triaged, this whole lane is ritual compute.
