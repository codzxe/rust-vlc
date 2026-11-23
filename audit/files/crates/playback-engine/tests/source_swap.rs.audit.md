# AUDIT: crates/playback-engine/tests/source_swap.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 1278 / 39 (4 tests + 35 harness/fake/builder fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only file driving the REAL engine: `Engine::open/poll/drain_outputs_into/cancel_token` (crates/playback-engine/src/session.rs) + the REAL seek command path (`SessionHandle::seek` -> `SeekTarget::Absolute`), same engine the shipped CliHost pumps (apps/cli/src/host.rs:448). Fakes: SwapDemuxer (plugin_api::Demuxer), LatencyDecoder (codec_api::Decoder), AttributionSink (FrameConsumer), frozen virtual clock. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p playback-engine --test source_swap` -> 4 passed / 0 failed (exit 0, my run); baseline cross-check audit/runs/baseline/cargo-test-workspace.txt:4540; sed/rg quote verifications. |

## 1. One-paragraph verdict
This is the strongest test file audited so far in this domain: four heavyweight tests drive the real Engine through 51-epoch source-replacement storms with the real seek command path interleaved (26 completed seeks, 25 cut off mid-flight), one-packet-latency decoders guaranteeing stale decoder output is structurally in flight at every boundary, an attribution sink that checks marker AND pipeline generation inline at sink-accept time (so a leaked stale frame cannot hide between polls), exact suppressed-event arithmetic (3 vs 2 per departure shape), exact stale-flush counts (limbs per seek), a journal proving teardown ordering (decoders drop before demuxer; no flush solicited at teardown), and a counting-allocator soak with pinned band/creep/churn bounds plus a `band > 0` shape assert so a dead soak cannot pass vacuously. All 4 pass in my own run. The only defects are hygiene: the sink counts marker-less frames (subtitle / GPU-handle video) as stale — a latent miscount currently unreachable in this suite; a dead `let _ = poll_frames;` assignment; and measurement numbers living only in comments (the asserted bounds are looser pins, which is the honest direction).

## 2. What actually works (verified)
— `cargo test -p playback-engine --test source_swap` -> `test result: ok. 4 passed; 0 failed; 0 ignored` (exit 0, finished in 0.02s). Mutation reasoning: leak one stale frame and `poll_and_check`'s marker assert (642-649) or the generation assert (650-657) fires; retain one slot per epoch and the creep bound (1254-1261) fires; solicit a flush at teardown and the journal assert (1095-1102) fires; suppress one event too many/few and the 3-vs-2 arithmetic (826-842) fires. `serialize_tests`/`AllocSnapshot`/`CountingGlobalAlloc` exist (crates/testkit/src/alloc_count.rs:64,136,211) — the global allocator and the lock are real, not decorative.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/source_swap.rs:383-392, 347-354
Code:
```
fn frame_marker(frame: &Frame) -> Option<u8> {
    match frame {
        Frame::Audio(audio) => audio.samples.as_slice().first().copied(),
        Frame::Video(video) => match &video.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice().first().copied(),
            FrameStorage::OpaqueHandle(_) => None,
        },
        Frame::Subtitle(_) => None,
    }
}
```
and
```
        let marker = frame_marker(&frame);
        let expected = self.expected_marker.load(Ordering::Relaxed);
        if marker != Some(expected) {
```
What is wrong: a frame with NO marker (subtitle frames, GPU/opaque-handle video frames) compares `None != Some(expected)` and is counted as a STALE hit. The stale counter is this suite's headline AC ("zero stale frames") — a legitimately-delivered marker-less frame would be misreported as stale and fail the suite. Currently unreachable (the track ladder at 429-437 only ever builds Audio/Video CPU-frame limbs), so this is latent harness fragility, not a live failure. What correct looks like: skip the marker check (or assert separately) when `marker.is_none()`. Confidence: HIGH — Verification: read (no subtitle/opaque path exists in this suite; rg `Subtitle` -> only the two match arms).
Confidence: HIGH — Verification: read

F-002 [S4]
Where: crates/playback-engine/tests/source_swap.rs:659-662
Code:
```
            records.len()
        });
        let _ = poll_frames;
        facts
    }
```
What is wrong: `poll_frames` (the per-poll record count) is computed by the closure, bound, then explicitly discarded — a dead assignment with no comment explaining intent. Either assert something about it (e.g., bounded by RECORD_CAPACITY, matching the D-1 claim in the comment at 98-103) or don't compute it. Why it matters: the RECORD_CAPACITY pre-sizing claim ("a poll that somehow exceeded it would show up as churn") is never directly checked anywhere; the natural place was here and it was left as `let _`. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/playback-engine/tests/source_swap.rs:1153-1171
Code:
```
/// The measured live-bytes band ceiling (steady-state oscillation).
/// Measured on this suite's fakes: 7,982 bytes (max 38,169 /
/// min 30,187 — the two-queue vs one-queue footprint swing of the
```
What is wrong: the specific measured numbers (7,982 band; 407 alloc calls/switch; creep ≤ 0) exist only in comments — no test asserts them, so they are unverifiable claims (per spec: never trust comments). The CODE asserts only the looser pins (32 KiB / 1 KiB / 2048), which is the safe direction (a regression beyond measurement still trips), so this is documentation debt, not dishonest pinning. What correct looks like: record the measured numbers as debug asserts or print them in a soak manifest. Confidence: HIGH — Verification: read; the pins themselves verified by my passing run.
Confidence: HIGH — Verification: read / ran

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow::now_ns | 112-118 | REAL | — | frozen virtual monotonic source |
| note | 125-132 | REAL | — | journal append, poison-recovering |
| SwapDemuxer::start | 152-155 | REAL | — | journals `demux:start` |
| SwapDemuxer::next_event | 157-176 | REAL | — | round-robin; starved audio limb yields Discontinuity but advances cursor (documented) |
| SwapDemuxer::seek | 178-186 | REAL | — | v0 refusal hook, correct error stage |
| SwapDemuxer::seekability | 188-190 | REAL | — | Fast{indexed:true} — lets real seek flow engage |
| SwapDemuxer::reanchor | 192-197 | REAL | — | rewinds cursor, reports landing = target |
| SwapDemuxer::Drop | 200-204 | REAL | — | journals `demux:drop` (teardown proof) |
| LatencyDecoder::configure | 223-227 | REAL | — | journals; takes generation |
| LatencyDecoder::decode | 229-249 | REAL | — | one-packet latency: emits previous, holds current; stale guard first |
| LatencyDecoder::flush | 251-261 | REAL | — | emits held frame under last-configured generation (the stale-drain shape) |
| LatencyDecoder::Drop | 264-268 | REAL | — | journals `decoder:drop` |
| SwapProvider::decoder_for | 276-289 | REAL | — | journals `decoder:built` |
| AttributionSink::new | 320-329 | REAL | — | pre-sized records (D-1) |
| AttributionSink::inspect_and_clear | 334-342 | REAL | — | clear-in-place reuse |
| AttributionSink::accept | 346-380 | REAL | S4 | inline stale check miscounts marker-less frames (F-001) |
| frame_marker | 383-392 | REAL | S4 | None for subtitle/opaque (F-001) |
| base | 398-400 | REAL | — | 1 µs tick |
| at_millis | 402-404 | REAL | — | ms→µs conversion, no overflow at suite scale |
| track | 406-421 | REAL | — | marker-carrying fake track |
| source_tracks | 429-437 | REAL | — | the limb ladder (V / A / A+V) |
| source_kinds | 439-441 | REAL | — | derived |
| marker_packet | 443-454 | REAL | — | 16-byte marker payload |
| latency_frame | 456-480 | REAL | — | audio/video marker frames; size varies by marker |
| switch_queues | 482-489 | REAL | — | 64-packet caps |
| switch_budget | 491-497 | REAL | — | 32 demux events, 4 packets/limb |
| Harness::new | 531-549 | REAL | — | retained seams across epochs (R4) |
| Harness::open_source | 557-593 | REAL | — | resets starve flag; flips inline expectations at epoch boundary |
| Harness::drain_and_check_events | 600-623 | REAL | — | asserts every drained event belongs to live session |
| Harness::poll_and_check | 636-663 | REAL | S4 | marker + generation asserts; dead `let _ = poll_frames` (F-002) |
| Harness::pump_to_playing | 667-679 | REAL | — | bounded, asserts at limit |
| Harness::pump_until_seeked_and_playing | 685-703 | REAL | — | requires Seeked terminal AND return to Playing |
| Harness::journal_since | 707-713 | REAL | — | observation window |
| Harness::journal_len | 715-720 | REAL | — | — |
| fifty_rapid_switches_with_in_flight_seeks | 739-921 | REAL | — | THE AC: 50 switches, 26 completed + 25 superseded seeks, exact suppression/flush/preroll arithmetic, 0 stale hits |
| fifty_switches_with_zero_polls_between_opens | 933-993 | REAL | — | zero-poll storm: 1 surviving event, 100 suppressed, last source still plays |
| teardown_orders_token_decoders_demuxer_and_drains_by_drop | 1013-1147 | REAL | — | journal proves decoders→demuxer→new-start ordering, no flush at teardown, token staleness |
| memory_stays_bounded_across_the_seek_interleaved_switches | 1193-1278 | REAL | S4 | band/creep/churn bounds + band>0 shape assert; measured numbers comment-only (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fifty_rapid_switches_with_in_flight_seeks | YES | per-switch deltas (suppressed 3/2, stale-flush == limbs, preroll >0/==0), per-poll marker+generation asserts, stale_hits==0 at every window and at end |
| fifty_switches_with_zero_polls_between_opens | YES | exact counts (1 event, 100 suppressed, 0 frames), then proves last session playable |
| teardown_orders_token_decoders_demuxer_and_drains_by_drop | YES | journal index-ordering asserts + no-flush assert + stats-zero asserts |
| memory_stays_bounded_across_the_seek_interleaved_switches | YES | creep/band/churn bounds; `band > 0` kills the vacuous-flat-soak escape |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "50 switches where every window also drives a seek ... half complete, half cut off mid-flight" | source_swap.rs:14-18 | REAL | test asserts completed_seeks==26, superseded_seeks==25 (903-910); my run ok |
| "decoders hold one undelivered frame at all times" | source_swap.rs:19-21 | REAL | LatencyDecoder::decode holds previous packet (239-243); flush emits it (253-259) |
| "teardown never solicits decoder output (no flush calls)" | source_swap.rs:40-42 | REAL | journal assert 1095-1102 |
| "bounded memory ... live-bytes band / creep and per-switch churn pinned from measurement" | source_swap.rs:43-47 | REAL (bounds) / UNVERIFIED (measured numbers 7,982 / 407 — comment-only, F-003) | my run passes the pinned bounds |
| "exactly one event survives the final drain" | source_swap.rs:35-36 | REAL | assert 968 |
| baseline 4 passed | audit/runs/baseline/cargo-test-workspace.txt:4540 | REAL | my run: 4 passed, exit 0 |

## 7. Dead / duplicated / unreachable code
- `let _ = poll_frames;` (661) — dead assignment (F-002).
- Latent unreachable branch: marker-less frames in AttributionSink (F-001) — subtitle/OpaqueHandle paths never produced by this suite's ladder.
- No #[ignore], no todo!/unimplemented! (rg verified).

## 8. Dependencies & flags
Workspace-only: codec_api, commands, media_clock, media_types, packet_queue, playback_engine, plugin_api, testkit. Process-global `#[global_allocator]` CountingGlobalAlloc with `serialize_tests()` mutex held for every test body — correct discipline for a multi-test binary sharing a global counter. No feature gates, no sleeps, no I/O.

## 9. Open questions for the next agent
- Does the real (non-fake) demux layer honor `reanchor` the way SwapDemuxer does (rewind + report landing), or can it land elsewhere and trip the same pre-roll arithmetic? Check demux host + seek.rs step 4.
- `RECORD_CAPACITY` (256) is claimed as a churn guard but never asserted (F-002) — the memory test's churn bound is the only indirect check.
