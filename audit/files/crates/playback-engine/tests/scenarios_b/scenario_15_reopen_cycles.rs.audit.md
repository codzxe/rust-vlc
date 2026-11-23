# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_15_reopen_cycles.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 157 / 2 (1 `#[test]`) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test lane (scenarios_b suite). Code under test shipped: the command layer (`Command::SessionOpen/PlaybackStop/SessionClose`) over the same engine the CLI host drives (apps/cli/src/dispatch.rs:81-131). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Disk full — no cargo runs. Evidence: baseline cargo-test-workspace.txt scenarios_b section: `test scenario_15_reopen_cycles::thousand_reopen_cycles_return_every_counter_to_baseline ... ok`, suite `23 passed ... in 5.06s` (so the 1_000-cycle loop fits the "well under 10 s" comment). |

## 1. One-paragraph verdict
A real, exact leak oracle: 1_000 full open→settle→stop→close cycles on ONE engine (the reuse is the point), with allocator live-bytes asserted EQUAL to the post-warmup baseline at three plateau checkpoints and at the end, plus fd-count, thread-count, outbox-drop, starvation, and exact frame-accounting asserts (2 frames × 1_000 cycles). The single weakness is cosmetic: the doc row's "open/play/stop" step is conditional (a clip settled in `Ended` skips the stop), and the test neither counts nor bounds how often the stop op actually ran — unlike the soak's D-6 skip discipline.

## 2. What actually works (verified)
— Not executed by me (disk full). Baseline: `scenario_15_reopen_cycles::thousand_reopen_cycles_return_every_counter_to_baseline ... ok` within the scenarios_b suite (23 passed, 5.06s — cargo-test-workspace.txt scenarios_b section). Read-verified: every assert in the body is exact-equality (no `>=` weaseling except none at all).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_15_reopen_cycles.rs:71-81
Code:
```
    // The doc's "stop" step: from Playing it is the mid-play teardown
    // (flush, generation bump); a clip that already drained to Ended
    // has no pipeline left to stop — Close from Ended is the exit.
    if settled == PlaybackState::Playing {
```
What is wrong: the stop step is conditional and uncounted. The doc row (doc 10 §6.4 row 15) says "1000× open/play/stop", but a cycle that settles directly in `Ended` performs open/play/close with NO stop; the test accepts `Playing | Ended` (line 68) and never counts stop executions. If a regression made the 1-frame clip settle in `Ended` on every cycle, the stop lane (mid-play teardown: flush + generation bump) would silently vanish from this scenario and the test would stay green. Mitigations: the module doc is honest about the dual path, and the mid-play stop IS covered elsewhere (session_flow `close_from_playing_is_the_two_step_composition`, soak scenario's active closes). What correct looks like: a `stops_executed` counter with a lower bound, mirroring the soak's D-6 skip discipline. Confidence: HIGH — Verification: read (+ `ScenarioHost::settle` semantics read: stops at the first non-Opening/Buffering state, host.rs:1112-1121, so which branch runs depends on tick timing the test does not pin).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cycle | 50-97 | REAL | S4 | open Accepted; settle Playing\|Ended; conditional stop (F-001); close → Idle; `session_id().is_none()` |
| thousand_reopen_cycles_return_every_counter_to_baseline | 101-157 | REAL | — | 3 warm-ups; baseline (live bytes/fds/threads/frames); 1_000 cycles; plateau asserts at 333/666/999 (exact equality); end asserts: live bytes, fds, threads, outbox 0/0, starve 0, frames == 2_000 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| thousand_reopen_cycles... | YES | Mutation reasoning: any allocation retained across a cycle (session struct, decoder, queue, handle, event) drifts live bytes → checkpoint/end exact-equality fails; an unclosed file descriptor fails the fd assert; a leaked thread fails the thread assert; outbox overflow (dropped events) fails 149-150; a skipped decode (frames < 2_000) or double delivery (> 2_000) fails 152-156; a failed open/stop/close fails the per-cycle asserts. The one blind spot is the stop-op count (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "1000× open/play/stop" served in full, "not a reduced one" | module docs 7-15 + doc 10 §6.4 row 15 | PARTLY — count is full (CYCLES=1_000, line 43), stop step conditional (F-001) | read; baseline green (5.06s suite) |
| "live-bytes reading returns to baseline at three sampled checkpoints *and* at the end" | module docs 20-23 | REAL | lines 123-129 (thirds) + 133-137 (end), exact equality |
| "the poll-model engine is threadless; the count must not move" | module docs 26-27 | REAL (asserted) | line 144; baseline green |
| "zero dropped events/receipts ... zero starvation episodes ... frames exactly per-cycle accounting" | module docs 28-32 | REAL | lines 149-156 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
commands (wire command layer), testkit (alloc_count::serialize_tests — the process-global allocator serializer, correctly taken), crate::alloc_state + crate::host (ScenarioHost, TempDir, dispatch, open_fds, thread_count — sibling modules, not my files). No unsafe, no features, no #[ignore].

## 9. Open questions for the next agent
- scenarios_b/host.rs (not my file): does `build_short_avi(_, 1)` produce exactly 1 video + 1 audio packet (FRAMES_PER_CYCLE=2)? The exact frame assert passing in baseline implies yes — confirm at the builder.
- If the settle-then-stop timing ever flips to Ended-first on some platform, F-001 becomes a live coverage hole; a stop counter would close it.
