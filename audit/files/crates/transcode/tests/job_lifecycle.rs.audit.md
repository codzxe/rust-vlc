# AUDIT: crates/transcode/tests/job_lifecycle.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 545 / 6 tests + 9 helpers/fixtures |
| File verdict | REAL — tests a REAL pipeline; whole crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (test-only file; the `transcode` crate under test has zero workspace dependents — see error.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/job_lifecycle-6124d913c7793298` -> 6 passed / 0 failed, exit 0 (binary 18:07 > source 17:35); sed verifications |

## 1. One-paragraph verdict
This is what an acceptance suite should look like: a scripted mpsc-backed demuxer drives the real two-thread pipeline (real `PcmDecoder`, real WAV sink, real `submit`) and every claim is asserted with byte-exact evidence — FIFO salt blocks re-read from the output, queue high-water marks against the configured axes, event-hub drop-and-count arithmetic (12 events, 2 kept, 10 dropped), a mid-flight cancel whose partial WAV is re-parsed by the crate's own demuxer with payload == counted bytes, and typed failure paths pinning the exact dotted codes from error.rs. All 6 tests pass on the prebuilt binary. The only gaps are coverage gaps, not fakeness: no test drives a panicking stage (the `worker_panic` seam), and the cancel test's lower bound depends on an `eventually` poll (bounded 5 s, 2 ms interval — documented, acceptable).

## 2. What actually works (verified)
— `./target/debug/deps/job_lifecycle-6124d913c7793298` -> all 6 pass, `0.01s`. Ran the full suite; output pasted above in command log.

## 3. Findings

F-001 [S4]
Where: crates/transcode/tests/job_lifecycle.rs:500-513
Code:
```
/// A decoder factory whose `open` refuses — the failing-stage seam
/// for the failure-lifecycle proof (a host registry with no decoder
/// for the track's codec descriptor).
struct RefusingFactory;
```
What is wrong: coverage gap, not a bug — error.rs's `worker_panic` (L344-351, "a worker thread ended without a terminal state — a panic in a caller-provided stage") and `flush_undrained` have no lifecycle test here; a panicking decoder factory would exercise the panic→error doctrine the docstring promises. The failure test only covers a clean `Err` refusal. Confidence: HIGH — Verification: read (rg over the crate's tests for worker_panic/panic-ing stages).

F-002 [S4]
Where: crates/transcode/tests/job_lifecycle.rs:263-272
Code:
```
fn eventually<F: FnMut() -> bool>(mut probe: F, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if probe() {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    unreachable!("timed out waiting for {what}");
}
```
What is wrong: a 5-second wall-clock deadline makes the cancel test's lower bound (`bytes_written >= PACKET`, L372-375) machine-load-sensitive on the 2-core sandbox — a heavily loaded runner could observe 5 packets written (still inside the asserted `<= 5*PACKET` bound, so it degrades safely, but the "promptness" in the test name is only bounded by 5 s of polling, not measured). Acceptable design, noted for the record. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Step / ScriptedDemuxer (Demuxer impl) | 53-88 | REAL | — | mpsc-backed; recv Err → Eos; seek → UNKNOWN |
| audio_track | 92-107 | REAL | — | pcm_s16le stereo, 1/1000 base |
| ScriptPcm (DecoderFactory) | 111-118 | REAL | — | real PcmDecoder::with_format(2, 48000) |
| SharedCursor (Write+Seek) | 122-167 | REAL | — | twin-buffer sink; poison-resistant locks; seek-beyond-end zero-fills on write |
| MemStream (SourceStream) | 171-227 | REAL | — | in-memory re-demux vehicle; typed seek error |
| mem_seek_error | 229-237 | REAL | — | typed Open-stage error |
| decode_graph | 240-259 | REAL | — | real TranscodeGraph, WavPcm sink |
| eventually | 263-272 | REAL | S4 | F-002 |
| file_of / PACKET consts | 275-282 | REAL | — | 16 KiB = 4096 frames @ s16 stereo |
| completed_lifecycle_runs_progresses_completes_and_stays_bounded | 285-360 | REAL | — | counts, high-water bounds, event order, ≥4 progress, dropped=0, byte-exact FIFO output |
| cancellation_is_prompt_terminal_and_leaves_a_truthful_prefix | 363-416 | REAL | — | mid-flight cancel, whole-packet prefix, header-patched data size, re-demux payload == counted |
| a_full_event_hub_drops_and_counts_but_never_blocks_the_pipeline | 419-451 | REAL | — | 2-slot hub: kept 2, dropped 10, pipeline completed |
| a_source_without_audio_fails_the_job_typed | 454-485 | REAL | — | pins "transcode.source.audio-track-missing" |
| a_filter_request_is_refused_at_submit_not_midjob | 488-498 | REAL | — | pins "transcode.graph.filter-unimplemented" at submit |
| RefusingFactory / a_failing_stage_fails_the_job_typed_with_the_stages_error | 503-544 | REAL | S4 | stage error code preserved; no panic-path test (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| completed_lifecycle_… | YES | exact packet/byte/frame counts; queue peaks vs axes; byte-exact salt blocks at exact offsets (any drop/reorder/duplicate fails); dropped_events()==0 |
| cancellation_… | YES | wrong terminal (Completed after cancel) hits unreachable!; prefix arithmetic; re-demux payload equality; patched header field |
| a_full_event_hub_… | YES | seen==2 and dropped==10 are exact — a blocking hub would hang wait(); a lossy counter fails the equality |
| a_source_without_audio_… | YES | exact dotted code + Failed state |
| a_filter_request_… | YES | submit returns Err with exact code (refused before any thread runs — drop(tx) first) |
| a_failing_stage_… | YES | exact propagated code "transcode.test.no-decoder" |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "queued→running→progress→completed/failed/cancelled over the real two-thread pipeline" | job_lifecycle.rs:1-6 | REAL | all five terminals exercised (Completed/Cancelled/Failed ×2 + submit-refused); ran green |
| "the stage queue's high-water marks … never exceed the configured axes" | job_lifecycle.rs:18-19 | REAL | L307-316 asserts both axes on the completed run |
| "the worker never blocks on observability" | job_lifecycle.rs:20-22 | REAL | L434-438: job completes with a 2-slot hub |
| "the partial WAV is structurally valid (our own demuxer re-parses it)" | job_lifecycle.rs:23-25 | REAL | L404-415 WavDemuxer re-parse, payload equality |

## 7. Dead / duplicated / unreachable code
None in-file. The tested crate is UNREACHABLE from shipped entry points (zero dependents) — these tests prove library behavior, not shipped behavior.

## 8. Dependencies & flags
codec_api, codecs (PcmDecoder), demux (WavDemuxer), media-types, plugin-api (Demuxer/SourceStream traits), testkit (Must/assert_err), transcode (the crate under test). No features. No unsafe. No I/O outside memory.

## 9. Open questions for the next agent
- Does any test in the crate drive `worker_panic`/`flush_undrained`? If not, the panic→error doctrine (error.rs:341-351) is UNVERIFIED end-to-end.
- wav_roundtrip.rs / record.rs / mp4_remux.rs (other agents' files) may cover the sink I/O error constructors — cross-check F-001 of error.rs.
