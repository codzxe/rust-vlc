# AUDIT: crates/packet-queue/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 150 / 0 functions — crate root: docs (1-135), 3 mod decls, 3 re-export groups, 1 doctest, 1 type alias |
| File verdict | REAL (glue) with one FALSE threading doc |
| Reachable from shipped entry point? | YES — the crate it roots is on the shipped path (playback-engine Cargo.toml:39 → pipeline.rs:1114/2201/2352; behind CLI open). This file is docs + re-exports. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | rg `QueueLimits::new|audio_default|video_default` in limits.rs (claim check), rg `PacketQueue` across production code; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

Crate-root documentation and re-exports for the bounded queue primitive. The invariants documentation (Q1-Q4) is accurate where it describes the code — I verified the "unbounded queue is unconstructible" claim against `QueueLimits::new` (limits.rs:149-165: `NonZeroUsize` × 2 + `Known(n > 0)` duration, typed errors otherwise) — and the doctest genuinely exercises the Full-signal and parking paths. Two documentation/hygiene defects: the threading section describes an MPMC multi-threaded pipeline ("demux thread and decode workers produce, audio pump and video scheduler consume") that exists nowhere in any shipped product, and the doc example imports the dev-only `testkit` crate, so it cannot compile for a downstream user. The `PacketQueue` alias is spelled out of existence by its only natural consumer.

## 2. What actually works (verified)

- **Q1 claim**: "An unbounded queue is unconstructible: caps are `NonZeroUsize` / known-positive Duration by typed constructor" (9-14) — verified at limits.rs:149-165: `new` rejects `Duration::Unknown` (`QueueConfigError::UnknownDuration`) and zero durations, and the packet/byte caps are `NonZeroUsize` at the type level. (Field-level bypass and `audio_default`/`video_default` literals are limits.rs's audit — W03 colleague.)
- **Re-exports resolve**: limits (LimitAxis/QueueConfigError/QueueLimits/QueueUsage/Watermark/WatermarkThresholds), queue (BoundedQueue/Discontinuity/PopOutcome/PushAborted/PushOutcome/QueueStats), stamped (Generational/Stamped/Weigh) — all present in their modules (verified while auditing queue.rs/stamped.rs).
- **Doctest logic** (71-125): 2-packet cap, third push → `Full` with item equality asserted, pop, then `push_wait` completing — walked by hand, consistent with queue.rs semantics.
- Line 22's "The session thread only ever uses the non-parking operations" is TRUE and is the honest sentence that contradicts F-001's threading story.

## 3. Findings

### F-001 [S3] Threading docs describe a multi-threaded MPMC pipeline that no shipped product has
Where: crates/packet-queue/src/lib.rs:36-43
Code:
```rust
//! `Mutex<VecDeque>` + two condvars (space, items) — MPMC-capable:
//! demux thread and decode workers produce, audio pump and video
//! scheduler consume. The lock is held only for deque pushes/pops —
//! never across I/O, decode, or user code: [`Weigh`] is evaluated
//! once per push *outside* the critical section, and critical
//! sections contain only infallible operations.
```
What is wrong: There is no demux thread, no decode workers, and no audio pump thread in any shipped binary. The v0 engine is a synchronous poll loop (playback-engine/src/pipeline.rs:2225-2228: "the synchronous v0 engine never observes them here"), and the parking API this paragraph is written around (`push_wait`/`pop_wait`) has zero production callers (queue.rs audit F-002). The paragraph presents the threading model as the crate's operating reality.
Why it matters: A reader (or claim-ledger agent) concludes a threaded MPMC pipeline exists. It does not. The lock-discipline sentence is true; the actors are fiction. The honest sentence exists at line 22 but is buried under this one.
What correct looks like: Present tense for what ships ("the session thread polls"), future/conditional for the Phase-3 threading plan.
Confidence: HIGH — Verification: read (rg call graph in queue.rs audit).
### F-002 [S4] Doc example imports a dev-only crate
Where: crates/packet-queue/src/lib.rs:80
Code:
```rust
//! use testkit::Must;
```
What is wrong: `testkit` is a `[dev-dependencies]` entry (packet-queue/Cargo.toml). Doctests of the same crate compile with dev-deps available, so this passes CI — but the example as API documentation cannot be copied by a downstream user of `packet-queue`, since a normal dependency edge does not bring `testkit`.
Why it matters: Doc examples are the crate's usage contract; this one is unbuildable outside the workspace. Hygiene.
What correct looks like: `NonZeroUsize::new(2).ok_or("zero")?` (as video-engine's lib.rs doctest does at line 137) instead of `.must()`.
Confidence: HIGH — Verification: read (Cargo.toml + doctest).
### F-003 [S4] `PacketQueue` alias has no production user
Where: crates/packet-queue/src/lib.rs:147-150
Code:
```rust
/// The demux→decode packet queue flavor of [`BoundedQueue`] —
/// media-types [`media_types::Packet`] payloads, audio/video caps from
/// [`QueueLimits::audio_default`] / [`QueueLimits::video_default`].
pub type PacketQueue = BoundedQueue<media_types::Packet>;
```
What is wrong: The one natural consumer spells the generic directly: playback-engine/src/pipeline.rs:552 `pub queue: Arc<BoundedQueue<Packet>>`. rg over production code finds no `PacketQueue` usage outside this file and lib.rs doc examples.
Why it matters: Dead public API surface maintained as if it were the crate's primary name.
What correct looks like: Use the alias at pipeline.rs:552 or drop it.
Confidence: HIGH — Verification: read (rg).

## 4. Per-function table

Not applicable — no functions. Declaration inventory:

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| Crate docs (Q1-Q4, threading, deviations) | 1-64 | REAL except threading § | F-001. |
| Doctest | 71-125 | REAL-UNPROVEN | Logic hand-verified; execution UNVERIFIED (no cargo); testkit dep F-002. |
| Module map docs | 127-135 | REAL | Accurate. |
| `pub mod limits/queue/stamped` | 137-139 | REAL | All real modules (limits ~340 LOC, queue 549, stamped 184). |
| Re-exports | 141-145 | REAL | All resolve. |
| `pub type PacketQueue` | 150 | UNREACHABLE | F-003. |

## 5. Test honesty

The doctest asserts real behavior (Full-with-item-back equality, push_wait completion after drain, watermark usage count). It would fail if try_push mutated the item on refusal or push_wait ignored space. Not a tautology.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "An unbounded queue is unconstructible" (9-14) | crate doc | REAL | QueueLimits::new validation verified (limits.rs:149-165). |
| "Producers never spin and never drop fresh data" (15-16) | crate doc | REAL | try_push hands the item back (queue.rs:407-410); pipeline.rs:2223 holds it; the parking path uses wait_while, not sleep loops. |
| "The session thread only ever uses the non-parking operations" (22) | crate doc | REAL | pipeline.rs:2201/2352 are the only queue calls. |
| "MPMC-capable: demux thread and decode workers produce, audio pump and video scheduler consume" (38-40) | crate doc | FALSE (as shipped reality) | No such threads exist; parking API uncalled (F-001). |
| "A flush drops every resident item wholesale … never as silent data loss" (25-30) | crate doc | REAL | queue.rs:336-374 + PopOutcome::Discontinuity; consumer-side merge verified. |
| "`CancelToken` is minted by the playback engine (L4, Phase 3) and does not exist yet" (51-52) | crate doc | SHALLOW | A `CancelToken` DOES exist now in playback-engine (epoch.rs:65, exported lib.rs:112) — for frame-step cancellation, not queue pushes. The doc's "does not exist yet" is stale; the queue indeed does not use it. |

## 7. Dead / duplicated / unreachable code

`PacketQueue` alias (F-003). `PushAborted` re-export is production-dead with its parent API (queue.rs F-002).

## 8. Dependencies & flags

No flags, no unsafe; deps media-types only (+ testkit dev). Consumers: playback-engine, plugin-host-wasm.

## 9. Open questions for the next agent

- W03 colleague with limits.rs: are `QueueLimits` fields `pub` (bypassing `new`'s validation), and do `audio_default`/`video_default` duplicate `new`'s invariants by hand?
- W12: does plugin-host-wasm use `PacketQueue` (the alias) or park on `push_wait`? (Starvation finding queue.rs F-001 becomes live if so.)
