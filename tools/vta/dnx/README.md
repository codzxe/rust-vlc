# tools/vta/dnx — VTA differential harness for filmcraft-dnx 0.9.0

Card 6.VTA-filmcraft-dnx (Phase 6 Wave 1). Standalone (own `[workspace]`),
NOT a Lumen workspace member. Evidence record:
`docs/dependencies/records/filmcraft-dnx-0.9.0.md`.

## Build (off the shared target dir)

    export PATH="$HOME/.cargo/bin:$PATH"
    CARGO_TARGET_DIR=/tmp/vta-dnx-target cargo build --release

## Lane generation (ffmpeg 7.1.5 — corpus has NO dnxhd lanes)

DNxHD encoder accepts only strict resolution+bitrate+pixfmt combos; the
valid set is printed by ffmpeg on any rejected attempt. Lanes used
(3 frames each, intra-only so frame compare is exact):

    ffmpeg -f lavfi -i testsrc2=size=1920x1080:rate=24 -frames:v 3 \
      -c:v dnxhd -b:v 175M -pix_fmt yuv422p  -f dnxhd L1_1080p_175M.dnxhd
    ffmpeg -f lavfi -i smptehdbars=size=1920x1080:rate=24 -frames:v 3 \
      -c:v dnxhd -b:v 36M  -pix_fmt yuv422p  -f dnxhd L2_1080p_36M.dnxhd
    ffmpeg -f lavfi -i mandelbrot=size=1920x1080:rate=24 -frames:v 3 \
      -c:v dnxhd -b:v 145M -pix_fmt yuv422p  -f dnxhd L3_1080p_145M.dnxhd
    ffmpeg -f lavfi -i testsrc2=size=1280x720:rate=24 -frames:v 3 \
      -c:v dnxhd -b:v 90M  -pix_fmt yuv422p  -f dnxhd L4_720p_90M.dnxhd
    ffmpeg -f lavfi -i testsrc2=size=1920x1080:rate=24 -frames:v 3 \
      -c:v dnxhd -b:v 185M -pix_fmt yuv422p10le -f dnxhd L5_1080p10_185M.dnxhd

## Differential

    # reference decode (ffmpeg dnxhd decoder, native pix_fmt)
    ffmpeg -i LANE.dnxhd -f rawvideo LANE.ref.raw
    # ours: split CBR ES into N frames, decode, pack rawvideo planar
    /tmp/vta-dnx-target/release/vta-dnx --frames 3 LANE.dnxhd LANE.mine.raw
    # compare: per-frame MD5 + per-plane maxDelta/differing counts
    python3 evidence/compare.py   # (as run; adjust paths if moved)

DNxHD is CBR → the ES splits into filesize/N frames; N is known from
generation and cross-checked by byte-count parity with ffmpeg's output.

## Hostile battery

    /tmp/vta-dnx-target/release/vta-dnx --hostile LANE.dnxhd

Six cases (zero-length, 64 B truncation, 0xAA fill, LCG garbage,
header+zeroed payload, half-truncated) + a 256-point sampled truncation
sweep, all under `catch_unwind`. Result this run: 0 panics (3 lanes).

## evidence/ (committed)

- `ffprobe-facts.txt` — per-lane codec/profile/pix_fmt/dims/rate facts
- `compare-results.txt` — full per-frame, per-plane differential table
- `hostile-results.txt` — battery + sweep output for L1/L4/L5
