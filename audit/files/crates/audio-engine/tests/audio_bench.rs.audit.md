# AUDIT: crates/audio-engine/tests/audio_bench.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-23 |
| Lines / functions | 265 / 7 helpers + 2 tests |
| File verdict | REAL (measurement lane; both profiles EXECUTED by me) |
| Reachable from shipped entry point? | N/A (test binary, not shipped). The measured code IS shipped: `AudioPipeline::process/mix/take_output` + convert.rs + resample.rs, reached from apps/cli pipeline_host.rs:51. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p audio-engine --test audio_bench -- --ignored --nocapture` (debug) → **2 FAILED** (bypass median 18.0 MB/s < 20 floor; corrected 11.3 < 15); `cargo test -p audio-engine --release --test audio_bench -- --ignored --nocapture` → **2 passed**: bypass median **606.6** MB/s, corrected **326.3** MB/s (exit 0). |

## 1. One-paragraph verdict
An honest, well-built measurement harness whose release claims I reproduced almost digit-for-digit: bypass 606.6 MB/s vs the committed W5-19 worklog 606.4, drift-corrected 326.3 vs 329.2 — the archived worklog entry (docs/worklog/archive-2026-10.md:7889-7895) is real and accurate, the fixture is committed and digest-pinned (FNV-1a over the extracted 60×5880-byte 01wb stream, walker asserts count + per-chunk size first), and the methodology is sound (pre-built frames outside the timing loop, one warm-up pass, 15 timed passes, median + interpolated p95, raw per-pass numbers on stderr, machine-parseable BENCH line). Two caveats: the debug "sanity floors" (20/15 MB/s) FAIL on this 2-core sandbox when the ignored tests are run in a debug profile (18.0/11.3 MB/s medians under load variance 5.9-18.9) — the floors are machine-sensitive and the file's doc does not caveat that; and as with every `#[ignore]`d bench in this repo, the release floors are enforced by no automated lane — they are documentation with a manual re-run instruction.

## 2. What actually works (verified)
— `cargo test -p audio-engine --release --test audio_bench -- --ignored --nocapture` → 2 passed; stdout: `BENCH pipeline_bypass: median=606.6 MB/s p95=613.3 MB/s N=15` and `BENCH pipeline_drift_corrected: median=326.3 MB/s p95=329.9 MB/s N=15` — reproduces the committed W5-19 numbers (606.4 / 329.2) on the same class of 2-core box.
— Fixture chain verified: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi exists (425,516 bytes); the walker's 60-chunk/5880-byte asserts + FNV-1a digest pin executed inside both runs (they run before timing).
— The committed-numbers claim traced: docs/worklog/archive-2026-10.md:7892-7893 contains exactly the quoted measurements, including the honest "load 3.5-4.0 — two concurrent ffmpeg corpus-regen processes" provenance note.

## 3. Findings
F-001 [S4]
Where: crates/audio-engine/tests/audio_bench.rs:245, 260
Code:
```rust
    let floor = if cfg!(debug_assertions) { 20.0 } else { 200.0 };
    assert!(
        median >= floor,
        "{median:.1} MB/s < {floor:.1} MB/s floor (N={REPS})"
    );
```
What is wrong: the debug sanity floors are machine-sensitive and fail on this environment — running the ignored tests in a debug profile (not the documented lane, but a plausible developer action) yields medians 18.0 MB/s (bypass, floor 20) and 11.3 MB/s (corrected, floor 15) with heavy load variance (raw passes 5.9-18.9 MB/s): both tests FAIL. The file doc (:26-27 "Debug builds assert only a sanity floor") implies the debug floor is safely below any plausible machine; on a loaded 2-core box it is not. Release lane unaffected (606.6/326.3, floors 200/150).
Confidence: HIGH — Verification: ran both profiles (outputs above).

F-002 [S4]
Where: crates/audio-engine/tests/audio_bench.rs:236-237, 252-253
Code:
```rust
#[test]
#[ignore = "W5-19 throughput measurement: cargo test -p audio-engine --release --ignored (numbers in the W5-19 worklog entry)"]
```
What is wrong: the release floors (200/150 MB/s) are evaluated by no automated lane — the tests are among the baseline's 51 ignored, no CI step in the baseline runs `--release --ignored`, so a pipeline throughput regression (e.g., an accidental per-quantum allocation) would surface only if a human re-runs the bench. Same enforcement-free pattern the pcm_bench audit found for the codecs side.
Confidence: HIGH — Verification: read + baseline cargo-test-workspace.txt (51 ignored; these tests not run).

F-003 [S4]
Where: crates/audio-engine/tests/audio_bench.rs:3-4
Code:
```rust
//! `cargo test -p audio-engine --release --ignored`; the committed
//! numbers live in the W5-19 worklog entry.
```
What is wrong: "the W5-19 worklog entry" is in the ARCHIVED worklog (docs/worklog/archive-2026-10.md:7882-7901), not the top-level worklog.md — rg for "W5-19"/"MB/s" in worklog.md returns nothing. Substantively true (numbers present and reproduced), but the pointer sends a reader to the wrong file first. Discoverability nit.
Confidence: HIGH — Verification: rg on both files + read of the archive entry.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `u32le` | 54-57 | REAL | — | bounds-checked LE read |
| `extract_pcm_chunks` | 62-99 | REAL | — | RIFF/movi walker; top-level non-advancing guard present; inner loop always advances (body = cursor+8); payload reads via `data.get` — no panic path found |
| `fixture_chunks` | 102-128 | REAL | — | count/size/digest triple pin before any timing; unreachable! on missing fixture (committed, exists) |
| `quantum_frame` | 132-148 | REAL | — | s16le stereo @44.1k contract frame, pts math exact |
| `bench` | 152-210 | REAL | — | frames pre-built outside timing; warm-up pass; 15 timed passes; raw per-pass on stderr |
| `stats` | 214-224 | REAL | — | linear-interpolation median/p95, NaN-tolerant comparator |
| `report` | 226-234 | REAL | — | machine-parseable BENCH line |
| `pipeline_throughput_bypass` | 238-250 | REAL | S4 | executed: debug FAIL 18.0<20, release PASS 606.6≥200 |
| `pipeline_throughput_drift_corrected` | 254-265 | REAL | S4 | executed: debug FAIL 11.3<15, release PASS 326.3≥150 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pipeline_throughput_bypass | YES (release) | floor 200 MB/s is ~1/3 of current 606 — a real regression (alloc-per-quantum, resampler re-parse) would blow through it; verified failing/passing states by running both profiles |
| pipeline_throughput_drift_corrected | YES (release) | same, floor 150 vs current 326 |
| (both, debug profile) | UNSTABLE | floors fail on this loaded 2-core sandbox (F-001) — the debug assertion is environment-noise, not a stable invariant |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "run explicitly with `cargo test -p audio-engine --release --ignored`" | :2-3 | REAL | executed exactly that; 2 passed |
| "the committed numbers live in the W5-19 worklog entry" | :3-4 | REAL (mis-filed) | entry is docs/worklog/archive-2026-10.md:7882-7901; numbers reproduced (606.6/326.3 vs 606.4/329.2) |
| "the walker below is pinned by chunk count, per-chunk size and an FNV-1a digest" | :8-9 | REAL | asserts at :114-126 executed inside my runs |
| "the same bytes are digest-pinned end-to-end by the codecs pcm_fixtures suite" | :10-11 | REAL-UNPROVEN | pcm_fixtures suite exists (prior agent audited it REAL); I did not re-run it here |
| "release expects ≥ 200 MB/s ... Debug: sanity only" | :243-245, 258-260 | REAL / SHALLOW | release floor holds (606.6); the debug "sanity" floor fails on this box (F-001) |

## 7. Dead / duplicated / unreachable code
None. (The AVI-walk helper duplicates codecs' `common` module pattern — documented at :59-61 as deliberate test-side discipline.)

## 8. Dependencies & flags
audio-engine + media-types + media-clock (DriftCorrection) only; std timing. No unsafe, no features. `#[ignore]` with accurate reason strings.

## 9. Open questions for the next agent
- resample.rs / pipeline.rs auditors: the drift probe replays the SAME 60 frames 16× through one pipeline instance — the resampler's cross-quantum state carries across replays (fine for throughput, but it means the bench never exercises a fresh-state corrected stream; correctness coverage must come from resample.rs's own suite).
- If CI ever gains a perf lane, the debug floors need either removal or a machine-class caveat (F-001).
