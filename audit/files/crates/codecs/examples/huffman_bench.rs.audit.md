# AUDIT: crates/codecs/examples/huffman_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 324 / 6 (main + split_frames + configured + loadavg + NullSink + FrameSpan) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — a developer instrument (cargo example), not shipped in lumen-cli/desktop. The decoder it measures IS the shipped contract path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `ffmpeg -f lavfi -i testsrc2=size=320x240:rate=30 -t 1 -c:v mjpeg -q:v 5 -f mjpeg /tmp/w06a03/bench.mjpeg` (281,953 bytes) then `./target/debug/examples/huffman_bench-d8c8e5ba44a9c849 /tmp/w06a03/bench.mjpeg --passes 5` → exit 0: `30 frames (splitter walk)`, 5 passes, `med_ms=170.3 p95_ms=199.4 min_ms=148.5 med_fps=176.2 med_mbs=1.7 load_before=0.40 load_after=0.40`, sink accounting consistent (90/120/150/180/210 = warm-up 2×30 + pass×30). |

## 1. One-paragraph verdict
A real, working decode-core benchmark: it splits a raw MJPEG stream with a careful marker walk, decodes every frame through the actual codec-api contract path (MjpegDecoder::decode → sink), warms up, measures N passes, and prints median/p95/min/fps/MB/s plus /proc/loadavg samples. I ran it end-to-end on a fresh ffmpeg-authored stream — the splitter found all 30 frames, every frame decoded (exit 70 on any refusal), and the accounting is internally consistent. Two flaws: the file name promises a Huffman-stage benchmark but the body measures the whole decode pipeline (stage attribution lives in examples/stage_bench.rs), and the machine-parseable summary line omits the build profile, so a debug-build run (like mine, 176 fps) is indistinguishable in the output from a release run — a real hazard for an instrument whose stated purpose is honest numbers.

## 2. What actually works (verified)
- Full run on generated input: splitter 30/30 frames; decode-to-null through the contract path; exit 0; summary line above.
- Exit-code contract implemented: 64 usage (`--passes` validation, unknown flags), 66 unreadable/no-frames, 70 decode failure (none hit — all frames decodable).
- Warm-up excluded from measurement; per-pass wall on stderr; sink frame counts printed for cross-checking (verified arithmetic above).

## 3. Findings
F-001 [S3]
Where: crates/codecs/examples/huffman_bench.rs:313-321
Code:
```rust
    println!(
        "HUFFMANBENCH file={file} frames={} passes={passes} med_ms={median:.1} p95_ms={p95:.1} \
         min_ms={min:.1} med_fps={fps:.1} med_mbs={mbs:.1} input_mb={:.1} \
         load_before={} load_after={}",
```
What is wrong: the summary line (the machine-parseable artifact the perf docs consume) records load average but NOT the build profile (debug vs release) or any build identity. My debug-build run printed `med_fps=176.2`; a release run of the same input would be several times faster — the two outputs are indistinguishable. For a self-described "honest-numbers instrument", any number it emits must be attributable to a profile; `cfg!(debug_assertions)` (or better, an opt-level probe) belongs on the line. Confidence: HIGH — Verification: ran (debug binary produced a full summary line with no profile field).
F-002 [S4]
Where: crates/codecs/examples/huffman_bench.rs:1-2 (vs the file name)
Code:
```rust
//! MJPEG decode-core benchmark (card W6-02) — the honest-numbers
//! instrument for the heavy-mjpeg 0.16x-vs-ffmpeg target.
```
What is wrong: the name `huffman_bench` says Huffman; the body benchmarks the entire decode pipeline (marker walk + entropy + IDCT + assembly + pooled handoff) — its own first line says "decode-core benchmark". The Huffman/stage-attribution instrument is a different example (`stage_bench`, which drives the ScanHooks seam). A reader grabbing "huffman_bench" for Huffman-stage numbers gets whole-decode numbers. Confidence: HIGH — Verification: read + ran (measured whole-path fps, no stage isolation).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NullSink (impl FrameSink) | 49-58 | REAL | — | counts and drops (decode-to-null, documented) |
| FrameSpan / split_frames | 60-152 | REAL | — | SOI resync, length walk, SOS entropy scan, stuffed-FF/RST/fill handling; found 30/30 on real input; defensively resyncs past broken SOIs |
| loadavg | 155-159 | REAL | — | /proc/loadavg 1-min sample |
| configured | 162-180 | REAL | — | real contract configure (descriptor "mjpeg") |
| main | 182-323 | REAL | S3/S4 | arg parsing + warm-up + measured passes + summary; F-001 (no profile on the line); median/p95/min math correct |

## 5. Test honesty
Not a test file — an instrument. Its self-honesty markers verified: decode failures fail the run (exit 70), sink counts printed for cross-check, load sampled both ends, allocation counting explicitly disclaimed as belonging to the counting-allocator suites (L29-33, honest scope statement). The header's "0.16x-vs-ffmpeg" framing is an unusually honest admission that the decoder is currently ~6× slower than ffmpeg on the heavy lane.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decodes every frame through the real codec-api contract path" | header L10-12 | REAL | my run decoded 30 frames/pass through MjpegDecoder::decode |
| "per-pass wall time on stderr, one machine-parseable summary line on stdout (median / p95 / min, fps, MB/s)" | header L16-18 | REAL | output pasted above |
| "/proc/loadavg is sampled before and after" | header L23-25 | REAL | load_before/load_after on the line |
| "Exit codes: 0 ok, 64 usage, 66 unreadable input, 70 decode failure" | header L27-29 | REAL (0, 66-path via no-frames guard; 64/70 read-verified) | ran exit 0; usage validation read |
| implied: numbers are attributable to a build | F-001 | SHALLOW | no profile on the summary line |

## 7. Dead / duplicated / unreachable code
None. (`--passes` minimum 3 enforces a meaningful median.)

## 8. Dependencies & flags
Workspace crates only (codec-api, codecs, media-types). No features. Built as a cargo example (present in target/debug/examples — someone built the example set).

## 9. Open questions for the next agent
- Where do the recorded huffman_bench numbers live (docs/perf/raw/mjpeg/)? Do the committed summaries carry the profile? If a debug-profile number was ever recorded as a release number, that is an S0-adjacent perf-claim fabrication — check the perf docs lane.
- examples/stage_bench.rs (the actual stage-attribution instrument behind the ScanHooks seam) is not in my assignment — it deserves its own audit since decoder.rs perf claims lean on it.
