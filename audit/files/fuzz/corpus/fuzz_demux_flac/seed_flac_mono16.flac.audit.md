# AUDIT: fuzz/corpus/fuzz_demux_flac/seed_flac_mono16.flac
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 12,243 bytes / aa5969e3636b5aa46e885c6060e4928dd1df5ebbe4e7e51521eb417134bb7aa2 |
| Type | FLAC 16-bit mono 44.1kHz, 11025 samples |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; `file` |

## 1. Verdict
Valid FLAC seed (file(1): "FLAC audio bitstream data, 16 bit, mono, 44.1 kHz, 11025 samples") matching the target name fuzz_demux_flac / codec FLAC mono16. Corpus-dir convention only; no committed fuzz run.
## 2. Findings
F-001 [S3] unexecuted fuzz lane (see seed_wav_rf64 F-001). Confidence: HIGH — Verification: ran.
## 3. What actually works (verified)
`file` output above; sha recorded.
