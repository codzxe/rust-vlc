# AUDIT: crates/codecs/tests/aac_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 66 / 2 (+1 ignored test) |
| File verdict | REAL (measurement tooling, executed) |
| Reachable from shipped entry point? | N/A — `#[ignore]`d measurement test (documented: "number goes into the card's worklog entry, never the normal gate run") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/aac_bench-021b09c784d7517c --ignored --nocapture` → 1 passed (exit 0); output: "corpus: ... absent — SKIP rows" + "fixture sine441_lc_stereo.aac: 268288 inter-channel samples, median 3596905 samples/s = 41x realtime (3.042s of audio)"; default run → 0 passed / 1 ignored (verified earlier) |

## 1. One-paragraph verdict
A clean, honest measurement lane: no performance assertions (a bench, not a gate), SKIP-aware corpus rows with messages, a warm-up decode that doubles as a decodability check (`.must()`), median-of-7 reporting with the realtime multiple derived by the correct arithmetic (I verified: 3,596,905 samples/s ÷ 88,200 samples-per-realtime-second = 40.8 ≈ 41×; 268,288 inter-channel = 134,144 per channel × 2 — consistent with the reference file's row). The `#[ignore]` message prescribes the release invocation; my debug-build execution proves the code path only. Zero findings.

## 2. What actually works (verified)
— executed the ignored test: fixture row measured end-to-end through the production `decode_stream` surface, corpus rows SKIP-observed, sample-count arithmetic cross-checked against aac_bitexact_reference.txt.

## 3. Findings
None. Checked: median index (REPS/2 of a 7-element sorted vec = true median), realtime arithmetic, duration arithmetic, SKIP semantics, absence of gate assertions, `.must()` decodability gate, fixture-row fallback when the corpus is absent.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| bench | 22-44 | REAL | warm-up + 7 reps + median + realtime multiple; eprintln report only |
| aac_decode_throughput | 46-66 | REAL | corpus rows SKIP-aware; fixture row always measured |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| aac_decode_throughput | Only on decode failure (`.must()`); throughput is reported, never asserted — by design and documented |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an ignored measurement test whose number goes into the card's worklog entry, never the normal gate run" | aac_bench.rs:1-4 | REAL | `#[ignore]` with the release invocation in the message; no assertions on numbers |
| "when absent the rows SKIP with a message" | aac_bench.rs:9-10 | REAL | SKIP messages observed live |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
tests/aac/mod.rs helpers (audited: REAL), testkit. No unsafe.

## 9. Open questions for the next agent
None.
