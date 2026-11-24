# AUDIT: fuzz/corpus/fuzz_codec_flac/seed_flac_s24.flac
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 44112 bytes |
| File verdict | REAL (FLAC STREAMINFO decoded; byte-identical to the codecs crate fixture) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 STREAMINFO parse + provenance scan (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Genuine FLAC seed: `fLaC` magic, STREAMINFO (block type 0, 34 B) decoded by me —
48 000 Hz, 2 channels, 24-bit, min/max blocksize 4608 — matching the seed's name (s24).
sha256 d9968c8399532f30... (recomputed). VERIFIED byte-identical to
crates/codecs/tests/fixtures/flac_s24_stereo_48000.flac — the README's provenance claim
("copied from the committed CC0 fixtures ... the crate test fixtures") holds exactly.

## 2. Consumers
fuzz/fuzz_targets/fuzz_codec_flac.rs (registry-gated; I ran the gate: CLEAN, 35 targets,
corpora present). Campaign unwired (honest README status); the in-crate flac decoder tests
consume the identical crate fixture and PASSED in the baseline (PACKAGE codecs exit_code=0).

## 3. Findings
None. Presence is gate-checked; content verified; provenance claim TRUE.

## 4. Open questions
None.
