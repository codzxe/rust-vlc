# AUDIT: tests/integration/first-vertical/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W16-A04 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 78 / 5 (pub mod decls; no fn bodies) |
| File verdict | REAL (crate root wiring real; unit tests green in baseline) |
| Reachable from shipped entry point? | NO — test-support crate, reachable from `cargo test -p first-vertical` (ci.yml:234 `cargo test --workspace --all-features`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read full file; rg module files; baseline citation |

## 1. Verdict
Thin, honest crate root: five `pub mod` declarations (digest/evidence/harness/json/scenarios) over
real modules, plus documentation that explicitly labels the audio sink and video render as mocks.
The declared modules' unit tests ran green in the orchestrator baseline (10 passed, incl. digest
NIST vectors, evidence artifact paths, json determinism). One doc claim in this file is currently
false: the "currency tests prove byte-for-byte match" — that test FAILED in the same baseline.

## 2. What actually works (verified)
- All 5 declared modules exist: src/digest.rs, src/evidence.rs, src/harness.rs, src/json.rs,
  src/scenarios.rs (rg --files, ls).
- Baseline cargo-test-workspace.txt:2726-2740 — unittests of this lib: 10 passed / 0 failed
  (digest::tests::nist_vectors … json::tests::scalars_render all `ok`).
- Mock labels are machine-checked, not prose: evidence.rs:39-40 asserts hardware_class == "mock"
  and devices == [null-audio, software-render] on every machine that runs the suite.

## 3. Findings
F-001 [S3]
Where: tests/integration/first-vertical/src/lib.rs:49-51
Code:
//! The committed artifacts are the recorded evidence of the sandbox
//! run that generated them; the currency tests prove they match a
//! fresh run byte-for-byte (which is also the determinism proof).
What is wrong: the doc asserts the currency proof holds; in the recorded baseline it does NOT —
`committed_transcripts_match_fresh_runs` FAILED (cargo-test-workspace.txt:2764, panic at
evidence.rs:47: "the committed transcript for `open` drifted from a fresh run"), as did
`final_frame_is_current` (:2767). The doc is false at HEAD.
Why it matters: this file is the suite's own claim ledger; a reader trusts the §7 currency claim.
What correct looks like: regenerate evidence/ (the #[ignore] `regen` entry point) or amend the doc.
Confidence: HIGH — Verification: read + baseline log.
F-002 [S4]
Where: tests/integration/first-vertical/src/lib.rs:55-60
Code:
//! - The card's "green on 3 OS in CI" AC is **PENDING**: the workflow
//!   files exist (`.github/workflows-disabled/ci.yml`, actionlint-
//!   validated) but no runner executed them in this sandbox.
What is wrong: stale pointer — the live workflow is .github/workflows/ci.yml (runs
`cargo test --workspace --all-features` at :234); a `.github/workflows-disabled/` directory does
not exist in the tree (rg --files .github → only live workflows). The PENDING framing
understates current wiring. UNVERIFIED whether any hosted runner actually executed it.
Confidence: MEDIUM — Verification: read + rg.

## 4. Per-function table
File has no functions; the surface is the module set:
| Module | Verdict | Severity | Notes |
|---|---|---|---|
| pub mod digest | REAL | — | SHA-256 NIST-vector-pinned; baseline `ok` |
| pub mod evidence | REAL | — | path/ledger helpers; baseline `ok` |
| pub mod harness | REAL | — | ScenarioRun/checks; exercised via class_b (5 ok) |
| pub mod json | REAL | — | deterministic emitter; 4 baseline `ok` |
| pub mod scenarios | REAL | — | SCENARIO_IDS; class_b/journey consumers |

## 5. Test honesty
No tests in this file; its unit tests live inside the modules (all green in baseline — see §2).
The suite-level honesty depends on evidence.rs, which is currently failing for `open` (see F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "currency tests prove they match a fresh run byte-for-byte" | lib.rs:49-51 | FALSE (at HEAD) | baseline :2764/2767 FAILED |
| "no runner executed them in this sandbox" (workflows-disabled) | lib.rs:55-58 | STALE | ci.yml is live; no workflows-disabled/ dir |
| audio/video are honestly labeled mock | lib.rs:7-12 | REAL | evidence.rs:39-40 machine-checks it |

## 7. Dead / duplicated / unreachable code
None in this file.

## 8. Dependencies & flags
Crate tests/integration/first-vertical/Cargo.toml (separate audit); workspace member via
tests/ dir — included in `cargo test --workspace` (baseline ran it: "### PACKAGE first-vertical").

## 9. Open questions for the next agent
Did anything regenerate evidence/ after the drift was recorded (2026-11-23)? If not, every
transcript audit inherits F-001.
