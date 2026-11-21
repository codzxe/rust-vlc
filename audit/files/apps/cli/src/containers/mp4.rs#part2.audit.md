# AUDIT: apps/cli/src/containers/mp4.rs#part2 (lines 573-1176)
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2026-11-21 |
| Lines / functions | 604 audited (of 1176; chunk boundary pinned at the natural seam: part2 = the `--start-time` test's doc comment (573) → EOF; part1 = 1-572 (A07) — ALL production code (`open`, `harvest_facts`, `config_blob`, `dsi_of_esds`, `opus_head_of_dops`, …) lives in part1; this chunk is entirely the test suite) / 14 items (8 tests + 2 helpers + `Lane` + `TRUTHS` + 2 corpus-gate fns) |
| File verdict | REAL (chunk = test suite; 4 of 8 tests executed green in baseline; the flagship corpus differential is FALSE-vs-code when its fixtures exist — see F-001) |
| Reachable from shipped entry point? | YES (the tests drive the REAL open path: `open_source` → containers/mod.rs `"mp4" => mp4::open` → `Mp4Demuxer` — the same chain the shipped binary executes, proven by mp4-smoke-open-session-420.txt / mp4-smoke-open-session-aac.txt; the executed tests are cited below) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `Read` over the full 1176 lines (part1 context) + part2 in detail; `rg "containers::mp4" audit/runs/baseline/*.log.txt` (all 11 mp4 tests "ok" ONLY in orchrun-test-workspace-nofailfast.log.txt:2813-2823 — the fail-fast run #3 aborted at apps/desktop/tests/zero_alloc_transport.rs before lumen-cli); `awk` timing over the nofailfast lumen_cli section (129 tests in 0.38s); `ls -d /tmp/my-project/corpus-v4/media` (ABSENT in this sandbox); `rg "corpus-v4" docs/` (docs/corpus/corpus-v4-smoke-manifest-20261023.json — jobs/params structure; h264 lanes mix yuv420p/yuv444p); `sed -n` of crates/codecs/src/video/h264/sw.rs:231-258,440-446 (`seed_extradata`; `configure` with `extra_data: None` → Ok) + pipeline_host.rs:346 ("h264"\|"avc1" → seeded H264SwDecoder) — the dispatch proof for F-001; `sed -n` of crates/demux/tests/corpus_mp4.rs (TRUTHS twin: 0008 h264 345 / 0060 h264 1412 aac 2029 / 0105 h264 84 aac 62 mp3 55 — CLI table matches); `rg "h264_sw_differential"` (committed-fixture decoder proof, tests/fixtures/h264, not corpus-gated); `sed -n` quote verification for every finding; `rg "corpus" audit/files/apps/cli/src/containers/es.rs.audit.md` (A02's precedent findings for the same gate pattern). |

## 1. One-paragraph verdict
This chunk is the MP4 container's test suite, and it splits into a real half and a vacuous half. The real half (4 tests: `--start-time` skip watermark + seekability, hidden chapter trak, dOps→OpusHead synthesis, esds DSI walk) executed green in the baseline no-fail-fast run and genuinely pins behavior through the REAL open path. The vacuous half is the flagship: `corpus_iso_bmff_lanes_match_the_ffmpeg_truth` — the CLI layer's only broad MP4/MOV/M4A end-to-end differential (31 lanes, ffmpeg-measured packet and decode counts) — is gated on an untracked sandbox path (`/tmp/my-project/corpus-v4/media`) that did not exist in the baseline environment, and, worse, its assertions encode the pre-2026-11-15 world in which h264 refused decode: since the H264 software decoder landed (dispatched at pipeline_host.rs:346, configure succeeds lazily even without a blob at sw.rs:442-445), every h264 lane in the table would fail one of the test's two refusal/cross-check asserts if the corpus were mounted — the recorded "ok" is a silent skip, and the skip is PROOF the corpus was absent (the test cannot pass against this code). Three sibling tests share the same gate and the same vacuous "ok" lines.

## 2. What actually works (verified)
— `start_time_skips_packets_and_reports_indexed_seek` EXECUTED ok (orchrun-test-workspace-nofailfast.log.txt:2823): opens the committed `mp4_h264_aac.mp4` with `--start-time 1ms` through the real open path, pins `start_position_ns == 1_000_000`, `SeekSupport::Fast { indexed: true }`, and walks every packet asserting `ns.ticks >= 1_000_000` with `seen > 0` — a real watermark test (the only hole is F-004).
— `chapter_trak_is_hidden_and_markers_stay_demuxer_side` EXECUTED ok (log 2813): pins the 2-track open (chapter trak hidden) and both media traks flowing, then asserts the demuxer's 3 titled markers directly — honest about the open path's missing chapter surface.
— `dops_synthesizes_the_rfc7845_opus_head` EXECUTED ok (log 2816): family-0 19-byte head, LE field re-packing (`[0x00,0x38]` pre-skip, `[0x00,0x00,0xac,0x44]` rate), family-1 mapping table verbatim — a real pin of `opus_head_of_dops` (version gate excepted, F-003).
— `esds_dsi_walk_extracts_the_asc` EXECUTED ok (log 2817): hand-built descriptor chain walked to the exact DSI slice `[0x11,0x90]`, LC sniff, and truncated-chain refusal — re-verified by hand here: expandable lengths (0x15/0x11/0x02), the 13 fixed DecoderConfig bytes, and the `esds[..12]` truncation all behave as asserted.
— TRUTHS cross-consistency: the CLI table's packet counts match the demux crate's `corpus_mp4.rs` twin on the checked lanes (0008 h264 345; 0060 h264 1412 + aac 2029; 0105 h264 84 + aac 62 + mp3 55) — no drift between the two tables.

## 3. Findings

### F-001 [S2] The flagship 31-lane corpus differential asserts a dead world: with the corpus mounted it FAILS against the current code (h264 dispatches); its recorded "ok" proves the corpus was absent in every evidence run
Where: apps/cli/src/containers/mp4.rs:1001-1016 (refusal allowlist) and 1063-1068 (cross-check), over the TRUTHS table 730-941
Code:
```
                        assert!(
                            matches!(
                                err.context.code.as_str(),
                                "cli.codec_unavailable" | "aac.engine.init"
                            ),
```
and
```
            if lane.video_frames.is_none() {
                assert!(video.is_none(), "{name}: no video decoder expected");
            }
```
What is wrong: every h264 lane in TRUTHS (0008, 0012, 0021, 0025, 0027, 0032, 0049, 0058, 0060, 0062, 0069, 0074, 0082, 0105, 0109, 0112, 0122, 0061 — all with `video_frames: None`) is asserted to produce NO video decoder. But `HostDecoderProvider::decoder_for` dispatches `"h264" | "avc1"` to the seeded `H264SwDecoder` (pipeline_host.rs:346, landed 2026-11-15), and `configure` returns `Ok` even with `extra_data: None` (sw.rs:442-445 — lazy Annex-B) and `Ok` with an avcC blob for 4:2:0 (proven end-to-end on committed MP4 fixtures by crates/codecs/tests/h264_sw_differential.rs, six 4:2:0 lanes). So for each h264 lane, run with the corpus present, `decoder_for` answers either (a) `Ok(decoder)` → `video` is `Some` → the cross-check `assert!(video.is_none())` at 1066-1068 PANICS, or (b) a typed `h264.sw.*` refusal (4:4:4 lanes — the corpus family mixes yuv420p and yuv444p h264, per docs/corpus/corpus-v4-smoke-manifest-20261023.json params) → not in the allowlist at 1008-1013 → PANICS. No outcome passes. The test therefore cannot pass against the current code with fixtures present — its recorded `ok` (orchrun-test-workspace-nofailfast.log.txt:2814) is only possible because `corpus_dir()` returned `None` and the test returned early after an `eprintln` that cargo swallows on success. This doubles as proof the corpus was absent in the baseline environment (the h264 lane predates the baseline by six days). The stale premise is visible in the table's own comment (712-714: "the honest typed refusal where none is (h264/hevc/mpeg4video — the hw-path cards own those seams)") and in the module docs (lines 29-32, part1's range — same stale claim, flagged for A07).
Why it matters: the CLI's strongest-sounding MP4 evidence line is (1) vacuous in every recorded run and (2) structurally red against the code it claims to pin — anyone mounting the corpus gets a guaranteed failure that says more about the test's staleness than about the code, and the recorded "ok" inflates the MP4 lane's proven coverage. This is the exact shape W13-A02 proved for es.rs (their F-002), here poisoning the flagship differential.
What correct looks like: update TRUTHS for the h264 lanes to the real behavior (decode counts vs ffmpeg, 4:2:0; typed `h264.sw.unsupported` allowed for 4:4:4 lanes), and convert the silent skip to `#[ignore = "corpus-v4 not mounted"]` so absence shows as ignored, not ok.
Confidence: HIGH — Verification: read (dispatch chain pipeline_host.rs:346 → sw.rs:442-445/235-258; allowlist + cross-check asserts; committed-fixture differential proof) + baseline log 2814 + `ls /tmp/my-project/corpus-v4/media` (absent here). The exact per-lane pix_fmt mix of the untracked corpus instance the TRUTHS table targets is UNVERIFIED (instance not in the repo); irrelevant to the conclusion — both branches fail.

### F-002 [S3] 4 of 8 tests in this chunk are gated on an untracked sandbox path and report "ok" while asserting nothing (the loud skip cargo swallows)
Where: apps/cli/src/containers/mp4.rs:943-960 (gate), gated tests at 650-703, 962-1070, 1078-1103, 1105-1126
Code:
```
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
```
What is wrong: `corpus_iso_bmff_lanes_match_the_ffmpeg_truth`, `cover_art_m4a_opens_with_still_image_trak`, `mpeg2_tagged_lane_opens_and_refuses_the_codec_typed`, and `six_channel_aac_geometry_flows_but_the_decoder_refuses` all begin `let Some(dir) = corpus_dir() else { skip_loudly(); return; }` — with the corpus absent (proven absent in baseline by F-001's contradiction argument, and absent in this sandbox by ls) each records a green `ok` while executing zero assertions. The corpus is not in the repository (docs/corpus manifest records it under worktree paths; A02's es.rs audit cites the worklog: the corpus "lives untracked").
Why it matters: four vacuous "ok" lines in the recorded evidence for the MP4/MOV/M4A family; indistinguishable from real passes in every gate that reads exit codes. The `eprintln` skip note goes to stderr, which cargo captures and discards for passing tests.
What correct looks like: `#[ignore = "corpus-v4 not mounted"]` (shows as ignored in summaries) or a committed minimal corpus lane; the es.rs A02 audit reached the same conclusion.
Confidence: HIGH — Verification: read + ls + the F-001 proof-by-contradiction + baseline log 2814/2815/2821/2822 ("ok" for all four).

### F-003 [S3] The dOps "unknown version refuses" assertion passes for the wrong reason — the version gate has no test that fails if the gate is deleted
Where: apps/cli/src/containers/mp4.rs:1149-1152
Code:
```
        assert!(
            opus_head_of_dops(&[0x01, 0x02]).is_none(),
            "unknown version"
        );
```
What is wrong: `opus_head_of_dops` rejects this input at its FIRST check — `if dops.len() < 11 || dops[0] != 0` (mp4.rs:409, part1) — and `[0x01, 0x02]` has length 2, so the length gate fires before the version byte is ever read. A mutant that deletes only the `dops[0] != 0` version check still returns `None` here and the whole suite stays green; the "never a guess for future dOps versions" behavior (the function's own doc, part1) is unfalsifiable.
Why it matters: the version gate is the function's only forward-safety behavior (a future dOps v1 must refuse, not be misparsed); it is currently pinned by nothing.
What correct looks like: an 11+-byte input with `dops[0] == 0x01` asserting `None` (e.g. the fam1 vector with its first byte flipped).
Confidence: HIGH — Verification: read of the test input and the guard order (409).

### F-004 [S4] The `--start-time` watermark assert silently skips Missing-PTS packets
Where: apps/cli/src/containers/mp4.rs:590-595
Code:
```
                    seen += 1;
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
```
What is wrong: only `Stamp::Present` packets are checked against the skip watermark; a regression that emitted `Stamp::Missing` PTS would walk every early packet past the assert (`seen > 0` still counts them, so the tail assertion also holds).
Why it matters: the watermark is this test's entire point; the hole is narrow (the MP4 demuxer currently always stamps) but the test would not notice the stamp loss.
What correct looks like: assert `packet.pts` is `Present` for every packet in an MP4 lane (stamps are declared), or count Missing stamps and assert zero.
Confidence: HIGH — Verification: read.

## 4. Per-function table (test items; production functions are part1 = A07's range)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `tests::start_time_skips_packets_and_reports_indexed_seek` | 573-603 | REAL | S4 (F-004) | Executed ok (log 2823); real watermark + clamp + seekability pins; Missing-PTS hole (F-004). |
| `tests::chapter_trak_is_hidden_and_markers_stay_demuxer_side` | 605-643 | REAL | — | Executed ok (log 2813); honest about the open path's chapter gap (module docs record it). |
| `tests::cover_art_m4a_opens_with_still_image_trak` | 645-703 | REAL-UNPROVEN | — (gated, F-002) | Corpus-gated → vacuous ok (log 2815). Would decode the cover packet through the real provider (mjpeg) — genuinely strong IF the corpus existed. |
| corpus differential comment block | 705-715 | — | — | Contains the stale "h264 … the hw-path cards own those seams" premise (F-001). |
| `struct Lane` | 717-728 | REAL | — | Data shape; `audio_frames`/`video_frames` Option discipline checked by the cross-asserts at 1060-1068. |
| `const TRUTHS` | 730-941 | PARTLY STALE | S2 (F-001) | 31 lanes; packet counts verified consistent with crates/demux/tests/corpus_mp4.rs on 0008/0060/0105; the h264 rows' `video_frames: None` encode the pre-h264-sw world; recorded mp3-lane divergence (37,487 vs ffmpeg 36,911) is honestly documented in-place (877-882). |
| `tests::corpus_iso_bmff_lanes_match_the_ffmpeg_truth` | 962-1070 | FALSE (as-asserted) / vacuous-as-run | S2 (F-001) | See F-001. Also: `counts[p.track.handle() as usize]` is an unchecked index — panics if handles were non-contiguous; unexercised (corpus-gated). |
| `tests::mpeg2_tagged_lane_opens_and_refuses_the_codec_typed` | 1072-1103 | REAL-UNPROVEN | — (gated, F-002) | Corpus-gated → vacuous ok (log 2821). Its `cli.codec_unavailable` assertion is CORRECT against current code ("mpeg4video" hits the provider's `other` arm). |
| `tests::six_channel_aac_geometry_flows_but_the_decoder_refuses` | 1105-1126 | REAL-UNPROVEN | — (gated, F-002) | Corpus-gated → vacuous ok (log 2822); pins (6, 12_000) geometry + `aac.engine.init`. |
| `tests::dops_synthesizes_the_rfc7845_opus_head` | 1128-1153 | REAL | S3 (F-003) | Executed ok (log 2816); version gate uncovered (F-003). |
| `tests::esds_dsi_walk_extracts_the_asc` | 1155-1175 | REAL | — | Executed ok (log 2817); hand-verified chain arithmetic here; truncation refusal pinned. |
| `tests::corpus_dir` | 943-952 | REAL | — (F-002 context) | The gate itself; `lumen-allow-literal` annotations present and justified (lint-literals gate passes on this site). |
| `tests::skip_loudly` | 954-960 | REAL | — (F-002 context) | eprintln-only skip note — swallowed by cargo on success. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| start_time_skips_packets… | YES (narrowly) | A broken watermark lets an early Present packet violate `ns.ticks >= 1_000_000`; but a Missing-PTS regression is invisible (F-004). |
| chapter_trak_is_hidden… | YES | Track-count and per-track packet counts pin the hidden-trak filtering; chapters count pins the demuxer side. |
| cover_art_m4a… | AS-RUN: NO (skipped) | Corpus-gated (F-002); if run: YES (decoder + counts). |
| corpus_iso_bmff_lanes… | AS-RUN: NO (skipped); IF RUN: FAILS EVEN ON CORRECT CODE | F-001 — the assertion set is unsatisfiable against the current h264 dispatch. |
| mpeg2_tagged_lane… | AS-RUN: NO (skipped) | Corpus-gated (F-002); if run: YES. |
| six_channel_aac… | AS-RUN: NO (skipped) | Corpus-gated (F-002); if run: YES. |
| dops_synthesizes… | YES, except the version gate | Field re-packing and table copy are pinned; the version-refusal mutation survives (F-003). |
| esds_dsi_walk… | YES | Exact DSI slice + truncation refusal pinned; hand-verified. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the honest typed refusal where none is (h264/hevc/mpeg4video — the hw-path cards own those seams)" | this file 712-714 | FALSE (for h264) | pipeline_host.rs:346 dispatches H264SwDecoder; sw.rs:442-445 configure-Ok; the committed h264_sw_differential decodes six MP4/avcC lanes (F-001). |
| "No software h264/hevc decoder is dispatched in this build … `decoder_for` refuses the codec typed (`cli.codec_unavailable`)" | module docs, this file 29-32 (part1's range — flagged for A07) | FALSE (for h264) | Same proof; hevc/mpeg4video claims remain true. |
| "decode-count assertions where a decoder is dispatched (aac ×1024 …, mjpeg one frame per packet, PCM's own sample extent)" | this file 709-712 | REAL-UNPROVEN | The machinery exists; no recorded run executed it (F-001/F-002). |
| Baseline INDEX "the one real end-to-end lane (MJPEG+PCM in AVI)" | INDEX.md:78 | REAL | rc-smoke-engine-only.log.txt:21-24 — consistent with this table's mjpeg/pcm rows being the only fully-proven decode lanes. |

## 7. Dead / duplicated / unreachable code
- The corpus-gated quartet is live code that no recorded environment executes (F-002) — dead-in-evidence, not dead-in-repo.
- No other dead code in the chunk.

## 8. Dependencies & flags
- Test-only deps: testkit (`Must`), std::env/fs, demux::mp4::test_support (mem_stream — used by the chapter test's direct construction). `lumen-allow-literal` annotations at 946/950/956 are present and pass the lint-literals gate (baseline #13 fails only on host.rs:1799). No feature gates; `#[cfg(test)]` only.

## 9. Open questions for the next agent
- A07 (part1 = production code): the module-doc h264 claim (lines 29-32) is stale — see F-001; also please confirm `child_header`'s `pos + size` overflow discipline (u32→usize on 64-bit is safe; on 32-bit a hostile size could wrap — worth a row in your audit).
- Orchestrator: F-001's proof-by-contradiction would be even harder with a mounted corpus: REQUEST-ORCH-RUN: `LUMEN_CORPUS=<path> cargo test -p lumen-cli --lib containers::mp4 -- --nocapture` with any corpus-v4 instance (expected: corpus differential FAILS on the first h264 lane — turning F-001 from read-proven to run-proven). Corpus media is not in the repo; if unavailable, F-002's #[ignore] recommendation stands on its own.
