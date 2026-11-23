# AUDIT: crates/playlists/tests/fixtures/dirty.m3u
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Type / size | `file`: ASCII text — 187 bytes, 8 lines |
| Verdict | REAL fixture (actively consumed with exact-warning pins) |
| Cited as evidence? | Not named in docs/matrix (the m3u suites' dirty-world coverage is the evidence; this file is the corpus). Consumed by `m3u_suite.rs:16` (`include_str!` DIRTY). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

Content verified (read in full): line 1 `#EXTM` (truncated header), line 2 `#EXTINF abc,Title Without Colon` (missing colon), line 3 `#EXTINF:not-a-number,Bad Duration`, line 4 `still-plays.mp3`, line 5 `#EXTINF:7,` (empty title), line 6 `after-empty-title.mp3`, line 7 `#EXTINF:11.5,Dangling`, line 8 `#EXTVLCOPT:http-referrer=http://example.com`.
Consumption verified: `m3u_suite.rs:175-196` asserts the EXACT ordered warning vector — `DirectiveMissingColon{1}`, `ExtinfMissingColon{2}`, `BadDuration{3}`, `DanglingEntry{7}` — plus 2 surviving entries, the bad duration dropped with title kept, duration `Some(7.0)`/`None` title on the second, and a round-trip (`assert_roundtrip(DIRTY)`); also a fuzz-lite mutation base (m3u_suite.rs:368). The `#EXTVLCOPT` line intentionally earns NO warning — it is preserved as a directive on the next entry per m3u.rs:14-16 / types.rs:86-91 ("recorded, not resolved"), which is the documented contract, not a silent drop.
This is a genuine dirty-world corpus: every line is a distinct damage class, each pinned by a typed warning. No fabrication indicators.
Commands run: `file` (ASCII), `stat` (187 B), full read, read of the consuming test + parser contract docs.
