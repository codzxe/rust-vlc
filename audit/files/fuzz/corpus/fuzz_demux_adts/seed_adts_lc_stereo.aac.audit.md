# AUDIT: fuzz/corpus/fuzz_demux_adts/seed_adts_lc_stereo.aac
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 25219 bytes |
| File verdict | REAL (ADTS AAC-LC stereo 44.1k; byte-identical to the codecs crate fixture) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 ADTS decode + provenance byte-compare (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Genuine ADTS stream: header `fff1 5080 11...` — MPEG-4, AAC-LC, sampling index 4
(44.1 kHz), channel_config 2 (stereo) — name (adts_lc_stereo) exact. sha256
1c934f5b8f71aace... (recomputed). VERIFIED byte-identical to
crates/codecs/tests/fixtures/sine441_lc_stereo.aac. Contains `Lavc61.19.10` (ffmpeg aac
encoder signature).

## 2. Consumers
fuzz/fuzz_targets/fuzz_demux_adts.rs (registry-gated — gate RAN by me, CLEAN). The identical
crate fixture drives the AAC decoder suite (baseline: codecs PACKAGE exit_code=0).

## 3. Findings
None. Provenance verified TRUE (README: "copied from ... the crate test fixtures").

## 4. Open questions
None.
