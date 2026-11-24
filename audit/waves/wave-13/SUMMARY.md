# WAVE 13 SUMMARY — apps/cli (open path) + apps/desktop (bridge)

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 55, 100% covered |
| Findings | S0:3 S1:5 S2:15 S3:~45 S4:~90 |
| Tree purity | clean |

## THE MP4 TRACE (final answer, executed end to end)
argv -> host.rs:890 open_session -> demux_host open_source_with (3 access
lanes: path / file: URL / http(s) ranged) -> probe -> containers::open
(15/15 probe ids have arms; 14 real demuxers + honest flac refusal) ->
mp4.rs sample_entry_bytes -> avcC (:237) / bare esds ASC (:375) ->
pipeline_host.rs:346 "h264"|"avc1" => H264SwDecoder (rust_h264 0.6.1,
avcC-seeded) + AacDecoder (symphonia-codec-aac) -> I420 -> RGBA
(BT.601/709 tables verified) -> headless software Renderer (BGRA8/Fifo)
-> NO WINDOW. MP4 open+decode+play-to-null WORKS (executed: plays
sample.mp4 to end-of-media). Refused: hevc/mpeg4video/mpeg2video, >2ch
AAC. NOT refused (accepted): fragmented (moof/SlowLinear), edts, .mov.
WHERE MP4 ACTUALLY FALLS OVER: (1) in-session seek ALWAYS fails (Wave 5
S0, re-confirmed by execution: seek accepted, position stuck 0.0); (2)
no audio device (null sink); (3) no visible surface in any shipped lane.

## THE FOUR DECODER REGISTRIES (only one true)
- TRUE: pipeline_host.rs:239-381 wires 14 descriptors (mjpeg, flac,
  mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_*, av1, vp9, dnxhd,
  h264/avc1, pcm_*).
- LIE 1 [S0]: engine-info "decoders: mjpeg, pcm_s16le" (host.rs:1643-58,
  hardcoded, understates by ~12) — and the "landed:" error string
  (pipeline_host.rs:376-377) omits wired vp9/dnxhd.
- LIE 2 [S0]: bench.rs:300/313/334 prints "no decoder in this build
  (first vertical: mjpeg + pcm)" WHILE THE SAME BINARY PLAYS THE FILE;
  v_frames counts only fed-to-limb packets -> decodable h264 reports 0.
  Wave-0's "zero decode" litmus reading was this instrument's artifact.
- LIE 3: lib.rs crate-root doc ("MJPEG+PCM", "avi today") + es.rs doc.

## Worst findings
1. [S0, static-proof] desktop-tauri (the SHIPPED GUI) is COMPILE-BROKEN:
   app.rs:301 calls undefined `spawn_session_pump` (E0425; real name
   `spawn_event_pump` at :190) — masked by gdk-sys failing first in this
   sandbox; "INFRA-BLOCKED compiles in CI" claim FALSE vs the tree
   (W13-A10; orchestrator rg-verified).
2. [S0, executed] host.rs:1307 hardcodes `seekable: true` — FLAC-in-WAV
   run: engine says seekable:false, host prints yes, seek fails
   not-seekable: one run, three contradictions (W13-A02).
3. [S1, executed] main.rs:14 env::args() panics on non-UTF8 argv (exit
   101) (W13-A05). [S1, re-executed] TS-188 probe panic exit 101 — STILL
   LIVE (W13-A08/A09). [S1] ts.rs mint_adts_aus infinite loop+OOM on
   frame_length==0 (replica-proof: 1e6 zero packets, `at` never advances)
   (W13-A08). [S1, executed] ogg.rs seeds the 38-byte flac second header
   where decoder parses 34 -> flac-in-ogg BROKEN (spec-shaped .oga ->
   "sample rate 0 invalid"); zero coverage (W13-A06).
4. [S2, executed] ogg --start-time decorative: 66/67 packets pass; clock
   1.1s, content from 0:00 (W13-A06). [S2] adts.rs: every raw-AAC session
   dies at first decoded frame (cli.audio_limb_absent; geometry
   discarded in a debug_assert) (W13-A04). [S2] opus_head_of_dops never
   byte-swaps BE dOps -> pre-skip 312->14337, gain -3dB->-58dB; the test
   asserts the BUGGY bytes (W13-A04). [S2] ttf_seek + sync_scenarios:
   pass with zero assertions / #[ignore]d criteria (W13-A10).
5. [KEY] Desktop reality: apps/desktop is a headless bridge LIBRARY +
   skeleton binary exiting 86; LUF1 frames dead-end in an in-process
   FrameMailbox; the only real blit is MediaSurface.tsx:227 behind
   __TAURI_INTERNALS__ in the compile-broken tauri app; harness.mjs
   cited as evidence by 5 files DOES NOT EXIST (W13-A01/A03/A05/A09/A10).
6. [S2] WindowFull I420 stash freeze (Wave 6) has ZERO test coverage;
   admitted latent ring_full fatal on longer media (W13-A01).
