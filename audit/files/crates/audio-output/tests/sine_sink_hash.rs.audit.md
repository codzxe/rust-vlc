# AUDIT: crates/audio-output/tests/sine_sink_hash.rs
| Field | Value |
|---|---|
| Auditor | W08-A03 |
| Date | 2026-11-23 |
| Lines / functions | 324 / 3 tests + 1 driver fn + 3 inner fns (support module `tests/common/mod.rs` examined: 263 lines) |
| File verdict | REAL (executed by me; all three anchors independently verified, including a python3 cross-check the suite itself does not perform) |
| Reachable from shipped entry point? | N/A — test lane; runs in workspace `cargo test` (baseline line ~3964 zone: audio-output suite) and I re-ran it. The code under test (audio-output `NullBackend`/`NullSink`/`NullDevice`) IS reachable: `apps/cli/Cargo.toml` depends on audio-output |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p audio-output --test sine_sink_hash` → 3 passed, exit 0; python3 AVI `01wb` extraction → `sha256: 1d9f5019c9b5181f0910528dce360f46e74f9c4df8d9bc9a20c2c68fd218b95b` == both TOMLs' `audio_stream_sha256`, 352800 bytes == `audio_stream_bytes`; `sed -n` quote verification |

## 1. One-paragraph verdict
This is the rare test suite that actually proves what its name claims. The hostile question — "does a hash of rendered sine prove the pipeline computes samples?" — answers YES here, for three verified reasons: (1) the oracle is INDEPENDENT — `s16_stream_to_f32_bytes` in `tests/common/mod.rs:255-262` is a plain std loop, not the engine's converter, and the capture is compared to it BYTE FOR BYTE, so any corruption in engine s16→f32/mix/interleave or ring transport fails the test; (2) the input is anchored to the committed binary corpus — I independently extracted the AVI's `01wb` PCM with python3 and its SHA-256 equals both parents' recorded `audio_stream_sha256` (1d9f5019…), closing the loop the test itself only half-closes (it pins regenerated-NCO == TOML; I pinned TOML == actual fixture bytes); (3) the sensitivity run (half volume must change the hash) kills the "hash of a constant" degenerate case. Scope is honestly stated: it proves the engine→sink-buffer vertical through the NULL backend — no real OS audio device, no cpal path, is exercised.

## 2. What actually works (verified)
— verified by `cargo test -p audio-output --test sine_sink_hash` → `test result: ok. 3 passed; 0 failed` (exit 0), including the wrap/refusal/pacing test that asserts content invariance across ring geometry and `refusals == 2` per tick.
— verified by python3 extraction of `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` audio stream → 352800 bytes, SHA-256 `1d9f5019c9b5181f0910528dce360f46e74f9c4df8d9bc9a20c2c68fd218b95b`, matching `audio_stream_sha256` in BOTH `avi_mjpeg_pcm.avi.toml:108` and `mp4_mjpeg_pcm.mp4.toml:108`. The test's sine IS the corpus's sine — proven at the binary level, not by trusting the TOML.

## 3. Findings
F-001 [S4]
Where: crates/audio-output/tests/sine_sink_hash.rs:212-216
Code:
```
    let attenuated = run_tone_through_output(Volume::from_linear(0.5).must());
    assert_ne!(
        attenuated.hash, first.hash,
        "the hash must track the samples"
    );
```
What is wrong: the sensitivity check proves the bytes CHANGE at half volume, not that they equal 0.5×-scaled samples — a volume implementation that scrambled samples would still pass this arm (it would be caught by (2) only at unity). What correct looks like: also assert the attenuated capture equals an independently computed `0.5 * v/32768` stream (rounding mode pinned). Confidence: HIGH — Verification: read. Severity stays S4 because the unity path carries the real oracle.

F-002 [S4]
Where: crates/audio-output/tests/sine_sink_hash.rs:206-217
Code:
```
    assert_eq!(second.sample_count, TONE_TOTAL_SAMPLES as u64);
    assert_eq!(second.underruns, 0);
```
What is wrong: the attenuated third run asserts `sample_count` but not `underruns == 0` — inconsistent with the unity runs; a volume-dependent pacing bug in the attenuated run would pass unnoticed. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/audio-output/tests/common/mod.rs:163-173 (and tone constants 224-232)
Code:
```
const SIN_QUARTER: [u16; 65] = [
    0, 804, 1608, 2410, 3212, 4011, 4808, 5602, //
```
What is wrong: the quarter-wave table, oscillator, and tone plan are a manual transcription of `tools/xtask/src/cmd/fixtures.rs` — duplicated constants that can drift. Mitigated (and why this is S4, not S3): the digest pin means drift breaks the suite against the COMMITTED corpus, and my python3 check proves the committed corpus matches the pin today. Confidence: HIGH — Verification: read + python3 digest cross-check.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run_tone_through_output | 63-130 | REAL (executed) | — | full pipeline→sink→render per tick; asserts frames==1470, fully_served, silence_filled==0 |
| specd_sine_is_the_committed_corpus_tone | 144-165 | REAL (executed) | — | pins NCO PCM to both parents' recorded digests; I closed the loop to the binary fixture |
| sine_fixture_sink_buffer_hash_is_byte_stable_across_runs | 168-218 | REAL (executed) | S4 | value truth (byte-exact vs independent oracle), double-run stability, clock truth, sensitivity (F-001/F-002) |
| sink_buffer_hash_survives_ring_wrap_refusal_and_pacing | 221-324 | REAL (executed) | — | 2048-ring, typed `audio.ring_full` retry, 735-frame drains, `refusals==2`, content invariance asserted |
| (inner) stage / render / write_with_backpressure | 251-295 | REAL (executed) | — | backpressure loop bounded by ring arithmetic (2048−1470=578<1470 → exactly 2 drains; verified arithmetic) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| specd_sine_is_the_committed_corpus_tone | Yes | SHA-256 equality against two committed digests; any change to PCM length/content fails |
| sine_fixture_sink_buffer_hash_is_byte_stable_across_runs | Yes | byte-equality vs an oracle computed by code NOT under test (common/mod.rs loop); constant-returning pipeline, broken conversion, ring corruption, clock miscount, or underrun all fail distinct asserts |
| sink_buffer_hash_survives_ring_wrap_refusal_and_pacing | Yes | geometry-sensitive (refusals==2 exact) AND content-equality vs the same independent oracle; a ring that dropped/duplicated frames on wrap fails the capture compare |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "deterministic PCM ... through engine→output sink buffers, hashed, byte-stable (double-run proof)" (card 2.C3 AC) | sine_sink_hash.rs:1-4 | REAL | my run: 3 passed; byte-compare + double-run asserts |
| "the captured bytes must equal the pinned f32 conversion of the PCM, byte for byte" | sine_sink_hash.rs:27-32 | REAL | assert at 183-186 vs common/mod.rs:255-262 (independent implementation) |
| "the test's sine IS the corpus's sine" (both parents) | sine_sink_hash.rs:24-26 | REAL | python3: AVI PCM sha256 == 1d9f5019… == both TOML digests |
| "Clock truth ... exactly the 88 200 sample-instants" | sine_sink_hash.rs:39-42 | REAL | asserts at 202-204 (sample_count, clock_count, clock_rate) |
| Scope note: this proves the sink-buffer vertical only | module doc 6-19 | Honest | NullBackend only; no OS device, no cpal — see §7 |

## 7. Dead / duplicated / unreachable code
None in this file. Cross-file note: the NCO constants are duplicated from `tools/xtask/src/cmd/fixtures.rs` (F-003, digest-pinned). The suite does NOT cover `audio-output/src/cpal.rs` (the real-device backend) — any feature-matrix row citing this suite as evidence of real audio OUTPUT on a platform is overclaiming; it evidences the engine→sink-buffer path only.

## 8. Dependencies & flags
audio-output (path dep of lumen-cli → reachable), audio-engine, media-types, testkit (dev-only). No feature gates on this suite; runs in the default test lane (baseline cargo-test-workspace.txt: audio-output compiled and suite ran; I re-ran it: 3 passed).

## 9. Open questions for the next agent
- Who tests `crates/audio-output/src/cpal.rs` (the real backend)? If nothing does, the only executed audio-output device path is the null one — check the matrix rows for "audio playback" evidence strings.
- `zero_alloc_null_sink.rs` and `callback_budget.rs` (other suites in this crate) share `common/` — the corpus anchor there inherits the same strength.
