# AUDIT: tests/integration/first-vertical/tests/wire_mode.rs
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 18 lines / 1 #[test] |
| File verdict | REAL-UNPROVEN (real scenario driver; NEVER RAN in the committed baseline) |
| Reachable from shipped entry point? | YES via test lane `cargo test -p first-vertical --test wire_mode` — blocked: needs the built `lumen` binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (S2 filed against the baseline/gate, below) |
| Commands run | file read; scenarios.rs driver read (1130-1296); baseline section analysis (python) |

## 1. One-paragraph verdict
A thin but real driver: `wire_session_through_the_binary` runs
`first_vertical::scenarios::scenario_wire_session()` through `assert_green`. The scenario is
genuine end-to-end: it spawns the REAL `lumen` binary (`lumen_binary_path()`,
scenarios.rs:1287) with `--control-stdio --json`, feeds NDJSON requests, parses actual
stdout lines into response envelopes (`{"requestId"...`) and events (`{"seq"...`),
and asserts exit code 0 plus causal ordering (scenarios.rs:1130-1160 read).

## 2. What actually works (verified)
- Driver wiring verified by read: `mod common;` + `assert_green(&scenario_wire_session())`.
- The scenario's run path is a real process boundary (scenarios.rs:1286-1296:
  `let binary = lumen_binary_path()?;` with an honest build-prerequisite error message).
- Baseline facts: the first-vertical package section (cargo-test-workspace.txt:2722-2795)
  shows ONLY `unittests src/lib.rs`, `tests/baselines_consistency.rs`,
  `tests/class_b.rs`, and `tests/evidence.rs` (FAIL, exit 101) ran.

## 3. Findings
F-001 [S2] — NOT this file's defect, but the committed baseline's: cargo test halts at the
first failing test binary, so `tests/wire_mode.rs`, `tests/journey.rs`, and
`tests/recovery_matrix.rs` NEVER EXECUTED in the committed baseline. The headline
"4744 passed / 2 failed" overstates first-vertical coverage: 3 of 6 test binaries in the
package — including the wire-session integration test and the entire malformed-fixture
recovery matrix — have zero committed passing evidence.
What correct looks like: `--no-fail-fast` in the baseline lane (or fix the two evidence.rs
drift failures) so every binary runs.
Confidence: HIGH — Verification: read (baseline log section lists exactly 4 "Running
tests/" lines; 6 test binaries exist on disk).

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| wire_session_through_the_binary | 16-18 | REAL-UNPROVEN | asserts green over the real-binary scenario; never ran in baseline |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| wire_session_through_the_binary | YES (by design) | scenario asserts exit 0, echoed envelopes, typed not-ready refusals, byte-determinism across the process boundary (doc comment lines 7-13) — but UNVERIFIED-run: baseline blocked upstream |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the serialized command layer across a real process boundary" (lines 1-3) | file itself | REAL-UNPROVEN | scenarios.rs:1287 real binary spawn; run evidence absent |

## 7. Dead code / dependencies
None.

## 8. Open questions
- Did any other lane (full_gate) run wire_mode? full_gate.txt `test` section is the same
  per-package run — same halt applies. UNVERIFIED.
