# AUDIT: fuzz/corpus/fuzz_codec_alac/seed_alac_stereo16.m4a
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 42907 B / f0662338bd3e159818b6decc741ede8292fb51270af288387f422d6fa2a7dacd |
| File verdict | REAL (as data); corpus copy itself never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-twin executed by the ALAC fuzz-survival suite |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (corpus-seed audit)
Real ALAC-in-M4A container: `ftyp M4A `, mdat(42149) then moov(722) with smhd/stbl — an audio-only QuickTime movie (box walk verified). It is sha256-identical to `crates/codecs/tests/fixtures/alac/alac_stereo16.m4a`, which the ALAC fuzz-survival suite demuxes (cookie + packets) and runs seeded-LCG mutations through the decoder (crates/codecs/tests/alac_fuzz_survival.rs). The corpus copy is read by nothing on stable.

## 2. What was checked and how
- batch.py: 42907 B, sha256 f0662338…, box walk (ftyp/free/mdat/moov with smhd).
- sha256 equality with crates/codecs/tests/fixtures/alac/alac_stereo16.m4a (verified).
- Twin consumers: crates/codecs/tests/alac_fuzz_survival.rs (fixture_path joins `tests/fixtures/alac`; the three pinned survival tests) — baseline: alac_fuzz_survival 3/3 ok (audit/runs/baseline/cargo-test-workspace.txt:961-967).
- Registry gate (existence only) run by me: exit 0.

## 3. Findings
F-001 [S3] Corpus-lane gap (shared): fuzz/corpus/fuzz_codec_alac/seed_alac_stereo16.m4a is never read from the corpus path; executed coverage exists only via the codecs-crate twin. Verification: ran (sha256 + rg).
