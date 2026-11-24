# AUDIT: fuzz/corpus/fuzz_demux_mp4/seed_mp4_chapters.mp4
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 24,839 / 81ca6cd5a47b4a07 |
| File verdict | REAL (distinct seed) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Seed for fuzz_demux_mp4 carrying a chapters track (24,839 B). NOT a copy of
the generated/valid/mp4_h264_aac.mp4 parent (24,839 vs 13,703 B, distinct
sha256 — python3 compare), and not a prefix of it — this is an independent
file exercising the MP4 chapter-box surface. First bytes are a real MP4
(`ftyp` box). Provenance class per fuzz/README.md:101-110: committed-CC0
fixture copy / truncation / cross-format / hand-authored — this one is an
independent real fixture; the README does not name it individually.
## 2. Verified
— python3: distinct from every other compared fixture; magic verified real
MP4 structure.
— Consumer: fuzz/Cargo.toml bin fuzz_demux_mp4; corpus dir non-empty (4
seeds: chapters, frag, h264_aac, trunc0).
## 3. Findings
None.
