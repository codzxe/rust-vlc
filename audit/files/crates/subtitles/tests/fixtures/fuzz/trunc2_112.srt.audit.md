# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_112.srt
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-23 |
| Size / type | 112 B; `file`: "SubRip, ASCII text" |
| Fixture verdict | REAL — byte-verified truncation of the canonical out2.srt |
| Cited as evidence by | srt_fuzz_survival.rs corpus (read_dir); no matrix row names it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; python3 prefix-compare vs `fixtures/canonical/out2.srt`; `./target/debug/deps/srt_fuzz_survival-e95f86f8859dba6b` -> 4 passed |

## 1. Blunt verdict
Real, correctly-named fuzz-corpus member. Byte-verified: EXACTLY the first 112 bytes of
canonical `out2.srt` (144 B) — matching the name `trunc2_112` (truncation, seed 2 = out2, cut
at 112 = 7 × 16-byte stride, per the documented "systematic truncations at 16-byte strides").
The cut lands inside cue 3's timing line (`00:01:02,050\nQuick flash...` truncated mid-text),
exercising trailing-cue truncation. Parent provenance (ffmpeg) UNVERIFIED — see
srt_canonical_differential.rs audit F-001.

## 2. Verification
python3 compare output: `trunc2_112.srt = out2.srt TRUNCATED to 112 bytes` (byte-exact prefix).
Exercised by `committed_corpus_survives` — executed, `test result: ok. 4 passed; 0 failed`.

## 3. Issues
None in the fixture.
