# AUDIT: crates/playlists/tests/fixtures/directives.m3u
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 9 / fixture (extended M3U) |
| File verdict | REAL (directive-attachment golden fixture, fully consumed) |
| Reachable from shipped entry point? | N/A — test fixture; consumer: crates/playlists/tests/m3u_suite.rs:14 (`include_str!`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → "M3U playlist, ASCII text"; `cargo test -p playlists --test m3u_suite` → 18 passed/0 failed (own run) |

## 1. One-paragraph verdict
A compact, deliberately shaped M3U fixture: `#EXTVLCOPT:http-user-agent=…`, `#EXTGRP:Favourites`, and an unknown `#EXTCUSTOM-UNKNOWN:some value: with colons` precede the first entry (testing in-order directive attachment to the FOLLOWING URI), a `#EXTINF:42,Title with: colon` entry with a colon inside the title, and a second entry with its own `#EXTVLCOPT:http-referrer=…` and a bare `#EXTINF:7.25,` (empty title). The consuming test (`golden_directives_attach_in_order`, m3u_suite.rs:80+) asserts the exact directive vectors per entry — including that the colon-laden custom value is preserved verbatim — and the suite's roundtrip law (`parse → write → parse` stability) is applied to it. Everything in the 9 lines is load-bearing for an assertion; the suite ran green in my run.

## 2. What actually works (verified)
— `cargo test -p playlists --test m3u_suite` → `test result: ok. 18 passed; 0 failed` (own run; includes `golden_directives_attach_in_order` asserting the EXTVLCOPT/EXTGRP/EXTCUSTOM-UNKNOWN vector in order, plus the roundtrip law over this fixture). `file` confirms plain ASCII.

## 3. Findings
None. Checked: 2 entries / 2 URIs consistent (one directive block + EXTINF per entry); no BOM; LF endings; the colon-in-value and colon-in-title traps are the point of the fixture and are asserted, not just present.

## 4-9. (fixture — per-function table N/A; no tests inside; no doc/matrix row cites this file as product evidence; no dead content; no dependencies; no open questions)
