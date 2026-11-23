# AUDIT: crates/playback-engine/tests/bounded_alloc.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 581 / 24 (1 #[test] + 23 helpers/fakes/builders) |
| File verdict | REAL (runtime allocation measurement with a process-global counting allocator; exact cap arithmetic) |
| Reachable from shipped entry point? | N/A (test file) — measures the shipped `Engine::poll`/OutputHub path (session.rs, previous audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (zero-findings disclosure in §3) |
| Commands run | `cargo test -p playback-engine --test bounded_alloc` → `1 passed; 0 failed` in 0.04s |

## 1. One-paragraph verdict
This is the D-1 boundedness proof the session.rs docs promise, done with runtime evidence instead of syntactic green: a `#[global_allocator] CountingGlobalAlloc` (testkit), one test per binary by design (module doc explains the process-global allocator serialization), and two storm phases over the REAL engine — a 10k receipt-only storm (session in `Ended`, every command a typed rejection) asserting **zero allocations** post-warmup with exact drop arithmetic (`10_000 − RECEIPT_CAPACITY`), and a 10k confirmation storm (session in `Paused`, receipt + `state-changed` each) asserting **flat live bytes** with both caps' arithmetic (`10_000 − RECEIPT_CAPACITY` receipts, `10_000 − OUTBOX_CAPACITY` events), strict delivery order, and receipt→event causal adjacency for every surviving pair. The fakes (FakeDemuxer/FakeDecoder/FakeProvider/Collector) fake the DEMUX/DECODE stages — the thing under test (the command/output path) is the real `Engine`. I executed the test: passes in 0.04s. No defects found.

## 2. What actually works (verified)
- `cargo test -p playback-engine --test bounded_alloc` → `test the_session_output_path_is_bounded_under_10k_storms ... ok` (executed).
- The storm design closes its own loopholes: `busy > 0` asserts the inbox cap was actually saturated (typed refusal exercised, not assumed); `delta.alloc_calls > 0` sanity-asserts the storm ran (transient vecs allocated and freed); the warmup phase pre-grows the hub ring so the measured window is the steady state, and the oldest-survivor id `100_000 + 10_000 − RECEIPT_CAPACITY` is the exact arithmetic, not a fuzzy bound.
- Cross-checked the design's hidden assumption: the Paused session must NOT end mid-storm — verified in pipeline.rs:1924–1932 ("Paused fills to the watermark then parks", decode frozen in Paused except seek/switch carve-outs), so 22k+ commands across ~350 polls keep the 40-packet media parked and every pause an accepted C8 self-transition. The test's final all-Accepted assertion is therefore load-bearing, not vacuous.

## 3. Findings
None. What was checked and how (zero-findings disclosure per spec §12):
- Fakeness: every fake is a demux/decode STAGE stand-in, disclosed in-module ("Fakes (trimmed from `session_flow.rs`…)"); the unit under test (Engine/OutputHub/inbox) is production code — verified against the session.rs audit (same `RECEIPT_CAPACITY`/`OUTBOX_CAPACITY` constants imported at lines 46–47).
- Arithmetic: warmup id ranges (0..12_000) and measured ranges (100_000..110_000) recomputed by hand — oldest survivor 105,904 = 110,000 − 4,096 consistent with RECEIPT_CAPACITY = 4096 (session.rs:100) and OUTBOX_CAPACITY = 1024 (session.rs:78).
- Measurement soundness: guard window excludes construction/drain; `sink.clear()` reuses capacity; `serialize_tests()` + one-test-per-binary prevents cross-thread counting noise; `delta.live_bytes == 0` with `alloc_calls > 0` is the correct formulation for transient allocs.
- Assert strength: every cap claim is an exact equality; order is strict monotonicity; adjacency is per-pair with origin correlation (`event.origin == Some(receipt.request)`).
- Hang safety: `poll_until` bounded with loud context; `storm_pauses` retries only on the typed InboxBusy.
- No `#[ignore]`, no feature gates, no sleep anywhere.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SharedNow::now_ns` | 63–67 | REAL | — | frozen-at-0 monotonic source (both clock + throttle) |
| `Collector::accept` | 74–83 | REAL | — | poison-tolerant banking sink (bounded 8/40-packet media) |
| `FakeDemuxer::start/next_event/seek` | 93–112 | REAL | — | scripted stage fake; seek refuses typed |
| `FakeDecoder::configure/decode/flush` | 119–143 | REAL | — | one frame per packet; stale → NeedMoreInput |
| `FakeProvider::decoder_for` | 147–157 | REAL | — | working decoder per track |
| `base/at_micros/audio_track/packet/audio_frame` | 164–223 | REAL | — | deterministic builders; static bytes (no per-packet alloc noise) |
| `tiny_queues/tiny_budget/frozen_throttle` | 225–245 | REAL | — | small caps; frozen throttle |
| `open_audio` | 251–283 | REAL | — | real `Engine::open` over the fakes; script + EOS |
| `poll_until` | 289–304 | REAL | — | bounded, loud failure |
| `storm_pauses` | 311–333 | REAL | — | poll-only-on-InboxBusy; counts busy refusals; final flush poll |
| `receipts_of` | 335–343 | REAL | — | |
| `the_session_output_path_is_bounded_under_10k_storms` | 350–581 | REAL | — | the two-phase proof; executed pass; assertions itemized in §2 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `the_session_output_path_is_bounded_under_10k_storms` | YES | mutation reasoning: a hub that grows per push fails `assert_zero`/`live_bytes == 0`; a miscounted drop fails the exact `10_000 − CAP` deltas; a lost/unordered receipt fails the survivor-id and monotonicity asserts; a detached event fails the origin-correlation loop (`paired == OUTBOX_CAPACITY`); an inbox that blocks/drops fails `storm_pauses`'s unreachable arm |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runtime evidence, not syntactic green (D-6)" | bounded_alloc.rs:5 | REAL | counting-allocator guards with zero/flat-byte asserts (executed) |
| "10k pushes allocate **nothing**" (receipt storm) | bounded_alloc.rs:12–14 | REAL | `guard.assert_zero(...)` passed (executed) |
| "memory stays **flat across the whole storm**" | bounded_alloc.rs:19–22 | REAL | `delta.live_bytes == 0` with `alloc_calls > 0` sanity (executed) |
| "The binary runs identically under `--test-threads=1` and the default parallel runner" | bounded_alloc.rs:10–11 | REAL-UNPROVEN | structurally sound (1 test + serialize_tests); I did not run both modes |

## 7. Dead / duplicated / unreachable code
- None. The fakes are trimmed copies of session_flow.rs fakes (disclosed duplication with rationale — storm-only needs).

## 8. Dependencies & flags
- Test-only; dev-deps codec-api, media-types, media-clock, packet-queue, plugin-api, testkit, commands. `#[global_allocator]` swap is test-binary-local. No features, no ignores.

## 9. Open questions for the next agent
- testkit's `CountingGlobalAlloc`/`alloc_count` correctness is load-bearing for this proof — whoever audits crates/testkit should confirm live_bytes/alloc_calls accounting (my file trusts it).
- The non-measuring halves (inbox typed overflow, pinned constants) live in session_flow.rs — not my assignment.
