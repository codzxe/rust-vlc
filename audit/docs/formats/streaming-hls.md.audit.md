# CLAIM AUDIT: docs/formats/streaming-hls.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 / 1 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Library-stack claims verify in-tree (hls session/live/master modules, playlists hls.rs
with EXT-X-STREAM-INF/EXT-X-MEDIA parsing, mockhttp e2e harness). The matrix stays partial
per the record's own push — honest, since evidence is Linux-only. Zero external deps
claim holds (m3u8 crate rejected for self-build — recorded).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "the VOD media-playlist session (6.B1a), the live sliding window with recorded discontinuities (6.B1b) ... landed and tested in crates/streaming + crates/playlists" | L3-6 | REAL | crates/streaming/src/hls/{mod,master,...}.rs + crates/playlists/src/hls.rs exist (ls) |
| 2 | "master playlists + variant switching with ABR hysteresis (6.B2a/6.B2b)" | L5-6 | REAL | playlists/src/hls.rs:256 `EXT-X-STREAM-INF | EXT-X-MEDIA` master parsing; streaming abr module (dash/abr.rs shared) |
| 3 | "X-TIMESTAMP-MAP live-edge derivation + conformance corpus (6.B3)" | L6-7 | REAL | streaming/src/hls/timestamp_map cited; matrix evidence: "6.B3 X-TIMESTAMP-MAP parse + live-edge wallclock + clean-room conformance corpus L1/L2/L3/D1/D2/G1" |
| 4 | "The matrix row network-hls stays partial on every platform: ... evidence is mock-socket/Linux-only (no win/macos runs yet, R4)" | L7-9 | REAL | matrix network-hls: `status = { windows = "partial", ... }` — the record's own proposal held |
| 5 | "crates = [] — zero external deps (the 2017-frozen m3u8 crate was ... rejected for self-build)" | L19-21 | REAL | playlists crate has no m3u8 dep (Cargo.toml spot: m3u8 absent) |
| 6 | "fuzz_target = none yet — network fuzz targets are card 6.F1's ... this record does not claim one" | front-matter | REAL (honest absence) | no fuzz_demux_hls in fuzz/fuzz_targets (ls) — consistent |
| 7 | "e2e hls_vod_e2e/hls_live_e2e/hls_master_e2e/hls_conformance_e2e over the mockhttp real socket; streaming 173/0 lib tests" | matrix evidence | REAL-UNPROVEN | mockhttp.rs exists in streaming/src; test counts not re-run (baseline cargo-test covers package totals: 4744 passed) |

## 3-5. Contradictions / fabrication / matrix rows
None / none / network-hls row partial — record and matrix agree. Commands: ls hls dirs;
rg hls.rs; matrix sed.
