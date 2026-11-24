# AUDIT: fuzz/corpus/fuzz_sub_srt/seed_srt_trunc2.srt
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (truncated SubRip) |
| Size / sha256-prefix | 32 bytes / 7c469209b2e0c063 (verified locally) |
| File verdict | REAL — genuine truncation |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 + od -c dump |

## 1. Verdict
Real truncated SRT seed: exactly 32 bytes — index line `1`, timing line
`00:00:01,000 --> 00:00:04,000`, and NOTHING after (no cue text, no final
newline). od confirms the file ends right after the timing newline. This is
the "deliberate truncations of over-size parents" arm (fuzz/README.md:104).
Feeds fuzz_sub_srt. Presence gated (baseline full_gate.txt:57-58).

## 2. What actually works (verified)
- Verbatim (od -c, tail):
```
0000000   1  \n   0   0   :   0   0   :   0   1   ,   0   0   0       -
0000020  -   >       0   0   :   0   0   :   0   4   ,   0   0   0  \n
```
- The truncation targets a real parser edge: cue-with-index-and-timing-but-
  no-text (EOF mid-cue).

## 3. Findings
None.
