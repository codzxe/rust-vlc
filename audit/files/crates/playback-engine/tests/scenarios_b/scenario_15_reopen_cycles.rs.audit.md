# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_15_reopen_cycles.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 157 / 1 test fn + 1 cycle helper |
| File verdict | REAL (strong leak-regression test; two platform/shape caveats) |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]` + `serialize_tests`; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read; cross-check of `open_fds`/`thread_count` Option semantics (host.rs:2292-2307) and `settle`'s stopping condition (host.rs:1114-1121); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

Doc 10 §6.4 row 15 (reopen cycles): one engine, 1 000 full open→settle→stop→close cycles over a genuinely well-formed 1-frame AVI built by the harness's byte surgery, with exact in-process leak oracles — allocator live bytes must equal the post-warm-up baseline at three plateau checkpoints and at the end, fd count and thread count must return unmoved, and the engine's own counters must show zero dropped events/receipts, zero starvation, and exactly 2 000 delivered frames (no skips, no double deliveries). This is the right leak test: same-engine reuse (a fresh engine per cycle would prove nothing), warm-up before baseline, plateau checkpoints that catch grow-then-free patterns the end-check alone would miss. Two caveats: the fd/thread oracles pass vacuously on non-Linux (both `Option`s are `None`), and the doc row's "stop" step is conditional on the 1-frame clip not draining within `settle` — disclosed in a comment, but the row's letter is not guaranteed every cycle.

## 2. What actually works (verified)

- One `ScenarioHost`/`Engine` across all cycles (107, 119-130) — the engine's own session teardown is what the test exercises; the doc header states this reasoning (7-10).
- Exact leak oracles: live bytes `assert_eq!` at ⅓ checkpoints (123-129) AND end (133-137); `outbox_dropped_events/receipts == 0` and `starve_episodes == 0` (149-151); `frames_delivered` delta exactly `CYCLES × 2` (152-156) — the frame accounting is bidirectional (catches lost AND duplicated frames).
- Per-cycle invariants in `cycle` (50-97): open accepted, settled state is Playing|Ended (the honest pair for a 1-frame clip), stop only from Playing (with the reason documented 71-73), close ok, state Idle, no session survives.
- The fixture is `build_short_avi("reopen-1frame.avi", 1)` — real byte surgery (host.rs:2162-2287), declared spans honest, so each cycle is a genuine full pipeline run, not a stub open.

## 3. Findings

### F-001 [S4] fd and thread oracles pass vacuously on non-Linux
Where: crates/playback-engine/tests/scenarios_b/scenario_15_reopen_cycles.rs:140-144; host.rs:2292-2307
Code:
```rust
    assert_eq!(open_fds(), fds, "the fd count returns to baseline");

    // Thread counters: nothing leaked (the engine is threadless; the
    // harness thread pool is stable under the serializer).
    assert_eq!(thread_count(), threads, "the thread count never moved");
```
What is wrong: `open_fds()`/`thread_count()` return `Option<usize>` — `None` off Linux (host.rs documents this). On macOS/Windows both sides of each assert are `None`, so the fd and thread leak oracles assert nothing on those platforms. The doc header presents them as unconditional oracles (24-27).
Why it matters: The test's own docs imply platform-independent coverage; two of its four counter families are Linux-only. The allocator oracle (the strongest) is platform-independent, so the test is not hollow — but the fd/thread lines are decorative off Linux. Hygiene/disclosure.
What correct looks like: `match (open_fds(), fds) { (Some(a), Some(b)) => assert_eq!(a,b), (None, None) => /* skip: not Linux */ , ... }` with a comment, or gate the asserts on the oracle being available.
Confidence: HIGH — Verification: read (Option semantics).
### F-002 [S4] The "stop" step of the documented cycle is conditional
Where: crates/playback-engine/tests/scenarios_b/scenario_15_reopen_cycles.rs:66-81
Code:
```rust
    let settled = host.state();
    assert!(
        matches!(settled, PlaybackState::Playing | PlaybackState::Ended),
        "cycle {index}: the 1-frame clip settles active or drained (got {settled:?})"
    );
    // The doc's "stop" step: from Playing it is the mid-play teardown
    // (flush, generation bump); a clip that already drained to Ended
    // has no pipeline left to stop — Close from Ended is the exit.
    if settled == PlaybackState::Playing {
```
What is wrong: The doc row says "1000× open/play/stop". If the 1-frame clip transitions Buffering→Playing→Ended inside a single poll before `settle` observes Playing, that cycle's stop (the mid-play teardown path: flush, generation bump) never runs and the cycle degrades to open/play/close. The comment explains why that is honest for a drained clip, but nothing counts how many cycles took which arm — the test cannot tell the reader whether the stop path ran 1 000 times or zero times.
Why it matters: The stop-path teardown (a distinct engine path from close-from-ended) may be under-exercised by an unknown amount. Hygiene: measure, don't assume.
What correct looks like: Count the arms (`let mut stopped = 0; … stopped += 1;`) and assert `stopped > 0` (or a minimum fraction) — or use a longer fixture that guarantees Playing at settle.
Confidence: MEDIUM — Verification: read (settle's first-Playing-observation semantics at host.rs:1114-1121 make the Ended arm reachable in principle).

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `cycle` | 50-97 | REAL | Full command-layer cycle with per-step asserts; F-002's conditional stop. |
| thousand_reopen_cycles_return_every_counter_to_baseline | 102-157 | REAL | Warm-up + baselines + plateau checkpoints + four counter families; F-001 platform vacuity. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| thousand_reopen_cycles… | YES | Per-cycle leak → checkpoint or end assert; fd leak → Linux fd assert; event/receipt overflow → engine counter asserts; frame skip/duplication → exact 2 000 delta; failed teardown → state/session asserts inside cycle. Off Linux the fd/thread lines are vacuous (F-001). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "1000× open/play/stop" served, not reduced (7-15) | module doc | SHALLOW | The count is served; the stop step is conditional per cycle (F-002). |
| "thread/fd/byte counters return to baseline" (4-5) | doc 10 row 15 / module doc | SHALLOW | Byte: exact, all platforms. fd/thread: exact on Linux, vacuous elsewhere (F-001). |
| "the doc's full count is served, not a reduced one" (14-15) | module doc | REAL-UNPROVEN | CYCLES = 1_000 pinned; "well under the 10 s bound" is a runtime claim I could not verify (no cargo). |
| "zero dropped events/receipts … zero starvation episodes" (28-29) | module doc | REAL | 149-151 asserts. |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

Harness + commands + testkit counting allocator. `/proc`-dependent oracles degrade to None (F-001). No feature gates.

## 9. Open questions for the next agent

- W03 colleague with session.rs teardown: is stop-from-Playing's teardown path shared with close-from-Ended below some seam (making F-002 immaterial), or genuinely distinct code?
- W20: when re-running this suite, record how many cycles took the stop arm (one-line instrumentation would close F-002's unknown).
