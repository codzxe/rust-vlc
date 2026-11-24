# AUDIT: fuzz/corpus/fuzz_codec_pcm/seed_pcm_u8.bin
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (raw PCM bytes) |
| Size / sha256-prefix | 512 bytes / bd7f86fe64d3e62c (verified locally) |
| File verdict | REAL — handmade deterministic ramp |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 + byte dump |

## 1. Verdict
Real hand-authored seed: 512 bytes of a perfect u8 sawtooth ramp
(`\x00\x03\x06...\x00` — step 3, wraps twice), i.e. the "hand-authored
text/cookie/skeleton seeds" arm of fuzz/README.md:106-107. Deterministic,
tiny, CC0. Fed to fuzz_codec_pcm (PCM decode contract) by cargo-fuzz's
corpus convention; presence gated by scripts/fuzz_registry_check.sh
(PASS in baseline full_gate.txt:57-58).

## 2. What actually works (verified)
- Byte dump: b"\x00\x03\x06\t\x0c\x0f..." — verified ramp.
- Consumer convention: fuzz/README.md:112-116; gate run: baseline
  full_gate.txt:56-58.

## 3. Findings
None.
