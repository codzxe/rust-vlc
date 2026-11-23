# AUDIT: crates/subtitles/tests/fixtures/vtt/styled.vtt (text fixture — short audit)
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / bytes | 13 / 167 (ASCII, LF-only, no BOM, no trailing newline) — `file`: "WebVTT subtitles, ASCII text" |
| File verdict | REAL — genuine ffmpeg-output fixture doing exactly what its consuming test claims |
| Reachable from shipped entry point? | NO (test fixture; crate-level unreachability as per segment.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cat -A` (full byte dump); `file`; `wc -c`; `cargo test -p subtitles --test vtt_acceptance` -> 4 passed, 0 failed (exit 0). |

## 1. Verdict
Verbatim content (cat -A; `$` = LF; no CR, no BOM, no trailing newline):
```text
WEBVTT

00:01.000 --> 00:03.000
Top anchored cue

00:04.000 --> 00:06.500
Middle cue

00:07.000 --> 00:09.000
Bottom-left cue

00:10.000 --> 00:12.000
Forced left-mid
```
Four settings-free plain cues. Its consumer, `tests/vtt_acceptance.rs:103-117` (`ffmpeg_from_ass_drops_positioning_plain_cues`), pins exact times/texts for all four cues, `positioning == Default::default()` for every cue, and zero damage — the test ran green this session. The fixture is the honest half of an honesty proof: the provenance comment (vtt_acceptance.rs:16-19) states ffmpeg's webvtt encoder drops ALL ASS positioning, so this file's job is to pin THAT behavior (the name records the styled ASS source, not the plain VTT content). Cross-checked: every timing in the file (1s-3s, 4s-6.5s, 7s-9s, 10s-12s) equals the test's expected µs tuple — no fixture/test drift.

## 2. Findings
F-001 [S4]
Where: crates/subtitles/tests/fixtures/vtt/styled.vtt:1-13 (name vs content)
What is wrong: the file named "styled.vtt" contains zero styling — no cue settings, no inline tags, no STYLE/NOTE blocks. A reader auditing "do we test styled VTT?" could mistake coverage that lives in `settings_heavy.vtt`. Mitigated by the provenance comment (the ASS source was styled; ffmpeg stripped it — "recorded honesty gap of the ffmpeg route, not of this parser").
What correct looks like: rename to `from_ass_plain.vtt` (or add a one-line header comment — impossible in WebVTT, so rename).
Confidence: HIGH — Verification: cat -A + consuming test read + test run.

## 3. Checked and clean
Times well-formed (`MM:SS.mmm` short form); no BOM/CRLF oddities; no damage expected and none asserted; fixture is committed (provenance claim "all committed, no test-time ffmpeg needed" verified — parse is pure over the included string).
