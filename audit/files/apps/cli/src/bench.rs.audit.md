# AUDIT: apps/cli/src/bench.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 586 / 12 production items + 3 tests |
| File verdict | SHALLOW (works as built for mjpeg+pcm; its capability message is FALSE and its decoder set is 2 of the build's ~14) |
| Reachable from shipped entry point? | YES — main.rs:19 `run::run` → run.rs:144 `Some(CliCommand::DecodeBench { source }) => bench::run_bench(&source, out, err)` → bench.rs:213 `run_bench` → :258 `bench` → the REAL `demux_host::open_source` (:270). |
| Findings | S0: 1 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `sed -n` quote verification (all quotes verbatim); W00 baseline outputs quoted from audit/runs/baseline/{mp4,mjpeg}-smoke-bench.txt; `rg` dispatch-site verification. NO cargo (adaptation). |

## 1. One-paragraph verdict

Answer to the hostile question: `decode-bench` measures **open + demux + decode-to-null for exactly two codecs — MJPEG and PCM** — and refuses every other codec with the message `"no decoder in this build (first vertical: mjpeg + pcm)"` (bench.rs:334). That message is FALSE: the same crate's player path (`pipeline_host.rs:241-380`) dispatches mjpeg, pcm, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm, av1, h264/avc1, vp9 and dnxhd — the W00 baseline proves the refusal fires for codecs that DO have decoders (aac, h264) in the shipped binary. The bench carries its own private decoder allowlist instead of the provider, so the two registries drifted: the player grew 12+ codecs; the bench stayed at 2 and now lies about the build whenever it skips. The loop machinery itself is honest (typed errors, concealment budget, exact BENCH-line contract, real open path), and the three in-file tests are real — but none ever ran in the W00 baseline, and any PERF-VS-VLC number produced by this runner covers only the first vertical while presenting as corpus-wide speed parity.

## 2. What actually works (verified)

- The refusal path the orchestrator asked for, found and quoted: bench.rs:328-341 (below). Reachable from `lumen decode-bench <file>` (run.rs:144).
- The open half is the REAL player path: `demux_host::open_source(&spec, None, false, false)` (bench.rs:270) — same canonicalize → access → probe → registry chain as `lumen open`.
- W00 baseline proves the tool runs end-to-end and its BENCH line is produced: `BENCH file=…sample-mjpeg-pcm.mov bytes=648314 v_frames=75 v_decoded=75 a_frames=130 a_decoded=130 wall_s=0.041 decode_s=0.038` (audit/runs/baseline/mjpeg-smoke-bench.txt) — the mjpeg+pcm lane measures real decode throughput.
- The BENCH-line contract is exact and pinned by a test (bench.rs:482-501).
- Failure discipline is typed throughout (probe refusal for garbage — pinned by `garbage_file_is_a_typed_refusal`, exit 65; demux fatals carry partial counts as details, bench.rs:381-395; concealment budget 1000, :76).

## 3. Findings

### F-001 [S0] — "no decoder in this build" is a false claim about the build; the bench's private allowlist refuses codecs that have decoders

Where: apps/cli/src/bench.rs:300, 313, 328-341 (the allowlist + message); disproven by apps/cli/src/pipeline_host.rs:241-380 and audit/runs/baseline/mp4-smoke-bench.txt

Code (bench.rs:300-301 + 333-335, verbatim):
```rust
            MediaKind::Video if video.is_none() && fact.codec == "mjpeg" => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
```
```rust
                } else {
                    "no decoder in this build (first vertical: mjpeg + pcm)"
                };
```

W00 baseline output (audit/runs/baseline/mp4-smoke-bench.txt, verbatim):
```text
bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm)); audio track#1 aac — not benched (no decoder in this build (first vertical: mjpeg + pcm)) (container mp4)
BENCH file=/home/z/my-project/sample-h264-aac.mp4 bytes=45682 v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.000 decode_s=0.000
```

Code (pipeline_host.rs:288-291 + 346, verbatim):
```rust
            "aac" | "aac-lc" => {
                // ASC mode needs the esds blob; ADTS mode ignores
                // `extra_data` (in-band) — seed unconditionally.
                self.seeded(track, config, AacDecoder::new)
```
```rust
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
```

What is wrong: the message asserts no decoder exists in the build. The build contains a working AAC decoder and a working H264 software decoder (dispatched by the same crate's provider), and the W00 run emitted the false claim for both, on the project's litmus file. The bench's own selection guard is `fact.codec == "mjpeg"` / `starts_with("pcm_")` — a private 2-entry allowlist that predates every codec landing after the first vertical and was never extended.
Why it matters: this is the shipped binary's explanation for not benchmarking, it is false, and it has already corrupted the audit trail — the W00 baseline narrative "h264+aac MP4 … 0 frames decoded" is an artifact of this allowlist (the bench refused to decode; the player path was never asked). A tool whose diagnostic output inverts the truth about build capabilities is exactly the §4 "claim is false" case. If wave-20 judges the blast radius too narrow for S0, the floor is S2 — but the message must change regardless.
What correct looks like: bench uses `HostDecoderProvider::decoder_for` (one registry, no drift), and the note says "not benched (bench dispatches only mjpeg+pcm)" or nothing at all once the provider is shared.
Confidence: HIGH — Verification: read + W00 baseline output (the refusal and the false claim both captured verbatim above).

### F-002 [S2] — The runner measures 2 codecs but presents as the corpus speed-parity benchmark

Where: apps/cli/src/bench.rs:1-4 (module doc: "the Lumen runner of the ffmpeg speed-parity benchmark … `speed_bench.py --runner lumen='lumen decode-bench {file}'`")

Code (bench.rs:1-4, verbatim):
```rust
//! `decode-bench` — the Lumen runner of the ffmpeg speed-parity
//! benchmark (card W4-R2; the runner half of `tools/corpus/
//! speed_bench.py --runner lumen='lumen decode-bench {file}'`,
//! the gap 4.CORPUS-V3 registered).
```

What is wrong: the corpus's lanes are dominated by h264/hevc/aac/mp3/vorbis/opus/vp9/av1/mpeg4 — every one of which benches nothing under this runner (v_decoded=0, a_decoded=0, exit 0). Any PERF-VS-VLC comparison built from this runner compares ffmpeg's full decode against Lumen's demux-only walk for all but the mjpeg/pcm lanes, while `wall_s`/`decode_s` still print — numbers that look like speed-parity data but measure open+demux and two decoders.
Why it matters: §3.3 "performance claims the measured method cannot support"; the runner is registered as the speed-parity gap's answer.
What correct looks like: F-001's fix (shared provider) plus a stderr note naming exactly which stages were skipped per track.
Confidence: HIGH — Verification: read + W00 bench outputs (h264/aac/hevc lanes all refused).

### F-003 [S3] — Second decoder registry in the same crate (drift made structural)

Where: apps/cli/src/bench.rs:299-343 vs apps/cli/src/pipeline_host.rs:241-380

Code (bench.rs:316-318, verbatim):
```rust
                let (channels, rate) = fact.pcm.ok_or_else(|| facts_absent(handle))?;
                let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)
                    .map_err(|error| demux_host::wire_error(&error))?;
```

What is wrong: `pipeline_host::HostDecoderProvider` already encapsulates codec dispatch + facts seeding (including the PCM `with_format` seam bench.rs re-implements here, and the `extra_data` seeding bench.rs cannot do at all — which is part of why non-first-vertical codecs were never benched). Two registries in one crate guarantee the observed drift. Root cause of F-001/F-002.
Confidence: HIGH — Verification: read of both files.

### F-004 [S4] — Stale AVI-only doc: "non-AVI containers refuse here (D-018)"

Where: apps/cli/src/bench.rs:264-266 (comment), 8-11 (module doc "AviDemuxer::start + the strf facts pre-pass")

Code (bench.rs:264-266, verbatim):
```rust
    // 1. Open — the exact path `lumen open` takes (canonicalize →
    //    access → probe → demuxer start → strf facts). Garbage files
    //    refuse here (probe), non-AVI containers refuse here (D-018).
```

What is wrong: open_source dispatches 15 probe ids through the container registry (containers/mod.rs:50-75); non-AVI containers do NOT refuse — the W00 bench output itself proves it (`(container mp4)` in mp4-smoke-bench.txt, and the runner benches the MOV mjpeg+pcm fixture). The comment describes the D-018 era.
Confidence: HIGH — Verification: read + baseline output.

### F-005 [S4] — `per_frame_damage` mirrors the engine's crate-private rule locally

Where: apps/cli/src/bench.rs:178-190

Code (bench.rs:181-182, verbatim):
```rust
/// the `per_frame_damage` rule of `playback-engine`'s pipeline.rs,
/// mirrored locally: the engine's own is crate-private).
```

What is wrong: acknowledged copy of a crate-private classification rule — the third instance in this crate of app-layer re-implementation of crate-private logic (mp4.rs's esds walk, ogg.rs's classify, this). If the engine widens its concealment class, the bench silently diverges.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mark_process_start` / `process_start` | 84-92 | REAL-UNPROVEN | — | OnceLock origin; binary path seeded by main.rs:13; library fallback documented. |
| `NullSink` | 98-108 | trivial-correct | — | Counts, drops. |
| `Limb` + `feed` | 111-163 | REAL-UNPROVEN | — | Move-only packet, concealment streak with budget; damage classes split correctly (per_frame_damage vs fatal). |
| `Limb::drain` | 166-175 | REAL-UNPROVEN | — | Flush outcome counted. |
| `per_frame_damage` | 182-190 | REAL-UNPROVEN | S4 (F-005) | Local mirror. |
| `BenchReport` | 193-204 | trivial-correct | — | |
| `run_bench` | 213-238 | REAL-UNPROVEN | — | Prints line + optional damage note; typed failure → exit code. Reachable (run.rs:144). |
| `bench_line` | 242-254 | REAL-UNPROVEN | — | Exact contract; test-pinned. |
| `bench` | 258-425 | SHALLOW | S0/S2 (F-001/F-002) | Real open path; private 2-codec allowlist; honest per-track notes; demux fatal carries partial counts. |
| `configure` | 431-446 | REAL-UNPROVEN | — | No `extra_data` seeding possible (see F-003). |
| `facts_absent` | 450-456 | trivial-correct | — | Guarded-impossible typed refusal. |
| `kind_name` | 460-467 | trivial-correct | — | |
| tests: `bench_line_is_the_exact_contract` | 482-501 | REAL-UNPROVEN | — | Pure; pins field order/shapes. |
| tests: `happy_path_fixture_reports_bench_line` | 506-551 | REAL-UNPROVEN | — | REAL `run::run` invocation, pinned bytes (425516), full-decode assertions. Runtime-UNPROVEN (binary never ran in baseline). |
| tests: `garbage_file_is_a_typed_refusal` | 555-585 | REAL-UNPROVEN | — | Real run; probe refusal exit 65 pinned. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 3 | YES | Contract string equality; real end-to-end run with pinned byte counts and decode-completeness; real refusal path. (All runtime-UNPROVEN — no lumen_cli binary in the W00 baseline log.) |
| Coverage gap | — | No test exercises the not-benched arm (F-001's message) — the false claim is unpinned by any test, which is how it survived. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no decoder in this build (first vertical: mjpeg + pcm)" (the note) | bench.rs:334 | FALSE | pipeline_host.rs:288-291 (aac), :346 (h264) and 10 more arms; W00 output shows the false claim for aac+h264. |
| "non-AVI containers refuse here (D-018)" | bench.rs:266 | FALSE | containers/mod.rs:50-75 wires 15 ids; W00 bench ran container mp4. |
| "the benchmark isolates open + demux + decode, the stages whose speed ffmpeg's `-f null -` run exercises" | bench.rs:14-15 | SHALLOW | True only for mjpeg/pcm inputs; for every other codec it isolates open + demux + drop (F-002). |
| BENCH line contract | bench.rs:17-30 | REAL-UNPROVEN | Test-pinned format; W00 output matches. |
| "a codec family without a decoder shows up as `v_decoded < v_frames`" | bench.rs:38-39 | MISLEADING | The mechanism is real but the cause is the bench's own allowlist, not decoder absence. |

## 7. Dead / duplicated / unreachable code

- No dead code. Duplication: the decoder allowlist (F-003), `per_frame_damage` (F-005). The `NullSink.frames` counter is written but never read except the doc's "cross-checked" claim — no cross-check exists (the outcomes' sums are used instead); harmless dead field, S4-level nit noted here.

## 8. Dependencies & flags

- No feature gates; `pub mod bench;` unconditional (lib.rs:68). Uses codecs directly (bypassing pipeline_host) — the layering point of F-003.

## 9. Open questions for the next agent

- PERF-VS-VLC.md (wave 19's ledger): which of its rows were measured through this runner, and were any non-mjpeg/pcm rows presented as decode-speed parity? If yes, those rows are FALSE measurements (open+demux speed only).
- After F-001's fix, `speed_bench.py` (tools/corpus) needs no change (contract preserved), but its recorded historical numbers need re-derivation.
