# VTA harness — oxideav-ac3 0.0.11 (card 6.VTA-oxideav-ac3)

> **STATUS: ARCHIVED — NOT ADMITTED.** The differential gate FAILED
> (corpus lane maxΔ 316 LSB, see
> `docs/dependencies/records/oxideav-ac3-0.0.11.md`). `oxideav-ac3`
> is NOT a dependency of any Lumen workspace crate. This harness is
> evidence only. Do not wire it into `crates/codecs`.

Standalone (own `[workspace]` — NOT a member of the Lumen workspace),
harness-only AC-3 decode differential vs ffmpeg. Never production.

## What it does

1. **Syncwalker** (harness-only, `syncwalk`): splits a raw `.ac3`
   elementary stream into syncframes using syncword `0x0B77` + the
   ATSC A/52B §5.4.1.4 Table 5.18 frame-size formula (documented in
   `src/main.rs`; 44.1 kHz carries alternating table rows —
   empirically confirmed 834/836-byte strides on the corpus lane).
   Resyncs (counted) when the predicted boundary misses the syncword.
2. **Decode**: `oxideav_ac3::decoder::make_decoder` (channels =
   passthrough) → `send_packet`/`receive_frame` → `Frame::Audio`
   interleaved **s16le** (the crate's only output format), 1536
   samples/ch per syncframe.
3. **Differential**: `ffmpeg -c:a pcm_s16le -f s16le` on the same
   file; full-stream sample compare (exact / differing / max |Δ| in
   LSB) + MD5s.
4. **Hostile** (`hostile` subcommand): truncated frame, garbage
   mid-stream, zero-length, reserved fscod, reserved frmsizecod,
   garbage-only — each on a warm decoder under `catch_unwind`.
5. **cplprobe** (`cplprobe` subcommand): per-block `cplinu` /
   `dithflag` side-info census via the crate's own parser (used to
   characterise which lanes exercise coupling).

## Run

```sh
# data: corpus lane + generated lanes live in /tmp/vta-ac3 (see
# record §D for the exact generation commands)
mkdir -p /tmp/vta-ac3/{lanes,gen}
cp <corpus>/media/0001-audio-only.ac3 /tmp/vta-ac3/lanes/corpus_0001.ac3
ffmpeg -y -f lavfi -i "sine=frequency=440:duration=6:sample_rate=44100" \
       -f lavfi -i "sine=frequency=554:duration=6:sample_rate=44100" \
       -filter_complex "[0:a][1:a]amerge=inputs=2[a]" -map "[a]" \
       -c:a ac3 -b:a 192k /tmp/vta-ac3/gen/g_st192_44k.ac3
# ... remaining lanes: see record §D table

cd tools/vta/ac3
cargo run --release            # differential table
cargo run --release -- hostile # hostile-input contract
cargo run --release -- cplprobe /tmp/vta-ac3/lanes/corpus_0001.ac3
```

Requires ffmpeg/ffprobe 7.1.5 on PATH; writes `out_*.s16` / `ff_*.s16`
into `/tmp/vta-ac3`.
