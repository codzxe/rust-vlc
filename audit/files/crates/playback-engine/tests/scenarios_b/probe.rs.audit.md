# AUDIT: crates/playback-engine/tests/scenarios_b/probe.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 51 / 1 `#[test]` fn |
| File verdict | SHALLOW (a committed debug print masquerading as a test) |
| Reachable from shipped entry point? | N/A (test file) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | full-file read; cross-check of the scenarios_b harness (`crate::host::{ScenarioHost, dispatch}` — the module's shared host); grep of the file's assertions |

## 1. One-paragraph verdict

A leftover. The file's own first line says "Temporary probe — measure the scripted source's read pattern (to be deleted before commit)" — and then it was committed. The single test opens a scripted AVI session, asserts exactly one thing (`open.result.is_ok()`), and spends the rest of its 40 lines printing read/byte statistics to stderr. It runs unconditionally in every test invocation, serializes against the allocation-counting tests via a global lock, and contributes one guaranteed-green row to the pass totals.

## 2. What actually works (verified)

Nothing to verify beyond the harness it rides on (`ScenarioHost`/`dispatch` — the scenarios_b shared host, another agent's file). The probe does confirm the scripted-source read-stats instrumentation exists (`host.script().must().read_stats()`), which is mildly informative about the harness's capabilities.

## 3. Findings

### F-001 [S3] — A self-declared temporary probe was committed and runs as a permanent near-assertion-free test

Where: crates/playback-engine/tests/scenarios_b/probe.rs:1-2 and 24.

Code (1-2, verbatim):
```rust
//! Temporary probe — measure the scripted source's read pattern (to
//! be deleted before commit).
```

Code (24, verbatim):
```rust
    assert!(open.result.is_ok(), "{:?}", open.result);
```

What is wrong: the file declares its own deletability and contradicts it by existing in the tree. The test's only assertion is that open succeeds — every other line is `eprintln!` debugging. §3.4: tests that assert nothing beyond "does not panic" inflate the suite's pass count (this is one of the 1751 "passing tests" in the W00 baseline) and pollute stderr in every run. It also takes the `serialize_tests()` global lock, needlessly serializing the suite.

Why it matters: exactly the paper-pass pattern the audit exists to count — a green row that proves nothing; and a process failure (the "delete before commit" discipline broke in wave 3.E2).

What correct looks like: deleted; or converted into a real assertion (e.g., "the scripted source performs zero reads after Buffering reaches Primed" — presumably what the probe was measuring toward).

Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `probe_read_pattern` | 12-50 | SHALLOW | S3 | one is_ok + 40 lines of eprintln; global test lock taken |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `probe_read_pattern` | Only if session.open fails outright | the read-pattern measurements it exists for are printed, never asserted |

## 6. Doc claims touching this file

None beyond its own self-condemnation.

## 7. Dead / duplicated / unreachable code

The entire file is vestigial by its own documentation.

## 8. Dependencies & flags

`commands`, `testkit` (incl. `alloc_count::serialize_tests`); the scenarios_b harness. No cfg gate — always runs.

## 9. Open questions for the next agent

- What did the probe measure toward, and did the intended assertion land elsewhere (scenario suite)? scenarios_b's other files (my next three) are the place to check.
