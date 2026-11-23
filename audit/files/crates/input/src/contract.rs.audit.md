# AUDIT: crates/input/src/contract.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 651 (431 production + 114 tests + header) / 18 production fns + 7 tests |
| File verdict | REAL (test/conformance-only surface, cfg-gated out of release — as documented) |
| Reachable from shipped entry point? | NO BY DESIGN: `crates/input/src/lib.rs:64-65` `#[cfg(any(test, feature = "contract"))] pub mod contract;` + `crates/input/Cargo.toml:30` `contract = []` — release binaries carry none of it (verified: no `features = ["contract"]` in any manifest). Executed by input's own unit tests (cfg(test)) — baseline-verified. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (contract consumers/features/workflows), sed -n, baseline citations |

## 1. One-paragraph verdict
A real, well-built executable contract: a scripted synthetic source (shrink/fail-next-read instrumentation, max-read-seen tracking) wrapped in the real BufferedReader + SourceStream trait-object shape, and five `#[track_caller]` assert suites (stream contract, borrow twin, peek honesty, bounded-read proof with exact read-count pinning, typed truncation). The cfg discipline is honest — the module is compiled out of release and its assert/unreachable vocabulary never ships. All 7 in-module tests pass in the baseline. The defects are claim-level, not code-level: the module doc says every access implementation — "this crate, `streaming` (Phase 6), the conformance kit (doc 06 §7.2)" — runs this same contract from its own tests, and that is false for streaming (zero references in crates/streaming; its ranged resource has its own suite instead) and unsupported for the "conformance kit" (no in-tree artifact enables the feature; only `--all-features` CI does, and the captured baseline logs contain no results for the feature-gated tests). The code itself is correct on read: the ScriptSource invariants hold (visible_len ≤ data.len() always, so the defensive `end <= data.len()` copy guard is never violated), seek/position math is right, and the bounded-read proof pins the exact I/O shape (bulk call + at most one refill).

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt` (PACKAGE input): all 7 `contract::tests::*` pass ("33 passed; 0 failed" for lib unittests), incl. `bounded_read_proof_on_synthetic_source`, `shrink_mid_read_is_a_typed_truncation_error` (pins code `input.file.truncated`, (1000,400)), `seek_within_buffer_serves_no_reads` (no-I/O buffer-hit seek pinned via reads_served).
— verified by read: `assert_bounded_read` L412-416 pins the exact source-call count (`+2` complete / `+1` short) — a real anti-refill-loop proof; `assert_stream_contract` checks position-stickiness, typed `input.seek.out-of-range` / `input.seek.before-start`, and cursor-unmoved-on-failed-seek.
— ADDENDUM (executed later in this run, see truncation.rs audit): `cargo test -p input --features contract --test truncation` → 6 passed; `--test file_stream` → 11 passed (incl. `real_file_passes_the_stream_contract`, `real_file_passes_the_read_into_contract`, `read_ahead_budget_opens_and_serves`). F-002's evidence gap is CLOSED: the feature-gated tests are green when the feature is on; the remaining gap is only that the default lane and the captured baseline logs never show them.

## 3. Findings
F-001 [S3]
Where: crates/input/src/contract.rs:21-23
Code:
```
//! Every access implementation — this crate, `streaming` (Phase 6), the
//! conformance kit (doc 06 §7.2) — runs the same executable contract
//! from its own tests:
```
What is wrong: the claim is false for streaming. `rg "assert_stream_contract|SyntheticStream|assert_bounded_read" crates/streaming/` → exit 1 (zero matches); streaming's ranged resource never runs this contract — `tests/ranged_wav_e2e.rs` does its own byte-parity check against `input::FileAccess` instead (L125, L191). The "conformance kit (doc 06 §7.2)" is a doc concept (doc 06 §7.2 "Capability honesty" describes manifest-claim exercising, and never mentions this module); no crate enables the `contract` feature, so the only lane that compiles the gated tests is `--all-features` CI. Why it matters: the "same executable contract across implementations" is exactly the cross-implementation credibility the module exists for; the one other production access implementation (RangedResource) is not held to it, and nothing would notice if the two implementations' seek/EOF contracts drifted. What correct looks like: either wire `assert_stream_contract(&mut ranged_resource, ...)` into a streaming test (dev-dep already exists) or delete streaming from the claim. Confidence: HIGH — Verification: read + rg (exit 1).

F-002 [S4]
Where: crates/input/src/contract.rs:4-7 (claim) vs baseline evidence
Code:
```
//! is `#[cfg(any(test, feature = "contract"))]` — unit tests compile
//! it always, integration tests and the doc 06 §7.2 conformance kit
//! enable the `contract` feature, and **release binaries carry none
```
What is wrong: the feature-gated integration tests have no captured passing run: baseline per-package (default features) shows `tests/truncation.rs ... 0 passed; 0 failed; 0 measured` (whole file is `#![cfg(feature = "contract")]`) and `tests/file_stream.rs ... 9 passed` of 11 tests (the 2 contract tests stripped). CI's `--all-features` lane (ci.yml:182, full_gate.sh:21) compiles and runs them, but the captured full_gate log's test section FAILED early on the two known first-vertical tests and contains no per-test results for these. So "integration tests ... enable the `contract` feature" is true mechanically, UNVERIFIED as a green run in the available evidence. Confidence: HIGH (evidence gap, not a code defect) — Verification: baseline logs read.

F-003 [S4]
Where: crates/input/src/contract.rs:407-411
Code:
```
    let seen = stream.source().max_read_seen();
    assert!(
        seen <= want.max(capacity),
        "the source never sees a read beyond the caller's request (saw {seen}, want {want})"
    );
```
What is wrong: `want > capacity` is asserted at L376-379, so `want.max(capacity)` is always `want` — the `.max(capacity)` is dead and, if the precondition were ever relaxed, would silently permit a capacity-sized speculative over-read that the message ("never ... beyond the caller's request") denies. Assertion/message precision drift only. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ScriptSource::new | 85-94 | REAL | — | |
| ScriptSource::shrink_to | 100-102 | REAL | — | min-reduce only; visible_len ≤ data.len() invariant holds |
| ScriptSource::fail_next_read | 105-110 | REAL | — | io::ErrorKind::Unsupported → typed read-io |
| ScriptSource::max_read_seen / reads_served | 113-121 | REAL | — | instrumentation accessors |
| Seekable::position/length/seek | 125-139 | REAL | — | seek>visible_len typed |
| RawRead::read | 143-158 | REAL | — | defensive copy-guard unreachable (invariant); fault injection first |
| SyntheticStream::new | 175-179 | REAL | — | synthetic URI label |
| SyntheticStream::source / shrink_to | 182-189 | REAL | — | |
| SourceStream::{uri,size,supports_seek} | 193-203 | REAL | — | |
| SourceStream::read / read_into | 205-213 | REAL | — | delegates to BufferedReader (part-2 bypass) |
| SourceStream::seek / position | 215-223 | REAL | — | resolve_origin from seekable.rs |
| peek_into / read_exact_into | 231-239 | REAL | — | |
| assert_truncated | 244-251 | REAL | — | kind destructure + unreachable! on wrong kind |
| assert_stream_contract | 257-360 | REAL | — | varied chunks, sticky EOF, seek round-trip, typed errors, cursor-unmoved |
| assert_bounded_read | 373-417 | REAL | S4 | F-003 dead `.max(capacity)`; exact call-count pin is real |
| assert_read_into_contract | 426-490 | REAL | — | borrow twin + read/read_into byte agreement |
| assert_peek_into_contract | 496-535 | REAL | — | no-advance, no-I/O, window-bound |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| synthetic_stream_passes_its_own_contract | YES | full contract over real BufferedReader |
| synthetic_stream_passes_the_read_into_contract | YES | borrow twin + API agreement |
| synthetic_stream_passes_the_peek_into_contract | YES | I/O-count + advancement pins |
| bounded_read_proof_on_synthetic_source | YES | exact reads_served delta (2/1) + capacity-never-grows |
| shrink_mid_read_is_a_typed_truncation_error | YES | (expected,found)=(1000,400) + code pin |
| scripted_read_failure_propagates_typed | YES | code `input.file.read-io` |
| seek_within_buffer_serves_no_reads | YES | reads_served equality across seeks |
All 7 baseline-passing (cargo-test-workspace.txt, PACKAGE input, "33 passed").

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "release binaries carry none of it" | contract.rs:7-9 | REAL | lib.rs:64-65 cfg gate + Cargo.toml:30 empty feature; release lane builds without it |
| "Every access implementation — this crate, streaming (Phase 6), the conformance kit — runs the same executable contract" | contract.rs:21-23 | FALSE (for streaming; UNVERIFIED for "kit") | rg over crates/streaming → 0 matches (F-001) |
| "integration tests ... enable the `contract` feature" | contract.rs:6-7 | REAL-UNPROVEN | gating verified; a green run of the gated tests is absent from all captured logs (F-002) |

## 7. Dead / duplicated / unreachable code
Unreachable from shipped binaries by design (documented, verified). Duplication: `crates/playback-engine/tests/scenarios_b/host.rs:44-49` re-implements a ScriptSource-like scripted source instead of reusing this one ("owned wholesale and cannot be scripted" — its own words); comments-only references in demux test_support.

## 8. Dependencies & flags
Depends on media-types, plugin-api, crate::buffered/seekable/error. Feature `contract = []` (Cargo.toml:30) — enables nothing but the cfg. No external deps.

## 9. Open questions for the next agent
- Did any CI run execute `cargo test --workspace --all-features` green end-to-end (the 2 first-vertical failures aside)? If not, the truncation suite (tests/truncation.rs, 6 tests) has NEVER had a recorded green run — check audit/runs/baseline/rc-smoke.txt and CI history before trusting it.
- Should `assert_stream_contract` be added to streaming's RangedResource tests (F-001)? The seam exists (streaming dev-depends on input).
