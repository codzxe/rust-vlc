# AUDIT: fuzz/corpus/fuzz_codec_h264/seed_mp4_h264_aac.mp4
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 13,703 B / 3acc7ebae6f41c3194bab8f23f935e57e27adb81d61d8f2b99b8cd98124c7d8e |
| File verdict | REAL (valid-seed corpus file; box walk exact) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

Assignment row 3071. What it is: MP4 (ftyp/moov/free/mdat, EOF-exact walk — see
the byte-identical twin audit tests/fixtures/generated/valid/mp4_h264_aac.mp4).
Consumer: cargo-fuzz target fuzz/fuzz_targets/fuzz_codec_h264.rs via the
fuzz/corpus/fuzz_codec_h264/ directory convention (target declared fuzz/Cargo.toml
`fuzz_codec_h264`). No test references it by name (rg: zero hits outside the
corpus dir).

S4 (duplication): byte-identical to tests/fixtures/generated/valid/
mp4_h264_aac.mp4 and fuzz/corpus/fuzz_demux_mp4/seed_mp4_h264_aac.mp4 (same
sha256, verified by sha256sum on all three). Whether this corpus is ever executed
is a fuzz-infra question — no cargo-fuzz run exists in audit/runs/baseline/
(see the fuzz_codec_h264.rs target audit).
