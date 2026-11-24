# AUDIT: .github/workflows/codeql.yml
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / jobs | 105 / 1 (analyze, 4-language matrix) |
| File verdict | REAL — ACTIVE workflow (live triggers; per worklog it has run green), but stock GitHub boilerplate with floating action tags |
| Reachable from shipped entry point? | N/A (CI config); triggers push/PR on main + weekly cron — runs on every merge candidate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 yaml parse -> OK (push/pull_request/schedule; 4 matrix languages); grep worklog CodeQL refs -> rounds 11/15 + tag v0.1.0 green claims |

## 1. One-paragraph verdict
codeql.yml is NOT disabled: it triggers on every push and PR to `main` plus a Friday-20:30 weekly cron, scans four languages (actions, javascript-typescript, python, rust), and has real fail capability (no continue-on-error; the manual-build guard hard-exits 1). The worklog records it running GREEN in the round-11 set (0a8ea17c) and the round-15 dispatched set (08727513), and in tag v0.1.0's "full green set" — so the "CI billing-blocked / every run dies at 0 steps" story that other workflow headers still tell is stale: worklog.md:1480 says "Actions billing unblocked — workflows now execute". The workflow's real defect is that it is untouched GitHub template boilerplate: none of its three actions is SHA-pinned, violating the repo's own pinning convention.

## 2. What actually works (verified)
— `python3 -c yaml.safe_load(...)` -> YAML OK; triggers push/pull_request/schedule; matrix languages [actions, javascript-typescript, python, rust]; fail-fast false.
— Cross-evidence of execution: worklog.md:1714 ("Rust/UI/CodeQL/clippy/Tauri-Launch-Verdicts green" at 0a8ea17c), :1893 ("round-15 dispatched set on 08727513 (CI GREEN ... Rust/UI/CodeQL/clippy green)"), :1951 ("Tag v0.1.0 ... the full green set: CI, Rust, UI, CodeQL, clippy, Release dry-run, Launch Verdicts"). Direct CI state UNVERIFIED from sandbox (no network/CI access).

## 3. Findings
F-001 [S3] — No action is SHA-pinned; floating tags violate the repo's own supply-chain convention.
Where: .github/workflows/codeql.yml:64,74,103
Code:
```
    - name: Checkout repository
      uses: actions/checkout@v7
```
and
```
    - uses: github/codeql-action/init@v4
```
and
```
    - uses: github/codeql-action/analyze@v4
```
What is wrong: Every other workflow in the repo pins actions by full commit SHA (soak-nightly.yml:63-67 documents "Pinned actions (the ci.yml/ci-full.yml pins, verbatim)"; docs/RELEASE.md:87 asserts "SHA-pinned `actions/checkout`"). This file uses floating tags (`@v7`, `@v4`) — a tag can be repointed, silently changing what the security workflow executes. For the repo's SECURITY SCANNING lane to be the one workflow with unpinned actions is backwards. Why it matters: the scanner itself is the attack surface a tag-swap would compromise first. What correct looks like: pin checkout + both codeql-action steps to commit SHAs like ci.yml does. Confidence: HIGH — Verification: read + cross-workflow comparison.
F-002 [S4] — Verbatim GitHub boilerplate, zero repo adaptation.
Where: .github/workflows/codeql.yml:1-10 (template header), 91-100 (manual-build placeholder)
Code:
```
# For most projects, this workflow file will not need changing; you simply need
# to commit it to your repository.
```
and
```
      run: |
        echo 'If you are using a "manual" build mode for one or more of the' \
        ...
        exit 1
```
What is wrong: The header is GitHub's stock text ("We have attempted to detect the languages in your repository") — no repo ownership, no link to the repo's own CI conventions; the manual-build step is dead placeholder (all four matrix entries use build-mode `none`). No `timeout-minutes`, no `concurrency` group, no queries selection. It works, but nothing about it was reviewed for this repo beyond committing the template. Confidence: HIGH — Verification: read.

## 4. Per-job table
| Job | Lines | Can it fail? | Verdict | Notes |
|---|---|---|---|---|
| analyze (×4 matrix) | 22-105 | YES (analysis failure exits the job; fail-fast false so one language failing does not cancel others) | REAL (executed per worklog; direct state UNVERIFIED) | rust uses build-mode none (autobuild; repo is pure-Rust so plausible); `actions` language scans the workflows themselves |

## 5. Test honesty
N/A (workflow). Cross-check: worklog run IDs 43189096326 / 43262953759 / 43378741271+43378741286+43378757248 (worklog.md:1480,1532,1625) corroborate that Actions runs execute since the unblock.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "Rust/UI/CodeQL/clippy ... green" (round-11, 0a8ea17c) | worklog.md:1714 | REAL-UNPROVEN (run happened per worklog; result uncheckable here) | no CI access from sandbox |
| "full green set: CI, Rust, UI, CodeQL, ..." (tag v0.1.0) | worklog.md:1951 | REAL-UNPROVEN | same |
| "CI billing blocked — every Actions run dies at 0 steps" | docs/packaging/README.md:238; docs/release-checklist.md:216; KNOWN-LIMITS R-16; soak-nightly.yml:57 | STALE/FALSE on current tree | worklog.md:1480 "Actions billing unblocked — workflows now execute" + run IDs |

## 7. Dead / duplicated / unreachable code
The `Run manual build steps` step (L91-100) is unreachable (no matrix entry uses build-mode manual) — template scaffold, harmless.

## 8. Dependencies & flags
actions/checkout@v7 (floating), github/codeql-action/init@v4 + analyze@v4 (floating). Permissions: security-events write, packages read, actions read, contents read — template defaults, appropriate.

## 9. Open questions for the next agent
- Did CodeQL findings ever get triaged (sarif uploaded)? No committed record of CodeQL alerts anywhere in docs/ found by `rg -i codeql` (only worklog run mentions). If scans have run since 2026-11-13, is anyone reading the sarif?
