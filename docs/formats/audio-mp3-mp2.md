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

- **MP3 corpus differential:** run live at card time (finisher report);
  the per-file PCM-MD5 **table was lost to a tool deadline** —
  `[VERIFY-P2]` the table must be re-printed from the test-run logs by
  the Q2-V audit (Lead-recorded in the 5.A3 entry). What is pinned
  in-tree today: `tests/mp3_corpus_differential.rs` re-derives every
  row against live ffmpeg 7.1.5 and FAILS on md5 drift from the
  recorded corpus reference; `tests/data/mp3_bitexact_reference.txt`
  records the fixture digests incl. the Layer II `max_sample_delta=1`
  row. Tests at merge: 251/0 (MP3 added ~43).
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
- **MP3 corpus MD5 table:** `[VERIFY-P2]` re-print owed (above).
- **Layer II joint stereo:** no ffmpeg encoder emits it —
  mode-extension/intensity-bound semantics unit-pinned on synthetic
  headers only, not differentially.
- Big-endian hosts (s16le byte order pinned by recorded MD5s);
  libFuzzer CI unwired; R3 must route descriptor `mp2` to
  `Mp2Decoder` (the family `Mp3Decoder` also accepts it — recorded
  behavior, unchanged).

## Next steps

1. R3 registry rows (snippets in the 5.A3/5.J3b worklog entries).
2. Q2-V audit re-prints the MP3 corpus table ([VERIFY-P2]).
3. VLC differential when the oracle exists (±1-LSB tolerance class
   for the lossy layers, matching the ffmpeg posture).
