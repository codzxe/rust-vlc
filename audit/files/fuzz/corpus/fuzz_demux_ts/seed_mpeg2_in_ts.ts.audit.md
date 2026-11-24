# AUDIT: fuzz/corpus/fuzz_demux_ts/seed_mpeg2_in_ts.ts
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 49,152 B / 2921740245e9073cf190cd914d1975fd0b9d3ecbb7e5cb62af7c7442a4e5cfc6 / MPEG transport stream (batch manifest) |
| Generated vs handmade | Real TS bytes (MPEG-TS per file(1)); NOT a prefix of tests/fixtures/hls/ts_live/t00.ts (python prefix-compare: False — independent 48 KiB stream). Provenance undocumented; UNVERIFIED authorship. |
| Consumer | cargo-fuzz campaign only: fuzz_demux_ts target (fuzz/Cargo.toml bin) via nightly .github/workflows/audit.yml `fuzz` tier — which has ZERO committed run evidence (see audit/files/fuzz/fuzz_targets/fuzz_demux_avi.rs.audit.md F-002). No stable test consumes this seed (`rg "fuzz/corpus" crates/` → only streaming netfuzz_sweep.rs, different targets). |
| Verdict | REAL bytes, SHALLOW reach. F-001 [S3]: seed is exercised by no committed run — dead corpus until the campaign's first evidenced execution.
