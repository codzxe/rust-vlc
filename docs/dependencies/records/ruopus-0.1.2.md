# deps/records/ruopus-0.1.2.md — ruopus 0.1.2 (purity + differential, card 6.VTA-ruopus)

> **Class:** P0 validate-then-admit SPIKE (Phase 6 W1). Candidate Opus
> decoder (RFC 6716 + RFC 7845 Ogg mapping) — the Phase-5 exit gate named
> ruopus the opus-decode candidate. This is NOT a wiring card: ruopus is
> not added to `crates/codecs`. Everything below was measured in this
> session (2026-10-29) in the build sandbox against ruopus **0.1.2** (the
> latest and only release line) with our own ogg demuxer
> (`crates/demux/src/ogg`) supplying the packets. Harness:
> `tools/vta/ruopus/` (standalone, non-workspace).

## R1 — Version, language & source

- **ruopus 0.1.2** (crates.io, published 2026-07-01; the release line is
  3 days old at measurement: 0.1.1 on 2026-06-29, 0.1.2 on 2026-07-01;
  9,327 downloads). MSRV `rust-version = "1.85"` (≤ our 1.99.1). Edition
  2024. License **MIT** (single, `Cargo.toml` `license` key; `LICENSE`
  in tarball). Repo: https://github.com/jmg049/ruopus. Author: Jack
  Geraghty.
- Pure Rust, **no native files** in the vendored source
  (`~/.cargo/registry/src/*/ruopus-0.1.2`): scans for C/C++/asm source
  extensions → 0 hits. No `build.rs` (`ls` → absent; manifest `build =
  false`). No `links=` key. No `extern "C"` / `extern "system"` /
  `#[link]` in `src/` (grep → 0; the one textual `pyo3` mention is a
  doc comment in `src/python/mod.rs`, see R2). No binary blobs.
- **unsafe: 69 `unsafe`-token lines** across 5 files (rg count):
  `src/simd.rs` 47, `src/celt/vq_simd.rs` 18, `src/celt/mdct.rs` 2,
  `src/celt/vq.rs` 1, `src/lib.rs` 1 (the crate-level
  `unsafe_code = "deny"` lint line). All sites are hand-written
  `core::arch` SIMD kernels behind CPU-feature checks, each with a
  `// SAFETY:` justification enforced by clippy
  `undocumented_unsafe_blocks = "deny"`; the crate ships the
  authoritative list in `docs/unsafe.md`, which states the kernels sit
  on the **encoder** path (we are admitting the **decoder**). No raw
  pointers in public API, no FFI, no transmute.
- The tarball additionally ships a `python/` module + `pyproject.toml`
  (maturin bindings) — **not compiled** by the Rust build (feature-gated,
  see R2).

## R2 — Transitive graph (the one real trap)

- **Default features are unacceptable**: `default = ["std",
  "spectrograms"]`, and `spectrograms` drags in ndarray, matrixmultiply,
  rawpointer, rustfft, regex-automata, num-complex/num-integer/
  num-traits + autocfg — 23 locked packages for what should be a codec.
  The `python` feature pulls **pyo3 + numpy** (CPython extension ABI).
- **Admission shape (measured, this session):**
  ```toml
  ruopus = { version = "0.1.2", default-features = false, features = ["std"] }
  ```
  `cargo tree -p ruopus` then prints exactly `ruopus v0.1.2` — **zero
  normal dependencies** (libm/pyo3/numpy/spectrograms are all
  `optional = true`; `opus ^0.3` is dev-only). This is the dependency
  shape the M-unit MUST use; anything else reintroduces the graph above.

## R3/R4 — Build scripts & native links

- None: no build.rs, no links=, no extern declarations, no blobs, no
  pkg-config. F2/F3/F4/F6 all NO.

## R5 — Targets

- Pure Rust, `#![no_std]`-capable core (`categories` includes `no-std`;
  `std` feature adds alloc/std). No `cfg(target_*)` native gates in the
  decode path. wasm32 expected to compile the same files (not measured
  this session — listed as a gap).

## R6 — API surface relevant to the wiring unit

- `OpusDecoder::new(channels)` / `with_rate(fs_hz, channels)` —
  **panics** unless channels ∈ {1, 2} and rate ∈ {48, 24, 16, 12, 8} kHz
  (documented `# Panics`). The wrapper MUST validate from OpusHead
  before constructing (OpusHead channel count ≤ 2 → mono/stereo path).
- `decode_packet(&[u8]) -> Result<Vec<f32>, PacketError>` — one full
  Ogg/Opus data packet (TOC included) → interleaved f32 at the decoder
  rate (we use 48 kHz). **Returns `Result` for malformed packets** — the
  hostile-input contract is error-return, not panic (§Hostile below).
  0/1-byte packets are treated as DTX and return concealment of the last
  good frame size (RFC 6716 §3.2.1 behavior).
- `decode_lost(frame_size)` (concealment), `decode_fec`,
  `final_range()` (bit-exactness oracle per packet — usable by our
  conformance kit later).
- `MultistreamDecoder::with_rate(fs, streams, coupled, mapping)` +
  `decode_packet` — RFC 7845 mapping family 1 (surround). Exercised
  below on two 5.1 lanes.
- `decode_packet_i16` converts with `(x*32768).clamp().round_ties_even()`
  — matches libopus convention; NOT used in the differential (f32
  compared directly, see §D).

## D — Conformance differential (measured this session)

Pipeline: **our ogg demuxer** (`OggDemuxer::new` → `start()` → pump
`next_event` to `EndOfStream`; header packets are harvested during
pumping, then read via `header_packets(serial)`) → ruopus
`OpusDecoder`/`MultistreamDecoder` at 48 kHz → interleaved f32. **Pre-skip**
from OpusHead[10..12] discarded as `preskip × channels` leading decoded
samples (all six lanes measured preskip = 312; ffmpeg applies the same
via its ogg skip_samples side data). **Output gain** (OpusHead[14..16])
reported but not applied — every lane measured gain = 0, so this is
moot for the evidence below. **End-trim**: the harness does NOT trim the
tail to the last granulepos; ffmpeg does (both corpus lanes: ruopus
192,648 samples/ch after pre-skip vs ffmpeg 192,000 → 648-sample tail is
the Ogg granule end-trim; generated lanes 288,648 vs 288,000). The
comparisons below are over the min-length prefix (the untrimmed tail
affects sample counts, not the compared prefix). The M-unit wrapper MUST
implement granulepos-based end-trim — recorded as a wiring requirement.

Reference: `ffmpeg -i <file> -f f32le -ar 48000` (ffmpeg 7.1.5 native
`opus` decoder, float path). f32-vs-f32 — no quantization on either
side. **Opus bit-exactness is a fixed-point-decoder requirement; two
independent float decoders are not expected to be bit-identical**, so the
bar used here: (a) zero samples differing by more than one s16 LSB
(1/32768) ⇒ s16-audible-equivalence, (b) max abs deviation reported.

**Channel order (reference-side, not a ruopus defect):** ruopus emits
RFC 7845 Vorbis order (5.1: FL FC FR BL BR LFE); ffmpeg's native decoder
reorders to ffmpeg's native layout (FL FR FC LFE BL BR). The 5.1 rows
below apply that permutation to the reference before comparing; raw
MD5s differ for that reason alone.

| Lane | ch / mapping | pkts | dec errors | ruopus samples/ch (post-pre-skip) | ffmpeg samples/ch | max abs Δ | samples > 1 s16 LSB | verdict |
|---|---|---|---|---|---|---|---|---|
| corpus 0047-audio-only.ogg | 1 / fam 0 | 201 | 0 | 192,648 | 192,000 | **1.378e-06** | 0 / 192,000 | s16-equivalent |
| corpus 0077-audio-only.ogg | 6 / fam 1 | 201 | 0 | 192,648 | 192,000 | **4.381e-06** (after ch-order norm) | 0 / 1,152,000 | s16-equivalent |
| gen g_stereo_vbr (48k stereo VBR) | 2 / fam 0 | 301 | 0 | 288,648 | 288,000 | **8.583e-06** | 0 / 576,000 | s16-equivalent |
| gen g_mono24k (24k mono, voip) | 1 / fam 0 | 301 | 0 | 288,648 | 288,000 | **8.643e-06** | 0 / 288,000 | s16-equivalent |
| gen g_stereo_cbr96 (CBR 96k) | 2 / fam 0 | 301 | 0 | 288,648 | 288,000 | **7.695e-05** | 162 / 576,000 (0.028%) | NOT s16-exact — float divergence crosses an LSB boundary on 162 samples |
| gen g_51 (5.1 CBR 256k) | 6 / fam 1 | 201 | 0 | 192,648 | 192,000 | **1.219e-05** (after ch-order norm) | 0 / 1,152,000 | s16-equivalent |

Representative raw f32 MD5s (trimmed to the ffmpeg count, un-normalized
channel order): 0047 — ff `e918a5f757bd3ab2dbfc49ce3fff86fa` vs ruopus
`a3b022c6a01b113b24ee2246a9b8da50`; 0077 — ff
`d5a54d6ed8182eee8d57fd1cf9f04bda` vs ruopus
`cd0551e21c29279fcc1abceb4bebcc1f` (differs only by the channel-order
convention + float noise). **No lane is f32-bit-exact** (first
divergence at sample 0, Δ 3.8e-9 on 0047) — consistent with two
independent float implementations, ~-97 dB worst-case (1.2e-5) on five
of six lanes.

## H — Hostile input (measured this session)

Feeding a warm decoder (real prior state) six hostile packets, each
wrapped in `catch_unwind`: truncated TOC+body → `Err(InvalidFrameCount)`;
random garbage → `Err(InvalidFrameLength)`; zero-length → Ok, DTX
concealment (960 samples — RFC-correct); one-byte TOC-only → Ok, DTX
concealment; 4096 × 0xFF → `Err(InvalidFrameCount)`; 4096 × 0x00 →
`Err(FrameTooLarge)`. **Zero panics.** (Constructor asserts on
channels/rate are the documented API contract — wrapper-side
validation required, see R6.)

## Verdict: **ADMITTED** (with two binding conditions)

ruopus 0.1.2 is a pure-Rust (SIMD-kernel `unsafe` only, documented),
**zero-dependency** (with `default-features = false, features = ["std"]`
— condition 1, non-negotiable: default features pull a 23-package
spectrogram/ndarray graph and the `python` feature pulls pyo3) MIT opus
decoder whose differential against ffmpeg 7.1.5 across six lanes
(mono/stereo/5.1, VBR/CBR, our own ogg demuxer feeding it) shows float
deviation ≤ 1.2e-5 with **zero audibly-different samples** on 5 of 6
lanes and 0.028% LSB-boundary crossings on the CBR stereo lane; hostile
packets error cleanly. Condition 2: the wiring unit must validate
channels/rate from OpusHead before `OpusDecoder::with_rate` (panics
otherwise) and implement pre-skip (measured 312 samples on every lane)
+ granulepos end-trim (measured 648-sample tail on every lane) +
channel-order mapping at the audio-output seam (RFC 7845 Vorbis order
is what ruopus emits).

Honest gaps: (1) young release line — 0.1.2 is 3 days old at
measurement, single-digit k downloads, one author; pin exact version
and re-verify on bump. (2) f32-bit-exactness NOT established (nor
expected between float implementations; `final_range()` oracle exists
for a future fixed-point-conformance pass). (3) CBR-96k lane crosses
the s16 LSB boundary on 162/576,000 samples — s16 MD5 equality is NOT
a valid test criterion for this crate. (4) Pre-skip/end-trim/gain
measured only on gain-0 streams; a gain≠0 stream was not in the corpus
or generated set. (5) wasm32 build not measured. (6) FEC
(`decode_fec`) and concealment quality not differentially tested (no
loss lanes).

## Wiring plan (for the admission M-unit, if Lead confirms)

Mirror the `crates/codecs/src/audio/mp3/` wrapper shape
(`symphonia-mp3` precedent): `crates/codecs/src/audio/opus/` with a
`Decoder` wrapper holding `ruopus::OpusDecoder` (mono/stereo) or
`ruopus::MultistreamDecoder` (family 1), seeded from
`DecoderConfig::extra_data` = the OpusHead packet (the ogg open-path
unit's `TrackFact.extra_data` seam, per 6.OPW-PREP); configure()
validates channels/rate (panic contract), decode() maps
`PacketError` → the codecs error type; pre-skip discard in the wrapper
at first-packet, end-trim via the ogg granulepos duration; repo
dependency line exactly `ruopus = { version = "0.1.2",
default-features = false, features = ["std"] }` + the allowlist row
(Lead-owned). Playback channel-order mapping belongs at the audio
output seam, not the decoder.

---

## Addendum (2026-11-09) — admission status: ADMITTED IN-TREE (issue #7, W16-EE)

The spike verdict above ("ADMITTED, two binding conditions") is now
ratified by in-tree evidence; this addendum closes the spike-era gap
in this record (issue #7, admissions-wave follow-up):

- **Cargo.toml pin (Lead admission):** `ruopus = { version = "=0.1.2",
  default-features = false, features = ["std"] }` —
  `crates/codecs/Cargo.toml` (card 6.M-opus; the exact condition-1
  shape from §Verdict above, zero-dependency under `cargo tree`).
- **ADR-010:** the "Software decoders implemented" table carries the
  Opus row — `crates/codecs/src/audio/opus/**` + `ruopus =0.1.2`
  (card 6.M-opus; this record).
- **Binding-condition closure:** the 6.M-opus wrapper implemented
  OpusHead validation, pre-skip discard and granulepos end-trim
  (condition 2); channel-order mapping sits at the audio-output seam
  per the wiring plan above.
- **In-tree differential evidence (this session, 2026-11-09, branch
  w15/07-codec-admissions at main 62453fb):** `cargo test -p codecs`
  → 643 test cases across the lib + all 51 integration targets:
  **624 pass, 18 ignored (bench/throughput-class, each ignored test
  names its `--release -- --ignored` re-run command), 0 fail.**
  Opus-specific: `opus_corpus_differential` — **2/2 pass** (corpus
  lanes through our ogg demuxer against the ffmpeg reference).
- **Matrix:** the `codec-audio-opus` row in `docs/feature-matrix.toml`
  is flipped `blocked → implemented` in this same commit (was stale
  since the Phase-5 exit gate 5.R2-V, commit 3116aec — ADR-010
  "matrix drift" note).
- Per doc 11 §3.7: any version bump or feature change re-triggers
  this record.
