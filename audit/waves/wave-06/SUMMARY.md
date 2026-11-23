# WAVE 06 SUMMARY — video codecs, codec-api, renderer-wgpu

| Field | Value |
|---|---|
| Agents | A02/A06/A09 clean; A01/A03/A04/A05/A07/A08/A10 infra-timeouts mid-run (123/158 files done before death); recovery batches R1-R3 completed 35 remaining; coverage 100% |
| Findings | S0:0 S1:0 S2:8+ S3:~40 S4:~90 (from completed audit headers; per-file detail in audit/files/) |
| Tree purity | clean |

## Headline answers (the wave's hostile questions)
1. H.264 decode IS reachable and REAL in the shipped build: CLI avcC path ->
   rust_h264 (external crate) decodes; the in-tree parser (sps/pps/slice/
   nal) is spec-conformant but UNREACHABLE — the CLI never constructs
   H264Decoder (W06-A02). 4:2:2/10-bit support unverified (crate-side).
2. renderer-wgpu is a NAME LIE: zero wgpu calls in the crate — a software
   compositor seam (disclosed in Cargo.toml comments, hidden in the name)
   (W06-A02/R1).
3. codec-api encode.rs: NO encoder exists anywhere — mock-only contract,
   zero implementations, transcode refuses honestly (W06-A02).
4. Hardware decode (VAAPI/VideoToolbox): VAAPI bridge IS wired into the
   shipped Linux build (constants ground-truthed against installed
   libva.so.2), but `--playback-hw-decode-policy` flips NOTHING (from_setting
   test-only) — the policy knob is dead in the shipped CLI (W06-A06/A09).
   VideoToolbox: cfg(macos), zero tests, 44 unsafe blocks SAFETY-commented;
   NULL-callbacks claim dubious (possible UAF) (W06-A06).
5. VP9/AV1: AV1 decoder REAL and CLI-reachable via .ivf — rav1d wrapper,
   8-bit 4:2:0 only, pixel-MD5-vs-ffmpeg 6/6 (W06-A09). VP9 decoder real
   (W06-A08). DNxHD: REAL decoder with zero producers (no demuxer mints
   dnxhd) but apps/web capabilities advertise it anyway (W06-A09).
6. mpeg12: real tree, production-unwired, "registered fuzz_codec_mpeg2"
   nonexistent, decode-blocked error never surfaces (W06-R3).

## Worst findings
1. [S2, executed] Shipped-CLI video FREEZE defect: WindowFull arm stashes
   UNCONVERTED I420 -> renderer refuses -> surface freezes (registered
   defect at pipeline_host.rs:1368-1372, shipped unfixed) (W06-A02).
2. [S2, executed] vorbis_bench fallback lane PANICS (66!=67), hidden by
   #[ignore] (W06-A09).
3. [S2, executed] benches.rs dense-4K probe FAILS its own debug floor
   (0.4 fps < 1.0); #[ignore]d — the 4K60 release claim enforced nowhere
   (W06-A06).
4. [S2] slice.rs's 4 ffmpeg-trace/mutation tests silently skip (corpus
   absent) — headline claims never execute; baseline "4744 passed" counts
   them as vacuous passes (W06-A02).
5. [S2] videotoolbox pool_exhausted stall + frame drops (no present-side
   surface release) (W06-A06).
6. [S2] av1_conformance.rs is NOT AOM conformance vectors — self-generated
   matrix, /tmp name-keyed cache = stale-spec tautology risk (W06-A06).
7. [S3 cluster] duplicate Exp-Golomb bitreaders (h264 slice/sei vs nal);
   stale doctest inventory (video/mod.rs line refs rotted); "conformance"
   naming lies; dead simd seams in mjpeg color.

## Note for Wave 16/20
The corpus-v4 absence makes ~10 differential lanes vacuous-green across
codecs (slice, mpeg12, adpcm, hevc parser, opus, mp3 corpus halves). The
"4744 passed" headline number includes these no-ops.
