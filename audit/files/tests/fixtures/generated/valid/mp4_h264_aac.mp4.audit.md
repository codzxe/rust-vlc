# AUDIT: tests/fixtures/generated/valid/mp4_h264_aac.mp4
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 13,703 B / 3acc7ebae6f41c3194bab8f23f935e57e27adb81d61d8f2b99b8cd98124c7d8e |
| File verdict | REAL (data fixture; box walk exact) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

What it is: MP4 (ftyp) with h264 video + aac audio; top-level walk ftyp(32) /
moov(1,792) / free(8) / mdat(11,871) sums exactly to 13,703 (EOF-exact, moov-first
progressive layout). sha256 recomputed; magic verified.

Consumers (rg): crates/demux/tests/mp4_fixtures.rs:36 (Valid table),
crates/transcode/tests/mp4_remux.rs:251/513/539, crates/media-probe/tests/
registry_roundtrip.rs:61. Matrix-evidence: docs/corpus/compat-matrix-20261110.jsonl
rows (mp4_h264_aac + its flip16/trunc0 derivatives), docs/inventory/formats.md,
docs/baselines/raw/linux/17-format-probes.txt — this fixture is the MP4 probe
baseline workhorse. Generated-vs-handmade: generated fixture family (h264+aac),
not in the 28-row CORPUS.md/MANIFEST ledger -> S4 drift-gate gap.

S4 (duplication): this file is byte-identical (sha256 3acc7ebae6f41c31…) to
fuzz/corpus/fuzz_codec_h264/seed_mp4_h264_aac.mp4 AND
fuzz/corpus/fuzz_demux_mp4/seed_mp4_h264_aac.mp4 — three committed copies of the
same 13,703 bytes. Convention-driven, not fraud, but a 41 KB duplication the
drift gate does not cross-check (only the tests/ copy has matrix evidence).
