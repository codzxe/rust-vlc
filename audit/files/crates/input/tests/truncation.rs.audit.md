# AUDIT: crates/input/tests/truncation.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 211 / 6 tests + 2 helpers (open_with_capacity, assert_truncation) |
| File verdict | REAL (executed green by this auditor) |
| Reachable from shipped entry point? | N/A — test artifact over the shipped file access (crates/input/src/file.rs, reachable via demux_host). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; `cargo test -p input --features contract --test truncation` → **6 passed; 0 failed** (compile 0.63s); also `--test file_stream` → 11 passed (evidence for the contract.rs audit) |

## 1. One-paragraph verdict
This is the strongest test file in my assignment and I executed it myself: six truncation-acceptance tests, all passing, that inject a REAL fault (`set_len` on the live file through `TempFile::truncate_to`, common/mod.rs:55-58) at four distinct positions relative to the reader cursor and buffer window, and pin the typed `Corrupt::Truncated` error — exact (expected, found) pairs, the exact structured Display string, the live `size()` after shrink, sticky-on-retry behavior, snapshot-drain semantics bounded by one window (with byte-identity of the stale bytes asserted), surviving-region reads, exact-at-end reads, and the dual typing (read across the vanished tail = `input.file.truncated`; seek past the shrunk end = `input.seek.out-of-range`, cursor unmoved). The single finding is lane hygiene: the whole file is `#![cfg(feature = "contract")]`, so the default `cargo test -p input` lane runs ZERO of these tests — the orchestrator baseline shows `tests/truncation.rs ... 0 passed; 0 failed; 0 measured` — and the truncation contract is pinned only under `--all-features` CI.

## 2. What actually works (verified)
— executed: `cargo test -p input --features contract --test truncation` →
```
running 6 tests
test file_truncated_mid_read_surfaces_the_typed_error ... ok
test seek_beyond_the_shrunk_length_is_typed_out_of_range ... ok
test surviving_region_reads_then_crossing_errors_typed ... ok
test truncation_lands_mid_refill ... ok
test truncation_below_the_cursor_drains_the_snapshot_then_errors_typed ... ok
test truncation_to_zero_is_typed ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
— also executed: `cargo test -p input --features contract --test file_stream` → 11 passed incl. `real_file_passes_the_stream_contract`, `real_file_passes_the_read_into_contract`, `read_ahead_budget_opens_and_serves` (resolves the contract.rs audit's F-002 evidence gap).

## 3. Findings
F-001 [S4]
Where: crates/input/tests/truncation.rs:15
Code:
```
#![cfg(feature = "contract")]
```
What is wrong: the entire truncation AC suite is compiled out of the default test lane. Evidence: baseline `cargo-test-workspace.txt` (PACKAGE input) `Running tests/truncation.rs ... test result: ok. 0 passed; 0 failed; 0 measured; 0 filtered out`. A regression that flips mid-read truncation to a silent EOF would pass `cargo test -p input` and be caught only by the `--all-features` lane (ci.yml:182, full_gate.sh:21) — which exists, so this is lane hygiene, not a missing test. The header's own words (L11-14) document the gate honestly. What correct looks like: flip the file to `#[cfg(test)]`-visible integration (drop the feature gate for the fault tests that need no conformance kit), or accept and document the two-lane posture. Confidence: HIGH — Verification: baseline log read + my own feature-on run proving the tests are healthy.

Checked and clean: `assert_truncation`'s hardcoded (1000, 300) pair is only used by tests with that shape (to-zero and mid-refill tests assert their own pairs); the drain loop fails loudly on a clean empty (L94) — exactly the silent-EOF failure mode the card forbids; `TempFile::truncate_to` is a real `set_len` on a second handle (no reader-state contamination); temp paths are pid+counter-unique (common/mod.rs:28-33) so parallel runs don't collide.

## 4. Per-test table
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| file_truncated_mid_read_surfaces_the_typed_error | YES | typed error + exact Display + live size pin; executed green |
| truncation_below_the_cursor_drains_the_snapshot_then_errors_typed | YES | bounded (≤ window) stale-drain with byte identity + sticky retry error; executed green |
| truncation_to_zero_is_typed | YES | (500, 0) pair; executed green |
| truncation_lands_mid_refill | YES | (1000, 150) after full-window consumption; executed green |
| surviving_region_reads_then_crossing_errors_typed | YES | reads inside live length OK, crossing typed, exact-at-end OK; executed green |
| seek_beyond_the_shrunk_length_is_typed_out_of_range | YES | code + Inaccessible kind + cursor-unmoved; executed green |

## 5. Test honesty
All six are fault-injection tests against the real filesystem — no mocks of the thing tested. Each would fail loudly under the exact regression class it names (silent EOF, panic, wrong error shape, stale size). Mutation confidence: HIGH.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a file that shrinks below its open-time length mid-read surfaces the typed Corrupt::Truncated error — never a panic, never a silent EOF" | truncation.rs:1-3 | REAL | executed: 6/6 green incl. the drain-then-error and sticky-retry cases |
| "run with `--features contract` (CI's --all-features always does)" | truncation.rs:11-14 | REAL | ci.yml:182/full_gate.sh:21 run --all-features; my feature-on run is the in-hand proof the suite is green |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
input (with `contract` feature), media-types, plugin-api, testkit; `mod common` (TempFile with real set_len/append). Real filesystem I/O in std temp dir.

## 9. Open questions for the next agent
- The two-lane posture (default lane runs 0 truncation tests) — is that a deliberate audit-F-2 outcome or an accident of the gate? If deliberate, a one-line note in docs/10-testing-ci-quality.md naming which lanes pin the truncation contract would close it.
