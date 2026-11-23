# AUDIT: crates/subtitles/tests/fixtures/fuzz/trunc2_080.srt
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Type / size | SubRip, ASCII, 80 bytes (truncated mid-text) |
| File verdict | REAL fixture (verified byte-exact derivation; consumed via corpus read_dir) |
| Reachable from shipped entry point? | N/A (test fixture); island crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; od dump; python3: `trunc2_080 == out2[:80]` -> True; Grep consumer -> srt_fuzz_survival.rs:56-70 |

## 1. Verdict
Deterministic truncation of canonical/out2.srt at exactly 80 bytes — verified byte-exact
by python3 (`trunc2_080 == base[:80]` -> True). Name decodes as trunc, parent out2, cut
at 080. The cut lands mid-text of cue 2 ("...00:00:09,250\nSec" + EOF): cue 1 must
survive intact, cue 2 must either survive with truncated text or drop to damage — never
abort the file (the survival contract at srt_fuzz_survival.rs:7-10). Consumed by the
corpus read_dir test (:56-70). Nothing wrong with the fixture.

## 2. What actually works (verified)
— python3 derivation check (True); od dump: file ends at offset 0x50 (80) after "Sec".

## 3. Open questions
- None.
