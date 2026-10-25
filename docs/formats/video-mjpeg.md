# Video record — Motion-JPEG (decode)

Records-consolidation entry (card 5.REC-2; engine evidence from card
2.B4 first-vertical + card 5.H3 polish). Front-matter keys per doc 04
§6:

- id = `codec-video-mjpeg` (matrix row already `implemented` on
  win/linux/macos with 2.B4 evidence; this record consolidates the
  5.H3 additions — evidence backfill proposed)
- status = **implemented** (baseline sequential DCT JPEG-in-video,
  AVI1.1 interlace fields, 4:2:0/4:2:2/4:4:4)
- phase = 2 (first vertical, D-018) + 5 (5.H3 polish)
- fuzz_target = `fuzz_codec_mjpeg` (registered per doc 04 §8.5;
  survival suite green; libFuzzer CI Lead-owned)
- crates = [] — **in-house, zero dependencies** (D-018; NOT
  jpeg-decoder/zune-jpeg — their [VERIFY-P0] audits were never
  evidenced and are recorded as not-performed)
- self_built = [`codecs::video::mjpeg` (decoder/parser/mod/field/
  idct_simd — 3 124 src + 1 423 test LOC at 2.B4)]
- last_audit = 2026-10-24 (this consolidation; 2.B4 evidence
  2026-10-1x, 5.H3 2026-10-2x)

## Parse / Decode / Encode

- Baseline sequential DCT, Huffman, 8-bit, 1/3 components, sampling
  1..=2 per axis, one interleaved scan, restart markers (DRI/RST,
  mid-MCU-row and row-boundary intervals bit-exact vs restart-free).
  Output: I420 for 4:2:0/grayscale (neutral-128 chroma — documented
  representation), RGBA8 for 4:4:4/4:2:2/4:4:0 with box
  (nearest-neighbor) chroma upsampling.
- **AVI1.1 (5.H3):** APP0 `AVI1\0` + polarity byte, vocab 0/1/2
  (FirstField/SecondField/Interleaved); out-of-vocab and
  missing-polarity-byte are typed structural errors; Interleaved is
  metadata-only (bit-exact vs marker-free decode); single-field
  frames decode at field height with polarity surfaced — weaving is
  presentation-layer. ffmpeg 7.1 neither writes nor parses AVI1
  (verified absent from libavcodec) — pinned by the hand-built suite.
- Typed-Unsupported (doc 04 gap list): progressive (SOF2)/extended/
  lossless/arithmetic, 12-bit, 2/4-component (CMYK/YCCK),
  non-interleaved scans, spectral selection, sampling factors 3–4.
- Encode: out of scope.

## Evidence (Tasks 2.B4 + 5.H3)

- **2.B4 oracle cross-check (ffmpeg 7.1.5):** 120/120 frames (AVI +
  MP4 parents) decode through the codec-api path and hash-match the
  committed reference digests; luma byte-exact under BOTH ffmpeg IDCTs
  (0/4 608 000 samples differ); chroma max |Δ|=1 on 1.068% of samples
  (integer-IDCT rounding class; reference = our output, the exact
  algebraic inverse of the 2.A3 encoder's Q16 FDCT). The salvage DQT
  de-zigzag permutation bug was found by this oracle and fixed with a
  distinct-value regression test.
- **5.H3 differential (re-measured, ffmpeg 7.1.5, 13 corpus avi-mjpeg
  + 4 generated):** 0015 (146f) and 0125 (8f) **bit-exact** (MD5
  equal, incl. 0015's damaged EOI-truncated tail); all other I420
  lanes within **±1 LSB** (0.056%–1.26% of samples); 444→RGBA max 3;
  422→RGBA max 4 (43.4% on 0064 — neighbor-upsample phase + conversion
  tables, the recorded convention divergence); interlaced tff/bff
  field order byte-preserved. 17-lane table pinned in
  `tests/mjpeg_field_differential.rs`.
- **Tooling honesty (5.H3):** two earlier sweep passes reported
  max-255 garbage — root-caused to the compare script's non-natural
  frame sort + an ffmpeg-stdin-eating loop; ffmpeg verified
  self-consistent before the clean numbers were pinned.
- **Fuzz:** 22 fixture files, 120 extracted frames, systematic
  truncations (every 128 B) + flips (every 64 B) of frame 0,
  ≈8.38 MiP near-cap survivor with concealment, past-cap typed
  rejection; 5.H3 added all-256-polarity AVI1 mutations, DRI/RST
  renumber/drop/fill damage — Ok-or-typed, zero panics.
- **Tests at gates:** 2.B4 workspace 881/0/3; 5.H3 codecs 371/0
  (mjpeg_field_differential 3/3, 31 s, live corpus).

## Honest gaps

- "Fancy" (libjpeg triangular) upsampling for 422/444 — box
  replication is the recorded convention (RGBA lanes above).
- Colorimetry = JFIF default (BT.601 full-range) `[PROPOSAL]`;
  Adobe APP14 and friends unparsed.
- wasm32/web: MJPEG is not in the WebCodecs registry — web column
  stays a Phase-7 assessment (matrix note).
- 12-bit/progressive/arithmetic etc. = typed refusals (list above);
  VLC oracle: PENDING-INFRA (ffmpeg is the always-on oracle).

## Next steps

1. R3 registration row (mjpeg; cover-art mjpeg from MP4 attached-pic
   and AVI movi lanes already ride the decoder).
2. Web-column assessment at Phase 7; optional fancy-upsampling card
   (S) if RGBA-lane divergence matters to output parity.
