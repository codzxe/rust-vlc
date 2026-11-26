# tools/vta/vp9 — 6.VTA-rusty-vp9 differential harness (SPIKE)

Standalone (non-workspace) cargo bin: decodes an .ivf VP9 file
through **OUR** ivf demuxer (`crates/demux/src/ivf`) into
**rusty_vp9 0.1.3**, and writes tightly packed I420 rawvideo
(8-bit, or u16LE for profile-2 10/12-bit) for MD5/per-frame
comparison against an ffmpeg `libvpx-vp9` reference decode.
Evidence lives in `docs/dependencies/records/rusty_vp9-0.1.3.md`;
every bit-exact verdict there is re-runnable by the committed
command below ("Re-run the §D differential").

## Re-run the §D differential (the committed regen command)

Every bit-exact verdict in `docs/dependencies/records/rusty_vp9-0.1.3.md`
§D (6/6 lanes, 431/431 frames byte-identical) is reproduced by ONE
committed command over the committed fixtures
(`crates/codecs/tests/fixtures/vp9/` — the byte-identical staged lanes of
the record's §D table; see `crates/codecs/tests/vp9_differential.rs`):

```sh
# reference half only (ffmpeg/ffprobe, no cargo): ffprobe facts +
# ffmpeg libvpx-vp9 rawvideo stream MD5s vs the record's §D table
bash tools/vta/vp9/evidence/run_vp9_bitexact.sh --ref-only
# full differential (adds the rusty_vp9 mine half below — needs cargo)
bash tools/vta/vp9/evidence/run_vp9_bitexact.sh
```

The reference half was re-verified GREEN 2026-11-26 (FX8-A06) on ffmpeg
7.1.5-0+deb13u1: 6/6 lanes' facts + stream MD5s match the record. The
full differential needs `cargo run --release` of this harness for the
mine half; on a box without cargo on PATH the script exits 3 with a
`[VERIFY-central]` marker — it never pretends the mine half ran. The
per-frame comparison (the record's method) is
`tools/vta/vp9/evidence/compare_frames.py`: per-frame MD5 of
frame-sized chunks both sides + a whole-stream MD5 cross-check.

## Run

```sh
cd tools/vta/vp9
cargo run --release -- /path/lane.ivf --out /tmp/lane.mine.raw
# ffmpeg reference (match layout exactly: rawvideo, yuv420p for 8-bit):
ffmpeg -y -c:v libvpx-vp9 -i /path/lane.ivf -f rawvideo -pix_fmt yuv420p /tmp/lane.ref.raw
# 10-bit lanes: -pix_fmt yuv420p10le (harness emits u16LE per sample)
# hostile-input no-panic battery:
cargo run --release -- /path/lane.ivf --hostile
```

Per-frame digests are computed EXTERNALLY (md5sum / python hashlib)
over frame-sized chunks of `.mine.raw` vs `.ref.raw` — the harness
itself only demuxes, decodes, and writes. (A hand-rolled in-harness
MD5 was tried first and was WRONG — the whole-stream md5sum
cross-check caught it; digests stay external on purpose.)

Comparator used for the record (frame size =
`bps·(w·h + 2·⌈w/2⌉·⌈h/2⌉)`, `bps=2` when bd>8):

```python
import hashlib
mine = open('lane.mine.raw','rb').read(); ref = open('lane.ref.raw','rb').read()
fsz = 115200  # 320x240 I420
ident = sum(hashlib.md5(mine[i*fsz:(i+1)*fsz]).digest() == hashlib.md5(ref[i*fsz:(i+1)*fsz]).digest() for i in range(len(mine)//fsz))
```

## Pipeline

file → `demux::avi::test_support::mem_stream` → `IvfDemuxer::new` →
`start()` → pump `next_event` to EOS (collect track-0 packets) →
`Vp9Decoder::push(pkt, None)` (superframes split internally) →
`flush()` → `next_frame()` to `Error::Eof` → `DecodedFrame` planes
(stride-padded) repacked tight: Y rows × w, then U/V rows × ⌈w/2⌉,
u16LE for high bit depth — byte-layout identical to ffmpeg rawvideo
`yuv420p` / `yuv420p10le`.

`.webm` VP9 lanes: remuxed to .ivf with `ffmpeg -c copy -f ivf`
(no re-encode, elementary stream byte-preserving; verified by the
whole-stream decode output matching ffmpeg decoding the .webm
directly — lane E in the record).
