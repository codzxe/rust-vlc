# deps/records/rust_h264-0.6.1.md — rust_h264 0.6.1 (default features, the H.264 software track)

> **Class:** P0 validate-then-admit record (ADR-001's conformance-gated
> software track; issue #2's audited candidate, admitted on the Lead's
> explicit 2026-11-15 instruction — `codec-video-h264-software` was the
> last P0 `blocked` video row). The candidate-evidence half is the
> issue-#2 admission-audit record
> (`h264-software-decoder-audits.md`, wave 15: language/graph/unsafe
> inventory, verdict RECOMMEND for `rust_h264` 0.6.1); THIS record is
> the admission itself: the differential evidence (§D), the pin, and
> the boundary. Follows the rusty_vp9-0.1.3.md shape (doc 11 §2,
> fields R1–R10 + F1–F7).

## The pin (the whole point of this record)

- **Admission shape: `rust_h264 = { version = "=0.6.1" }`, default
  features (`std` only).** The optional features (`minifb`, `pprof`,
  `profile`, `dev-internals`, `portable-simd`) are example/bench
  surfaces and nightly-only experiments — all stay OFF; the pinned
  graph resolves to exactly one node.
- Version pin `=0.6.1` (exact): 0.x crate (R8 churn risk — a bump
  re-triggers this record).

## R1 — Language & source (F1, F7)

- Pure Rust (crates.io description: "Pure Rust H.264/AVC video
  decoder"). Tarball: 100 % `.rs`, zero `.c`/`.h`/`.S`/`.asm`/`.a`
  blobs, **no `build.rs`** (F2 vacuously clean), **0 `extern "C"`**.
- License `MIT OR Apache-2.0` (both files ship in the tarball).
  MSRV 1.89, edition 2021. Repo:
  https://github.com/roticv/rust_h264.
- `unsafe` surface (the issue-#2 audit's R5 inventory, re-verified
  against the 0.6.1 tarball this session): **3 `unsafe impl` + 26
  `unsafe fn` + 31 blocks, all `pub(crate)`-internal** —
  `simd/x86.rs` (SSE2/SSSE3/AVX2/AVX-512, runtime-detected),
  `simd/neon.rs` (aarch64), `once.rs` (a `std` re-export under the
  default feature), `picture_sync.rs`/`dpb.rs` (the threaded
  decoder's single-writer plane contract). **None reachable from the
  public API** — and this admission's wrapper only uses the
  single-threaded `OrderedDecoder` + the NAL framing entry points
  (all safe Rust).

## R2 — Transitive graph (F5)

```
rust_h264 v0.6.1        ← LEAF. ZERO dependencies (default `std`).
```

## R3 — Build hygiene (F2)

No build script; nothing to sabotage-test. `cargo tree` prints one
node. No proc-macros, no `links`, no native libraries.

## R4 — Provenance & maintenance

Publisher roticv; the 0.6.1 line's changelog is entirely
fuzz/robustness hardening (all SPS/PPS/slice-header fields
range-checked; bitstream reads pad-bounded). The issue-#2 audit
measured the candidate family on 2026-11-09; this admission's
differential (§D) ran on 2026-11-15 against ffmpeg 7.1.5.

## D — The differential (admission condition: bit-exact vs ffmpeg)

The binding evidence is `crates/codecs/tests/h264_sw_differential.rs`
(the rusty_vp9 lane pattern): six libx264-authored lanes (testsrc2,
30 fps, 8-bit 4:2:0), each decoded through **the wired
`H264SwDecoder`** on BOTH entry forms — Annex-B ES (the real
`demux::es` h264 demuxer, in-band parameter sets) and MP4/AVCC (the
real `demux::mp4`, `avcC` `extra_data` seeding) — with the
concatenated tight-I420 output MD5-compared against **ffmpeg
7.1.5's own decode** of the same fixtures (live re-check when ffmpeg
is on PATH; the pinned constants are that command's output):

| lane | profile | frames | size | MD5 (both forms) |
|------|---------|--------|------|------------------|
| A baseline I/P | Constrained Baseline (CAVLC) | 45 | 320x240 | `a0a9144b154d0403aa38429a85adce56` |
| B main I/P | Main (CABAC) | 45 | 320x240 | `19520a80eb3f586eca053d04ae90b129` |
| C main + B | Main (CABAC, bf=2) | 45 | 320x240 | `532c57170f56fcf4d20243b1242b88d3` |
| D high + B | High (bf=2) | 45 | 640x360 | `412bd350f062af2e35cffb45cfec7f9b` |
| E odd | Constrained Baseline, crop 128x96→126x94 | 30 | 126x94 | `d0fc84c9dc83f3df47e725db6d82c40f` |
| F still | Constrained Baseline | 1 | 320x240 | `810d6f44c4f4e3b9eae14fd9aece2a76` |

**Classification: bit-exact** (whole-dump MD5 equality, no tolerance
budget). The MP4/AVCC lane C additionally pins the PTS metas through
B-frame reordering (display-order strictly-increasing presentation
stamps). The hostile battery (the rusty_vp9 §H pattern): garbage
packets `Recovered` (damage counted, decoder usable), the
off-by-one truncation sweep panic-free, garbage-then-real recovers
to the full pinned dump, and the contract state rules (unconfigured
decode/flush, wrong descriptor, damaged `avcC`) answer typed.

Engine-side end-to-end (the CLI, this session): `lumen open
tests/fixtures/generated/valid/mp4_h264_aac.mp4` plays to
`Playing → Ended`, 100.0 %, exit 0 — the `mp4_h264_family_lane`
conformance row pins it.

## R7/R8 — Policy deviations & risk

- NOT `#![forbid(unsafe_code)]` — same honest deviation class as
  rusty_vp9/rav1d (SIMD-kernel unsafe, unreachable from the public
  API; the audit's R5 inventory above).
- **The boundary stays OURS** (the rusty_vp9 admission's rule): the
  NAL framing, EPB unescape, `avcC` parse and damage accounting ride
  the in-house 5.F1 parser (`AnnexBNals`/`AvcNals`/
  `unescape_rbsp_into`) — the crate's own splitters are never used,
  so the project's total-on-any-byte-string + count-every-skip rules
  hold at the wrapper seam.
- Profile coverage (honest): the engine decodes Baseline/Main/High
  8-bit 4:2:0 (+ monochrome as 4:2:0 with neutral chroma) incl.
  B-frames/B-pyramid, MBAFF/PAFF (weaved to progressive frames),
  weighted prediction, MMCO long-term refs. 4:4:4, 4:2:2, 10-bit and
  lossless (transform bypass) decode in the engine but refuse at
  THIS wrapper's output vocabulary (typed `h264.sw.surface_format`,
  the `vp9.bitdepth` precedent) — PixelFormat extensions are the
  follow-up card. SP/SI slices and FMO slice groups refuse typed
  (`h264.sw.unsupported`) — the engine's own rule string rides the
  error source.

## F1–F7 (the checklist)

F1 Rust: YES. F2 build.rs: NONE. F3 proc-macros: NONE. F4 native
libs: NONE. F5 graph: ZERO deps. F6 license: MIT OR Apache-2.0 (both
files in tarball). F7 bundled non-Rust engine: NO.

## Registration (the Lead's 2026-11-15 instruction)

- `crates/codecs/Cargo.toml`: `rust_h264 = { version = "=0.6.1",
  optional = true }` behind `codec-h264-sw` (default-on; the
  in-house parser/hw modules stay unconditional).
- `boundaries.toml` `[crates.codecs] external_allowlist`:
  `rust_h264` appended (this record is the allowlist's evidence).
- `deny.toml`: no change (MIT OR Apache-2.0, already-allowed
  expressions).
