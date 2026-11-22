# AUDIT: crates/packet-queue/tests/zero_alloc_queue.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 149 / 1 test + 2 helpers + global allocator |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — test binary with `#[global_allocator]` (testkit counting allocator); W00 baseline: tests 1751/2/25 workspace-wide, this binary's individual result UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `cat` full file (exit 0). No cargo. |

## 1. One-paragraph verdict

A genuine allocation measurement: a counting global allocator guards a 100k-round steady-state push/pop cycle (both push flavors, pops, watermark/stats/usage queries — the engine tick's queue surface) over a half-filled queue and asserts exactly zero allocations. The settle+replay discipline ("D-5") handles libtest's own foreign thread-spawn noise, and one-test-per-binary plus `serialize_tests()` respects the process-global allocator. Two observations: the replay backstop can mask a once-per-process allocation inside the measured section (out of the AC's steady-state scope, but worth naming), and — as in the property suite — the parking flavor only ever runs with room available, so the park/wake path's allocation behavior under actual contention is unmeasured.

## 2. What actually works (verified)

- The measurement is exact, not sampled: `ALLOC.state().guard()` … `guard.delta()` around the whole 100k loop; final assert `(alloc_calls, total_bytes) == (0, 0)` with a message naming the scope (144-148).
- Warm-up (101-114) separates one-time lazy internals from the steady state before the guard opens — the right methodology.
- Net-zero fill discipline (+2/−2 per round at half capacity, comment 121-123) keeps both push flavors on the accept fast path for the entire window — the claimed hot path is the one measured.

## 3. Findings

### F-001 [S4] The replay backstop masks once-per-process allocations in the measured section
Where: crates/packet-queue/tests/zero_alloc_queue.rs:45-59
Code (45-58, verbatim):
```rust
fn clean_replay(mut section: impl FnMut() -> AllocDelta) -> AllocDelta {
    settle_foreign_spawns();
    let first = section();
    if first.alloc_calls == 0 && first.total_bytes == 0 {
        return first;
    }
    settle_foreign_spawns();
    let second = section();
    assert!(
        second.alloc_calls == 0 && second.total_bytes == 0,
```
What is wrong: a defect that allocates exactly once per process (a lazily-initialized cache inside `try_push`, say) dirties the first window, then replays clean and passes. The D-5 doc frames the replay as hardening "never weakens the proof", and strictly the AC is steady-state (a one-time init is arguably outside it) — but the proof's own warm-up precedent (101-114) shows one-time allocations are considered in-scope when known. A stricter form would fail on any dirty first window after a longer settle, and re-spawn the queue per replay.
Confidence: HIGH — Verification: read.
Severity: S4 — the masking class is out of the AC's literal scope.

### F-002 [S4] The park-with-contention path's allocation behavior is unmeasured
Where: crates/packet-queue/tests/zero_alloc_queue.rs:128-132
Code (verbatim):
```rust
            let parked = queue.push_wait(
                Stamped::new(GenerationId::FIRST, Blob { id: round + 1 }),
                deadline(),
            );
            assert!(parked.is_ok(), "room exists: the park is skipped");
```
What is wrong: the assert's own message concedes it — the park is always skipped because room always exists. The future threaded demux (2.E1) will park for real; whether an actual condvar wait allocates (e.g., through timing paths) is unknown to this proof.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ALLOC` static | 30-31 | REAL | — | process-global counting allocator |
| `settle_foreign_spawns` | 37-39 | REAL | — | 30 ms sleep; empirical numbers in doc (2-10 events / 55 B-10 KiB without; 40/40 clean with) |
| `clean_replay` | 45-59 | REAL | S4 | F-001 |
| `Blob` + `Weigh` | 61-74 | REAL | — | constant 4 B / 1000 ns |
| `steady_state_push_pop_cycle_allocates_nothing` | 81-149 | REAL | S4 | exact zero-alloc assert; park always skipped (F-002) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| steady_state_push_pop_cycle_allocates_nothing | YES for per-round heap churn in push/pop/watermark/stats/usage (reasoned — any allocation in the 100k loop dirties the window and reproduces on replay); NO for once-per-process allocation (F-001); NO for park-under-contention (F-002) |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the push/pop loop over a pre-filled BoundedQueue allocates nothing" (design E3.2a) | doc 1-8 | REAL-UNPROVEN | methodology read-verified; run result UNVERIFIED (no cargo) |
| "the parked producer's wake path is the same mutex/condvar arithmetic exercised here" | doc 6-8 | SHALLOW | the arithmetic is the same only on the fast path; the actual park/wait is skipped every round (F-002) |

## 7. Dead / duplicated / unreachable code

None. The `Must` import is used (86-90).

## 8. Dependencies & flags

packet-queue, media-types, testkit (alloc_count). No features. Discipline documented: one `#[test]` per binary because the allocator is process-global; `serialize_tests()` enforces it.

## 9. Open questions for the next agent

- Do the demux/engine zero-alloc binaries (referenced at 16-18) share the `clean_replay` masking property? Files: the demux zero-alloc test binary (other agent).
