# AUDIT: fuzz/fuzz_targets/fuzz_playlist_m3u.rs

| Field | Value |
|---|---|
| Auditor | W16-RR5 (orchestrator-completed; orig W16-A10) |
| Date | 2026-11-24 |
| File verdict | REAL (thin libFuzzer shim; campaign UNWIRED) |
| Reachable from shipped entry point? | NO (fuzz crate non-member) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | head, rg -> fuzz/Cargo.toml (registered), playlists::fuzz delegate audit cross-ref |

## 1. One-paragraph verdict
782-byte libFuzzer shim delegating to playlists::fuzz::fuzz_playlist_m3u.
Same honest-not-wired disclosure as the flac twin. The DELEGATE's
subject (playlists crate) is itself unreachable from shipped binaries
(W10) — so even a wired campaign would fuzz a library no product uses.

## 3. Findings
F-001 [S3] Same README-vs-audit.yml fuzz-wiring contradiction as
fuzz_demux_flac.rs F-001 (repo-wide pattern).
F-002 [S4] Fuzzing an orphan crate: zero product consumers of
playlists (W10-A01) — the campaign's blast radius is a library island.
