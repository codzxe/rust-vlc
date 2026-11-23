# AUDIT: crates/demux/src/ps/tests.rs
| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-23 |
| Lines / functions | 537 / 31 (26 #[test] + 5 helpers) |
| File verdict | REAL (payload-exact, ffmpeg-delta-pinned; two name-vs-body honesty defects around seek/DTS) |
| Reachable from shipped entry point? | Test-only by design (`mod tests` under cfg(test) in ps/mod.rs). Runs under cargo test (baseline cargo-test-workspace.txt:2048-2073, 26/26 ok). Code under test reachable: `PsDemuxer::new` <- apps/cli/src/containers/ps.rs:144 <- `("mpegps") => ps::open` (CLI open path, rg verified). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | sed quote checks (all quotes below byte-verbatim); rg across crates/demux + apps; od hexdump of fixtures/ps/ps-mp2-video-audio.mpg (exit 0); ls fixtures/ps (4 files present). No cargo runs (baseline cited); ffmpeg cross-verification inherited from W04-A08's runs. |

## 1. One-paragraph verdict
A strong, honest in-module suite: every PES lane pins the exact payload bytes after the strip (AC-3 substream+3-filler, DTS 4, LPCM 7, SPU 1), every typed refusal pins its code (`demux.ps.not-ps`/`truncated`/`bad-pack`/`bad-pes-header`), the ffmpeg-vs-spec divergences are pinned from BOTH sides (the forged `0x0F` no-stamp byte AND the spec's `0xFF`-stuffing flavor both accepted; the mux-rate trailing bits `01` accepted where ISO says markers `11` — and I verified against the committed ffmpeg fixture that the divergence is real: byte 11 of ps-mp2-video-audio.mpg's first pack is `0x1d`, low bits `01`), the stamp/pack builders' bit layouts match ISO 11172-1/13818-1 arithmetic (hand-verified: 3/15/15 PTS split, 6-byte SCR + 22-bit rate, MPEG-2 '01'-prefix SCR with extension), and the fuzz-lite lane runs 120 deterministic LCG rounds over two committed ffmpeg fixtures. All 26 tests pass (baseline 2048-2073). The defects are honesty gaps where test names/comments promise more than the body verifies: the seek test neither exercises the clamp branch it is named after (its comment delegates that leg to "the fixture suite's VOB spans" — ps_fixtures.rs contains NO seek call at all, only a `seekability()` assert at line 194, so the `.min(last)` clamp at mod.rs:1132 executes in ZERO repo tests) nor observes the Discontinuity its name promises (drain discards the event); and `mpeg2_pts_dts_pair_parses_both` never asserts the DTS value — the `4_000` stamp is parsed into a field no test reads anywhere in the crate.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt:2048-2073: all 26 `ps::tests::*` pass.
— verified by `od -A d -t x1 -N 32 crates/demux/fixtures/ps/ps-mp2-video-audio.mpg`: first pack `00 00 01 ba 21 00 01 00 01 a1 9b 1d` — byte 11 = `0x1d`, `& 0x03 == 0b01`: the test's "pinned ffmpeg divergence" (tests.rs:86) is a REAL property of the committed ffmpeg file, not a self-generated golden.
— verified by hand against ISO bit math: test_support stamp_bytes (3/15/15 split with marker bits), mpeg1_pack ('0010' + 33-bit SCR + 22-bit rate + 2 trailing bits), mpeg2_pack ('01' + SCR[32..30]+marker+SCR[29..28]... + SCR_ext + rate '11' + 0xF8|stuffing) all implement the standard layouts, so the builder↔parser roundtrip is not purely circular; additionally W04-A08 independently re-derived the PTS extents and payload digests of the same demuxer from ffmpeg extractions (ps_fixtures.rs.audit.md §2) — parse_stamp has an external oracle.
— verified by ls: all four fixtures exist (ps-mp2-video-audio.mpg 75,776 B; ps-vob-ac3.vob 157,696 B; ps-vob-dts.vob 192,512 B; ps-vob-lpcm.vob 241,664 B); the two mutation-loop fixtures are inside the asserted 200 KiB budget.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/ps/tests.rs:437-462 (+ mod.rs:1132, tests/ps_fixtures.rs:194)
Code (verbatim, tests.rs:455-461):
```
    // Packs carry SCRs 0 and 90 000: a 50 000-tick target is inside
    // the extent and lands at itself (the clamp leg is exercised by
    // the fixture suite's VOB spans).
    assert_eq!(ts.convert(&base).must().ticks, 50_000);
    let _ = ps.reanchor(span).must();
    let Drain { packets, .. } = drain(&mut ps);
    assert_eq!(packets.len(), 2, "reanchor rewound to the first pack");
```
What is wrong: the test is named `seek_clamps_into_the_scr_extent_and_reanchor_marks_discontinuity` but verifies neither half of its name. (a) The clamp branch — `let clamped = u64::try_from(ticks).unwrap_or(0).min(last);` (mod.rs:1132) — never executes: a 50_000 target inside a 0..90_000 SCR extent lands at itself, and the comment's claim that "the clamp leg is exercised by the fixture suite's VOB spans" is FALSE — rg over crates/demux/tests/ps_fixtures.rs finds no `.seek(` call at all (only `ps.seekability()` at line 194), so an out-of-extent seek (target > last SCR, or the negative-tick `seek_out_of_range` legs at mod.rs:1121/1130) is executed by ZERO tests in the repo. (b) The discontinuity mark (`self.next_discontinuity = true;` mod.rs:1212, emitted as `DemuxEvent::Discontinuity` on the next packet) is never observed: drain discards it (`Ok(DemuxEvent::Discontinuity) => {}`, tests.rs:40) and the test asserts only the landed ticks and 2 packets. Why it matters: a regression that clamps to the wrong SCR, drops the discontinuity flag, or returns out-of-range on valid targets fails nothing. What correct looks like: seek a target above the last SCR and assert the landing equals the last SCR; capture Discontinuity in Drain and assert exactly one after reanchor. Confidence: HIGH — Verification: read + rg (no seek in ps_fixtures.rs).
F-002 [S3]
Where: crates/demux/src/ps/tests.rs:189-206 (+ drain 30-49)
Code (verbatim, tests.rs:201-205):
```
    let mut ps = open(stream);
    let tracks = ps.start().must();
    assert_eq!(track_ticks(&tracks[0]), Some(5_000));
    let Drain { packets, .. } = drain(&mut ps);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].1, vec![0x7E]);
```
What is wrong: `mpeg2_pts_dts_pair_parses_both` builds a PTS(5_000)+DTS(4_000) pair but asserts only the PTS (via the track-start span) and the payload byte; the DTS stamp `4_000` is parsed into `Packet.dts` and never read — `Drain.packets` is `Vec<(u32, Vec<u8>)>` (handle + bytes only, tests.rs:25), and no other test in the crate reads a PS packet's dts (the integration suite pins "first/last PTS extents" only, per W04-A08's table). If the DTS half of the pair parse regressed (wrong offset, swapped stamps, dropped), every test still passes. What correct looks like: extend Drain to capture `(pts, dts)` per packet and assert `dts == 4_000`, `pts == 5_000`. Confidence: HIGH — Verification: read + rg.
F-003 [S4]
Where: crates/demux/src/ps/tests.rs:494-519
Code (verbatim, tests.rs:499-512):
```
    for name in ["ps-mp2-video-audio.mpg", "ps-vob-ac3.vob"] {
        let original = fixture(name);
        assert!(original.len() < 200 * 1024, "{name} inside the budget");
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        for round in 0..120u32 {
            let mut bytes = original.clone();
            for _ in 0..8 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let at = ((state >> 33) as usize) % 512.min(bytes.len());
                bytes[at] ^= (state >> 13) as u8 | 1;
```
What is wrong: the mutation loop asserts only absence of panic via `fuzz_demux_ps` — a gutted harness body passes (same class as mp4 F-008 / W04-A01 F-003); mutations are confined to the first 512 bytes (deliberate header bias, but the mdat-equivalent body region is untouched); and only 2 of the 4 committed fixtures are mutated (dts 192,512 B and lpcm 241,664 B excluded by the budget). Partially compensated by the companion `fuzz_lite_clean_fixtures_drain_without_terminating_errors` which asserts `terminal == None` on all four. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/demux/src/ps/tests.rs:429 (+ ps/error.rs:104)
Code (verbatim, tests.rs:428-429):
```
    let err = testkit::assert_err(ps.next_event(), "next_event before start");
    assert_eq!(err.context.code, "demux.not-started");
```
What is wrong: the PS module's not-started code is `demux.not-started` (ps/error.rs:104) while every sibling demuxer namespaces it — `demux.mp4.not-started` (mp4/tests.rs:696), `demux.adts.not-started`, `demux.es.not-started`, `demux.wav.not-started` (rg-verified). The test pins the inconsistent code as-is. Any consumer filtering `demux.<format>.*` misses PS's not-started. Root cause lives in error.rs (cross-ref for that auditor); the test merely enshrines it. Confidence: HIGH — Verification: read + rg.
F-005 [S4]
Where: crates/demux/src/ps/tests.rs:354-360 (the only truncation test) vs 399-409
Code (verbatim, tests.rs:355-359):
```
    let mut ps =
        PsDemuxer::new(mem_stream(vec![0x00u8, 0x00, 0x01, 0xBA, 0x21, 0x00]).must()).must();
    let err = testkit::assert_err(ps.start(), "pack cut mid-header");
    assert_eq!(err.context.code, "demux.ps.truncated");
```
What is wrong: truncation coverage stops at the pack header — a PES whose declared length runs past the stream end (the classic mid-PES cut) has no in-module lane; the fuzz truncations (`bytes[..cut]`, tests.rs:514-515) hit that shape only under the panic-only contract. Zero-length PES is covered (399-409), pack cut is covered, mid-PES cut is not. Confidence: HIGH — Verification: read.

## 4. Per-function table (all 31)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Drain struct | 24-28 | REAL | — | handle+bytes, recovered reasons, terminal code |
| drain | 30-49 | REAL | S4 | records terminal (better than mp4's drain) but discards Discontinuity (F-001) |
| open | 51-55 | REAL | — | new + start, must() |
| track_ticks | 57-63 | REAL | — | track-start span → 90 kHz ticks |
| mpeg1_pack_roundtrips_scr_and_rate | 67-75 | REAL | — | SCR + extent pinned |
| mpeg1_pack_carries_ffmpegs_trailing_rate_bits | 77-91 | REAL | — | divergence verified against real fixture byte 0x1d |
| mpeg2_pack_roundtrips_scr_rate_and_stuffing | 93-102 | REAL | — | stuffing skip pinned via extent |
| mpeg2_pack_marker_bits_are_data_never_enforced | 104-118 | REAL | — | 0xF8 shape + no-marker-trust pin |
| stamp_bytes_roundtrip_with_the_parser | 120-126 | REAL | — | edge ticks incl. (1<<33)-1; layout hand-verified vs ISO |
| mpeg1_no_stamp_packet_carries_the_single_0x0f_byte | 130-158 | REAL | — | BOTH flavors (ffmpeg 0x0F + spec 0xFF/0x0F) accepted |
| mpeg1_stamped_pes_roundtrips_pts | 160-171 | REAL | — | 48_754 pinned; start() idempotency exercised |
| mpeg2_pes_flags_use_marker_bit_zero | 173-187 | REAL | — | 0x80/0x00/0x01/0xFF pinned |
| mpeg2_pts_dts_pair_parses_both | 189-206 | REAL | S3 | DTS never asserted (F-002) |
| private1_ac3_strips_substream_and_three_filler_bytes | 210-228 | REAL | — | exact frame bytes after 4-byte strip |
| private1_dts_lpcm_and_spu_route_with_their_strips | 230-264 | REAL | — | 3-lane table, exact strips |
| private1_unclassified_substream_is_skipped_loudly | 266-283 | REAL | — | damage-noted skip pinned |
| private2_nav_packets_form_a_data_track | 287-306 | REAL | — | whole nav payload passthrough |
| padding_stream_and_system_header_are_skipped_positionally | 308-327 | REAL | — | "padding-stream" note pinned |
| program_end_stops_the_walk_before_trailing_garbage | 329-343 | REAL | — | walk stops at 0xB9 |
| wrong_head_is_the_typed_not_ps | 347-352 | REAL | — | code pinned |
| truncated_pack_is_the_typed_truncation | 354-360 | REAL | S4 | only pack-header cut (F-005) |
| bad_pack_prefix_is_typed | 362-375 | REAL | — | 0x30 prefix refused typed |
| bad_pes_flags_and_stuffing_are_typed | 377-397 | REAL | — | both MPEG-1 and MPEG-2 lanes |
| zero_pes_length_is_damage_noted | 399-409 | REAL | — | "pes-length-zero" pinned, bounded read |
| damaged_bytes_resync_within_the_window | 411-423 | REAL | — | "start-code-resync" + both PES survive |
| not_started_is_typed_and_start_is_idempotent | 425-433 | REAL | S4 | pins non-namespaced code (F-004) |
| seek_clamps_..._reanchor_marks_discontinuity | 437-462 | REAL | S3 | name promises clamp+discontinuity; verifies neither (F-001) |
| demo_stream_yields_two_tracks_and_packets | 466-480 | REAL | — | codecs + counts + terminal None |
| fixture | 484-492 | REAL | — | CARGO_MANIFEST_DIR-local, unreachable! on miss |
| fuzz_lite_mutations_of_the_committed_fixtures_never_panic | 494-519 | REAL | S4 | panic-only; 512-byte bias; 2/4 fixtures (F-003) |
| fuzz_lite_clean_fixtures_drain_without_terminating_errors | 521-537 | REAL | — | all 4 fixtures drain clean, terminal None |

## 5. Test honesty (would it fail if the demuxer broke?)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pack roundtrips ×4 | YES | SCR/rate/extent values pinned; layouts hand-verified vs ISO |
| ffmpeg-delta pins ×3 (rate bits, 0x0F, marker bits) | YES | forged bytes asserted in the BUILDER then accepted by the demuxer; divergence verified against committed fixture |
| stamp_bytes_roundtrip | YES | parser pinned to ISO-correct builder across edge ticks |
| PES lanes (mpeg1/2 ×4) | YES | exact payload bytes + PTS via track start |
| private-1 routing ×3 | YES | codec strings, kinds, exact post-strip bytes |
| private-2 nav | YES | whole payload + Data kind |
| padding/system-header | YES | counts + note reason |
| program_end | YES | trailing garbage ignored |
| typed errors ×4 | YES | exact codes |
| zero_pes_length / resync | YES | note reasons + surviving packets |
| not_started/idempotent | YES | code + Vec equality |
| seek/reanchor | PARTIAL | in-extent landing + packet resume only; clamp and Discontinuity unverified (F-001) |
| demo_stream | YES | 2 tracks, codecs, 2 packets, terminal None |
| fuzz_mutations | NO (panic-only) | gutted harness passes (F-003) |
| fuzz_clean_drain | YES (for terminal) | but damage notes invisible (drops Recovered — same as W04-A08 F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the pinned ffmpeg-vs-spec deltas (the 0x0F no-stamp byte, the 3-byte AC-3 filler, the never-enforced mux-rate/flag marker bits)" | tests.rs:2-4 | REAL | 0x0F lane 131-157; rate bits verified on fixture byte 0x1d (od); AC-3 filler bit-exact per W04-A08's independent ffmpeg extraction |
| "measured: byte 7 of every pack in ps-mp2-video-audio.mpg is ..01" | tests.rs:81-82 | REAL | od of first pack: rate trailing byte 0x1d, low bits 01 (SCR-relative byte 7) |
| "the clamp leg is exercised by the fixture suite's VOB spans" | tests.rs:456-457 | FALSE | ps_fixtures.rs has no seek call (rg: only seekability() at 194); clamp branch untested repo-wide |
| "verified bit-exact against ffmpeg -c copy -f ac3 in the fixture suite" | tests.rs:213-214 | REAL | W04-A08 independently reproduced the digest; lane pins pass |
| "private_stream_2 has no optional PES header" | tests.rs:291-293 | REAL | 0xBF + length + payload shape demuxes to dvd_nav_packet |

## 7. Dead / duplicated / unreachable code
None dead. Note: `ramp(982, 0x00)` (tests.rs:289) — the 982-byte nav size matches the real VOB nav packets (980 payload + 2 lead bytes per W04-A08's pins; here the whole 982 rides as payload) — shape-consistent, not a lie.

## 8. Dependencies & flags
media_types (time, MediaKind), plugin_api (Demuxer/DemuxEvent), testkit (Must, assert_err), super::fuzz (fuzz_demux_ps), super::test_support (builders), super::{CLOCK, PsDemuxer, parse_stamp} — production symbols, no test-only seams. Fixtures read from crates/demux/fixtures/ps (committed, all four present). No feature gates.

## 9. Open questions for the next agent
- ps/mod.rs auditor: the seek clamp (`min(last)`, 1132) and the Discontinuity emission (1212) have zero test coverage anywhere — my F-001; also the non-namespaced `demux.not-started` (error.rs:104) vs siblings.
- fuzz.rs auditor: does `fuzz_demux_ps` walk the full demuxer or a reduced parser? The mutation test's value rides entirely on that (F-003).
- Is a mid-PES truncation typed in mod.rs? If yes, it deserves the missing unit lane (F-005).
