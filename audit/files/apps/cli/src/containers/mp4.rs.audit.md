# AUDIT: apps/cli/src/containers/mp4.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-24 |
| Lines / functions | 1177 (incl. ~747 test lines) / 18 non-test fns + 15 test fns |
| File verdict | REAL (open path executed end-to-end; one real codec-config defect in the opus lane) |
| Reachable from shipped entry point? | YES: main.rs → commands open → demux_host::open_source (demux_host.rs:362) → open_source_with → containers::open (containers/mod.rs:67 `"mp4" => mp4::open`) → mp4::open (:95). Executed via /home/z/my-project/bin/lumen on samples/sample.mp4 (exit 0, tracks `#0 video h264 320x240`, `#1 audio aac 1ch 44100Hz`, duration 3.000000s, seekable, Playing). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | lumen engine-info (0); lumen decode-bench sample.mp4 / sample-hevc.mp4 (0, both "not benched"); lumen open sample.mp4 / sample-faststart.mp4 --start-time 2s / sample-aac-only.mp4 (all 0); strings bin/lumen → rust_h264-0.6.1 paths; ./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::mp4 (11 passed, 0.01s); same --nocapture (4 loud corpus SKIPs); ./target/debug/deps/container_conformance-f03491b3772927ce mp4_h264_family_lane (1 passed). |

## 1. One-paragraph verdict
The MP4/MOV/M4A open path is real and works: box walk, stsd/avcC harvest, esds→ASC extraction, geometry/dimension harvest, start-time clamp + pure seek query all execute through the shipped CLI (proven on 3 sample files and the committed fixtures). The h264 wiring is REAL: `avcC` at :237 flows as `extra_data` into `H264SwDecoder` (rust_h264 0.6.1) via pipeline_host.rs:346, and the aac ASC flows into `AacDecoder`; the conformance lane `mp4_h264_family_lane` passes end-to-end (`time 1.532517s (100.0%)`, no `cli.codec_unavailable`). But the module documentation is FALSE on the h264 point (claims a typed refusal that no longer exists), the corpus differential table asserts that stale refusal and would FAIL on any corpus-equipped machine, and `opus_head_of_dops` fabricates a malformed OpusHead (no big-endian→little-endian swap) that its own unit test enshrines. Fragmented MP4 and edit lists are NOT refused (corpus lanes + mp4_edts fixture, demuxer-side moof-scan); what IS refused: h264→ nothing anymore, hevc/mpeg4video/mpeg2video decode (`cli.codec_unavailable`), >2-channel AAC (`aac.engine.init`), unknown dOps version.

## 2. What actually works (verified)
— `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::mp4` → 11 passed / 0 failed (fixture-backed tests: avcC[0]==1, `asc[0]>>3==2`, pcm pair (1,44100), edts/mov open, start-time skip, chapters hidden).
— `./target/debug/deps/container_conformance-f03491b3772927ce mp4_h264_family_lane` → 1 passed: h264+AAC fixture decodes to end-of-media at 100% position, no codec refusal.
— `bin/lumen open samples/sample.mp4` (exit 0): `#0 video h264 320x240 (selected)`, `#1 audio aac 1ch 44100Hz` — visual_dimensions (:395) and audio_geometry (:386) verified through the real binary. faststart + `--start-time 2s` → Playing (seek path exercised); aac-only opens.
— `strings bin/lumen | rg rust_h264` → rust_h264-0.6.1 decoder sources present in the shipped binary.

## 3. Findings
### F-001 [S2] `opus_head_of_dops` never byte-swaps BE→LE; the synthesized OpusHead is malformed and its own test pins the bug
Where: apps/cli/src/containers/mp4.rs:412-421
Code (sed-verified):
```rust
    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(b"OpusHead");
    head.push(1); // OpusHead version
    head.push(dops[1]); // channel count
    head.extend_from_slice(&dops[2..4]); // pre-skip (LE)
```
What is wrong: ISO/IEC 23003-5 `dOps` fields are big-endian; RFC 7845 OpusHead fields are little-endian. The module doc (:33-37) itself promises "dOps' big-endian fields re-packed little-endian, ffmpeg's `ff_mov_rewrite_dops` mapping" — the code copies the BE bytes verbatim with no swap. The consumer is real: crates/codecs/src/audio/opus/head.rs:101-103 parses `u16::from_le_bytes([blob[10], blob[11]])` (pre-skip) and `u16::from_le_bytes([blob[16], blob[17]])` (output gain), decoder.rs:307 turns gain into `10.0f32.powf(gain/5120.0)`, decoder.rs:403/460 subtract pre-skip from the timeline. A dOps pre-skip of 312 (0x0138 BE, the libopus default — head.rs's own test uses 312) is read back as 0x3801 = 14337 samples (~299 ms sync shift); a gain of −3 dB (0xFFC4 BE) is read as 0xC4FF = −15105 → ×0.0012 (≈ −58 dB, near-silence). Why it matters: every ffmpeg-muxed opus-in-MP4 with non-palindromic pre-skip/gain plays desynced and/or at the wrong volume. The unit test :1140-1141 asserts the unswapped bytes (`assert_eq!(&head[10..12], &[0x00, 0x38], "pre-skip LE")` — 0x0038 read LE = 14336, not 56), so the test would FAIL if the code were fixed: it proves the wrong value. No corpus lane covers opus-in-MP4 (TRUTHS has no dOps lane), so nothing else would catch it. What correct looks like: `u16::from_be_bytes` → `to_le_bytes` for pre-skip/gain, `u32` for input rate (rate is advisory but still wrong). Confidence: HIGH — Verification: read (both producer and consumer); end-to-end playback UNVERIFIED (no opus-in-mp4 sample in /home/z/my-project/samples).

### F-002 [S3] Module doc claims no software h264 decoder is dispatched — FALSE since the 2026-11-15 rust_h264 landing
Where: apps/cli/src/containers/mp4.rs:27-32
Code (sed-verified):
```rust
//! * **h264/hevc `extra_data` = the `avcC`/`hvcC` child payload**
//!   (the config record WITHOUT the box header — ffmpeg's extradata
//!   shape). No software h264/hevc decoder is dispatched in this
//!   build: the blob is the forward-honest config, and
//!   `decoder_for` refuses the codec typed (`cli.codec_unavailable`)
```
What is wrong: apps/cli/src/pipeline_host.rs:346 is `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),` — the avcC blob harvested at :237 seeds a real rust_h264 0.6.1 software decoder, and the conformance lane decodes h264 to 100% (§2). Only hevc/mpeg4video remain refused. The doc asserts the pre-landing state as current truth. Why it matters: this module doc is the file's own "decoder-config truth table"; half of it describes a build that no longer exists. What correct looks like: h264 split out of that bullet (hevc keeps the refusal sentence). Confidence: HIGH — Verification: read + ran (mp4_h264_family_lane passed; strings shows rust_h264-0.6.1).

### F-003 [S3] Corpus differential table is stale: it asserts h264 typed refusals that the current dispatch contradicts — the test would FAIL on any corpus-equipped machine
Where: apps/cli/src/containers/mp4.rs:1001-1016 and 1063-1068 (with TRUTHS :730-737 all-h264 lanes `video_frames: None`)
Code (sed-verified):
```rust
                        // The honest typed refusals: the hw-path
                        // codecs (no software decoder dispatched)
                        // and the AAC engine's own mono/stereo
                        // boundary for the 6-channel lanes (5.C2a's
```
and
```rust
            if lane.video_frames.is_none() {
                assert!(video.is_none(), "{name}: no video decoder expected");
            }
```
What is wrong: pass 2 calls `decoder_for` per track; for h264 it now returns `Ok(H264SwDecoder)` (pipeline_host.rs:346), so `video = Some(..)` and the `assert!(video.is_none())` cross-check fails for every h264 lane (0008, 0021, 0025, 0027, 0032, 0049, 0058, 0060, 0062, 0069, 0074, 0082, 0105, 0109, 0112, 0140, 0122, 0011, 0061…). The test passes today ONLY because `/tmp/my-project/corpus-v4/media` is absent — verified: `--nocapture` printed `corpus(mp4-open): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset — SKIP` for all 4 corpus-gated tests. The 31-lane table therefore proves nothing in this tree and would regress-fail on a corpus machine instead of measuring the new decodes. What correct looks like: h264 lanes re-measured with `video_frames: Some(n)` (as 0115-mjpeg/0079-mjpeg already do). Confidence: HIGH on the contradiction (static: dispatch arm vs assert), UNVERIFIED by execution (corpus absent — cargo-build-budget does not apply; the corpus files simply do not exist here).

### F-004 [S3] `dsi_of_esds` ignores the ES_Descriptor flags: streamDependence/URL/OCR entries fail the ASC walk
Where: apps/cli/src/containers/mp4.rs:357-358
Code (sed-verified):
```rust
    at = expandable_len(body, at + 1)?.0;
    at += 3; // ES_ID + flags
    if body.get(at) != Some(&0x04) {
```
What is wrong: ISO/IEC 14496-1 §7.2.6.5: after ES_ID+flags, `streamDependenceFlag` (0x80) adds a 2-byte dependsOn_ES_ID, `URL_Flag` (0x40) adds URLlength+URL, `OCRstreamFlag` (0x20) adds a 2-byte OCR_ES_ID. The unconditional `at += 3` misaligns the walk for any such entry → `None` → no `extra_data` → the AAC/MP4V decoder is seeded with nothing. Rare in ffmpeg/Apple output but legal ISO-BMFF. What correct looks like: read the flags byte and skip the optional fields. Confidence: MEDIUM — Verification: read (no such fixture available to execute).

### F-005 [S4] Module doc's audio-geometry offsets contradict the implementing code
Where: apps/cli/src/containers/mp4.rs:43-44 vs :386-389
Code (sed-verified):
```rust
//!   entry's own declaration — `channelcount` at offset 12, the
//!   16.16 `samplerate`'s integer half at offset 20 (the fields
```
vs
```rust
fn audio_geometry(entry: &[u8]) -> Option<(u32, u32)> {
    let channels = u16be(entry, 16)?;
    let rate = u16be(entry, 24)?;
```
What is wrong: the doc says 12/20; the code (correct for the post-8-byte-header payload the demuxer stores — crates/demux/src/mp4/mod.rs:1191 `entry = body.get(16..8+entry_size)`, "payload handed downstream starts after it") reads 16/24, matching the fn's own doc comment and the verified output (`1ch 44100Hz`). The module doc numbers match no layout. Confidence: HIGH — Verification: read + ran (bin/lumen info output).

### F-006 [S4] `find_child_box` and `child_box_full` are copy-paste duplicates
Where: apps/cli/src/containers/mp4.rs:271-288 vs 291-308
Code (sed-verified, the two bodies differ only in the returned slice):
```rust
            if kind == *magic {
                return entry.get(pos + 8..pos + size);
            }
```
vs
```rust
            if kind == *magic {
                return entry.get(pos..pos + size).map(|bytes| bytes.to_vec());
            }
```
What is wrong: identical candidate-start walk duplicated; a fix to one (e.g. version-1 entries, children at 36) will miss the other. Confidence: HIGH — Verification: read.

### F-007 [S4] `open()` clones the track list twice into the wrapper and the facts
Where: apps/cli/src/containers/mp4.rs:123-129 vs :221-223
Code (sed-verified): `tracks: tracks.clone(),` (into StartedMp4) and `MediaFacts { tracks: tracks.to_vec(), … }` — the same Vec copied twice per open. Cosmetic cost only. Confidence: HIGH — Verification: read.

### F-008 [S4] Module doc overclaims the clamp's guarantee for fragment-only files
Where: apps/cli/src/containers/mp4.rs:65-67
Code (sed-verified, lines 65-67):
```rust
//!   (`SlowLinear`). The demuxer's `seek` refuses past-the-end
//!   targets typed — the clamp above means the open-time query
//!   never asks for one.
```
What is wrong: for fragment-only files `duration_us()` is `None` → clamp bound is `i64::MAX/4` (:106), so a past-the-end `--start-time` IS asked and answered by the demuxer's typed refusal — the sentence is only true for duration-declaring files. Confidence: HIGH — Verification: read (:103-107).

## 4. Per-function table (non-test; every function)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 95-143 | REAL | — | executed via bin/lumen (3 samples + fixtures); clamp/seek wiring verified |
| StartedMp4::start | 161-163 | REAL | — | cached pre-pass answer; adapter is honest about the demuxer's second-call refusal |
| StartedMp4::next_event | 164-166 | REAL | — | verbatim forward |
| StartedMp4::seek | 167-169 | REAL | — | verbatim forward |
| StartedMp4::seekability | 170-172 | REAL | — | `Fast{indexed:true}` asserted by test :581-584 (passed) |
| StartedMp4::reanchor | 173-178 | REAL | — | forwards the demuxer's typed refusal, does not invent |
| harvest_facts | 186-226 | REAL | — | geometry/dims/blob per track; verified via binary `info` |
| config_blob | 229-250 | REAL | — | per-codec dispatch; h264/aac lanes proven end-to-end |
| u16be / u32be | 253-262 | REAL | — | bounds-checked via `get` |
| find_child_box | 271-288 | REAL | — | candidates 28/76/78 correct for the stored payload; dup body (F-006) |
| child_box_full | 291-308 | REAL | — | ALAC cookie shape; dup body (F-006) |
| child_header | 312-326 | REAL | — | overrun-checked; no overflow on 64-bit (size ≤ u32) |
| dsi_of_esds (+expandable_len) | 336-370 | SHALLOW | S3 | flags-blind walk (F-004); correct for flags=0 (unit test passed) |
| asc_of_entry | 375-379 | REAL | — | composed of the two above; LC ASC verified via test + conformance |
| audio_geometry | 386-390 | REAL | — | 16/24 correct; verified `1ch 44100Hz`; version-1 entries misread (unverified, folded into F-004 scope note) |
| visual_dimensions | 395-399 | REAL | — | 24/26 correct; verified `320x240` on sample.mp4 |
| opus_head_of_dops | 408-428 | FALSE | S2 | no BE→LE swap (F-001); test enshrines it |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| h264_aac_fixture_opens_with_expected_facts | YES | real fixture, real open path, asserts avcC byte 1, ASC AOT 2, exact geometry/dims; ran → ok |
| mov_and_edts_fixtures_open | YES | real .mov + edts fixtures, 2 tracks + declared duration; ran → ok |
| mjpeg_pcm_fixture_opens_with_pcm_facts | YES | asserts pcm pair (2,44100) and no blobs; ran → ok |
| start_time_skips_packets_and_reports_indexed_seek | YES | walks every packet asserting pts ≥ 1s; ran → ok |
| chapter_trak_is_hidden_and_markers_stay_demuxer_side | YES | counts packets on both traks + demuxer chapters == 3; ran → ok |
| cover_art_m4a_opens_with_still_image_trak | vacuous here | corpus-gated → SKIP (verified loud skip); with corpus it decodes the cover (would fail if broken) |
| corpus_iso_bmff_lanes_match_the_ffmpeg_truth | NO — worse: fails if code is CURRENT | corpus-gated → SKIP; statically contradicts the h264 dispatch (F-003) |
| mpeg2_tagged_lane_opens_and_refuses_the_codec_typed | vacuous here | corpus-gated → SKIP; body still matches current behavior (mpeg4video refused) |
| six_channel_aac_geometry_flows_but_the_decoder_refuses | vacuous here | corpus-gated → SKIP |
| dops_synthesizes_the_rfc7845_opus_head | NO — inverted | asserts the un-swapped BE bytes; FAILS against a correct implementation (F-001) |
| esds_dsi_walk_extracts_the_asc | YES | hand-built chain + truncation refusal; ran → ok |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No software h264/hevc decoder is dispatched in this build … refuses the codec typed" | mp4.rs:29-32 | FALSE (h264 half) | pipeline_host.rs:346; mp4_h264_family_lane passed; rust_h264 strings in bin/lumen |
| "dOps' big-endian fields re-packed little-endian" | mp4.rs:35-36 | FALSE | :416-421 copies verbatim; head.rs:101-103 reads LE |
| "channelcount at offset 12 … samplerate … offset 20" | mp4.rs:43-44 | FALSE | code :387-388 reads 16/24; verified output 1ch/44100Hz |
| "the clamp above means the open-time query never asks for one" | mp4.rs:66-67 | FALSE for fragment-only | :106 `unwrap_or(i64::MAX / 4)` |
| "AAC extra_data = the esds AudioSpecificConfig … the whole-entry blob would not parse" | mp4.rs:17-23 | REAL | test asserts `asc[0]>>3==2`; conformance decodes aac to 100% |
| "avcC … config record WITHOUT the box header — ffmpeg's extradata shape" | mp4.rs:27-28 | REAL | test asserts `avcc[0]==1`; H264SwDecoder seeded end-to-end |

## 7. Dead / duplicated / unreachable code
None dead. Duplicates: find_child_box/child_box_full (F-006), double track clone (F-007). All code is reachable from the `mp4` registry arm.

## 8. Dependencies & flags
No feature gates inside this file (plain `#[cfg(test)]` only). Cross-file: the real decode arms live in pipeline_host.rs behind no feature flag (release lane builds them — release lane per AGENT-SPEC §7); bench.rs:334 and host.rs engine_info (:1643-1657) still hardcode the mjpeg+pcm-only story, contradicting pipeline_host in the SAME shipped binary (decode-bench on sample.mp4 prints "not benched (no decoder in this build (first vertical: mjpeg + pcm))" while `open` decodes both tracks — the lie is bench/engine-info's, not mp4.rs's; owners: bench/host files).

## 9. Open questions for the next agent
- bench.rs / host.rs engine_info hardcode "mjpeg, pcm" — confirm their owning agents file the registry-vs-dispatch contradiction (my probes: §2).
- Is any opus-in-MP4 fixture planable for the corpus? F-001 needs an end-to-end pin.
- audio_geometry/find_child_box ignore version-1 AudioSampleEntry layout (children at 36, 32-bit rate) — unverified, no fixture; demuxer crate may own the equivalent question.
