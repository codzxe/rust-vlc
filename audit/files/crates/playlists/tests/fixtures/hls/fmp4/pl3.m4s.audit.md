# AUDIT: crates/playlists/tests/fixtures/hls/fmp4/pl3.m4s
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Type / size | `file`: data (binary) — 6335 bytes; CMAF/fMP4 media segment |
| Verdict | REAL bytes, INERT fixture — no test in the repo reads them |
| Cited as evidence? | Indirectly only: referenced by name inside `pl.m3u8:14` (`#EXTINF:1.996916,\npl3.m4s`). No doc/matrix row names it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

Structure verified by box walk (python3, big-endian sizes): `styp` (24 B, brands msdh/msix) + `sidx` (52 B) + `moof` (272 B) + `mdat` (5987 B) — sizes sum exactly to 6335, no trailing garbage: a well-formed CMAF segment, consistent with the lane's ffmpeg `-hls_segment_type fmp4` provenance.
F-001 [S4]: the segment bytes are never consumed. `hls_attrs.rs` reads only `pl.m3u8` (text) and checks only `init.mp4`'s length (`fixture_len("fmp4", "init.mp4")` = 764, hls_attrs.rs:192-196 — verified by stat); `netfuzz_sweep.rs:128` seeds from `fmp4/pl.m3u8` text only. The five `pl*.m4s` files (29 138 B total) are committed, structurally valid, and loaded by NOTHING — the fMP4 lane's evidence (EXTINF/MAP parsing, ffprobe duration cross-check) never touches segment bytes. Hygiene: either serve them in an e2e (the ts_live lane shows the shape) or note them as directory-completeness only.
Commands run: `file` (data), `stat` (6335 B), box walk (structure), grep for `pl[0-9].m4s` consumers (playlist text only).
