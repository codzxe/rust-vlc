# Algorithmic-quality audit — decode pipeline (card 5.X1a)

Status: **measurement + fix-list card — NO fixes landed here** (the
mandate: profile, name every slow/wasteful algorithm, hand X1b a
ranked list). Every number below was measured this session on this
machine with the committed tooling; raw stdout rows preserved in
`docs/perf/raw/mjpeg/stage-x1a-20261024.txt` and
`docs/perf/raw/speed-x1a-20261024.json`.

| Field | Value |
|---|---|
| Card | 5.X1a (wave 8, delta card 5.X1 part 1 — the user's speed mandate) |
| Date | 2026-10-24 (session 1) |
| Machine | shared 2-core sandbox (Intel Xeon 6/173, 2 cores) — **loadavg 1.4–4.0 during the runs** (concurrent wave-8 agents); stage medians are over 5 interleaved passes, end-to-end is best-of-2 fresh-process per the tool's protocol |
| Binaries | `lumen` release @ worktree `wip/p5-x1a` (main 774ac5b, clean tree; compile lines verified `worktrees/p5-x1a`), and `stage_bench` release example from the same tree — both copied aside to `/tmp/x1a-lumen`, `/tmp/x1a-stage_bench` before measurement |
| Corpus | `/tmp/my-project/corpus-v4/media` (the wave-8 corpus of record): heavy MJPEG lanes `0073-avi-video.avi` (3840×2160, 19 f, RGBA — the uhd4k lane), `0087-avi-video.avi` (1920×1080, 77 f, I420), `0124-avi-video.avi` (512×512, 337 f, RGBA), `0052-avi-video.avi` (1024×768, 150 f, RGBA) |
| Tools | `crates/codecs/examples/stage_bench.rs` (ablation stage attribution, committed 7-b) + `tools/corpus/speed_bench.py` (end-to-end decode-to-null vs ffmpeg) — **no src edits for profiling** |

## 1. End-to-end: where the speed mandate stands (measured)

`speed_bench.py`, best-of-2 wall, fresh process per run, decode-to-null
(ffmpeg `-f null -`, lumen `decode-bench`):

| lane | geometry / fmt | ffmpeg wall | lumen wall | lumen vs ffmpeg | lumen realtime |
|---|---|---|---|---|---|
| 0073 uhd4k | 3840×2160 RGBA, 19 f | 1.046 s | **6.619 s** | **0.158×** | **0.239×** |
| 0052 | 1024×768 RGBA, 150 f | 1.074 s | 3.182 s | 0.337× | 0.786× |
| 0124 | 512×512 RGBA, 337 f | 0.936 s | 2.444 s | 0.383× | 1.379× |
| 0087 | 1920×1080 I420, 77 f | 0.898 s | 1.092 s | 0.822× | 2.937× |

The uhd4k lane is the mandate's heavy case: **0.239× realtime
(goal ≥ 1.0×), 0.158× ffmpeg**. Note the split: RGBA-output lanes are
2.5–6× off ffmpeg; the one I420 lane is at 0.82× — the RGBA assembly
path and the per-frame pipeline overhead (§3) are what separate them.

## 2. Stage attribution (the hotspot table)

`stage_bench --passes 5` (2 warm-up, arms interleaved; medians; the
validation gate — staged walk must byte-equal the production walk —
ran green on every lane before timing). ms/frame = median ÷ frames.

### 0073 uhd4k, 3840×2160, RGBA (19 f, full walk 1380.7 ms = 72.7 ms/f)

| stage | % of walk | ms/frame | suspected cause |
|---|---|---|---|
| assemble (color/upsample) | **63.1%** | **45.9** | scalar per-pixel YCbCr→RGB (`color.rs` `compose_color_row`): 3 Bresenham steppers + `while`-carry per pixel defeat auto-vectorization; RGBA write is 4 byte-stores per pixel |
| idct+dequant | 23.6% | 17.1 | scalar pass-2 column transform; SSE4.1 pass-1 dispatch is a net **loss** (§4) |
| huffman | 6.7% | 4.9 | already 8-bit-LUT'd (ED-FIX-1); near floor for this density |
| store | 5.4% | 3.9 | plane write traffic, padded rows |
| parse + residual | ~1.2% | 0.9 | header walk + MCU bookkeeping |
| **scalar-vs-SIMD** | full scalar walk = 1079.4 ms | 56.8 | — |

### 0087, 1920×1080, I420 (77 f, full walk 631.3 ms = 8.2 ms/f)

| stage | % of walk | ms/frame | suspected cause |
|---|---|---|---|
| idct+dequant | **56.8%** | 4.7 | I420 assembles by row-crop (cheap), so IDCT dominates; scalar pass-2 |
| huffman | 21.8% | 1.8 | LUT walk; higher share = denser bitstream at 1080p |
| assemble (crop only) | 13.2% | 1.1 | `crop_plane_into` memcpy-class |
| store | 11.3% | 0.9 | plane write traffic |
| residual | −3.2% (−20.1 ms) | — | **negative** — stub-arm subtraction noise under loadavg 2.4–2.9; the share table for this lane over-attributes idct/store by that amount (recorded, not hidden) |

### 0124, 512×512, RGBA (337 f, 1088.6 ms = 3.2 ms/f) and 0052, 1024×768, RGBA (150 f, 1382.4 ms = 9.2 ms/f)

| stage | 0124 % / ms-f | 0052 % / ms-f |
|---|---|---|
| assemble | **46.3%** / 1.50 | **48.2%** / 4.44 |
| idct+dequant | 33.3% / 1.08 | 29.5% / 2.72 |
| huffman | 16.1% / 0.52 | 16.1% / 1.48 |
| store | 3.1% / 0.10 | 4.2% / 0.39 |

**Consistent law across all four lanes: RGBA output spends 46–63% of
the codec walk in `assemble` (color conversion + nearest-neighbor
upsampling); I420 output spends 57% in IDCT.** Huffman is 7–22%
(already optimized, low headroom). Parse is ~0%.

## 3. The end-to-end vs codec-walk gap (the other half of the mandate)

`decode-bench` wall minus the stage_bench full walk (same file, same
binary family, SIMD dispatch — apples-to-apples on the walk side):

| lane | end-to-end (best) | codec walk (median) | gap | gap per frame |
|---|---|---|---|---|
| 0073 uhd4k | 6619 ms | 1381 ms | **5238 ms (79% of end-to-end)** | **275 ms/f** |
| 0052 | 3182 ms | 1382 ms | 1800 ms | 12.0 ms/f |
| 0124 | 2444 ms | 1089 ms | 1355 ms | 4.0 ms/f |
| 0087 | 1092 ms | 631 ms | 461 ms | 6.0 ms/f |

On the uhd4k lane **the pipeline outside the codec walk costs ~3.8×
the codec walk itself** (275 vs 73 ms/frame). The gap is NOT constant
per frame (4–275 ms) and not cleanly per-MB either — suspects, in
order: the pooled-output slot path for 33 MB RGBA frames
(`vec![0; len]` cold-path zero-fill + pool re-key churn), per-packet
engine framing/handoff, AVI packet-loop overhead. The demux read path
itself was already optimized (V-1/7-c: one source call per payload,
arena pool — `docs/perf/v1-input-readpath-20261024.md`), so the read
seam is an unlikely suspect. **[VERIFY X1b: instrument the
decode-bench path per frame before touching anything — this table is
the measurement that says where to look.]**

## 4. The SIMD IDCT dispatch is a measured net loss on every lane

`stage_bench`'s paired arm (full SIMD-dispatch walk vs identical walk
with `transform` pinned to scalar), medians of 5 interleaved passes:

| lane | med full (SIMD) | med scalar | SIMD penalty (full/scalar) |
|---|---|---|---|
| 0073 uhd4k | 1380.7 ms | 1079.4 ms | **1.279× slower** |
| 0087 1080p I420 | 631.3 ms | 394.9 ms | **1.599× slower** |
| 0124 | 1088.6 ms | 925.4 ms | 1.176× slower |
| 0052 | 1382.4 ms | 1160.7 ms | 1.191× slower |

4/4 lanes, same direction. This sharpens 7-b's "parity, point
estimates slow side" verdict (their two lanes: 1.021×/0.995×) into a
consistent 18–60% penalty on the corpus-v4 lanes. Suspected cause:
the dispatch path's pass-1 store/reload staging defeats the scalar
body's auto-vectorization (the scalar `idct_block` is
branch-structured for the compiler; the intrinsics path pays a
re-shuffle that pass-2's scalar consumption re-pays). Load-caveat
applies (§ header), but the arms are interleaved within each pass, so
drift hits both arms.

## 5. The other hot paths (quick audit)

- **FLAC / MP3 / WAV decode throughput: DNF at the probe layer —
  measured, not assumed.** `lumen decode-bench` on
  `0138-audio-only.wav`, `0145-cover-audio.mp3`,
  `0106-cover-audio.flac` → typed refusal `probe.unsupported /
  probe.no-demuxer` (exit 65) on all three (raw rows in the speed
  JSON). The decode engines exist (claxon / symphonia-bundle-mp3 /
  PCM) but no standalone-container demuxer wires them on this path —
  a **coverage** finding, not a speed finding. Until a wav/mp3/flac
  demux lane exists there is no honest throughput number to audit.
- **MKV framing**: mkv lanes (`0003-multi-av.mkv`, `0149-dynres-video.mkv`,
  both h264 payloads) also refuse at decode (h264 software decode
  blocked — codec coverage, per doc 04). The mkv demux framing itself
  was exercised by 5.D1's test-pinned header work; no throughput lane
  exists to measure until a decodable mkv payload lands (AV1 via
  rav1d would provide one — see next).
- **AV1 (rav1d 1.1.0, asm-off): DNF — no corpus lane.** corpus-v4 has
  zero AV1 files (ffprobe census: mkv lanes are ffv1/h264/vp9/hevc).
  A 2 s 320×240 `libaom-av1` fixture encode exceeded the 120 s
  tool-call budget on the loaded sandbox (attempt recorded, killed).
  Handoff: pre-generate the AV1 fixture offline (or reuse 5.E1's
  fixtures) and measure rav1d throughput in X1b — the asm-off
  throughput question stays open, honestly.

## 6. Ranked fix list for X1b (expected gain · risk · bit-exactness)

1. **RGBA assemble SIMD/specialization** (`color.rs`
   `compose_color_row`). Target: the 46–63% assemble share on every
   RGBA lane. Shape: specialize the dominant sampling geometries
   (4:2:0-style h=1/v=1 chroma over h_max=2; 4:2:2; 4:4:4) so the
   chroma steppers disappear (chroma row read once per 2 pixels), then
   an SSE2/AVX2 kernel converting 4–8 pixels per iteration with the
   identical Q16 math. Expected: 3–4× on assemble ⇒ −35 to −45%
   total codec-walk time on RGBA lanes (0073: 73→~45 ms/f).
   Risk: medium (new kernel; pinned by the existing
   `compose_row_matches_the_per_pixel_reference` sweep + decode
   digests). Bit-exact: **yes, keepable** — same integer rounding,
   different loop tiling.
2. **Flip the IDCT dispatch default to scalar** (keep the SSE4.1
   kernel + its bit-exact dispatch test). Expected: −18 to −60% codec
   walk per §4 (0073: 1381→1079 ms = −22%; 0087: −37%). Risk: **none**
   (scalar is the reference the SIMD is pinned against; the 7-b
   [PROPOSAL] now has 4-lane evidence). Bit-exact: yes, trivially.
3. **Instrument + attack the per-frame pipeline gap** (§3). First
   action: per-frame `Instant` probes in a bench harness around the
   decode-bench packet loop (demux pull, decode, handoff, pool slot) —
   NO src edits for profiling. If the 33 MB pooled-slot path confirms
   as the 4K cost, fixes range from warm-slot reuse without zero-fill
   to smaller handoff granularity. Expected: unknown until
   instrumented; the ceiling is large (4K lane: 275 ms/f of non-codec
   time vs 73 ms/f of codec time — halving the gap alone takes the
   lane from 0.24× to ~0.36× realtime). Risk: low (seams already
   pooled/test-pinned). Bit-exact: output bytes unaffected by pool
   timing changes.
4. **IDCT pass-2 for I420 lanes** (`idct.rs`). 57% of the 1080p walk.
   An AVX2 two-pass rewrite (or improved scalar factoring that the
   compiler vectorizes) targeting 2× on IDCT ⇒ −28% walk on I420
   lanes. Risk: high-ish (arithmetic restructure risks bit drift —
   keep the exact Q16 rounding ladder, differential-test against the
   scalar reference). Bit-exact: achievable but must be proven, not
   assumed.
5. **Huffman fusion** — LOW priority: 7–22% share, already
   LUT-optimized (ED-FIX-1); remaining ideas (two-symbol decode per
   refill) are diminishing-returns. Only after 1–4.
6. **Coverage follow-ups (not speed)**: wav/mp3/flac probe demuxers so
   audio throughput becomes measurable at all (§5); an AV1 fixture so
   rav1d asm-off throughput gets its first number.

Combined honest projection for the uhd4k lane if 1+2+3 land at their
expected midpoints: codec walk 1381→~700 ms and pipeline gap
5238→~2600 ms ⇒ end-to-end ~3.3 s ⇒ **~0.48× realtime** — still short
of the 1.0× goal; the goal likely also needs 4 (I420) or output-format
work (RGBA handoff is 4× the bytes of I420 — worth an X1b design
question: is RGBA the right engine output for 4:2:0 sources?).

## 7. Honest limitations

- Shared sandbox at loadavg 1.4–4.0 (concurrent wave-8 agents);
  stage numbers are medians of 5 interleaved passes, end-to-end is
  best-of-2 — the record, not a quiet-room claim. The 0087 negative
  residual (−20 ms) is flagged in-line (§2).
- stage_bench ablation slightly understates each stubbed stage's
  standalone cost (cache-traffic removal) — bounded by the residual
  column, per the 7-b design note.
- 5 passes (not 15) on the heavy lanes — deadline economics on a
  loaded 2-core box; per-pass rows are in the raw file for re-aggregation.
- The §3 gap attribution is a suspects list with a measurement
  bounding its size, NOT a per-stage attribution — that instrument
  does not exist yet and is fix-list item 3's first action.
- No AV1/wav/mp3/flac throughput numbers exist (§5) — DNF with
  evidence, not silent omission.

## 8. Reproducing

```bash
export CARGO_TARGET_DIR=<worktree>/target
cargo build --release -p lumen-cli && cargo build --release -p codecs --example stage_bench
cp $CARGO_TARGET_DIR/release/lumen /tmp/x1a-lumen
cp $CARGO_TARGET_DIR/release/examples/stage_bench /tmp/x1a-stage_bench

cd /tmp/my-project/corpus-v4/media
/tmp/x1a-stage_bench 0073-avi-video.avi 0087-avi-video.avi --passes 5
/tmp/x1a-stage_bench 0124-avi-video.avi 0052-avi-video.avi --passes 5

# end-to-end: manifest with the 4 heavy lanes (+ audio/mkv probes) at
# /tmp/x1a-speed (symlinks into corpus-v4; builder inline in session log)
python3 tools/corpus/speed_bench.py --corpus /tmp/x1a-speed --repeat 2 \
  --runner "lumen=/tmp/x1a-lumen decode-bench {file}" \
  --json out.json --md out.md
```

Raw data: `docs/perf/raw/mjpeg/stage-x1a-20261024.txt` (stage stdout
+ per-pass stderr), `docs/perf/raw/speed-x1a-20261024.json` (end-to-end
per-file, ffmpeg + lumen + the three refusal rows).
