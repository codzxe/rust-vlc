# Container record — AAC ADTS / ADIF (raw elementary stream)

Status: **implemented** (demux, card 5.C2b; the `.aac` registry
surface is shared with the ES wrapper card 5.I3a). Feature-matrix
row `container-adts-aac` still reads `not_started` with empty
evidence — this record is that evidence; amendment recorded in-file
for the Lead (docs-only card 5.REC-1). Front-matter keys per
doc 04 §6:

- id = `container-adts-aac` (`docs/feature-matrix.toml:270`)
- status = implemented (demux); platform-independent code, evidence
  Linux-only
- phase = 5 (landed)
- fuzz_target = `fuzz_demux_adts` (in-crate `adts/fuzz.rs` + fuzz-lite
  corpus sweeps; doc 04 §8.5 registry name)
- crates = [] — zero external deps
- self_built = [`crates/demux/src/adts/**` (mod/error/tests/fuzz/
  test_support)]
- last_audit = 2026-10-24

## Scope landed (5.C2b)

- Probe: 0xFFFx syncword or `ADIF` (ID3-prefixed head legal);
  `EXTENSIONS = ["aac","adts","adif"]` (adts/mod.rs:135, verified
  this audit).
- **ADTS**: syncword/layer/protection-absent + CRC-16 (poly 0x1021,
  header scope)/profile/sampling-index/channel-config/13-bit
  variable frame length; ID3v2 head + interframe skip (syncsafe,
  16 MiB ceiling); bounded lost-sync resync (4 MiB) with Recovered
  events — a frame claim that overruns EOF raises the SAME sticky
  `demux.adts.truncated` the strict walk raises (real impl gap
  fixed by the finisher; a claim is a claim wherever the scan finds
  it).
- **ADIF**: header + PCE parse; CBR block arithmetic
  `bitrate×128/rate` (O(1) seek); VBR and multi-program ADIF refuse
  typed (boundaries are decode-domain).
- Timestamps: exact frame arithmetic `n·1024/rate`.
- Profile vocabulary Main/LC/SSR/LTP (ADTS cannot prove SBR/HE —
  decode domain; ffprobe `aac/LC` → `aac-lc` mapping row).

## Corpus evidence (5.C2b, corpus-v4, ffmpeg 7.1.5 / Lavf61.7.103)

| Lane | ffprobe truth | Lumen | Verdict |
|---|---|---|---|
| 0059-audio-only.aac | 346 pkts, aac/LC 88 200 Hz mono | 346 pkts, every pts = i·1024 | PASS |
| 0059 payload | `-f data` md5 9c5542e8…; adts roundtrip == source | identical md5 | PASS |
| 0110-audio-only.aac | 376 pkts, aac/LC 96 000 Hz mono | identical series | PASS |
| 0110 payload | raw md5 693bead8… | identical | PASS |
| dirty/tail-garbage-003 | 351 pkts (4 garbage frames accepted, final overrunner SHORT 3405 B) | 346 real frames byte-exact + 4 recovered + sticky truncation | PASS w/ recorded divergence (350 vs 351) |
| ADIF (generated CBR) | ffmpeg has NO ADIF demuxer/muxer (measured) | cross-format agreement: same essence via ADTS and ADIF demuxes to identical payload bytes (unit-pinned) | PASS (only available reference) |

## Honest divergences (measured, recorded — 5.C2b)

- Dirty-lane overrunner: ffmpeg emits a short packet; we raise the
  typed sticky truncation (strict-walk policy consistency).
- Resync acceptance = the measured ffmpeg predicate (syncword +
  header-covered frame + fits; layer/sampling-index NOT rechecked
  mid-scan).

## Error paths

Each pinned by test: lost-sync bounded resync; truncated frame
(mid + first + resync-found); sampling-index 15; channel-config 0;
CRC mismatch. +FB: fuzz-lite harness + corpus sweeps + committed
mutation fixtures.

## Honest gaps (D-10, from 5.C2b)

- **LOAS/LATM not implemented** (doc 04 §2.1 row's named gap —
  unchanged; no corpus lane either).
- HE-AAC/SBR detection decode-domain (none in v1; AAC-LC decode
  landed 5.C2a with max-delta 1 LSB).
- Zero-copy row in `tests/zero_alloc_read_chain.rs` pending (5.X2;
  pool unit-bounded ≤ ~16.4 KiB, 2-slot × (8191+9) B).
- Decode-throughput (5.X1).

## Gates (5.C2b, exact)

`cargo fmt -p demux -- --check` CLEAN; clippy CLEAN;
`cargo test -p demux --all-features` 331/0 (244 unit incl. 30 adts
+ 13 suites + 4 adts_corpus + 5 doctests). Workspace at wave-7
close: 3040/0 (5.W7-CLOSE).

## Next steps

- Lead: matrix-row amendment. LOAS/LATM only on demand (S card if a
  corpus lane ever shows up). 5.X2 zero-alloc row.
