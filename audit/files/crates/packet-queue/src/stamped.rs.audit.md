# AUDIT: crates/packet-queue/src/stamped.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 184 / 10 (Stamped 4 methods, Generational trait + Packet impl, Weigh trait + Packet impl, 6 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen open` → `Pipeline::push_packet` (pipeline.rs:2201 `queue.try_push(Stamped::from_payload(packet))`) → `BoundedQueue::try_push` (queue.rs:383-387 calls `item.value.size_bytes()` / `media_duration()`); the stamp comparison is the Q2 boundary check at queue.rs:394. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/packet_queue-d758e65f89ba0951 stamped` → 6 passed, 0 failed, finished in 0.00s, exit 0. |

## 1. One-paragraph verdict
Small, correct, and load-bearing: `Stamped<T>` is the generation stamp the queue boundary trusts (producer stamps at push; `from_payload` copies the payload's own generation so the two cannot drift), `is_stale` is strictly-past-only matching `Packet::is_stale` semantics, and `Weigh for Packet` is the pure, total weight seam the queue calls outside its lock. The unknown-duration→0 mapping is the documented PROPOSAL that the engine's packets-axis starvation policy explicitly compensates for (pipeline.rs `QueuePlan::thresholds` degeneration) — the two halves of the design agree. All 6 unit tests executed and pass. One S4: the doc's claim that frame types will "follow the same shape when their crate lands" is stale — frames landed long ago (media_types has Frame) and still have no `Weigh`/`Generational` impl, which is why the engine has no decode→sink frame queue.

## 2. What actually works (verified)
- `./target/debug/deps/packet_queue-d758e65f89ba0951 stamped` → `test result: ok. 6 passed` — stamp-copy, strictly-past staleness (incl. the future-stamp-is-not-stale pin), into_value, payload-bytes weight, known/unknown/missing duration mappings.
- The producer-stamp flow verified end-to-end in my pipeline.rs audit: `packet.generation = self.generation` at push (pipeline.rs:2147) → `Stamped::from_payload` → queue refuses `stamp < published` (queue.rs:394) — the doc 03 §5 checking-point-3 chain is real.
- The unknown-duration→0 wedge case is compensated upstream: pipeline.rs:406-413 makes packets the only low-axis authority precisely because `Weigh` maps unknown to 0 (test `unknown_duration_mix_reaches_low` in pipeline.rs, executed).

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/src/stamped.rs:65-68
Code:
```
/// Payloads that know their own generation.
///
/// Implemented for [`Packet`] (its `generation` field, doc 01 §3.3);
/// frame types will follow the same shape when their crate lands.
```
What is wrong: the claim is stale. `media_types` has shipped `Frame` (Video/Audio) for many waves (the engine's `LimbSink` consumes them, pipeline.rs:2675-2723), yet no `Generational`/`Weigh` impl for frames exists in this crate — rg over packet-queue and media-types shows only `impl … for Packet`. Consequently the engine's decode→sink stage has NO bounded frame queue (pipeline.rs:39-43 documents this as "deliberately absent … packet-queue's `Weigh` is not yet implemented for frames"), and decoded frames flow straight through the sink seam — the bounded-queue discipline (Q1–Q4) stops at the decode boundary. That is a real architectural consequence (frame backpressure is the consumer's `ready_for` probe instead of a queue), documented at the engine side but presented here as a "when their crate lands" future that has already arrived. What correct looks like: implement `Generational`/`Weigh` for `Frame` (frames carry `generation` and pts/duration already) or update the doc to state the frames-are-unbounded-by-design decision. Confidence: HIGH — Verification: read + rg (`impl Generational` / `impl Weigh` → only Packet).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Stamped` (struct) | 21-27 | REAL | — | Field named `generation` (edition-2024 keyword rationale documented). |
| `Stamped::new` | 31-34 | REAL | — | Explicit stamp. |
| `Stamped::from_payload` | 40-49 | REAL | — | Copies the payload's own generation — no drift possible. Used in production (pipeline.rs:2201). |
| `Stamped::into_value` | 52-55 | REAL | — | Discard stamp. |
| `Stamped::is_stale` | 59-62 | REAL | — | Strictly past (`self.generation < current`); future stamps are not stale (pinned by test; the comment names it an ordering bug elsewhere). |
| `Generational` (trait + Packet impl) | 69-78 | REAL | S4 | F-001 stale "frames will follow" doc claim. |
| `Weigh` (trait) | 86-96 | REAL | — | Pure/total contract; called outside the lock (queue.rs:386-387 — verified). |
| `Weigh for Packet::size_bytes` | 98-103 | REAL | — | Payload bytes only; metadata-exclusion rationale in comment. |
| `Weigh for Packet::media_duration` | 105-111 | REAL | — | Known→n, Unknown/None→0 (documented PROPOSAL; engine compensates — verified interlock). |
| 6 unit tests | 113-184 | REAL | — | All executed; exact-value asserts. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| from_payload_stamps_from_the_payload_generation | YES | Asserts both the stamp and the payload's view agree. |
| stamped_is_stale_is_strictly_past | YES | Pins `<` (not `<=`) and the future-stamp non-staleness — a `<=` regression fails. |
| stamped_into_value_discards_the_stamp | YES | Trivial but exact. |
| packet_weigh_reports_payload_bytes | YES | 1500-byte payload → 1500. |
| packet_weigh_maps_known_duration / …unknown…to_zero | YES | Known/Unknown/None (mutated) all pinned exactly. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the producer stamps the current token generation onto the item at push time; the queue then treats the stamp as the single authority" | stamped.rs:11-14 | REAL | pipeline.rs:2147 + queue.rs:392-400 (traced and covered by the queue's own stale tests, which ran under my earlier `generation` binary run: `stale_check_precedes_the_capacity_check` etc.). |
| "For payloads that already carry their generation — Packet does — from_payload copies it over, so the two can never drift" | 14-16 | REAL | from_payload:45-48; test pins it. |
| "unknown-duration items count as 0 toward the duration limit — they must not wedge it" | 91-94 | REAL | media_duration impl + test; the non-wedge policy lives in the engine's packets-axis thresholds (verified in my pipeline.rs audit, test executed). |
| "frame types will follow the same shape when their crate lands" | 67-68 | FALSE (stale) | Frames shipped; no impl exists (F-001). |

## 7. Dead / duplicated / unreachable code
- None dead. `Stamped::new` has production callers in tests and queue internals (`PushOutcome` round-trips); `into_value` used by pipeline.rs:2223 (`item.value`).

## 8. Dependencies & flags
- `media_types` only (GenerationId, Packet, Nanoseconds, Duration). No features, no unsafe, pure functions — matches the "queue never runs user code under lock" contract.

## 9. Open questions for the next agent
- Whether a `Weigh for Frame` should land (F-001): the engine currently bounds frames only via `ready_for`/hold caps; a threaded Phase 3 will need a real frame queue or the sink probe must remain the only backpressure.
- queue.rs/limits.rs (same crate) are other agents' files; my read of them for context found the boundary logic consistent with this seam.
