# AUDIT: fuzz/corpus/fuzz_demux_mp4/seed_mp4_trunc0.mp4
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 28 bytes / 1d514cd1aff19efd8f5b0f0e537c0f39c71ee0a4036a64dfccfb7aa0a6b0673d |
| Type | ISO Media MP4 header fragment |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; od dump |

## 1. Verdict
28-byte trunc0-style seed: complete `ftyp` box (size 0x1c=28, brands isom/iso2/mp41) with NOTHING after it — the minimal "valid header, truncated everything else" case for fuzz_demux_mp4 (matches the `__trunc0` convention: cut at a structure boundary). Consumed via corpus-dir convention; lane unexecuted in baselines.
## 2. Findings
F-001 [S3] unexecuted fuzz lane (see seed_wav_rf64 F-001). Confidence: HIGH.
## 3. What actually works (verified)
od dump: `00 00 00 1c 66 74 79 70 69 73 6f 6d ...` — exact ftyp-only file; box size field equals file size.
