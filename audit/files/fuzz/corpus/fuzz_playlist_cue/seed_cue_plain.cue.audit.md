# AUDIT: fuzz/corpus/fuzz_playlist_cue/seed_cue_plain.cue
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | text seed, 14 lines, 244 bytes |
| File verdict | REAL (valid minimal CUE sheet) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file read; python3 sha256 (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Hand-authored valid CUE seed: REM/PERFORMER/TITLE session commands, `FILE "seed.wav" WAVE`,
two TRACKs with TITLE/PERFORMER/INDEX (00:00:00 and 02:30:00 — mm:ss:ff frames, syntactically
valid). sha256 b914e4d0704afd92... (recomputed).

## 2. Consumers
fuzz/fuzz_targets/fuzz_playlist_cue.rs (harness verified at crates/playlists/src/fuzz.rs:50;
registry gate RAN by me, CLEAN). The playlists crate's CUE unit tests pin the parser's
behavior on this grammar (baseline: playlists PACKAGE exit_code=0).

## 3. Findings
None. Checked: command grammar, INDEX frame format, quoted strings.

## 4. Open questions
None.
