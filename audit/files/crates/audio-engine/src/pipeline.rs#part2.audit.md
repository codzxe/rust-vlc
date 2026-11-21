# AUDIT: crates/audio-engine/src/pipeline.rs#part2 (lines 449-994, `#[cfg(test)] mod tests`)
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 449-994 (entire tests module), per the boundary agreed in part1's audit header ("part1 = lines 1-448 … the `#[cfg(test)] mod tests` at 449-994 is part2 (W07-A02)"). Production code 1-448 audited by W07-A01 — not re-audited here except where a test's claim forces a re-read. |
| Lines / functions | 546 / 20 (2 consts + 4 helpers + 14 test fns) |
| File verdict | REAL (all 14 tests executed green in baseline, assertions discriminate; gaps are S4) |
| Reachable from shipped entry point? | Test code — executed by `cargo test --workspace` (baseline run #3). The 5 drift-seam tests (826-992) exercise paths that NO shipped composition reaches (see resample.rs F-002 / part1 F-001); the 9 core tests cover the live quantum path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Read pipeline.rs 249-333, 404-994 (full tests module + `process`/`mix` context) · `rg -n "partial_frame" crates/audio-engine` (raised in convert.rs:152,264; unit-tested at convert.rs:471 — not at pipeline level) · `sed -n` quote verification (all quotes below) · baseline `test-workspace.log.txt:98-110,123` (14/14 pipeline tests green) · `sed -n '1,80p' governor.rs` (window 1 s, W_c 20 s, step 250 ppm — resolves test-12's comment math) |

## 1. One-paragraph verdict
This is an unusually strong test module: exact-value (not tolerance) assertions on the s16→f32 scale identity, adversarial slot-retention pressure pinning the C6b pool bound at `OUT_SLOTS`, a 1000-quantum ±1000 ppm ratio conservation with ±1 tolerance, and a loop-closure test whose comment math I verified against governor.rs (1 s window × W_c 20 s ⇒ `for_divergence(−20 ms, 20 s) = +1000 ppm`, step-limited to 250 — the asserted `ppm == 250` and `samples == 41` are arithmetically correct). All 14 tests ran green in the baseline workspace run. The defects are coverage gaps, not dishonesty: three misuse/ambiguity classes have zero pinning — silent chunk replacement on re-`process`, silent `pending` overwrite on `mix` without `take_output`, and the pts-semantics ambiguity (field doc says "lowest track index that staged", code is first-to-stage, the one ordering test stages track 0 first so both semantics pass) — plus every test feeds S16-interleaved only, the one format the shipped build produces.

## 2. What actually works (verified)
— Verified by baseline `audit/runs/baseline/test-workspace.log.txt:98-110,123` → all 14 `pipeline::tests::*` `ok` (86 tests total in the crate's lib target, 0 failed) — baseline run #3, release profile.
— Verified by hand-check of test-12's arithmetic: `quantum_ns = 1470×1e9/44100 = 33_333_333` (truncation matches the comment); 31×33.33 ms = 1.033 s ≥ 1 s window (governor.rs:44-45 `CORRECTION_WINDOW_NS = 1_000_000_000`); 1+40 observations = `samples == 41`; desired +1000 ppm (governor.rs:50-52 `INTEGRAL_TIME_NS = 20_000_000_000`) step-limited to 250 (governor.rs:54-55 `STEP_LIMIT_PPM = 250`) — every magic number in the asserts is traceable to a named constant.
— Verified `audio.partial_frame` (the malformed-payload typed refusal promised by `process`'s doc, pipeline.rs:281-283) is unit-tested at the component that raises it: `rg -n "partial_frame" crates/audio-engine` → convert.rs:471 `partial_frames_are_typed_corrupt`, error.rs:104-108.

## 3. Findings
### F-001 [S4]
Where: crates/audio-engine/src/pipeline.rs:269-273 (documented replace semantics), 297-304 (replace code), 376-381 (pending overwrite) — no test in 449-994 pins either
Code:
```
        let pts = self.timing.take().unwrap_or(Stamp::Missing);
        self.pending = Some(AudioBuffer {
            format: self.device_format(),
            data,
            pts,
        });
```
What is wrong: two silent-data-drop misuse paths have zero test coverage and zero runtime enforcement: (a) calling `process` twice for the same track before `mix` silently REPLACES the staged chunk (documented at 272-273 "The slot holds the latest chunk" — the earlier chunk vanishes, no error, no counter); (b) calling `mix` twice before `take_output` silently overwrites `pending` — the first quantum is dropped unread. Neither behavior is asserted anywhere in the module, so a regression to append/accumulate semantics, or to a panic, would not be caught here.
Why it matters: the pump is the only production caller and (per pipeline_host.rs stage flow) stages once and takes once — but the API is `pub`, and a future wiring bug (double stage under a retry, missed take under an error branch) loses audio with no diagnostic. The test module is where this contract should be pinned.
What correct looks like: one test asserting replace-semantics (`process` twice → mix emits only the second chunk's frames) and one asserting overwrite-semantics or a typed error for mix-without-take.
Confidence: HIGH — Verification: read (both code paths traced; `rg` over the module confirms no double-stage/double-mix test).

### F-002 [S4]
Where: crates/audio-engine/src/pipeline.rs:75-76 (field doc) vs 305-307 (code) vs 623-624 (the only ordering assertion)
Code:
```
    /// The quantum's pts (lowest track index that staged).
    timing: Option<Stamp>,
```
```
        if self.timing.is_none() {
            self.timing = Some(frame.pts);
        }
```
```
        // The pts is the first staging track's.
        assert_eq!(buffer.pts, a.pts);
```
What is wrong: the field doc promises "lowest track index that staged", the code implements first-to-stage-wins (`timing.is_none()`), and `two_tracks_mix_straight` stages track 0 first — the one ordering under which both semantics agree. Mutation check (reasoned, not run): replacing 305-307 with `if track == 0 { self.timing = Some(frame.pts) }` passes the entire 14-test suite, including the pts assert. Related: `set_channel_map`'s documented `audio.track_index_out_of_range` error (252-256) is asserted nowhere — `shape_and_geometry_errors_are_typed` (627-694) only exercises the valid-track/bad-map case, so part1's F-004 error-priority inversion is invisible to the suite.
Why it matters: two documented behaviors (pts tie-break, track-range error for set_channel_map) are unpinned; the doc/code disagreement can silently rot either side.
What correct looks like: a track-1-stages-first test pinning first-to-stage (and fixing the field doc), plus one `set_channel_map(99, …)` assertion.
Confidence: HIGH — Verification: read; mutation reasoning above.

### F-003 [S4]
Where: crates/audio-engine/src/pipeline.rs:458-476 (`stereo_s16_frame` — the module's ONLY frame factory), every test 494-992
Code:
```
    fn stereo_s16_frame(pairs: &[(i16, i16)], pts_ticks: i64) -> AudioFrame {
        let mut bytes = Vec::new();
        for (l, r) in pairs {
            bytes.extend_from_slice(&l.to_le_bytes());
            bytes.extend_from_slice(&r.to_le_bytes());
        }
```
What is wrong: every pipeline-level test feeds S16-interleaved stereo (or mono at 566-603) at 44.1 kHz. That is exactly the shipped lane (pcm_s16le → f32 planes) — good — but the pipeline's conversion contract spans U8/S24/S32/F32/F64 × interleaved/planar (convert.rs:188-213), and no pipeline-level test runs any of them through stage-A + channel-map + mix; likewise `audio.partial_frame` is never observed at the pipeline boundary (it is unit-tested one layer down, convert.rs:471). The 5 drift-seam tests (826-992) additionally cover machinery no shipped composition arms (part1 F-001 / resample.rs F-002) — accurate tests, but nothing labels that the covered configuration is production-unreachable.
Why it matters: the moment a second decoder ships (e.g. the Symphonia AAC/FLAC wrappers, which emit f32), the pipeline integration for non-S16 inputs will have zero pre-existing end-to-end pinning at this layer.
What correct looks like: one parameterized format pass (at minimum F32-interleaved, since Symphonia emits it) asserting the same bit-exact identity the S16 tests assert.
Confidence: HIGH — Verification: read + `rg -n "SampleFormat::" crates/audio-engine/src/pipeline.rs` (only S16 in the tests module).

## 4. Per-function table (part2: lines 449-994)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `const RATE` | 456 | REAL | — | 44_100; every test's rate |
| `stereo_s16_frame` (helper) | 458-476 | REAL | F-003 | the module's only factory; S16-LE interleaved; `Timestamp::from_parts(ticks, 1, RATE)` via testkit Must |
| `read_f32_pairs` (helper) | 478-492 | REAL | — | `as_chunks::<8>().0` silently drops a remainder — benign for well-formed buffers (len % 8 == 0 asserted implicitly by exact-value asserts) |
| `unity_single_track_is_bit_exact_end_to_end` | 494-522 | REAL | — | exact f32 values (v/32768, all exactly representable), pts propagation, take-once, empty-mix → false. Would fail on scale/interleave/pts regressions |
| `volume_halves_the_output` | 524-543 | REAL | — | 0.5 linear exact |
| `channel_swap_routes_the_pair` | 545-564 | REAL | — | swap_stereo routing exact |
| `mono_track_maps_into_a_stereo_pipeline` | 566-603 | REAL | — | mono→stereo duplication; `Stamp::Missing` frame accepted, pts Missing propagates (no assert on pts — minor) |
| `two_tracks_mix_straight` | 605-625 | REAL | F-002 | straight sum exact; pts assert only discriminates under track-0-first ordering |
| `shape_and_geometry_errors_are_typed` | 627-694 | REAL | F-002 | pins `audio.pipeline_shape_invalid` ×2, `audio.rate_mismatch` (48k→44.1k — THE no-rate-conversion pin), `audio.channel_count_mismatch`, `audio.track_index_out_of_range` ×2; set_channel_map only valid-track/bad-map |
| `misaligned_quanta_are_typed_errors` | 696-710 | REAL | — | 2-vs-1 frame staging → `audio.chunk_length_mismatch` |
| `frame_counts_are_conserved_through_the_quantum` | 712-731 | REAL | — | doc 03 §4.8(e) invariant: 1470 in = 1470 out, byte length 1470·2·4 |
| `quantum_output_rotation_is_bounded_and_values_stay_exact` | 733-800 | REAL | — | adversarial: retains 3·OUT_SLOTS+5 quanta, asserts pool caps at OUT_SLOTS (reaches private field — in-crate, fine), every held quantum byte-exact (no stale-sample leakage), phase-2 reuse ≤ OUT_SLOTS. Would fail on pool regression |
| `const TARGET_FILL_NS` | 804 | REAL | — | 40 ms target |
| `drift_pipeline` (helper) | 806-814 | REAL | — | arms the governor — configuration no shipped code uses |
| `quantum` (helper) | 816-824 | REAL | — | deterministic 1470-frame L/R-antisymmetric ramp |
| `zero_correction_is_the_bit_exact_bypass` | 826-853 | REAL | — | bypass identity: plain vs corrected(ZERO) byte-equal ×4 quanta; the LIVE shipped path pinned |
| `nonzero_correction_shifts_the_quantum_by_the_exact_ratio` | 855-892 | REAL | — | 1000 quanta, ±1000 ppm, out = in×(1001|999)/1000 ± 1 — would fail on any ratio error; production-unreachable path |
| `armed_governor_closes_the_loop_through_observe_drift` | 894-935 | REAL | — | loop closure: draining fill → +250 ppm step-limited; stats.updates==1, samples==41 (arithmetic verified vs governor.rs constants); next quantum 1470|1471 |
| `unarmed_pipeline_ignores_drift_observations` | 937-965 | REAL | — | the SHIPPED configuration (unarmed): ZERO forever, default stats; armed+unmeasurable parks ZERO (stomps a manual 600 — governor ownership documented); unmeasurable==90 |
| `reset_drift_restarts_the_measurement_regime` | 967-992 | REAL | — | resets==1, correction→ZERO, resampler phase restart (next quantum exactly 1470), samples monotonic +1 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unity/volume/swap/mono/two_tracks | YES | exact f32 equality on hand-computed values; any scale, routing, or sum regression flips them (e.g. scale 32767 vs 32768 breaks unity) |
| shape_and_geometry_errors_are_typed | YES | asserts exact error-code strings per branch |
| misaligned_quanta | YES | exact code string |
| frame_counts_are_conserved | YES | exact count + byte length |
| quantum_output_rotation | YES | pool-cap + byte-exact retention under adversarial pressure (a stale-slot reuse bug flips the per-quantum equality) |
| zero_correction_is_the_bit_exact_bypass | YES | byte equality across the two configurations; an accidental resampler call at ZERO (copy/interpolate) breaks it |
| nonzero_correction_shifts | YES | ±1-sample tolerance over 1.47 M frames — a ratio off by anything ≥ 1/1470 flips it |
| armed_governor_closes_the_loop | YES | exact ppm (250), updates (1), samples (41), next-quantum 1470/1471 — all derived from governor constants |
| unarmed_pipeline_ignores | YES | ZERO + default-stats equality; an accidental governor effect flips it |
| reset_drift_restarts | YES | exact counters and post-reset conservation |
| Mutation that passes ALL of them: | — | (a) `timing` first-to-stage → lowest-track-index (F-002 — only ordering tested agrees with both); (b) re-stage replace→append (F-001 — never exercised); (c) `pending` overwrite→error (F-001); (d) set_channel_map error-priority swap (part1 F-004 — valid-track path only). All documented above; none mask a production-reachable defect today. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Frame counts are conserved end to end" (drift bypass) | module doc 20-21; test 713-731 | REAL | 1470→1470 exact, baseline green (test-workspace.log.txt:100) |
| "rate ≠ mixer rate is a typed error" | module doc 21-22; test 652-666 | REAL | `audio.rate_mismatch` asserted for 48k-into-44.1k — confirms sample-rate conversion does NOT exist (wave question) |
| "The quantum's pts (lowest track index that staged)" | field doc 75-76 | FALSE (doc) / unpinned (test) | code 305-307 is first-to-stage; test 623-624 cannot discriminate (F-002) |
| doc 03 §4.8(e) "frames in == frames out" under correction | test 855-892 | REAL (code) / UNREACHABLE (production) | ±1000 ppm ratio conserved ±1 sample over 1000 quanta; no shipped composition arms the governor |

## 7. Dead / duplicated / unreachable code
- Tests 826-992 (5 of 14) cover the drift seam that no shipped composition activates (part1 F-001, resample.rs F-002). They are honest tests of real code; the gap is production wiring, not test fraud. No dead tests found; no `#[ignore]` in this module (the ignored throughput tests live in `audio-engine/tests/`, baseline log lines 136-137).
- Helpers are minimal; no duplication beyond the deliberate two-configuration pattern in test 826-853.

## 8. Dependencies & flags
- `testkit::Must` (dev-dep, crate manifest) — panics on Err, so `.must()` lines are real assertions, not swallows.
- `media_clock::DriftCorrection` equality used in asserts — DriftCorrection derives PartialEq (drift.rs).
- No feature flags; module compiles under default `cargo test --workspace` (baseline run #3).

## 9. Open questions for the next agent
- mix.rs auditor (W07-A02, same agent — resolved): does `Mixer::mix` write exactly `frames` samples per plane into `self.mixed` (the resample.rs F-001 reachability precondition)? → RESOLVED: yes — see mix.rs audit §2.
- convert.rs auditor: the S16 fast path at convert.rs:175 (`fmt.sample == SampleFormat::S16 && …`) — confirm it is bit-identical to the generic path (pipeline tests only pin S16 end to end).
