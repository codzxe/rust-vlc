# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_032.srt
| Field | Value |
|---|---|
| Auditor | W09-A01 | Date | 2026-11-23 |
| Kind / size | text fixture (mid-cue truncation) / 32 B |
| File verdict | REAL |
| Consumed by | `srt_fuzz_survival.rs::committed_corpus_survival` (whole-dir read; baseline: 4/4 ok) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## Mutation encoded (byte-verified)
`fixtures/canonical/out2.srt` truncated to its first **32 bytes** = cue 1's index line + complete timing line (`1\n00:00:01,000 --> 00:00:04,000\n`) with **no text line and no blank separator** — EOF mid-cue. Family: `trunc2_NNN` = out2 parent cut at 16-byte strides (verified programmatically for all 8 members: each is an exact prefix).

## Executed outcome (probe, exit 0)
```
trunc2_032.srt: cues=0 damages=1 kinds=["EmptyCueText"] texts=[]
```
The headless cue 1 is typed `EmptyCueText` and dropped; empty document, no panic. Note the coverage nuance vs. the sibling truncations: `truncations_never_beat_the_parser` separately sweeps EVERY prefix of the canonical parents (so this exact cut is also covered by the generated sweep); the committed member pins it deterministically in the corpus dir. No findings.
