# AUDIT: fuzz/corpus/fuzz_playlist_m3u/seed_basic_extended.m3u
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 266 B / 2e38838dce38af2960d6d922e98831b1f1d405ad74aa50f0a26a2b9f71c615eb |
| File verdict | REAL (as data); the corpus copy itself is never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-twin executed via include_str! |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (corpus-seed audit)
Real extended M3U seed with format variety: `#EXTINF:123,Sample Artist - Sample Title` + relative path, Windows path (`C:\Music\Example.flac`), http stream URL, `../relative/track01.ogg`, `#EXTINF:-1` live stream, and `https://relay...` — full content verified in batchmeta.txt. sha256-identical to `crates/playlists/tests/fixtures/basic_extended.m3u`, loaded via `const BASIC: &str = include_str!("fixtures/basic_extended.m3u");` (crates/playlists/tests/m3u_suite.rs:12) and round-tripped in `roundtrip_all_fixtures_and_dirty_corpus`.

## 2. What was checked and how
- batch.py: 266 B, 11 lines, content verified.
- sha256 equality with the playlists twin (2e38838d… both, verified).
- Twin execution: m3u_suite passes in baseline (audit/runs/baseline/cargo-test-workspace.txt, playlists suites ok).
- Registry gate (existence only) run by me: exit 0.

## 3. Findings
F-001 [S3] Corpus-lane gap (shared): no stable gate reads fuzz/corpus/fuzz_playlist_m3u/*; campaign unwired. Verification: ran (sha256 + rg).
