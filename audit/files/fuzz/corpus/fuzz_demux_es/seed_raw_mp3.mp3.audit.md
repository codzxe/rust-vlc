# AUDIT: fuzz/corpus/fuzz_demux_es/seed_raw_mp3.mp3
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 6,476 bytes / f9e387c2b3fdddb69e23725bfe1c938a28c6e300798c3c7232d2737058ab92d7 |
| Type | MP3: ID3v2.4 + MPEG ADTS layer III v2 64kbps 24kHz stereo |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; `file`; rg consumers |

## 1. Verdict
Valid raw-MP3 elementary-stream seed for fuzz_demux_es (ID3v2.4 header then ADTS frames — exactly the "raw ES" shape that target exists for). No source references the seed by name; corpus-dir convention only. Fuzz lane unexecuted in committed baselines (no cargo-fuzz run recorded).
## 2. Findings
F-001 [S3] unexecuted fuzz lane (see seed_wav_rf64 F-001). Confidence: HIGH — Verification: ran (rg baseline).
## 3. What actually works (verified)
`file` output (above) — real decodable MP3.
