# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_016.srt (text fixture — short audit)
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / bytes | 1 / 16 (pure prefix of the parent; no trailing newline) |
| File verdict | REAL — verified exact 16-byte truncation of the canonical parent |
| Reachable from shipped entry point? | NO (test fixture; crate-level unreachability per segment.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 byte-diff vs `canonical/out2.srt`; `cat -A`; md5sum; `cargo test -p subtitles --test srt_fuzz_survival` -> 4 passed (exit 0, includes this file via `committed_corpus_survives`). |

## 1. Verdict
Byte-identical to the first 16 bytes of `tests/fixtures/canonical/out2.srt` (python3: no differing byte in the kept range; dropped tail = 128 B starting `-> 00:00:04,000\n...`). Full content, byte-verbatim:
```text
1
00:00:01,000 -
```
The cut lands MID-TOKEN inside cue 1's `-->` arrow — the most hostile truncation point in the whole file (a block that has an index line and a half-written timing line, no text, no blank separator, no trailing newline). This is the stride-16 truncation family's first member (`trunc2_016`..`trunc2_128`); md5 confirms the family relationship with the committed cargo-fuzz seed `fuzz/corpus/fuzz_sub_srt/seed_srt_trunc2.srt` (= trunc2_032). Consumed by `committed_corpus_survives` — ran green this session: the parser must terminate with a valid (empty or single-damaged) document, ledger under cap, no panic. Note the survival suite additionally sweeps EVERY prefix of the canonical parents (`truncations_never_beat_the_parser`, also green), so this committed fixture is representative coverage, not the only truncation coverage — an honest, non-inflating arrangement.

## 2. Findings
None. Checked and how: (a) provenance — python3 proves exact-prefix truncation at 16 B; (b) consumption — survival suite green including this file; (c) mid-token cut confirmed the hostile intent; (d) no doc/matrix citation to contradict (rg over docs/: no matches); (e) no trailing-newline edge is exercised by construction and tolerated by the passing run.
