# PERF-VS-VLC — the final Phase 8 benchmark report (issue #55, cards 8.D1–8.D2)

| Field | Value |
|---|---|
| Purpose | The MP §16 Phase 8 gate artifact "perf-vs-VLC" (doc 12 §9.2): Lumen vs the pinned VLC 3.0.23 oracle, with honest ✓/partial/✗ vocabulary, decoder attribution and method notes |
| Card / issue | 8.D1 + 8.D2 / GitHub issue #55 (docs-finalization lane `7.w22h`) |
| Compiled | 2026-11-10, from committed evidence only (every claim below names its artifact; nothing was re-run for this document except the two deterministic xtool audits named in §6) |
| Oracle pin | **VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)** — asserted at run time by the harness (doc 10 §6.6 pin; version line recorded in `docs/corpus/vlc-oracle-run-20261109.json`) |
| Speed oracle actually used for timing | **ffmpeg 7.1.5-0+deb13u1** — VLC was absent from the sandbox during every timing wave; doc 12 §1.3/`docs/perf/breadth.md` header record the fallback. **No wall-clock Lumen-vs-VLC timing number exists anywhere in this repo** (see §3) |

---

## 1. The honest headline — what was measured and what was not

| Class of evidence | Status | Where |
|---|---|---|
| **Decode-output differential vs VLC 3.0.23** (deterministic, bit/tolerance level) | **MEASURED** — 2026-11-09 real-oracle run, 35 comparable corpus rows: **29/35 bit-exact, 6/35 f32 tolerance-FAIL** | `docs/corpus/vlc-diff-report-20261023.md` §"Oracle run 2026-11-09", `docs/corpus/vlc-oracle-run-20261109.json`, `docs/corpus/vlc-f32-tolerance-20261109.json` |
| **Timing (decode throughput, TTF, seek, demux, memory)** | **MEASURED — but against ffmpeg 7.1.5, not VLC, on a shared contended 2-core sandbox** (waves 3–7, 2026-10-22..2026-10-26) | `docs/perf/baseline.md`, `docs/perf/breadth.md`, `docs/perf/mjpeg-simd-20261024.md`, `docs/perf/memory-audit.md`, `docs/perf/raw/**`, `docs/corpus/speed-baseline-20261022.md` |
| **Wall-clock Lumen vs VLC 3.0.23** | **NOT MEASURED — no run exists.** VLC was not installable in the perf waves' sandbox (`docs/perf/breadth.md` oracle row: "VLC 3.0.23 is absent from this sandbox… so ffmpeg is the speed oracle everywhere, per the doc-12 §1.3 fallback"); the 2026-11-09 VLC acquisition was used **only** for the decode differential, never timed. This document does not invent any | — |
| **3-OS × reference-hardware matrix** (doc 12 §9.2 full scope) | **BLOCKED (external)** — 3-OS runners and CI billing are user-owned blockers (`docs/plans/phase8-cards.md` §1; risk R-20 refresh, doc 10 §10.4) | `docs/14-risk-register.md` |

Per the anti-gaming rules (doc 12 §1.3, §9.3): losses are published next to wins, the decoder that produced every run is named, and no number was re-run until liked.

## 2. Differential methodology (issue #19 → §6.6 decision record)

Clean-room rules (doc 10 §6.6, MP §2): the pinned VLC binary runs externally,
never vendored, never linked; every invocation asserts the version pin (drift
aborts the run); sandboxed container, no egress. Harness: `tools/bench-vs-vlc`
(card 3.E3) with a **binding mock/real separation** — `--oracle vlc` requires
the real binary and refuses (exit 2) without it; `--oracle mock` labels every
artifact `[MOCK]` and is plumbing validation only; `--oracle pending` emits the
exact to-run commands with zero oracle digests. The pinned dump invocation
(transcribed from this repo's own baseline of the pinned Linux build,
`docs/baselines/raw/linux/14-options-full-help.txt` L2335-2348):

```sh
vlc -I dummy --play-and-exit --vout none --aout \
    --no-audiofile-wav --audiofile-format {u8|s16|float32} \
    --audiofile-file OUT INPUT
```

- **Comparable set (v1):** lossless end-to-end PCM audio (`pcm_u8`,
  `pcm_s16le`, `pcm_f32le`) in **AVI** — the landed first vertical (decision
  D-018) — compared over the **first audio track's decoded PCM**; the video
  codec does not gate the row. 35 of the 500-file corpus-big rows are
  comparable; every non-comparable fixture carries an explicit reason in the
  committed report (no audio track; no Lumen demuxer for that container in the
  audited build; VLC `afile` cannot dump pcm_s24le/s32le — the transcode-chain
  comparison stays doc 10 §6.6's open `[VERIFY]`).
- **Lumen side:** real decode through the workspace crates
  (`bench-vs-vlc/lumen-pcm-digests/1`, MD5 from RFC 1321 in-tree — zero new
  dependencies), independently cross-verified by a Python RIFF walk of the
  `01wb` payloads (60/60 clean ok-rows agree, run notes in the report) and
  **equal to the ffmpeg 7.1.5 decode on 35/35 rows** (md5 + byte count,
  re-proven in the 2026-11-09 run).
- **Host endianness:** little (recorded per artifact; a BE host makes rows
  non-comparable).
- **Corpus provenance honesty:** the original 2026-10-23 corpus-big inputs no
  longer exist on this machine; 16/35 regenerate sha256-identical from the
  committed manifest argvs, 19/35 embed runtime-random lavfi sources
  (`anoisesrc` without seed, `life`) and were **re-based on fresh realizations
  with Lumen re-decoded on the same fresh files** — every verdict in the table
  is same-input. Provenance record: `docs/corpus/vlc-oracle-run-20261109.json`
  `inputs` block (with the double-run sha256 proof of randomness).

## 3. Differential results (the 29/35 + 6 record)

**29/35 rows — every `pcm_u8` (8) and `pcm_s16le` (21) row: VLC's dump is a
bit-exact PREFIX of the Lumen payload** (head drop 0 B on all 29). The entire
byte-count shortfall is a **tail of 96–16384 B left unflushed** by VLC's
`afile` output at `--play-and-exit` end-of-stream — a dump-method artifact,
not a decode difference. Verdict vocabulary in the report: `MATCH-prefix
(bit-exact; tail N B unflushed)`.

**6/35 rows — every `pcm_f32le` row: the dump is not a sub-window of the
payload.** Per doc 10 §6.6 these fell to the sample-wise RMS/peak deviation
path. The **Lead decision (ratified by the orchestrator 2026-11-09, issue #19
§6.6; landed in commits `76a700b`/`192ffc7`)**: f32 parity is
**tolerance-based, not bit-based** — a row PASSES iff per-row RMS deviation
≤ 1e-4 (−80 dB) **AND** peak deviation ≤ 1e-3 (−60 dB), thresholds derived
from f32 rounding at VLC's float pipeline scale. Measurement basis: the actual
PCM on both sides, head-aligned over the VLC-delivered window
(`scripts/vlc_f32_deviation.py`; per-row values preserved in
`docs/corpus/vlc-f32-tolerance-20261109.json`).

Per-row measurements (canonical record: `vlc-f32-tolerance-20261109.json`;
re-verified rows reproduce exactly on deterministic inputs, \*-rows are fresh
realizations of the same magnitude — `vlc-diff-report-20261023.md` §6.6 table):

| fixture | audio | ch × rate | samples | RMS | peak | RMS ≤ 1e-4 | peak ≤ 1e-3 | verdict |
|---|---|---|---|---|---|---|---|---|
| media/0022-avi-av.avi | pcm_f32le | 1 × 22050 | 139244 | 1.2520e-2 | 2.4955e-2 | ✗ (125×) | ✗ (25×) | **FAIL** |
| media/0247-avi-av.avi \* | pcm_f32le | 1 × 48000 | 186348 | 1.2376e-2 | 5.0894e-2 | ✗ (124×) | ✗ (51×) | **FAIL** |
| media/0280-avi-av.avi \* | pcm_f32le | 2 × 22050 | 233690 | 8.7608e-3 | 3.2807e-2 | ✗ (88×) | ✗ (33×) | **FAIL** |
| media/0297-avi-av.avi | pcm_f32le | 8 × 11025 | 879632 | 8.2151e-5 | 4.6266e-2 | ✓ | ✗ (46×) | **FAIL** |
| media/0305-avi-av.avi \* | pcm_f32le | 1 × 22050 | 221164 | 1.2094e-1 | 5.4242e-1 | ✗ (1209×) | ✗ (542×) | **FAIL** |
| media/0330-avi-av.avi | pcm_f32le | 1 × 96000 | 102380 | 6.4252e-6 | 1.1998e-3 | ✓ | ✗ (1.20×) | **FAIL** |

**Outcome: 0/6 f32 rows PASS the ratified tolerances. Overall parity on the
comparable set: 29 bit-exact + 0 tolerance-pass = 29/35.** No row is close on
both axes: 0297 and 0330 clear the RMS bound but miss the peak bound by 46×
and 1.20×; the other four miss both. These are **genuine parity gaps under
the ratified policy and are not hidden** (`worklog.md` 7.w19rr).

Two facts bound the interpretation honestly (from the §6.6 decision record):

1. **The Lumen side is not the suspect:** on all 35 rows the fresh Lumen
   digest equals the ffmpeg 7.1.5 decode (md5 + byte count), and the
   RIFF-walk payload MD5 re-asserts it on the 6 f32 rows.
2. **The deviation pattern tracks VLC's float `aout` chain, not decode:** the
   failing magnitudes cluster by fixture profile (0305's violet-noise row is
   worst at RMS ≈ 0.12; the near-threshold 0330 runs at 96 kHz). The gap is
   between VLC's `afile` float32 output and the decoded wire PCM — VLC does
   not pass f32 through bit- or tolerance-exactly at these settings.

**Open follow-up (recorded, not resolved):** characterize VLC 3.0.23's float
`aout` chain on these fixture profiles (gain/dither/resampler behavior at
22050/11025/96000 Hz, mono vs 8-ch) and re-evaluate whether a decode-level
comparison channel exists for f32 — the u8/s16 rows' bit-exactness suggests
the integer paths bypass whatever the float chain applies.

## 4. Recorded timing evidence (oracle = ffmpeg 7.1.5; shared 2-core sandbox)

Every number in this section is **prior-wave recorded evidence**, quoted from
its committed artifact — none was produced for this report. Environment (all
runs): Intel Xeon 6/173, **2 logical cores**, ~3.95 GiB RAM, shared sandbox
at low load; host-measured wall clocks (doc 12 §1.5 — engine `ts_ns` events
are not in the v1 catalog); release builds; medians over N fresh processes
with fixed run-selection rules (doc 12 §3.5–§3.6). **These numbers are
this-machine-specific and are not a clean 3-OS reference-hardware record**
(doc 12 §9.2's full scope remains blocked, §1 above). Ratios are
median(ffmpeg wall) / median(Lumen wall): >1 = Lumen faster.

### 4.1 Audio decode throughput (`docs/perf/breadth.md` §2, raw `docs/perf/raw/r1/codec-benches-20261025.txt`)

| codec / lane | Lumen inter-ch. samples/s (median) | × realtime | vs ffmpeg CPU (coarse) |
|---|---|---|---|
| FLAC 88.2 kHz 24-bit st (claxon) | 46 421 260 | 526.3× | **~0.9–1.05× (parity)** |
| MP3 11.025 kHz st | 71 675 908 | 3 251× | 3.2× slower |
| MP2 44.1 kHz st | 105 655 092 | 1 198× | 2.7× slower |
| Vorbis 8 kHz st (ogg demux incl.) | 24 152 607 | 1 510× | 3.0× slower |
| AAC-LC 96 kHz mono | 107 433 098 | 1 119× | 3.1× slower |
| ALAC 48 kHz st (mp4 demux incl.) | 133.1 MB/s decoded PCM | 503× | DNF (below ffmpeg's 1 ms CPU clock) |
| ADPCM-IMA 44.056 kHz st | 153 767 893 | 1 745× | ~2.3× slower (coarse) |
| ADPCM-MS 32 kHz st | 137 700 868 | 2 152× | ~2.1× slower (coarse) |

ffmpeg's own number includes demux + null-mux; Lumen's includes the packet
walk — pipeline-adjacent, labeled as such in the source doc. **Honest
reading: every landed audio decoder is comfortably faster than realtime
(526×–6 220×); the self-built lossy decoders are 2.3–4.6× ffmpeg's CPU time
on these lanes; claxon-FLAC sits at parity.**

### 4.2 Video decode throughput (`docs/perf/breadth.md` §3)

| lane | Lumen Mpix/s | ffmpeg Mpix/s | Lumen/ffmpeg | × realtime |
|---|---|---|---|---|
| MJPEG 0052 (1024×768 yuvj444p, 150 f) | 131 | 274 | **0.477×** | 2.77× |
| MJPEG 0073 uhd4k (3840×2160, 19 f) | 207 | 384 | **0.539×** | 2.08× |
| MJPEG 0124 (512×512, 337 f) | 125 | 265 | **0.472×** | 4.76× |
| MJPEG 0087 (1920×1080 yuvj420p, 77 f) | 378 | 553 | **0.685×** | 7.60× |
| AV1 1080p 12 f (rav1d 1.1.0 **asm OFF** — dependency policy) | 68.41 | (libdav1d, asm on) 112.9 fps | **0.292×** | — |

h264/hevc: **parsers only, no decoder** (hardware-path cards 5.F2/5.G2; doc 04
gap register) — no throughput number exists and none is invented.

The MJPEG improvement history (the 8.D2-class bounded fix that already
landed, pre-Phase 8): the X1b assemble-rewrite + pipeline-gap fix moved
0052 3.182 s → 0.902 s and uhd4k 6.619 s → 0.760 s (**8.7× end-to-end**;
0.239× → 2.08× realtime) with **bit-exact output preserved** — recorded in
`docs/perf/breadth.md` §3 and the X1a audit; the SSE4.1 SIMD IDCT pass-1
ablation is separately recorded as a **no-win (≈ parity, point estimates
slightly slow side)** in `docs/perf/mjpeg-simd-20261024.md`.

### 4.3 Pipeline-adjacent families (`docs/perf/breadth.md` §4–§6, `docs/perf/baseline.md`)

| family | result (median) | source |
|---|---|---|
| Open → `media.info-ready` (AVI lanes, N=10) | 2.48–2.49 ms | breadth §4 |
| Seek 50 % (AVI 1024×768, N=10) | 102.10 ms (warm 100.70 ms) | breadth §5 |
| Seek 50 % (AVI uhd4k, N=10) | 404.35 ms (warm 705.30 ms — unexplained warm-seek discrepancy flagged to the engine owner) | breadth §5 |
| mkv demux walk (3.69 MiB, 255 pkt) | 4.77 ms/file; 53 494 pkt/s; 0.11 alloc calls/pkt | breadth §6, raw `memaudit-0014-demux-throughput.txt` |
| Memory | peak-RSS + per-frame allocation audits committed | `docs/perf/memory-audit.md`, `memory-audit-20261023.md` |

Not measurable on the audited builds (breadth §7 honesty ledger): per-frame
TTF events (v1 catalog gap), seek on flac/mp3/mkv seek machineries (open-path
wiring, D-018 first vertical), ALAC vs ffmpeg ratio (below the 1 ms CPU
clock). The soak 4 h tier is wired as CI but its first real measurement is
**pending** (`docs/perf/soak-nightly-20261109.md`).

## 5. The ✓ / partial / ✗ summary (doc 12 §9.2 vocabulary)

| Scope | verdict | Evidence |
|---|---|---|
| VLC-differential decode parity, lossless integer PCM (AVI first vertical) | **✓** — bit-exact over VLC's delivered window on 29/29 comparable u8/s16 rows | §3 |
| VLC-differential decode parity, pcm_f32le | **✗ (honest parity gap)** — 0/6 pass the ratified RMS/peak tolerances; follow-up recorded | §3 |
| Audio decode, all landed codecs | **✓** faster than realtime (526×–6 220×); **partial** vs ffmpeg CPU on self-built lossy (2.3–4.6×) | §4.1 |
| MJPEG video decode | **partial** — 2.08×–7.60× realtime but 0.47×–0.69× ffmpeg wall-to-wall (RGBA assembly is the known cost) | §4.2 |
| AV1 video decode | **✗ (policy gap, not engineering)** — 0.292× because rav1d runs asm-OFF by dependency policy while libdav1d runs full asm | §4.2 |
| h264/hevc software decode | **✗** — no decoder exists (parsers only; hardware lanes per ADR-001, ADR-013 draft) | §4.2 |
| Wall-clock Lumen-vs-VLC, 3-OS reference hardware | **not measured** — blocked on runners/CI billing (user-owned); ffmpeg-oracle sandbox numbers stand in, clearly labeled | §1, §4 |

## 6. Tool runs executed for THIS document (both deterministic, non-timing)

- `cargo xtask parity-report` — 2026-11-10, this worktree: rows=252,
  windows=64/252 (25.4%), linux=64/252, macos=64/252, web=0/252;
  verified-only 0/252 on all four platforms (full table + statement:
  `docs/PARITY-REPORT.md`).
- `cargo xtask feature-audit` — 2026-11-10, this worktree: **PASS** (252
  rows, evidence present + path-resolved; two stale evidence-path spellings
  fixed in `docs/feature-matrix.toml`, commit `c9a89c6` — no status changed).

No timing benchmark was run for this report: the sandbox is contended
(shared 2-core machine, other agents may run concurrently — the standing
caveat in every perf doc above), so a fresh timing run here could not be
presented as clean evidence, and doc 12 §9.3 forbids selective re-runs.
New timing evidence belongs on the reference-hardware tier when the external
blockers clear (issue #55's blocked-by-external note).

## 7. Known limitations of this report

1. **No VLC wall-clock timing** — the single largest gap vs doc 12 §9.2's
   full scope; every timing row in §4 is vs **ffmpeg 7.1.5** on a shared
   sandbox, per the recorded §1.3 fallback. When 3-OS runners exist, the
   same harnesses (`tools/corpus/speed_bench.py`, `tools/perf/baseline.py`,
   `tools/perf/r1_ff_lanes.py`) run unchanged with VLC added as a runner.
2. **Differential comparable set is v1-narrow** — PCM-in-AVI only (35/500
   corpus rows); every other container was a non-comparable row in the
   audited build for stated reasons. pcm_s24le/s32le rows are deferred on
   VLC's missing `afile` dump formats (doc 10 §6.6 open `[VERIFY]`).
3. **19/35 differential inputs are fresh realizations** of lost
   runtime-random corpus files (same argvs, re-decoded both sides) — the
   2026-10-23 originals are unrecoverable (§2).
4. **The f32 deviation is not yet attributed** to a specific VLC float-chain
   stage (§3 follow-up) — the Lumen side is ffmpeg-equal on those rows.
5. **4 h soak measurement pending**; memory ceilings beyond the corpus-v4
   audit are not claimed.
6. Cross-references: release-specific limits (unsigned binaries, packaging
   `[VERIFY]`s) live in `docs/RELEASE.md` (wave `7.w22g`); the consolidated
   limit register is `docs/KNOWN-LIMITS.md`; the parity statement is
   `docs/PARITY-REPORT.md`.

## 8. Reproducing the underlying evidence

Differential: `tools/bench-vs-vlc/README.md` (mock/real/pending modes,
version-pin rule); per-row inputs and argvs in
`docs/corpus/corpus-big-manifest-20261022.json`; deviation math in
`scripts/vlc_f32_deviation.py`. Timing: the reproduce blocks in
`docs/perf/breadth.md` §8 and `docs/perf/baseline.md` §8 (binaries, corpora,
median-of-N disciplines). Raw data: `docs/perf/raw/**`,
`docs/corpus/vlc-oracle-run-20261109.json`.
