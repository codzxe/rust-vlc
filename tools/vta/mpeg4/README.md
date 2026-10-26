# tools/vta/mpeg4 — 6.VTA-oxideav-mpeg4video SPIKE harness (ARCHIVED — crate BLOCKED)

Standalone (non-workspace) validate-then-admit harness for
`oxideav-mpeg4video` 0.1.8. **Verdict: BLOCKED** — see
`docs/dependencies/records/oxideav-mpeg4video-0.1.8.md` (tolerance-class
differential ±1..3 on 7/8 lanes vs the house bit-exact bar + packed-bitstream
duplicate-frame divergence). The crate is NOT a dependency of any workspace
crate; nothing here is wired into `crates/codecs`.

## Layout

- `src/main.rs` — demuxes AVI/MP4 through OUR `crates/demux`
  (`AviDemuxer`/`Mp4Demuxer` over `avi::test_support::mem_stream`),
  feeds the first video track's packets to `Mpeg4VideoDecoder`
  (literal-spec mode; `--ecosystem` for the crate's compat mode),
  primes VOL config from `--extradata <hex.txt>` when the container
  carries it out-of-band, crops frames to the VOL display dims
  (macroblock-padded planes → tight I420, chroma ceil) and writes
  ffmpeg-rawvideo-layout bytes + one `FRAME idx type WxH pts ticks`
  line per output frame. `--hostile` runs the no-panic battery
  (truncate sweep, zero-length, garbage, VOP-header corruption,
  garbage-then-real recovery, truncated mid-stream).
- `diff.py` — differential driver: ffprobe `-show_data` extradata →
  hex; harness → `mine.raw`; ffmpeg `-map 0:v:0 -f rawvideo -pix_fmt
  yuv420p` → `ref.raw` (+ `framehash` md5s); per-frame MD5 + pixel
  deltas + whole-stream md5.

## Reproduce (ffmpeg/ffprobe 7.1.5)

```bash
export PATH="$HOME/.cargo/bin:$PATH"
CARGO_TARGET_DIR=/tmp/vta-mpeg4-target cargo build

# lanes: corpus mp4 direct; corpus avi via honest -c copy remux (our AVI
# demuxer's v0 codec filter is MJPG-only — see record §D); 2 generated
cd /tmp/vta-mpeg4/lanes
C=/tmp/my-project/corpus-v4/media
cp $C/0097-mp4-video.mp4 0097.mp4; cp $C/0045-mp4-video.mp4 0045.mp4
cp $C/0093-param-midchange.mp4 0093.mp4
ffmpeg -v error -i $C/0117-packed-bs.avi -c copy 0117.mp4
ffmpeg -v error -i $C/0132-avi-av.avi -map 0:v:0 -c copy 0132.mp4
ffmpeg -v error -i $C/0107-odd-dims.avi -c copy 0107.mp4
ffmpeg -v error -f lavfi -i testsrc2=size=320x240:rate=25:duration=3 \
        -c:v mpeg4 -q:v 5 g_base.mp4
ffmpeg -v error -f lavfi -i testsrc2=size=320x240:rate=25:duration=3 \
        -c:v mpeg4 -q:v 5 -bf 2 -g 12 g_bf2.mp4

HARNESS=/tmp/vta-mpeg4-target/debug/vta-mpeg4 python3 \
  /home/z/my-project/worktrees/p6-vta-mpeg4/tools/vta/mpeg4/diff.py \
  g_base g_bf2 0097 0045 0132 0107 0093 0117

# packed lane vs the ORIGINAL avi reference:
ffmpeg -v error -i $C/0117-packed-bs.avi -map 0:v:0 -f rawvideo \
        -pix_fmt yuv420p /tmp/vta-mpeg4/out/0117.orig-ref.raw
# ...then the dedupe analysis from the record §D (python one-liner in the
# spike worklog / record footnote: 37 → drop 9 consecutive byte-identical
# dups → 28, compare vs orig-ref).

# hostile battery (0 panics measured):
/tmp/vta-mpeg4-target/debug/vta-mpeg4 g_bf2.mp4 --hostile \
  --extradata /tmp/vta-mpeg4/out/g_bf2.extradata.txt
```

## Results snapshot (2026-10-26, full tables in the record)

| lane | frames | identical | differing | max Δ |
|---|---|---|---|---|
| g_base 320×240 | 75/75 | 0 | 4.68 % | 3 |
| g_bf2 (b-frames) | 75/75 | 0 | 4.50 % | 2 |
| 0117 packed-bs | **37/28** (9 dups) | 0 | 3.38 %¹ | 2¹ |
| 0097 mp4 | 37/37 | 0 | 0.998 % | 1 |
| 0045 mp4 | 37/37 | 0 | 0.623 % | 1 |
| 0132 avi-remux | 41/41 | 0 | 0.845 % | 2 |
| 0107 odd-dims 1×1 | 25/25 | **25/25** | 0 | **0** |
| 0093 param-midchange | 63/63 | 0 | 2.25 % | 2 |

¹ after dropping the 9 byte-identical duplicate frames the crate emits
for DivX N-VOP placeholders (the packed-bitstream defect).

Hostile: 0 panics (3 lanes × 6 cases + truncate sweeps 0/9595, 0/2872,
0/6617). Decode errors: 0 packets across all lanes.
