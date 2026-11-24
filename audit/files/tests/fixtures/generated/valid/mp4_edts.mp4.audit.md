# AUDIT: tests/fixtures/generated/valid/mp4_edts.mp4
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 14,442 B / 7e305b605d4ca69c… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (data fixture; box structure independently parsed) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

What it is: MP4 class-A fixture with an edit list (ftyp isom / free / mdat
12,550 / moov 1,852 — all top-level box sizes walk the file exactly to EOF,
verified). moov carries `edts`/`elst` with 2 entries; entry 1 is the classic
empty edit (media_time = 0xFFFFFFFF = -1), the standard audio-delay shape. Magic
and sizes verified with python; sha256 recomputed.

Consumers (rg): crates/demux/tests/mp4_fixtures.rs:83 (`Valid { fixture:
"generated/valid/mp4_edts.mp4", codecs: [("h264", Video), ("aac", Audio)],
fragmented: false, chapters: 0 }`) and :193 (behavioral test),
crates/transcode/tests/mp4_remux.rs:422 (`const WHAT: &str =
"generated/valid/mp4_edts.mp4"`), sibling malformed variant
`mp4_edts__trunc0.mp4` (row 27). Matrix-evidence:
docs/corpus/compat-matrix-20261110.raw-rows.jsonl:68 — outcome "decoded_partial",
aac 75 packets/frames, h264 no_decoder (build has no h264), demux_recovered=2;
consistent with an elst-carrying h264+aac file.
Generated-vs-handmade: generated media fixture family (h264+aac with chapters/
edts variants; NOT one of the 28 xtask-ledger rows). S4: like the other
non-avi/mp4-mjpeg families, no CORPUS.md/MANIFEST.toml ledger row — no drift gate.
