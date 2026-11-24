# AUDIT: fuzz/corpus/fuzz_sub_ass/seed_ass_hostile.ass
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 123 bytes / a0c3659e6e52a86e5f51d289216addaba7558a357cf2348086b11d3df7c194a5 |
| Type | hand-authored hostile ASS subtitle |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; od -c dump |

## 1. Verdict
Deliberately hostile ASS: `[Script Info] Title: <ff fd ff bytes> garbage \0\0`; `[Events] Dialogue: 0,99:99:99.99,0:00:00.00,,<ff fd>` — out-of-range timestamp fields (99:99:99.99), reversed start>end, invalid UTF-8 bytes (0xff 0xfd), embedded NULs, empty style. Exactly the "hostile" seed the name promises (od dump verified). Consumed via corpus-dir convention by fuzz_sub_ass only; that lane has no committed run.
## 2. Findings
F-001 [S3] unexecuted fuzz lane (see seed_wav_rf64 F-001). Confidence: HIGH.
## 3. What actually works (verified)
od dump (above) matches the hostile description byte-for-byte.
