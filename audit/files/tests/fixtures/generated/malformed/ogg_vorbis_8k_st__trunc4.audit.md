# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__trunc4
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 3,233 B / 99bf6de59510745ed0e56421fd183b13625c6e58a281d0a7b82649cf922df139 / Ogg Vorbis stereo 8kHz (batch manifest) |
| Registration | NOT in MANIFEST.toml (28-file AVI/MP4 set; no ogg rows at all); no drift gate; no committed generator |
| Consumers (rg) | crates/demux/tests/ogg_fixtures.rs L565/L616 `format!("generated/malformed/ogg_vorbis_8k_st__{}", d.name)` (class-B error-contract loops) |
| Provenance claim | ogg_fixtures.rs L68 mirrors the wav header prose ("__trunc0..4, __flip1/4/16, __garbage0 derivatives were ... deterministic byte surgery") — prose only |
| Verdict | SHALLOW provenance, real usage. F-001 [S3]: unregistered corpus — no sha256 ledger, invisible to `cargo xtask fixtures --check`, CORPUS.md drift claim does not reach it.
