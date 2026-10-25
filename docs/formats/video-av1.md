# Codec record — AV1 (AOMedia Video 1) decode

Decision record (card 5.E2; the decode engine itself is 5.E1, landed).
Front-matter keys per doc 04 §6:

- id = `codec-video-av1` (matches `docs/feature-matrix.toml`, whose
  row reads `status = not_started` ×4 — this record is the evidence
  for the Lead's matrix pass to flip it to implemented)
- status = **implemented** (8-bit 4:2:0 software decode via rav1d
  1.1.0 asm-off; boundary refusals below are typed, never silent)
- phase = 5 (matrix `phase_target = 5` — landed on schedule)
- fuzz_target = none yet as a standalone binary; the fuzz-lite sweep
  (random + structure-aware mutations through OBU framer and the
  full decode path, "typed outcome or Ok, zero panics") lives in
  `crates/codecs/tests/av1_differential.rs::fuzz_lite_*` (5.E1) — a
  dedicated `fuzz_codec_av1` cargo-fuzz target is Phase-6 fuzz-sweep
  material
- crates = [`rav1d 1.1.0 (MIT OR Apache-2.0, pinned =1.1.0,
  default-features = false, features = ["bitdepth_8", "bitdepth_16"],
  **asm OFF**)`]
- self_built = [`codecs::video::av1::obu` (OBU temporal-unit framing
  validator, typed `av1.obu.*` damage pre-engine)]
- last_audit = 2026-10-26 (5.E1's dependency record
  `docs/dependencies/records/rav1d-1.1.0.md`: sparse-index query,
  tarball sha256 match, build.rs `#[cfg(feature = "asm")]` no-op
  proof, `CC=/nonexistent-cc` sabotage build, license + advisory
  census)

## Parse / Decode / Encode

- **Decode (landed 5.E1):** `codecs::video::av1::Av1Decoder` behind
  the `codec_api::Decoder` contract. Engine: rav1d 1.1.0 — the
  Rust-port of dav1d — admitted with **`asm` OFF** (purity: the
  default-on `asm` feature compiles 174 shipped native `.asm`/`.S`
  files via `cc`/`nasm-rs`, F2/F7 fail, doc 11 §2.4). The engine seam
  is the repo's second sanctioned `unsafe` exception (rav1d's only
  public surface is its `dav1d_*` C ABI). Our OBU validator runs
  first: every temporal unit is framing-checked (forbidden bit, OBU
  size vs remaining bytes, empty TU, per-TU OBU census) so typed
  `av1.obu.*` corruption never reaches the engine.
- **Delivery plumbing:** IVF (`demux::ivf`, 5.I3b) is landed —
  `AV01` fourcc → descriptor `av1` → one temporal unit per packet,
  byte-for-byte passthrough. Matroska/WebM `AV01` and MP4 `av01`
  tracks re-frame to the same TU contract in their container cards.
  Ogg-AV1: no mapping exists (no `Av1` in the Ogg codec table —
  verified 5.E1).
- **Encode:** out of scope v1 (rav1e is not admitted; no card).
- **Reference-state containment:** rav1d 1.1.0 has an upstream
  missing-reference abort (recorded in the decoder module docs) — our
  reference-state gate contains it: after a dropped TU, dependent
  packets are `Recovered { Discontinuity }`-skipped until the next
  independent frame, then decode resumes (5.E1 differential proves
  15/18 frames survive a mid-stream corruption).

## Bit depths / layouts

v0 output boundary — **8-bit 4:2:0 → `PixelFormat::I420`**; pooled
copy-out per frame. Typed `ErrorKind::Unsupported` refusals (never a
mis-decode, never silence):

| stream class | refusal site |
|---|---|
| 10/12-bit 4:2:0 (profile 0/2) | `av1.bitdepth` |
| 4:2:2 / 4:4:4 any depth, monochrome-class layouts | `av1.layout` |

Conformance for the boundary (5.E2): the 10-bit and 4:4:4 matrix rows
assert every packet refuses with exactly these codes and zero frames
are fabricated. A 16-bit planar `media_types` vocabulary is the
recorded follow-up (would unlock 10/12-bit output).

## Tracks / Seeking / Timestamps

Per-container (IVF landed: pts 0..n−1 ladder restored exactly through
the `Dav1dDataProps::timestamp` → pending-map association — pinned by
both AV1 test binaries). Frame reordering drain on `flush()`. Seek
granularity = container keyframe index; OBU validator re-syncs at the
next independent frame.

## Corrupt-input behavior

OBU framing damage → typed `av1.obu.*` `Corrupt`, per-packet, engine
never sees the bytes; mid-stream damage → reference-state gate skips
dependents, resync at next independent frame (above). Fuzz-lite
(5.E1): 6000 inputs (random + mutated real TUs) × full decode path —
zero panics, every outcome typed or Ok. No crash, no abort, no
fabricated frame observed.

## Limits

Sequence-level limits are the engine's (AV1 spec: ≤ 8192×4384 at
seq_level_idx for common profiles); tested here to 1920×1080 (the
largest conformance row + the speed fixture). Memory: pooled output
slots (decoder.rs), bounded per-session by the ThreadBudget contract.

## Platforms

Pure software, no OS-ABI on the decode path (the one `unsafe` is the
in-process C ABI of a pure-Rust-engine build with asm off — not an OS
decoder). windows/linux/macos: identical bit-exact behavior expected
(same scalar code); tested on this Linux sandbox (ffmpeg oracle).
wasm32: not yet run — recorded gap, Phase-6 web column (no WebCodecs
needed for the software path; the asm-off scalar engine is
wasm-compatible by construction, unproven here).

## Corpus / conformance evidence (card 5.E2)

The official AOM test vectors are not available offline; the honest
conformance set is the **systematic ffmpeg-generation matrix**
(`crates/codecs/tests/av1_conformance.rs`, 15 rows, oracle = the same
ffmpeg 7.1.5-0+deb13u1 build that authors each file):

| axis | coverage | result |
|---|---|---|
| resolution | 64x64, 128x96, 320x240, 854x480, 1280x720, 1920x1080 | 6/6 pixel-MD5 exact |
| keyframe interval | g=1 (all-intra), g=6, g=240 (single key) | 3/3 exact |
| tiles | 1 (default), 2x1, 2x2 (`-tiles`) | 3/3 exact |
| encoder speed | `-cpu-used 8`, `-cpu-used 4` (LR on by default), cpu-4+`-enable-restoration 0` (LR off) | 3/3 exact |
| output boundary | 10-bit 4:2:0, 8-bit 4:4:4 | typed `av1.bitdepth` / `av1.layout` ×every packet, 0 frames |

"Exact" = demux packet count == ffprobe `nb_frames` == decoded frame
count, dimensions == ffprobe per frame, pts ladder restored, and our
concatenated I420 MD5 == the live ffmpeg yuv420p dump MD5 of the same
file. Rows run live per invocation (generation is the oracle; ffmpeg
absent ⇒ SKIP with message). The always-on committed surface (5.E1,
`tests/fixtures/av1/`, ≈39 KB): 5 fixtures with pinned pixel-MD5s +
live-oracle fallback.

Honest scope notes: `-cpu-used 0..3` rows dropped (encode cost
measured >100 s wall for 5 frames @320x240 cpu-used 0 in this sandbox
— beyond the card budget; the cpu-4 rows carry the decoder tool mix);
ffprobe 7.1.5 exposes no tile count, so tile rows are pinned by
encoder flags + decode exactness, not by independent tile
introspection. **Restoration flag probe (5.E2):** `-enable-
restoration 1` is a no-op in this build — libaom enables loop
restoration by default at cpu-used 4 and 8, so the flag-on encodes
are byte-identical to default (md5-pinned during the card) — meaning
every default matrix row already carries LR. The explicit
`-enable-restoration 0` row produces a genuinely different bitstream
(file MD5 `2ed26999…` vs `5ebceaba…` for the same row spec) whose
decode is **pixel-identical** on this testsrc2/crf-40 content (same
pixel MD5 as the default row) — it exercises a second bitstream, not
a second pixel output; both decode bit-exact vs their own ffmpeg
dumps.

## Results

- **Differential vs ffmpeg (authoring oracle): bit-exact.** 5/5
  committed fixtures (5.E1) + 13/13 exact matrix rows (5.E2) — pixel
  MD5 equality on every frame, zero tolerance.
- **Differential vs VLC 3.0.23: PENDING INFRA** (established pattern,
  `vorbis_vlc_differential.rs`): no `vlc`/`cvlc` binary in the
  sandbox. AV1 output is integer-exact like FLAC (not float-lossy),
  so the VLC comparison the day the oracle exists is MD5 equality on
  the same fixtures. VLC's dav1d and our rav1d are the same decoder
  lineage (dav1d) — the comparison validates integration, not the
  arithmetic.
- **Speed (doc 12 §1.3 honest-gap rule; asm-off cost):** release
  build, 1920x1080 12-frame fixture (generation command recorded in
  `av1_bench.rs`), same machine/session, decoder attribution per
  doc 12 §6.4:
  - ours — rav1d 1.1.0 **asm off**, in-process IVF demux + decode +
    I420 copy-out: **32.9 fps ≈ 68.1 Mpix/s** (median of 5)
  - ffmpeg 7.1.5-0+deb13u1 `libdav1d` (asm on), full pipeline incl.
    process spawn (asymmetric, labeled): **112.7 fps**
  - **asm-off cost ratio: ≈ 0.29×** (we decode at roughly one third
    of dav1d-with-asm throughput even against an ffmpeg number that
    carries process-spawn overhead; the gap is the expected scalar-C
    penalty of the purity pin, to be attacked by the doc 12 §6
    SIMD program — never by re-enabling the forbidden `asm` feature)

## Known gaps (honest)

1. 10/12-bit and 4:2:2/4:4:4 output: typed refusal (above) until the
   16-bit planar media-types vocabulary lands.
2. wasm32: untested.
3. Standalone cargo-fuzz target: not yet (fuzz-lite only).
4. Reduced-still-picture profile / still-picture mode: not exercised
   by the matrix (no ffmpeg-authored fixture class for it).
5. `-cpu-used 0..3` encoder tool mix: unexercised (cost, above).
6. Loop-restoration ON-vs-OFF output difference is covered as a
   decode-exactness axis only (both rows bit-exact vs ffmpeg); no
   LR-internals introspection exists.
7. Ogg-AV1, Matroska/MP4 AV1 track plumbing: container cards.
8. Tile-count introspection: not independently verified (encoder-flag
   pinned only).

*Evidence dates: 5.E1 2026-10-26; this record's matrix + speed run
2026-10-26, ffmpeg 7.1.5-0+deb13u1 sandbox build, `libaom-av1` encoder +
`libdav1d` decoder.*
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
