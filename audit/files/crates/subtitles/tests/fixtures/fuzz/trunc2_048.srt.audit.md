# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_048.srt
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| File verdict | REAL (exact 48-byte truncation of out2.srt, cutting cue 2's timing line mid-field; exercised by the committed-corpus survival test) |
| Reachable | N/A (test fixture) — consumed namelessly by srt_fuzz_survival.rs:56-71 `committed_corpus_survives` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → "SubRip, ASCII text"; `od` hexdump; python3: `d == a[:48]` → True; `cargo test -p subtitles --test srt_fuzz_survival` → 4 passed |

## 1. Verdict
48 bytes, byte-identical to the first 48 bytes of out2.srt (python-verified). The cut lands inside cue 2's header: the file ends with `…Hello world\n\n2\n0` — the cue counter "2" and the first digit of `00:00:05,500` are present, the rest is gone. So cue 1 must survive intact and the half-formed cue 2 must be dropped or typed as damage, never a panic. 48 = 3×16, matching the documented "truncations at 16-byte strides" scheme. The survival invariants pass over it. Note the dedicated `truncations_never_beat_the_parser` test (srt_fuzz_survival.rs:140-149) additionally sweeps EVERY prefix of out2.srt at 1-byte granularity, so this committed artifact is a reproducibility anchor rather than the only truncation coverage. No defects.
