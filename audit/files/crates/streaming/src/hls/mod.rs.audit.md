# AUDIT: crates/streaming/src/hls/mod.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Lines / functions | 31 / 4 (module doc + 4 `pub mod` declarations; no functions, no re-exports) |
| File verdict | UNREACHABLE (module REAL, all four children implemented and tested; no shipped entry point opens an HLS URL) |
| Reachable from shipped entry point? | **NO.** The module is compiled into the shipped `lumen` binary (apps/cli → streaming) but no app code touches `streaming::hls` (rg "streaming::" apps/cli/src → fetch/ranged/schemes/mockhttp only; rg "hls::" outside crates/streaming → nothing). The public constructors (`media::open_vod`, `master`'s session, `live`'s session) have only in-crate test callers — see the media.rs#part2 audit for the full trace. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `wc -l crates/streaming/src/hls/*.rs` → live 1473, master 1189, media 1134, timestamp_map 421. `rg -n "live-timeout|lost-window|stalled" live.rs` → typed codes exist (:27 `streaming.hls.live-timeout`, :446 `streaming.hls.segment-lost-window`). `rg -n "absolute media sequence" master.rs` → continuity contract present (:46, :985, :1049). `rg -n "33-bit" timestamp_map.rs` → wraparound arithmetic present (:23-25). `ls docs/formats/` → `streaming-hls.md` exists. Baseline: streaming lib tests `:4941` → **212 passed / 0 failed / 2 ignored**; hls::media 8/8 (`:5052-5059`), hls::master 7/7 (`:5045-5051`), timestamp_map (`:5060+`), hls_vod_e2e 2/2 (`:5212-5219`), net_interrupted 3/3 (`:5228-5234`); conformance e2e fixtures present (tests/hls_conformance_e2e.rs: live-edge join, window slide, reload cadence). |

## 1. One-paragraph verdict
A 31-line module aggregator whose doc is a four-bullet map of the HLS stack — and every bullet checks out against the code and the citable baseline: media VOD sessions with recorded discontinuities (my media.rs#part2 audit), live sliding-window sessions with stalled-reload budgets and typed live-timeout/lost-window refusals (codes verified in live.rs), master-playlist selection + hysteresis switching with absolute-sequence continuity (tests green in baseline), and the X-TIMESTAMP-MAP 33-bit-wraparound anchor (arithmetic present, tests green). No re-exports, no logic, nothing to hide behind. The only finding is that the matrix evidence string citing this module quotes a stale lib-test count (173; the crate now runs 212/0/2) — the same under-claiming staleness family as the sd-upnp row. And the headline for the wave: none of this 4,248-line, 212-test stack is reachable from a shipped entry point.

## 2. What actually works (verified)
— All four children exist with the promised surfaces (wc + rg outputs in header).
— Module-doc bullet claims verified one by one:
  - media: "serve the segment concatenation as one continuous SourceStream" + "records #EXT-X-DISCONTINUITY seams" → `open_vod` + `SegmentBoundary`/`HlsVodStats::discontinuities` (media.rs#part2 audit §2; baseline `:5052-5059` 8/8).
  - live: "live-edge join, demand-driven reload under a stalled-request budget, ENDLIST termination, typed live-timeout / lost-window refusals" → live.rs:25-44 (stalled budget, `streaming.hls.live-timeout`, `streaming.hls.segment-lost-window`); conformance e2e `live_edge_join_clamps_to_first_segment_of_a_short_window`, `window_slide_never_refetches_or_disturbs_the_stream`, `reload_cadence_at_the_exact_target_duration_boundary` (tests/hls_conformance_e2e.rs:100-199).
  - master: "variant ladder selected by a bandwidth cap … switches rungs … under a hysteresis band (timeline continuity via absolute media sequence)" → baseline `:5045-5051` (`selection_is_highest_under_cap`, `abr_hysteresis_holds_inside_the_band`, `adaptive_session_switches_on_scripted_bandwidth`); continuity pinned at master.rs:1049.
  - timestamp_map: "exact across the 33-bit PTS wraparound" → timestamp_map.rs:23-25, 82, 111; baseline `bom_mapping_converts_pts_to_epoch_milliseconds … ok`.
— The record named by the matrix evidence exists: `docs/formats/streaming-hls.md` (ls verified).

## 3. Findings

### F-001 [S4] — matrix evidence citing this module quotes a stale lib-test count (173 vs the actual 212)
Where: docs/feature-matrix.toml network-hls evidence (line 1441) vs the baseline run
Code (matrix):
```toml
evidence = "… e2e hls_vod_e2e/hls_live_e2e/hls_master_e2e/hls_conformance_e2e over the mockhttp real socket; streaming 173/0 lib tests; record docs/formats/streaming-hls.md"
```
What is wrong: the evidence string claims "streaming 173/0 lib tests"; the crate's lib suite in the citable baseline is **212 passed / 0 failed / 2 ignored** (`orchrun-test-workspace-nofailfast.log.txt:4941`, the `streaming-cdb9cc116cf1f2b0` binary's result). The network-dash row has the same rot ("streaming 191 lib tests (2 ignored)" — also not 212). The counts drift because each row froze the number at its card's merge time and nothing re-derives them.
Why it matters: the matrix is the project's claimed evidence ledger; both HLS and DASH rows cite test counts that are wrong today. The drift direction is under-claiming (honest), but a ledger nobody re-checks is a ledger nobody can trust in either direction — the same staleness family as the sd-upnp row (ssdp.rs audit F-004).
What correct looks like: evidence strings cite a command + artifact (e.g. "cargo test -p streaming → see audit/runs/baseline/test-workspace.log.txt: 212/0/2"), not a hand-transcribed integer; or an xtask regenerates the count.
Confidence: HIGH — Verification: baseline log line quoted verbatim above.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc (1-27) | 1-27 | REAL | — | All four bullets verified against children + baseline (§2). No overclaim found — including the honest scope line "Later cards land here on top: records + conformance (6.B3)". |
| `pub mod live` | 28 | REAL | — | 1473 lines; owner A02 (part1)/A07 (part2)/A03 (part3). |
| `pub mod master` | 29 | REAL | — | 1189 lines; owner A08. |
| `pub mod media` | 30 | REAL | — | 1134 lines; owner A08 (part1)/W10-A06 (part2, audited). |
| `pub mod timestamp_map` | 31 | REAL | — | 421 lines; owner A04. |

## 5. Test honesty
No tests in this file. The children's suites are the module's evidence: hls::media 8/8, hls::master 7/7, timestamp_map, plus e2e (hls_vod_e2e 2/2, net_interrupted 3/3, hls_conformance_e2e) — all green in the baseline citations above.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| media bullet (VOD, SourceStream concatenation, recorded discontinuities) | mod.rs:5-11 | REAL | media.rs#part2 audit; baseline `:5052-5059`. |
| live bullet (sliding window, stalled budget, typed live-timeout/lost-window) | mod.rs:12-17 | REAL | live.rs:25-44, :446; conformance e2e test names; baseline green. |
| master bullet (cap selection, hysteresis switching, absolute-sequence continuity) | mod.rs:18-21 | REAL | baseline `:5045-5051`; master.rs:1049. |
| timestamp_map bullet (33-bit wraparound-exact anchor) | mod.rs:22-24 | REAL | timestamp_map.rs:23-25; baseline `:5060+`. |
| "streaming 173/0 lib tests" (matrix evidence for this module) | feature-matrix.toml:1441 | STALE/FALSE count | F-001: actual 212/0/2 (baseline `:4941`). |
| "record docs/formats/streaming-hls.md" | feature-matrix.toml:1441 | REAL | File exists (ls). |

## 7. Dead / duplicated / unreachable code
- No re-exports means every consumer path is fully qualified (`crate::hls::media::open_vod`) — fine at this scale, no dead surface.
- The whole module is unreachable from shipped entry points (header) — the aggregate fact for all four children, each individually traced in this wave's audits.

## 8. Dependencies & flags
- No dependencies of its own; the children use `crate::{fetch, clock, buffer, error}` + `playlists` + `plugin_api::SourceStream` + `media_types`. No feature gates — the HLS stack is always compiled into the streaming crate (and thus into `lumen`), dead weight in the shipped binary.

## 9. Open questions for the next agent
- For A02/A07/A03 (live.rs) and A08 (master.rs): the mod.rs doc promises "the same recorded-discontinuity contract" for live as for VOD — verify the live session actually surfaces a boundaries/ discontinuity surface symmetric to `HlsVodStream::segment_boundaries`.
- For the matrix owners: F-001 joins the growing drift list (sd-upnp row, network-dash count) — the wave's cross-cutting recommendation is one xtask that re-derives evidence counts from `audit/runs/baseline/`.
