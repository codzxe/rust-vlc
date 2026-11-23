# AUDIT: crates/audio-output/tests/callback_budget.rs
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-23 |
| Lines / functions | 379 / 4 tests + 2 helpers |
| File verdict | REAL (as an operation-budget proof of the pure render core) — the "real-time safety" framing beyond that is aspiration, honestly labeled |
| Reachable from shipped entry point? | YES for the code under test: render_into is the null-backend render path (crates/audio-output/src/null.rs:301-306) reached from AudioBackend::open (used by the CLI first-vertical headless aout); the test itself runs in cargo test --workspace |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | sed -n (quotes, all verified); read of src/render.rs, src/ring.rs, src/counter.rs, src/null.rs (call-graph check); baseline citation audit/runs/baseline/cargo-test-workspace.txt:190-196 (4/4 ok, exit_code=0 at line 240) |

## 1. One-paragraph verdict
This is the honest version of a real-time-safety test: it proves, by execution,
an OPERATION budget (ring segments ≤ 2, whole-frame accounting, exactly one
counter advance, ≤ 1 underrun count, silence-on-underrun policy, pause =
zero-op) for the pure callback core `render_into`, driven synchronously from
the test thread over 180 real callbacks, a starved sweep, and a geometry sweep
(channels × capacity × request sizes incl. requests > ring capacity). It does
NOT and cannot prove real-time safety in the OS sense: no wall-clock/WCET
measurement exists (admitted, deferred to Phase 3), the allocation budget is a
structural signature argument — not a trapping allocator (admitted,
[PROPOSAL: tooling]) — and there is no real audio adapter anywhere (cpal NOT
admitted per docs/feature-matrix.toml:2362). The structural argument verified
by my read: render_into's entire call graph is ring.read (atomics on a
Box<[AtomicU32]> fixed at construction), counter.advance (fetch_add), one
fetch_add, slice writes — nothing allocates or locks. One assertion sold as
"zero storage growth ... every call" is an algebraic tautology that cannot
fail (F-001). Answer to the hostile question: operation-count proof = REAL;
allocation proof = structural, verified by review, untrapped; time proof =
absent. Real-time safety overall = aspiration, correctly scoped in every doc
that cites it.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:190-196 — "running 4
  tests ... ok" ×4; package exit_code=0 (line 240).
- Call-graph no-alloc/no-lock check: read render.rs:89-120, ring.rs:113-190
  (atomics only, slots boxed at new()), counter.rs:68-70 (fetch_add),
  null.rs:292-313 (capture extends AFTER render_into returns) — the doc's
  structural claim checks out line-by-line.
- Mutation reasoning: NAN-prefilled output buffers (lines 213, 238, 363) mean
  a renderer that skips writing silence or touches paused buffers fails; the
  capture-equality assert (line 180) pins the full 88 200-frame payload
  byte-exact; counter/underrun reads bracket every call, so "exactly one
  advance" is measured, not self-reported by one side.

## 3. Findings
F-001 [S3]
Where: crates/audio-output/tests/callback_budget.rs:159-165 and 183-187
Code:
```
            // Zero storage growth: the ring's shape is frozen at
            // construction — buffered + free == capacity, every call.
            assert_eq!(
                stream.device.buffered_frames() + free_frames(&stream, RING),
                RING
            );
```
with
```
fn free_frames(stream: &audio_output::NullStream, capacity: usize) -> usize {
    capacity - stream.device.buffered_frames()
}
```
What is wrong: `free_frames` is DEFINED as `capacity - buffered_frames()`, so
the assertion is `b + (RING - b) == RING` — true for every possible ring state,
including a hypothetically growing one. It cannot fail; it proves nothing
about storage growth. The real invariant the comment invokes (slots fixed at
construction, ring.rs:57 `slots: Box<[AtomicU32]>`, ring.rs:77-82 allocated
once in `new`) is true but is proven only by code review, not by this
assertion. Why it matters: the module header (lines 23-24) claims "the ring's
slot count and capacity are proven constant across every callback of the
driven session (zero storage growth)" and worklog archive-2026-10.md:1434
repeats "zero storage growth checked every call" — an evidence string resting
on a vacuous assertion. What correct looks like: assert
`stream.device.ring_capacity() == RING` (a real frozen-geometry probe) or pin
`ring.available_frames() <= RING` from the ring itself; the sweep test's line
318 (`assert_eq!(ring.capacity_frames(), capacity)`) is the honest form.
Confidence: HIGH — Verification: read (both functions + null.rs:142-144
buffered_frames → ring.available_frames).

F-002 [S3]
Where: crates/audio-output/tests/callback_budget.rs:1-9 (suite charter)
Code:
```
//! AC suite 2 — the **callback budget** (card 2.C3): "the callback
//! path instrumented (operation/allocation budget per doc 03 §4.9)
//! and proven within budget on the null adapter".
```
What is wrong: nothing in the text is false — but the reachable proof is an
operation ledger for a pure function called synchronously; the suite runs on
no callback thread, under no scheduler, with no priority, on the only backend
that exists (null). The allocation half is untrapped and the time half absent
(both admitted below in the same header). Any reader citing "callback budget
proven" as real-time safety is over-reading it; docs/feature-matrix.toml:2362
correctly stops at "partial-seam, not done ... no real-device evidence
exists". What correct looks like: exactly what Phase 3 owes — WCET numbers
per platform, a #[global_allocator] trap, and the cpal/direct adapters.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/audio-output/tests/callback_budget.rs:65-80 (ledger checks)
Code:
```
    assert!(
        stats.ring_segments <= 2,
        "ring segments {} > 2 (wrap bound violated)",
        stats.ring_segments
    );
```
What is wrong: the ledger is self-reported by the code under test
(`RenderStats` is produced by render_into itself). If the ring looped more
but miscounted `segments`, or allocated while counting correctly, no assertion
here notices. This circularity is inherent to instrumentation and the doc
(§4.9 c) names code review as the co-gate — recorded as a limitation, not a
defect. The independent cross-checks that DO exist: capture equality (line
180), counter bracketing (lines 142-151), NAN canaries.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/audio-output/tests/callback_budget.rs:79
Code:
```
    assert!(stats.silence_filled <= stats.frames_requested);
```
What is wrong: strictly implied by the frames-accounting equation asserted at
lines 72-76 (`frames_read + silence_filled == frames_requested` with
frames_read ≥ 0). Redundant assert; harmless.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| assert_within_budget (helper) | 56-95 | REAL | S4 | bracketed deltas; one redundant assert (F-004); self-reported ledger (F-003) |
| s16_to_device_f32 (helper) | 100-107 | REAL | — | duplicates common::s16_stream_to_f32_bytes byte-for-byte (S4-level dup, see §7) |
| callback_ledger_within_budget_over_a_fully_driven_session | 109-181 | REAL | S3 | 180 callbacks, ledger per call, accounts close 88 200 = 180×490, capture byte-exact; tautological growth assert (F-001) |
| free_frames (helper) | 185-187 | REAL | S3 | makes F-001 vacuous by construction |
| starved_callback_fills_silence_and_still_honors_the_budget | 189-263 | REAL | — | data-then-zeros ordering, NAN canary, 21 events / 9 375 instants exact |
| callback_budget_holds_across_a_geometry_sweep | 265-323 | REAL | — | 3×3×6 geometries incl. request > capacity; honest frozen-capacity assert (line 318) |
| paused_device_is_a_zero_operation_callback | 325-379 | REAL | — | default ledger, counter frozen, output NAN-untouched, resume plays frozen frame |

## 5. Test honesty
All 4 pass in baseline (cargo-test-workspace.txt:192-195). Would they fail if
the code were broken? Yes for: wrong segment bound (if counted honestly),
frame tearing, silence-fill omission, stale-audio-on-underrun, counter drift,
underrun miscount, pause pulling data, resume losing buffered data, capture
corruption. No for: an allocation inside render_into (untrapped, F-002), a
miscounted segment loop (F-003), ring storage growth (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the callback path instrumented ... and proven within budget on the null adapter" | file header:1-3 quoting card 2.C3 | REAL | 4/4 baseline, ledger asserts per call |
| "it *cannot* allocate (the structural half)" | file header:21-22 | REAL-UNPROVEN | call graph verified by read (render.rs/ring.rs/counter.rs); no allocator trap exists |
| "zero storage growth ... proven constant across every callback" | file header:23-25 | FALSE-as-proof / TRUE-as-fact | the asserting line is a tautology (F-001); the fact holds by construction (ring.rs:57,77-82) — proven only by review |
| "wall-clock asserts are deliberately absent" | file header:31-34 | REAL (honest omission) | no timing assertion in file |
| "status is partial-seam, not done — ... no real-device evidence exists" | docs/feature-matrix.toml:2362 | REAL | cpal.rs marker refuses; null is the only backend |
| "zero storage growth checked every call" | docs/worklog/archive-2026-10.md:1434 | MISLEADING | rests on F-001's vacuous assert |

## 7. Dead / duplicated / unreachable code
`s16_to_device_f32` (lines 100-107) is a byte-identical re-implementation of
`common::s16_stream_to_f32_bytes` (tests/common/mod.rs, "pinned f32
conversion" section) which this same test file imports `common` for — the dup
is deliberate-looking (local doc comment says "value-proven by the sine-hash
suite") but is duplication: one conversion, two sources of truth (S4).

## 8. Dependencies & flags
Deps: audio-output, media-clock, media-types, testkit (dev). No features. The
suite is std-only; no unsafe; no threads (the "callback" is the test thread).

## 9. Open questions for the next agent
- Phase 3 owes: WCET per platform + trapping allocator. Neither exists yet —
  track as debt against docs/gates/phase2-exit.md:26 ("audio on screen
  (audible) MET (mock-labeled)").
- When cpal/direct adapters land, this suite must be re-run against the REAL
  callback thread (device-driven, not test-driven) — nothing today exercises
  preemption or priority inversion.
