# AUDIT: crates/streaming/src/hls/mod.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 31 (doc hub + 4 `pub mod` declarations) |
| File verdict | REAL (hub) — the subtree it declares is UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO for the subtree: lumen-cli links streaming but never references `streaming::hls` (rg over apps/cli: zero matches); the open path (demux_host.rs:427) serves http(s) via RangedResource + container probe, and an .m3u8 URL is probed as a container and refused; product.toml `streaming-hls = false`, and KNOWN-LIMITS.md:37 records "HLS/DASH/RTSP URL sources: typed-refused at the open path". |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | full read; wc -l (submodules); rg (hls tests in baseline, 6.B3 in docs, lumen-cli hls references); ls (docs/formats/streaming-hls.md) |

## 1. One-paragraph verdict
A 31-line module hub whose four declared submodules are real and large — media.rs 1134 loc (VOD), live.rs 1473 (live window), master.rs 1189 (variant ladder + ABR hysteresis), timestamp_map.rs 421 (PTS↔wallclock, 33-bit wrap-exact) — with 39 `hls::*` tests baseline-green plus four e2e binaries (hls_vod/hls_live/hls_master/hls_conformance, 13 more tests, all passing per PACKAGE streaming results). The doc header's card mapping is accurate against the code I verified elsewhere in this wave (media playlists typed through playlists::parse_media_playlist — hls/media.rs:637; master playlists through playlists::parse_master_playlist — hls/master.rs:730; discontinuity recorded, not refused, since 6.B1b — live.rs test `discontinuity_is_recorded_across_the_seam` baseline-green). Two findings: the entire 4248-loc subtree is unreachable from every shipped entry point (honest per KNOWN-LIMITS, but the verdict stands), and the closing line "Later cards land here on top: records + conformance (6.B3)" is stale — 6.B3 landed (docs/formats/streaming-hls.md exists, 133 lines; phase6-exit gate #11 "HLS conformance corpus + X-TIMESTAMP-MAP + records" is merged; the feature-matrix evidence names the L1/L2/L3/D1/D2/G1 conformance corpus).

## 2. What actually works (verified)
— verified by baseline (PACKAGE streaming): 39 `test hls::*` unit tests pass (e.g. `live::tests::discontinuity_is_recorded_across_the_seam`, `live_open_joins_back_from_the_edge`, `paced_reloads_wait_the_rfc8216_cadence`, `slid_out_segments_refuse_typed_at_the_gap`, `timestamp_map_absent_is_none_not_a_guess`) and 4 hls e2e binaries pass (6+2+3+2 = 13 tests).
— verified by file existence + wc: all four declared submodules exist with real content (421-1473 loc each); docs/formats/streaming-hls.md (the 6.B3 record) exists.

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/hls/mod.rs:28-31 (whole subtree)
Code:
```
pub mod live;
pub mod master;
pub mod media;
pub mod timestamp_map;
```
What is wrong: 4248 loc of implemented, tested HLS machinery with no shipped caller. lumen-cli (the only binary linking streaming) never touches `streaming::hls`; an `http(s)://…/x.m3u8` source opens through RangedResource and is probed as a container (refused); `streaming-hls = false` in product.toml. The lane is honestly recorded as blocked (KNOWN-LIMITS.md:37), so this is not a fraud — but per the release-lane rule the whole subtree is UNREACHABLE, and any feature-matrix row claiming HLS playback on a shipped platform is false (the matrix row I found cites the in-crate evidence, not a shipped-platform status — cross-check the row's platform cells in the matrix-chunk audits). Confidence: HIGH — Verification: rg (zero apps/cli references), KNOWN-LIMITS read.

F-002 [S4]
Where: crates/streaming/src/hls/mod.rs:26
Code:
```
//! Later cards land here on top: records + conformance (6.B3).
```
What is wrong: stale. Card 6.B3 (doc 13 row 644: "HLS records + conformance ... AC: `+FB` record complete | docs/formats/streaming-hls.md, tests") has landed: the record doc exists (133 lines) and the conformance corpus e2e (hls_conformance_e2e.rs, 6 tests) is baseline-green — the same file even lists `timestamp_map (card 6.B3)` as landed at L21-24. The closing line reads as if 6.B3 were future work. Confidence: HIGH — Verification: docs/formats/streaming-hls.md existence + baseline test names + doc 13 row.

## 4. Per-function table
Not applicable (31-line hub, no functions). Module inventory: 4 `pub mod` declarations, all resolving to real files.

## 5. Test honesty
No tests in this file; the subtree's 39 unit + 13 e2e tests are baseline-green (names sampled in §2 — they are behavioral pins: cadence, live-edge join, typed refusal tables, seam recording).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "media (6.B1a) — VOD media playlists ... serve the segment concatenation as one continuous SourceStream" | mod.rs:3-8 | REAL | hls/media.rs:637 parse + session; hls_vod_e2e baseline-green |
| "Since 6.B1b it also records #EXT-X-DISCONTINUITY seams instead of refusing them" | mod.rs:9-10 | REAL | live.rs `discontinuity_is_recorded_across_the_seam ... ok` (baseline) |
| "live (6.B1b) — live-edge join, demand-driven reload under a stalled-request budget, ENDLIST termination, typed live-timeout / lost-window refusals" | mod.rs:11-15 | REAL | baseline test names: `live_open_joins_back_from_the_edge`, `stalled_reloads_fail_typed_live_timeout`, `slid_out_segments_refuse_typed_at_the_gap`, `endlist_terminates_and_fixes_the_total` |
| "master (6.B2a/B2b) — variant ladder ... hysteresis band" | mod.rs:16-20 | REAL | master.rs:730 parse_master_playlist; hls_master_e2e (3 tests) incl. the anti-thrash contract test |
| "timestamp_map (6.B3) — PTS↔wallclock anchor ... exact across the 33-bit PTS wraparound" | mod.rs:21-24 | REAL (in-crate) | timestamp_map.rs 8 tests baseline-green; wraparound exactness pinned there (not re-verified by me — not my file) |
| "Later cards land here on top: records + conformance (6.B3)" | mod.rs:26 | FALSE (stale) | F-002 |

## 7. Dead / duplicated / unreachable code
The entire declared subtree is unreachable from shipped binaries (F-001) — compiled into the streaming lib, exercised only by tests. Not dead in-tree.

## 8. Dependencies & flags
No dependencies of its own; submodules use crate::fetch/mockhttp/playlists. No feature flags gate the subtree (it is always compiled; the product-level gate is the open-path refusal, not a cfg).

## 9. Open questions for the next agent
- Which card exposes HLS at the open path (the lane currently refused)? Until it lands, the hls_* feature-matrix rows must stay non-shipped.
- media.rs/live.rs/master.rs/timestamp_map.rs each deserve their own full audit (400-1500 loc, only their entry points were verified here).
