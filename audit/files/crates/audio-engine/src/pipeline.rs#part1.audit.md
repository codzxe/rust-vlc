# AUDIT: crates/audio-engine/src/pipeline.rs#part1 (lines 1-448)
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = lines 1-448 (all production code: struct + impl + `pooled_slot`); the `#[cfg(test)] mod tests` at 449-994 is part2 (W07-A02). Boundary chosen at the tests-module start; noted here so part2 does not re-audit 1-448. |
| Lines / functions | 448 / 20 (17 fns/methods + 3 struct/const items) |
| File verdict | REAL (core quantum path; the drift/resampler half is UNREACHABLE-in-production — see F-001) |
| Reachable from shipped entry point? | YES for `new/process/mix/take_output/spec/device_format/observe_drift/reset_drift/drift_stats` (call chain: `lumen open` → apps/cli/src/host.rs:926 `open_audio_session` → pipeline_host.rs:1600-1604 `AudioPipeline::new(PipelineSpec { rate, channels, tracks: 1 })` → `AudioLimb::stage` 658-666 `process(0,..)→mix()→take_output()→sink.write_bytes` → pump `observe_audio_drift` 1059-1062 / seek `reset_drift` 1106). NO production caller for `set_volume`/`volume`/`set_channel_map`/`with_drift_governor`/`arm_drift_governor`/`set_drift` (rg: only lib.rs doc example, mix.rs internal tests, apps/cli/tests/sync_scenarios.rs:93, audio-engine/tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "AudioPipeline\|PipelineSpec\|observe_drift\|set_drift\|arm_drift" apps crates` (wiring + setter callers); `rg -n "set_channel_map\|drift_stats\|set_volume" apps crates` (no production callers); `sed -n '540,680p;1585,1640p;1050,1075p' apps/cli/src/pipeline_host.rs` (limb wiring, construction, pump call); Read on pipeline.rs 1-509; `rg -n "fn chunk_alignment" -A 10 crates/audio-engine/src/error.rs` (code string `audio.chunk_length_mismatch` confirmed); `rg -n "get_mut" crates/media-types/src/packet.rs` (Arc::get_mut semantics for the `unreachable!` at 435); `rg -n "out_frames\|fn process" crates/audio-engine/src/resample.rs` (out ∈ [in·ratio−1, in·ratio+1]); `rg -n "pipeline::tests\|zero_alloc" audit/runs/baseline/test-workspace.log.txt` (99-119, 141-151: green). NOTE: Bash `sed` output in this sandbox eats `[m…` sequences (rendered `#ust_use]`); all quotes below were verified with the Read tool against the real file text. |

## 1. One-paragraph verdict
The core quantum path (stage→channel-map→mix→pooled interleave→take_output) is real, reachable from the shipped CLI, conserves frame counts, and is panic-free on every path I traced (index bounds via `get_mut`/`get`, alignment gate before mixing, `unreachable!` at 435 proven unreachable: `Arc::get_mut` on a refcount-1 freshly-built `SharedBytes` — packet.rs:213-216). The hostile question's resampling half has a hard answer: **no sample-rate conversion exists anywhere in this pipeline** — `process` refuses any frame whose rate ≠ the negotiated mixer rate (line 288-290), and the shipped CLI negotiates the mixer rate from the decoded track itself (pipeline_host.rs:1600-1604), so nothing is ever converted; the only resampler present is the ±1000 ppm drift micro-resampler, and it is **dormant in the shipped build** (no production code arms the governor or sets a correction — F-001). Remaining findings are hygiene: a stale module doc claiming "the micro-resampler is a later card" while the struct owns one, a buffer-drop on the `process` error path, and an error-priority inconsistency in `set_channel_map`.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/test-workspace.log.txt:99-119` → `pipeline::tests::*` (13 tests incl. `unity_single_track_is_bit_exact_end_to_end`, `frame_counts_are_conserved_through_the_quantum`, `unarmed_pipeline_ignores_drift_observations`, `nonzero_correction_shifts_the_quantum_by_the_exact_ratio`) all `ok`; `:144,151` zero-alloc quantum and drift-resample paths `ok` (baseline run #3).
— Verified by call-chain grep: the shipped CLI drives this pipeline per decoded audio frame (`AudioLimb::stage`, pipeline_host.rs:658-666) into the NullSink; baseline smoke `sample-aac.mp4` "Buffering → Playing" (INDEX.md corrected matrix) exercised this lane end to end with AAC frames at ZERO drift.
— Verified by reading `resample.rs:188-191` contract (`out_frames ∈ [in·ratio−1, in·ratio+1]`) that `mix`'s byte-length arithmetic (371) stays consistent with `planes_to_f32_interleaved_into`'s `out.len() != needed` typed check (convert.rs:262-265): a 0-frame drift output produces a 0-length slot, filled with 0 bytes — no panic.

## 3. Findings
### F-001 [S3]
Where: crates/audio-engine/src/pipeline.rs:353-360 (with 143-165)
Code:
```
        let (planes, out_frames) = if self.drift.ppm() == 0 {
            (self.mixed.as_slice(), frames)
        } else {
            // Field-disjoint borrows: the resampler mutates while the
            // mixed planes are read (no `mem::take`, no Default).
            let out_frames = self.resampler.process(&self.mixed, frames);
            (self.resampler.planes(), out_frames)
        };
```
What is wrong: The entire drift-corrected half of `mix` — plus `with_drift_governor`, `arm_drift_governor`, `set_drift` — is dead code in the shipped binary. `rg` over all non-test code finds zero callers of any setter; the production open path constructs the pipeline unarmed (pipeline_host.rs:1600-1604) and the pump's `observe_audio_drift` (1059-1062) hits the `governor.as_mut() else return DriftCorrection::ZERO` branch (186-188) forever. The doc comment on `arm_drift_governor` (149-154) admits "the composition root opens the session unarmed", and `AudioLimb::observe_drift`'s regime note (pipeline_host.rs ~615-630) says the governor is armed "by compositions that pace" — i.e., only test harnesses (apps/cli/tests/sync_scenarios.rs:93) and no shipped composition.
Why it matters: Drift correction (card 3.B1) is presented as a shipped capability of the audio path; in the binary users run, A/V sync under clock drift gets exactly zero correction — the bypass is bit-exact by construction. Anyone reasoning from the API surface ("drift is handled") is misled; the feature is UNREACHABLE from the shipped entry point.
What correct looks like: Either the production pump arms the governor where pacing exists (the doc's own "once 3.B3's backpressure lands steady-state pacing" — which has landed, per the ring-parking code in the same file), or the module docs state plainly: drift correction is test-harness-only in v0.
Confidence: HIGH — Verification: read + rg call-chain (all setter call sites enumerated).

### F-002 [S4]
Where: crates/audio-engine/src/pipeline.rs:18-22 (with 84-86 and 277-279)
Code:
```
//! All working buffers are owned by the pipeline and reused (doc 03
//! §4.8 c — no per-frame allocation after negotiation; the first
//! quantum warms the capacities). Frame counts are conserved end to
//! end; a track whose rate differs from the negotiated mixer rate is
//! a typed error (the micro-resampler is a later card, doc 03 §4.7 e).
```
What is wrong: The module doc says "the micro-resampler is a later card", but this same file owns a `MicroResampler` (field at 86, card 3.B1). The distinction it fumbles: the thing that exists is a ±1000 ppm *drift* corrector; *sample-rate conversion* (e.g. 48 kHz→44.1 kHz = −8000 ppm, 8× beyond the limiter) does not exist and never will inside this resampler. `process`'s doc (277-279) compounds it: "rate conversion is the micro-resampler card, doc 03 §4.7 e" — pointing rate conversion at a component whose invariant forbids it.
Why it matters: A reader (or future wiring author) auditing "can we mix a 48k track into a 44.1k session?" is told a card is coming and pointed at the wrong primitive; the honest statement is "v0 has no rate conversion; sessions must negotiate per-rate".
What correct looks like: Rewrite both docs to separate the two: drift micro-resampler (present, ±1000 ppm, dormant in production per F-001) vs rate conversion (absent, typed `audio.rate_mismatch`).
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/audio-engine/src/pipeline.rs:299-303
Code:
```
        let mut staged = std::mem::take(&mut self.stage[track]);
        let staged_result = map.apply(&planes, &mut staged);
        self.scratch = planes;
        staged_result?;
        self.stage[track] = staged;
```
What is wrong: On a `ChannelMap::apply` error, `staged_result?` returns before `self.stage[track] = staged` — the taken `Vec<Vec<f32>>` is dropped and the slot reverts to the empty Vec from `mem::take`. State stays consistent (`staged[track]` keeps the last successful count and the chunk was the last successful one — I traced the error path: no stale-data emission is possible because count and chunk stay paired), but the track's plane allocations are lost and the next successful `process` reallocates every plane — exactly the C6a false-reuse class the file's own comment at 385-392 warns against.
Why it matters: A session feeding interleaved good/bad frames (map reconfigures mid-stream, channel-count changes) pays per-plane reallocation on every error; the D-1 "no per-frame allocation after negotiation" claim (18-19) breaks on that path. Bounded, cold-path-only.
What correct looks like: `self.stage[track] = staged;` unconditionally before `staged_result?` (the buffer is reusable regardless of whether `apply` failed mid-fill).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/audio-engine/src/pipeline.rs:257-264
Code:
```
    pub fn set_channel_map(&mut self, track: usize, map: ChannelMap) -> Result<(), EngineError> {
        if map.out_channels() != self.spec.channels {
            return Err(error::channel_range("audio.channel_count_mismatch"));
        }
        let slot = self
            .maps
            .get_mut(track)
```
What is wrong: The map-shape check runs before the track-index check, so `set_channel_map(99, bad_map)` on a 1-track pipeline reports `audio.channel_count_mismatch`, not `audio.track_index_out_of_range` — inverting the documented error priority (doc 252-256 lists track-range first) and disagreeing with `set_volume`/`volume`, which defer to the mixer's index-first validation.
Why it matters: Callers switching on error codes to distinguish "bad track" from "bad map" get the wrong class; diagnostics mislead. (Unreachable in production today — no shipped caller — so S4.)
What correct looks like: Validate the track index first (or document the actual precedence).
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1: lines 1-448)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `OUT_SLOTS` const + module docs | 1-45 | REAL | F-002 | Rotation depth 4; doc staleness finding. |
| `PipelineSpec` | 48-60 | REAL | — | rate/channels/tracks, Copy. |
| `AudioPipeline` struct | 64-102 | REAL | — | Field docs match fields (75-76 "lowest track index that staged" vs first-to-stage-wins at 305-307 — mismatch noted, harmless single-track in production, fold into S4 hygiene; not separately numbered). |
| `new` | 113-136 | REAL | — | Rejects zero rate/channels/tracks typed; identity maps; dormant resampler/governor. |
| `with_drift_governor` | 143-147 | UNREACHABLE (production) | F-001 | Builder; only tests call it (audio-engine/tests/zero_alloc_resample.rs:73). |
| `arm_drift_governor` | 155-157 | UNREACHABLE (production) | F-001 | Only apps/cli/tests/sync_scenarios.rs:93. |
| `set_drift` | 162-165 | UNREACHABLE (production) | F-001 | Only audio-engine/tests/audio_bench.rs:168. |
| `drift` | 168-171 | REAL (returns ZERO always in production) | — | Trivial getter. |
| `observe_drift` | 185-192 | REAL (no-op unarmed) | — | Early-return branch is the only live one in shipped binary; doc matches behavior. |
| `reset_drift` | 198-204 | REAL | — | Called at seek boundary (pipeline_host.rs:1106); resets unarmed governor + ZERO correction + resampler phase — harmless no-op set. |
| `drift_stats` | 208-213 | REAL | — | All-zero stats in production (no governor); exposed via pipeline_host.rs:1068. |
| `spec` | 216-218 | REAL | — | Trivial. |
| `device_format` | 222-229 | REAL | — | F32/interleaved/mixer rate — matches NullStream format negotiated at open (pipeline_host.rs:1590-1596). |
| `set_volume` | 237-239 | UNREACHABLE (production) | — | No shipped caller; defers to Mixer. |
| `volume` | 246-248 | UNREACHABLE (production) | — | Same. |
| `set_channel_map` | 257-267 | UNREACHABLE (production) | F-004 | Error-priority inversion; no shipped caller. |
| `process` | 284-309 | REAL | F-003 | Rate-mismatch typed refusal (288-290) — THE answer to "is sample-rate conversion correct where it exists": it does not exist; channel-count gate; scratch/staged take-restore with drop-on-error; timing first-to-stage-wins. |
| `mix` | 328-394 | REAL | F-001 | Alignment gate → early false at 0 frames; mixer with field-disjoint borrows; drift seam (ZERO = byte-exact bypass — live path); pooled stage-D; staged counts reset, chunks deliberately kept (correct — the C6a comment is sound). |
| `take_output` | 398-400 | REAL | — | `Option` take; contract "once per true mix". |
| `pooled_slot` | 412-447 | REAL | — | Warm path exact-length + `get_mut`; cold path bounded at OUT_SLOTS with eviction; `unreachable!` at 435 is provably unreachable (fresh `SharedBytes` refcount 1 → `Arc::get_mut` Some, packet.rs:213-216); `byte_len` arithmetic `out_frames * channels * 4` (371) unchecked — 32-bit-only theoretical overflow, allocations of that magnitude cannot succeed, not elevated. |

## 5. Test honesty
Tests live in part2 (449-994, W07-A02's range) — listed here only as external evidence: baseline test-workspace.log.txt:99-119 shows 13 pipeline tests green, including `unarmed_pipeline_ignores_drift_observations` and `nonzero_correction_shifts_the_quantum_by_the_exact_ratio` (which exercise the branches production never reaches). Zero-alloc suites (`:144,151`) pin the warm-path claims. Mutation reasoning: `unity_single_track_is_bit_exact_end_to_end` would fail on any s16→f32 scale or interleave-order regression; `frame_counts_are_conserved_through_the_quantum` would fail on the alignment/reset logic. Deep test-body audit belongs to part2.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Frame counts are conserved end to end; a track whose rate differs from the negotiated mixer rate is a typed error" | module docs 18-22 | REAL | process:288-290 refuses typed; tests green (baseline :101). |
| "(the micro-resampler is a later card, doc 03 §4.7 e)" | module docs 21-22 | FALSE (stale) | MicroResampler is a field (86); it is drift-only (±1000 ppm), not a rate converter — F-002. |
| "Zero allocation after warm-up" | mix docs 316-321 | REAL (success path) / breaks on process-error path | F-003; zero_alloc tests green (baseline :144,151). |
| "a non-zero correction resamples … ZERO is a bit-exact bypass by ownership" | mix docs 347-352 | REAL code / UNREACHABLE in shipped binary | F-001: no production setter caller; bypass is the only live branch. |
| "v0 mixes whole aligned quanta; alignment is the session's job" | mix docs 323-327 | REAL | chunk_alignment() typed error, code `audio.chunk_length_mismatch` (error.rs:86-94). |

## 7. Dead / duplicated / unreachable code
- Dead-in-production (not dead-code-eliminated, but never entered): the non-ZERO half of `mix` (356-359), `with_drift_governor`/`arm_drift_governor`/`set_drift`, `set_volume`/`volume`/`set_channel_map` — all exercised only by tests/doc-examples (call sites enumerated in the header row). This is the F-001 core.
- `pooled_slot` (412-447) vs flac `pooled_sample_slot` (flac/decoder.rs:629-674): third-party-duplication pattern; acceptable (crate boundary), both correct.

## 8. Dependencies & flags
Deps: media-types + media-clock only (Cargo.toml: deps block) — matches boundaries registry comment; no codecs dependency (the pipeline consumes decoded AudioFrames, engines stay decoupled — verified `rg codecs audio-engine/src` = no matches). No feature gates on this file; unconditional in the crate; crate is a workspace member of the shipped CLI graph.

## 9. Open questions for the next agent
- part2 (W07-A02): do the in-file tests cover the `pooled_slot` cold-path eviction branch (437-444) and the 0-frame drift output? Baseline names suggest rotation coverage (`quantum_output_rotation_is_bounded_and_values_stay_exact`).
- A02 owns resample.rs: verify `MicroResampler::process` truncates its own planes to `out_len` (the `planes()` borrow mix() hands to stage D must have length == returned out_frames, or `planes_to_f32_interleaved_into` returns the typed length error — which would surface as playback failure under drift, untestable in production since drift never engages).
- Host wiring: VERIFIED while auditing — apps/cli/src/host.rs:911-926 derives `(channels, rate)` from the selected audio track's harvested geometry facts (`fact.audio_geometry()`) and passes the TRACK's own rate to `open_audio_session`; a non-PCM track without harvested geometry opens limb-less on the wall clock ("never a guess"). So no fixed device rate is forced and `audio.rate_mismatch` cannot fire for a consistent track in the shipped wiring.
