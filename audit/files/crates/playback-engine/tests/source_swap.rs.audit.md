# AUDIT: crates/playback-engine/tests/source_swap.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 1278 / 4 tests + Harness (SwapDemuxer, LatencyDecoder, AttributionSink, journal) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — W5R-03 acceptance suite for card 3.A2 source replacement (`Engine::open` on a live session — my session.rs audit's R2-R9 flow). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `head -70`, inventory `rg`, `sed -n '739,800p'`, `sed -n '1192,1260p'` (exit 0). No cargo; run UNVERIFIED (W00: 1751/2/25). |

## 1. One-paragraph verdict

The relaunch re-proof of source replacement on the hardest shape the card names, and it delivers what the header promises: 50 switches where every window drives a seek (half completing the seven-step flow, half cut off mid-flight **with a non-empty pre-roll hold** — engineered by starving the audio limb so the seek is genuinely in flight when the next `open` lands), with **latency-holding decoders** (one undelivered frame always resident) so stale decoder output is structurally in flight at every generation boundary. Per switch it proves: the epoch bump (fresh session id; the old handle answers typed `SessionClosed`), zero stale frames at the sink (inline marker check + per-poll generation check), the seek flush draining exactly the latency-held frame per limb (counted stale, never delivered), supersession arithmetic, and the dying epoch's suppressed events. Plus the zero-poll storm (50 opens, no polls — exactly one surviving event, last source still plays), a teardown-ordering journal (token stale → decoders drop → demuxer drops → new demuxer starts; no flush solicited), and a real counting-allocator memory soak with band/creep/churn bounds pinned from measurement (warm-up 0..=4 excluded, window 5..=50). Two notes: the allocator discipline serializes every test in the binary (documented), and the whole suite runs on fakes (marker packets, latency decoders) — the real-stack composition lives in track_switch_fixture.rs instead.

## 2. What actually works (verified)

- The in-flight-seek engineering (783-800): starving the audio limb after the seek submit so the pre-roll hold is non-empty at the cut-off — the scenario actually constructs the race it claims to test, rather than hoping a race occurs.
- The dead-handle assert (756-763): the replaced session's handle must answer the typed staleness rejection — the command-side half of the epoch rule, pinned per switch.
- The memory soak (1192-1260+): snapshots pre-sized outside the window (bookkeeping not a growth term — a subtle correctness point handled), band/creep asserts with the pinned constants named in messages, plus frames≥51 (the soak really played), zero stale hits, and `frames_delivered == sink.frames` (engine stats vs sink truth cross-check).
- The teardown journal (1013-1191): observable order via drop hooks, asserting teardown never solicits decoder output (no flush calls) — the drop-by-construction claim of my session.rs audit, pinned.
- Serialization discipline: every test holds `serialize_tests()` for its whole body (68-70 documented rationale).

## 3. Findings

### F-001 [S4] The suite proves replacement on fakes; the real-stack variant is only the track-switch fixture
Marker `SwapDemuxer`/`LatencyDecoder` fakes are the right tool for 50 adversarial switches, but the only real-stack replacement coverage is incidental (track_switch_fixture switches tracks, not sources). A real-container `open`-while-playing regression (e.g., demuxer Drop blocking on a real file) would not be caught here. Recorded as a coverage note, disclosed implicitly by the file's own "relation to the merged suites" section.

### F-002 [S4] Pinned memory constants are measurement-derived, not derived from invariants
`BOUND_BAND`/`BOUND_CREEP`/churn constants "carry the measured values beside them" (header 30-33) — the honest form of a soak bound, but a bound that tracks measurement moves with the implementation rather than policing a budget. A slow leak just under the band's growth per switch would pass. Inherent to the soak genre; recorded.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `SharedNow`/`note`/journal | 112-140 | REAL | virtual time + teardown journal |
| `SwapDemuxer` (+Drop) | 142-212 | REAL | marker packets; drop journaled |
| `LatencyDecoder` (+Drop) | 214-268 | REAL | one-frame latency, the reorder shape |
| `SwapProvider`/`DeliveryRecord`/`AttributionSink` | 270-396 | REAL | per-source marker attribution at the sink |
| builders (tracks/packets/frames/queues/budget) | 398-503 | REAL | |
| `DrainFacts`/`Harness` (517-737) | 505-737 | REAL | open/pump/drain/check machinery — per-poll generation law inline |
| `fifty_rapid_switches_with_in_flight_seeks` | 738-931 | REAL | THE AC; per-switch arithmetic |
| `fifty_switches_with_zero_polls_between_opens` | 932-1011 | REAL | storm; one surviving event; last source plays |
| `teardown_orders_token_decoders_demuxer_and_drains_by_drop` | 1012-1191 | REAL | journal ordering; no-flush proof |
| `memory_stays_bounded_across_the_seek_interleaved_switches` | 1192-1278 | REAL | band/creep/churn + cross-checks |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fifty_rapid_switches_with_in_flight_seeks | YES (reasoned) | per-switch epoch/handle/stale/flush/supersession asserts; a leaked epoch, a delivered stale frame, or a completed-but-superseded terminal fails with the switch index |
| fifty_switches_with_zero_polls… | YES (reasoned) | exact surviving-event count |
| teardown_orders… | YES (reasoned) | journal order + absence of flush calls |
| memory_stays_bounded… | YES (reasoned) for band/creep/churn breaches and stats-vs-sink drift; NO for sub-band leaks (F-002) |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "stale decoder output is structurally in flight at every switch" | header 2-5 | REAL | LatencyDecoder holds one frame by construction |
| "teardown never *solicits* decoder output (no `flush` calls)" | header 27-29 | REAL-UNPROVEN | journal asserts it; run UNVERIFIED |
| "bounded memory … pinned from measurement (D-6)" | header 30-33 | REAL-UNPROVEN | F-002 qualifier |

## 7-9. Dead code / Dependencies / Open questions

All-serialize allocator discipline (documented). Open: the "merged suites" `replacement.rs`/`replacement_alloc.rs`/`epoch_staleness_alloc.rs` referenced at 8-11 and 68-70 — other agents' files; the claimed base-flow coverage should be verified there.
