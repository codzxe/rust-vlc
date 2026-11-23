# AUDIT: crates/playlists/tests/fixtures/plain_urls.m3u (text fixture)
| Field | Value |
|---|---|
| Auditor | W10-A10 | Date | 2026-11-23 |
| Type / size | `file` -> "ASCII text"; 81 bytes, 4 lines, no trailing oddities |
| File verdict | REAL fixture — deliberately header-less plain-URL list covering the 4 grammar shapes; honest golden test |
| Reachable from shipped entry point? | Test-only (playlists crate parse tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, full read; grep of consumer (crates/playlists/tests/m3u_suite.rs:13,66-77) |

## 1. Blunt verdict
Four lines, four shapes, no `#EXTM3U` — which is the point (the plain-list grammar, not HLS): a relative path (`music/song1.mp3`), an absolute path (`/songs/song2.flac`), an http URL (`http://example.com/stream`), and a Windows backslash relative (`..\sibling\song3.wav`). The golden test (m3u_suite.rs:66-77) asserts 4 entries, all with `duration: None`/`title: None`/no directives, and the Windows path preserved verbatim (`r"..\sibling\song3.wav"`) — it would fail if the parser mangled, joined, or normalized any shape. The URL line is opaque data to the parser (no network involved), correctly so for a playlist-import grammar.
## 2. Consumers & citations
- crates/playlists/tests/m3u_suite.rs:13 `include_str!` + `golden_plain_urls_without_header` (:66).
- docs/feature-matrix.toml:1167 (playlist-m3u row) cites the 4.D1 M3U card evidence family this fixture belongs to.
## 3. Verdict
REAL. No fabrication indicators.
