# AUDIT: fuzz/corpus/fuzz_playlist_m3u/seed_dirty.m3u
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 187 B / 9a4693cb1841f4d3f06f4e27ce7876ed7e8751896116ea7eba2371c589e8231e / ASCII text (batch manifest) |
| Generated vs handmade | Read in full: genuinely hostile seed — malformed `#EXTM` (missing 3U), `#EXTINF abc,` (space, no colon), non-numeric duration, empty title, 11.5 fractional duration with dangling comma, VLC-specific `#EXTVLCOPT` directive, one plain playable URL. Hand-authored quality. |
| Consumer | cargo-fuzz campaign only (fuzz_playlist_m3u target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002) |
| Verdict | REAL seed bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
