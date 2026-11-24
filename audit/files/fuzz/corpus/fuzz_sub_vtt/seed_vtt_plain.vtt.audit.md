# AUDIT: fuzz/corpus/fuzz_sub_vtt/seed_vtt_plain.vtt
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (hand-authored WebVTT) |
| Size / sha256-prefix | 134 bytes / faeef084eee03b05 (verified locally) |
| File verdict | REAL — handmade baseline cue pair |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256, full text dump |

## 1. Verdict
Real hand-authored 2-cue WebVTT seed. Presence gated (baseline
full_gate.txt:57-58). The hostile sibling (seed_vtt_hostile.vtt) exists in
the same dir; this file is the plain baseline arm.

## 2. What actually works (verified)
- Verbatim (full file):
```
WEBVTT

00:00:00.000 --> 00:00:01.500
Hello from the seed

00:00:01.500 --> 00:00:03.000 align:start line:0%
Second cue with settings
```
- Second cue carries cue settings (`align:start line:0%`) — mild structure
  coverage, not just the empty minimum.

## 3. Findings
None.
