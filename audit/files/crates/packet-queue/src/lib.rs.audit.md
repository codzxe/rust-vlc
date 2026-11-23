# AUDIT: crates/packet-queue/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 150 / 0 code functions (module docs + re-exports + `PacketQueue` alias + 1 doctest) |
| File verdict | REAL (the crate's claims verified against queue.rs/limits.rs and 48 green tests) — with one production-dead API |
| Reachable from shipped entry point? | YES for the crate: playback-engine's pipeline pushes through `PacketQueue` (crates/playback-engine/src/pipeline.rs:2201 `queue.try_push(Stamped::from_payload(packet))`); NO for `push_wait` (zero non-test callers) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
The lib.rs facade's four documented laws (boundedness, backpressure,
generations, watermarks) are not aspirational — I verified each against the
implementation: `try_push` weighs outside the lock, checks the generation
boundary with a typed stale signal, and hands the item back untouched on every
non-pushed outcome (queue.rs:379-420); `push_wait`'s `deadline: None` is
documented as explicit unbounded-park consent; the doctest exercises
Full-with-item-back, deadline parking, and watermark usage and passes in the
baseline, alongside 48 crate tests. The one hostile finding: `push_wait` — the
entire parking-producer half of the Q3 story — has zero production callers;
the shipped engine is a synchronous single-writer that only ever calls
`try_push`, and the one crate in the tree that wants a parking queue
(transcode) implemented its own instead of reusing this one.

## 2. What actually works (verified)
- Baseline (cargo-test-workspace.txt:3829-3941): packet-queue — 17 unit +
  13 backpressure + 2 backpressure_property + 7 concurrency + 9 generation
  tests, all `ok`, 0 failed; doctest `crates/packet-queue/src/lib.rs - (line 71) ... ok` (:3936).
- Law-by-law spot verification in queue.rs: Weigh outside the critical
  section (:384-388, matching lib.rs:41-43); stale generation dropped at the
  boundary and counted (:393-398, `stale_rejected`); typed
  `PushOutcome::Full { item, binding }` with the item back (:414-418);
  `deadline: None`-is-consent documented on push_wait (:421-424).
- Doc-metric mapping resolved: lib.rs:28's "doc 03 §5 metric `stale_dropped`"
  maps to `QueueStats::stale_rejected + flushed` per queue.rs:146-147 — the
  indirection is documented in-crate, not a naming lie.
- Production use: pipeline.rs:2201 (`try_push`) and the whole engine-side
  queue-invariant property suite (my queue_invariants.rs audit) exercise this
  crate's laws through the real engine.

## 3. Findings
F-001 [S3]
Where: crates/packet-queue/src/lib.rs:15-22 (and queue.rs:420-445)
Code:
```
//! - **Backpressure (Q3).** Backpressure is the *only* flow control.
//!   Producers never spin and never drop fresh data: a non-blocking
//!   [`try_push`](BoundedQueue::try_push) returns a typed
//!   [`PushOutcome::Full`] signal (with the item back, untouched), and
```
What is wrong: the parking half of Q3 — `push_wait` + `PushAborted` — is
production-dead. `rg push_wait` over all crates/*/src and apps/*/src: the only
callers are packet-queue itself (its doctest and tests); the shipped engine
(session/pipeline) exclusively uses `try_push` (single-writer synchronous
model — state.rs's teardown effect notes "there are no worker threads to join
yet"). Meanwhile crates/transcode/src/{queue,runner}.rs implement their own
parking queue (PushWait enum) rather than reusing `BoundedQueue::push_wait` —
the primitive exists, is tested (13 backpressure tests), and is bypassed by
the one in-tree consumer that needs it. Why it matters: the doc's threading
story ("demux thread and decode workers produce ... parks on a condvar")
describes an architecture the shipped product does not run; a reader auditing
flow control is directed at an API nothing reaches. What correct looks like:
either the engine's Phase-3 session-thread card wires push_wait, or transcode
adopts it, or the lib.rs threading paragraph is labeled v0-future.
Confidence: HIGH — Verification: rg over the tree; read of queue.rs and
pipeline.rs:2201.

## 4. Per-function table
Not applicable — no functions in this file. Inventory: module docs (1-135),
module declarations + re-exports (137-145), `pub type PacketQueue =
BoundedQueue<media_types::Packet>` (147-150), one doctest (71-125). All
re-exported names exist (the release lane compiles clean — baseline
cargo-build-release-lumen-cli.txt exit 0).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 71) | YES | asserts the Full-with-item-back contract and a deadline-parked push completing after a drain — a queue that dropped or spun would fail |
| (crate suite, another agent's files) | YES per baseline | 48 tests green incl. 7 concurrency (MPMC) and 9 generation (stale/flush) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "An unbounded queue is unconstructible: caps are NonZeroUsize / known-positive Duration by typed constructor" | lib.rs:9-14 | REAL | QueueLimits::new typed ctor; 17 unit tests |
| "Producers never spin and never drop fresh data" | lib.rs:15-16 | REAL (try_push path, in production) | queue.rs:379-420; engine property suite |
| "push_wait sleeps on a condvar until space, close, or deadline — never unboundedly by surprise" | lib.rs:19-21 | REAL but UNREACHABLE in product | queue.rs:420-445; zero production callers (F-001) |
| "A flush drops every resident item wholesale ... a later push of an older-generation item is dropped at the boundary ... counted" | lib.rs:23-28 | REAL | queue.rs:393-398, tests/generation.rs 9 green |
| "demux thread and decode workers produce, audio pump and video scheduler consume" (MPMC) | lib.rs:38-40 | REAL-UNPROVEN in product | concurrency tests prove the primitive MPMC-safe; the shipped engine is single-threaded per session |
| "The session thread only ever uses the non-parking operations" | lib.rs:22 | REAL | pipeline.rs:2201 try_push only |

## 7. Dead / duplicated / unreachable code
`push_wait` / `PushAborted` / the space-condvar: reachable only from tests
(F-001). The transcode crate's parallel parking-queue implementation is the
duplication flag (transcode is another agent's file).

## 8. Dependencies & flags
media-types only (layer L2 per doc 01 §2.2); std sync primitives; no unsafe
seen in the audited paths (queue.rs audited only where cited; full audit
belongs to the packet-queue files' owner).

## 9. Open questions for the next agent
- queue.rs/limits.rs/stamped.rs (not my files): the `unpoison` recovery
  pattern — is any panic actually possible inside the critical sections
  (the docs claim infallibility)?
- transcode/src/queue.rs: why a fork? If BoundedQueue::push_wait is
  unsuitable for transcode's shape, the lib.rs "reused for every stage
  boundary" claim (:4-5) is weaker than stated.
