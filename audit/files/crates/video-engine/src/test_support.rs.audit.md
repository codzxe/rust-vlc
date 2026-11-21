# AUDIT: crates/video-engine/src/test_support.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 193 / 13 items |
| File verdict | REAL (test fixture; properly cfg(test)-gated; exercised by green tests) |
| Reachable from shipped entry point? | NO — BY DESIGN: `#[cfg(test)] mod test_support;` (lib.rs:181-182) — verified; the "dev-only types can never leak into the public API" claim holds |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "test_support"` → lib.rs:182 cfg(test) + queue.rs:237 + scheduler.rs:1089 imports; `rg "failing_at" crates/video-engine/` → sole src user queue.rs:370 + the declared tests/common copy at 223; `rg "gens_seen\|advance_ns\|\.now_ns\(\)"` in src (excl. this file) → zero users; NV12 2×2 byte math hand-checked (4+2=6) |

## 1. One-paragraph verdict
Clean, correctly-gated test infrastructure: the cfg(test) gate is real (lib.rs:181-182), the fixture types are consumed by the crate's genuine scheduler/queue unit tests, the synthetic NV12 frame is dimensionally correct (2×2 = 6 bytes: 4 Y + 2 interleaved UV), and the virtual clock follows media-clock's injected-truth pattern with a monotonic-safe `fetch_max` setter. Two hygiene defects: the module-wide `#![allow(dead_code)]` hides three genuinely dead helpers, and `RecordingPresenter::failing_at`'s doc comment promises a single refusal while the implementation refuses the indexed request and every request after it.

## 2. What actually works (verified)
- Gating: `#[cfg(test)] mod test_support;` (lib.rs:181-182) — no leak into the shipped binary or public API; the module's ARCH-7 claim (1-3) is TRUE.
- Consumers exist and are real tests: queue.rs:237 (`RecordingPresenter`, `request`) and scheduler.rs:1089 (`VirtualVideoClock`, `frame`, `frame_missing`, `frame_stream_base`) — these unit suites ran green in `orchrun-test-workspace-nofailfast.log.txt` (video-engine not among the 7 failing binaries).
- `frame()` byte math: PixelFormat::Nv12 at 2×2 → Y plane w×h = 4 bytes + UV plane (w/2)×(h/2)×2 = 2 bytes → `vec![0u8; 6]` (96) — correct.
- `VirtualVideoClock::set_ns` uses `fetch_max` (66) — monotonic by construction, matching its documented never-backwards contract; the system-monotonic master choice gives exact ns positions at rate 1 (38) — consistent with media-clock's semantics (my drift.rs/mapping.rs audits).
- `frame_stream_base` / `frame_missing` exercise the scheduler's normalize-at-push and missing-pts paths — genuine coverage shapes, not tautologies.
- The declared duplication (tests/common/mod.rs carries its own RecordingPresenter, 223+) is deliberate (ARCH-7: a cfg(test) module cannot be shared with an integration suite) and honestly documented at 1-3 — verified the copy exists.

## 3. Findings

### F-001 [S4]
Where: crates/video-engine/src/test_support.rs:164-170 + 183-192
Code:
```rust
    /// A presenter that refuses the `fail_at`-th request (0-based).
    pub fn failing_at(fail_at: usize) -> RecordingPresenter {
```
and
```rust
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        if self.fail_at == Some(self.log.len()) {
            return Err(presenter_failure());
        }
        self.log.push((request.when.ticks, request.generation));
```
What is wrong: the doc comment promises the presenter refuses exactly the `fail_at`-th request. The implementation refuses the `fail_at`-th and EVERY subsequent request: a refused request is never pushed to `log`, so `log.len()` stays at `fail_at` forever and every later call fails too. It is a persistent-from-index failure, not a one-shot.
Why it matters: the sole current user (queue.rs:370) swaps to a healthy presenter after the refusal, so no test is wrong today — but any future retry-semantics test (present fails once, retry succeeds) will silently test a permanently-failing presenter and can never observe recovery against the same sink. A test-support fixture whose doc misstates its behavior is a trap.
What correct looks like: advance a separate `attempts` counter (or push a tombstone) on failure so only the indexed request is refused; or fix the doc to "refuses the `fail_at`-th request and every request after it".
Confidence: HIGH — Verification: read; behavior traced through both present calls; queue.rs:360-393 read to confirm the current test is unaffected.
### F-002 [S4]
Where: crates/video-engine/src/test_support.rs:5,70-77,177-180
Code:
```rust
#![allow(dead_code)]
```
with (70-72):
```rust
    /// Advance the virtual clock by a delta.
    pub fn advance_ns(&self, by: u64) {
        let _ = self.source.0.fetch_add(by, Ordering::Relaxed);
    }
```
What is wrong: the module-wide allow hides that three helpers have zero users in the crate: `VirtualVideoClock::advance_ns` (70-72), `VirtualVideoClock::now_ns` (75-77), and `RecordingPresenter::gens_seen` (178-180) — no occurrence anywhere in video-engine/src outside this file.
Why it matters: dead test helpers accumulate; the blanket allow means future dead code is invisible too. The scheduler tests use only `set_ns`/`set_generation` (verified by grep over scheduler.rs/queue.rs).
What correct looks like: delete the three dead helpers and narrow the allow to the module doc's actual justification, or drop the allow entirely and let the compiler keep the fixtures honest.
Confidence: HIGH — Verification: rg for each name across video-engine/src excluding this file → no matches.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FakeMonotonic` + `MonotonicSource::now_ns` | 26-32 | REAL | — | Relaxed atomic load — fine for a single-threaded test source |
| `VirtualVideoClock` (struct) + `new` | 39-55 | REAL | — | system-monotonic master at rate ONE — exact-ns contract holds |
| `VirtualVideoClock::clock` | 58-60 | REAL | — | The scheduler-facing handle |
| `VirtualVideoClock::set_ns` | 65-67 | REAL | — | fetch_max — monotonic-safe, documented |
| `VirtualVideoClock::advance_ns` | 70-72 | REAL | S4 | Dead (F-002) |
| `VirtualVideoClock::now_ns` | 75-77 | REAL | S4 | Dead (F-002) |
| `VirtualVideoClock::set_generation` | 81-83 | REAL | — | Used (scheduler.rs:1291) |
| `frame` | 92-102 | REAL | — | NV12 2×2/6 bytes — correct |
| `frame_stream_base` | 106-111 | REAL | — | Arbitrary-base pts for normalize coverage — used |
| `frame_missing` | 114-119 | REAL | — | Missing-pts path — used |
| `request` | 122-128 | REAL | — | Used by queue tests |
| `presenter_failure` | 136-145 | REAL | — | Structured, no secrets — asserted verbatim by queue.rs:374-376 |
| `RecordingPresenter` + `new`/`failing_at`/`slots_seen`/`gens_seen` + `Presenter::present` | 150-193 | REAL | S4 | F-001 (failing_at semantics); gens_seen dead (F-002); present logs (when.ticks, generation) |

## 5. Test honesty
This file IS the test machinery; honesty = do the tests using it actually constrain behavior?
| Consumer test | Constrained? | Reason |
|---|---|---|
| queue.rs `presenter_failure_keeps_the_refused_request_at_the_front` (362-393) | YES | Asserts delivered-count, verbatim error code, queue retention, and in-order resume ([2,3]) — the RecordingPresenter's log makes the ordering claims checkable |
| scheduler.rs unit suite (1089+) | YES | Virtual-clock positions are exact; set_ns/set_generation drive the generation and due-time edges |
| Baseline: video-engine green in `orchrun-test-workspace-nofailfast.log.txt` (not among the 7 failures) | — | cited |
| Note: no test exercises `failing_at` retry-then-succeed semantics — the trap F-001 describes is untested by construction | — | gap |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "cfg(test)-gated; dev-only types can never leak into the public API" | module doc 1-3 | REAL | lib.rs:181-182 `#[cfg(test)] mod test_support;` |
| "the integration suites under tests/ carry their own copy" | module doc 2-3 | REAL | tests/common/mod.rs:223 `failing_at` copy verified |
| "A presenter that refuses the `fail_at`-th request (0-based)" | doc 164 | FALSE (understates) | F-001: refuses that request and all later ones |
| "monotonic-safe: never backwards" (set_ns) | doc 62-64 | REAL | fetch_max at 66 |

## 7. Dead / duplicated / unreachable code
- Dead: `advance_ns`, `VirtualVideoClock::now_ns`, `gens_seen` (F-002) — hidden by the blanket allow.
- Duplicated: RecordingPresenter exists here and in tests/common/mod.rs — declared, intentional (ARCH-7), and the copies have already drifted slightly (tuple record here vs named fields there) — acceptable, noted.

## 8. Dependencies & flags
- Uses `testkit::Must` — the workspace's own assertion helper; no external test crates.
- `#![allow(dead_code)]` is the only lint suppression (F-002).
- No unsafe, no feature gates.

## 9. Open questions for the next agent
- Q-1 (video-engine queue/scheduler owner): when a presenter retry-succeeds test is written, fix `failing_at` first (F-001) or the test will pass against a permanently-failing presenter only if it expects permanent failure.
- Q-2 (testkit owner): the module doc says "local to this crate until testkit grows it" (148-149) — the FrameRecorder shape now exists in two copies; the graduation ticket should be tracked.
