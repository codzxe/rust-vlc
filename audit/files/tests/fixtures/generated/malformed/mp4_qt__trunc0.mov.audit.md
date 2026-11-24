# AUDIT: tests/fixtures/generated/malformed/mp4_qt__trunc0.mov
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 11,900 B / 29392a91aa4359c1… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (data fixture; structure independently parsed) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

What it is: QuickTime-flavored MP4 (ftyp `qt `, 20 B) with `wide` + `mdat`
(11,871 B, same payload length as the h264_aac sibling family) and **no moov** —
top-level walk ends at 11,899 of 11,900 with 1 stray byte, i.e. the moov box was
amputated. This is the no-moov hostile shape.

Consumer: crates/demux/tests/mp4_fixtures.rs:232 (`"generated/malformed/
mp4_qt__trunc0.mov",` — malformed set entry). Matrix-evidence:
docs/corpus/compat-matrix-20261110.raw-rows.jsonl:47 verbatim outcome:
`"status":"refused_demux","message":"corrupt structure (demux.mp4.no-moov)"` —
matches the physical structure I parsed (no moov present). Generated family
(qt variants); not in the 28-row CORPUS.md/MANIFEST ledger -> S4 drift-gate gap.
