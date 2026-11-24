# AUDIT: fuzz/corpus/fuzz_codec_adpcm/seed_adpcm_ima_ws.bin
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 512 / c029dfc944a023be |
| File verdict | REAL (seed corpus copy) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Seed for fuzz_codec_adpcm. BYTE-IDENTICAL copy of the committed crate fixture
crates/codecs/tests/fixtures/breadth/ws_mono.bin (512 B, sha256
c029dfc944a023be both — python3 compare). Documented copy class per
fuzz/README.md:101-104. Campaign unwired; stable consumers are the registry
gate and the twin fixture's breadth tests.
## 2. Verified
— python3 sha256 compare (identical); 512 B < 50 KB budget.
— Consumer: fuzz/Cargo.toml bin fuzz_codec_adpcm; twin in
crates/codecs/tests/fixtures/breadth.
## 3. Findings
F-001 [S4] same unlinked-duplicate drift risk as seed_adpcm_swf.bin
(no checksum tie between copy and source). Confidence: HIGH.
