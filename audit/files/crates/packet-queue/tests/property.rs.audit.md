# AUDIT: crates/packet-queue/tests/property.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 981 / 4 `#[test]` + harness (XorShift, seed bank, shrink) + model oracle (`run`, `check_gap`) + threaded workers |
| File verdict | REAL (a genuinely strong model-oracle property suite) with one dead drain assertion |
| Reachable from shipped entry point? | N/A (test). The subject — `packet_queue::BoundedQueue` — is live production code (pipeline.rs queue stage). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | Read (full file, 2 chunks); Grep/Read of subject: `Discontinuity` (queue.rs:114-134, 336-366, 490-530 — flush ALWAYS sets `pending_gap` even for 0 drops; `try_pop` surfaces the gap first); `#[test]` count = 4, no `#[ignore]`/cfg/env gates on the tests themselves (`LUMEN_PROP_SEEDS`/`LUMEN_PROP_SHRINK` only widen/diagnose); sed verification of both quoted ranges. No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

The card 3.D3 property suite for the bounded generation-aware queue,
and it is the real thing: a hand-rolled seeded XorShift drives 6000-op
adversarial interleavings (try/park push, try/park pop, advancing AND
regressing flushes, close) through the real `BoundedQueue` against an
exact resident model checked after EVERY op — packets/bytes/duration
identity, the conservation law `pushed == popped + resident + flushed`,
non-regressing published generation, FIFO by id, stale-stamp rejection
with item-untouched checks, and even the `Full` binding-axis oracle
(first violated cap, computed in u128 like the crate). 64 seeds × 4
caps × 6000 ops plus a dense 48×N stale sweep plus a watchdogged
threaded no-deadlock storm — all ungated, so all ran in the W00
baseline's 1751. One defect: the stale sweep's drain loop is dead code
— the setup flush leaves a pending `Discontinuity` marker (the queue
sets one even for a 0-drop flush), so the very first `try_pop` in the
`while let PopOutcome::Item` loop returns `Discontinuity`, the pattern
fails to match, and the "no stale item is ever popped" assertion
inside NEVER executes. Relatedly, the queue's documented ordering law
("gap before any new item", queue.rs:327) is nowhere asserted — the
interleaving model happily accepts an Item while a gap is pending.

## 2. What actually works (verified)

— All four tests are ungated `#[test]`s → part of W00's 1751 passing (`cargo test --workspace`); env vars only widen (`LUMEN_PROP_SEEDS`) or diagnose (`LUMEN_PROP_SHRINK`).
— Exact accounting after every op (:541-616): `usage()` packets/bytes/duration must equal the model's sums; `stats()` (pushed/popped/flushed/stale_rejected/full_rejections) must equal the model's counters; conservation identity checked per op and terminally (:611-616, :664-669).
— The `Full` binding-axis oracle (:312-346): the rejection's `binding` must be the FIRST violated cap in u128 arithmetic, and the refused item must come back bit-identical — a mutation swapping the axis order or corrupting the item fails.
— Admission oracle includes the livelock invariant (:294-297, "the empty queue admits one item unconditionally" — 6.FIX-pcm) and both push paths check it (:298-311, :395-405).
— Regressing flushes are generated (`regress: rng.below(3) == 0`, :216) and the non-regression law asserted both on the queue (:585-591) and via the model (:519).
— Dense stale sweep (:735-817): for every `published` in 0..48 × every stamp in 0..(published+6): stale stamps never enter (`stats().pushed` unchanged), `Stale` reports the published generation, non-stale stamps take Pushed or the capacity path with the correct binding axis.
— Threaded storm (:874-962): 2 producers + 1 consumer with bounded deadlines, a main-thread storm of 12 seeded flushes under generation movement, then close; a 20 s `recv_timeout` watchdog turns a hung worker into a loud `unreachable!`; post-storm drain is spin-bounded (65536) and the drained queue must be empty. Producers stamp from a shared `published` that the storm moves — parked producers must wake with typed `Stale` aborts (the doc 03 §4.4(f) shape).
— Mutation reasoning (UNVERIFIED, no cargo): break FIFO (pop from the back) → :453-457 fails on id mismatch; let a stale stamp enter → :774-779 or the interleaving Pushed guard (:300-308) fails; break conservation (leak a resident) → :611 fails; unbounded acceptance past a cap → :564-583 fails; a lost wakeup → the watchdog fires.

## 3. Findings

### F-001 [S3] The stale sweep's drain assertion is dead code — the pending gap marker exits the `while let` before any item is ever popped

Where: crates/packet-queue/tests/property.rs:807-815
Code:
```rust
        // Drain: every popped item is at or after the published
        // generation — stale generations are unreachable by pop.
        while let PopOutcome::Item(item) = queue.try_pop() {
            assert!(
                item.generation.counter() >= published,
                "a stale item ({} < {published}) was popped",
                item.generation.counter()
            );
        }
```
What is wrong: the sweep's setup flush at :745 (`let _ = queue.flush(GenerationId::new(published));`) ALWAYS leaves `state.pending_gap = Some(...)` — the queue sets the consumer-side marker even when it dropped zero residents (queue.rs:336-366: `pending_gap` is unconditionally assigned; there is no zero-drop short-circuit). `try_pop` surfaces the pending gap first (queue.rs:497-499: `if let Some(gap) = state.pending_gap.take() { return PopOutcome::Discontinuity(gap); }`). So the FIRST `try_pop()` at :810 returns `Discontinuity`, the `while let PopOutcome::Item` pattern does not match, the loop body never executes, and the assertion inside — the test's own claim "after a flush, no pop ever yields a stale item" (doc comment :731-732) — never runs, for every one of the 48 `published` values.
Why it matters: a queue regression that pops stale items post-flush would pass this sweep; the only real protection left is the interleaving run's pop arm (:466-473), which checks staleness of whatever pops — that arm is real, but the sweep's drain was written specifically to pin this law and pins nothing. Worse, the same blind spot means the queue's documented ordering law — "Consumers observe the drop via `PopOutcome::Discontinuity` before any new item: never silent" (queue.rs:327) — is asserted NOWHERE: the interleaving model's `Item` arm (:449-476) never checks `model.pending_gap.is_none()`, so a queue that delivered post-flush items before the gap marker would pass the whole suite.
What correct looks like: drain with `loop { match queue.try_pop() { Item(item) => assert!(...), Discontinuity(_) => continue, Closed|Empty => break } }`; and in the interleaving model's `Item` arm, fail if `model.pending_gap.is_some()` (the gap must be surfaced before any new item).
Confidence: HIGH — Verification: read (property.rs:745→:807-815 + queue.rs flush/pop bodies quoted above; no pop between the flush and the drain).

### F-002 [S4] The seed bank + XorShift are copy-pasted from playback-engine's property suite although a shared testkit crate exists

Where: crates/packet-queue/tests/property.rs:56-93 (vs crates/playback-engine/tests/property/rng.rs)
Code:
```rust
// ---------------------------------------------------------------------------
// The seeded harness (duplicated from playback-engine's
// tests/property/rng.rs — integration tests are separate crates)
// ---------------------------------------------------------------------------

struct XorShift(u64);
```
What is wrong: the header's justification ("integration tests are separate crates") does not hold: `testkit` is a shared dev-dependency this very file imports (:54, `use testkit::Must;`) and it already hosts cross-crate test utilities (`testkit::alloc_count::serialize_tests()`, used by the soak suites). The RNG and the 64-seed bank could live there; instead the generator and bank are forked, and the two copies must now be kept in sync by hand (a seed-bank change in one silently diverges the other).
Why it matters: duplication between the two property suites is drift debt; §3.5.
What correct looks like: move `XorShift`/`seed_bank` into testkit (or a shared `testkit::prop` module) and import from both.
Confidence: HIGH — Verification: read + testkit usage in this file and in soak_scenarios/runner.rs's unit tests.

### F-003 [S4] The canonical "rebuild from the seed alone" path in `run` is unreachable from this file's tests

Where: crates/packet-queue/tests/property.rs:252-258
Code:
```rust
    let canonical = ops.is_empty();
    let mut rng = XorShift::new(seed);
    let ops = if canonical {
        generate_ops(&mut rng, 6000)
    } else {
        ops.to_vec()
    };
```
What is wrong: every caller passes a non-empty `ops` (the test pre-generates at :720-721; the shrink bisection calls `run` on prefixes `ops[..mid]` with `mid >= 1`, :104-119), so `canonical` is always false and the regeneration branch never fires. The header's documented shrink procedure (:38-41, "each rerun rebuilds the case from the seed alone") describes this dead path — the actual failure message's rerun recipe (`LUMEN_PROP_SEEDS={seed}`) also does not drive it: with the env set, the test still pre-generates ops from the same seed, which is equivalent, but through the non-canonical branch.
Why it matters: dead defensive branch + a documented procedure that does not match the live code path; misleading for whoever debugs a failure.
What correct looks like: either delete the canonical branch or make the test call `run(seed, cap, &[])` so the documented rebuild is the real path.
Confidence: HIGH — Verification: read (all call sites enumerated).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `XorShift` (new/next/below) | 61-80 | REAL | S4 | modulo bias in `below` — irrelevant for a test RNG; duplicated (F-002) |
| `seed_bank` | 83-93 | REAL | S4 | first 64 odds; env widening; duplicated (F-002) |
| `shrink_requested` | 95-100 | REAL | — | env-gated diagnosis |
| `shrink_prefix` | 104-119 | REAL | — | bisection; self-tested |
| `run_case` | 121-145 | REAL | — | rerun/shrink recipe in failure text |
| `Blob`/`impl Weigh` | 152-167 | REAL | — | id/size/duration payload |
| `Op`/`generate_ops` | 174-221 | REAL | — | 46% try-push (stale-biased stamps), 12% park-push, 26% try-pop, 8% park-pop, 4% flush (⅓ regressing), 2% close — the adversarial mix |
| `Model` (+bytes/duration) | 224-247 | REAL | — | exact resident oracle; pending-gap merge |
| `run` | 251-674 | REAL | S3/S4 | the law engine; F-003 canonical dead path; Item arm lacks the gap-ordering check (F-001) |
| `check_gap` | 676-710 | REAL | — | dropped/bytes/duration/generation identity |
| test `bounded_accounting_and_stale_laws...` | 716-727 | REAL | — | 64×4×6000; ran in W00 |
| test `stale_generation_packets_are_always_dropped` | 734-817 | REAL | S3 | push-phase asserts real; drain phase dead (F-001) |
| `producer` | 827-855 | REAL | — | bounded 3 ms deadlines; poison-tolerant |
| `consumer` | 858-867 | REAL | — | bounded 2 ms deadlines |
| test `threaded_storm_terminates...` | 874-962 | REAL | — | 8 seeds; watchdog; spin-bounded drain; close-then-close idempotence exercised |
| test `shrink_prefix_bisects...` | 969-981 | REAL | — | minimal prefix = 41; empty/non-failing edge cases |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| bounded_accounting_and_stale_laws... | YES | per-op exact accounting, conservation, axis oracle, FIFO, stale guards — any queue-law break fails with seed+op in the message |
| stale_generation_packets_are_always_dropped | YES for the push phase / NO for the drain phase | dense push-phase asserts are real; the drain assert never executes (F-001) |
| threaded_storm_terminates... | YES | watchdog + join + drain bound + emptiness; a lost wakeup or unbounded drain fails loudly |
| shrink_prefix_bisects... | YES | exact minimal-prefix length and value |

All four ran in the W00 baseline (ungated). Runtime note: the threaded storm uses real sleeps/deadlines (~tens of seconds for 8 seeds worst-case) — the baseline's merge-gate run absorbed it.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exact accounting … conservation identity after every op" | header :9-13 | REAL | :541-616 |
| "stale generations always dropped … a popped item is never stale" | header :16-19 | PARTLY | push-side pinned (:735-806); the sweep's pop-side assertion is dead code (F-001); the interleaving pop arm does check staleness (:466-473) |
| "no deadlock — a threaded storm … must terminate under a watchdog" | header :26-28 | REAL | :874-962 |
| "rerun with LUMEN_PROP_SHRINK=<seed> … reports the minimal failing op prefix" | header :37-41 | REAL-UNPROVEN | machinery present and self-tested (:969-981); no actual failure was produced to exercise it end-to-end (would require a broken queue) |

## 7. Dead / duplicated / unreachable code

- The drain loop body :809-815 — dead (F-001).
- The canonical regeneration branch :254-258 — unreachable from this file (F-003).
- XorShift/seed_bank duplicated from playback-engine (F-002).

## 8. Dependencies & flags

No feature flags; dev-deps `media_types`, `packet_queue`, `testkit` only (per crate manifest, W03-A05's audit). No proptest (header :32-33 — "not in Cargo.lock; C5" — consistent with the std-only purity policy). Env knobs are additive, never gating.

## 9. Open questions for the next agent

- F-001's ordering gap: does any OTHER suite assert gap-before-items (backpressure.rs / backpressure_property.rs — other agents' files)? If not, the queue's :327 contract is entirely untested repo-wide — worth a claim-ledger row.
- The zero-drop flush still minting a pending marker is queue-side behavior (queue.rs:366) that every consumer must handle; whether the pipeline's consumer does is a pipeline.rs question (already audited by W03-A01).
