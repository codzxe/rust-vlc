# AUDIT: tests/integration/first-vertical/tests/class_b.rs
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines / functions | 51 / 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration test (runs via workspace `cargo test`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline block check |

## 1. One-paragraph verdict
Five class-B scenario tests (missing-file, truncated-open, truncated-midplay, garbage-appended, noidx) driven through the full command layer in-process, each asserting via `assert_green` (common/mod.rs:9-22) that EVERY machine-evaluated check passed AND that the scenario produced at least one check (an asserting-nothing scenario fails). Would fail if the stack regressed: the assertions name the first failing claim + detail. Baseline: all 5 ok (audit/runs/baseline/cargo-test-workspace.txt:2750-2759, `class_b_* ... ok` ×5, 0.91s).
## 2. What actually works (verified)
- `assert_green` empty-checks guard (common/mod.rs:17-21) — kills the vacuous-scenario class.
- Baseline: 5/5 ok (:2753-2757).
## 3. Findings
None. Zero-finding justification: read the full file; all 5 tests call real scenario functions from the lib (scenarios.rs, another agent's file); the honesty mechanism (per-check named assertion + non-empty check list) is present; no tautologies, no mocks of the subject, no #[ignore].
## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| class_b_missing_file | YES | typed inaccessible expected via scenario checks |
| class_b_truncated_open | YES | typed corrupt refusal at open asserted |
| class_b_truncated_midplay | YES | playback.error event + Error state asserted |
| class_b_garbage_appended | YES | plays to exact end, bounded recovery |
| class_b_noidx | YES | full media recovered + accounting closes |
(All subject to scenarios.rs actually encoding those checks — its audit is another agent's; spot-verified via assert_green contract.)
## 5. Doc claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "union contract ... never panic/hang/unbounded alloc (bounded pump counts make never hang mechanical)" | file L4-7 | REAL-UNPROVEN | mechanics live in scenarios.rs/harness (not this file); UNVERIFIED here |
