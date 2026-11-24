# AUDIT: apps/cli/src/containers/mkv.rs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-24 |
| Lines / functions | 1303 / 7 prod fns + 10 tests (+4 helpers) |
| File verdict | REAL (adapter + blob harvest executed via fixtures; module doc and corpus differential are STALE on h264) |
| Reachable from shipped entry point? | YES: `lumen open` → host.rs:904 `open_source` → demux_host.rs:378 `open_source_with` → demux_host.rs:458 `containers::open` → containers/mod.rs:57 `"matroska" => mkv::open` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/lumen_cli-9887c2f6df9cd333 containers::mkv` → 10 passed, exit 0; same `--nocapture \| rg SKIP` → 4 SKIP lines; `./target/debug/deps/unsupported_path-7b9ed041c8c263b5 matroska_container_opens_and_decodes_h264` → 1 passed, exit 0; rg/sed verifications exit 0 |

## 1. One-paragraph verdict
The adapter itself is real and executed: open/harvest/wrap runs against four committed fixtures through the real CLI open path, and the blob grammar (Vorbis Xiph lacing, FLAC STREAMINFO, verbatim avcC/ASC/cookie/OpusHead) is unit-pinned. The hostile-focus answers: V_MPEG4/ISO/AVC is mapped to the `"h264"` descriptor by the demuxer (`crates/demux/src/mkv/header.rs:784` `codec_id.starts_with("V_MPEG4/ISO/AVC")`), and this file wires it to a decoder by passing the CodecPrivate (avcC record) VERBATIM as `extra_data` (mkv.rs:251), which `HostDecoderProvider` seeds into `H264SwDecoder` (pipeline_host.rs:346) — MKV h264 DECODES in this build (executed, see §2), contradicting the module doc. Subtitles: the demuxer mints them, `SelectDemuxer::start` filters non-A/V kinds from the engine's track list (deselect at open — Wave 9's claim CONFIRMED), but their packets still flow (executed: 2 subrip packets emitted). The 34-lane corpus differential is dead weight in this tree: the corpus is absent, 4/10 tests skip while cargo counts them "ok", and the differential's h264 expectations are stale — on a corpus machine the test FAILS.

## 2. What actually works (verified)
— `./target/debug/deps/lumen_cli-9887c2f6df9cd333 containers::mkv` → `test result: ok. 10 passed; 0 failed` (h264_aac facts, flac full-stream decode 20 pkt→88200 frames, multitrack subtitle deselect counts `[20, 88, 20, 2]`, webm vp9 20 frames + opus 96648 frames, vorbis lacing + flac streaminfo units). 4 of the 10 printed `corpus(mkv-open): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset — SKIP` under `--nocapture` (corpus absent: `ls /tmp/my-project/corpus-v4/media` → No such file or directory).
— `./target/debug/deps/unsupported_path-7b9ed041c8c263b5 matroska_container_opens_and_decodes_h264` → 1 passed: the committed mkv_h264_aac fixture opens end-to-end via the `lumen open` binary path with the h264 limb decoding (no `cli.codec_unavailable`).

## 3. Findings
F-001 [S2]
Where: apps/cli/src/containers/mkv.rs:1146-1160 and 1224-1232 (with 789-1079 TRUTHS)
Code: (L1147-1150) `// The honest typed refusals: the hw-path and\n// unlanded codecs (h264/hevc/mpeg4/vp8/vp9/\n// ffv1/ac3, the subtitle vocabulary) and the\n// AAC engine's mono/stereo boundary for the` … (L1230-1232) `if lane.video_frames.is_none() {\n    assert!(video.is_none(), "{name}: no video decoder expected");\n}`
What is wrong: The corpus differential pins every h264 lane (0003/0007/0009/0010/0014/0017/0023/0024/0043/0053/0057/0063/0083/0096/0120/0121/0133/0149 all `video_frames: None`) as a typed `cli.codec_unavailable` refusal. Since the 2026-11-15 H.264 software landing, `HostDecoderProvider::decoder_for` dispatches `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (pipeline_host.rs:346), so `decoder_for` returns Ok for those lanes, `video` becomes Some, and the L1230-1232 cross-check fails. Why it matters: on a corpus-enabled machine this test FAILS; in this tree it passes vacuously (corpus absent, `skip_loudly()` returns normally) and is counted in the baseline's 4744 passes — stale evidence that cannot fail here. What correct looks like: re-measure the h264 lanes with a decoder (frame counts) or delete the stale refusal pin; gate corpus tests on `#[ignore]`/an explicit skip that cargo reports. Confidence: HIGH — Verification: ran (fixture twin `matroska_container_opens_and_decodes_h264` passes; corpus lanes unrunnable here — corpus absent, marked UNVERIFIED for the corpus lanes themselves).

F-002 [S3]
Where: apps/cli/src/containers/mkv.rs:52-58
Code: `//! * **h264/hevc `extra_data` = the avcC/hvcC config record\n//!   verbatim** (measured on `0007`/`0035`: the private starts with\n//!   avcC `configurationVersion` 1 — ffmpeg's extradata shape). No\n//!   software h264/hevc decoder is dispatched in this build: the\n//!   blob is the forward-honest config, and `decoder_for` refuses\n//!   the codec typed (`cli.codec_unavailable`) — the hw-path cards\n//!   own the seam.`
What is wrong: The claim is FALSE for h264 (it remains true for hevc): this build dispatches `H264SwDecoder` seeded with the very avcC blob this module harvests, and the committed fixture decodes (§2, executed). Why it matters: a reader of the CLI adapter would believe MKV h264 refuses decode; the same staleness drives F-001's dead expectations. What correct looks like: split the claim — h264 dispatches (2026-11-15 landing), hevc/mpeg4/vp8/ffv1/ac3 refuse typed. Confidence: HIGH — Verification: ran (`matroska_container_opens_and_decodes_h264` exit 0).

F-003 [S3]
Where: crates/demux/src/mkv/header.rs:832-840 (gap recorded at apps/cli/src/containers/mkv.rs:73-79)
Code: `if codec_id.starts_with("A_PCM/INT/LIT") {\n    return match bit_depth {\n        Some(8) => "pcm_u8".to_string(),\n        Some(16) => "pcm_s16le".to_string(),` … `_ => codec_id.to_string(),\n    };`
What is wrong: ffmpeg's matroska muxer omits `BitDepth`, so A_PCM/INT/LIT tracks pass through with the raw CodecID string and the provider refuses typed — PCM-in-MKV from the most common muxer does not decode. Why it matters: realistic input, no decode; the module doc admits it ("this corpus instance carries none"), so this is an honestly-recorded demuxer-domain gap, not a cover-up. What correct looks like: default the LIT mapping to s16le when the Audio master's channel/rate pair is present (or harvest a raw descriptor). Confidence: HIGH — Verification: read (demuxer mapping) + module-doc admission; no corpus PCM lane here to execute (UNVERIFIED end-to-end).

F-004 [S4]
Where: apps/cli/src/containers/mkv.rs:762-767
Code: `// emission windows), and the honest typed refusal where none is\n// (h264/hevc/mpeg4/vp8/vp9/ffv1/ac3 — the hw-path and unlanded\n// codec cards own those seams; the 6-channel aac lanes hit the`
What is wrong: Lists vp9 among "no decoder" refusals, contradicting the module doc itself (L68-72 "vp9 DECODES (card 6.M-vp9 landed)"), lanes 0075/0141 (`video_frames: Some(76/52)`), and the executed webm fixture test. Same stale-comment family as F-002. Confidence: HIGH — Verification: read + ran (webm_vp9_and_opus test passed).

F-005 [S4]
Where: apps/cli/src/containers/mkv.rs:1092-1098
Code: `fn skip_loudly() {\n    eprintln!(\n        // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable\n        "corpus(mkv-open): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \\\n         — SKIP (the differential table for this instance is in the worklog)"\n    );\n}`
What is wrong: The "loud" skip is silent in default harness runs — libtest captures stderr of passing tests, so without `--nocapture` the 4 corpus-gated tests report `ok` with zero output. Verified: bare run shows 10 `ok`; `--nocapture` reveals 4 SKIP lines. Why it matters: the pass count (incl. the orchestrator baseline) silently includes no-op tests. What correct looks like: `#[ignore = "corpus"]` + `--ignored` lane, or panic-based gating so absence is visible. Confidence: HIGH — Verification: ran both invocations.

## 4. Per-function table (production fns; tests below)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 142-187 | REAL | — | Demuxer construct + pre-pass + clamp (MAX/4 honest bound, MKV duration always None) + pure seek gate + wrap; executed via 4 fixture tests. `saturating_mul(1000)` guards us→ns. |
| `harvest_facts` | 195-233 | REAL | — | Linear `find` per track (O(n²), n≤dozens — negligible); missing entry silently yields no blob → decoder refusal (honest). |
| `config_blob` | 239-257 | REAL | — | h264 blob now feeds a REAL decoder (F-002 doc stale); `_ => None` for vp8/vp9/ffv1/av1/pcm/subs matches doc. |
| `vorbis_chain` | 268-313 | REAL | — | `checked_add`/`get` guards, magic-gated, exactly-3-headers; unit-executed incl. 0xFF accumulation and unterminated-lacing refusal. |
| `flac_streaminfo` | 320-334 | REAL | — | Type/length verified before slice; bare-34 fallback; unit-executed incl. wrong-type/lying-length refusals. |
| `audio_pair` | 340-347 | REAL | — | channels 1..=64, rate 0<..≤4.096 MHz, finite — no guess; `as u32` safe under the guards. |
| `dimensions_of` | 350-357 | REAL | — | Coded box only (rotation demuxer-side); u32-bounded cast guarded. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| h264_aac_fixture_opens_with_expected_facts | YES | Asserts avcC[0]==1, 5-byte ASC AOT-LC, geometry (1,44100), dims, frame_ns=100ms — executed. |
| flac_fixture_opens_and_decodes_the_full_stream | YES | 20 pkt → 88200 samples + channel count — executed. |
| multitrack_fixture_deselects_subtitles_at_open | YES | 4 facts, wrapper start()→3 A/V, counts [20,88,20,2] (subtitle packets flow) — executed. |
| webm_vp9_and_opus_decode_from_the_private_head | YES | vp9 20 frames, opus 101 pkt → 96648 samples, OpusHead magic — executed. |
| vorbis_lacing_strip_joins_the_measured_packing | YES | Pure unit on hand-built privates incl. negative cases — executed. |
| flac_streaminfo_extracts_the_verified_block | YES | Pure unit incl. wrong type/length — executed. |
| start_time_skips_packets_and_reports_indexed_seek | NO (here) | Corpus-gated: skips vacuously, counted "ok" (F-005). UNVERIFIED by execution. |
| six_channel_aac_geometry_flows_but_the_decoder_refuses | NO (here) | Corpus-gated skip (F-005). |
| corpus_matroska_lanes_match_the_ffmpeg_truth | NO — WORSE: would FAIL with corpus | Stale h264 refusal pin (F-001). |
| vorbis_lane_decodes_the_full_stream_from_the_laced_private | NO (here) | Corpus-gated skip (F-005). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No software h264/hevc decoder is dispatched in this build … refuses typed" | mkv.rs:55-58 | FALSE (h264) / REAL (hevc) | pipeline_host.rs:346 dispatch + executed unsupported_path test. |
| "Subtitles deselect at open … never block playback (pinned on the multitrack fixture)" | mkv.rs:108-111 | REAL | demux_host.rs:566-577 filter `_ => false`; executed multitrack test. |
| "av1 carries NO blob … decodes all 101 frames … zero seeding" | mkv.rs:63-67 | REAL-UNPROVEN | Corpus lane 0034 (`video_frames: Some(101)`), corpus absent here. |
| "PCM … maps to pcm_s16le ONLY when BitDepth declared" | mkv.rs:73-79 | REAL | header.rs:832-840 bit_depth match; honest gap (F-003). |
| "Cues → Fast{indexed:true}; 0083 answers SlowLinear" | mkv.rs:105-107 | REAL-UNPROVEN | Corpus-gated assertion (L1120), unrunnable here; MP4/other lanes don't cover MKV seekability. |

## 7. Dead / duplicated / unreachable code
None found in production paths. The TRUTHS table (789-1079) is currently unexecutable in-tree (corpus absent) and stale on h264 — dead evidence, see F-001.

## 8. Dependencies & flags
No feature gates in this file (plain `pub(crate)` module, always compiled into lumen-cli; containers/mod.rs:27 `mod mkv;` unconditional). Deps: demux, media_types, plugin_api, commands — all workspace crates. Test-only env knob `LUMEN_CORPUS` (L1085).

## 9. Open questions for the next agent
- pipeline_host.rs (owner: another agent): confirm whether `H264SwDecoder` is reachable from the shipped release lane build (baseline engine-info listed only mjpeg+pcm for the release `lumen` binary — which registry lied? engine-info vs provider dispatch).
- With a corpus present, run `corpus_matroska_lanes_match_the_ffmpeg_truth` to confirm the F-001 failure mode (predicted: `assert!(video.is_none())` fails on lane 0002? — no, 0002 is ac3-only; first failure expected on 0003 at the h264 track).
- native_seek.rs (next file in my list) must answer whether in-session seek works for MKV given MP4's S0.
