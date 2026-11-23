# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_096.srt
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Size / type | 96 bytes / SubRip + ASCII text (a strict prefix of the parent) |
| File verdict | REAL fixture — genuine truncation mutant of canonical/out2.srt |
| Cited as evidence? | Only collectively (srt_fuzz_survival.rs corpus); no per-file outcome pinned |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Verdict
Genuine truncation mutant, byte-verified: `cmp fuzz/trunc2_096.srt canonical/out2.srt` -> `EOF on fuzz/trunc2_096.srt after byte 96, in line 9` — an exact 96-byte prefix (96 matching the filename's `096`). The cut lands immediately after cue 3's orphaned number line (`3\n`), leaving a block with a cue number but no timing and no text — the mid-block truncation case the survival suite exists for.

## 2. What actually works (verified)
— `cmp` (exit 1, EOF at byte 96) + `od -c`: the 96 bytes are byte-identical to out2.srt's first 96; nothing fabricated. Runs through the parser in `committed_corpus_survives` (4/4 green).

## 3. Findings
None. Checked: prefix provenance (cmp EOF position = 96 = name), cut position semantics (orphan cue number), ASCII purity, size budget.

## 4. Notes for the next agent
The expected outcome (cues 1-2 survive, cue 3 truncated-typed-damage) is unasserted; the exhaustive `truncations_never_beat_the_parser` test independently covers every prefix of out2.srt, making this fixture's marginal value provenance/history rather than coverage.
