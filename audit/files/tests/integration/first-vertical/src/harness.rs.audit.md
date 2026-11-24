# AUDIT: tests/integration/first-vertical/src/harness.rs
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 746 / 24 (2 consts, 6 types/impls, 16 fns/methods) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES from the test lane: `cargo test -p first-vertical` (baseline ran it); NOT in the release build (test crate) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | Read (746 lines); rg baseline; python3 semantic diff of the baseline panic payload |

## 1. One-paragraph verdict
This is a genuinely constructed integration harness: it drives the REAL
pipeline (input → probe → AVI demux → MJPEG/PCM decode → null audio device →
software compositor) exclusively through the CLI's single dispatch entry
(`lumen_dispatch`, L257), records every command/response/event into a JSON
transcript, and pins `hardware_class: "mock"` with an explicit
anti-fabrication rationale (L78-83). The determinism seam (FrozenThrottle,
L54-60) is documented and load-bearing. The one hard defect is not in the
harness code but in its product: the committed evidence transcripts it
generates are STALE — the baseline's own currency test FAILED with a 3×
accounting drift (frames_accepted 30 committed vs 94 fresh), so the committed
evidence does not describe current engine behavior.

## 2. What actually works (verified)
— Baseline execution: audit/runs/baseline/cargo-test-workspace.txt:2763-2767
shows the suite ran: `ledger_matches_the_artifacts ... ok`,
`committed_transcripts_match_fresh_runs ... FAILED`, `final_frame_is_current
... FAILED`, `regen ... ignored`.
— python3 semantic diff of the panic's left/right payloads: fresh run differs
from committed `open` transcript by `.events` 8→9 (extra
`session.buffering progress 0.00`), `.observables.accounting.frames_accepted`
30→94, `scheduler_pushed` 15→16. Real observables, real drift, honest
failure — the harness surfaces it instead of hiding it.
— Transcript assembly records only redaction-safe shapes: L423-451 unrecorded
result variants answer `"(result variant not recorded by this suite)"`,
L643-646 unrecorded event kinds are marked, never `Debug`-dumped paths.

## 3. Findings
F-001 [S2]
Where: product of this file — tests/integration/first-vertical/evidence/*
(the transcripts this harness renders); failure surfaced by
tests/integration/first-vertical/tests/evidence.rs:47 (not in this file)
Code (baseline, audit/runs/baseline/cargo-test-workspace.txt:2774):
`assertion 'left == right' failed: the committed transcript for 'open' drifted from a fresh run — regenerate with: cargo test -p first-vertical --test evidence -- --ignored regen`
What is wrong: the committed evidence was not regenerated after engine
changes; frames_accepted recorded 30 vs 94 actually decoded — the committed
transcripts understate current behavior by 3× and are presented as current
evidence.
Why it matters: doc comment L19-22 claims "a transcript is byte-stable for a
given code version — which is what lets the committed evidence be re-verified
by a fresh run"; that contract is currently broken at the artifact level, and
anyone citing the transcripts as proof of decode counts cites stale numbers.
What correct looks like: run the ignored `regen` and re-commit.
Confidence: HIGH — Verification: read baseline + executed python3 diff.
F-002 [S3]
Where: harness.rs:572-578
Code: `let bucket = if *progress <= 0.25 { "0.00" } else if *progress < 0.75 { "0.50" } else { "1.00" };`
What is wrong: buffering progress is collapsed to 3 buckets before
committing; a real regression that moves mid-episode progress from 0.02 to
0.24 changes no transcript byte.
Why it matters: the transcript is the evidence artifact; coarse-bucketing
shrinks its discriminating power for buffering behavior. The in-file
rationale (load-dependence of the fill fraction) is plausible, but the cost is
real. What correct looks like: pin the pump-count at which progress is
sampled so the exact fraction is deterministic.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: harness.rs:196-199
Code: `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")`
What is wrong: workspace root derived by hard-coded three-level hop; silent
wrong-path if the crate ever moves. Confidence: HIGH.
F-004 [S4]
Where: harness.rs:732-744 — `normalize_run_specific` rewrites any line whose
trimmed text starts `"environment_kind":` or `"os":` to a placeholder; scoped
correctly today, but a future transcript field with the same name would be
silently normalized. Confidence: MEDIUM.

## 4. Per-function table (every function/method)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FrozenThrottle::now_ns | 54-60 | REAL | — | frozen clock seam, returns 0 by design (determinism, documented) |
| Labels::detect / to_json | 77-106 | REAL | — | honest mock pin; reads CI env for environment_kind only |
| Check::is / int / same | 124-155 | REAL | — | claim records with observed-vs-expected detail |
| ScenarioRun::all_passed / transcript_bytes | 174-185 | REAL | — | verdict + exact bytes |
| run_pumps | 190-192 | REAL | — | delegates CLI RUN_PUMPS (no disagreement possible) |
| workspace_root | 196-199 | REAL | S4 | 3-level path hop |
| fixture_path | 206-213 | REAL | — | existence-checked, typed error |
| SessionHarness::new | 229-247 | REAL | — | installs FrozenThrottle |
| SessionHarness::dispatch | 251-260 | REAL | — | through lumen_dispatch only; records step JSON |
| settle / advance / state / position_us / session_id / accounting / state_history / screenshot_png | 264-307 | REAL | — | thin observation surface over the host |
| SessionHarness::finish | 312-362 | REAL | — | assembles transcript; PNG digest+IHDR only |
| step_json / error_json / result_json / media_info_json / track_json | 375-513 | REAL | — | honest placeholders for unrecorded variants (L451, L643-646) |
| state_json | 516-518 | REAL | — | Debug-name of state enum |
| event_json | 540-649 | REAL | S3 | buffering bucketing (F-002); seq-gap doc is excellent |
| accounting_json | 653-686 | REAL | — | 15 counter fields incl. seek-boundary observability |
| png_json | 693-709 | REAL | — | real IHDR extent parse + sha256, no panic on short PNG |
| kind_str | 712-719 | REAL | — | 4-way match |
| normalize_run_specific | 732-745 | REAL | S4 | 2-line rewrite (F-004) |

## 5. Test honesty
| Test (suite this harness feeds) | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_transcripts_match_fresh_runs | YES — it is failing RIGHT NOW | baseline :2764 FAILED with the 30-vs-94 drift; the check works |
| final_frame_is_current | YES — failing now | baseline :2767 FAILED (stale final-frame digest) |
| ledger_matches_the_artifacts | YES | baseline :2765 ok — ledger/artifact consistency enforced |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "transcript is byte-stable for a given code version" | harness.rs:19-22 | FALSE at artifact level | baseline FAILED with semantic drift (frames 30→94) |
| "hardware_class is always mock … asserting anything else would be fabrication" | harness.rs:23-26 | REAL | code pins "mock" (L90) |
| "gaps indicate filtered events … never reordering" | harness.rs:526-539 | REAL-UNPROVEN | doc-cited rule; seq gaps explained, not verified by run here |

## 7. Dead / duplicated / unreachable code
None found. All 24 items are used by the scenario/test files of this crate
(rg: `SessionHarness`, `dispatch`, `finish` used in scenarios/*.rs).

## 8. Dependencies & flags
Depends on lumen-cli (CliHost, lumen_dispatch, RUN_PUMPS), commands crate,
crate::digest/json — first-party only. No features gated.

## 9. Open questions for the next agent
Which engine change grew frames_accepted 30→94 without a regen? If the
answer is "decode depth grew," the committed stop/pause transcripts I audit
next carry the same stale numbers.
