# AUDIT: tools/mem-audit/examples/mjpeg-frame-attribution.rs
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 233 / 1 main + Counting(GlobalAlloc) + NullSink(FrameSink) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — dev-tool example (`cargo run -p lumen-mem-audit --example mjpeg-frame-attribution`, header line 5); release lane builds only lumen-cli + desktop (baseline cargo-build-release-*). Cited as evidence by docs/perf/memory-audit-20261023.md:59,200. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `wc -l` (233); `rg -l "pub struct MjpegDecoder"` -> crates/codecs/src/video/mjpeg/mod.rs; deps verified in tools/mem-audit/Cargo.toml:47-53; output-format cross-check vs docs/perf/memory-audit-20261023.md:205-212. NOT executed — cargo/rustc not installed (environment); execution UNVERIFIED (reason: no toolchain). |

## 1. One-paragraph verdict
A genuine allocation-attribution instrument: real access->probe->AVI-demux path, real `MjpegDecoder`, a counting `GlobalAlloc`, 200-iteration same-packet replay plus a full-pipeline streaming phase, histogrammed via BTreeMap. The code's ATTRIB/STREAM output grammar matches the perf doc that cites it. Three defects: the replay packet hardcodes `TrackId::new(0)`, a `unreachable!` panic path, and the citing doc presents a doctored "output" block (values the program cannot print).

## 2. What actually works (verified)
Nothing executed (no toolchain). Verified structurally: `rg` shows MjpegDecoder real (crates/codecs/src/video/mjpeg/mod.rs); Cargo.toml path-deps for codec-api/codecs/demux/input/media-probe/media-types/plugin-api all present (:47-53); doc cite at docs/perf/memory-audit-20261023.md:59. Baseline rc-smoke.txt proves the mjpeg decode limb itself works (headless-decode AVI/MJPEG/PCM PASS).

## 3. Findings
F-001 [S4]
Where: tools/mem-audit/examples/mjpeg-frame-attribution.rs:164-165
Code:
```
        let replay = Packet {
            track: TrackId::new(0),
```
What is wrong: the replayed packet claims track 0 instead of the harvested `video_track` id found above; the comment says "rebuilt per iteration" but silently discards the real track identity (and forces `discontinuity: false`). Why it matters: the tool bills itself as the ground truth for per-frame attribution; if the decoder ever keyed state off track id the measurement would silently measure a different configuration than the real walk. What correct looks like: `track: video_track`. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: tools/mem-audit/examples/mjpeg-frame-attribution.rs:140
Code:
```
    let descriptor = CodecDescriptor::new("mjpeg").unwrap_or_else(|err| unreachable!("{err}"));
```
What is wrong: unreachable! panic path — a failure of `CodecDescriptor::new` converts into a process panic instead of the exit(65) convention every other error path in this file uses. Why it matters: inconsistent error surface in an instrument whose other failure paths are exit-coded (adds to the tree's 4126 unreachable! per baseline loc-patterns.txt). Confidence: HIGH — Verification: read.
F-003 [S3]
Where: docs/perf/memory-audit-20261023.md:205-212 (cross-ref — evidence block citing THIS file)
Code:
```
STREAM calls=12 bytes=~2.87e6 packets=1 (×~27)   # real pipeline interleaved
```
What is wrong: the program prints exact integers (`println!("STREAM calls={calls} bytes={bytes} packets={count}")`); `~2.87e6` and the editorial `(×~27)` cannot be program output — the doc presents doctored/normalized output as a paste. Why it matters: evidence-provenance rot — the reader cannot tell machine output from human editing, and exact byte counts are the instrument's entire point. What correct looks like: verbatim capture under docs/perf/raw/ (like memaudit-0014) or an explicit "normalized" marker. Confidence: HIGH — Verification: read (format strings compared verbatim).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Counting (GlobalAlloc) | 37-53 | REAL-UNPROVEN | — | forwards to System, counts on success only; SAFETY comments present |
| NullSink | 57-64 | REAL | — | drops frames post-decode; correct for alloc attribution |
| main | 65-233 | REAL-UNPROVEN | S4 | real open/probe/demux/decode chain; Phase A replay (F-001), Phase B streaming |

## 5. Test honesty
No tests — it is itself a measurement instrument; honesty = does the doc's claimed output match the program grammar (see F-003: partially doctored).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "re-feeds ONE real packet through the real MjpegDecoder 200×" | memory-audit-20261023.md:200 | REAL | code: ITERATIONS=200, replay loop |
| output block verbatim | memory-audit-20261023.md:205-212 | FALSE-as-paste | F-003: values not producible by the program |

## 7. Dead / duplicated / unreachable code
Dev-tool example; no shipped consumer. Track-format find via stream_format match is used.

## 8. Dependencies & flags
All 7 path deps verified in tools/mem-audit/Cargo.toml:47-53 (codec-api, codecs, demux, input, media-probe, media-types, plugin-api).

## 9. Open questions for the next agent
Run `cargo run --release -p lumen-mem-audit --example mjpeg-frame-attribution -- tests/fixtures/mjpeg_h3/mjpeg_interlaced_tff.avi` when a toolchain exists to convert REAL-UNPROVEN to REAL.
