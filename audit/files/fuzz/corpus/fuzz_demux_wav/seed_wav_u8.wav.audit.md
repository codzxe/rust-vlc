# AUDIT: fuzz/corpus/fuzz_demux_wav/seed_wav_u8.wav
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (WAV container) |
| Size / sha256-prefix | 8898 bytes / e1a813dfc37f8ef9 (verified locally) |
| File verdict | REAL — byte-identical copy of a committed valid fixture |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 vs tests/fixtures/generated/valid/wav_pcm_u8_44k1_mono.wav -> MATCH |

## 1. Verdict
Real seed: sha256 `e1a813dfc37f8ef9` byte-identical to
tests/fixtures/generated/valid/wav_pcm_u8_44k1_mono.wav — the u8/44.1k/mono
WAV parent fixture. Header self-consistent (`RIFF...WAVE fmt PCM 44100`).
Same consumption and gate status as the mkv seed (registry gate PASS in
baseline full_gate.txt:57-58).

## 2. What actually works (verified)
- Hash match (command above); header bytes verified.
- Convention: fuzz/README.md:112-116.

## 3. Findings
None.
