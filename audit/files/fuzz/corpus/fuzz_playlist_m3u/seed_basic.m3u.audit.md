# AUDIT: fuzz/corpus/fuzz_playlist_m3u/seed_basic.m3u
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 0 B / e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 (the empty-file digest) / empty (batch manifest; `ls -la` confirms 0 bytes) |
| Generated vs handmade | EMPTY — the "basic" seed carries no bytes at all, while every sibling in the same directory has real content (seed_basic_extended.m3u 266 B, seed_directives.m3u 240 B, seed_dirty.m3u 187 B, seed_hls_vod.m3u8 238 B, seed_plain_urls.m3u 81 B) |
| Consumer | cargo-fuzz campaign only (fuzz_playlist_m3u target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002) |
| Verdict | STUB. F-001 [S4]: a 0-byte "basic" seed is a degenerate placeholder — the fuzzer gains no parsing surface from it (empty input is the trivial case every target already covers); the name promises a basic playlist shape the bytes do not deliver. Likely a seed-writing bug (the sibling seeds show the intended pattern). No committed run has ever distinguished it (campaign has zero evidenced executions). What correct looks like: a minimal valid `#EXTM3U` + 2-3 entries seed.
