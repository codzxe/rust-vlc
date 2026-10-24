# Memory audit — corpus-v4 (card 5.X2a, 2026-10-24)

Peak-RSS + per-frame allocation audit of the landed decode/demux lanes
on **corpus-v4** (`/tmp/my-project/corpus-v4/media`, 143 files) via
`tools/mem-audit`, feeding the X2b fix list. Every number below was
measured on this machine, this date, release build, single walk
thread; raw lines are quoted or tabled verbatim from tool output.
NO fixes in this card — measurements + ranked fix list only.

Companion to `memory-audit-20261023.md` (the W5-15 first audit, whose
register items V-1..V-4 are re-verified here).

## 1. What this card added to the instrument

- **Peak RSS per file** (`rss_hwm_kB` / `rss_base_kB` on every
  `MEMAUDIT` line): the process watermark `VmHWM` is read from
  `/proc/self/status` after the walk window and reset before the open
  window via `/proc/self/clear_refs` mode 5 (verified working on this
  host: a 200 MB touch → free → reset dropped HWM 207,848 → 12,624 kB).
  The reads happen OUTSIDE every counting window.
- **Multi-family demux lanes**: the walk now drives the landed demux
  `crates/demux` families, not just AVI — `mkv`, `mp4`, `wav`
  (PCM decode via `fmt` facts), `ogg` — because the probe registry
  still knows only `avi`+`mp4` signatures; for a probe refusal the
  audit tries the signature-less demuxer constructors directly on
  fresh streams and propagates the probe's typed refusal when the
  try-list is exhausted (the honest in-tree state, e.g. native FLAC
  → `probe.no-demuxer`).

**Per-file RSS discipline (honesty note):** the C library does not
return freed heap to the OS, so *sequential* in-process runs share a
retained RSS floor (measured: after the 4K lane, every later file
reports `rss_hwm == rss_base`). All RSS figures in §3/§4 therefore
come from **one process per file** invocations; the in-process
`--repeat 2` runs are used only for the dimension-stability proof
(walk totals byte-identical across reps; peak deltas ≤ 528 B).

## 2. Method (unchanged core)

Counting global allocator over the real path (`input` → `media-probe`
→ `demux` → `codecs` behind `codec-api`), open/walk windows, decode
to null. The D-1 detector is `calls_per_pkt` (walk-window allocation
calls ÷ packets fed): **> 0 in the steady state is a per-frame
allocation site.** `peak_B` is requested-bytes live peak; `rss_hwm_kB`
is the system's resident-page truth — both reported, never conflated.

```sh
cargo run --release -p lumen-mem-audit -- CLASS=FILE...          # classified run
cargo run --release -p lumen-mem-audit -- FILE                    # per-file RSS (one process)
cargo run --release -p lumen-mem-audit -- --repeat 2 --demux-only FILE  # drilldowns
```

## 3. Main lanes (measured 2026-10-24, one process per file)

| lane | file (corpus-v4/media) | content | size B | peak RSS kB (base) | peak_B req. | calls/pkt | B/pkt | verdict |
|---|---|---|---|---|---|---|---|---|
| uhd4k mjpeg | 0073-avi-video.avi | mjpeg 3840×2160, 19 f | 2,849,644 | **59,472** (1,160) | 58,288,622 | **1.32** | 3,063,757.68 | decoded 19/19 |
| heavy-mjpeg | 0052-avi-video.avi | mjpeg 1024×768, 150 f | 9,393,612 | **8,196** (1,132) | 5,823,178 | **0.22** | 38,278.40 | decoded 150/150 |
| mjpeg-1080p | 0087-avi-video.avi | mjpeg 1920×1080, 77 f | 4,470,084 | **8,888** (1,208) | 6,434,434 | **0.42** | 82,537.04 | decoded 77/77 |
| h264-in-mkv demux | 0014-muxed-av.mkv | h264 1280×720 + aac, 255 pkt | 3,864,699 | **3,924** (1,212) | 1,475,024 | **4.10** | 45,824.38 | demuxed 185 v + 70 a |
| 24h-still mp4 | 0012-still-24h.mp4 | still, 19 h declared | 1,684 | **2,672** (1,192) | 136,125 | 3.00 (1 pkt) | 978.00 | demuxed 1 v |
| mp4 control | 0027-mp4-video.mp4 | mp4 video, 42 pkt | 222,520 | 2,644 (1,236) | 198,856 | **0.07** | 158.24 | demuxed 42 |
| mp4 control | 0049-muxed-av.mp4 | v+a, 77 pkt | 26,458 | 2,596 (1,224) | 159,775 | **0.05** | 20.70 | demuxed 42+35 |

Honest lane note: corpus-v4 has **no 1080p h264-in-mkv file** — the
largest h264-MKV is 0014 at 1280×720 (ffprobe-verified); it is the
representative lane. The 1920×1080 decode lane is 0087 (mjpeg/AVI).

**Where the memory lives** (allocation-window + code attribution):

- **uhd4k**: ≈ 2 full frame buffers + pools. Peak 58.3 MB requested ≈
  decoder output buffer (3840×2160×4 RGBA = 33.2 MB) + scratch/planes
  + payload arenas; RSS 59.5 MB is +2.0% over requested bytes —
  malloc overhead is negligible at frame sizes. The 25 steady-state
  calls over 19 frames = ~1/frame (output handoff wrap) + first-frame
  buffer growth + occasional DHT rebuilds (this stream re-emits DHT).
- **heavy-mjpeg / 1080p**: same shape, smaller constants; 33/32 calls
  TOTAL over 150/77 frames — the payload pool serves every video
  packet and the decoder reuses its output buffer.
- **h264-mkv**: 1.48 MB peak, but **11,685,217 B allocated over the
  walk = 3.0× file size**, 1,046 calls = 4.10/pkt — see F-2 (the
  double-buffer read).
- **24h-still mp4**: bounded — RSS 2.67 MB, peak_B 136 kB, 3 walk
  calls. Duration (19 h) costs nothing: no read-ahead buffering of
  the future, exactly the streaming property wanted.
- **mp4 controls**: 3-4 walk calls for 42-77 packets (0.05-0.07/pkt)
  — the mp4 payload pool is HIT; the contrast that indicts mkv (F-2).

## 4. Audio set (flac/wav lanes, one process per file)

| file | payload | pkts | peak RSS kB | peak_B | calls/pkt | outcome |
|---|---|---|---|---|---|---|
| 0006-audio-only.wav | pcm_s24le 176.4 kHz stereo | 36 | 2,836 | 198,859 | **0.08** | decoded 36/36 (PCM limb) |
| 0013-sr-chaos.wav | pcm_s16le 11 kHz | 1 | 2,764 | 162,987 | 3.00 | decoded 1/1 |
| 0016-audio-only.wav | FLAC-in-WAV 96 kHz 4 ch | 2 | 2,836 | 198,859 | 1.50 | demuxed (counting-only limb) |
| 0042-audio-only.wav | adpcm_ima_wav | 3 | 2,660 | 198,859 | 1.00 | demuxed (counting-only limb) |
| 0138-audio-only.wav | pcm_s16le 16 kHz 6 ch | 12 | 2,700 | 198,859 | **0.25** | decoded 12/12 |
| 0046-audio-only.flac | native FLAC | — | 2,764 | 264,718 | — | **refused** `probe.no-demuxer` (typed, bounded) |
| 0071-cover-audio.flac | native FLAC + cover | — | 2,848 | 236,222 | — | **refused** `probe.no-demuxer` |

The WAV payload pool (`PayloadSlots::new(2, MAX_PACKET_BYTES)`) is
**HIT**: 3 walk calls total for whole files (multi-packet lanes
0.08-0.25 calls/pkt); the 1.00-3.00 rows are single/dual-packet files
where the 3 setup calls dominate the ratio. FLAC-in-WAV and ADPCM
count only (their decoders have not landed); native FLAC is a typed
probe refusal with open-phase peak ≤ 265 kB — bounded, honest.

## 5. Dimension stability (`--repeat 2`, in-process)

walk totals are **byte-identical** across reps on every lane (e.g.
0073: 58,211,396 B / 25 calls both reps; 0014: 11,685,217 B / 1,046
both; 0052: 5,741,760 / 33 both) and peaks move ≤ 528 B — the
bounded-memory property holds on every family measured.

## 6. Findings + ranked X2b fix list

**F-1 (VERIFIED FIXED — old register V-1/V-3).** The AVI video-packet
pool spill (2 allocs + 2× payload bytes per video packet, measured
2.000/pkt in the 2026-10-23 audit) is gone: whole-pipeline steady
state on the mjpeg lanes is **0.22-1.32 calls/pkt** (heavy: 33 calls
for 150 frames). The range-view arena rewrite of `PayloadSlots`
(riff.rs) and the DHT handling land as designed. No X2b action.

**F-2 (NEW D-1 violation — mkv demux, every mkv lane measured).**
4.10-4.13 calls/pkt and ~3× file-size bytes on h264-mkv (0014), av1-
mkv (0034: 414 calls/101 pkt = 4.10) and audio-only-mkv (0002: 347/84
= 4.13). Root cause (code attribution): `EbmlReader::read_prefix_at`
(`crates/demux/src/mkv/ebml.rs:307-329`) allocates a fresh
`vec![0u8; want]` per positioned read, and its only packet-path
caller (`crates/demux/src/mkv/mod.rs:577-583`) then **copies** that
throwaway Vec into the pooled arena the closure was handed — one
alloc + one zero-fill + one full payload copy per frame that the pool
was built to avoid. Remaining ~3 calls/block: the per-block
`frames`/`sizes`/`packets` Vecs (`cluster.rs:250,285,363`;
`mod.rs:569`).

**F-3 (peak RSS at 4K is frame-buffer-bound — old register V-4).**
uhd4k peak = 58.3 MB requested / 59.5 MB RSS ≈ output buffer +
scratch ≈ 2× frame bytes. Decoder-side frame-buffer pooling /
`SharedBytes` range handoff (the registered 3.B3 follow-up) is the
only lever that moves this number.

**F-4 (bounded by construction, re-proven).** 24h-still mp4: RSS
2.67 MB / peak_B 136 kB — duration-independent. Refusal paths
(native FLAC, garbage): ≤ 265 kB open-phase peak. mp4/wav pools hit
(0.05-0.25 calls/pkt).

| # | X2b fix | site | expected reduction (measured basis) | risk |
|---|---|---|---|---|
| 1 | **mkv: read directly into the pool buffer** — a positional `read_into_at` on `EbmlReader` so the `fill` closure's `&mut [u8]` is the destination (kill the throwaway Vec + copy) | `ebml.rs:307`, `mod.rs:577` | −1 alloc + −1 zero-fill + −1 full copy **per frame** on every mkv lane; walk bytes 11.7 MB → ~4 MB on 0014 (−3×file-size → ~1×); calls/pkt 4.10 → ~3.1 | Low — the closure already receives the destination slice; needs an `io`-shaped reader method mirroring `read_into` |
| 2 | **mkv: reuse per-block scratch Vecs** (`frames`, `sizes`, `packets`) in the demuxer instead of fresh Vecs per block | `cluster.rs:250,285,363`, `mod.rs:569` | −2..3 calls/block → mkv steady state ~3.1 → **~0.5-1 calls/pkt** (parity with mp4's 0.05-0.07 + lacing tables) | Low-Med — lifetimes across the block parse; pure demuxer-internal |
| 3 | **MJPEG frame-buffer pooling / `SharedBytes` range output handoff** (3.B3) | `codecs/src/video/mjpeg/decoder.rs` (`take_output`/`resize`), `mod.rs` FrameStorage | −~25-33 MB peak RSS at 4K (≈ one frame buffer), −~1 call/frame; 1080p −~4-6 MB | Medium — decoder output ownership is pinned by `zero_alloc_decode.rs` at exactly 2 events; contract change, needs codec-api review |
| 4 | *(watch, no action)* DHT re-emitting streams (uhd4k 1.32 vs heavy 0.22 calls/pkt) — rebuilds are **required for correctness**; in-place table reuse only if profiling ever justifies it | `codecs/.../mjpeg/huffman.rs` | marginal | correctness-sensitive |

## 7. Gates

`cargo fmt -p lumen-mem-audit` clean; `cargo clippy -p lumen-mem-audit
--all-targets` zero warnings; `cargo test -p lumen-mem-audit` **9/9
passed** (one timing-credibility micro-bench in `selfcheck` flaked
once under sandbox load — 1 MiB alloc pair at 1,235 ns — and passed on
re-run unchanged; it measures wall time, not this card's counters).

## 8. Reproduce

```sh
T=target/release/lumen-mem-audit
cd /tmp/my-project/corpus-v4/media
$T --repeat 2 uhd4k=0073-avi-video.avi heavy-mjpeg=0052-avi-video.avi \
   mjpeg-1080p=0087-avi-video.avi h264-mkv-720p=0014-muxed-av.mkv still-24h-mp4=0012-still-24h.mp4
for f in 0073-avi-video.avi 0052-avi-video.avi 0087-avi-video.avi \
         0014-muxed-av.mkv 0012-still-24h.mp4 0027-mp4-video.mp4 \
         0006-audio-only.wav 0013-sr-chaos.wav 0016-audio-only.wav \
         0042-audio-only.wav 0138-audio-only.wav 0046-audio-only.flac; do $T "$f"; done
```

— 5.X2a, 2026-10-24. Tool: `tools/mem-audit` (branch `wip/p5-x2a`).
