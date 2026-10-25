# tools/vta/ruopus — 6.VTA-ruopus differential harness (SPIKE)

Standalone (non-workspace) cargo bin: decodes an Ogg/Opus file through
**OUR** ogg demuxer (`crates/demux/src/ogg`) into ruopus, and writes raw
f32le 48 kHz PCM for MD5/sample-level comparison against an ffmpeg
reference decode. Evidence lives in
`docs/dependencies/records/ruopus-0.1.2.md`.

## Run

```sh
cd tools/vta/ruopus
cargo run --release -- /tmp/my-project/corpus-v4/media/0047-audio-only.ogg --out /tmp/ru.pcm
# ffmpeg reference (match format exactly: f32le, 48 kHz, native channel count):
ffmpeg -i /tmp/my-project/corpus-v4/media/0047-audio-only.ogg -f f32le -ar 48000 /tmp/ff.pcm
# hostile-input contract check (no-panic requirement):
cargo run --release -- <any>.ogg --out /dev/null --hostile
```

`--trim-per-ch N` cuts the output to N samples/channel (end-trim to the
reference's granulepos-based count — the harness does NOT apply Ogg Opus
end-trimming itself; see record §D).

Pipeline: file → `demux::avi::test_support::mem_stream` →
`OggDemuxer::new` → `start()` → pump `next_event` to EOS (the demuxer
harvests OpusHead/OpusTags during pumping) → resolve the opus serial via
`header_packets` → `OpusDecoder` (mono/stereo, mapping family 0) or
`MultistreamDecoder` (family 1, RFC 7845 streams/coupled/mapping from
OpusHead[19..]) → **pre-skip**: discard `preskip × channels` leading
decoded samples (RFC 7845 §5.1; ffmpeg does the same via skip_samples
side data) → output gain field is reported but NOT applied (both corpus
and generated lanes measured gain = 0).

Channel order: ruopus emits RFC 7845 Vorbis order (5.1: FL FC FR BL BR
LFE); ffmpeg's native opus decoder reorders to FL FR FC LFE BL BR — the
record's comparison applies that permutation on the reference side.
