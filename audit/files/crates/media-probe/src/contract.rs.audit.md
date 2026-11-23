# AUDIT: crates/media-probe/src/contract.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 510 / 14 prod fns + 2 trait impls + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — by documented design: `#[cfg(any(test, feature = "contract"))]` (lib.rs:160-161), feature not default (Cargo.toml:29-36); release binaries carry none of it. Test-lane reachability EXECUTED (see below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p media-probe --features contract --test probe_io` → 10 passed / 0 failed in 0.01s (9.15s compile); sed/rg/python3 verification; baseline :3586-3590, :3684-3687 |

## 1. One-paragraph verdict
A real executable-contract module: two instrumented sources (in-memory
`CountingSource` with scripted faults/chunk-limits, pass-through
`CountingStream` over the real access stack), a parallel (request, served)
ledger, and two `#[track_caller]` assertions that encode the probe-depth
bound and position-restoration contracts exactly. The unit tests are
honest — they prove the assertions themselves fire via `catch_unwind`
(441-464, 505-508), not just that they pass. I executed the feature-gated
integration lane myself: all 10 probe_io tests pass over the real input
stack, including `depth_bound_holds_over_the_real_input_stack_for_every_window`
and `the_probe_never_consumes_real_stream_bytes`. The one defect: the
default per-package test lane silently runs ZERO of those integration
tests (baseline shows `running 0 tests ... ok` for probe_io) — a green
zero-test run that only CI's `--all-features` lane (ci.yml:234) and
full_gate.sh:21 recover.

## 2. What actually works (verified)
- EXECUTED: `cargo test -p media-probe --features contract --test probe_io`
  → `test result: ok. 10 passed; 0 failed` (my run, 2026-11-23), including
  `depth_bound_holds_at_the_doc_ceiling_over_the_real_stack ... ok`,
  `truncated_files_prove_the_eos_fill_is_bounded ... ok`.
- Unit tests pass in baseline: cargo-test-workspace.txt:3586-3590
  (`contract::tests::counting_source_serves_and_instruments ... ok` etc.,
  all 5).
- Gating verified: lib.rs:160-161 cfg; Cargo.toml `contract = []` not in
  default; probe_io.rs:18 `#![cfg(feature = "contract")]`; CI ci.yml:234
  `cargo test --workspace --all-features`; full_gate.sh:21 same.

## 3. Findings
F-001 [S4]
Where: crates/media-probe/src/contract.rs:5-7 (with baseline
audit/runs/baseline/cargo-test-workspace.txt:3684-3687)
Code:
```
//! **Test/conformance-only surface (audit F-2):** the whole module is
//! `#[cfg(any(test, feature = "contract"))]` — unit tests compile it
//! always, integration tests and the doc 06 §7 conformance kit enable
//! the `contract` feature, and **release binaries carry none of it**
```
What is wrong: the module doc is accurate, but the practical consequence
is that the plain per-package lane (`cargo test -p media-probe`, which the
orchestrator baseline ran) compiles probe_io.rs to `running 0 tests ... ok`
(baseline :3684-3687) — a green zero-test binary. The disclosure exists
only inside probe_io.rs:14-18 ("run with `--features contract`"); nothing
in this module's own docs warns that the headline test count (4744 passed)
includes none of the 10 real-stack conformance tests. Mitigated: CI and
full_gate both run `--all-features`, and I executed the lane myself — all
10 pass. What correct looks like: repeat the `--features contract`
instruction in this module doc too. Confidence: HIGH — Verification: ran
(the 10 pass) + baseline citation.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ReadLog (trait) | 49-52 | REAL | — | parallel ledgers contract |
| CountingSource::new / with_uri | 84-108 | REAL | — | `unreachable!` on literal URI — documented sanctioned test vocabulary (:79-88) |
| set_unseekable / set_chunk_limit / fail_next_read | 111-129 | REAL | — | fault scripting; typed error codes |
| requests/served/total_served/max_request | 131-149 | REAL | — | accessors |
| CountingSource::read | 165-179 | REAL | — | fault-first (excluded from ledger, doc :44); `take = want.min(available)`; indexing safe |
| CountingSource::seek | 181-218 | REAL | — | checked_add on Current; out-of-range typed; no panic |
| ReadLog for CountingSource | 225-229 | REAL | — | |
| CountingStream::new + accessors | 248-279 | REAL | — | |
| CountingStream SourceStream impl | 281-310 | REAL | — | forwards; read logged after `?` (a faulted inner read is unlogged — consistent with CountingSource) |
| ReadLog for CountingStream | 312-316 | REAL | — | |
| assert_probe_depth_bounded | 324-347 | REAL | — | budget arithmetic re-traced: `request <= budget`, `budget -= got.min(budget)` — sound vs the fill loop's ask; total bound asserted |
| assert_position_restored | 352-359 | REAL | — | position equality |
| 5 unit tests | 365-509 | REAL | — | include catch_unwind proofs that both bound violations fire (449-463, 505-508) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 5 unit tests (baseline :3586-3590) | YES | ledger contents asserted exactly; assertion-firing proven via catch_unwind |
| 10 probe_io integration tests (my run) | YES | real files through input's access stack; depth/position asserted per window; garbage/truncation lanes typed |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "release binaries carry none of it" | :7-10 | REAL | cfg gate verified; feature not default; release lane builds -p lumen-cli/desktop without it |
| "after probing, the source never served more than max_lookahead bytes in total, and no single request asked for more than the budget remaining" | :23-28 | REAL | :325-347 implements exactly this; executed by probe_io (10/10 ok, my run) |
| "the stream is back where it started and still serves the same bytes" | :29-31 | REAL | :352-359 + probe_io `the_probe_never_consumes_real_stream_bytes ... ok` |
| "integration tests … enable the `contract` feature" | :5-6 | REAL with caveat | they gate on it (probe_io.rs:18); enabled only under --all-features (CI ci.yml:234, full_gate.sh:21) — F-001's lane note |

## 7. Dead / duplicated / unreachable code
None dead. The module is deliberately unreachable from release builds —
that is its documented design, not a defect.

## 8. Dependencies & flags
media-probe, std-only; feature `contract = []` (Cargo.toml:29-36). No
unsafe. Uses media-types EngineError kinds and plugin-api SourceStream.

## 9. Open questions for the next agent
- None blocking. The probe_io green-zero lane note (F-001) applies to any
  other `--features contract`-gated test binaries in the repo — worth a
  workspace-wide rg for `#![cfg(feature` in tests/ dirs.
