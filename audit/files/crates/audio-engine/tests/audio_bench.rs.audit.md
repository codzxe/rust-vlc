# AUDIT: crates/audio-engine/tests/audio_bench.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 266 / 2 `#[test]` fns (both `#[ignore]`d) + 6 helper fns |
| File verdict | REAL-UNPROVEN (a measurement harness, honestly `#[ignore]`d, with committed numbers verified in the worklog) |
| Reachable from shipped entry point? | N/A (test/bench file). The path it measures is real (AudioPipeline::process/mix/take_output) but the drift-corrected probe exercises the resampler path that **production never arms** (my resample.rs audit F-001). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | full-file read; rg for the W5-19 worklog entry (docs/worklog/archive-2026-10.md:7881+, numbers found at lines ~7889-7895); percentile-math hand-check |

## 1. One-paragraph verdict

A std-only throughput harness over the committed class-A AVI fixture (60 × 1470-frame stereo s16le chunks, FNV-1a digest-pinned so fixture or walker drift is caught), two probes: the zero-correction pump path (bypass) and the +600 ppm drift-corrected path. Both are `#[ignore]`d with the run instruction in the ignore message — deliberate measure-first discipline (D-10), not test rot. The floors are honest sanity bounds (release ≥ 200/150 MB/s), debug floors explicitly "sanity only", raw per-pass samples printed, median + p95 via linear interpolation (hand-checked: N=15, q=0.95 → pos 13.3, indices in range). The committed numbers exist and match the claims: the W5-19 worklog entry records "pipeline bypass … 458.0 → 606.4 MB/s (1.34×, ~3,440× realtime)" and "pipeline drift-corrected (+600 ppm): 229.6 → 329.2 MB/s (1.43×)" on a 2-core box with the concurrent-load caveat noted. The one caveat that matters for the claim ledger: the drift-corrected probe is the **only** throughput measurement of the resampler — the same path no shipped binary ever arms.

## 2. What actually works (verified)

- The fixture pin: chunk count (114-115), per-chunk size, and the FNV-1a digest over the extracted `01wb` stream (116-126) — a fixture change cannot silently alter the benchmark's input.
- The W5-19 worklog entry (docs/worklog/archive-2026-10.md:7881+) contains the claimed committed numbers, including the honest load condition ("two concurrent ffmpeg corpus-regen processes, noted honestly") and raw-sample locations.
- `stats` interpolation arithmetic verified by hand for N=15 and the degenerate N=1.

## 3. Findings

### F-001 [S4] — The drift-corrected probe measures a path production never arms; both probes are ignored by default, so no CI run can catch a throughput regression

Where: crates/audio-engine/tests/audio_bench.rs:252-265 (`#[ignore]` at 253; `set_drift` at 255).

Code (253-255, verbatim):
```rust
#[ignore = "W5-19 throughput measurement: cargo test -p audio-engine --release --ignored (numbers in the W5-19 worklog entry)"]
fn pipeline_throughput_drift_corrected() {
    let samples = bench("pipeline-drift-corrected", DRIFT_PPM);
```

What is wrong: nothing about the bench itself — the `#[ignore]` is deliberate and documented. The structural note: this probe (via `pipeline.set_drift`, the direct-correction API at pipeline.rs:159-164) is the only throughput exerciser of the micro-resampler's hot path, which in production stays at ppm=0 forever because the governor is never armed (my resample.rs audit F-001). Any PERF-VS-VLC claim citing "drift-corrected 329 MB/s" describes a library configuration no shipped binary reaches, and since the probe is ignored, a regression in that dormant path would surface only if someone re-runs the bench by hand.

Confidence: HIGH — Verification: read + worklog numbers + resample.rs audit cross-reference.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (protocol, fixture provenance) | 1-27 | REAL-UNPROVEN | — | honest about `#[ignore]` and debug-vs-release |
| consts (AVI path, RATE, CHUNKS, REPS, DRIFT_PPM, digest) | 37-52 | REAL-UNPROVEN | — | +600 ppm inside the ±1000 bound |
| `u32le` | 54-57 | REAL-UNPROVEN | — | bounds-checked |
| `extract_pcm_chunks` | 62-99 | REAL-UNPROVEN | — | RIFF walker, non-advancing guard (78-80), digest-pinned |
| `fixture_chunks` | 102-128 | REAL-UNPROVEN | — | count/size/digest asserts |
| `quantum_frame` | 132-148 | REAL-UNPROVEN | — | s16le interleaved contract shape |
| `bench` | 152-210 | REAL-UNPROVEN | — | warm-up pass + N=15 timed passes; raw samples on stderr |
| `stats` | 214-224 | REAL-UNPROVEN | — | median + p95 interpolation, hand-verified |
| `report` | 226-234 | REAL-UNPROVEN | — | one machine-parseable stdout line |
| `pipeline_throughput_bypass` | 236-250 | REAL-UNPROVEN (ignored) | — | floor 200 (release) / 20 (debug) MB/s |
| `pipeline_throughput_drift_corrected` | 252-265 | REAL-UNPROVEN (ignored) | S4 | F-001; floor 150/15 MB/s |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `pipeline_throughput_bypass` | Only when explicitly run (`--ignored`) | sanity floor, not a correctness test — by design |
| `pipeline_throughput_drift_corrected` | Only when explicitly run | same |

These are measurements, not proofs; the file never claims otherwise. The ignored-by-default status means the W00 baseline's 1751-pass count does not include them.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the committed numbers live in the W5-19 worklog entry" | audio_bench.rs:3-4 | REAL | docs/worklog/archive-2026-10.md:7881+ contains both medians and the p95/raw-sample note |
| "the same bytes are digest-pinned end-to-end by the codecs pcm_fixtures suite against the writer's recorded sha256" | audio_bench.rs:9-11 | UNVERIFIED | codecs suite is another agent's territory; the local FNV pin is verified |
| "Debug builds assert only a sanity floor — the measurements are release-profile claims" | audio_bench.rs:26-27 | REAL | cfg!(debug_assertions) floors at 245/260 |

## 7. Dead / duplicated / unreachable code

None (test file). The `#[ignore]` tests are the only "dead" weight in default runs, by design.

## 8. Dependencies & flags

dev-deps: `audio_engine`, `media_types`, plus direct use of `media_clock::DriftCorrection`. `--release --ignored` is the documented invocation; no feature gates.

## 9. Open questions for the next agent

- PERF-VS-VLC.md — does it cite the drift-corrected number without noting the path is dormant in production? (Wave 19 doc audit; cross-reference my resample.rs F-001.)
- The pre-W5-19 "BEFORE" numbers were measured via git checkout of old sources — the methodology note is in the worklog; the perf-methodology audit (wave 19) should evaluate it.
