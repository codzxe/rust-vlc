# AUDIT: fuzz/corpus/fuzz_codec_aac/seed_adts_lc_mono.aac
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 13201 bytes |
| File verdict | REAL (ADTS AAC-LC mono 48k; byte-identical to the codecs crate fixture) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 ADTS decode + provenance byte-compare (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Genuine ADTS stream: header `fff1 4c40 12...` — MPEG-4, protection_absent, AAC-LC,
sampling index 3 (48 kHz), mono — name (adts_lc_mono) is exact. sha256 6e11e8c04db377f2...
(recomputed). VERIFIED byte-identical to crates/codecs/tests/fixtures/sine48_lc_mono.aac.
Contains `Lavc61.19.10` at offset 10 (ffmpeg aac encoder signature in the fill element) —
the codecs crate fixtures are external-encoder-authored test media; no doc claims otherwise
for that directory (CORPUS.md's in-tree-authorship claim is scoped to tests/fixtures/
generated/ + expectations/).

## 2. Consumers
fuzz/fuzz_targets/fuzz_codec_aac.rs (registry-gated — gate RAN by me, CLEAN). The identical
crate fixture drives the AAC decoder conformance suite (baseline: codecs PACKAGE
exit_code=0, aac decoder tests all ok).

## 3. Findings
None. Provenance chain (README "copied from ... the crate test fixtures") verified TRUE.

## 4. Open questions
None.
