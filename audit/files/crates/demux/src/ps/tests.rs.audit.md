# AUDIT: crates/demux/src/ps/tests.rs
| Field | Value |
|---|---|
| Auditor | W05-A01 |
| Date | 2026-11-21 |
| Lines / functions | 537 / 26 tests + 5 helpers (`Drain`, `drain`, `open`, `track_ticks`, `fixture`) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (cfg(test) suite). Its SUBJECT is reachable: `apps/cli/src/containers/mod.rs:71` `"mpegps" => ps::open` → `apps/cli/src/containers/ps.rs:144-145` `PsDemuxer::new` + `start()`; seek/reanchor via `ps.rs:164` `demuxer.seek(target)` (`--start-time`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | Read of tests.rs + test_support.rs + error.rs; `rg -n "fn seek\|clamp\|scr_extent\|reanchor" crates/demux/tests/ps_fixtures.rs` → 0 hits; `rg -n "\.seek\(" crates/demux/tests/` → mkv/adts/mp4/avi only, no PS; `rg "ps.seek.out-of-range\|ps.resync-failed\|ps.payload-too-large"` repo-wide → only error.rs definition + its own unit test; baseline `sed -n '1874,1900p'` + `sed -n '2217,2224p' audit/runs/baseline/test-workspace.log.txt`; `ls crates/demux/fixtures/ps/` (4 files, 75,776–241,664 B). |

## 1. One-paragraph verdict
A genuinely round-tripping unit suite: every test drives the real `PsDemuxer` over builder-forged bytes and asserts payloads, tracks, damage notes, or typed error codes — no mocks of the subject, no tautologies. But the suite's self-description overstates its coverage twice: the seek test's comment claims the clamp leg is "exercised by the fixture suite's VOB spans" (that suite contains zero `seek` calls), and the module doc claims "every typed error path" (3 of 8 codes are never triggered by any test in the repo). The clamp and the out-of-range/refusal legs sit on the shipped `--start-time` path, untested.

## 2. What actually works (verified)
— All 26 tests executed and passed in baseline: `audit/runs/baseline/test-workspace.log.txt:1876-1900` (`test ps::tests::… … ok`, every name in this file appears); `ps::error::tests` at :1874-1875; integration differential `tests/ps_fixtures.rs` at :2217-2224 ("4 passed", incl. `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit`).
— Builders verified to forge the claimed ffmpeg shapes and the tests pin them: `pack[11] & 0x03 == 0b01` (:86), `pes[6..10] == [0x80, 0x00, 0x01, 0xFF]` (:180), `bytes[13] == 0xF8` (:113) — cross-checked against `test_support.rs:53, 74, 87-93`.
— Round-trip assertions bite: payload equality (`packets[0].1 == vec![0xAA, 0xBB]` :156, strip table `payload[strip as usize..]` :262), track codec/kind (:223-224, :257-259), damage notes (`recovered.contains(&"pes-length-zero")` :408, `"start-code-resync"` :422), typed codes (:351, :359, :374, :387, :396, :429).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ps/tests.rs:455-457 (and module doc :1-7)
Code:
```rust
    // Packs carry SCRs 0 and 90 000: a 50 000-tick target is inside
    // the extent and lands at itself (the clamp leg is exercised by
    // the fixture suite's VOB spans).
    assert_eq!(ts.convert(&base).must().ticks, 50_000);
```
```rust
//! ... nav/padding/program-end handling, every typed
//! error path, seek/reanchor, and the committed `+FB` fuzz-lite
//! derivatives of the ffmpeg fixtures.
```
What is wrong: two false coverage claims. (a) The only `seek` test lands mid-extent (50,000 between SCRs 0 and 90,000) — the identity leg. The comment defers the clamp leg to "the fixture suite's VOB spans", but `crates/demux/tests/ps_fixtures.rs` (4 tests, baseline :2217-2224) never calls `seek`/`reanchor` (its only seek-adjacent line is `ps.seekability()` at :194), and a repo-wide sweep finds `PsDemuxer` seek/reanchor exercised nowhere else (`rg "\.seek\(" crates/demux/tests/` → mkv/adts/mp4/avi hits only). The clamp (`crates/demux/src/ps/mod.rs:1132` `let clamped = u64::try_from(ticks).unwrap_or(0).min(last);`) and the refusal legs (`mod.rs:1121/1126/1130` `seek_out_of_range`) are untested. (b) The module doc's "every typed error path": of the 8 codes in `ps/error.rs`, `demux.ps.resync-failed`, `demux.ps.payload-too-large`, and `demux.ps.seek.out-of-range` are never minted by any test input in the repo (repo-wide rg: only the error.rs unit test constructs them directly, which proves the string, not the trigger).
Why it matters: seek clamping is on the shipped path — `lumen open --start-time` → `apps/cli/src/containers/ps.rs:164` `demuxer.seek(target)` — so a regression in `min(last)` or the out-of-range refusals ships silently; the suite's comments tell the next maintainer the safety net exists when it does not. Baseline `rc-smoke` covers AVI only (INDEX #20), so no runtime lane backstops this either.
What correct looks like: one unit test with a target past the last SCR asserting the clamped landing, one past-the-end/negative target asserting `demux.ps.seek.out-of-range`, one >`MAX_PACKET_BYTES` PES asserting `payload_too_large`, one garbage-past-budget stream asserting `resync-failed` — or corrected comments that say the legs are untested.
Confidence: HIGH — Verification: read + rg sweeps cited above.

### F-002 [S4]
Where: crates/demux/src/ps/tests.rs:189-206
Code:
```rust
    pes.extend_from_slice(&super::test_support::stamp_bytes(0b0011, 5_000));
    pes.extend_from_slice(&super::test_support::stamp_bytes(0b0001, 4_000));
    pes.push(0x7E);
```
What is wrong: the name promises "parses both" (PTS+DTS) but the DTS value (4,000) is never asserted; only the PTS lands on the track (:202). DTS consumption is pinned only indirectly — if the 10-byte stamp block were mis-consumed the payload assertion `packets[0].1 == vec![0x7E]` (:205) would misalign and fail.
Why it matters: a decoder-side DTS value bug (e.g., wrong 3/15/15 split on the second stamp) that keeps byte offsets intact would pass; the test's name promises more than the body.
What correct looks like: assert the packet's DTS-carrying timestamp (or a `Packet` stamp field) equals 4,000, or rename to `…_consumes_the_pair`.
Confidence: HIGH — Verification: read; DTS value absent from all assertions in :190-206.

### F-003 [S4]
Where: crates/demux/src/ps/tests.rs:495-519
Code:
```rust
    for name in ["ps-mp2-video-audio.mpg", "ps-vob-ac3.vob"] {
        let original = fixture(name);
        assert!(original.len() < 200 * 1024, "{name} inside the budget");
```
```rust
                let at = ((state >> 33) as usize) % 512.min(bytes.len());
                bytes[at] ^= (state >> 13) as u8 | 1;
```
What is wrong: the mutation loop flips bytes only in the first `min(512, len)` bytes of each fixture and only for 2 of the 4 committed fixtures (`ps-vob-dts.vob`, `ps-vob-lpcm.vob` get clean-drain only, :522-537). The header-region bias is declared in the comment ("biased into the pack/PES header region") — honest — but the module doc's "the committed `+FB` fuzz-lite derivatives of the ffmpeg fixtures" reads as the fixture set, not half of it.
Why it matters: mid-file PES bodies (where the DTS/LPCM/SPU strip logic bites on real data) are never mutation-tested; the doc overstates the derivative surface. Cosmetic-to-minor.
What correct looks like: rotate the mutation window per round (e.g., `(state >> 33) % len` for some rounds) or state "2 of 4 fixtures, first 512 bytes" in the module doc.
Confidence: HIGH — Verification: read; fixture sizes confirmed by ls (157,696 B and 75,776 B mutated; 192,512 B and 241,664 B clean-only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Drain` (struct) | 24-28 | REAL | — | collector; fine |
| `drain` | 30-49 | REAL | — | drains to EOS/terminal; records code not full error (enough for the assertions made) |
| `open` | 51-55 | REAL | — | construct + `start()`; `.must()` panics on failure (test-only, acceptable) |
| `track_ticks` | 57-63 | REAL | — | converts span start to CLOCK ticks |
| `mpeg1_pack_roundtrips_scr_and_rate` | 67-75 | REAL | — | SCR decode asserted (180,000) + extent; mutation of SCR decode fails it |
| `mpeg1_pack_carries_ffmpegs_trailing_rate_bits_not_the_specs` | 77-91 | REAL | — | pins the ffmpeg `01` divergence; accepted-not-rejected asserted |
| `mpeg2_pack_roundtrips_scr_rate_and_stuffing` | 93-102 | REAL | — | stuffing skip asserted via extent |
| `mpeg2_pack_marker_bits_are_data_never_enforced` | 104-118 | REAL | — | regression pin for the prior `b[11]&1` false-marker bug |
| `stamp_bytes_roundtrip_with_the_parser` | 120-126 | REAL | — | boundary ticks incl. 33-bit max; bites on stamp codec change |
| `mpeg1_no_stamp_packet_carries_the_single_0x0f_byte` | 130-158 | REAL | — | both ffmpeg `0x0F` and spec `0xFF…0x0F` shapes accepted, payload asserted |
| `mpeg1_stamped_pes_roundtrips_pts` | 160-171 | REAL | — | PTS → track ticks asserted; start() idempotency touched |
| `mpeg2_pes_flags_use_marker_bit_zero_and_one_stuffing_byte` | 173-187 | REAL | — | pins ffmpeg flags byte + stuffing; payload asserted |
| `mpeg2_pts_dts_pair_parses_both` | 189-206 | REAL | F-002 | DTS value unasserted (name overpromises) |
| `private1_ac3_strips_substream_and_three_filler_bytes` | 210-228 | REAL | — | strip + codec `ac3` + payload equality; bites |
| `private1_dts_lpcm_and_spu_route_with_their_strips` | 230-264 | REAL | — | 3 families × (codec, kind, strip, payload); `_sub` param unused-but-named (harmless) |
| `private1_unclassified_substream_is_skipped_loudly` | 266-283 | REAL | — | skip is damage-noted — asserted, not silent |
| `private2_nav_packets_form_a_data_track` | 287-306 | REAL | — | Data track + whole-payload passthrough |
| `padding_stream_and_system_header_are_skipped_positionally` | 308-327 | REAL | — | positional skip + `padding-stream` damage note |
| `program_end_stops_the_walk_before_trailing_garbage` | 329-343 | REAL | — | EOS contract; trailing garbage out of scope — asserted |
| `wrong_head_is_the_typed_not_ps` | 347-352 | REAL | — | code `demux.ps.not-ps` |
| `truncated_pack_is_the_typed_truncation` | 354-360 | REAL | — | code `demux.ps.truncated` |
| `bad_pack_prefix_is_typed` | 362-375 | REAL | — | code `demux.ps.bad-pack` |
| `bad_pes_flags_and_stuffing_are_typed` | 377-397 | REAL | — | both mpeg2 flags and mpeg1 vocabulary legs |
| `zero_pes_length_is_damage_noted_never_an_unbounded_read` | 399-409 | REAL | — | `pes-length-zero` recovered reason; unbounded-read guard |
| `damaged_bytes_resync_within_the_window` | 411-423 | REAL | — | resync SUCCESS leg only (failure leg untested, F-001) |
| `not_started_is_typed_and_start_is_idempotent` | 425-433 | REAL | — | `demux.not-started` + replay equality |
| `seek_clamps_into_the_scr_extent_and_reanchor_marks_discontinuity` | 437-462 | REAL (identity leg only) | F-001 | clamp/refusal legs deferred to a suite that lacks them |
| `demo_stream_yields_two_tracks_and_packets` | 466-480 | REAL | — | doctest demo; 2 tracks + clean drain |
| `fixture` | 484-492 | REAL | — | `unreachable!` on missing fixture (test-only); fixtures verified present |
| `fuzz_lite_mutations_of_the_committed_fixtures_never_panic` | 494-519 | REAL | F-003 | deterministic LCG, 120 rounds, header-biased, 2/4 fixtures |
| `fuzz_lite_clean_fixtures_drain_without_terminating_errors` | 521-537 | REAL | — | 4 fixtures, clean drain asserted |

## 5. Test honesty
Covered in §4 per test (all would fail on the mutations named). The suite-wide gaps: no input anywhere triggers `resync_failed` / `payload_too_large` / `seek_out_of_range` (F-001b); no test asserts a clamped seek landing (F-001a); DTS value unasserted (F-002).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every typed error path" (suite scope) | tests.rs:5 (module doc) | FALSE for 3 of 8 codes | repo-wide rg; only error.rs's string-level unit test touches them |
| "seek/reanchor" (suite scope) | tests.rs:6 | SHALLOW — identity leg + reanchor rewind only; clamp + out-of-range untested | tests.rs:437-462; ps_fixtures.rs has no seek |
| "the committed `+FB` fuzz-lite derivatives of the ffmpeg fixtures" | tests.rs:6-7 | SHALLOW — 2 of 4 fixtures mutated, first 512 bytes | tests.rs:499, :509 |
| matrix row `container-mpeg-ps-vob` (mpegps CLI arm) | docs/feature-matrix.toml (PS row, owned by another agent) | REAL-UNPROVEN at shipped-binary level — no PS smoke in baseline (rc-smoke is AVI-only, INDEX #20); demux+tests proven per §2 | baseline INDEX; test log :1874-1900, :2217-2224 |

## 7. Dead / duplicated / unreachable code
None in this file. (`Drain.terminal` unused by most tests — collected for the two that need it; fine.)

## 8. Dependencies & flags
Test-only: `testkit::Must`, `super::test_support`, `super::fuzz::fuzz_demux_ps`, `super::{CLOCK, PsDemuxer, parse_stamp}` — all in-crate. No feature gates; suite compiled and run in the baseline workspace test pass (release profile, INDEX note *).

## 9. Open questions for the next agent
- ps/mod.rs owner: does `reanchor`'s linear pack scan (:1158-1177+) have a budget (the `resync_failed` recovery budget the error doc cites)? The unit test only reanchors a 2-pack stream — cost on real VOBs is unmeasured here.
- error.rs (my file 4): the three untested codes should get trigger tests alongside F-001's additions.
