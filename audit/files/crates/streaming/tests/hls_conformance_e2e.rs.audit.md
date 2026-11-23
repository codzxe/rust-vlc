# AUDIT: crates/streaming/tests/hls_conformance_e2e.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 445 / 6 tests (+4 helpers) |
| File verdict | REAL (as a test suite) — proves an UNREACHABLE capability |
| Reachable from shipped entry point? | N/A (test file); `streaming::hls::live::open_live` it exercises has NO caller outside crates/streaming (CLI http lane rides RangedResource, not the live session) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (open_live users, CLI wiring) exit 0/1; sed -n verifications exit 0; ls/file fixtures exit 0; baseline citation |

## 1. One-paragraph verdict
A high-quality conformance suite: six spec-named vectors (live-edge join, window slide,
reload cadence, two discontinuity-seam shapes, GAP tolerance) served over a REAL
TcpListener (mockhttp.rs:454 binds 127.0.0.1:0), driven through the real live session,
pinned to byte-exact oracles built from real ffmpeg-generated MPEG-TS fixtures, with
request-count and pacing assertions (FakeClock makes the §6.3.4 boundary test
deterministic, both sides of the boundary checked). All 6 pass in the baseline. The
wallclock arithmetic is correct (re-derived by hand: 2020-01-01 epoch ms 1_577_836_800_000;
2^33−900000 = 8_589_034_592; per-segment ms-rounding sums 5_016/7_524). The catch is
reachability: `open_live`/`HlsLiveStream` appear ONLY inside crates/streaming — no
shipped app opens an HLS live session, so this suite is conformance evidence for a
library no product path calls. The HTTP client in the loop is the mock, not UreqClient
(documented, honest — but it means the conformance run never exercises the TLS
transport either).

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt L5641-5656: 6/6 `ok` in 0.04s
(discontinuity_seam_with_continuing_timestamps, ..._encoder_timestamp_reset,
gap_tag_is_tolerated..., live_edge_join_clamps..., window_slide_never_refetches...,
reload_cadence_at_the_exact_target_duration_boundary).
— verified by `ls`/`file`: tests/fixtures/hls/ts_live/{s00,s01,s02,t00,t01}.ts exist
(23500-25944 bytes, "MPEG transport stream data").
— verified by `rg "open_live|HlsLiveStream|HlsLiveConfig" crates/ apps/ tests/`: only
crates/streaming/{src/hls,tests/*} — zero production callers; `rg ... apps/cli/` empty;
playback-engine does not depend on streaming.
— verified by rg mockhttp.rs:454: `TcpListener::bind(("127.0.0.1", 0))` — the socket is real.

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/hls_conformance_e2e.rs:8 (scope) + crates/streaming/src/hls/live.rs
Code: `//! testkit, driven through the live session, and pinned to an`
What is wrong: the "live session" under test (`open_live`, hls/live.rs) is library-only.
The shipped CLI's HTTP lane is `SchemeRegistry::open_http` → `RangedResource`
(demux_host.rs:489-496), a static ranged fetch — no shipped entry point ever constructs
an HLS live session. Any matrix row citing this conformance suite as evidence that LIVE
HLS playback works in the product is inflation: the capability is real, tested, and
unreachable. Confidence: HIGH — Verification: read (repo-wide rg).
F-002 [S4]
Where: crates/streaming/tests/hls_conformance_e2e.rs:109 (and 164, 217, 284, 357, 425)
Code: `    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());`
What is wrong: every vector drives the session through the mock HTTP client, so the
conformance corpus validates session logic over a plaintext socket — never ureq, never
rustls, never real redirect/TLS behavior on the HLS path. The header says "real socket
... by the mock testkit" (honest), but "conformance e2e" invites a stronger reading
than what ran. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/streaming/tests/hls_conformance_e2e.rs:11-17
Code:
```
//! # Clean-room provenance
//!
//! No proprietary vendor test suite is used or copied. Each vector
```
What is wrong: clean-room provenance is a claim, not a verifiable artifact — the
vectors are hand-written playlists (spec-derived literals), which is consistent with
the claim, but nothing in-tree can prove it. UNVERIFIED by nature. Confidence: N/A —
Verification: read (claim consistent with content; no vendor-suite strings found).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture / segment_routes / serve | 40-66 | REAL | — | real TS fixtures routed on a real TCP server |
| read_exactly / cat | 68-89 | REAL | — | byte oracles from fixture concatenation |
| live_edge_join_clamps_to_first_segment_of_a_short_window (L1) | 99-132 | REAL | — | oracle + explicit wallclock ms (LOCAL + 2×2508) |
| window_slide_never_refetches_or_disturbs_the_stream (L2) | 139-189 | REAL | — | stats pin: 6 requests, 3 segments, 0 lost |
| reload_cadence_at_the_exact_target_duration_boundary (L3) | 198-262 | REAL | — | FakeClock; both sides of boundary (paced 0 vs ≥1) |
| discontinuity_seam_with_continuing_timestamps (D1) | 272-326 | REAL | — | seam offset == s00+s01 bytes; map round-trip |
| discontinuity_seam_with_encoder_timestamp_reset (D2) | 335-407 | REAL | — | superseding map, 33-bit wrap arithmetic exact |
| gap_tag_is_tolerated_no_gap_semantics_yet (G1) | 415-445 | REAL | — | pins CURRENT v0 behavior (played, not skipped) — honest |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| L1 join clamps | YES | byte oracle cat(s00,s01) + exact edge ms |
| L2 slide | YES | concatenation oracle + request-count pin (6) — a refetch or lost window breaks stats |
| L3 cadence | YES | both branches asserted: boundary→paced 0, early→paced ≥1 |
| D1 continuing | YES | seam offset in bytes + PTS↔wallclock round-trip |
| D2 reset | YES | superseding anchor values + wrap-point conversions (4 exact ms values) |
| G1 gap | YES | pins play-not-skip (regression pin, not a spec claim) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every vector is a fixture playlist over the REAL ffmpeg-generated TS segments" | L4-6 | REAL | fixtures exist, `file` = MPEG TS; generation evidence deferred to hls_live_e2e.rs |
| "served over a real socket" | L7 | REAL | mockhttp.rs:454 TcpListener::bind |
| "pinned to an explicit oracle" | L8-9 | REAL | all six assert byte-exact concatenations |
| "clean-room ... no proprietary suite" | L11-17 | UNVERIFIED | provenance claim, consistent, not provable |
| G1 "no gap semantics yet ... recorded as pending" | L409-414 | REAL | test asserts the segment IS played (honest v0 pin) |
| (implied) HLS live conformance = shipped capability | suite's card 6.B3 framing | UNREACHABLE | open_live has no production caller (F-001) |

## 7. Dead / duplicated / unreachable code
None in-file. Subject-under-test unreachable (F-001).

## 8. Dependencies & flags
dev-deps: streaming (hls live, clock, mockhttp), plugin-api, media-types, testkit. No
features, no unsafe, no env reads.

## 9. Open questions for the next agent
- Which card was supposed to wire hls::live into apps/cli or playback-engine (a
  `lumen play <live-url>` lane)? The whole live stack is finished and orphaned.
- Does the HLS record/matrix cite 6.B3 as product conformance? If yes → UNREACHABLE row.
