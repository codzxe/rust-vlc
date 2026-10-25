# Container record — IVF (DKIF; AV1/VP9 delivery container)

Status: **implemented** (demux, card 5.I3b). Feature-matrix row
`container-ivf` still reads `not_started` with empty evidence — this
record is that evidence; amendment recorded in-file for the Lead
(docs-only card 5.REC-1). Front-matter keys per doc 04 §6:

- id = `container-ivf` (`docs/feature-matrix.toml:336`)
- status = implemented (demux); platform-independent code, evidence
  Linux-only
- phase = 5 (landed)
- fuzz_target = `fuzz_demux_ivf` (`ivf/fuzz.rs` — open/drain/seek
  pass, 500K event budget; + seeded-LCG fuzz-lite ×512)
- crates = [] — zero external deps
- self_built = [`crates/demux/src/ivf/**` (mod/error/test_support/
  tests/fuzz)]
- last_audit = 2026-10-24

## Scope landed (5.I3b)

- Header: DKIF magic, hlen == 32, nonzero dims/rate; fourcc gate
  **AV01 → "av1", VP90 → "vp9"** (everything else = typed
  `unsupported-fourcc` with the fourcc bytes in the sniffed hint —
  v0 scope).
- Field semantics pinned empirically (ffmpeg 7.1.5 hexdump +
  ffprobe, not memory): @16 rate = fps numerator, @20 scale →
  **one tick = scale/rate seconds**; pts in ticks.
- Walk: 12-byte frame headers (u32 size + u64 pts); payloads pass
  through **byte-for-byte** (no OBU/superframe parse — decode
  domain); zero-size frames emitted (ffmpeg does too); VFR pts gaps
  passed through; declared frame count advisory (walk is truth).
- Codec honesty: packets carry keyframe=false + dts=Missing (IVF
  declares neither — ffprobe's `K_` on packet 0 is parser-derived,
  excluded from the differential's compared columns).
- Seek: `SlowLinear` — bounded payload-free frame-header scan
  (2M-frame budget); lands on last frame with pts ≤ target, clamps
  at extremes; reanchor compute-then-commit (failures leave the
  walk untouched, unit-pinned).
- Bounds: size > 16 MiB ceiling refused BEFORE allocation; u64 pts >
  i64::MAX typed `pts-overflow`; pool 2×4 MiB slots.

## Corpus evidence (5.I3b, ffmpeg 7.1.5 — 2/2 bit-exact)

**No IVF lane exists in corpus-v4** — the committed ffmpeg-generated
fixtures are the honest reference set (≈4.5 KB total,
`tests/fixtures/generated/ivf/`, pinned by
`tests/ivf_fixtures.rs` through `input`'s FileAccess):

| Fixture (lavfi testsrc2 → `-f ivf`) | Codec/fourcc | Dims | fps | Frames ours = ffprobe | pts ladder | Payload MD5 ours = `ffmpeg -c copy -f md5` |
|---|---|---|---|---|---|---|
| av1_64x48_30fps.ivf (libaom v3.12.1) | av1/AV01 | 64×48 | 30/1 | 30 = 30 | 0..29 = 0..29 | 0db6161a… ✓ |
| vp9_64x48_25fps.ivf (libvpx) | vp9/VP90 | 64×48 | 25/1 | 25 = 25 | 0..24 = 0..24 | e62f0aba… ✓ |

## ffmpeg-behavior probes (crafted shapes, recorded — 5.I3b)

Five divergences, all documented in the integration suite's header
docs: truncated final frame → ffmpeg SHORT packet vs our typed
refusal; header-only file → ffmpeg clean-empty vs our typed
`no-frames` (AVI no-movi / ADIF no-data precedent); hlen=64 →
ffmpeg "Invalid data" vs our typed `bad-header`; zero rate →
ffmpeg "Invalid frame rate" = our `bad-header`; VP80 → ffmpeg
demuxes, ours = typed v0-scope refusal.

## Error paths / fuzz

Typed `demux.ivf.*` ×10: truncated, not-ivf, bad-header,
unsupported-fourcc, no-frames, frame-beyond-file, frame-too-large,
pts-overflow, not-started, seek-out-of-range — all sticky,
unit-pinned. Trailing sub-header fragment → bounded Recovered +
clean end. Fuzz-lite: bit-flip sweeps ×512 rounds over the demo
stream AND both real fixtures, full truncation sweeps, wild
size/pts — no panic/hang/unbounded allocation (pool ceiling pinned:
`pool_bytes() == 4` after a 16 MiB+ claim).

## Codec rides (later cards, cited)

**AV1 decode landed after this card**: rav1d 1.1.0 asm-off, pixel-MD5
bit-exact vs ffmpeg on 5 fixtures (5.E1, wave 7) — the IVF payload
contract (descriptor `av1`, byte-exact passthrough) is the delivery
seam. VP9 decode = blocked pending rusty_vp9 validation (5.P1-I
record `video-vp8-vp9.md`).

## Honest gaps (D-10, from 5.I3b)

- **VP8-in-IVF**: typed v0-scope refusal (ffmpeg-parity exists for
  a later card); same for other fourccs (H264-in-IVF etc.).
- Seek wall-time NOT benchmarked (structurally bounded: O(frames)
  12-byte header reads — 5.X1's bench if it ever matters).
- No real-world IVF corpus lane (test/delivery format; the 2
  generated fixtures are the reference set — stated).
- keyframe=false on all packets (IVF declares none) — consumers
  needing keyframe truth must derive it codec-side.

## Gates (5.I3b, exact)

`cargo fmt -p demux -- --check` CLEAN; clippy EXIT 0;
`cargo test -p demux --all-features` 437/0 (322 lib incl. 26 ivf +
4 ivf_fixtures integration + all prior suites). Merged wave-5 tree:
workspace 2752/0 (5.W5-CLOSE).

## Next steps

- Lead: matrix-row amendment (status/evidence; the row's
  "census-level [VERIFY]" note can cite this record).
- VP8-in-IVF only alongside a VP8 decode path (blocked today).
