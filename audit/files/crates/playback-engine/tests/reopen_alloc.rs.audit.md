# AUDIT: crates/playback-engine/tests/reopen_alloc.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 154 / 1 test + 1 cycle helper |
| File verdict | REAL — the measured bounded-memory half of the 1000× reopen mandate: warm-up 100 cycles, then 1000 cycles under one allocator guard asserting `live_bytes == 0` |
| Reachable from shipped entry point? | N/A (test binary; one-test-per-binary allocator discipline) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (not executed — no cargo) |

## 1. One-paragraph verdict

The leak proof for session reopen: everything the open builds, the
close must free. After 100 warm-up cycles that size the reused pools,
1000 full open→play→stop→close cycles run under a
`CountingGlobalAlloc` guard and the window must return to its opening
`live_bytes` exactly (:137-145), with two sanity asserts that the
cycles really played and really allocated (:146-153). The cycle helper
fails loudly on any step (unreachable! with the error), asserts
Playing and Idle per cycle, and drains the outbox between cycles so
nothing is misattributed (:108-110). Same allocator-window caveat as
bounded_alloc.rs (F-001 there): net-zero, not churn-free — but here
the per-cycle transients are much larger (whole sessions), so a
small leak would show unless exactly masked by frees; the warm-up
baseline makes masking implausible.

## 2. What actually works (verified)

— Uses the shared `tests/common` harness (FakeDemuxer/FakeProvider/
Collector) — the honest fakes, real Engine.
— Per-cycle assertions: Playing reached (:85-89), Idle after close
(:102), frames > 0 per cycle (:126, :134) — the measured window
cannot silently degrade into no-op cycles.
— The functional half (epoch/seq continuity over the same cycles)
lives in model_walk.rs (verified there: `reopen_cycles_1000x_…`).
— Mutation reasoning (UNVERIFIED): a per-cycle leak of any retained
object (a decoder-cache entry, an outbox slot, a session bookkeeping
struct) grows `live_bytes` by 1000× its size → assert fails.

## 3. Findings

### F-001 [S4] The 24-poll bound inside `cycle` can silently pass through without reaching Playing on slow machines… no — it asserts; the real note is the outbox drain attribution
Where: reopen_alloc.rs:79-89 vs :108-110

Code:
```
    for _ in 0..24 {
        if engine.state() == SessionState::Playing {
            break;
        }
        engine.poll();
    }
    assert_eq!(engine.state(), SessionState::Playing, "the cycle reached playing");
```
What is wrong: nothing functional — the poll loop is bounded and the
assert is loud. The subtler issue is measurement hygiene: outputs are
drained AFTER close (:110), so a cycle whose events arrived late (in
the next cycle's polls) would be attributed to the next window — the
frozen virtual clock makes this deterministic here, but the pattern
is copy-paste-fragile for a future wall-clock variant.
What correct looks like: drain once before the guard opens and once
after close inside `cycle` (it does the latter; add the former when
adapting).
Confidence: MEDIUM — hygiene note, not a defect in this deterministic
harness.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `cycle` | 49-112 | REAL | — | loud failures; per-cycle state asserts; outbox drained |
| `reopen_cycles_are_allocation_neutral_in_steady_state` | 114-154 | REAL | S4 | warm-up excluded from the claim; sanity asserts included (F-001 note) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `reopen_cycles_are_allocation_neutral_in_steady_state` | YES | any retained-per-cycle allocation fails `live_bytes == 0`; vacuous runs fail the two sanity asserts |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a reopen cycle must be allocation-neutral in steady state" | :15-17 | REAL (per the test) | the guard + delta assert |
| "the functional half … lives in tests/model_walk.rs" | :5-7 | REAL | verified there (:828-894) |
| "the 4 h wall-clock soak stays 3.E4's" | :7 | UNVERIFIED | soak/ memory.rs is in my queue; a 4 h soak cannot run in this sandbox regardless |

## 7. Dead / duplicated / unreachable code

None (uses the common harness — no fake duplication).

## 8. Dependencies & flags

`playback_engine`, `plugin_api`, `media_types`, `commands`,
`testkit` (alloc_count), `mod common`. No feature gates.

## 9. Open questions for the next agent

- Wave 16: `AllocSnapshot`/guard semantics (testkit) — shared
  foundation of the three alloc-proof binaries.
