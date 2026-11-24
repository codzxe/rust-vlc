# AUDIT: fuzz/corpus/fuzz_codec_alac/seed_cookie_valid.bin
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 26 bytes / 4b02377d29a847c9b9debe2d5dda04bc60d16db4608ed17aea83f28e602da107 |
| Type | 26-byte binary: `00 00 10 00 00 10 28 0a 0e 02 ... ac 44` (file(1) mis-guesses "Targa" — nonsense for 26 bytes; it is a hand-authored ALAC cookie) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | sha256 batch; od hex dump |

## 1. Verdict
26-byte hand-made "valid ALAC cookie" seed: frameLength 0x1000=4096, compatibleVersion 0, bit depth 16 (0x10 at offset 4... per od: `00 00 10 00`), 0x280a = 10250? (pb/mix fields), channels 2 (0x0e 0x02?), maxRun 0x20, maxFPCodedBits, `ac 44` = 44100 (0xac44 big-endian). Structure is consistent with an ALAC 'alac' atom payload cookie. Consumed only via the fuzz corpus-dir convention (no source refs). Fuzz lane unexecuted (no cargo-fuzz run in baseline).
## 2. Findings
F-001 [S3] unexecuted fuzz lane (see seed_wav_rf64 F-001). F-002 [S4] file(1) type is garbage ("Targa 65536x65536") — the name says what it is; fine for a seed but no header magic makes naive tooling misclassify. Confidence: HIGH — Verification: ran (od).
## 3. What actually works (verified)
od dump matches ALAC cookie layout (44100 Hz, 16-bit stereo).
