# AUDIT: tests/fixtures/generated/valid/ogg_vorbis_8k_st.ogg
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 3,245 / 91e9db3851e54323 |
| File verdict | REAL (class-A ffmpeg-authored parent) |
| Reachable? | NO (test fixture); consumers: crates/demux/tests/ogg_fixtures.rs:149+375 |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |

## 1. Verdict
Class-A valid Ogg/Vorbis parent. Provenance is documented INLINE in the
consumer test file (crates/demux/tests/ogg_fixtures.rs:16-23): "`ogg_vorbis_8k_st.ogg` — `ffmpeg -f lavfi -i …`" with the exact command;
the same doc records `ogg_chain_two_links.ogg` as `cat ogg_vorbis_8k_st.ogg …`
(concatenated from this parent). Distinct from the codecs-crate vorbis
fixtures (vorbis_sine_8000_mono.ogg differs — python3 compare), i.e., the
repo carries two separate 8 kHz vorbis sine families in two crates.
## 2. Verified
— python3: distinct from crates/codecs/tests/fixtures/vorbis_sine_8000_mono.ogg
(3,245 vs 3,458 B; different sha256).
— Consumers: ogg_fixtures.rs:149 (valid-lane truth row), :375
(`demuxed_payload("generated/valid/ogg_vorbis_8k_st.ogg")` — payload-parity
differential), :565/:616 (the derivative family is named after it).
— Baseline: demux suite green in audit/runs/baseline/cargo-test-workspace.txt.
## 3. Findings
F-001 [S3] duplicated fixture concept across crates: the demux ogg family
and the codecs vorbis fixtures encode two different 8 kHz vorbis sines
produced by (presumably) different ffmpeg invocations; no shared manifest
ties them (CORPUS.md does not cover either — see CORPUS.md audit).
Confidence: HIGH for the duplication fact; the provenance difference is
documented at ogg_fixtures.rs:16-23.
