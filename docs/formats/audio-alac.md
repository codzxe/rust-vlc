# Audio record — ALAC (Apple Lossless, decode)

Records-consolidation entry (card 5.REC-2; engine evidence from card
5.J3a, finisher P5-J3AF). Front-matter keys per doc 04 §6:

- id = `codec-audio-alac` (matrix row `not_started` + empty
  `evidence` is stale vs the landed decoder — backfill proposed)
- status = **implemented** (lossless decode, bit-exact)
- phase = 5 (landed)
- fuzz_target = `fuzz_codec_alac` (survival suites in-tree;
  libFuzzer CI Lead-owned)
- crates = [`symphonia-codec-alac =0.6.1 (MPL-2.0)`] — purity record
  `docs/dependencies/records/symphonia-alac-0.6.1.md`: no build.rs
  (F2 NO), no native links/blob, `#![forbid(unsafe_code)]`, tarball
  sha256 matched
- self_built = [`codecs::audio::alac` (decoder/error modules —
  cookie parse, typed taxonomy, pooled interleaved output)]
- last_audit = 2026-10-24 (this consolidation; engine evidence
  2026-10-2x, card 5.J3a)

## Parse / Decode / Encode

- Decode: `AlacDecoder` behind the codec-api contract; ALAC 'alac'
  cookie (frame length, compatible version, sample size, channels)
  parsed by us; engine = symphonia-codec-alac. 16/24-bit, mono +
  stereo, multi-frame packets, partial final frames. Output
  interleaved into pooled SharedBytes slots sized to the cookie frame
  max — zero per-frame allocation after warm-up.
- Encode: out of scope.

## Evidence (Task 5.J3a)

- **Differential (re-verified independently at card close, ffmpeg
  7.1.5):** all pinned interleaved-dump MD5 truths recomputed —
  **5/5 fixtures bit-exact** through the real MP4 composition:

  | fixture | shape | verdict |
  |---|---|---|
  | alac stereo16 | multi-frame (8 pkts) | bit-exact |
  | alac stereo16 | 9 pkts | bit-exact |
  | alac mono | 16-bit | bit-exact |
  | alac stereo24 | 24-bit | bit-exact |
  | alac_stereo24_noise.m4a (ADDED at finish) | 24-bit stereo, 13 pkts, white noise = max-entropy rice-escape stressor | bit-exact (`1ee6321067a82b421e33038f4fc00b70`) |

- **The suspected multi-frame bug: NOT REPRODUCED.** The prior
  session's rice-escape `read_unary_ones_capped(9)` concern was
  re-derived bit-for-bit vs ffmpeg's `get_unary_0_9` +
  `get_bits_long(bps)` (audit committed in decoder.rs docs — engine
  matches; one harmless trailing-`111`-END-tag under-consumption
  documented, zero output impact). NO engine patching, no #[ignore]
  rows — the pinned digests are the tripwire.
- **Throughput (measured, release, median of 7, FULL pipeline incl.
  MP4 demux):** stereo24 130.3 MB/s, stereo16 100.8 MB/s of decoded
  PCM (≈450× realtime at 48k stereo).
- **Fuzz-lite:** 512 packet + 256 cookie LCG mutation rounds —
  Ok-or-typed only, zero panics, mutations verified to bite.
- **Tests at card gate:** 372 passed / 0 failed / 15 ignored (all
  ignored = bench measurements per convention).

## Honest gaps

- **32-bit differential row: impossible with this ffmpeg** — its ALAC
  encoder quantizes s32p to 24 bits per raw sample; the S32 output
  width stays unit-tested only (recorded at card close).
- **VLC oracle: PENDING-INFRA** (family status; ffmpeg is the
  always-on oracle, and ALAC is lossless — any future VLC digests
  must be bit-exact).
- libFuzzer CI campaign unwired.

## Next steps

1. R3 registry row (snippet in the 5.J3a worklog entry).
2. VLC differential when the oracle exists (bit-exact class).
3. 32-bit coverage only via a non-ffmpeg reference author (recorded
   as a C-V6-class acquisition item, not scheduled).
