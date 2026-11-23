# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_128.srt
| Field | Value |
|---|---|
| Auditor | W09-A09 | Date | 2026-11-30 |
| Type / size | text fixture (truncated SRT), 128 bytes (`file`: "SubRip, ASCII text") |
| File verdict | REAL (legitimate, exactly-named corpus member) |
| Reachable from shipped entry point? | NO — consumed only by `tests/srt_fuzz_survival.rs` directory walk |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `od -An -c` (byte dump); python prefix check vs `canonical/out2.srt`; `./target/debug/deps/srt_fuzz_survival-8c37ba1040decd00` → 4 passed |

## 1. Verdict
Byte-verified content: the exact 128-byte prefix of `canonical/out2.srt` (144 B) — python check `out2.startswith(fix)` → True, truncated at 128. The cut lands mid-way through cue 3's text: the file ends with `... 00:01:02,050\nQ` (a lone `Q` of "Quick flash cue"), so cue 3 is a text-truncated orphan — exactly the truncation-survival case. The name (`trunc` at 128, the trunc2 series at 16-byte stride) matches the content. Covered by `committed_corpus_survives` (executed: 4/4 pass) and thematically by `truncations_never_beat_the_parser` (which additionally walks EVERY prefix of the canonical parents — a stronger guarantee than this fixture alone).
## 2. Notes
- No by-name citation; collective citation tests/srt_fuzz_survival.rs:2-3 ("systematic truncations at 16-byte strides").
- Same UNVERIFIED generator-script note as the corpus at large.
