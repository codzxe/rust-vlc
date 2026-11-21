# AUDIT: crates/packet-queue/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 150 / 9 rows (crate root: docs, doctest, module decls, re-exports, one type alias) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → playback-engine (`apps/cli` dep) → `pipeline.rs:105-107` `use packet_queue::{BoundedQueue, …}` → `Limb.queue: Arc<BoundedQueue<Packet>>` (pipeline.rs:552) constructed in `wire()` — every demux→decode boundary of every shipped session runs through this crate). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `ls src/ tests/` (module map vs reality: limits/stamped/queue all exist; 7 test files); `rg "Doc-tests packet_queue"` in nofailfast (doctest line 71 GREEN :6448–6453); `rg "BoundedQueue<(Frame|VideoFrame)"` over crates/ (0 hits — the frame-queue reuse claim is unrealized); `rg "packet_queue"` in pipeline.rs (imports at :105-107, `Limb.queue` :552, try_push :2875, try_pop :2352/:2843 — the "session thread only uses non-parking ops" claim verified); packet-queue baseline results collated (lib 17, backpressure 13, backpressure_property 2, concurrency 7, generation 9, limits 3, property 4, zero_alloc_queue 1 — all green, :3534–3645); `sed -n` on quoted ranges. |

## 1. One-paragraph verdict
The crate root documents a genuinely load-bearing primitive — and the primitive is real, wired into every shipped session's demux→decode boundary, and tested to an unusually high standard (56 tests across 8 binaries green in the no-fail-fast baseline, including property, concurrency, and zero-alloc suites; the doctest executes green). The four-enforcement summary (boundedness, backpressure, generations, watermarks) matches what I verified from the consumer side in my pipeline.rs#part5 audit (thresholds degenerating low to the packets axis, try_push/try_pop only in the session thread, wholesale flush at mints). Two stale doc claims: the crate is NOT "reused for every stage boundary … decode→engine frame queues" (no frame queue exists anywhere in the tree — pipeline.rs:39–43 records the stage as deliberately absent), and "CancelToken … does not exist yet" is false at HEAD (playback-engine's epoch module ships it and even uses it as the staleness oracle).

## 2. What actually works (verified)
— The queue primitive in shipped use: `BoundedQueue<Packet>` limbs in the engine (pipeline.rs:552), non-parking ops only in the session thread (try_push/try_pop — verified at pipeline.rs:2352/2843/2875), watermark as the single Buffering input (pipeline.rs:2056 `limb.queue.watermark()`).
— All 56 packet-queue tests green in nofailfast: lib 17 (:3536), backpressure 13 (:3557+), backpressure_property 2 (:3576+), concurrency 7 (:3584+), generation 9 (:3597+), limits 3, property 4 (:3621+), zero_alloc_queue 1 (:3631+).
— Doctest (full push/full-refusal/pop/park/watermark cycle) executed green (:6451).
— Module map completeness: limits/stamped/queue all exist and are re-exported (L137–145); `PacketQueue` alias pins the packet flavor (L150).

## 3. Findings
### F-001 [S4]
Where: crates/packet-queue/src/lib.rs:4-6
Code:
```
//! One generic, heavily tested primitive reused for every stage
//! boundary (doc 01 "crates/packet-queue"): demux→decode packet
//! queues, decode→engine frame queues. What the crate enforces, per
```
What is wrong: The second named reuse does not exist. `rg "BoundedQueue<(Frame|…|VideoFrame)"` over crates/ and apps/ returns zero hits — no decode→engine frame queue is constructed anywhere. The consumer side documents the absence as deliberate: pipeline.rs:39–43 — "The decode→engine **frame queue** stage is deliberately absent (packet-queue's `Weigh` is not yet implemented for frames — 2.B1's own recorded limitation); decoded frames flow directly through the sink seam."
Why it matters: The crate-root doc claims a reuse story ("every stage boundary") that the tree contradicts; a boundary reviewer reading only this file would believe frame queues flow through this crate's caps.
What correct looks like: "reused for the demux→decode packet boundary; the decode→engine frame stage is deliberately absent in v0 (see playback-engine's pipeline docs)".
Confidence: HIGH — Verification: rg 0 hits + pipeline.rs:39–43 read (my part5 audit context).
### F-002 [S4]
Where: crates/packet-queue/src/lib.rs:49-55
Code:
```
//! - `push(&self, item, gen_check: &CancelToken)` → the parking
//!   [`push_wait`](BoundedQueue::push_wait) plus non-blocking
//!   [`try_push`](BoundedQueue::try_push). `CancelToken` is minted by
//!   the playback engine (L4, Phase 3) and does not exist yet; per
```
What is wrong: "`CancelToken` … does not exist yet" is false at HEAD. `playback_engine::epoch::CancelToken` exists (lib.rs:112 re-export), is public, and is used as the independently-written staleness oracle in `events.rs`'s `the_stamp_rule_agrees_with_the_cancel_token_oracle` (baseline green :3780). The queue still doesn't take a token in push — the deviation note's conclusion stands; only its factual aside about the type's existence is stale.
Why it matters: Doc drift in the crate that everything else cites as the layer-2 contract; a future implementer reading this would re-create a type that already exists.
What correct looks like: "…the playback engine's `CancelToken` exists (epoch.rs) but the queue's own published generation is the boundary check, and a parked producer wakes on its movement exactly as a token would."
Confidence: HIGH — Verification: playback-engine lib.rs:112 read; events.rs:1901–1922 oracle test read (my part3 audit); baseline :3780 green.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (Q1–Q4 enforcement summary) | 1–34 | REAL | — | Each claim verified from the consumer side (pipeline.rs audit) + green suites. |
| module docs (Threading) | 36–43 | REAL | — | Mutex+2 condvars (queue.rs, tested by concurrency suite 7/7); "session thread only non-parking" verified at 3 call sites. |
| module docs (Deviations ledger) | 45–63 | REAL | S4 | F-002: stale CancelToken existence claim. |
| doctest | 65–125 | REAL | — | Full cycle incl. Full-hand-back and park-with-deadline; GREEN in baseline :6451. |
| module docs (Module map) | 127–135 | REAL | — | Complete — all three modules exist and are re-exported (contrast with video-engine's map, my F-001 there). |
| `pub mod` declarations | 137–139 | REAL | — | limits, queue, stamped. |
| re-exports | 141–145 | REAL | — | Complete public surface. |
| `PacketQueue` type alias | 147–150 | REAL | — | Documented flavor pin; the engine's limbs are this type. |
| (claim) "reused for every stage boundary … frame queues" | 4–6 | FALSE (as written) | S4 | F-001. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 71) | YES | Typed outcome equality incl. the handed-back item; park-with-deadline; watermark usage assert. Green :6451. |
| (indirect) 56 tests across 8 binaries | YES | Subject is the queue itself (no mocks of the subject); property + concurrency + zero-alloc suites are the anti-toy coverage. All green :3534–3645. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "reused for every stage boundary: demux→decode packet queues, decode→engine frame queues" | lib.rs L4–6 | FALSE (frame half) | F-001: rg 0 frame instantiations; pipeline.rs:39–43 records the absence. |
| "Backpressure is the only flow control … never drop fresh data" | lib.rs L15–18 | REAL | PushOutcome::Full hands the item back (doctest); engine parks on it (pipeline.rs demux_step pending). |
| "The session thread only ever uses the non-parking operations" | lib.rs L22 | REAL | pipeline.rs:2352/2843/2875 — try_* only. |
| "CancelToken … does not exist yet" | lib.rs L52–55 | FALSE (stale) | F-002: epoch::CancelToken exists and is exercised. |
| "Consumers observe the wholesale drop as a typed discontinuity" | lib.rs L28–30 | REAL | PopOutcome::Discontinuity — pipeline.rs tests assert the wire-mint marker (my part5 audit, wired() helper L2842–2845). |

## 7. Dead / duplicated / unreachable code
None in this file. (The frame-queue reuse is absent code, not dead code — F-001.)

## 8. Dependencies & flags
media-types only (registry row quoted in header) + testkit dev-only. No cfg gates, no unsafe at this level. All 8 test binaries green in nofailfast (:3534–3645); crate compiled clean in baseline build #1.

## 9. Open questions for the next agent
- To the packet-queue module auditors (queue.rs / limits.rs / stamped.rs owners): the threading claim "Weigh is evaluated once per push outside the critical section" (L41–43) is this root's strongest concurrency promise — verify the lock scope in `try_push` directly.
- To the engine wave: if/when frame queues arrive, the packets-axis-degenerate low watermark (my pipeline.rs#part5 audit, thresholds L400–415) will need a frame-flavor policy decision — the root doc's "every stage boundary" framing will then become true only with new policy.
