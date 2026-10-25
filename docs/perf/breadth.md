# Phase-5 breadth snapshot — decode throughput, TTF, seek, demux (card 5.R1)

Status: **measured breadth snapshot — every number below was produced
this session on this machine by the commands in §8; nothing is quoted
from a previous run.** Where a lane is **not** honestly measurable on
this build the row says DNF with the evidence, never a silent skip.

| Field | Value |
|---|---|
| Card | 5.R1 (wave 10 — the Phase-5 perf-vs-VLC breadth snapshot per doc 12) |
| Captured | 2026-10-25 03:40–03:52 UTC (single session, loadavg 0.12–0.3, idle 2-core sandbox) |
| Binaries | `lumen` release (`/tmp/r1-lumen`, built this session from this worktree @ 2422a44, `CARGO_TARGET_DIR=<worktree>/target cargo build --release -p lumen-cli`), codec bench test binaries from the same tree (`cargo test -p codecs --all-features --release --test <name>_bench -- --ignored --nocapture`), `lumen-mem-audit` release (same tree) |
| Corpus | `/tmp/my-project/corpus-v4/media` (143 files; the lanes named per lane below) + committed fixtures (`crates/codecs/tests/fixtures/`) |
| Oracle | **ffmpeg 7.1.5-0+deb13u1** — VLC 3.0.23 is absent from this sandbox (the `tools/bench-vs-vlc` harness refuses without it, per its binding mock/real rule), so ffmpeg is the speed oracle everywhere, per the doc-12 §1.3 fallback. All numbers are this-sandbox-specific. |
| Raw data | `docs/perf/raw/r1/` (this card's CSVs + bench transcripts, committed) |

Run-selection discipline (fixed before any run, doc 12 §3.5–3.6):
warm-up discarded; **median-of-5** fresh processes for every
wall-clock family (mjpeg decode-bench lanes, ffmpeg lane walls,
mem-audit walks); the per-codec in-process benches keep their own
committed REPS (7, or 5 for av1) with medians, run once, never
re-run until liked; probe/seek families N=10 fresh sessions each.
Failures stay in the CSVs as rows.

## 1. Hardware manifest (read from /proc this session)

| Field | Value |
|---|---|
| CPU model | Intel(R) Xeon(R) Processor (family 6 / model id 173) |
| Logical cores | 2 |
| RAM | 4 138 564 kB (~3.95 GiB) |
| Kernel | 5.10.134-013.15.kangaroo.al8.x86_64 |
| GPU | none (software decode everywhere, both sides) |
| rustc / cargo | 1.99.1 (58cf8f9c 2026-10-13) / 1.99.1 (5f94df478 2026-08-27) |
| Cargo profile | `release` (optimized, default codegen-units; no PGO/LTO flags) |
| Machine | shared 2-core sandbox (`c-6adae26f-…`), same host class as the 3.F1 baseline and every prior perf doc; medians over N are the record |

## 2. Audio decode throughput (per-codec benches, in-process)

Surface: each codec's committed ignored-bench (contract path — packet
walk + decode + sink; no container, no process spawn). Realtime
factor uses the lane's own channel count × rate. The ffmpeg column
is `-benchmark` child CPU (utime+stime) on the same file — **1 ms
kernel granularity**, so the 4-second lanes carry ±12–25 % clock
coarseness; ratios are labeled coarse where utime rounds to 0.

| codec / lane | inter-ch. samples/s (median) | × realtime | ffmpeg CPU rate | lumen/ffmpeg (coarse) |
|---|---|---|---|---|
| FLAC 0046 (88.2 kHz stereo 24-bit) | 46 421 260 | 526.3× | ~44.1 M/s (utime-only: 50.4 M/s) | **~0.9–1.05× (parity)** |
| MP3 0139 (11.025 kHz stereo) | 71 675 908 | 3 251× | ~22.1 M/s | 3.2× |
| MP3 fixture cbr128 (44.1 kHz stereo) | 98 926 446 | 1 122× | — | — |
| MP2 0119 (44.1 kHz stereo) | 105 655 092 | 1 198× | ~39.4 M/s | 2.7× |
| MP2 0005 (16 kHz mono) | 99 516 548 | 6 220× | ~21.5 M/s (utime=0) | 4.6× |
| Vorbis 0085 (8 kHz stereo, ogg demux incl.) | 24 152 607 | 1 510× | ~8.0 M/s | 3.0× |
| AAC-LC 0059 (88.2 kHz mono) | 68 413 446 | 776× | ~29.5 M/s | 2.3× |
| AAC-LC 0110 (96 kHz mono) | 107 433 098 | 1 119× | ~35.0 M/s | 3.1× |
| ALAC stereo-24 fixture (48 kHz st., mp4 demux incl.) | 133.1 MB/s decoded PCM (1.515 ms/file median; 33 600 frame-samples) | 503× | utime 0.000–0.003 s — below clock | DNF (below 1 ms resolution, both sides fast) |
| ADPCM-IMA 0042 (44.056 kHz stereo, in wav) | **DNF — no ignored-bench harness exists for ADPCM** (decode itself is covered green by `adpcm_corpus_differential`); ffmpeg reference: ~67.1 M/s | — | ~67.1 M/s | — |

Honest reading: every landed audio decoder is comfortably faster
than realtime (526×–6 220×); the self-built lossy decoders (MP3/MP2/
Vorbis/AAC) are 2.3–4.6× ffmpeg's own CPU time on these lanes, while
claxon-FLAC sits at parity with ffmpeg's FLAC. The asymmetry runs
both ways (ffmpeg's number includes its demux + null-mux; ours
includes the packet walk) — these are pipeline-adjacent numbers, not
isolated-kernel numbers, and are labeled as such.

## 3. Video decode throughput

### MJPEG (post-X1b; `lumen decode-bench`, median-of-5 fresh processes, vs ffmpeg decode-to-null wall, median-of-5 fresh processes — the one fully symmetric comparison in this doc)

| lane | geometry / fps | lumen wall | lumen ms/frame | lumen Mpix/s | ffmpeg wall | ffmpeg Mpix/s | lumen/ffmpeg | × realtime |
|---|---|---|---|---|---|---|---|---|
| 0052 (1024×768 RGBA, 150 f, 60 fps) | yuvj444p | 0.902 s | 5.98 | 131 | 0.430 s | 274 | **0.477×** | **2.77×** |
| 0073 uhd4k (3840×2160 RGBA, 19 f, 12 fps) | yuvj444p | 0.760 s | 39.7 | 207 | 0.410 s | 384 | **0.539×** | **2.08×** |
| 0124 (512×512 RGBA, 337 f, 100 fps) | yuvj444p | 0.708 s | 2.07 | 125 | 0.334 s | 265 | **0.472×** | **4.76×** |
| 0087 (1920×1080 I420, 77 f, 24 fps) | yuvj420p | 0.422 s | 5.40 | 378 | 0.289 s | 553 | **0.685×** | **7.60×** |

Movement vs the 5.X1a audit (same lanes, same protocol, cited
unchanged from that doc): 0052 3.182 s → 0.902 s, 0073 uhd4k
6.619 s → 0.760 s (**8.7× end-to-end**; 0.239× → 2.08× realtime),
0124 2.444 s → 0.708 s, 0087 1.092 s → 0.422 s. The X1b optimization
(assemble rewrite + pipeline-gap fix) is the whole delta; the lane
geometries and the ffmpeg side are unchanged within noise.

### AV1 (rav1d 1.1.0, asm OFF — the 5.E2 fixture, regenerated and re-measured this session)

| surface | result |
|---|---|
| ours, in-process IVF demux + rav1d-asm-off decode + I420 copy-out | 33.0 fps = **68.41 Mpix/s** (1080p, 12 f) |
| ffmpeg `libdav1d` (asm on), full pipeline incl. spawn | 112.9 fps |
| honest-gap ratio | **0.292×** (5.E2 recorded 0.309× same-session-class; the asm-off purity cost, `docs/dependencies/records/rav1d-1.1.0.md` policy) |

### Video lanes with no decoder (parsers only) — honest DNF

h264 (0014 mkv) and hevc (0044 raw .265 / 0038 avi) have parsers,
not decoders (hw-path cards 5.F2/5.G2): no throughput number exists
to report and none is invented. ffprobe confirms the payloads;
decode is blocked per doc 04 codec coverage.

## 4. Time-to-first-frame / first-sample (M-03/M-04 family)

What is honestly measurable on this build (same §2.4 contract gap
as the 3.F1 baseline: no `video.first-frame-presented` /
`audio.first-sound` events in the v1 catalog — the per-frame signal
is DNF-with-evidence, the open-path latencies below are the real
surface):

| lane | signal | N | median |
|---|---|---|---|
| 0052 / 0073 / 0124 (AVI, the only openable container) | open → `media.info-ready` | 10 | **2.48–2.49 ms** (responses 2.45–2.47 ms) |
| 0085 ogg / 0042 wav / 0059 adts / 0094 m4a / 0034 mkv / 0014 mkv | open → typed refusal `probe.unsupported` (no open-path wiring, D-018 first vertical) | 10 | 2.42–2.53 ms |

The Phase-5 container lanes refuse at open in ~2.5 ms — typed and
fast, but **they are refusals, not playback**: the honest TTF state
for every non-AVI container on this build is "not reachable through
the command layer". The decoders themselves are wired in the
pipeline host (`HostDecoderProvider`: flac/mp3/mp2/vorbis/aac-lc/
alac/adpcm/av1 all dispatched) — the per-container open-path wiring
is the named next card (the refusal message says so verbatim).

## 5. Seek latency (M-05 family)

| lane | signal | N | median |
|---|---|---|---|
| 0052 AVI (1024×768, 150 f) | seek#1 (50 %) write→`timing.seek-completed` | 10 | **102.10 ms** (warm seek#2: 100.70 ms) |
| 0073 AVI uhd4k (3840×2160, 19 f) | seek#1 write→`timing.seek-completed` | 10 | **404.35 ms** (warm seek#2: 705.30 ms) |

The 4 K lane's seek is dominated by pre-roll re-decode (~40 ms/frame
post-X1b) plus the pooled-output handoff — a real number a user of
this build would feel; recorded, not hidden. The warm-seek
discrepancy on 0073 (404 → 705 ms) is unexplained by anything in
this card's scope — flagged for the engine owner.

**DNF with evidence — the three seek machineries the card names:**
FLAC SEEKTABLE (`codecs::audio::flac::seek::SeekPlanner`), MP3
CBR/VBR (the Xing-TOC `seek` module), and MKV Cues
(`demux::mkv` `fn seek`, mod.rs:811) all exist and are correctness-
pinned by their suites (`flac_seek.rs`, the mp3 seek tests, mkv cue
tests), but **none is reachable from the CLI on this build**: the
open path refuses their containers (§4), so no honest end-to-end
latency exists to measure, and a codec-layer timing harness is a src
change this docs-only card forbids. Handoff: measure all three the
moment the per-container open-path card lands.

## 6. Demux throughput post-X2b (mkv)

`lumen-mem-audit` on 0014-muxed-av.mkv (3.69 MiB, 255 packets,
h264+mp3), per-walk time derived from `--repeat 6` vs `--repeat 1`
walls (median-of-5 fresh processes each, warm-up discarded):

| metric | value |
|---|---|
| per full-file demux walk | **4.77 ms** |
| packets/s | **53 494 pkt/s** |
| container bytes walked | 773 MiB/s (walk bytes 3 930 502 = 1.02× file, post-X2b) |
| allocation rate (same run) | 0.11 calls/pkt (28 calls total — the X2b after-table re-confirmed on this tree) |

## 7. Honest summary — where Phase-5 stands vs the ffmpeg-class bar

**Faster than realtime everywhere that decodes at all**: audio
526×–6 220×, MJPEG 2.08×–7.60× (the X1b mandate outcome — the
uhd4k lane moved from 0.239× to 2.08× realtime), mkv demux at
53 k pkt/s with near-zero allocs. **Slower than ffmpeg, honestly**:
MJPEG sits at 0.47×–0.69× ffmpeg wall-to-wall (RGBA assembly is the
known remaining cost — the X1a stage table still applies), and AV1
is 0.29× because rav1d runs asm-OFF by dependency policy while
libdav1d runs full asm — a policy gap, not an engineering one
(Phase-7 admission reviews it). **At parity**: claxon-FLAC vs
ffmpeg FLAC (~0.9–1.05×). **Not measurable on this build** (the
card's own honesty ledger): ADPCM throughput (no bench harness),
TTF per-frame events (v1 catalog gap), seek on flac/mp3/mkv (open
path), h264/hevc decode (parsers only), ALAC ratio (fixture below
ffmpeg's 1 ms CPU clock). **Sandbox limits**: VLC 3.0.23 absent →
ffmpeg 7.1.5 is the oracle; a shared 2-core sandbox at low load;
every number is this-machine-specific and not comparable across
hosts (doc 12 §1.5 host-measured caveat).

## 8. Reproducing

```bash
export CARGO_TARGET_DIR=<worktree>/target
cargo build --release -p lumen-cli && cp $CARGO_TARGET_DIR/release/lumen /tmp/r1-lumen
export LUMEN_CORPUS=/tmp/my-project/corpus-v4
for t in flac_bench mp3_bench mp2_bench vorbis_bench aac_bench alac_bench av1_bench; do
  cargo test -p codecs --all-features --release --test $t -- --ignored --nocapture
done
# mjpeg end-to-end + ffmpeg lanes (median-of-5 each):
python3 tools/perf/r1_ff_lanes.py <lanes...>          # ffmpeg side
for i in 1 2 3 4 5; do /tmp/r1-lumen decode-bench <lane>; done   # lumen side
# probe/seek families (N=10; absolute paths override the fixture dir):
python3 tools/perf/baseline.py probe --binary /tmp/r1-lumen --raw-dir <abs>/docs/perf/raw/r1 --runs 10 --fixtures <abs lanes...>
python3 tools/perf/baseline.py seek  --binary /tmp/r1-lumen --raw-dir <abs>/docs/perf/raw/r1 --runs 10 --fixtures <abs lanes...>
# demux throughput:
cargo build --release -p lumen-mem-audit && time target/release/lumen-mem-audit [--repeat 6] <0014.mkv>
```

## 9. Raw data (committed)

`docs/perf/raw/r1/`: `codec-benches-20261025.txt` (audio+av1 bench
transcripts), `mjpeg-decode-bench.csv` (25 rows), `ffmpeg-lanes.csv`
(median-of-5 walls), `ffmpeg-benchmark-audio-utime.txt`,
`memaudit-0014-demux-throughput.txt`, `probe.csv` (60 rows: 6
refusal lanes), `r1-avi/probe.csv` (30 rows) + `r1-avi/seek.csv`
(20 rows). The CSVs are the truth; the tables above are their
medians.
