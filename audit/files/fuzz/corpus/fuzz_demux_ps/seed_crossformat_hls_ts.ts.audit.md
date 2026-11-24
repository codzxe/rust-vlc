# AUDIT: fuzz/corpus/fuzz_demux_ps/seed_crossformat_hls_ts.ts
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 8,084 / 3f1e55a70c30773f |
| File verdict | REAL (cross-format seed — BY DESIGN) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
The documented cross-format seed: MPEG-TS bytes fed to fuzz_demux_ps
(fuzz/README.md:104-106 "cross-format seeds (e.g. TS bytes into
fuzz_demux_ps — format confusion is fair fuzz material)"). BYTE-IDENTICAL to
crates/playlists/tests/fixtures/hls/ts_vod/pl0.ts (the ffmpeg-authored first
segment of the HLS ts_vod fixture) and to
fuzz/corpus/fuzz_demux_ts/seed_hls_pl0.ts; it is also an exact prefix of
crates/playlists/tests/fixtures/hls/single_file/pl.ts (35,344 B). Verified
by python3 compare over all repo .ts files. The same 8,084 bytes therefore
exist at three tree locations.
## 2. Verified
— python3: == pl0.ts == seed_hls_pl0.ts (sha256 3f1e55a70c30773f); prefix of
single_file/pl.ts; head `47 40 11 10 …` = TS packet with sync byte 0x47.
— Consumer: fuzz/Cargo.toml bin fuzz_demux_ps (2-seed corpus: this +
crossformat_mp4).
## 3. Findings
F-001 [S4] the cross-format copy is intentional and documented, but like the
other seed copies it carries no checksum tie; a regeneration of the HLS
fixture family silently diverges all three copies. Confidence: HIGH —
Verification: executed compare.
