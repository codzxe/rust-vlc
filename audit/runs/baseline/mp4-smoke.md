# MP4 SMOKE TEST — Wave 0 baseline (litmus test)
# date: 2026-11-23T17:42:08Z
# binary: target/release/lumen (cargo build --release --locked -p lumen-cli, exit 0)

## cmd 1: lumen open sample.mp4 (stdin closed, 30s timeout)
```
$ timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample.mp4 </dev/null
ok: accepted session=1 gen=0
event: state-changed Idle → Opening
event: tracks 2 ready
event: buffering 0.00
event: state-changed Opening → Buffering
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { info: MediaInfo { source: File { path: "/…/sample.mp4" }, container: None, title: None, duration: Known { time: Timestamp(3000000) }, seekable: true, is_live: false, metadata: {}, tracks: [TrackInfo { id: TrackId(0), kind: Video, name: None, language: Some("voe"), codec: CodecDescriptor { fourcc: None, description: Some("h264"), profile: None, level: None }, is_default: false, is_selected: true, origin: Stream, video: None, audio: None }, TrackInfo { id: TrackId(1), kind: Audio, name: None, language: Some("voe"), codec: CodecDescriptor { fourcc: None, description: Some("aac"), profile: None, level: None }, is_default: false, is_selected: true, origin: Stream, video: None, audio: None }], chapters: [] } }
event: buffering 0.02
event: buffering 1.00
event: state-changed Buffering → Playing
event: state-changed Playing → Stopped
event: state-changed Stopped → Idle
exit_code=0
```

## moov box positions (first 40 bytes of each box scanned)
sample.mp4 ftyp@0 free@32 mdat@40 moov@86752
sample-faststart.mp4 ftyp@0 moov@32 free@3314 mdat@3322
sample-hevc.mp4 ftyp@0 free@28 mdat@36 moov@39144
sample-aac-only.mp4 ftyp@0 free@28 mdat@36 moov@36580

## cmd 2: timed open (wall clock)
```
$ time (timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample.mp4 </dev/null >/dev/null 2>&1)
```

## cmd 3: decode-bench sample.mp4
```
$ ./target/release/lumen decode-bench /home/z/my-project/samples/sample.mp4
bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm)); audio track#1 aac — not benched (no decoder in this build (first vertical: mjpeg + pcm)) (container mp4)
BENCH file=/home/z/my-project/samples/sample.mp4 bytes=90034 v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.000 decode_s=0.000
exit_code=0
```

## cmd: lumen open sample-faststart.mp4
```
$ timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample-faststart.mp4 </dev/null
event: MediaInfoReady { info: MediaInfo { source: File { path: "/…/sample-faststart.mp4" }, container: None, title: None, duration: Known { time: Timestamp(3000000) }, seekable: true, is_live: false, metadata: {}, tracks: [TrackInfo { id: TrackId(0), kind: Video, name: None, language: Some("voe"), codec: CodecDescriptor { fourcc: None, description: Some("h264"), profile: None, level: None }, is_default: false, is_selected: true, origin: Stream, video: None, audio: None }, TrackInfo { id: TrackId(1), kind: Audio, name: None, language: Some("voe"), codec: CodecDescriptor { fourcc: None, description: Some("aac"), profile: None, level: None }, is_default: false, is_selected: true, origin: Stream, video: None, audio: None }], chapters: [] } }
event: buffering 0.02
event: buffering 1.00
event: state-changed Buffering → Playing
event: state-changed Playing → Stopped
event: state-changed Stopped → Idle
exit_code=0
```

## cmd: lumen open sample-hevc.mp4
```
$ timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample-hevc.mp4 </dev/null
error: [decode.unsupported]: unsupported (cli.codec_unavailable): no decoder for `hevc` in this build (landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)
  code: cli.codec_unavailable
event: state-changed Idle → Opening
event: error [decode.unsupported] unsupported (cli.codec_unavailable)
event: state-changed Opening → Error
event: state-changed Error → Idle
exit_code=65
```

## cmd: lumen open sample-aac-only.mp4
```
$ timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample-aac-only.mp4 </dev/null
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { info: MediaInfo { source: File { path: "/…/sample-aac-only.mp4" }, container: None, title: None, duration: Known { time: Timestamp(3000000) }, seekable: true, is_live: false, metadata: {}, tracks: [TrackInfo { id: TrackId(0), kind: Audio, name: None, language: Some("voe"), codec: CodecDescriptor { fourcc: None, description: Some("aac"), profile: None, level: None }, is_default: false, is_selected: true, origin: Stream, video: None, audio: None }], chapters: [] } }
event: buffering 1.00
event: state-changed Buffering → Playing
event: state-changed Playing → Stopped
event: state-changed Stopped → Idle
exit_code=0
```

## cmd: lumen open sample.mp4 info (one-shot info after open)
```
$ timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample.mp4 info </dev/null
error: open takes exactly one source path
  usage: lumen open <path> [flags]
exit_code=0
```

## calibration: decode-bench on PCM wav (first-vertical codec)
```
$ ./target/release/lumen decode-bench /home/z/my-project/samples/sample.wav
bench: audio track#0 pcm_s16le — decoding (container wav)
BENCH file=/home/z/my-project/samples/sample.wav bytes=264678 v_frames=0 v_decoded=0 a_frames=5 a_decoded=5 wall_s=0.000 decode_s=0.000
exit_code=0
```

# cmd: ./target/release/lumen engine-info
ok: engine
  version: 0.1.3
  host: cli
  features: first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor
  decoders: mjpeg, pcm_s16le
exit_code=0
