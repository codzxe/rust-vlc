# Audio record — MP3 / MP2 (MPEG-1/2/2.5 audio, Layers II+III)

Records-consolidation entry (card 5.REC-2; engine evidence from cards
5.A3 and 5.J3b). Front-matter keys per doc 04 §6:

- id = `codec-audio-mp3` (primary) + sibling `codec-audio-mp2`
  (both rows in `docs/feature-matrix.toml` carry `not_started` +
  empty `evidence` — stale vs the landed decoders; backfill proposed)
- status = **implemented** (probe/decode/seek/gapless both layers)
- phase = 5 (landed; MP3 wave 2, MP2 wave 5)
- fuzz_target = `fuzz_codec_mp3` / `fuzz_codec_mp2` (in-crate harness
  logic; libFuzzer CI wiring Lead-owned)
- crates = [`symphonia-bundle-mp3 =0.6.1 (MPL-2.0)`] — one admission
  covers the family (record `docs/dependencies/records/
  symphonia-mp3-0.6.1.md`; note: doc 04's `symphonia-bundle-mp3`
  spelling resolves to this crate family on crates.io)
- self_built = [`codecs::audio::mp3` (header/id3/xing/seek/decoder,
  34 unit tests at landing) + `codecs::audio::mp2` (dedicated Layer II
  surface: header/seek/decoder/fuzz, 27 unit tests)]
- last_audit = 2026-10-24 (this consolidation; MP3 evidence 5.A3,
  MP2 evidence 5.J3b, both 2026-10-2x)

## Parse / Decode / Encode

- **MP3 (5.A3):** MPEG-1/2/2.5 Layer III decode via the symphonia
  engine behind our header chain — ID3v2.2/2.4 head+interframe skip
  (syncsafe), Xing/Info/VBRI parse, CBR+VBR seek planning, and the
  gapless window (encoder-delay/padding trim via Info/LAME tag —
  latency shift/clamp unit-pinned).
- **MP2 (5.J3b):** dedicated `Mp2Decoder` with a strict Layer II gate
  (non-Layer-II frame = typed `mp2.header.not_layer_ii`, never silently
  decoded); MPEG-1 + MPEG-2/2.5 LSF bitrate tables cross-pinned against
  the family parser; CBR geometry gate (`mp2.stream.cbr_mismatch`);
  seek = CBR arithmetic both directions + `refine_to_frame` snapping to
  a real header (padding-bit slack < 64 B measured on the 44.1 kHz
  corpus stream); `extra_data` refused typed (`mp2.facts.unexpected` —
  Layer II has no Xing/LAME gapless construction).
- One packet = one exact-frame-size frame; pooled 4-slot output
  rotation, allocation-free per packet after warm-up.
- Encode: out of scope (roadmap Phase 7+).

## Evidence (Tasks 5.A3 + 5.J3b)

- **MP3 corpus differential (re-printed 2026-10-25, card
  6.MP3-reprint — closes Q2-V gap G-3):** fresh run of
  `cargo test -p codecs --test mp3_corpus_differential -- --nocapture`
  vs live ffmpeg 7.1.5, same gapless window on both sides (ffmpeg
  `-map 0:a -c:a pcm_s16le -f s16le -`; Lumen's +529-shifted trim —
  window contract in the test header). The table that was lost to
  the 5.A3 tool deadline, now with this session's measurements
  (samples are per-channel; MD5s are over the s16le window;
  diff% = samples differing out of all inter-channel samples):

  | lane | file | profile | rate/ch | samples/ch | differing (%, maxΔ) | Lumen s16le MD5 | ffmpeg s16le MD5 |
  |------|------|---------|---------|------------|---------------------|-----------------|------------------|
  | 1 | corpus media/0139-audio-only.mp3 | MPEG-2.5 LSF Layer III 64 kb/s | 11025/2 | 44100 | 176 (0.1995%, ±1) | `7620224ff543fd30ad982ec5d5ac1fca` | `87ffd7a83802f439d60fad3a91783515` |
  | 2 | corpus media/0145-cover-audio.mp3 | MPEG-2 LSF Layer III 160 kb/s | 24000/2 | 2304 | 12 (0.2604%, ±1) | `149a47c47e39c7636f763c1f0af1a9f5` | `98454c6c428e916f1f80db77073a4ab0` |
  | 3 | fixture cbr128.mp3 | MPEG-1 Layer III CBR 128 kb/s | 44100/2 | 88200 | 154 (0.0873%, ±1) | `52d32e3c30972087dadbd97998a0d3cd` | `253c926ff193cfe2db3a5b092acba2f4` |
  | 4 | fixture vbr_q4.mp3 | MPEG-1 Layer III LAME VBR q4 (Xing) | 44100/2 | 88200 | 254 (0.1440%, ±1) | `988161d1c81b5723c44ebab84fa2afb9` | `aa132c33436a959052f262e82d5d1e86` |
  | 5 | fixture mono48.mp3 | MPEG-1 Layer III mono | 48000/1 | 72000 | 114 (0.1583%, ±1) | `95e0c8a02a5a2cce9c6175165fdbaaa6` | `204e24ea83aee0b18df7b041dc898b16` |
  | 6 | fixture mpeg2_24k.mp3 | MPEG-2 LSF Layer III stereo | 24000/2 | 36000 | 58 (0.0806%, ±1) | `c808aa5c0f0af3eba1ab9401840bd0fa` | `bba85e8d4e9c1d0b6eb3f0dae13a37c9` |
  | 7 | fixture mp2_48k.mp2 | MPEG-1 Layer II stereo | 48000/2 | 72576 | 35526 (24.475%, ±1) | `6db7b51bbe177897a71e213bf1f3a8f2` | `bf3bee58ca7e7c34d791abeffb6f9bfb` |
  | 8 | fixture tagged_art.mp3 | lane 3's essence behind ID3v2.3 + art | 44100/2 | 88200 | 154 (0.0873%, ±1) | `52d32e3c30972087dadbd97998a0d3cd` | `253c926ff193cfe2db3a5b092acba2f4` |

  **Verdict: 8/8 lanes inside the ±1-LSB lossy tolerance** (max
  sample delta exactly 1 on every row — two independent lossy
  syntheses; Layer III rows differ on ≤0.26% of samples, the Layer II
  row on 24.5%, matching the MP2 record below). ffmpeg MD5s were
  re-derived independently this session (`ffmpeg -v error -i F -map
  0:a -c:a pcm_s16le -f s16le - | md5sum`) — byte-identical to the
  test's oracle output and to the recorded corpus reference. The
  corpus root's session-artifact `ref_*.raw`/`mine_*.raw` pairs
  (cbr128, mono48, vbr_q4, tagged_art, mp2_48k, mpeg2_24k) are NOT
  consumed by any in-tree test (rg over `crates/` finds no reader);
  cross-verified this session: every `ref_*.raw` MD5 equals the fresh
  ffmpeg MD5 and every `mine_*.raw` equals the fresh Lumen MD5 above
  (both sides byte-stable since 5.A3). What is pinned in-tree:
  `tests/mp3_corpus_differential.rs` re-derives every row against
  live ffmpeg 7.1.5 and FAILS on md5 drift from the recorded corpus
  reference; `tests/data/mp3_bitexact_reference.txt` records the
  fixture digests incl. the Layer II `max_sample_delta=1` row.
  Tests at merge: 251/0 (MP3 added ~43).
- **MP2 differential (measured, live ffmpeg 7.1.5):** 4 rows — corpus
  0005 (V2 LSF L2 80k 16k mono), corpus 0119 (V1 L2 384k 44.1k
  stereo), fixtures mp2_48k + mp2_lsf_22k. **Honest verdict: NOT
  MD5-bit-exact — max sample delta exactly ±1 LSB on every row**
  (~25% of samples off by one rounding step; two independent lossy
  Layer II syntheses). The card brief's "bit-exact" assumption was
  empirically wrong and recorded as such. Both sides' MD5s are pinned
  in-test as regression tripwires.
- **MP2 throughput (release, full pipeline):** 96.3–118.2 M
  inter-channel samples/s ≈ 602–6021× realtime.
- **MP2 tests:** 295 lib + integration suites green at card gate
  (27 new mp2); seek proven executable on the real padded corpus
  stream (0119).
- **Fuzz:** mp3 fuzz-lite + family survival; mp2 340+ mutants
  (flips/truncations/garbage-prefixes, header-biased) — all typed
  (`mp2.*`/`mp3.*`), zero panics.

## Honest gaps

- **VLC oracle: PENDING-INFRA** for both layers (no binary in sandbox;
  ffmpeg is the always-on oracle).
- **MP3 corpus MD5 table:** re-printed 2026-10-25 (card 6.MP3-reprint,
  Evidence above) — Q2-V gap G-3 closed.
- **Layer II joint stereo:** no ffmpeg encoder emits it —
  mode-extension/intensity-bound semantics unit-pinned on synthetic
  headers only, not differentially.
- Big-endian hosts (s16le byte order pinned by recorded MD5s);
  libFuzzer CI unwired; R3 must route descriptor `mp2` to
  `Mp2Decoder` (the family `Mp3Decoder` also accepts it — recorded
  behavior, unchanged).

## Next steps

1. R3 registry rows (snippets in the 5.A3/5.J3b worklog entries).
2. ~~Q2-V audit re-prints the MP3 corpus table~~ DONE 2026-10-25
   (card 6.MP3-reprint, G-3 closed — see the Evidence table).
3. VLC differential when the oracle exists (±1-LSB tolerance class
   for the lossy layers, matching the ffmpeg posture).
