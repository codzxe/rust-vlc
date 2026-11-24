# AUDIT: fuzz/corpus/fuzz_codec_mp3/seed_mp3_mpeg2_24k.mp3
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 6,476 B / f9e387c2b3fdddb6… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (valid-seed corpus file; header parsed) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: MP3 with ID3v2.4 tag (tag size 34, audio at byte 44); first frame
header fff38400 = MPEG-2 (version bits 2), Layer III, 24 kHz (sr index 1 for
MPEG-2) — name `mp3_mpeg2_24k` is accurate. ffmpeg Lavf heritage per the
formats.md ffmpeg-synthesis note (family provenance).

Consumer: cargo-fuzz target fuzz_codec_mp3 via corpus-dir convention
(fuzz/Cargo.toml); no in-repo test references it by name (rg: zero hits).
Probably shared bytes with crates/codecs mp3 fixtures (same family; not
byte-verified — UNVERIFIED which, if any, tests/ copy is byte-identical).
Execution: never executed in any committed baseline (no fuzz run in
audit/runs/baseline/) — corpus viability only.
