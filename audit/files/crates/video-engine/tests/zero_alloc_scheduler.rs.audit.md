# AUDIT: crates/video-engine/tests/zero_alloc_scheduler.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 150 / 2 callables (1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — runs in the workspace lane: baseline cargo-test-workspace.txt:6547-6550 `dropping_ticks_allocate_nothing_after_warm_up ... ok`. NOTE: the assignment sheet named this file "zeroalloc_scheduler.rs"; the real path is `zero_alloc_scheduler.rs` (audited at the real path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
A textbook zero-allocation proof: a real `PresentationScheduler` over a real
`MediaClock` (system-monotonic master on a settable fake source —
tests/common/mod.rs:39-94 builds it with `MediaClock::system_monotonic`),
128 cycles of aggressive dropping work (16-frame late storms, 4-frame
supersede bursts, quiet repeats) executed under a counting global allocator
that asserts ZERO bytes allocated after an honest warm-up (window growth,
first-present slot), with per-cycle behavioral asserts (drop counts exact,
presents exact) and a full post-window stats ledger (pushed 2580 = 129×20,
late 2064, superseded 387, presented 129, stale 0, pending 0) that would catch
any silently-ignored push. The ignored `let _ = scheduler.push(...)` results
are covered by that ledger. No defects found.

## 2. What actually works (verified)
- Baseline (:6547-6550): the binary ran, `dropping_ticks_allocate_nothing_after_warm_up ... ok`.
- The zero-alloc claim is MEASURED, not argued: `#[global_allocator]
  CountingGlobalAlloc` (:12-14) + `guard.assert_zero(...)` (:135-137) — the
  module doc's "measured by the counting global allocator" (:1-4) is the
  literal mechanism.
- The clock is the real production type: tests/common/mod.rs:48-50 constructs
  `MediaClock::system_monotonic` (my clock.rs audit's reachable constructor)
  over a `FakeMonotonic` atomic — not a scheduler-local time fake.
- Arithmetic checked: CYCLES×3 = 384 dropping ticks (matches the doc's claim);
  warm-up + 128 cycles = 129 storms; 129×(16+4) = 2580 pushed; 129×16 = 2064
  late; 129×3 = 387 superseded — all pinned at :143-149.
- One-#[test]-per-binary discipline (:9-10) held: exactly one test in the file
  (the allocator is process-global).

## 3. Findings
None. Exactly what was checked, and how:
- Fakeness: no todo!/unimplemented!/unreachable-in-prod paths; the single
  `unreachable!`-free body; pools built outside the measured window with the
  Arc-refcount rationale verified (SharedBytes is an Arc — media-types).
- Correctness: drop/supersede/present counts asserted per cycle AND in
  aggregate; the ignored push outcomes are redeemed by the `stats.pushed`
  equality (:144) — a refused or lost push would break it.
- Test honesty: warm-up separates one-time costs (documented at :78-80);
  the measurement window covers pushes, ticks, and presents alike.
- Perf claim: "allocate zero bytes" is exactly what assert_zero checks;
  the doc's ED-FIX-2 provenance (old per-tick Vec materializations removed)
  is the kind of claim this test would falsify if regressed.
- Threading: serialize_tests() taken; no sleeps; deterministic (clock parked
  at 10_000×FRAME, no writes inside the window — documented :60-62).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cfg | 40-51 | REAL | — | doc 03 §4.10(e) default 30 fps shape; arithmetic local |
| dropping_ticks_allocate_nothing_after_warm_up | 53-150 | REAL | — | warm-up + 128 cycles + zero-alloc guard + stats ledger |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| dropping_ticks_allocate_nothing_after_warm_up | YES | any per-tick allocation (the ED-FIX-2 regression) trips assert_zero; any miscounted drop/present trips the per-cycle or ledger asserts |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "after one warm-up cycle, N aggressive dropping ticks ... allocate zero bytes, measured" | zero_alloc_scheduler.rs:1-4 | REAL | assert_zero :135; baseline green |
| "the old per-tick drops.late/superseded Vec materializations are gone (fixed-capacity inline trails)" | :5-7 | REAL-UNPROVEN (history) | the current behavior is proven; the historical fix is worklog provenance I did not independently verify |
| "One #[test] per binary by design" | :9-10 | REAL | exactly one test |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
video_engine (PresentationScheduler, DropPolicy, PresentAction, SchedulerCfg),
media-types, testkit (CountingGlobalAlloc, Must, serialize_tests),
crate::common (VirtualClock). No env knobs, no features.

## 9. Open questions for the next agent
- The scheduler itself (video-engine/src/*) is not my file: does
  `PresentationScheduler::tick` ever allocate on paths NOT exercised here
  (stale drops, generation mints)? This proof covers late/supersede/repeat
  only — `dropped_stale` is asserted zero, so the stale path's allocation
  behavior is unmeasured.
- The scheduler consumes `PlaybackClock::now()` per tick — with the shipped
  CLI's step-counter clock (my clock.rs audit), does the scheduler's
  vsync-then-advance ordering still hold when the counter jumps by whole
  quanta between pumps? (scheduling.rs / sync_scenarios.rs owners.)
