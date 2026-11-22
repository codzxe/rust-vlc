# AUDIT: apps/cli/src/containers/mkv.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 1303 / 7 production + 17 test items |
| File verdict | REAL-UNPROVEN (production half) / SHALLOW (corpus-differential half — see F-002) |
| Reachable from shipped entry point? | YES — main.rs:19 `run::run` → run.rs:165/198/272 `lumen_dispatch` → dispatch.rs:82 `host.open_session` (SessionOpen arm of `route`) → host.rs:904 `open_source` → demux_host.rs:362→378 `open_source_with` → demux_host.rs:458 `crate::containers::open` → containers/mod.rs:57 `"matroska" => mkv::open(stream, opts)` → mkv.rs:142 `open` |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `rg`/`sed -n` reachability + quote verification (all quotes below verified with `sed -n`); corpus presence checks (`ls /tmp/my-project/corpus-v4/media` → absent). NO cargo (adaptation) — run claims cite baseline logs or are UNVERIFIED. |

## 1. One-paragraph verdict

This is the Matroska container *host* — NOT a second EBML parser (the layering question): every box/EBML walk is delegated to `demux::mkv::MkvDemuxer` (mkv.rs:147-148); the file only harvests per-track facts (codec config blobs, geometry, dimensions) and wraps in the shared `SelectDemuxer`. The production half (357 lines) is careful, bounds-checked code. The remaining ~940 lines are tests, and they are the problem: the 34-lane "corpus differential" silently self-skips when the corpus dir is absent (it is absent here), its h264 expectations contradict the current `pipeline_host.rs` (which dispatches an H264 software decoder while this file's docs and test assertions claim a typed refusal), and the W00 baseline `cargo test --workspace` never executed any lumen-cli test at all (it stopped, fail-fast, at `first-vertical`). So the file's headline evidence — the ffmpeg-measured differential — has no recorded green run against the current tree.

## 2. What actually works (verified)

- Call chain above verified by `rg -n` at every hop (each site read in context).
- `vorbis_chain` (268-313) and `flac_streaminfo` (320-334): all slicing through `Option`-returning `get`/`get(..)?`, saturating accumulation, magic/type/length gating — no panic path found on hostile input; unit tests at 692-725/730-753 exercise wrong-length, wrong-block-type, lying-length, unterminated-lacing and 0xFF-accumulation vectors. Verified by reading; not run (no cargo) — UNVERIFIED at runtime.
- Fixtures referenced by the non-corpus tests exist: `tests/fixtures/generated/valid/{mkv_h264_aac.mkv, mkv_h264_flac.mkv, mkv_multitrack.mkv, webm_vp9_opus.webm}` (verified with `ls`).
- Baseline fact (W00, audit/runs/baseline/mjpeg-smoke-bench.txt): the `matroska`-family decode path is exercised end-to-end only for vp9/opus/flac/vorbis/aac lanes in this file's tests — none of which ran in the W00 baseline (see F-002).

## 3. Findings

### F-001 [S2] — Module doc and corpus test claim h264 typed-refusal; the shipped provider dispatches an H264 software decoder

Where: apps/cli/src/containers/mkv.rs:54-58 (doc), 763-767 (comment), 1146-1160 (assertion); contradicting apps/cli/src/pipeline_host.rs:341-346

Code (mkv.rs:54-58, verbatim):
```rust
//! avcC `configurationVersion` 1 — ffmpeg's extradata shape). No
//! software h264/hevc decoder is dispatched in this build: the
//! blob is the forward-honest config, and `decoder_for` refuses
//! the codec typed (`cli.codec_unavailable`) — the hw-path cards
//! own the seam.
```

Code (pipeline_host.rs:346, verbatim):
```rust
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
```

What is wrong: `H264SwDecoder` is imported unconditionally (pipeline_host.rs:65) and dispatched at 346 (ADR-001, comment cites "Lead-admitted 2026-11-15"). The mkv.rs doc claim "No software h264/hevc decoder is dispatched in this build … `decoder_for` refuses the codec typed (`cli.codec_unavailable`)" is therefore FALSE for h264 (still true for hevc — no arm). Consequently the corpus differential's pass-2 refusal assertion cannot hold for the ~17 h264 lanes:

Code (mkv.rs:1152-1156, verbatim):
```rust
                        assert!(
                            matches!(
                                err.context.code.as_str(),
                                "cli.codec_unavailable" | "aac.engine.init"
                            ),
```

If `H264SwDecoder::configure` succeeds for the harvested avcC, `decoder_for` returns `Ok` and the closing cross-check `assert!(video.is_none(), "{name}: no video decoder expected")` (mkv.rs:1230-1232) fails; if configure fails, its error code is neither of the two accepted codes and the assertion above fails. Either way the 34-lane test cannot pass against the current tree when the corpus is present.

Why it matters: this file's documentation is the recorded "truth table" for what the CLI does with MKV h264 (the project's core open question), and it is stale in the direction of under-claiming; the differential test marketed as "the user's standing mandate" (mkv.rs:755-758) has expectations that contradict the shipped provider.
What correct looks like: doc updated to the ADR-001 state; TRUTHS lanes re-measured with h264 decode counts (or the refusal list shrunk to hevc/mpeg4/vp8/ffv1/ac3).
Confidence: HIGH — Verification: read (both files, contradiction is syntactic); runtime failure of the test UNVERIFIED (no cargo; corpus absent).

### F-002 [S2] — The corpus differential self-skips as PASS when the corpus is absent, and no baseline run ever executed it

Where: apps/cli/src/containers/mkv.rs:1083-1098 (gate), 1100-1105 (skip-and-return-Ok); context: audit/runs/baseline/cargo-test.log

Code (mkv.rs:1088-1089 + 1092-1097, verbatim):
```rust
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
```
```rust
        eprintln!(
            "corpus(mkv-open): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for this instance is in the worklog)"
        );
```

What is wrong: 4 of this file's 10 tests (`start_time_skips_packets_and_reports_indexed_seek`, `six_channel_aac_geometry_flows_but_the_decoder_refuses`, `corpus_matroska_lanes_match_the_ffmpeg_truth`, `vorbis_lane_decodes_the_full_stream_from_the_laced_private`) `return` after the eprintln — counted as pass, not ignored. In this sandbox `/tmp/my-project/corpus-v4/media` does not exist (verified `ls`), so none of them test anything here. Worse: the W00 baseline `cargo test --workspace` output (audit/runs/baseline/cargo-test.log, 2.8 MB) contains **no `lumen_cli` test binary at all** — the run stopped fail-fast at `first-vertical` (`error: test failed, to rerun pass \`-p first-vertical --test evidence\`` at cargo-test.log:2499, EXIT=101), so every package sorted after it — including all of apps/cli — never ran. The "1751 pass / 2 fail / 25 ignored" baseline fact is a partial-workspace count.

Why it matters: the differential is this file's only broad correctness evidence, and there is no recorded green run of it against the current tree — combined with F-001 the expectation table is provably stale, so the lane table's "measured this session" comments (e.g. mkv.rs:16-17) describe a session that predates the h264 arm.
What correct looks like: corpus-gated tests should `#[ignore]` + a documented `--ignored` invocation (counted as ignored, not pass), and the workspace baseline must be re-run with `--no-fail-fast`.
Confidence: HIGH — Verification: read + `ls` + baseline-log grep (no `lumen_cli`/`container_conformance`/`native_seek`/`pipeline_accounting`/`bench_decode` binary among the 116 listed).

### F-003 [S3] — `pcm` field carries non-PCM data for AAC tracks (field-name lie)

Where: apps/cli/src/containers/mkv.rs:205-208

Code (verbatim):
```rust
        let pcm = private
            .filter(|_| track.kind == MediaKind::Audio)
            .and_then(audio_pair)
            .filter(|_| codec.starts_with("pcm_") || codec == "aac");
```

What is wrong: `TrackFact.pcm` is documented in demux_host.rs:155 as "PCM channels + rate, when the track is PCM". Here it is stuffed with the Audio master's geometry for `aac` tracks (module doc admits it at mkv.rs:84-87). It only works because the `pcm_` decode arm in pipeline_host.rs:347 matches on the codec string first — the field's name and its contract are false for this container.
Why it matters: naming that lies (§3.5); a future consumer reading `fact.pcm.is_some()` as "this is PCM" misroutes AAC.
What correct looks like: a dedicated `audio_geometry: Option<(u32,u32)>` field (the accessor `TrackFact::audio_geometry` already exists at demux_host.rs:202 — the field should carry it, not the PCM alias).
Confidence: HIGH — Verification: read.

### F-004 [S4] — Silent facts degradation when the TrackEntry ordinal lookup misses

Where: apps/cli/src/containers/mkv.rs:198-204

Code (verbatim):
```rust
        let entry = demuxer
            .track_entries()
            .iter()
            .find(|entry| entry.ordinal == track.id.handle());
        let codec = track.codec.as_str();
        let private = entry.map(|entry| &entry.private);
```

What is wrong: an unmatched ordinal yields `private = None` → no `extra_data`, no `pcm`, no `dimensions`, no `frame_ns`, with no error or log. A demuxer/host ordinal mismatch would silently strip decoder seeding for the whole session (decoders then refuse at `configure` with codec-side codes, pointing investigation in the wrong direction).
Why it matters: swallowed structural mismatch (§3.2 "swallowed errors"); blast radius limited to a hypothetical mismatch.
What correct looks like: the harvest should assert/return a typed error when a minted track has no entry.
Confidence: MEDIUM — Verification: read (the mismatch is hypothetical; no evidence it occurs).

### F-005 [S4] — Duplicated `i64::MAX / 4` duration-clamp fallback across 7 container hosts

Where: apps/cli/src/containers/mkv.rs:153 (identical line in ogg.rs:86, wav.rs:53, avi.rs:43, ps.rs:153, mp4.rs:106, aiff.rs:59 — verified by `rg`)

Code (verbatim, mkv.rs:153-154):
```rust
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
```

What is wrong: the same magic bound + clamp logic is copy-pasted in every host (deliberate per the wiring doc's collision rule, but still 7 copies of a subtle numeric policy). `saturating_mul(1000)` downstream makes it overflow-safe; the issue is duplication, not correctness.
Why it matters: §3.5 copy-paste; a future fix (e.g. real duration fallback) must land 7 times.
What correct looks like: one `clamp_start(opts, &facts)` helper in demux_host.
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 142-187 | REAL-UNPROVEN | — | Reachable (chain §top). Delegates EBML to `MkvDemuxer::new/start/seek`; `saturating_mul` for µs→ns; seek query before wrapper. No panic path. Not executed here (no cargo). |
| `harvest_facts` | 195-233 | REAL-UNPROVEN | — | O(tracks × entries) linear find per track — fine at track counts. F-004 silent-miss; F-003 pcm-stuffing. |
| `config_blob` | 239-257 | REAL-UNPROVEN | — | Empty-private → None for every codec (honest). Verbatim passthrough for aac/alac/opus/h264/hevc/mpeg4; vorbis/flac via parsers. |
| `vorbis_chain` | 268-313 | REAL-UNPROVEN | — | Bounds-checked throughout (`get`, `checked_add`, saturating add); magic gate; exactly-3-headers enforced. Adversarial unit tests exist (692-725). |
| `flac_streaminfo` | 320-334 | REAL-UNPROVEN | — | len==34 bare fallback; fLaC path verifies type+declared length before `[8..42]` slice (len≥42 gate). Unit tests 730-753. |
| `audio_pair` | 340-347 | REAL-UNPROVEN | — | channels 1..=64, rate finite (0, 4_096_000]; `round() as u32` safe under the bound. |
| `dimensions_of` | 350-357 | REAL-UNPROVEN | — | u64→u32 guarded by explicit `> u32::MAX` check; zero rejected. |
| tests: `fixture`/`open`/`CountingSink`/`decoder_for` helpers | 373-426 | REAL-UNPROVEN | — | Helpers drive the REAL `open_source` path + REAL provider — no mocks of the subject. |
| `h264_aac_fixture_opens_with_expected_facts` | 435-475 | REAL-UNPROVEN | — | Committed fixture; asserts concrete blob bytes (avcC v1, 5-byte ASC, AOT=LC), dims, frame_ns, geometry. Would fail if harvest broke. |
| `flac_fixture_opens_and_decodes_the_full_stream` | 483-525 | REAL-UNPROVEN | — | Full-stream decode through real provider (20 pkts → 88 200 frames). Strong if it runs — it never ran in baseline (F-002). |
| `multitrack_fixture_deselects_subtitles_at_open` | 532-560 | REAL-UNPROVEN | — | Asserts SelectDemuxer kind-filter + per-track packet counts. |
| `webm_vp9_and_opus_decode_from_the_private_head` | 570-628 | REAL-UNPROVEN | — | Full decode of both tracks. |
| `start_time_skips_packets_and_reports_indexed_seek` | 634-664 | SHALLOW | S2 (F-002) | Corpus-gated; skips-as-pass without corpus. |
| `six_channel_aac_geometry_flows_but_the_decoder_refuses` | 671-687 | SHALLOW | S2 (F-002) | Corpus-gated. Asserts `aac.engine.init` refusal (engine mono/stereo gate) — consistent with pipeline_host (AAC arm exists). |
| `vorbis_lacing_strip_joins_the_measured_packing` | 692-725 | REAL-UNPROVEN | — | Hand-built vectors incl. adversarial; runs without corpus. |
| `flac_streaminfo_extracts_the_verified_block` | 730-753 | REAL-UNPROVEN | — | Adversarial vectors; runs without corpus. |
| `Lane` struct + `TRUTHS` (34 lanes) | 773-1079 | STALE | S2 (F-001) | ~17 h264 lanes expect typed refusal contradicted by pipeline_host.rs:346. |
| `corpus_dir` / `skip_loudly` | 1083-1098 | SHALLOW | S2 (F-002) | Skip-as-pass gate. |
| `corpus_matroska_lanes_match_the_ffmpeg_truth` | 1100-1234 | SHALLOW | S2 (F-001/F-002) | Two-pass differential; cannot pass with corpus present per F-001; never ran in baseline. |
| `vorbis_lane_decodes_the_full_stream_from_the_laced_private` | 1243-1302 | SHALLOW | S2 (F-002) | Corpus-gated. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| h264_aac_fixture_opens_with_expected_facts | YES | Concrete byte-level assertions on a committed fixture through the real open path. (Not executed here — UNVERIFIED at runtime.) |
| flac_fixture_opens_and_decodes_the_full_stream | YES | Exact frame counts vs ffmpeg's own; would catch harvest/decode regressions. |
| multitrack / webm tests | YES | Exact packet/frame counts. |
| vorbis_lacing / flac_streaminfo unit tests | YES | Adversarial vectors pin the refusal arms. |
| The 4 corpus tests | NO (in a corpus-less env) | They return Ok before asserting anything (F-002); in a corpus env the h264 lanes fail for the WRONG reason (F-001 — stale expectations, not broken code). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No software h264/hevc decoder is dispatched in this build … refuses typed (`cli.codec_unavailable`)" | mkv.rs:55-58 | FALSE (h264 half) | pipeline_host.rs:346 `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)`; import at :65. |
| "every shape below MEASURED on this corpus instance this session" | mkv.rs:16-17 | UNVERIFIED | Corpus absent here; demux crate's `corpus_matroska_differential_vs_ffprobe ... ok` in baseline (cargo-test.log:2100) proves the corpus existed in the W00 env, but the lumen_cli differential never ran there (no lumen_cli binary in log). |
| AAC extra_data = bare ASC "RESOLVED POSITIVE by measurement" | mkv.rs:33-40 | REAL-UNPROVEN | Fixture test 435-475 asserts the 5-byte ASC with AOT 2 on a committed fixture. |
| vp9 "DECODES (card 6.M-vp9 landed)" | mkv.rs:69-71 | REAL-UNPROVEN | pipeline_host.rs:313-317 dispatches `Vp9Decoder`; webm fixture test asserts 20 frames. Not run here. |
| Subtitles deselect at open | mkv.rs:108-111 | REAL-UNPROVEN | `SelectDemuxer::start` filter at demux_host.rs:566-576 drops non-A/V kinds; multitrack test pins it. |

## 7. Dead / duplicated / unreachable code

- No dead production code found; `open` is the module's only entry and is dispatched (mod.rs:57).
- Duplication: `vorbis_chain`'s output shape is the same "verbatim join" the OGG host builds (ogg.rs — audited separately); `flac_streaminfo` here vs `flac_streaminfo_rate` in crates/demux/src/ogg/mod.rs:307 are different jobs (block extraction vs rate sniff) — acceptable, but a shared Matroska/Ogg "Vorbis chain" helper would remove one copy.
- F-005's 7-way clamp duplication.

## 8. Dependencies & flags

- No feature flags gate this file: `mod mkv;` unconditional (containers/mod.rs:27), dispatch arm unconditional (mod.rs:57). `demux`, `media-types`, `plugin-api`, `commands` path deps (apps/cli/Cargo.toml). No cfg(test) branches in production code.

## 9. Open questions for the next agent

- Does `H264SwDecoder::configure` succeed on the mkv avcC harvest? If yes, F-001's test-failure prediction is certain; either way the doc is stale. (Needs a run — no cargo here.)
- Wave-20 re-verification: the W00 baseline "1751 pass / 2 fail / 25 ignored" is a partial count (fail-fast at first-vertical, cargo-test.log:2499). Everything after `first-vertical` alphabetically — including ALL of apps/cli — has no baseline coverage. This must be flagged to the orchestrator (see audit/runs/W13-A03.log.md).
