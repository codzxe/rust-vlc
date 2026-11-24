# AUDIT: fuzz/corpus/fuzz_demux_ts/seed_hls_pl0.ts
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 8,084 / 3f1e55a70c30773f |
| File verdict | REAL (seed corpus copy) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Seed for fuzz_demux_ts: BYTE-IDENTICAL copy of
crates/playlists/tests/fixtures/hls/ts_vod/pl0.ts — the first segment of the
ffmpeg-authored HLS TS fixture (worklog archive-2026-10.md:10794 "8 s 44.1
kHz mono AAC-LC sine … ffmpeg -f segment"). Also identical to
fuzz/corpus/fuzz_demux_ps/seed_crossformat_hls_ts.ts and an exact prefix of
crates/playlists/tests/fixtures/hls/single_file/pl.ts. Three tree locations
carry these same 8,084 bytes.
## 2. Verified
— python3: identical bytes + prefix relation verified; head `47 40 11 …` TS
sync byte 0x47.
— Consumer: fuzz/Cargo.toml bin fuzz_demux_ts (2-seed corpus: this +
mpeg2_in_ts).
## 3. Findings
F-001 [S4] triple-duplicated bytes, unlinked (same class as the other seed
copies). Confidence: HIGH — Verification: executed compare.
