# AUDIT: crates/playlists/tests/fixtures/basic.pls
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| File verdict | REAL fixture (test-authored golden covering the dirty-world PLS surface) |
| Reachable from shipped entry point? | NO (test fixture; playlists crate unreachable from shipped apps) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → PLS playlist, ASCII text; consumer suite run (below) |

## 1. Content summary
10-line PLS v2, `[playlist]` INI shape: File1 = http URL with port
(`http://relay.example.com:8000/live.mp3`) + Length1 = -1 (the live/unknown
marker); File2 = relative path `music/track01.ogg`, Length2 = 213; File3 =
Windows drive path `C:\Music\Example.flac`, Length3 = 321.5 (FRACTIONAL —
spec says integer seconds; a deliberate dirty-world case); NumberOfEntries = 3
(honest count), Version = 2. Exercises: scheme variety, -1 sentinel, fractional
length, backslash path verbatim-keeping.

## 2. Consumer test
`pls_suite.rs` (crates/playlists/tests/pls_suite.rs:17 `const BASIC`):
`golden` parse (L25+) asserts the typed triple list (locations verbatim,
Length -1 → unknown, 213 s, 321.5 s), plus NumberOfEntries cross-check; BASIC
is also a fuzz/roundtrip seed (L257, L339). Ran in my `cargo test -p playlists`
run: 20/20 pls suite tests pass. Would fail on any INI-mapping, sentinel, or
verbatim-path regression.

## 3. Provenance
No generation header — hand-written golden matching the documented PLS surface
(playlists/src/lib.rs:56-59: 1-indexed FileN/TitleN/LengthN triples,
NumberOfEntries cross-check "with a damage note on a lying count"). `file`
confirms the format. No doc/matrix row cites this file by name; nothing
overstates it.
