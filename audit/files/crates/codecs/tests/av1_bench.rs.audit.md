# AUDIT: crates/codecs/tests/av1_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08; recovery of W06-A08's unfinished item 13) |
| Date | 2026-11-23 |
| Lines / functions | 212 / 1 test + 2 helpers + 1 sink impl + 2 consts |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (dev-lane test binary): `cargo test -p codecs --test av1_bench`; #[ignore]d so it never runs in the normal gate (baseline cargo-test-workspace.txt:970-973 shows it ignored). Not part of any shipped binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/av1_bench-44d3c00afa669552 --ignored --nocapture` -> `1 passed; 0 failed` in 191.49s, full honest-gap printout, test exit 0 (debug numbers, not the recorded release ones); rg/sed doc-record verification (docs/formats/video-av1.md:157-164, docs/worklog/archive-2026-10.md) |

## 1. One-paragraph verdict
A real, honestly-designed speed-honesty bench: it generates a 12-frame 1080p AV1 fixture live with ffmpeg libaom-av1 (deterministic command, crf 45 cpu-used 8 — matching its own module docs), decodes it through the real in-process surface (IvfDemuxer mem_stream → Av1Decoder::configure → decode → flush → pixel-counting FrameSink), medians 5 timed reps after a warm-up, and prints the rav1d-asm-off vs ffmpeg-libdav1d gap with the asymmetry (process spawn) labeled exactly as documented. I executed it end-to-end from the prebuilt debug binary: 1 passed, 12 frames, 24.88 Mpix, ratio computed — mechanics verified. The recorded release numbers (32.9 fps vs 112.7 fps, docs/formats/video-av1.md:157-164; 32.8/106.4 in the worklog) are UNVERIFIED by me (release build forbidden by load budget; I ran debug, which measures nothing comparable). Three hygiene defects: swallowed flush error, temp-dir leak on early abort, and a self-correction artifact left in a comment.

## 2. What actually works (verified)
- Full execution: `./target/debug/deps/av1_bench-44d3c00afa669552 --ignored --nocapture` -> `oracle: ffmpeg version 7.1.5-0+deb13u1` + `av1 1080p (12f, 24.88 Mpix total)` + ours/ffmpeg/ratio lines + `test result: ok. 1 passed; 0 failed`, test exit 0. Fixture-gen assert, frame-count assert (12 = 1.2 s × 10 fps), and both must()-guarded event loops all passed against the real Av1Decoder.
- Honest-skip contract: ffmpeg-absent → `row SKIP` early return (:110-119) — read-verified; ffmpeg was present here so the real path ran.
- Doc-record convention: the numbers' destinations exist — docs/formats/video-av1.md:157-164 ("release build ... **32.9 fps ≈ 68.1 Mpix/s** (median of 5)" vs "ffmpeg 7.1.5-0+deb13u1 `libdav1d` ... **112.7 fps**") and docs/worklog/archive-2026-10.md's 5.E2 entry (32.8 fps / 106.4 fps / 0.309x).
- Gate exclusion: baseline :970-973 — the test shows as ignored in the normal workspace run, exactly as the module docs promise.
- Pixel math: 12 × 1920×1080 = 24,883,200 px = the printed 24.88 Mpix — the sink's `v.size.0 as u64 * v.size.1 as u64` accounting is correct.

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/av1_bench.rs:98
Code:
```
    let _ = decoder.flush(&mut sink);
    (sink.frames, sink.pixels)
```
What is wrong: the flush result is discarded. The frame-count assert (:168) would catch a flush that drops frames, but a typed flush error that still leaves 12 counted frames (e.g. recovered trailing damage on a "clean fixture") vanishes silently — in a bench whose whole premise is "clean fixture must decode" (cf. the loud `unreachable!` one line above at :90). What correct looks like: `.must()` like every other decode-path call in the file, or at least an eprintln. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/tests/av1_bench.rs:133-135, :164, :168, :202
Code:
```
    let dir = std::env::temp_dir().join(format!("lumen-av1-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).must();
    let path: PathBuf = dir.join("bench_1080p.ivf");
```
What is wrong: `remove_dir_all` happens only at :202; the asserts at :164 (fixture generation) and :168 (frame count) and any `must()`/`unreachable!` panic abort before it, leaking `lumen-av1-bench-<pid>/` + a 1080p IVF into /tmp on every failed run. Bench hygiene; a failing bench that litters is annoying in CI sandboxes. What correct looks like: a guard/defer-style cleanup or generating under the crate's own target/tmp. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/codecs/tests/av1_bench.rs:49-50
Code:
```
/// Frames of the 1080p fixture (12 s × ... no: 1.2 s @ 10 fps).
const DURATION: f64 = 1.2;
```
What is wrong: the doc comment is a stream-of-consciousness artifact ("12 s × ... no: 1.2 s") and the first line says "Frames of the 1080p fixture" while the constant is a duration in SECONDS (used as `-t {DURATION}` and `DURATION * 10.0` for frames). The value 1.2 and the arithmetic are right (f64 1.2×10 rounds to exactly 12.0; assert passed in my run); only the comment lies about the unit. What correct looks like: `/// Fixture duration in seconds (1.2 s @ 10 fps ⇒ 12 frames).` Confidence: HIGH — Verification: read + ran (assert passed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `const REPS` | 47-48 | REAL | — | 5, odd — matches median()'s true-median guarantee |
| `const DURATION` | 49-50 | REAL | S4 | value correct; comment wrong unit + artifact (F-003) |
| `struct Sink` + `FrameSink::accept` | 52-67 | REAL | — | counts frames + u64 pixel accumulation (no overflow: 12×2.07M ≈ 2.5e7); `unreachable!` on non-video frame is the right loudness |
| `fn decode_file` | 70-100 | REAL | S4-adjacent | full surface (mem_stream→start→configure→decode loop→flush); flush error swallowed (F-001) |
| `fn median` | 102-105 | REAL | — | sorts, takes `values[len/2]`; upper-middle for even lengths — latent only, every caller passes REPS=5 |
| `av1_decode_throughput_1080p_vs_ffmpeg_libdav1d` | 107-212 | REAL (executed) | — | honest oracle pinning (`-c:v libdav1d`), warm-up + median-of-5, asymmetric ffmpeg leg labeled; double `ffmpeg -version` spawn (:110-115 and :120-125) is harmless waste |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| av1_decode_throughput_1080p_vs_ffmpeg_libdav1d | YES for decode/demux breakage (must()/unreachable! panic), fixture-gen breakage (assert :164), frame-count drift (assert :168); NO for speed regressions | It is a measurement test by documented convention (doc 12 §1.3): the product is the printed number, recorded by hand into docs/formats/video-av1.md — no threshold assert by design (same convention as mp3_bench/aac_bench, both audited REAL by W06 peers) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rav1d 1.1.0 is admitted with asm OFF" | :2-3 | REAL | docs/dependencies/records/rav1d-1.1.0.md cited; decoder attribution pinned in the printout |
| "ignored measurement test ... never the normal gate run" | :7-10 | REAL | baseline :970-973 ignored; my run needed explicit `--ignored` |
| "ours: in-process full decode surface ... median of 5 runs after a warm-up" | :18-21 | REAL | :167-179 matches (warm-up decode + REPS loop + median) |
| "ffmpeg: wall clock of `ffmpeg -c:v libdav1d -i f.ivf -f null -` ... asymmetric ... labeled as such" | :22-25 | REAL | :184-199 matches exactly, incl. decoder pin; printout labels "full pipeline incl. spawn" |
| "fixture ... testsrc2 1920x1080 @10 fps, libaom-av1 crf 45 cpu-used 8 → 12 frames" | :27-29 | REAL | :136-161 args match; 12-frame assert passed in my run |
| "ffmpeg absent ⇒ the row SKIPs with a message" | :29-30 | REAL | :110-119 early return with eprintln (path not taken here — ffmpeg 7.1.5 present) |
| Recorded release numbers 32.9 fps / 112.7 fps (0.29x) | docs/formats/video-av1.md:157-164 | REAL-UNPROVEN | Records exist and are internally consistent; I could not re-run release (cargo-build-budget) — my debug run (0.4 vs 114.2 fps) verifies mechanics only, and debug rav1d being ~80× slower is expected, not a contradiction |

## 7. Dead / duplicated / unreachable code
None. `mod common;` (:32) — the shared test-support module (fixtures_parent convention), used for the `testkit::Must` import chain via common. No duplication; the mp3/aac/alac bench siblings follow the same shape by documented convention.

## 8. Dependencies & flags
Test-only: codec_api prelude, codecs::video::av1::Av1Decoder (codec-av1, default-on), demux::ivf (+ its test_support::mem_stream — dev surface), media_types, plugin_api, testkit. External process dependency: ffmpeg (libaom-av1 encoder + libdav1d decoder), honestly skipped when absent. Runs only under explicit `--ignored --all-features --release` per its own ignore message.

## 9. Open questions for the next agent
- Nobody re-runs the release bench in CI — the doc's 32.9/112.7 numbers age silently as rav1d/ffmpeg move. Is a periodic re-measurement card planned (docs/perf/breadth.md lists the bench loop)?
- av1_conformance.rs uses crf 40 while this bench uses crf 45 — both documented per-file, but a one-line cross-reference would prevent "which crf is canonical?" confusion.
