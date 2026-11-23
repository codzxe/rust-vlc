# AUDIT: crates/subtitles/tests/fixtures/vtt/settings_heavy.vtt
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-23 |
| Size / type | 738 B; `file`: "WebVTT subtitles, Unicode text, UTF-8 (with BOM) text, with CRLF line terminators" |
| Fixture verdict | REAL — honestly-labeled handcrafted fixture, byte-verified, exercised by a passing exact-truth test |
| Cited as evidence by | docs/gates/phase5-records-audit.md:87 ("5.M3: 3 real ffmpeg fixtures + settings_heavy, ledger exact"), docs/worklog/archive-2026-10.md:9445 (disclosed "handcrafted") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `od -c` (BOM `357 273 277` = EF BB BF confirmed; CRLF throughout); `./target/debug/deps/vtt_acceptance-43c3d0b477654f70` -> 4 passed incl. `handcrafted_settings_heavy_equals_truth` |

## 1. Blunt verdict
Genuine fixture, no fraud. The worklog and gate record disclose it is HANDCRAFTED (unlike the
ffmpeg-generated siblings basic/long/styled.vtt), and its content delivers exactly what the
test needs: BOM, CRLF, NOTE/STYLE/REGION blocks, numeric + non-numeric + absent cue ids, every
supported setting form with suffixes (`position:25%,left`, `line:2,start`), unsupported
`vertical:rl size:50%`, a reversed interval (dropped cue), and an out-of-range `position:120%`.

## 2. Verification
- `od -c`: starts `357 273 277 W E B V T T` (UTF-8 BOM + WEBVTT), every line ends `\r\n`.
- Consumed via `include_str!` by tests/vtt_acceptance.rs:33; the test (L120-236) asserts 6
  cues with exact µs/text/positioning AND the exact 5-event damage ledger with 1-based line
  numbers (12 REGION, 28 vertical, 28 size, 34 ReversedInterval, + out-of-range percent).
  Executed: `test result: ok. 4 passed; 0 failed`.
- The fixture also pins the auto-id collision behavior (id "three" → auto id 1, colliding
  with explicit cue id 1) — see vtt/mod.rs audit F-003.

## 3. Issues
None. The fixture is small, purpose-built, disclosed as handcrafted, and its consumer test
asserts byte-exact truth including the damage ledger.
