# AUDIT: crates/playlists/tests/fixtures/comments_blanks.m3u
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| File verdict | REAL fixture (minimal golden for comment/blank tolerance) |
| Reachable from shipped entry point? | NO (test fixture; playlists crate unreachable from shipped apps) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → M3U playlist, UTF-8 text; `wc` → 10 lines / 170 bytes; consumer suite run (below) |

## 1. Content summary
10-line extended M3U, 170 bytes, LF endings: `#EXTM3U` header; a plain
`# this is a plain comment`; a hostile plain comment containing BOTH a colon
and an em-dash (`#another comment that contains: a colon — not a directive`) —
the case that separates "plain comments are ignored, even with colons" from
directive parsing; blank lines before/between/after entries; two entries with
`#EXTINF:5,T` → `a/b.wav` and `#EXTINF:5.5,U` → `c d.wav` (location with an
INTERNAL SPACE, the verbatim-keeping case). The em-dash makes the file UTF-8
(confirmed by `file`).

## 2. Consumer test
`m3u_suite.rs` (crates/playlists/tests/m3u_suite.rs:15 `const COMMENTS`):
`golden_comments_and_blanks_ignored` (L114-124) asserts zero warnings, exactly
2 entries, locations "a/b.wav" and "c d.wav" verbatim. COMMENTS is also a fuzz
seed (L367). Ran in my `cargo test -p playlists` run: m3u suite passes
(46-test lib binary includes these; 46/46). Would fail if comments were parsed
as directives, blanks dropped entries, or locations were trimmed/split.

## 3. Provenance
Hand-written golden matching the documented discipline (playlists/src/lib.rs:36:
"plain `#` comments (even with colons) are ignored, per the public spec"; L31-33
verbatim locations after line-trim). No tool provenance claim exists for it, and
no doc/matrix row cites it by name.
