# AUDIT: fuzz/corpus/fuzz_demux_aiff/seed_aiff_trunc0.bin
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 12 B / c8d0e1fb851425dc… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (hostile trunc0-style seed) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: exactly the 12-byte AIFF container header (`FORM` + big-endian
length 0x000089FE = 35,326 + `AIFF`) with no COMM/SSND chunks — the FORM length
claims 35,326 bytes that do not exist. Hostile trunc0-style seed (od dump above:
`46 4f 52 4d 00 00 89 fe 41 49 46 46`). Same claimed length as the valid
seed_aiff_s16be.aiff (35,326) — i.e., the trunc0 of that sibling.

Consumer: corpus-dir convention (fuzz/corpus/fuzz_demux_aiff/ + Cargo.toml
target fuzz_demux_aiff); no in-repo test reads it by name. In-crate twin shapes
exist in crates/demux/src/aiff/fuzz.rs (empty/short input survival tests pass in
the committed baseline).
