# AUDIT: crates/video-engine/src/test_support.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 193 / 10 fixture items |
| File verdict | REAL (test fixture, correctly cfg(test)-gated) |
| Reachable from shipped entry point? | NO — by design: lib.rs:181-182 `#[cfg(test)] mod test_support;`; the ARCH-7 claim ("dev-only types can never leak into the public API") verified structurally |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/video_engine-189b232358e17f55` -> 48 passed, 0 failed, exit 0 (all crate unit tests incl. the queue/scheduler suites that consume this fixture); grep of tests/common/mod.rs (duplicate-copy claim) |

## 1. One-paragraph verdict
A clean test-fixture module: a settable monotonic source (fetch_max — provably never backwards), a virtual MediaClock over it with generation publication, three synthetic-frame builders (ns base, arbitrary stream base, missing PTS), a request builder, one typed presenter-failure error, and a recording presenter with scripted-index failure. It is correctly excluded from every shipped build (`#[cfg(test)]`, verified at lib.rs:181), every piece of it is consumed by the crate's unit suites (48/48 executed green, including the queue tests audited separately that use `RecordingPresenter`/`request`), and the failure-injection presenter refuses BEFORE logging, so the refused request is never recorded as delivered — the semantics the queue's error-path test depends on. The only debt is the disclosed cross-copy duplication with the integration suites' own fixture module.

## 2. What actually works (verified)
- `./target/debug/deps/video_engine-189b232358e17f55` -> `test result: ok. 48 passed; 0 failed` (exit 0) — the fixture is exercised by the scheduler/queue/policy/adjust unit suites.
- Gating verified: lib.rs:181-182; `grep RecordingPresenter tests/common/mod.rs:208` confirms the integration suites carry their own copy exactly as the module doc claims (line 2: "the integration suites under `tests/` carry their own copy").
- `RecordingPresenter::present` failure semantics verified by reading (184-192): fails at `fail_at == log.len()` before pushing — the queue.rs error-path test's [2,3] resume assertion depends on this and passes.

## 3. Findings
F-001 [S4]
Where: crates/video-engine/src/test_support.rs:1-3 vs crates/video-engine/tests/common/mod.rs:27-216
Code:
```
//! Shared fixtures for the crate's unit tests (cfg(test)-gated; the
//! integration suites under `tests/` carry their own copy — dev-only
//! types can never leak into the public API, ARCH-7).
```
What is wrong: the duplication is real (FakeMonotonic, VirtualVideoClock, RecordingPresenter exist in both src/test_support.rs and tests/common/mod.rs — verified by grep) but it is DISCLOSED, deliberate (Rust's test-binary separation), and the doc says so. Risk: the two copies drift (e.g., a fix to the presenter's failure semantics in one, not the other) with no cross-check. Confidence: HIGH — Verification: read + grep.
Why it matters: fixture drift silently weakens one of the two suites. What correct looks like: a `#[cfg(test)] pub(crate)` re-export from one place both can reach, or a periodic diff assertion — low priority for 193 lines of stable fixture.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FakeMonotonic + impl | 26-32 | REAL | — | settable u64 ns source |
| VirtualVideoClock::new/clock | 44-60 | REAL | — | system-monotonic master, rate 1, position 0 |
| VirtualVideoClock::set_ns | 62-67 | REAL | — | fetch_max: monotonic-safe (documented) |
| VirtualVideoClock::advance_ns / now_ns | 69-77 | REAL | — | fetch_add / load |
| VirtualVideoClock::set_generation | 79-83 | REAL | — | delegates to MediaClock (the §5 mint) |
| frame | 90-102 | REAL | — | 2×2 NV12, 6-byte payload, ns base |
| frame_stream_base | 104-111 | REAL | — | arbitrary num/den base (conversion coverage) |
| frame_missing | 113-119 | REAL | — | Stamp::Missing (missing-PTS path) |
| request | 121-128 | REAL | — | PresentRequest with when == pts |
| presenter_failure | 134-145 | REAL | — | typed Device{Display,Busy} at Present stage |
| RecordingPresenter new/failing_at/slots_seen/gens_seen | 155-181 | REAL | — | recording + scripted refusal |
| Presenter for RecordingPresenter | 183-193 | REAL | — | refuses before logging; returns presented_at == when |

## 5. Test honesty
This file IS the fixture; its honesty is the consumers' (queue.rs 6/6 and scheduler suites 48/48 executed green). The scripted-failure presenter would surface a queue error-path bug (verified in queue.rs audit: resume delivers [2,3] with no skip/dup).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cfg(test)-gated" (1) | module doc | REAL | lib.rs:181-182 |
| "the integration suites under tests/ carry their own copy" (2) | module doc | REAL | tests/common/mod.rs:27/208 (grep) |
| "dev-only types can never leak into the public API, ARCH-7" (2-3) | module doc | REAL | module is private + cfg(test); no pub re-export path exists |

## 7. Dead / duplicated / unreachable code
- Duplication: src/test_support.rs vs tests/common/mod.rs (F-001, disclosed).
- `#![allow(dead_code)]` (5) is the honest blanket for fixture items unused by some suites (e.g., `gens_seen` used only by scheduler tests) — acceptable for cfg(test) code.

## 8. Dependencies & flags
media_clock (MediaClock, PlaybackClock, NANO_BASE), media_types (frames/errors/time), testkit::Must, crate::sink types. `#[cfg(test)]` gate verified; no features.

## 9. Open questions for the next agent
- tests/common/mod.rs (the integration copy) is a separate file assignment if listed — its audit should diff the twin against this one (the F-001 drift risk).
- The zero_alloc_scheduler test claims the scheduler tick is allocation-free — this fixture's `frame()` allocates a 6-byte vec per frame, which is push-side, not tick-side; confirm the measurement window excludes fixture allocation when auditing that suite.
