# Video record — AV1 (decode)

Records-consolidation entry (card 5.REC-2; engine evidence from card
5.E1, finisher P5-E1F2; dependency record
`docs/dependencies/records/rav1d-1.1.0.md`). Front-matter keys per
doc 04 §6:

- id = `codec-video-av1` (matrix row `not_started` + empty `evidence`
  is stale vs the landed decoder — backfill proposed)
- status = **implemented** (8-bit 4:2:0 software decode via rav1d
  asm-off; >8-bit and non-420 = typed refusals)
- phase = 5 (landed; conformance breadth = the un-landed E2 card)
- fuzz_target = `fuzz_codec_av1` (in-tree fuzz-lite; libFuzzer CI
  Lead-owned)
- crates = [`rav1d =1.1.0 (default-features = false, features =
  ["bitdepth_8","bitdepth_16"])`] — asm-off pin; purity proven by the
  sabotage build (below)
- self_built = [`codecs::video::av1` (decoder/obu — OBU framing,
  temporal-unit census, the reference-state gate, delivery plumbing)]
- last_audit = 2026-10-24 (this consolidation; engine evidence
  2026-10-2x per 5.E1; sabotage build executed 2026-10-26)

## Parse / Decode / Encode

- Decode: `Av1Decoder`-path feeding rav1d temporal units. The
  **reference-state gate** contains the known upstream hazard
  (rav1d 1.1.0's `decode.rs::on_error` unwraps a `None` frame header
  when a dependent frame meets broken reference state — C dav1d
  errors gracefully, the Rust port aborts): the engine is fed a TU
  only after an independent frame peeks true (KEY/INTRA_ONLY via a
  3-bit frame-header peek; misclassification degrades to refusal, not
  mis-decode) and any corrupt-class outcome re-closes the gate.
  Gated packets return `Recovered{Discontinuity}`, typed + counted,
  never an abort. Retirement condition recorded in the module.
- Output: 8-bit I420 frames; pts restored via pending-map association
  from demuxer stamps. Encode: out of scope (rav1e is encoder-only,
  doc 04 §4.4).

## Evidence (Task 5.E1)

- **Differential (measured, all ffmpeg 7.1.5 libaom-authored,
  ffprobe verified):** 5 fixtures (testsrc2_320x240_60f,
  smpte_160x128_18f, rgb_192x108_6f, tenbit_128x96_5f,
  testsrc2_g6_192x108_18f). Frame counts + dims + pts ladders exact
  on all 5; **pixel MD5 bit-exact vs live `ffmpeg -f rawvideo
  -pix_fmt yuv420p`** for the three 8-bit fixtures
  (`275082414fa880cc72a4c52f71875953` / `fbedc17b688665d981b917380e83e2c1`
  / `f153f1d34f1c8828a8c061fefc100d74`); **10-bit = typed
  `av1.bitdepth` Unsupported ×5, zero fabricated frames**.
- **Corruption recovery (measured):** mid-stream forbidden-bit at
  packet 9 of g6 = 1 typed Corrupt + 2 gate-skips + resync at packet
  12; 15/18 frames survive at stream geometry with their own pts.
- **Fuzz-lite:** 3 000 random (<200 KB) + 3 000 structure-aware
  mutations of a real TU through OBU framer AND full decode — zero
  panics, every outcome typed or Ok, all emitted frames well-formed
  I420.
- **Purity (sabotage build, executed 2026-10-26):** scratch crate,
  exact feature pin, `CC=/nonexistent-cc AR=/nonexistent-ar cargo
  build`, nasm absent from PATH → **BUILD SUCCEEDED** — build.rs
  `main` is `#[cfg(feature = "asm")]`-gated, a compile-time no-op with
  asm off. Doc 11 §2.4 `[VERIFY-P0]` closed by direct experiment.
- **Tests at card gate:** av1_differential 6/6; full codecs suite
  green (fmt + clippy clean).

## Honest gaps

- **10/12-bit + 4:2:2/4:4:4 pixel fidelity:** typed refusals are the
  v0 boundary — the 16-bit planar frame vocabulary is the recorded
  follow-up (E2-class work).
- **Decode speed asm-off: unmeasured** (doc 12 perf card owns it).
- **Reduced-still-picture header layout:** `[VERIFY-P1]` in obu.rs —
  re-trigger on the next rav1d bump.
- **Container composition:** IVF-AV1 is the proven delivery (bit-exact
  MD5, 5.I3b); Matroska/AV01 + MP4/av01 consumers ride their
  container cards re-framing TUs into this decoder unchanged
  (recorded, not yet differentially run).
- wasm32 build unverified; libFuzzer CI unwired.

## Next steps

1. E2 conformance-breadth card (the remaining AV1 parent card):
   10-bit vocabulary, more profiles/containers.
2. R3 registration row; perf measurement at the R-01/doc-12 card.
3. Monitor rav1d releases (asm-off pin re-check + the
   reduced-still-picture VERIFY).
