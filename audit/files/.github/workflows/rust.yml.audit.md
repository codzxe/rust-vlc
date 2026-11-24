# AUDIT: .github/workflows/rust.yml
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / jobs | 27 / 1 (build) |
| File verdict | REAL — GitHub's default Rust template (build + test) with ONE repo customization (ffmpeg oracle install); CAN fail and WOULD fail on the current tree |
| Reachable from shipped entry point? | N/A (CI); triggers push/PR main |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 yaml parse -> OK (2 steps); baseline cross-check (cargo-test-workspace.txt:2764-2791) |

## 1. One-paragraph verdict
This is GitHub's stock Rust starter workflow — `cargo build` + `cargo test` on ubuntu-latest — with exactly one repo-specific touch: an ffmpeg install so the testkit differential suites run (a real integration, matching testkit::oracle_available). The lane is genuine: test failures fail the job, and on the CURRENT tree it would go red — the orchestrator baseline records 2 failing tests (first-vertical `committed_transcripts_match_fresh_runs`, `final_frame_is_current`), which this workflow's plain `cargo test` would hit. The worklog's "Rust green" round claims predate that drift. Defects are template hygiene: floating `actions/checkout@v4`, no `--locked`, no cache, no toolchain assert.

## 2. What actually works (verified)
— `python3 yaml.safe_load(...)` -> 2 run steps parsed; no continue-on-error, no `|| true` — a failing `cargo test` fails the job.
— ffmpeg-oracle integration is real: the same rationale text appears in ci.yml:213-217 (testkit::oracle_available) — the differential suites genuinely skip without it.
— Would-fail-on-current-tree proof: baseline audit/runs/baseline/cargo-test-workspace.txt:2791 `test result: FAILED. 1 passed; 2 failed; 1 ignored` — the same suite `cargo test` here runs.

## 3. Findings
F-001 [S4] — Template hygiene: unpinned action, no --locked, no cache/timeout/concurrency.
Where: .github/workflows/rust.yml:18,23-26
Code:
```
    - uses: actions/checkout@v4
    ...
    - name: Build
      run: cargo build --verbose
    - name: Run tests
      run: cargo test --verbose
```
What is wrong: (a) `checkout@v4` floating tag (repo convention is full-SHA pins — ci.yml/soak-nightly.yml pin everything); (b) no `--locked` — a drifted lockfile silently re-resolves in CI instead of failing (the release lane uses `--locked` everywhere); (c) no rust-cache → full rebuild per run; no concurrency group (PR runs pile). Toolchain: no explicit setup step — relies on the runner's rustup honoring rust-toolchain.toml (1.99.1 auto-install), which works but is implicit; every other workflow asserts the pin explicitly. Confidence: HIGH — Verification: read + convention comparison.
F-002 [S4] — Redundant lane: duplicates ci.yml's `gate` test coverage minus the clippy/fmt halves.
Where: .github/workflows/rust.yml:23-26 vs ci.yml:47 ("gate runs the IDENTICAL clippy+test strings")
What is wrong: same `cargo test` surface twice per PR (ci.yml gate + this lane), on different action pins — divergent signals if one drifts (this one has no toolchain assert). Not harmful; extra compute. Confidence: HIGH — Verification: read + ci.yml comparison.

## 4. Per-job table
| Job | Lines | Can it fail? | Verdict | Notes |
|---|---|---|---|---|
| build | 12-27 | YES — build or test failure fails it; no continue-on-error | REAL (template-grade) | would FAIL on current tree (baseline's 2 test failures) |

## 5. Test honesty
The workflow itself is the test invocation. Cross-evidence: worklog.md:1714/1893/1951 claim "Rust ... green" in rounds 11/15 and tag v0.1.0 — true at those commits; the current tree's 2 first-vertical failures would make this lane red if run today (baseline proof). No contradiction alleged: time-relative claims, honestly framed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "The differential tests skip loudly without ffmpeg (testkit::oracle_available); installing it makes them run" | rust.yml:20-21 | REAL | same rationale + mechanism in ci.yml:213-217; rc-smoke/oracle audits corroborate the skip-loudly design |
| "Rust green" (rounds/tag) | worklog.md:1714,1893,1951 | TRUE-then / would-be-RED-now | baseline cargo-test-workspace.txt:2764-2791 (2 failures) |

## 7. Dead / duplicated / unreachable code
F-002 duplication with ci.yml gate.

## 8. Dependencies & flags
actions/checkout@v4 (floating); apt ffmpeg; implicit toolchain via rust-toolchain.toml (1.99.1 + rustfmt + clippy components declared).

## 9. Open questions for the next agent
- Which lane does branch protection actually require (this one, ci.yml's gate, or both)? Determines whether the 2 current-tree test failures are already visible on every PR.
