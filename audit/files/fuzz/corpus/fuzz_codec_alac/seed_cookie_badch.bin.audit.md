# AUDIT: fuzz/corpus/fuzz_codec_alac/seed_cookie_badch.bin

| Field | Value |
|---|---|
| Auditor | W16-RR5 (orchestrator-completed; orig W16-A08) |
| Date | 2026-11-24 |
| Lines / functions | binary corpus seed (26 bytes) |
| File verdict | REAL (test-lane data) |
| Reachable from shipped entry point? | NO (fuzz corpus; fuzz crate is a non-member) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | sha256sum (f5d7670cdbe0fe79...), rg consumers (0 direct by-name; consumed via fuzz_codec_alac corpus dir glob) |

## 1. One-paragraph verdict
26-byte hand-made ALAC cookie seed with a bad channel count (name says
badch). It feeds the fuzz_codec_alac target's corpus directory. No test
or doc cites it by name; it is fuzz-lane inventory only, consistent with
the repo-wide "scaffolds-not-wired" fuzz story (campaign never run).

## 2. What actually works (verified)
— verified by sha256 + `rg -l seed_cookie_badch` (corpus inventory only).

## 3. Findings
F-001 [S4] Where: fuzz/corpus/fuzz_codec_alac/seed_cookie_badch.bin:1-26
What is wrong: corpus seed has zero evidenced execution (no libFuzzer
campaign artifact anywhere in the tree; fuzz/README admits not wired).
Confidence: HIGH — Verification: read.

## 4. Dependencies & flags
Part of fuzz/ workspace member (excluded from release lane).
