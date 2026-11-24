# AUDIT: fuzz/corpus/fuzz_codec_vorbis/seed_ogg_vorbis8k_st.ogg
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 3,245 B / 91e9db3851e5432393c008f749235dc124782428ad005ce26fb700878549646c / Ogg Vorbis stereo 8kHz ~27.2kbps (batch manifest) |
| Generated vs handmade | Distinct from tests/fixtures/generated/valid/ogg_vorbis_8k_st.ogg (3,533 B per manifest listing — different size, so NOT a copy of the test fixture; independent seed bytes). Provenance undocumented — UNVERIFIED authorship. |
| Consumer | cargo-fuzz campaign only (fuzz_codec_vorbis target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002) |
| Verdict | REAL bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
