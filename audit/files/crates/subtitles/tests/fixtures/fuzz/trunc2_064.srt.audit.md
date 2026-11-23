# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_064.srt
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Size / type | 64 bytes, text (SRT), valid UTF-8, truncated mid-line by design |
| File verdict | REAL — verified truncation of the canonical parent, executed through the parser |
| Reachable from shipped entry point? | NO (test fixture; crate unlinked — parser.rs F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | od -c (exit 0); python3 prefix-compare vs canonical/out2.srt (exit 0); probe_srt parse (exit 0, output below) |

## 1. Verdict
Genuine fuzz corpus member: the EXACT 64-byte prefix of `canonical/out2.srt` (python3:
`is out2 prefix(64): True`), cut mid-way through cue 2's timing line (the file ends with
`"2\n00:00:05,500 --> "` — no end timestamp). Parsed by me through the real parser
(probe_srt): cue 1 ("Hello world", [1_000_000, 4_000_000] µs) survives; the truncated
timing line is typed `BadTimestamp @line 6`; no panic, ledger bounded. The "trunc2" series
(16-byte strides) matches the corpus generator description
(srt_fuzz_survival.rs:20-21 "systematic truncations at 16-byte strides"), and the
every-prefix sweep (`truncations_never_beat_the_parser`, srt_fuzz_survival.rs:140-149)
covers this cut and all others directly.

## 2. Evidence
— probe_srt (exit 0): `64 bytes, 1 cues, 1 damage (dropped 0), encoding Utf8`;
  `cue id=1 [1000000,4000000] text=Plain("Hello world")`; `damage BadTimestamp @line 6`.
— Exercised by `committed_corpus_survives` AND the systematic prefix sweep
  (srt_fuzz_survival.rs:143-147 covers cut=64 of out2 explicitly; baseline ok).

## 3. Findings
None. Checked: parentage (byte-exact 64-byte prefix of out2), truncation point (mid
timing line), parse outcome (executed — surviving cue intact, truncation typed).
