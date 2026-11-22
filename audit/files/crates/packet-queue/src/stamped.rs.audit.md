# AUDIT: crates/packet-queue/src/stamped.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 184 / 7 production items (4 methods, 2 trait impls, 2 traits) + 6 test fns |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `Stamped::from_payload` at playback-engine/src/pipeline.rs:2201 (`queue.try_push(Stamped::from_payload(packet))`), `Weigh` impls consumed by queue.rs try_push/push_wait (queue.rs:386-387, 433-434); the whole path is behind the shipped CLI open (see queue.rs audit header). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | rg for `Stamped|Generational|Weigh` across crates/ and apps/; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

Two small seams and one wrapper type: `Stamped<T>` (generation-tagged value), `Generational` (payloads that know their generation — implemented for `Packet`), and `Weigh` (byte/duration weight — implemented for `Packet`). All of it is on the shipped demux→decode path, all of it is pure and total (the queue's lock-safety argument in queue.rs depends on that purity, and it holds here), and the six tests are exact, boundary-aware, and honest. The single finding is a documented-but-risky policy choice: unknown-duration packets weigh zero on the duration axis, so the duration cap cannot bound a stream of unknown-duration packets.

## 2. What actually works (verified)

- `Stamped::from_payload` copies the payload's own generation (41-49) — the "two can never drift" claim (14-16) is structurally true for `Packet` (the impl at 74-78 reads the same field).
- `is_stale` is strictly-past (60-62), matching `Packet::is_stale` semantics and the queue's one-sided boundary check (queue.rs:394 — cross-referenced there as F-003).
- `Weigh for Packet`: `size_bytes` = payload bytes only (99-103), `media_duration` = `Known(n)` else 0 (105-110). Pure, total, no panics — the queue's "user code never runs under the lock" contract (83-85) is upheld by these bodies.
- Test 174-183 covers BOTH unknown-duration arms (`Duration::Unknown` and `duration: None`) — the two ways the 0-weight path triggers.

## 3. Findings

### F-001 [S4] Unknown-duration packets are weightless on the duration axis — the duration cap cannot bound them
Where: crates/packet-queue/src/stamped.rs:105-110
Code:
```rust
    fn media_duration(&self) -> Nanoseconds {
        match self.duration {
            Some(media_types::time::Duration::Known(n)) => n,
            _ => Nanoseconds(0),
        }
    }
```
What is wrong: A demuxer that emits packets without durations (several raw/stream formats do) produces items that count 0 toward the `Duration` limit axis. The queue's duration cap then never binds for such streams; bounding falls to the packets and bytes axes alone.
Why it matters: The doc itself tags this `[PROPOSAL]` (92-94) and justifies it ("must not wedge it") — a queue of unknown durations could otherwise deadlock the duration cap at zero fill. The residual risk is mitigated only if the packets/bytes caps are always configured; if a wiring sets a generous byte cap and relies on the duration cap for latency, unknown-duration streams get unbounded latency. Hygiene-level because the fallback axes exist and the choice is documented.
What correct looks like: An estimated-duration fallback (e.g., from the track's nominal frame rate) stamped at demux time, or a doc note in the queue's limit configuration that duration caps are advisory for unknown-duration formats.
Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Stamped::new` | 32-34 | REAL | — | Trivial constructor. |
| `Stamped::from_payload` | 41-49 | REAL | — | Reachable (pipeline.rs:2201). |
| `Stamped::into_value` | 53-55 | REAL | — | Trivial. |
| `Stamped::is_stale` | 60-62 | REAL | — | Strictly-past; boundary tested. |
| `Generational::generation` (impl for Packet) | 74-78 | REAL | — | Field copy. |
| `Weigh::size_bytes` (impl for Packet) | 99-103 | REAL | — | Payload bytes; conservative for shared buffers. |
| `Weigh::media_duration` (impl for Packet) | 105-110 | REAL | S4 | F-001. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| from_payload_stamps_from_the_payload_generation (134-139) | YES | Stamp == payload field. |
| stamped_is_stale_is_strictly_past (142-149) | YES | Equal/not-stale, next/stale, future/not-stale — the exact boundary triple. |
| stamped_into_value_discards_the_stamp (152-155) | YES | Value identity. |
| packet_weigh_reports_payload_bytes (158-161) | YES | Exact byte count. |
| packet_weigh_maps_known_duration (164-171) | YES | Exact ns passthrough. |
| packet_weigh_maps_unknown_duration_to_zero (174-183) | YES | Both zero-arm variants. |

No mocks-of-subject (the subject IS data plumbing over real `Packet` values), no ignores.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the two can never drift" (stamp vs payload generation, 14-16) | type doc | REAL | from_payload reads the same field the impl exposes. |
| "Both methods must be pure and total" (83-84) | trait doc | REAL | Verified bodies: field reads/matches only. |
| "unknown-duration items count as 0 … they must not wedge it" (92-94) | trait doc | REAL | The code does exactly this; the residual risk is F-001. |
| "The doc's field name `gen` is spelled `generation` … `gen` is a reserved keyword in edition 2024" (18-20) | type doc | REAL | Edition 2024 in Cargo.toml; `gen` is indeed a reserved keyword there. |

## 7. Dead / duplicated / unreachable code

None. `Generational` has one impl (`Packet`) and one consumer (`from_payload`); the doc's "frame types will follow" (68) is future work, disclosed.

## 8. Dependencies & flags

No flags, no unsafe, media-types only. testkit dev-only.

## 9. Open questions for the next agent

- W03 colleague with limits.rs: does `QueueLimits` ever get built with a duration cap and NO packets/bytes cap? If yes, F-001 becomes a real unboundedness for unknown-duration streams.
- W04/W05 (demuxers): which formats emit `duration: None`/Unknown packets in practice? That determines F-001's real-world exposure.
