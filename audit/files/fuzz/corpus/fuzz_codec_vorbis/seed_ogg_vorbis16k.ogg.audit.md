# AUDIT: fuzz/corpus/fuzz_codec_vorbis/seed_ogg_vorbis16k.ogg
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 3796 bytes |
| File verdict | REAL (Ogg/Vorbis; byte-identical to the committed valid fixture) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 provenance byte-compare (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Genuine Ogg container (`OggS` magic), 3796 bytes, sha256 58046ef1733838a4... (recomputed).
VERIFIED byte-identical to tests/fixtures/generated/valid/ogg_vorbis_16k_mono.ogg — the
README's "copied from the committed CC0 fixtures" provenance holds.

## 2. Consumers
fuzz/fuzz_targets/fuzz_codec_vorbis.rs (registry-gated — gate RAN by me, CLEAN). The
identical bytes as a fixture pin Vorbis decode truth in the demux ogg family tests
(baseline: demux PACKAGE exit_code=0).

## 3. Findings
None. (Source fixture sits outside the MANIFEST drift gate — systemic note only.)

## 4. Open questions
None.
