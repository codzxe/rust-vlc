# W07-A04 measurement evidence — MP3/MP2 head alignment vs ffmpeg (settles mp3/decoder.rs#part2 F-001 + the tagless open question)

Executed 2026-11-21 in-session with the sandbox's ffmpeg 7.1.5-0+deb13u1 (the same
version the baseline used for fixtures). NOT a cargo/npm run — generate-and-measure
only, per the baseline's own sample-generation precedent. Artifacts in this directory.

## Setup
- `tagless.mp3`: `ffmpeg -f lavfi -i sine=frequency=440:duration=2:sample_rate=44100
  -ac 2 -c:a libmp3lame -b:a 128k -write_xing 0 -id3v2_version 0` → 78 frames
  (ffprobe packet count), no Xing/LAME/ID3 tag (verified by -write_xing 0 +
  -id3v2_version 0; the file is 32,256 bytes ≈ pure frames).
- `src.wav`: the same 2 s sine as PCM (88,200 samples/channel) — the encoder's input.
- `tagless_mp3_ffmpeg_decode.raw`: `ffmpeg -i tagless.mp3 -map 0:a -c:a pcm_s16le
  -f s16le` → 359,424 bytes = 89,856 s16 values/channel = **78 × 1152 EXACTLY**.
- `t.mp2` / `tsrc.wav` / `tdec.raw`: the same shape with `-c:a mp2` @ 48 kHz →
  84 frames × 1152 = 96,768 samples/channel EXACTLY.

## Result 1 — tagless MP3: ffmpeg applies NO head skip
Cross-correlation (least-|Δ| shift over samples 1000..3000): **the source's sample 0
lands at ffmpeg-output index 1105** (= 576 LAME encoder delay + 529 synthesis
latency; fit error ≈ 92/sample on an amplitude-2896 signal — lossy rounding).
The output's first ~1105 samples/channel are the delay/priming head and its length
is the full frames × 1152.
Consequence: for a tagless MP3 the reference (ffmpeg) timeline is the RAW timeline,
shift 0. `decode_stream` (no trim without a LAME tag) matches it; the CONTRACT
decoder's unconditional 529-sample head trim (`decoder.rs:493`) misaligns the
output by 529 samples (≈12 ms @44.1 kHz). Since no demuxer produces the 12-byte
facts blob (xing.rs.audit.md F-002), EVERY shipped `lumen open *.mp3` hits this
case — the S2 in mp3/decoder.rs#part2 F-001 covers mp1/mp2 AND tagless mp3.
(The 529 shift IS correct for LAME-tagged streams — the recorded differential
proves it there; the defect is applying it without the tag.)

## Result 2 — MP2: both decoders include the synthesis head; relative shift 0
`t.mp2`: the source's sample 0 lands at ffmpeg-output index ≈481 (the Layer II
subband-synthesis ramp) — ffmpeg's tagless MP2 output ALSO starts with the
synthesis head, and the repo's own differential (`[fixture.mp2_48k]
samples_per_channel = 72576` = 63 × 1152 exact, MD5s match, max_delta ≤ 1) proves
Lumen's raw MP2 output is byte-exact with ffmpeg's at shift 0. Trimming 529
samples from an MP2 stream (contract decoder, `decoder.rs:493`) therefore chops
into the aligned head+signal — confirmed misalignment vs the reference for every
shipped mp2/mp1 playback.

## Commands (reproducible)
```
ffmpeg -y -v error -f lavfi -i sine=frequency=440:duration=2:sample_rate=44100 \
  -ac 2 -c:a libmp3lame -b:a 128k -write_xing 0 -id3v2_version 0 tagless.mp3
ffprobe -v error -select_streams a: -show_packets -of csv tagless.mp3 | wc -l   # 78
ffmpeg -y -v error -i tagless.mp3 -map 0:a -c:a pcm_s16le -f s16le tagless_mp3_ffmpeg_decode.raw
# 359424 bytes = 78*1152*2ch*2B
python3 <least-abs-shift correlation vs src.wav>  # -> 1105
# mp2 twin: -> 84 frames, 96768 = 84*1152, shift ~481
```
