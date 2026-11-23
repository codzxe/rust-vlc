# AUDIT: crates/audio-engine/src/pipeline.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 993 / 17 (+14 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli `open_audio_session` (apps/cli/src/pipeline_host.rs:1588-1603) builds `AudioPipeline::new`; web host too (apps/web/src/host.rs:1746-1752) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `cargo test -p audio-engine --lib pipeline::` → 14 passed / 0 failed (1.10s); `cargo` absent from PATH, used `~/.cargo/bin` |

## 1. One-paragraph verdict
This IS the real audio output graph of the shipped build — but it feeds a null device only. The full production chain (verified): lumen-cli `open_audio_session` (apps/cli/src/pipeline_host.rs:1588) constructs `NullBackend::open_stream` + `AudioPipeline::new(PipelineSpec{tracks:1})` inside `AudioLimb`; decoded audio flows playback-engine `Pipeline` → `CliFrameConsumer::accept` (apps/cli/src/pipeline_host.rs:1228) → `accept_audio` → `AudioLimb::stage` → `pipeline.process → mix → take_output → sink.write_bytes` into the NullSink ring; the pump/device half advances the sample counter that is the session's audio-master clock. `engine-info` honestly labels this `audio.null-device` (apps/cli/src/host.rs:1639; baseline audit/runs/baseline/rc-smoke.txt:127). The DSP (s16→f32, channel map, straight sum, volume, pooled-slot interleave, drift-governor seam) is real and tested; no real OS device exists anywhere — the cpal seam is a typed refusal (crates/audio-output/src/cpal.rs:43 `AUDIO_ADR005_ADMITTED=false`, :98-107 returns `Err(BackendNotAdmitted::CPAL)`). Bugs found are error-path/state-machine defects (stale pts, poisoned staged counts after a failed mix, dropped staged allocation on map error), all latent because production runs single-track and treats mix errors as fatal.

## 2. What actually works (verified)
— `cargo test -p audio-engine --lib pipeline::` → `test result: ok. 14 passed; 0 failed; 0 ignored` (includes the +1000/−1000 ppm frame-count conservation test over 1000 quanta, the OUT_SLOTS retention-cap test, and the zero-correction bit-exact bypass).

## 3. Findings
F-001 [S3]
Where: crates/audio-engine/src/pipeline.rs:328-337 + 305-307 + 376
Code:
```
        for staged in &self.staged {
            if *staged != frames {
                return Err(error::chunk_alignment());
            }
        }
```
What is wrong: the alignment-error return happens before `self.timing.take()` (line 376), and `process` only records a pts `if self.timing.is_none()` (305-307). After any failed `mix`, the next successful `mix` emits the stale pts of the pre-error quantum, not the new one — a wrong-timestamp quantum on the error-recovery path. Why it matters: A/V sync truth downstream (the ring's pts bookkeeping) is fed a lie. What correct looks like: clear `timing` (and `staged`) on every error exit of `mix`. Latent: production is single-track and treats mix errors as fatal (CLI propagates EngineError up). Confidence: HIGH — Verification: read.

F-002 [S2]
Where: crates/audio-engine/src/pipeline.rs:382-384 (reset) vs 328-333
Code:
```
        for staged in self.staged.iter_mut() {
            *staged = 0;
        }
```
What is wrong: `staged` counts are zeroed only after a *successful* mix; `process` never zeroes them per-track entry. A track that stops staging (EOS in a multi-track session) leaves its last count in `staged[track]`, so every later `mix` fails `chunk_length_mismatch` forever — a permanent session-fatal lock for multi-track use — and a failed mix leaves stale counts, so the *next* mix compares a fresh count against a stale one. What correct looks like: `staged[track] = 0` at the top of `process` (a failed stage must not resurrect the previous quantum's count) and an explicit per-track retire call. Latent today: both shipped hosts construct `tracks: 1` (apps/cli/src/pipeline_host.rs:1600, apps/web/src/host.rs:1749). Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/audio-engine/src/pipeline.rs:299-303
Code:
```
        let mut staged = std::mem::take(&mut self.stage[track]);
        let staged_result = map.apply(&planes, &mut staged);
        self.scratch = planes;
        staged_result?;
        self.stage[track] = staged;
```
What is wrong: on `map.apply` error the `?` returns before `self.stage[track] = staged`, so the mem::take leaves an empty Vec in the slot and the taken allocation is dropped — the "no per-frame allocation after negotiation" contract (module docs, lines 18-21) breaks on every failing stage, and `staged[track]` keeps its pre-error count (compounding F-002). What correct looks like: write `self.stage[track] = staged` before `staged_result?`. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/audio-engine/src/pipeline.rs:222-229
Code:
```
    pub fn device_format(&self) -> AudioFormat {
        AudioFormat {
            sample: SampleFormat::F32,
            channels: self.spec.channels,
```
What is wrong: device format is hard-pinned to interleaved f32 (doc admits it, "the device format v0 devices speak"); any real device that wants s16/be needs a stage-D conversion that does not exist yet. Honest but a real-device prerequisite. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/audio-engine/src/pipeline.rs:288-290
Code:
```
        if frame.format.rate != self.spec.rate {
            return Err(error::unsupported_format("audio.rate_mismatch"));
        }
```
What is wrong: any rate change (device re-negotiation, 44.1→48 kHz track swap) is a typed refusal — the pipeline must be rebuilt from scratch; no re-negotiation path. Acceptable v0, but a cpal integration (Wave 8) hits this immediately when the OS device's native rate ≠ the track's rate (the micro-resampler is drift-only, ±1000 ppm). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| OUT_SLOTS const | 39-45 | REAL | — | 4-slot rotation; cap verified by test |
| PipelineSpec | 47-60 | REAL | — | plain data |
| AudioPipeline struct | 64-102 | REAL | — | all buffers owned/reused |
| new | 113-136 | REAL | — | validates zero rate/channels/tracks → typed error |
| with_drift_governor | 143-147 | REAL | — | builder sugar |
| arm_drift_governor | 155-157 | REAL | — | late-arming; used by CLI (unarmed in prod open path) |
| set_drift | 162-165 | REAL | — | direct control |
| drift | 168-171 | REAL | — | getter |
| observe_drift | 185-192 | REAL | — | unarmed → ZERO no-op (verified by test) |
| reset_drift | 198-204 | REAL | — | resets governor+resampler |
| drift_stats | 208-213 | REAL | — | counters |
| spec | 216-218 | REAL | — | getter |
| device_format | 222-229 | REAL | S4 | hard-pinned f32 interleaved (F-004) |
| set_volume / volume | 237-248 | REAL | — | delegates to Mixer |
| set_channel_map | 257-267 | REAL | — | validates out shape |
| process | 284-309 | REAL | S3 | error path drops staged alloc + stale count (F-003) |
| mix | 328-394 | REAL | S2/S3 | stale pts + poisoned counts on error paths (F-001/002); warm path verified |
| take_output | 398-400 | REAL | — | Option take |
| pooled_slot | 412-447 | REAL | — | warm/cold/evict paths exercised by test; `unreachable!` at 435 is sound (fresh local, refcount 1) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unity_single_track_is_bit_exact_end_to_end | YES | exact f32 values incl. -32768→-1.0 |
| volume_halves_the_output | YES | exact 0.5 gain values |
| channel_swap_routes_the_pair | YES | swapped pair asserted |
| mono_track_maps_into_a_stereo_pipeline | YES | duplicated channels asserted |
| two_tracks_mix_straight | YES | sum + pts provenance |
| shape_and_geometry_errors_are_typed | YES | 7 error codes asserted |
| misaligned_quanta_are_typed_errors | YES | proves the F-002 poison path errors (but no recovery test) |
| frame_counts_are_conserved_through_the_quantum | YES | 1470 in = 1470 out, byte count |
| quantum_output_rotation_is_bounded_and_values_stay_exact | YES | adversarial retention, exact bytes, cap == OUT_SLOTS |
| zero_correction_is_the_bit_exact_bypass | YES | byte-eq armed vs plain |
| nonzero_correction_shifts_the_quantum_by_the_exact_ratio | YES | ±1000 ppm, 1000 quanta, ±1 frame tolerance |
| armed_governor_closes_the_loop_through_observe_drift | YES | step-limited 250 ppm first window, live frame growth |
| unarmed_pipeline_ignores_drift_observations | YES | ZERO + unmeasurable counters |
| reset_drift_restarts_the_measurement_regime | YES | resets==1, exact 1470 after reset |
Missing: no test for a mix()-error recovery sequence (F-001/F-002/F-003 blind spot).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no per-frame allocation after negotiation" (module doc L18-20) | pipeline.rs | REAL on happy path / FALSE on map-error path | F-003; allocation dropped on `staged_result?` |
| "Frame counts are conserved end to end" (L20-21) | pipeline.rs | REAL at ZERO drift | test frame_counts_...; at non-zero drift ±1000 ppm by design (documented) |
| engine-info `audio.null-device` | apps/cli/src/host.rs:1639 | REAL (honest label) | the only sink is NullSink (open_audio_session, pipeline_host.rs:1598) |
| cpal real backend exists | — | FALSE (does not exist; typed refusal) | audio-output/src/cpal.rs:43,98-107 |

## 7. Dead / duplicated / unreachable code
None. Every public method is reachable from shipped hosts except `with_drift_governor` (the builder form — the CLI uses the late `arm_drift_governor` instead; the builder is used by tests only) — near-dead API duplication, S4-grade nit.

## 8. Dependencies & flags
Depends on: media-clock (DriftCorrection, DriftGovernor), media-types, crate::{buffer,channel_map,convert,error,governor,mix,resample,volume}. No feature gates in this file; audio-engine has no default-off features hiding this path (see Cargo.toml audit).

## 9. Open questions for the next agent
- Wave 8 (cpal): confirm `AudioBackend` (audio-output/src/device.rs) is the only seam a cpal adapter must implement, and whether stage-D integer/s16 conversion is planned (F-004) — the pinned f32 device format will be the first integration failure.
- Does any host ever construct `tracks > 1` (would promote F-002 to live)?
- pump.rs / null.rs auditors: does `PcmPump` (audio-output) ever feed AudioPipeline in production, or is it test/demo-only? (CLI uses direct `sink.write_bytes`, not PcmPump.)
