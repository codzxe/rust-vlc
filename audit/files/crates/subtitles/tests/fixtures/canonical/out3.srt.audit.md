# AUDIT: crates/subtitles/tests/fixtures/canonical/out3.srt
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Type / size | SubRip, ASCII + CRLF/LF mixed line endings, 148 bytes, 3 cues |
| File verdict | REAL fixture (canonical parent of the fuzz corpus; consumed by 2 test files) |
| Reachable from shipped entry point? | N/A (test fixture); island crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; Read (full); python3 derivation checks (see fuzz audits); Grep `out3.srt` -> srt_fuzz_survival.rs:76,143 + srt_canonical_differential.rs:105,148 |

## 1. Verdict
Canonical ffmpeg-transcode fixture (provenance asserted by
srt_canonical_differential.rs:12 — "`ffmpeg -i truth3.vtt -c:s srt`" — UNVERIFIED: no
ffmpeg in sandbox to re-derive). Content is a good edge set: cue 1 is a 1 ms cue
(00:00:00,000 --> 00:00:00,001), cue 2 spans 83.456 s with a CRLF mid-text line pair
("Line one\r\nLine two" — the mixed CRLF/LF terminators `file` reports), cue 3 crosses
the hour rollover with an out-of-convention hour 24 (23:59:59,900 --> 24:00:00,100).
Both consumers are real: `canonical_parents_still_parse_exactly` (srt_fuzz_survival.rs:76)
asserts zero damage, and the differential test parses it by name (:105) and loops it (:148).
Nothing wrong with the fixture itself; zero findings.

## 2. What actually works (verified)
— `file` -> "SubRip, ASCII text, with CRLF, LF line terminators"; full read: 3 well-formed
cues; consumers located; child fuzz fixture xff3_144.srt verified byte-derived from this
file (python: byte 144 XOR 0xff == 0x9a — see that audit).

## 3. Open questions
- Does the SRT parser accept hour 24+ as a legal timestamp (claimed "24 h+ hour" coverage
  in the differential's provenance table)? Parser file is another agent's assignment.
