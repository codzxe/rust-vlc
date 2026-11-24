# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__trunc4 (no extension)
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | binary, 35,346 B / n/a (assignment chunks #part1..#part5, L1-3166) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; consumed by crates/demux/tests/wav_aiff_fixtures.rs:668 `derivative_payloads_are_parent_prefixes`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 byte-diff parent vs derivative; sed consumer test |

Data audit: class-B derivative = byte-exact prefix of parent `generated/valid/wav_pcm_s16le_44k1_st.wav` (35,358 B) cut by exactly 12 bytes (verified: cut=12, prefix identity True). Consumer pins it two ways: (1) `WavDerivative { name: "trunc4", bytes: 35_268, reasons: riff-extends-past-stream + data-extends-past-stream, terminal: "demux.wav.truncated" }` (wav_aiff_fixtures.rs:407-413, comment "12 bytes short of the end: 35268 whole frames deliver"); (2) payload-prefix assertion `trunc4[78..78+35_268] == parent_payload[..35_268]` (:668-672) — the test would fail if the fixture were regenerated with a different cut. Honest, non-tautological pinning.
F-001 [S3]: same as aiff family — no extension, no expectation TOML, no MANIFEST.toml row (ledger covers only the 28 avi/mp4_mjpeg_pcm files). The doc-10 §5.1 "fixture without a ledger row fails G6" gate is not implemented for this family; pinning exists only in the demux test table. Confidence: HIGH — Verification: read.
