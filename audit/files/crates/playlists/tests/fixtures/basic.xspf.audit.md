# AUDIT: crates/playlists/tests/fixtures/basic.xspf
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| File verdict | REAL fixture (test-authored golden, deliberately entity-dense) |
| Reachable from shipped entry point? | NO (test fixture; playlists crate unreachable from shipped apps) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → XML 1.0 document, ASCII text; consumer suite run (below) |

## 1. Content summary
Minimal XSPF 0.1 playlist, 3 tracks, engineered to exercise the entity decoder
and field mapping: (1) `file:` location, `&amp;` in title, `&lt;One&gt;` in
creator, decimal `&#38;` in annotation, duration 213000 ms; (2) http location,
`&quot;`/`&#x27;` quoted creator, duration 0; (3) relative `../relative/`
location, hex `&#233;` unicode title, duration 321500 ms. All five named
entities plus decimal and hex numeric references appear.

## 2. Consumer test
`xspf_suite.rs` (crates/playlists/tests/xspf_suite.rs:18 `const BASIC`):
`golden_basic_entities_and_fields` (L31+) asserts the full typed Entry for
tracks 1-2 — entity-decoded strings ("Song & Dance", "Artist <One>", "First &
foremost", "\"Quoted\" 'name'"), ms→seconds conversion (213.0, 0.0), creator/
annotation riding directive pairs. BASIC is also a fuzz seed (L341, L435).
Ran in my `cargo test -p playlists` run: 20/20 xspf suite tests pass. The
golden would fail on any entity-decode or unit-conversion regression.

## 3. Provenance
No generation header (it is a hand-written golden, not a tool product). The
entity forms are the documented XSPF parser surface (playlists/src/lib.rs:59-63
describes the five named + numeric references). No doc/matrix row cites this
file by name; nothing overstates it. ASCII-only bytes verified via `file`.
