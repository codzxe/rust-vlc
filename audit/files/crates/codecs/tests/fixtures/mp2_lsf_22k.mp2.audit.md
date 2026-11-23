# AUDIT: crates/codecs/tests/fixtures/mp2_lsf_22k.mp2 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| File verdict | REAL (valid fixture, actively used) |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> MPEG ADTS, layer II, v2, 96 kbps, 22.05 kHz, Stereo; `ls -la` -> 19435 bytes; rg citations; baseline test results |

## 1. One-paragraph verdict
Real MPEG-2 LSF (low-sampling-frequency) Layer II fixture — 19435 bytes, 96 kbps, 22.05 kHz stereo, v2. It is the LSF lane of the mp2 family: consumed by mp2_corpus_differential.rs's fixture differential (baseline 3/3 ok) and its dedicated geometry test `the_generated_lsf_fixture_walks_and_decodes_with_declared_geometry` (pins 22050/2, 96000 bps, MpegVersion::V2, 1152-sample frames). The format doc honestly records the family as NOT bit-exact (±1 tolerance). No defects.

## 2. What actually works (verified)
— verified by `file` (MPEG ADTS layer II v2 96 kbps 22.05 kHz stereo — matches every claim in the geometry test and format doc).
— verified baseline audit/runs/baseline/cargo-test-workspace.txt:1232-1239 -> mp2_corpus_differential 3 passed, including the LSF geometry test.

## 3. Findings
None. Type sane; used by real passing tests; no doc mis-cites it.

## 4-9. (n/a for binary assets)
Cited by: crates/codecs/tests/mp2_corpus_differential.rs:114, 204-215; docs/formats/audio-mp3-mp2.md:85; also mp2_fuzz_survival/mp2_bench fixture lists.
UNVERIFIED: ffmpeg authorship provenance (recorded in the format doc; digests + geometry pins are the tamper guard).
