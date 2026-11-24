# AUDIT: apps/cli/src/bench.rs
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-23 |
| Lines / functions | 586 / 12 prod fns (+3 tests) |
| File verdict | SHALLOW (mjpeg+pcm lane REAL and executed; the instrument as claimed is stale and lies about the build for 12 of 14 wired codec families) |
| Reachable from shipped entry point? | YES: main.rs → run::run:144 `Some(CliCommand::DecodeBench { source }) => bench::run_bench(&source, out, err)` (argv-only host verb) |
| Findings | S0: 1 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `./target/debug/lumen decode-bench /tmp/my-project/sample-h264-aac.mp4` → `BENCH … v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.036 decode_s=0.000`, exit 0, stderr "no decoder in this build (first vertical: mjpeg + pcm)"; `echo quit | ./target/debug/lumen open /tmp/my-project/sample-h264-aac.mp4` → h264+aac accepted, Buffering→Playing→Stopped→Idle, exit 0 (SAME binary decodes what the bench denies); `./target/debug/lumen engine-info` → `decoders: mjpeg, pcm_s16le`; `./target/debug/lumen decode-bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` exit 0 (the working lane); `lumen_cli-9887c2f6df9cd333 bench::` → 3/3 pass. Binary dated 2026-11-24 00:45 (newer than source). |

## 1. One-paragraph verdict
This is Wave 0/3's core instrument defect, reproduced end-to-end in one binary: the bench's limb selection is hard-coded to `fact.codec == "mjpeg"` and `starts_with("pcm_")` (bench.rs:300, 313) — it never consults `HostDecoderProvider`, the real registry that wires 14 codec descriptors (pipeline_host.rs:239-381, incl. h264/aac/flac/mp3/vorbis/opus/alac/adpcm/av1/vp9/dnxhd). On an H.264+AAC MP4 that the SAME binary demonstrably plays (`lumen open` → Playing, exit 0), `decode-bench` prints a success-shaped all-zero BENCH line, exits 0, and tells the user "no decoder in this build (first vertical: mjpeg + pcm)" — a false claim about the build. The module doc's coverage semantics ("a codec family without a decoder shows up as v_decoded < v_frames") is also false: the no-decoder case produces 0/0 (no limb, packets dropped), never v_decoded<v_frames. As the designated ffmpeg speed-parity instrument (its stated purpose), it measures nothing for 12 of 14 shipped decoder families while reporting success.

## 2. What actually works (verified)
— `./target/debug/lumen decode-bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → `BENCH file=… bytes=425516 v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.088 decode_s=0.087`, exit 0 — real open path, real MJPEG+PCM decode, counts match the pinned fixture truth.
— `lumen_cli-9887c2f6df9cd333 bench::` → 3/3 pass (format contract pin; happy-path fixture with real decode; garbage-file typed refusal exit 65, no BENCH line).
— Typed error paths are real: probe refusals, mid-walk demux fatals carry partial v_/a_ counts as details (bench.rs:381-396).

## 3. Findings
F-001 [S0]
Where: apps/cli/src/bench.rs:300-327 (selection), 334 (the claim)
Code:
```
            MediaKind::Video if video.is_none() && fact.codec == "mjpeg" => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
```
and
```
                } else {
                    "no decoder in this build (first vertical: mjpeg + pcm)"
                };
```
What is wrong: the bench hand-rolls a 2-codec registry instead of using the pipeline's `HostDecoderProvider` (which run.rs's player path uses). The stderr message asserts "no decoder in this build" — FALSE: the same binary (target/debug/lumen, built from this tree) decodes h264+aac via `lumen open` (executed: tracks accepted, Buffering→Playing, exit 0), and pipeline_host.rs:239-381 wires mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_*, av1/av01, vp9, dnxhd, h264/avc1, pcm_*. Executed proof: `decode-bench /tmp/my-project/sample-h264-aac.mp4` → all-zero BENCH line + that message + exit 0. Why it matters: this verb is the ffmpeg speed-parity instrument (module doc line 1-4; gap 4.CORPUS-V3); for every non-first-vertical codec family it records a successful zero measurement — speed-parity numbers for 12/14 shipped decoder families are garbage, and the user is told the build lacks decoders it has. The baseline MP4 smoke recorded the same contradiction (audit/runs/baseline/mp4-smoke.md:57 area). What correct looks like: build limbs via `HostDecoderProvider::decoder_for` (the open path's own registry); the "first vertical" framing dies with it. Confidence: HIGH — Verification: ran (three executions above).
F-002 [S2]
Where: apps/cli/src/bench.rs:35-42 (doc) vs 294-343, 358-375 (code)
Code:
```
//! their packet/decoded counts are the line's `v_*`/`a_*` pairs, so
//! a codec family without a decoder shows up as `v_decoded < v_frames`
//! (or `v_frames=0` when the demuxer enumerates no such track — the
```
What is wrong: both semantics claims are false. (a) "v_decoded < v_frames" for a missing decoder is impossible: `frames` is incremented only inside `Limb::feed`, and no Limb exists for a non-mjpeg/pcm track — its packets are dropped at the routing match (bench.rs:365-372), so the pair is 0/0. (b) `v_frames=0` is attributed to "the demuxer enumerates no such track", but my h264 run shows a track enumerated (note prints "video track#0 h264") with v_frames=0 — the bench's own codec gate produces the zero. v_decoded<v_frames occurs ONLY via concealed per-frame damage on an mjpeg limb. Why it matters: consumers reading the doc (speed_bench maintainers, rc-smoke authors) misread the line's meaning; the rc-smoke gate's PASS predicate `v_decoded==v_frames && a_decoded==a_frames` is vacuously true on all-zeros. What correct looks like: doc says "a family outside the bench's registry answers v_frames=0 (track enumerated, not benched)"; better, fix F-001 and the doc becomes true. Confidence: HIGH — Verification: read + ran.
F-003 [S2]
Where: apps/cli/src/bench.rs:213-237, 344-351
Code:
```
    match bench(source, err) {
        Ok(report) => {
            let _ = writeln!(out, "{}", bench_line(&report, source));
```
What is wrong: a bench that benched NOTHING (no limb matched) still answers exit 0 with a normal BENCH line — stdout is indistinguishable from a real measurement; only stderr carries "not benched". scripts/rc-smoke.sh's headless-decode gate greps `^BENCH ` and checks decoded==frames — it would PASS on an all-zero line. tools/perf/baseline.py's decode_once records the zero row as status ok. Why it matters: evidence inflation — zero measurements enter perf/speed tables as successful data points. What correct looks like: no limb ⇒ typed refusal (nonzero exit), or a `benched=0` field. Confidence: HIGH — Verification: ran (h264 mp4 → exit 0).
F-004 [S3]
Where: apps/cli/src/bench.rs:71-76, 126-163, 355-372
Code:
```
const CONCEALMENT_BUDGET: u32 = 1000;
```
(one `consecutive` counter shared by both limbs: `limb.feed(packet, &mut sink, &mut consecutive)?`)
What is wrong: the "1000 consecutive damaged packets" abort is cross-limb: an `Emitted` from the healthy audio limb resets the streak, so a fully-damaged video limb interleaved with healthy audio never trips the budget and the bench walks the whole file — contradicting the doc's "a stream whose every packet fails is not being benchmarked, it is broken" (per-stream failure is benched to EOF). Wasted time only; counts stay honest (concealed tallied). What correct looks like: one streak counter per limb. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: apps/cli/src/bench.rs:98-108
Code:
```
struct NullSink {
    /// Frames accepted (cross-checked against the outcomes' sums).
    frames: u64,
}
```
What is wrong: `frames` is never read (sink dropped after the loop; the report uses `limb.decoded` from DecodeOutcome sums) and no cross-check exists anywhere — the comment asserts a verification that is not implemented. Confidence: HIGH — Verification: read (rg for `sink`/`frames` uses).
F-006 [S4]
Where: apps/cli/src/bench.rs:1-4, 17-19
Code:
```
//! `decode-bench` — the Lumen runner of the ffmpeg speed-parity
//! benchmark (card W4-R2; the runner half of `tools/corpus/
//! speed_bench.py --runner lumen='lumen decode-bench {file}'`, the
```
What is wrong: the cited consumer doesn't consume: speed_bench.py's `bench_one` records only {rc, wall, cpu} — it never parses the BENCH line — and its own usage example (speed_bench.py:23) invokes `lumen decode-null {file}`, a verb that does not exist in the CLI (parse.rs:212 wires only "decode-bench"; rg for decode-null in apps/ + crates/ → zero hits). The "contract with speed_bench.py" has no working in-tree consumer as documented. (Defect lives in tools/corpus/speed_bench.py; bench.rs's citation is stale.) The actual line parsers are scripts/rc-smoke.sh:97-113 and tools/perf/baseline.py:421-455. Confidence: HIGH — Verification: read both tools.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mark_process_start | 84-86 | REAL | — | OnceLock; main.rs:13 seeds it first |
| process_start | 90-92 | REAL | — | honest library fallback |
| NullSink::accept | 103-108 | REAL | S4 | dead counter, lying comment (F-005) |
| Limb::feed | 126-163 | REAL | S3 | damage streak/budget shared (F-004); typed budget error |
| Limb::drain | 166-175 | REAL | — | flush counts Emitted |
| per_frame_damage | 182-190 | REAL | — | local mirror of engine rule (engine's crate-private) |
| run_bench | 213-238 | REAL | S2 | zero-limb success exit 0 (F-003); coverage note stderr-only |
| bench_line | 242-254 | REAL | — | exact contract pinned by test |
| bench | 258-425 | SHALLOW | S0/S2 | the hard-coded 2-codec selection (F-001); false doc semantics (F-002); packet drop for unbenched tracks |
| configure | 431-446 | REAL | — | no extra_data (mjpeg/pcm don't need it — true only for the hard-coded set) |
| facts_absent | 450-456 | REAL | — | guarded-impossible refusal, honest |
| kind_name | 460-467 | REAL | — | |
| tests ×3 | 482-585 | REAL | — | see §5; none covers the gating defect |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bench_line_is_the_exact_contract | YES | exact string pin of the line format |
| happy_path_fixture_reports_bench_line | YES | real open+decode, bytes=425516 pinned, v_decoded==v_frames>0, decode_s≤wall_s+0.002 — executed, passed |
| garbage_file_is_a_typed_refusal | YES | exit 65, `[probe.unsupported]`, no BENCH line — executed, passed |
| (missing) | — | NO test exercises a non-mjpeg/pcm track through the gate: F-001/F-002/F-003 are invisible to the suite (the h264-aac corpus files exist in tests/compat and /tmp but no bench test touches them) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decode to null over the real first-vertical pipeline pieces — the very open_source path the player uses" | bench.rs:8-11 | REAL | open_source called at bench.rs:270; executed |
| "a codec family without a decoder shows up as v_decoded < v_frames" | bench.rs:38-39 | FALSE | no limb → 0/0 (packets dropped); executed h264 run |
| "v_frames=0 when the demuxer enumerates no such track" | bench.rs:39-42 | FALSE (incomplete) | bench's own codec gate also yields v_frames=0 with the track enumerated ("video track#0 h264 — not benched") |
| "runner half of tools/corpus/speed_bench.py --runner lumen='lumen decode-bench {file}'" | bench.rs:2-4 | FALSE (stale) | speed_bench.py times rc/wall/cpu only; its example verb is `decode-null` which parse.rs doesn't have |
| "Never panics — every failure path is a typed CommandError" | bench.rs:209-212 | REAL-UNPROVEN | no unwrap/expect/index found (read); garbage + happy + h264 runs all clean; not exhaustively executed |
| "zero per-frame allocations of its own" | bench.rs:45-53 | REAL-UNPROVEN | consistent with code (packet moved, no per-iteration allocs visible); no allocation profiling run |

## 7. Dead / duplicated / unreachable code
- `NullSink.frames` — dead (F-005).
- Duplicated registry knowledge: the mjpeg/pcm hard-coding duplicates what HostDecoderProvider already owns (F-001's root cause) — a copy-paste registry that drifted.

## 8. Dependencies & flags
- codecs (mjpeg + pcm paths only — the narrow import is the smoking gun), codec_api, commands, media_types, plugin_api, crate::{demux_host, exit}. No feature gates. Note: the crate links the FULL codecs crate (h264 etc. compiled in — nm shows rust_h264 symbols) while using 2 of its decoders.

## 9. Open questions for the next agent
- tools/corpus/speed_bench.py owner: usage example verb `decode-null` (line 23) doesn't exist — any doc/row citing speed_bench as the speed-parity evidence chain should be checked for this break.
- If F-001 is fixed (provider-driven limbs), rc-smoke's all-zeros PASS hole (F-003) closes too; re-pin bench_line contract consumers (rc-smoke.sh, baseline.py).
