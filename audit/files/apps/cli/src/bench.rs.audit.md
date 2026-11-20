# apps/cli/src/bench.rs — W13-A01 audit

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A01 |
| Date | 2026-11-16 |
| Lines | 586 (12 functions + tests) |
| File verdict | **SHALLOW** — the harness runs, measures honestly for the mjpeg/pcm pair it still selects, but its codec selection layer froze at the first vertical: 11+ codec families decode in the shipped binary yet bench refuses them with a FALSE capability message, and its stdout line reports `v_frames=0` for a 75-frame video file |
| Reachable from shipped entry point | YES — `lumen decode-bench <file>`: main.rs:19 → run.rs:144 `bench::run_bench(&source, out, err)` (bypasses catalog dispatch by documented design, run.rs:42-47) |
| Findings | F-001 [S2], F-002 [S2], F-003 [S3], F-004 [S4], F-005 [S4] |

## 1 One-paragraph verdict

The runner's skeleton is real: it opens through the player's own `open_source` path (so the MP4 opened and demuxed to end-of-stream in the executed run), feeds packets to per-kind limbs, conceals per-frame damage with a bounded budget, and prints the exact BENCH contract line. What is not real is the codec lane: limb selection at 300-334 re-implements a two-entry decoder allowlist (`mjpeg`, `pcm_`) that forked away from the provider's 13-family dispatch table, so the flagship h264/aac MP4 produced `v_frames=0 v_decoded=0 a_frames=0 a_decoded=0` while the same binary's `open` path decoded both limbs to Ended. The message printed alongside ("no decoder in this build (first vertical: mjpeg + pcm)") is false in this build. And it is worse than a message problem: the bench has no extra_data seeding seam at all — its `configure` helper mints `extra_data: None` — so deleting the allowlist would not make h264/aac/flac/vorbis/opus/alac/adpcm decode here; every seeded codec would fail its configure gate. The harness verdict on the hostile question: the allowlist is not the only stale thing, but the harness does NOT miscount — it counts packets fed to selected limbs, which are zero because it selects none; the lie is the coverage (a 75-sample video reporting `v_frames=0`) and the message, not the arithmetic.

## 2 What actually works (verified)

- The open + demux half runs the player's real path: orchestrator run (audit/runs/baseline/mp4-smoke-decode-bench.txt):
  ```
  $ ./target/release/lumen decode-bench audit/runs/baseline/sample-h264-aac.mp4
  bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm)); audio track#1 aac — not benched (...) (container mp4)
  BENCH file=audit/runs/baseline/sample-h264-aac.mp4 bytes=45682 v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.000 decode_s=0.000
  EXIT=0
  ```
  The `bench:` note line proves the open path resolved the container (`container mp4`), enumerated both tracks (`track#0 h264`, `track#1 aac`), and the demux loop ran to `EndOfStream` (a BENCH line only prints after the loop's `break`, bench.rs:380). `bytes=45682` matches my own python box-walk of the file (45,682 bytes; video stsz count=75, audio stsz count=131, timescales 12800/44100, both durations 3.0 s) — so the file genuinely has 75 video frames and 131 audio frames, and the BENCH line reports zeros for both.
- The mjpeg/pcm lane works end-to-end and is pinned in the 1001-pass suite by `happy_path_fixture_reports_bench_line` (bench.rs:507-551): every packet of both streams of `avi_mjpeg_pcm.avi` decodes (`v_decoded == v_frames`), decode_s ≤ wall_s.
- Garbage input refusal is typed and pinned (556-585: `[probe.unsupported]`, exit 65, no BENCH line).
- The concealment budget and the per-frame damage rule (182-190) mirror the engine's — the loop discipline (D-1) claims match the code (no per-packet allocation: the packet moves into `feed` at 371).

## 3 Findings

### F-001 [S2] The limb-selection allowlist is a drifted fork of the provider registry, and its message is false in this build
Where: bench.rs:300-334
```rust
            MediaKind::Video if video.is_none() && fact.codec == "mjpeg" => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
```
… audio arm at 312-313:
```rust
            MediaKind::Audio
                if audio.is_none() && fact.codec.starts_with("pcm_") && fact.pcm.is_some() =>
```
… and the refusal reason at 328-341:
```rust
                } else {
                    "no decoder in this build (first vertical: mjpeg + pcm)"
                };
```
Wrong: this is a second, hand-maintained codec registry inside the app that drifted from the real one (`HostDecoderProvider::decoder_for`, pipeline_host.rs:218-363, 13 families + families incl. `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` at 328). Executed consequence: the same binary that just played the MP4's h264+aac to `Ended (100.0%)` (mp4-smoke-open-advance.txt) reports `v_frames=0 … a_decoded=0` and tells the user "no decoder in this build" — a false statement about the build, in both stdout semantics and stderr message.
Correct: route limb construction through `HostDecoderProvider` (or a shared codec-availability table) so the bench measures exactly what the player dispatches; delete the local message and derive any "not benched" note from the shared source.
Confidence: high — the executed bench output, the executed open output, and both code sites are in evidence.
Severity S2: this is the ffmpeg speed-parity harness (W4-R2/4.CORPUS-V3); for every codec family after the first two it silently measures nothing while claiming the build lacks the decoders — the benchmark's core metric is wrong for 11+ families in the shipped default feature set.

### F-002 [S2] No seeding seam: even with the allowlist fixed, every blob-dependent codec fails to configure here
Where: bench.rs:427-446
```rust
fn configure(
    decoder: &mut dyn Decoder,
    codec: &CodecDescriptor,
    generation: GenerationId,
) -> Result<(), CommandError> {
    let config = DecoderConfig::new(
        codec.clone(),
        generation,
        ThreadBudget::default(),
        None,
        None,
    );
```
Wrong: the last `None` is `extra_data` — the bench constructs decoders with no container blob, while the landed codecs that need one (flac STREAMINFO, vorbis three-header chain, opus OpusHead, aac ASC, alac cookie, adpcm WAVEFORMAT, h264 avcC — pipeline_host.rs:184-214's whole `seeded` seam exists because "the engine's `obtain_decoder` has no container knowledge") refuse configure typed. The bench's only facts seam is `PcmDecoder::with_format` (316-318), the 2.B5-era shape. So the harness is not one stale `if` away from correctness — it structurally cannot bench the modern pipeline; it measures the first-vertical pipeline only.
Correct: reuse `HostDecoderProvider::new(&facts)` for limb construction (it already takes exactly the `Arc<MediaFacts>` the bench holds at 271) and the provider's own refusal becomes the honest note.
Confidence: high — the `configure` signature and call sites are fully quoted; the seeded-codec configure gates are cited from the provider's own arm comments (pipeline_host.rs:235-284).

### F-003 [S3] `v_frames=0` on a 75-frame file: coverage-by-omission, and the module doc's own explanation of the zero is wrong
Where: bench.rs:35-42 (module doc) vs the executed line; the drop at 365-374
```rust
                if let Some(limb) = routed {
                    limb.feed(packet, &mut sink, &mut consecutive)?;
                }
                // Packets of unbenched tracks are dropped here (the
                // coverage note above said which and why).
```
Wrong in two layers. (1) Arithmetic: NOT a miscount — `Limb::frames` increments only inside `feed` (132), packets of unbenched tracks are dropped unrouted, so 0 fed → 0 counted is internally consistent. (2) Semantics: the field name `v_frames` on a machine-parseable line reads as demux coverage; for the h264 file it is 0 while the demuxer yielded 75 video samples (my box-walk) — and the module doc's own rule for this case is factually wrong: "or `v_frames=0` when the demuxer enumerates no such track" (41-42) — here the demuxer DID enumerate the track (the note line names `track#0 h264`), the bench simply selected no limb. A consumer parsing only stdout (the documented contract: "stdout carries exactly one machine-parseable line") cannot distinguish "file has no video" from "harness refused the video".
Correct: either bench the track (F-001/F-002) or report a demuxed-packet counter per kind regardless of limb selection, so the line distinguishes refused lanes from absent tracks.
Confidence: high — executed output, python box-walk (75/131 samples), and the drop site quoted.
Severity S3: the line is the speed_bench.py contract; its zeros are currently ambiguous-laden.

### F-004 [S4] The happy-path test cements the stale lane
Where: bench.rs:507-551 — `happy_path_fixture_reports_bench_line` runs only `generated/valid/avi_mjpeg_pcm.avi`; no test exercises `decode-bench` on any post-first-vertical codec file, which is why F-001 could drift through waves 4-8 unnoticed. Recorded as test-honesty debt.

### F-005 [S4] Module doc names the retired AVI-only open path
Where: bench.rs:8-15 — "the very [`crate::demux_host::open_source`] path the player uses (canonicalize → `input` access → `media-probe` → `AviDemuxer::start` + the strf facts pre-pass)". The `open_source` claim is TRUE (bench.rs:270 calls it; the MP4 run proves it opens non-AVI), but "AviDemuxer::start + the strf facts" describes the 2.D2-era tail; the real tail since 6.OPW is the container registry (demux_host.rs:458 → containers/mod.rs:50-75). Stale prose only; the code beneath it is shared-verbatim. S4.

## 4 Per-function table

| Function (line) | Role | Verdict |
|---|---|---|
| `mark_process_start` (84-86) | seed process wall origin (called by main.rs:13) | REAL |
| `process_start` (90-92) | origin or fallback now | REAL |
| `NullSink::accept` (103-108) | decode-to-null sink, counts frames | REAL |
| `Limb::feed` (126-163) | move + decode + count; damage streaks; budget refusal | REAL — arithmetic honest |
| `Limb::drain` (166-175) | end-of-stream flush | REAL |
| `per_frame_damage` (182-190) | the engine's concealment class, mirrored locally | REAL — acknowledged duplicate (crate-private original) |
| `run_bench` (213-238) | verb entry: print BENCH or typed error, exit code | REAL — executed |
| `bench_line` (242-254) | the exact contract formatter | REAL — pinned by test (483-501) |
| `bench` (258-425) | open via `open_source`, stat, limb selection (the stale allowlist), the packet loop, drains, report | REAL skeleton with F-001/F-002 selection layer |
| `configure` (431-446) | limb configure with `extra_data: None` | REAL code, structurally stale (F-002) |
| `facts_absent` (450-456) | impossible-facts typed refusal | REAL (defensive) |
| `kind_name` (460-467) | note word for MediaKind | REAL |
| tests (469-586) | contract pin, happy path (avi only), garbage refusal | REAL — but cements the lane (F-004) |

## 5 Test honesty

- `bench_line_is_the_exact_contract` (483-501) — honest: pins the field order/shape speed_bench.py depends on.
- `happy_path_fixture_reports_bench_line` (507-551) — honest for what it runs, dishonest as coverage: it pins `v_decoded == v_frames` on the ONLY lane the harness supports; no decode-bench test exists for any of the 11+ families the player decodes (h264/aac/flac/mp3/vorbis/opus/alac/av1/vp9/…). The gap is exactly why the executed MP4 bench reported zeros without failing anything.
- `garbage_file_is_a_typed_refusal` (556-585) — honest.
- All three are in the 1001-pass suite (orchestrator context, 2026-11-16).

## 6 Doc claims

- docs/feature-matrix.toml: no row claims decode-bench coverage for the h264 lane (grep for `decode-bench|engine-info` in feature-matrix.toml → no matches), so no doc lie is propagated there — the matrix's h264 e2e row (line 428) cites the PLAYER lane (`mp4_h264_family_lane`), not the bench. CONSISTENT, and it is precisely this split that leaves the bench unaccountable to the matrix.
- bench.rs's own module doc (35-43) claims honest per-stream coverage — the shape is honest, the `v_frames=0` explanation is wrong for the refused-limb case (F-003).

## 7 Dead/duplicated/unreachable

- `per_frame_damage` (182-190) — a locally mirrored copy of the engine's crate-private rule, acknowledged in its own doc comment ("the engine's own is crate-private"). Duplicated by necessity; drift risk recorded.
- `facts_absent` (450-456) — unreachable by its own admission ("harvest cannot produce it, the selection guards on it anyway"). Defensive, documented.
- The `DemuxEvent::Discontinuity` arm (379) is a no-op — honest pass-through for a shape the bench does not need to act on.

## 8 Dependencies & flags

Depends on `crate::demux_host` (open path, `wire_error`), `codec_api`/`codecs` (the two decoders it constructs directly), `commands` (typed errors), `plugin_api` (DemuxEvent). It does NOT depend on `crate::pipeline_host` — that is the structural root of F-001/F-002: the bench sits beside the provider, not on it. `codec-h264-sw` is in the shipped default (crates/codecs/Cargo.toml:47-53) and apps/cli/Cargo.toml resolves default features, so the decoders the message denies are compiled into this very binary.

## 9 Open questions

- Does tools/corpus/speed_bench.py (the W4-R2 consumer of this line) branch on `v_frames=0` vs nonzero? If it averages parity over lanes, the zeros are silently degrading every non-mjpeg/pcm lane's parity row. UNVERIFIED — outside my file set (tools/corpus belongs to another auditor).
- `wall_s=0.000` for a full open+demux of a 45 KB file: plausible (sub-millisecond open), but the 3-decimal format cannot represent it; a larger file's parity numbers will be fine, a tiny file's wall_s is always 0.000. Cosmetic bound of the format, recorded.
