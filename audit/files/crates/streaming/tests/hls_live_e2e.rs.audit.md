# AUDIT: crates/streaming/tests/hls_live_e2e.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 353 / 5 helpers + 2 tests |
| File verdict | REAL (test file; executed 2/2 pass over real loopback TCP; oracle constants independently re-measured by me — exact) |
| Reachable from shipped entry point? | N/A (test file). The tested surface `streaming::hls::live::open_live` has NO shipped caller: the CLI's http(s) open lane dispatches only `RangedResource` (apps/cli/src/demux_host.rs:427 `open_http_source`, :461-474); an .m3u8 URL in the shipped binary never reaches the live session. Matrix row `hls-playback` honestly `partial` (feature-matrix.toml:1439). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/hls_live_e2e-f29d74ffb26011e2 --nocapture` -> "2 passed" + measured line "100204 bytes byte-exact, seam at 51136 recorded, 455 PES pushes / 0 packets", exit 0; `stat`/`file`/`od` on tests/fixtures/hls/ts_live/* |

## 1. One-paragraph verdict
This is what an e2e test should be: a real `TcpListener` server (mockhttp.rs:454) spoken to by a real HTTP/1.1 client over `std::net::TcpStream` (mockhttp.rs:808-889 — connect_timeout, real request-head serialization, bounded body reads — not an in-memory fake), serving REAL ffmpeg-encoded AAC MPEG-TS segments through a reload-mutating 4-window playlist with the `#EXT-X-DISCONTINUITY` sitting on a genuine 44.1 kHz→22.05 kHz encoder switch, asserting a byte-exact oracle against the on-disk concatenation, the exact 8-request wire shape, exact reload/stall/discontinuity stats, a typed stalled-source failure, and a final demux pass through the real TS machinery. I re-measured the oracle constants against the committed fixtures: `cat(s01,s02,t00,t01)` = 100 204 B and `s01+s02` = 51 136 B — both exact; the committed s.m3u8/t.m3u8 are verifiably ffmpeg's own output (ENDLIST + 0.023/0.031 s flush tails, which the corpus drops, exactly as documented). The one structural caveat: none of this is reachable from the shipped binary — the CLI's URL lane is ranged-HTTP only, no HLS dispatch.

## 2. What actually works (verified)
— `./target/debug/deps/hls_live_e2e-f29d74ffb26011e2 --nocapture` -> `live lane: 100204 bytes byte-exact, seam at 51136 recorded, 455 PES pushes / 0 packets through the ended session (measured)` + `test result: ok. 2 passed`, exit 0.
— Oracle re-measured: `stat` -> s01=25568, s02=25568, t00=23500, t01=25568; sum = 100204 = `ORACLE_BYTES` (hls_live_e2e.rs:66); s01+s02 = 51136 = `DISCONTINUITY_OFFSET` (:68). Exact.
— Fixtures real: `file` -> "MPEG transport stream data" (0x47 sync byte via od); committed ffmpeg playlists carry the exact EXTINF values (2.507756/2.484533/2.507767) reused in the hand-authored windows.
— Real-socket claim verified in mockhttp.rs: `TcpListener::bind(("127.0.0.1", 0))` (:454), client `TcpStream::connect_timeout` (:837).

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/hls_live_e2e.rs:60 (module context) + apps/cli/src/demux_host.rs:427
Code: `use streaming::hls::live::{HlsLiveConfig, open_live};`
What is wrong: the tested surface ships nowhere. `open_live` has zero production callers (grep: definition + tests only); the shipped CLI's `http|https` lane calls `open_http_source` → `RangedResource` (demux_host.rs:427,461-474) — a user opening a live .m3u8 URL in the shipped binary gets the probe/container refusal, never this session machinery. The matrix row is honest (`partial`, feature-matrix.toml:1439-1441, evidence cites these suites as in-crate proof), so this is the unwired-surface class, not fraud. Why it matters: the strongest e2e proof in the streaming crate proves a library, not a player behavior. What correct looks like: dispatch .m3u8 URLs to the HLS session in the open lane (or keep matrix `partial` — which it does). Confidence: HIGH — Verification: ran (grep) + read.
F-002 [S4]
Where: crates/streaming/tests/hls_live_e2e.rs:299-303
Code: `let pes_pushes = match pushes.lock() {\n        Ok(guard) => *guard,\n        Err(_) => unreachable!("the counter lock"),\n    };\n    assert!(pes_pushes > 0, "PES flowed across the seam (measured)");`
What is wrong: the demux-through-real-TS stage asserts only `pes_pushes > 0` — a regression that scrambles every PES payload still passes; and the run measured 0 `Packet` events (the PES consumer tap replaces packetization, so `events` counts nothing — the printed "0 packets" is honest but the variable is dead weight in the assert story). Mitigation: the module doc's "What is NOT claimed here" (:45-53) explicitly scopes out the decode differential — the test does not overclaim. What correct looks like: assert a minimum push count (measured 455) or a payload-length sum. Confidence: HIGH — Verification: ran (output shows 455/0).

## 4. Per-function table (test file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture | 70-72 | REAL | — | testkit fixture_bytes (workspace tests/fixtures root) |
| windows | 77-114 | REAL | — | hand-authored over ffmpeg-measured EXTINF (verified vs committed s/t.m3u8) |
| serve_live | 118-134 | REAL | — | static segment routes + scripted playlist route; real socket |
| live_config | 136-142 | REAL | — | live_edge_back=2, max_stalled_reloads=2 |
| read_exactly | 145-158 | REAL | — | empty-chunk break is caught by the byte-exact assert |
| live_slides_endlist_and_discontinuity_over_a_real_socket | 165-308 | REAL | S4 | byte-exact oracle, ENDLIST EOF + size, seam record, stats, 8-request wire pin, real-TS demux (F-002 weak tail assert) |
| stalled_live_source_fails_typed_live_timeout | 313-353 | REAL | — | typed `streaming.hls.live-timeout`, exact 6-request wire shape, measured stall counts |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| live_slides_endlist_and_discontinuity_over_a_real_socket | YES | byte-exact vs on-disk oracle (any segment corruption/reorder fails), exact request sequence (any reload-policy change fails), exact stats, seam offset exact |
| stalled_live_source_fails_typed_live_timeout | YES | typed error code + exact stall/reload counts + exact wire shape |
Oracle constants are externally checkable (I checked — exact). No self-generated goldens.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "served over a **real socket** by the mock testkit's scripted route" | hls_live_e2e.rs:3-4 | REAL | mockhttp TcpListener/TcpStream verified; test executed over loopback |
| "the honest oracle is cat(s01, s02, t00, t01) = 100 204 bytes (measured this session) … seam at offset 51 136" | hls_live_e2e.rs:40-43 | REAL | re-measured: exact |
| "committed s.m3u8/t.m3u8 are ffmpeg's own output … flush-tail segments (s03.ts, t02.ts) are dropped" | hls_live_e2e.rs:23-28 | REAL | committed playlists carry ENDLIST + 0.023178/0.030933 tails; s03/t02 absent from the corpus |
| "No decode differential across the discontinuity seam … not claimed here" | hls_live_e2e.rs:45-53 | REAL | the assert really is only pes_pushes > 0 — honest scoping |
| hls-playback "partial" + evidence citing hls_live_e2e over mockhttp real socket | feature-matrix.toml:1439-1440 | REAL | suites exist and pass (ran); partial honestly covers the unwired open lane (F-001) |

## 7. Dead / duplicated / unreachable code
None in-file. Tested surface production-dead (F-001). `events` counter (0 packets, PES-tap mode) is informational only.

## 8. Dependencies & flags
streaming (open_live, HlsLiveStream, mockhttp), demux::ts (real TS demuxer), plugin_api (DemuxEvent/SeekOrigin/SourceStream), testkit. No feature gates; no network beyond loopback.

## 9. Open questions for the next agent
- hls/live.rs (not my file): the demand-driven reload model means the test never exercises wall-clock pacing (`pacing_clock` injectable but unused here) — is the paced path tested anywhere? UNVERIFIED.
- If the open lane ever dispatches .m3u8, F-001 flips to stale and the matrix row can move past partial for the live arm.
