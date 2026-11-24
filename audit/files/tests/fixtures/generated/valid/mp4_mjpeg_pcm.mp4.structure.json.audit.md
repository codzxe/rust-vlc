# AUDIT: tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4.structure.json
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 | 4,093 B / 3054dd9fa5c97ac3… (audit/runs/W16-A09-batchmeta.txt; CORPUS.md row records same sha) |
| File verdict | REAL (structure map; sample table independently re-verified) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: `structure-map/1` for the MP4 parent: top-level boxes (ftyp 28@0,
moov 2,172@28, mdat 422,252@2200 — sums exactly to 424,452 = file size), mvhd/
trak offsets, stsz_video_count_offset 827, and full 60+60 video/audio sample
offset+size tables.

Independent verification (python): file size matches `size`; last audio sample
ends exactly at EOF (424,452); walking the samples in offset order yields a
perfect v/a interleave covering the entire mdat contiguously with zero gaps
("vavavavavava…", 60 v + 60 a, covered to 424,452); bytes at 827 decode as
0x0000003C = 60 = stsz sample_count — the `stsz_video_count_offset` is exact;
`video_samples[0]` (offset 2208, size 1193) hashes to 35e3ec6c740edd122aa988
16629b3a5a663c68fbc54d24521e7dc0c0ffde03a — the identical JPEG frame as the
AVI parent's first sample (verified against the AVI expectation digest earlier),
consistent with the shared generator family.

Consumer: tools/xtask/src/cmd/fixtures.rs (the class-B cut/patch positions
derive from these offsets — e.g. negheight0's patched_offset 180 came from the
AVI structure map; this file serves the MP4 derivatives: trunc0/1/2/3/4 cut at
structure boundaries recorded here), plus tests/fixtures/CORPUS.md row and
MANIFEST.toml (drift-gated, 28-row ledger). Zero defects — the map is honest
and byte-verified.
