# AUDIT: crates/playlists/tests/fixtures/dirty.xspf
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines | 13 (ASCII text; `file` reports "HTML document" — angle-bracket heuristic, actual content is damaged XSPF) |
| File verdict | REAL (fixture) |
| Reachable from shipped entry point? | N/A — compile-time test fixture (`include_str!` at tests/xspf_suite.rs:20) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` (type confirmed), full read, rg (consumers), sed -n (assertions) |

## 1. Verdict
A purposeful damage corpus, not filler: it delivers nine distinct dirty-world classes in 13 lines — `version="2"` (unexpected version), `<duration>not-a-number</duration>`, unclosed `<title>`, `&badent;` (unknown entity), unclosed `<creator>`, `<track>` closed only by `</trackList>`, `</wrong>` mismatched end tag, a stray second `<playlist>` opener at EOF, and the unclosed root. Its consumer (`xspf_suite.rs:146-188 dirty_fixture_salvages_entries`, baseline-green in the 14-test pls-adjacent suite run) pins the exact 9-warning ordered vector and asserts both tracks survive with salvaged title/creator — the fixture would be dead weight only if the parser ignored any of these shapes, and the test proves it does not. No doc/matrix row cites this fixture as evidence (rg exit 1) — test-internal only, no inflated claims to check.

## 2. What actually works (verified)
— `tests/xspf_suite.rs` (PACKAGE playlists) baseline: 14 passed, incl. the DIRTY consumer; the exact warning vector (lines 1, 5, 6, 10, 10, 8, 12, 13, 1) matches the fixture's damage line-for-line (verified by side-by-side read).

## 3. Findings
None. Checked: every fixture line maps to at least one asserted warning or a surviving-entry assertion; no hidden/extra bytes beyond the 13 lines read; example-host URLs absent (paths only: one.mp3/two.mp3 — no network literals at all).

## 4-9. Per-function / test honesty / doc claims / dead code / deps / open questions
N/A for a 13-line text fixture; consumer honesty covered in §2. No open questions.
