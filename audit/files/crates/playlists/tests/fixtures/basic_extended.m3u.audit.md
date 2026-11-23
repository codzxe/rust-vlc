# AUDIT: crates/playlists/tests/fixtures/basic_extended.m3u
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines | 10 (ASCII; `file` reports "M3U playlist, ASCII text") |
| File verdict | REAL (fixture) |
| Reachable from shipped entry point? | N/A — compile-time test fixture (`include_str!` at tests/m3u_suite.rs:12) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (type confirmed), full read, rg (consumers + doc citations), sed -n (assertions) |

## 1. Verdict
The canonical golden fixture for the extended-M3U parser, and it earns its keep: five location lines covering the format's whole shape space — integer duration + title (`#EXTINF:123,…` + `sample.mp3`), fractional duration + verbatim Windows path (`321.5` + `C:\Music\Example.flac`), an `http://stream.example.com:8000/live.aac` URL under `#EXTINF:10` (reserved-host test double, consistent with the repo's allow-literal policy), a metadata-less bare relative path (`../relative/track01.ogg` — the plain-entry-interleaved-with-EXTINF case), and the `-1` unknown-duration marker + `https://relay.example.net/channel`. Its consumer (`m3u_suite.rs:37-70 golden_basic_extended`, baseline-green in the 18-test suite) asserts the exact first `Entry`, the Windows path verbatim, the relative path kept relative, `-1.0` preserved (not re-interpreted), count == 5, and a warnings-free detailed parse. No doc/matrix row cites this fixture (rg exit 1) — no evidence inflation to check.

## 2. What actually works (verified)
— baseline PACKAGE playlists: `tests/m3u_suite.rs ... 18 passed`, incl. `golden_basic_extended`; every fixture line is consumed by an assertion in that test (verified by side-by-side read — no decorative lines).

## 3. Findings
None. Checked: the five entries each map to asserted fields; the URLs use reserved example hosts only; no BOM/CRLF surprises (plain LF, parser tolerance for those is tested elsewhere with inline strings, correctly not baked into the golden).

## 4-9. Per-function / test honesty / doc claims / dead code / deps / open questions
N/A for a 10-line text fixture; consumer honesty covered in §2. No open questions.
