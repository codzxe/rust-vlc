# AUDIT: crates/playback-engine/tests/replacement_alloc.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 502 / 24 (1 test, 23 fakes/builders/harness items) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary). Exercises the shipped `Engine::open` replacement path (play-new-while-playing, card 3.A2) on the real engine. Ran the prebuilt binary directly: `./target/debug/deps/replacement_alloc-27049c6265f430ec` -> 1 passed, 0 failed (exit 0). Baseline: cargo-test-workspace.txt:4379-4382 (ok). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | direct prebuilt-binary run -> `test result: ok. 1 passed; 0 failed` (exit 0); `rg` verifications of the cross-referenced queue prealloc claim |

## 1. One-paragraph verdict
A real bounded-memory proof, not a ritual: 51 opens / 50 switches over the real engine with a counting global allocator, a non-retaining sink (correct attribution discipline), a pre-sized snapshot vector and a reused drain buffer so the bookkeeping itself is not a growth term, three pinned-from-measurement ceilings (band/creep/churn) with the measured values and margins recorded beside each constant, and an anti-tautology guard (`band > 0`) so a soak that silently stopped running cannot pass. I found nothing wrong. This is the strongest test file audited so far in this wave.

## 2. What actually works (verified)
— verified by running the prebuilt test binary directly (exit 0, 1 passed, 0.01s): the 50-switch soak runs green under the counting allocator.
— verified cross-reference: the doc's "BoundedQueue::new preallocates min(max_packets, 1024) slots" is true (packet-queue/src/queue.rs:243: `items: VecDeque::with_capacity(limits.max_packets.get().min(1024))`).
— verified liveness/attribution machinery exists: `frames >= 51` floor + `last_marker == 50` witness (lines 454-460) — the final epoch's frames are provably the last delivered.

## 3. Findings
None. What was checked and how (per spec §12):
- Arithmetic: window = snapshots[5..=50] is 46 entries (asserted, line 464); 0..=50 is 51 opens = 50 switches (comment accurate); creep is isize so freed-memory (negative creep) passes — correct direction for a leak detector; churn deltas are over a monotonic u64 counter (no underflow); saturating/`must()` only on Option results that cannot be None after the max/min over a non-empty 46-slice.
- Attribution: the sink retains nothing (count + last marker); the returned session handle is dropped immediately with the retention hazard documented (lines 384-388); bookkeeping pre-sized (`Vec::with_capacity(51)`, line 437) so no snapshot-push allocates inside the measured window.
- Bounds honesty: all three constants carry their measured values and margins in the doc comments (band measured 10,666 pinned 32 KiB; creep measured 24 B pinned 1 KiB; churn measured 474 pinned 2048) with the re-measure-not-weaken discipline stated (lines 36-41).
- Fake honesty: the marker family stamps source identity into packet payloads and frame payloads (marker → first byte), so the last-marker assert proves the replacement really replaced (epoch 50's data, not a stale epoch's).
- Liveness: 80-poll bound to Playing with a loud assert (413-425); `band > 0` guards the vacuous pass (501).
- Reachability of the measured path: every switch calls the real `Engine::open` (line 409) — the same call the CLI's replacement flow uses; no mock of the engine.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow / impl MonotonicSource | 98-104 | REAL | — | Frozen virtual clock (0 forever) — see §7 note on ticks. |
| TallyConsumer / frame_marker | 109-133 | REAL | — | Non-retaining sink; marker extraction for audio/video/None for subtitle handles. |
| MarkerDemuxer (start/next_event/seek/seekability/reanchor) | 137-181 | REAL | — | On-demand synthesis, round-robin tracks, marker payload; reanchor resets counters honestly. |
| MarkerAudioDecoder / MarkerVideoDecoder | 183-237 | REAL | — | One frame per packet, generation-stale honored, marker propagated into the frame payload. |
| MarkerProvider | 239-256 | REAL | — | Kind-dispatched decoder seam. |
| base/at_millis/track/source_tracks | 262-298 | REAL | — | The track-shape ladder (video-only / audio-only / A+V) driving the band's dominant term. |
| marker_packet/audio_frame/video_frame | 300-338 | REAL | — | Fresh 16/8-byte payloads (real allocations on the measured path — churn honestly includes them). |
| switch_queues / switch_budget | 340-355 | REAL | — | 64-packet queues, 32/4/8 budget. |
| Harness::new / open_source | 357-411 | REAL | — | Real Engine::open per switch; handle dropped (documented hazard). |
| pump_to_playing | 413-425 | REAL | — | Bounded 80 polls, loud assert. |
| memory_stays_bounded_across_fifty_switches | 431-502 | REAL | — | The AC: liveness + attribution + band/creep/churn + anti-tautology. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| memory_stays_bounded_across_fifty_switches | YES | A per-epoch retention leak (even one queue slot ≈ tens of bytes/switch) accumulates 45× inside the window and breaks the 1 KiB creep ceiling; an unbounded churn regression breaks 2048; a broken replacement that never delivers the new epoch's frames fails `last_marker == 50`; a soak that stops running fails `band > 0` or the Playing assert. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no epoch reaches end-of-stream inside its window, so every switch departs Playing" | lines 70-72 | REAL | PACKETS_PER_TRACK=512 vs the few polls per switch; final state asserted Playing (453). |
| "BoundedQueue::new preallocates min(max_packets, 1024) slots" | lines 26-27 | REAL | queue.rs:243 verbatim match. |
| "The three constants are pinned from measurement … roughly the design's ~2× margin" | lines 36-39 | REAL | Margins recorded per constant (3×/42×/4×) — conservative and honest. |
| "a retaining collector would attribute the test's own growth to the engine" | lines 4-6 | REAL | TallyConsumer retains two atomics only. |

## 7. Dead / duplicated / unreachable code
- None. Every fake/builder is used by the single test.
- Measurement-fidelity note (not a defect): the throttle source is frozen at 0 for the whole soak, so `timing.time-tick` events (the only continuous event stream) never allocate inside the measured window; per-switch events (state-changed/tracks/buffering/info-ready, ~10/switch) ARE counted. The churn ceiling therefore slightly under-represents a real wall-clock session's tick allocations — bounded impact (ticks are plain-data payloads, replaced not queued).

## 8. Dependencies & flags
- testkit (alloc_count::{CountingGlobalAlloc, AllocSnapshot, serialize_tests}, Must), codec_api, media_clock, media_types, packet_queue, plugin_api, playback_engine — all shipped crates; no feature gates; whole-binary single-test discipline (process-global allocator).

## 9. Open questions for the next agent
- None for this file. Environmental: the sandbox disk hit 100% during my wave (baseline already recorded a disk-poisoned cargo attempt); later agents should prefer prebuilt test binaries under target/debug/deps over fresh cargo invocations.
