# AUDIT: fuzz/corpus/fuzz_sub_srt/seed_srt_flip2.srt
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 144 bytes / c751bff79ac7f85dc746877426840d18c79f617924b5815d3d6e612a001c6567 |
| Type | SRT subtitle with a corrupted timestamp byte (ISO-8859) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; cat dump |

## 1. Verdict
3-cue SRT where cue 1's start timestamp is `00:00:<corrupted byte>1,000` (byte after the second colon is non-ASCII garbage — a seeded bit-flip inside a timestamp), cues 2-3 valid. Matches the `__flip2` naming convention (2 flipped bits per wav_aiff_fixtures.rs:37-41 docs). Consumed via corpus-dir convention by fuzz_sub_srt; lane unexecuted in baselines.
## 2. Findings
F-001 [S3] unexecuted fuzz lane (see seed_wav_rf64 F-001). Confidence: HIGH.
## 3. What actually works (verified)
cat dump shows the corrupted timestamp `00:00:\xef\xbf\xbd1,000` in cue 1.
