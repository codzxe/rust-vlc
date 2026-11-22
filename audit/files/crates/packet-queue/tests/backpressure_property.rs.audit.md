# AUDIT: crates/packet-queue/tests/backpressure_property.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 422 / 2 property tests + 5 helpers + oracle |
| File verdict | REAL-UNPROVEN (genuine property suite with model oracle; real parking untested) |
| Reachable from shipped entry point? | N/A — test binary (`cargo test -p packet-queue`); W00 baseline: workspace tests 1751 pass / 2 fail / 25 ignored (this binary's individual result UNVERIFIED — no cargo permitted). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | `cat` full file (exit 0); cross-ref queue.rs push paths (sed 392-470) and limits.rs audit. No cargo. |

## 1. One-paragraph verdict

A real seeded property suite, not a tautology: 8 fixed seeds × 4000 ops of adversarial interleavings (try-push with fresh/stale stamps, deadline-expired park, try-pop, parking pop, generation advance/stutter/regress flushes, close, burst mode) with three invariants asserted after **every** op — Q1 (usage ≤ caps), item conservation (`pushed == popped + resident + flushed`), and one-directional watermark laws — plus an independent oracle that recomputes the refusing axis on every `Full` and demands agreement. The weakness is what it does not test: no thread ever actually parks (every `push_wait`/`pop_wait` uses an already-expired or immediately-satisfied deadline), so the condvar wake-on-space/close/generation semantics — the "no deadlock" half of the very AC it cites — are untested in this file; and the generated item weights (≤ 24 B, ≤ 8 ns) never exceed the caps (96 B / 64 ns / 8 pkts), so the single-item oversize admission (limits.rs F-001) is never exercised.

## 2. What actually works (verified)

- The oracle (212-231): on every `Full`, the test recomputes all three axes in u128 from the post-refusal usage and asserts the queue's reported `binding` equals the oracle's. A wrong-axis or off-by-one refusal fails the test. Read-verified against limits.rs's `binding_axis` (identical arithmetic).
- Per-step invariants (296-298): Q1/conservation/hysteresis after every one of 32k ops — a drifting `flushed`/`popped` counter or a cap overshoot surfaces immediately with the seed and op index in the context string.
- Terminal conservation (303-316): close + full drain + `pushed == popped + flushed` with zero residents — the rejection accounting is pinned ("rejections never entered").
- `engine_shaped_low_is_reachable_for_every_mix` (326-422): the two wedge mixes (bytes-heavy duration-less; single items dwarfing the byte stall line) plus 50 seeded flood/drain cycles under engine-shaped marks — pins the packets-axis-low design decision against the all-axes-low stranding bug it fixed.

## 3. Findings

### F-001 [S3] The parking/waking semantics — the AC's "no deadlock" — are never actually parked
Where: crates/packet-queue/tests/backpressure_property.rs:154-160 (doc claim), 249, 270
Code (154-160, verbatim):
```rust
/// The extended model driver (design E1): adversarial interleavings of
/// the full backpressure vocabulary — try-push, the *bounded park*
/// (`push_wait` under an already-expired deadline: the deterministic,
/// sleep-free form of the parking producer's deadline path), try-pop,
/// the bounded parking pop, flush, close, and **burst mode** (a run of
```
Code (249, verbatim):
```rust
                    let outcome = queue.push_wait(item, Some(std::time::Instant::now()));
```
What is wrong: every `push_wait` in the suite passes an already-expired deadline (`Instant::now()`), so the condvar wait path — the code that parks a producer until space/close/generation movement — never executes under park pressure; `pop_wait(Some(Instant::now()))` likewise. The doc 13 §6 row this suite cites demands "property tests with synthetic producers/consumers — no deadlock": there are no consumers running concurrently with a parked producer anywhere in this file, so a lost-wakeup or condvar-misuse deadlock would pass this suite green.
Why it matters: the queue's most dangerous code (lock + condvar + generation wake discipline, queue.rs push_wait loop) has no test in this crate that can fail on a wake bug.
What correct looks like: one threaded test — producer parks on a full queue; consumer drains in another thread with a timeout; assert both complete; repeat with close-during-park and flush-during-park.
Confidence: HIGH — Verification: read (whole file); cross-checked queue.rs:423-470 park loop exists and is untested here.
Note: a threaded test may live in queue.rs's own `#[cfg(test)]` (other agent's assignment) — flagged as open question, severity capped at S3 for this file.

### F-002 [S4] Item weights never exceed the caps — the single-item oversize admission is unexercised
Where: crates/packet-queue/tests/backpressure_property.rs:168-172 (caps 8/96/64) vs 194-195 (sizes ≤ 24, durations ≤ 8)
What is wrong: the generator's max item (24 B, 8 ns) is always admissible under the caps (96 B, 64 ns, 8 pkts), so the limits.rs empty-queue-admits-oversize invariant (my limits.rs F-001) is never driven by this suite; Q1's per-step check can never see the "cap + one item" state the production invariant permits. The oracle itself is written correctly for it (u128 sums), it just never fires on that shape.
Confidence: HIGH — Verification: read + arithmetic.
Note: `zero_alloc`'s Blob (4 B / 1000 ns vs 1 MiB / 1 ms caps) doesn't hit it either.

### F-003 [S4] Dead condition in the burst arm
Where: crates/packet-queue/tests/backpressure_property.rs:191-193
Code (verbatim):
```rust
            match if in_burst { 0 } else { rng.below(100) } {
                // ---- try-push (fresh/future/stale stamps) ----
                r if r < 40 || in_burst && r < 100 => {
```
What is wrong: `in_burst` implies the scrutinee is `0`, so `r < 40` already matches; `|| in_burst && r < 100` is dead. Cosmetic; costs nothing but misleads a reader into thinking bursts can route to other arms.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Blob` + `Weigh` impl | 23-38 | REAL | — | fixed-size test item |
| `XorShift::next`/`below` | 42-57 | REAL | — | `% n` modulo bias — irrelevant here |
| `engine_thresholds` | 64-76 | REAL | — | mirrors playback-engine QueuePlan thresholds (cross-file duplication by design, documented); low binds packets-only |
| `check_q1` | 78-98 | REAL | — | all three axes asserted with context strings |
| `check_conservation` | 78-114 | REAL | — | exact identity assert |
| `check_hysteresis` | 119-139 | REAL | — | one-directional (low ⇒ packets; high ⇒ some axis); converse covered by test 2 |
| `seeds` | 142-152 | REAL | — | 8 fixed + env-gated `LUMEN_PROP_SEEDS` (seed|1 — even seeds coerced odd, silent) |
| `backpressure_vocabulary_holds_q1_conservation_and_hysteresis` | 162-318 | REAL | S3 | oracle-differential, per-step invariants; no real parking (F-001) |
| `engine_shaped_low_is_reachable_for_every_mix` | 325-422 | REAL | — | wedge mixes + flood/drain cycles |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| backpressure_vocabulary_holds_q1_conservation_and_hysteresis | YES for cap/refusal/counter/watermark bugs (reasoned); NO for park/wake bugs (F-001); NO for oversize admission (F-002) | oracle + per-step invariants are exact; the untested surfaces enumerated |
| engine_shaped_low_is_reachable_for_every_mix | YES (reasoned) | exact low-flag asserts on both wedge mixes; high-flag assert at resume line; 50 seeded cycles with invariants |

Would a mutation of `binding_axis`'s axis order survive? No — the oracle asserts the exact axis. Would a lost condvar wake survive? Yes — that is F-001.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc 13 §6 row 3.B3 AC: "property tests with synthetic producers/consumers — no deadlock, bounded memory under burst" | test doc 1-5 | SHALLOW | bounded memory under burst: proven (Q1 + burst mode); synthetic producers/consumers: single-threaded model only; no deadlock: not tested (F-001) |

## 7. Dead / duplicated / unreachable code

`engine_thresholds` duplicates playback-engine's `QueuePlan::thresholds` (documented mirror, engine-agnostic rationale). Dead condition F-003.

## 8. Dependencies & flags

packet-queue + media-types + testkit. No features. `LUMEN_PROP_SEEDS` env widens the sweep — a gate that runs only the 8 fixed seeds gets exactly that coverage.

## 9. Open questions for the next agent

- Does queue.rs's in-file test module contain a threaded park/wake test? If not, the crate's concurrency core is entirely untested for wake semantics (files: crates/packet-queue/src/queue.rs, stamped.rs).
