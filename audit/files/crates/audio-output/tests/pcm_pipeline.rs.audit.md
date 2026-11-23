# AUDIT: crates/audio-output/tests/pcm_pipeline.rs
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-23 |
| Lines / functions | 247 / 4 tests + 3 helpers |
| File verdict | REAL (in-memory pipeline-integrity proof on the null backend) — proves NOTHING about what a user hears; no audible path exists anywhere in the repo |
| Reachable from shipped entry point? | YES for every production type exercised: PcmPump/SinePattern/NullBackend are the shipped audio path (apps/cli pipeline_host.rs:52-53,1599 opens NullBackend); test itself runs in cargo test --workspace |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | sed -n (quotes verified); read of src/pump.rs, src/pattern.rs (oracle independence), src/null.rs; baseline citation audit/runs/baseline/cargo-test-workspace.txt:206-212 (4/4 ok) |

## 1. One-paragraph verdict
This is a genuine, strong data-integrity and pacing proof of the producer half
of the audio pipeline: synthesized sine → PcmPump (high-water budget vs the
clock's consumed count) → SPSC ring → render_into → capture, verified
per-sample against an independent pure-function oracle (pattern.rs:73-76,
sample_at(n) = amp·sin(2πfn/rate) in f64 — its purity is pinned by its own
unit test), plus honest drop/backpressure policies and a compile-time
tripwire that the cpal gate is closed. What it proves about what a user
hears: NOTHING, twice over. (1) The "device" is NullBackend with capture: the
"played audio" is a Vec<u8> in the test process; no OS audio API, no speaker.
(2) The SHIPPED build is the same: apps/cli opens NullBackend with
capture:false (pipeline_host.rs:1599) — the shipped player decodes audio and
discards it; a user of lumen-cli hears silence by design (ADR-005 era, cpal
NOT admitted; the matrix says so honestly, docs/feature-matrix.toml:2362 "no
real-device evidence exists"). The header's first line sells "decoded frames"
where the test actually pushes a synthesized SinePattern — no decoder, no
PTS, no format conversion runs in this suite.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:206-212 — 4/4 ok
  (exit_code=0 at line 240).
- Oracle independence verified by read: pattern.rs:71-76 sample_at is a pure
  function of the instant; the pipeline test's expected values are computed
  per-instant, not read back from fill state (pattern.rs's own test
  "the_oracle_is_a_pure_function_of_the_instant" pins this).
- Pacing verified by read: pump.rs:146-154 in_flight = written − clock
  sample_count; push refuses over-budget buffers BEFORE touching the sink
  (pump.rs:203-217); typed audio.pump_backpressure / audio.pump_clock_mismatch
  / audio.pump_high_water_invalid all exist and are tested (pump.rs tests).
- Mutation reasoning: any dropped, duplicated, reordered, or corrupted sample
  in the 44 100-instant stream fails the per-instant compare (lines 134-138);
  a pump that silently lost frames fails written_frames/dropped accounting
  (lines 121-123, 174-175); a lying backpressure error fails the typed-error
  match (lines 217-221); admitting cpal without updating the test fails
  COMPILE (lines 238-243).

## 3. Findings
F-001 [S3]
Where: crates/audio-output/tests/pcm_pipeline.rs:1-4
Code:
```
//! The wave-14 PCM pipeline proof (issue #1): **decoded frames → the
//! bounded device queue → the sink trait, paced by the playback
//! clock** — the whole path behind the ADR-005 seam, on the null
//! backend, with no new dependencies.
```
What is wrong: "decoded frames" is false for this suite. No decoder runs; the
buffers come from SinePattern::fill (line 97), wrapped with
`pts: Stamp::Missing` (line 60). The pump's input TYPE is the decoded-frame
type, so the pipeline shape is right, but the actual producer path — codec
output, s16→f32 conversion, PTS stamping, A/V sync handoff — is untested
here. Why it matters: the header (and pump.rs's module doc, "decoded
AudioBuffers") invites the reading that the decode→output path is proven; a
PTS bug, a channel-order bug, or a conversion bug would pass this suite. What
correct looks like: either a decoder-fed variant (the committed corpus AVI +
PCM through the real decode path) or a header that says "synthesized
AudioBuffers of the decoded-frame type".
Confidence: HIGH — Verification: read (whole file + pump.rs + pattern.rs).

F-002 [S3]
Where: crates/audio-output/tests/pcm_pipeline.rs:8-15 and the whole suite
Code:
```
//! 1. `clock_synced_pump_delivers_the_exact_sine_stream` — a
//!    [`SinePattern`] (440 Hz) is pushed through a [`PcmPump`] whose
//!    pacing clock is the **same `SampleCounterSource` a real
//!    `MediaClock::audio_master` reads**; the drive loop renders only
//!    when the pump says the device is fed. Result: the device
//!    capture is **bit-for-bit the oracle's sample stream**, zero
//!    underruns, zero drops, and the media clock lands on exactly
//!    1.000 s after 44 100 instants.
```
What is wrong: nothing quoted is false — but the suite is titled "the
end-to-end audio path" (wave-14 PCM pipeline proof) while its entire
"endpoint" is the null device's capture Vec. The user-audible answer: this
test proves the bits that WOULD be played are exact; it proves nothing about
playback, because playback does not exist (cpal refused by typed marker —
test 4 — and the shipped CLI renders into a discarding null sink,
apps/cli/src/pipeline_host.rs:1599). Every doc that cites this suite
(docs/feature-matrix.toml:2361 evidence string) correctly scopes it to
"pump/pattern/clock", so no external claim is false; the risk is a reader
inflating "PCM pipeline proof" into "audio output works".
Confidence: HIGH — Verification: read + baseline.

F-003 [S4]
Where: crates/audio-output/tests/pcm_pipeline.rs:100-108
Code:
```
        while !pump.fits(CHUNK as u64) {
            let mut out = vec![0.0f32; CHUNK * 2];
            let stats = device.render(&mut out).must();
```
What is wrong: the "drive loop" renders from the TEST thread synchronously —
there is no device thread, no scheduler, no pacing in wall-clock time
whatsoever (the pump paces on frame counts, not seconds). This is the
declared v0 model (null.rs doc: "manual driving on the caller's thread"),
recorded here as a limitation, not a defect. Any real-device behavior
(callback timing, drift, preemption) is untested.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| stereo_f32 (helper) | 42-49 | REAL | — | format literal |
| buffer (helper) | 52-62 | REAL | — | f32→AudioBuffer, pts always Missing (see F-001) |
| as_f32s (helper) | 65-72 | REAL | — | capture→f32 decode |
| clock_synced_pump_delivers_the_exact_sine_stream | 74-140 | REAL | — | 30 ticks, per-instant oracle compare, clock==1.000s, zero underrun/drop |
| drop_policy_bounds_memory_and_keeps_the_prefix | 142-191 | REAL | — | 4 accepted/2 dropped ticks, bounded in_flight + buffered asserts, exact prefix |
| backpressure_is_typed_and_clears_as_the_clock_advances | 193-234 | REAL | — | typed ResourceLimit, refused push never counts, clears after render |
| the_cpal_seam_refuses_with_the_typed_marker | 236-247 | REAL | — | compile-time !ADMITTED tripwire + typed runtime refusal |

## 5. Test honesty
All 4 pass in baseline (cargo-test-workspace.txt:209-212). Would they fail if
the code were broken? Yes: sample corruption/drop/duplication (per-instant
oracle), pacing-budget violation, silent drop (loss ledger), wrong error
kind/code, cpal admission without test update (compile error), clock not
advancing on silence, written/consumed accounting drift. No: PTS handling
(never set), channel mapping (same value all channels by design), any
real-device behavior (none exists).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decoded frames → the bounded device queue → the sink trait" | file header:1-2 | SHALLOW (type-path only) | no decoder in suite; SinePattern + pts Missing (F-001) |
| "bit-for-bit the oracle's sample stream, zero underruns, zero drops" | file header:13-15 | REAL | lines 124-139 asserts, baseline green |
| "media clock lands on exactly 1.000 s after 44 100 instants" | file header:15 | REAL | line 124-128 assert |
| "wave-14 PCM pipeline proof: clock-synced bounded-queue pump, exact sine oracle, drop/backpressure policy" | docs/feature-matrix.toml:2361 evidence | REAL | all four elements present and passing |
| "no real-device evidence exists (typed marker refuses on every platform)" | docs/feature-matrix.toml:2362 | REAL | cpal.rs marker; test 4 pins it |

## 7. Dead / duplicated / unreachable code
None in this file. (The duplicated s16→f32 converter noted in the
callback_budget audit lives in common/, not here.)

## 8. Dependencies & flags
Deps: audio-engine (AudioSink, AudioBuffer), audio-output, media-clock,
media-types, testkit. No features, no unsafe, no threads.

## 9. Open questions for the next agent
- The first-vertical corpus (AVI+MJPEG+PCM) decoded end-to-end into this pump
  would convert F-001 from SHALLOW to REAL — the pieces exist; nobody wired
  them in a test.
- apps/cli uses NullBackend capture:false — confirm no shipped path ever
  reaches a non-null sink (cpal.rs is the only flip point; AUDIO_ADR005_ADMITTED=false).
