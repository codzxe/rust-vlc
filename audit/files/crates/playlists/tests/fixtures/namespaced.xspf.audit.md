# AUDIT: crates/playlists/tests/fixtures/namespaced.xspf
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 27 / fixture (text asset) |
| File verdict | REAL (hand-authored XSPF torture fixture, fully consumed by a golden test) |
| Reachable from shipped entry point? | N/A — test fixture; consumer: crates/playlists/tests/xspf_suite.rs:19 (`include_str!`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → "XML 1.0 document, ASCII text"; `cargo test -p playlists --test xspf_suite` → 20 passed/0 failed (own run) |

## 1. One-paragraph verdict
A purpose-built golden fixture, not filler: every line exercises a distinct parser behavior and the consuming test (`golden_namespaced_prefixes_cdata_and_subsets`, xspf_suite.rs:70-112) asserts the exact outcome of each — namespaced `xspf:` prefixes throughout, an `<xspf:extension>` block whose nested foreign-namespace element must be ignored, a duplicate `<xspf:location>` (line 11 → `DuplicateField` warning, first wins), an unknown `<xspf:album>` (ignored), CDATA raw text with `<title>` & `&` un-decoded, a multiline annotation that must be trimmed, a `<meta>` with attributes (ignored), and a self-closing `<xspf:track/>` (line 25 → `TrackWithoutLocation` warning, kept as an empty entry). The test ran green in my run. Nothing in the fixture is dead weight; no doc or matrix row cites this file as product evidence.

## 2. What actually works (verified)
— `cargo test -p playlists --test xspf_suite` → `test result: ok. 20 passed; 0 failed` (own run, includes the fixture's golden test). `file` confirms well-formed ASCII XML.

## 3. Findings
None. Checked: fixture is ASCII (no BOM/encoding traps beyond the separately-tested BOM case), line numbers cited in the test's expected warnings (11, 25) match the fixture's actual duplicate-location and self-closing-track lines (verified by reading the file), UTF-8 valid.

## 4-9. (fixture — per-function table N/A; no tests inside; no doc claims cite it; no dead code; no dependencies; no open questions)
