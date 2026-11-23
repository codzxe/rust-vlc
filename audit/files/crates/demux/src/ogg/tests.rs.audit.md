# AUDIT: crates/demux/src/ogg/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 575 / 19 tests + 2 helpers (drain, packets) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (#[cfg(test)] module) — the code under test IS reachable: apps/cli/src/containers/ogg.rs uses `OggDemuxer` (lumen-cli release lane) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p demux --lib ogg::tests` → 19 passed / 0 failed (exit 0); sed quote verification |

## 1. One-paragraph verdict
An honest, mutation-hard test suite: every test drives the real `OggDemuxer` over
deterministic built pages (no mocks of the code under test) and asserts exact packets,
stamps, track ids, error codes, and damage counters. The hostile cases I care about are
all present: unbounded lacing growth is capped and typed
(`unterminated_lacing_run_is_a_typed_overrun` pins `demux.ogg.packet-overrun` against
MAX_PACKET_BYTES), CRC corruption recovers, mid-stream garbage resyncs, granule
regression and −1 holes surface as Discontinuities with Missing stamps (never guessed),
and the seek re-anchor landing reports the page-start granule (768, not 1024) with
exactly one marked packet. One test name overpromises: "zero_copy_views" is asserted
nowhere.

## 2. What actually works (verified)
— `cargo test -p demux --lib ogg::tests` -> 19 passed, 0 failed.
— All asserts are exact (packet byte vectors, stamp vectors, error code strings,
Discontinuity positions), so any behavioral break in the page/demuxer layers fails
these tests.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ogg/tests.rs:552-575
Code:
```
fn pool_stays_bounded_and_packets_are_zero_copy_views() {
```
...
```
    assert_eq!(packets(&events).len(), 32);
    // 2 arenas × 256 KiB ceiling, demand-grown: far below in practice.
    assert!(ogg.pool_bytes() <= 2 * super::MAX_PACKET_BYTES as usize);
    assert!(ogg.pool_bytes() > 0);
```
What is wrong: The name promises "packets are zero copy views"; the body checks only the
packet count and the pool byte bound. No pointer-identity or allocation-count assertion
exists anywhere in the ogg module (rg `as_ptr|zero.copy|from_pool_range` over ogg/*.rs
hits only doc comments mod.rs:55,893). The zero-copy property (mod.rs:55 "zero copy,
zero alloc") is UNTESTED here — unlike the AVI twin, whose riff.rs pool tests prove
pointer identity. A regression to per-packet copies would pass this test.
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| drain | 12-21 | REAL | — | collects events + sticky error code |
| packets | 23-33 | REAL | — | (track, bytes, keyframe) extraction |
| single_stream_start_packets_and_duration | 35-66 | REAL | — | exact 3 packets, 128 ms duration, header stash, links/discontinuity counters |
| only_the_last_packet_on_a_page_owns_the_granule | 68-89 | REAL | — | stamps [None, Some(512), Some(1024)] |
| mux_two_serials_two_tracks_interleaved | 91-123 | REAL | — | vorbis+opus, interleaving order, links==1 |
| chained_links_boundary_discontinuity_and_fresh_tracks | 125-176 | REAL | — | Discontinuity at boundary, fresh track id, links==2 |
| continued_packet_assembles_across_pages | 178-208 | REAL | — | 600-byte exact reassembly + tail |
| bad_crc_page_is_dropped_with_sync_recovery | 210-238 | REAL | — | 1 Recovered, packet lost not fatal, damage>0 |
| mid_stream_garbage_recovers_at_the_next_capture | 240-260 | REAL | — | 2 KiB noise + fake capture, 3 packets survive, damage>=2058 |
| truncated_page_is_a_typed_sticky_error | 262-285 | REAL | — | code demux.ogg.truncated-page, sticky replay |
| sequence_discontinuity_is_damage_noted_and_continues | 287-309 | REAL | — | seq jump → Discontinuity |
| granule_regression_is_an_honest_discontinuity | 311-331 | REAL | — | backwards clock |
| minus_one_data_page_punches_a_visible_hole | 333-365 | REAL | — | stamps [Some(256), None, Some(1024), Some(1280)] |
| continued_flag_lies_are_damage_noted | 367-398 | REAL | — | orphan run dropped, real packet survives |
| unterminated_lacing_run_is_a_typed_overrun | 400-425 | REAL | — | the unbounded-growth cap: demux.ogg.packet-overrun |
| no_bos_page_is_a_typed_no_streams | 427-438 | REAL | — | garbage → demux.ogg.no-streams |
| unknown_codec_demuxes_opaque_with_missing_stamps | 440-457 | REAL | — | "ogg-opaque", duration None |
| skeleton_stream_mints_no_track_and_no_packets | 459-477 | REAL | — | fishead serial → no track |
| not_started_and_not_ogg_are_typed | 479-489 | REAL | — | demux.ogg.not-started, SeekSupport::None |
| seek_and_reanchor_land_on_the_last_page_before_the_target | 491-550 | REAL | — | landing 768 (6.FIX), marks [true,false×4] |
| pool_stays_bounded_and_packets_are_zero_copy_views | 552-575 | REAL | S4 | F-001: zero-copy asserted nowhere |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 19 | YES | exact byte/stamp/code/count asserts against the real demuxer over real built pages |
| pool_stays_bounded... | PARTIAL | pool bound yes; the zero-copy half of the name would NOT fail on a copies regression |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the page layer has its own suite in super::page" | :2 | REAL | page.rs exists (not my file) |
| "deterministic bytes, no I/O" | :4 | REAL | mem_stream over built pages only |
| zero-copy pool views (name) | :553 | UNPROVEN | no assertion in this module |

## 7. Dead / duplicated / unreachable code
None: every helper is used; no duplicated builders (test_support.rs owns them).

## 8. Dependencies & flags
#[cfg(test)] only; uses super::test_support builders + plugin_api DemuxEvent/Demuxer.
MAX_PACKET_BYTES = 256 KiB (mod.rs:90) — the pool bound is 2×256 KiB.

## 9. Open questions for the next agent
- ogg/mod.rs:876/897 both gate run_len against MAX_PACKET_BYTES — confirm the
  cross-page accumulator and the single-run gate cannot disagree (integer types).
- Add the pointer-identity pool test to match riff.rs's
  `payload_slots_over_a_real_reader_recycle` (fixes F-001).
