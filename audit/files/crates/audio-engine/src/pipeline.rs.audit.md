# AUDIT: crates/audio-engine/src/pipeline.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 993 / 20 production fns + 13 tests |
| File verdict | REAL for the quantum path (process/mix/take_output, reached from the CLI audio limb); the drift-governor half is UNREACHABLE in the shipped build (never armed); the multi-track mixer is UNREACHABLE (all hosts pin `tracks: 1`) |
| Reachable from shipped entry point? | YES: `lumen` CLI → `open_audio_session` (apps/cli/src/pipeline_host.rs:1594-1617) → `AudioLimb::stage` (pipeline_host.rs:659-668) → `process`/`mix`/`take_output` (this file) → `sink.write_bytes` (null device) |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:3 |
| Commands run | Read/Grep/sed only (no cargo; the sandbox Bash output filter strips the substring `[mus` — all quotes below re-verified with the Read/Grep tools; the `#ust_use]` seen in raw sed output is a display artifact, the real bytes are `#[must_use]`, as already documented in worklog.md:739) |

## 1. One-paragraph verdict

This is the composed audio DSP path (stage A s16→f32 planes, per-track
channel map, per-track volume, straight sum, optional micro-resampler,
stage D interleave into pooled output slots). The quantum path is real
and well-tested with exact-value assertions; it runs in the shipped CLI
for every PCM-bearing session (mjpeg+pcm AVI per W00 baseline). Two
halves are dead in the shipped build: (a) the drift governor / 3.B1
micro-resampler loop — production constructs the pipeline unarmed and
nothing ever arms it, so `observe_drift` answers `ZERO` forever and the
resampler is bypassed by ownership; (b) the multi-track mixer — both
shipped hosts construct `PipelineSpec { tracks: 1 }`, so cross-track
mixing is test-only. Neither is hidden: the comments say so, but the
module docs sell the drift seam as "the wiring's one-call loop
closure", which is only true in tests.

## 2. What actually works (verified)

— Call chain (static): `apps/cli/src/host.rs` open →
`open_audio_session` (pipeline_host.rs:1594) → `AudioPipeline::new`
(this file :113) with `tracks: 1`; per decoded audio frame
`CliFrameConsumer::accept` → `AudioLimb::stage`
(pipeline_host.rs:659-668):
```
        self.pipeline.process(0, frame)?;
        self.pipeline.mix()?;
        if let Some(buffer) = self.pipeline.take_output() {
            self.sink.write_bytes(buffer.data.as_slice())?;
```
— Verified by in-file tests with exact float equality
(`unity_single_track_is_bit_exact_end_to_end`, :494-522) and by W00's
baseline (PCM AVI audio decodes and renders through the null sink).
— Zero-allocation claim verified by reading `mix` (:328-394): pooled
`out_slots` rotation via `SharedBytes::get_mut`, staged chunks not
cleared (comment :385-392 explains the false-reuse avoidance).

## 3. Findings

### F-001 [S3] The drift-governor loop (card 3.B1) is never armed in any shipped build
Where: pipeline.rs:138-192 (the seam), apps/cli/src/pipeline_host.rs:1600-1604
(construction), :639-641 (arming wrapper), apps/cli/tests/sync_scenarios.rs:93
(the only arming call)

Code (apps/cli/src/pipeline_host.rs:1600-1604 — production construction):
```
    let pipeline = AudioPipeline::new(PipelineSpec {
        rate,
        channels,
        tracks: 1,
    })?;
```
Code (apps/cli/src/pipeline_host.rs:600-602 — the regime admission):
```
    /// The governor is therefore armed by compositions
    /// that pace (the S8 convergence harness, and the production pump
    /// once 3.B3's backpressure lands steady-state pacing).
```
What is wrong: `AudioPipeline::new` leaves `governor: None`
(pipeline.rs:131). The only production-reachable arming path is
`AudioLimb::arm_drift_governor` (pipeline_host.rs:639-641), whose only
caller in the repo is `apps/cli/tests/sync_scenarios.rs:93` (a test).
The pump dutifully calls `observe_audio_drift` once per rendered
quantum (host.rs:618 → pipeline_host.rs:613 → this file :185-192),
which without a governor returns `DriftCorrection::ZERO` and does
nothing. The micro-resampler therefore never runs in the shipped
binary; A/V drift correction is a test-only feature.
Why it matters: the module docs present the seam as working wiring
("the wiring's one-call loop closure", :140); a reader of the docs
would believe shipped playback performs drift correction. It does not.
What correct looks like: arm the governor in the wall-clock pump path
(where production is paced at the consumption rate — exactly the
`pump_wallclock` path that W03-A01's playback-engine audit showed is
itself unreachable in the CLI), or mark the whole 3.B1 surface
`#[doc(hidden)]`-equivalent honest-dormant in the shipped-build docs.
Confidence: HIGH — Verification: read + repo-wide Grep for
`arm_drift_governor|with_drift_governor` (callers: tests only).

### F-002 [S3] Multi-track mixer is production-dead: every shipped host pins `tracks: 1`
Where: pipeline.rs:59 (`pub tracks: usize`), :328-334 (alignment), apps/cli/src/pipeline_host.rs:1600-1604, apps/web/src/host.rs:1746-1749

Code (pipeline.rs:329-334):
```
        let frames = self.staged[0];
        for staged in &self.staged {
            if *staged != frames {
                return Err(error::chunk_alignment());
            }
        }
```
What is wrong: the mixer, per-track volumes, per-track channel maps
and the cross-track alignment error exist and are tested
(`two_tracks_mix_straight`, :605-625), but both shipped constructions
pass `tracks: 1`, so none of it can execute outside tests. Not a bug in
the code — an honest-capacity note the docs do not carry: doc 03 §4.8's
"mix" stage is single-source in every shipped build.
What correct looks like: a doc note, or wire real multi-track (e.g.
audio + a second stream) when a host needs it.
Confidence: HIGH — Verification: read (both construction sites).

### F-003 [S4] Error paths drop the reused buffers (D-1 churn on repeated damage)
Where: pipeline.rs:297-303

Code:
```
        let mut planes = std::mem::take(&mut self.scratch);
        let frames = frame_to_f32_planes(frame, &mut planes)?;
        let mut staged = std::mem::take(&mut self.stage[track]);
        let staged_result = map.apply(&planes, &mut staged);
        self.scratch = planes;
        staged_result?;
        self.stage[track] = staged;
```
What is wrong: on a `frame_to_f32_planes` error (`audio.partial_frame`)
the taken `planes` is dropped and `self.scratch` stays empty; on a
`map.apply` error the taken `staged` is dropped and
`self.stage[track]` stays empty. Every subsequent quantum re-allocates
and re-warms the pools. The zero-allocation contract (:18-20 "no
per-frame allocation after negotiation") holds only while no frame ever
fails. Shape mismatches (rate/channels) are checked before the take,
so the churn needs repeated partial-frame payloads — a corrupt stream
reaching the mixer.
What correct looks like: restore the buffers before `?` (swap back on
error), as `mix` already does for `mixed` (:341-345).
Confidence: HIGH — Verification: read.

### F-004 [S4] `unreachable!` on the fresh-slot path
Where: pipeline.rs:433-436

Code:
```
    let filled = match fresh.get_mut() {
        Some(bytes) => planes_to_f32_interleaved_into(planes, bytes),
        None => unreachable!("fresh SharedBytes has refcount 1"),
    };
```
What is wrong: a panic macro in the audio hot path. The invariant
(freshly built `SharedBytes::from_vec` has refcount 1) holds by
construction today; if `SharedBytes` ever gains interior sharing, this
panics inside playback. Low risk, but a typed error or
`unwrap_or_else` with the allocation repeated would keep the
no-panics posture the rest of the crate maintains.
Confidence: HIGH — Verification: read.

### F-005 [S4] `PipelineSpec.rate` documented as "the first track's rate in practice" — nothing enforces or derives it
Where: pipeline.rs:50-53 vs apps/cli/src/pipeline_host.rs:1600-1604

Code (pipeline.rs:50-53):
```
    /// The mixer rate — the first track's rate in practice (doc 03
    /// §4.8 e); every later track must match in v0 (rate conversion
    /// is the micro-resampler card).
```
What is wrong: the doc phrase "in practice" describes a convention of
the current hosts, not a property of this code; any rate mismatch is
only caught per-frame at `process` (:288-290, `audio.rate_mismatch`)
after the session has already been built on the wrong assumption. A
wrong negotiation fails one frame at a time rather than at open.
Bounded blast radius (typed error, no panic), hence hygiene.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AudioPipeline::new` | 113-136 | REAL | — | validates rate/channels/tracks ≠ 0; identity maps |
| `with_drift_governor` | 143-147 | REAL | S3 | builder for a feature no shipped host arms (F-001) |
| `arm_drift_governor` | 155-157 | REAL | S3 | late-arming half; test-only callers (F-001) |
| `set_drift` | 162-165 | REAL | — | direct control; ZERO restores exact bypass |
| `drift` | 168-171 | REAL | — | getter |
| `observe_drift` | 185-192 | REAL-UNPROVEN | S3 | loop closure; dormant without governor (F-001); governor math lives in media-clock/governor (not this file) |
| `reset_drift` | 198-204 | REAL | — | governor + resampler reset |
| `drift_stats` | 209-213 | REAL | — | all-zero when unarmed |
| `spec` | 216-218 | REAL | — | |
| `device_format` | 222-229 | REAL | — | interleaved f32 at mixer shape |
| `set_volume` | 237-239 | REAL | — | delegates to Mixer (typed range error) |
| `volume` | 246-248 | REAL | — | |
| `set_channel_map` | 257-267 | REAL | — | validates out-shape vs negotiated shape |
| `process` | 284-309 | REAL | S4 | shape checks; stage A + map; error-path buffer loss (F-003) |
| `mix` | 328-394 | REAL | — | alignment; straight sum; drift bypass by ownership at ppm==0; pooled stage D; `timing` first-stager pts |
| `take_output` | 398-400 | REAL | — | once per true mix |
| `pooled_slot` | 412-447 | REAL | S4 | warm path get_mut; cold path bounded at OUT_SLOTS; `unreachable!` (F-004); eviction replaces a possibly-shared slot (Arc keeps downstream clone alive — safe) |

## 5. Test honesty (in-file, :449-993)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `unity_single_track_is_bit_exact_end_to_end` | YES | exact f32 equality incl. −32768→−1.0 edge |
| `volume_halves_the_output` | YES | exact ratio |
| `channel_swap_routes_the_pair` | YES | exact swapped values |
| `mono_track_maps_into_a_stereo_pipeline` | YES | exact duplication |
| `two_tracks_mix_straight` | YES | 8192+4096 sum exact; pts = first stager |
| `shape_and_geometry_errors_are_typed` | YES | pins all typed error codes incl. out-of-range track |
| `misaligned_quanta_are_typed_errors` | YES | 2 vs 1 frames → `audio.chunk_length_mismatch` |
| `frame_counts_are_conserved_through_the_quantum` | YES | 1470 frames, 11760 bytes |
| `quantum_output_rotation_is_bounded_and_values_stay_exact` | YES | adversarial retention caps pool at OUT_SLOTS; no cross-quantum leakage |
| `zero_correction_is_the_bit_exact_bypass` | YES | byte-equal vs plain pipeline |
| `nonzero_correction_shifts_the_quantum_by_the_exact_ratio` | YES | ±1000 ppm over 1000 quanta, ±1 frame tolerance |
| `armed_governor_closes_the_loop_through_observe_drift` | YES | step-limited +250 ppm first window; live ratio on next quantum |
| `unarmed_pipeline_ignores_drift_observations` | YES | pins the production-dormant behavior (ironically proves F-001) |
| `reset_drift_restarts_the_measurement_regime` | YES | counters + exact 1470 after reset |

High-quality suite: exact values, no tautologies, no mocks of the
subject. Not executed by me (no cargo); W00 ran the workspace suite
green except 2 golden-drift failures elsewhere. UNVERIFIED per-test.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decode → convert → volume + channel map → straight mix → device format" | :1-2 | REAL | call chain §2 |
| "no per-frame allocation after negotiation" | :18-19 | REAL (happy path) / SHALLOW (error path) | F-003 |
| "a track whose rate differs … is a typed error" | :21-22 | REAL | :288-290 + test |
| "the wiring's one-call loop closure" (drift governor) | :139-140 | FALSE for shipped build | never armed outside tests (F-001) |
| "mixer rate — the first track's rate in practice" | :50-51 | REAL-UNPROVEN | convention of hosts, not enforced here |

## 7. Dead / duplicated / unreachable code

- Governor/resampler path: real code, dormant in every shipped host
  (F-001) — the resampler executes only under test-set non-zero drift.
- Multi-track mixer paths (`Mixer`, per-track volume/maps, alignment
  error): production-dead, `tracks: 1` everywhere (F-002).

## 8. Dependencies & flags

- `media_clock::DriftCorrection` (the correction type),
  `crate::{buffer, channel_map, convert, governor, mix, resample, volume}`.
  No feature gates. audio-engine is a workspace member and ships in
  the `lumen` CLI binary (W00 release build PASS).

## 9. Open questions for the next agent

- Wave 8 (audio-output): does `NullDevice::render`'s
  `write_bytes`/`buffered_frames` interplay with `stage` hold under
  ring-full conditions (typed `ring full` error path swallows a whole
  quantum? `stage` returns Err after `take_output` — the quantum is
  then lost; who re-stages it?).
- Wave 3 A02 (governor.rs in audio-engine — if assigned elsewhere):
  the step-limiter/window math cited by the tests (`+250` first
  window, 1 Hz window) lives in `crate::governor`; I did not audit it
  here (not in my file list).
