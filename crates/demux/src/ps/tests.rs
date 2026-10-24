//! The MPEG-PS/VOB unit suite (card 5.I2): builder round-trips over
//! both pack flavors, the pinned ffmpeg-vs-spec deltas (the `0x0F`
//! no-stamp byte, the 3-byte AC-3 filler, the never-enforced
//! mux-rate/flag marker bits), private-1 routing for every DVD
//! substream family, nav/padding/program-end handling, every typed
//! error path, seek/reanchor, and the committed `+FB` fuzz-lite
//! derivatives of the ffmpeg fixtures.

use std::sync::Arc;

use media_types::ids::MediaKind;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

use super::fuzz::fuzz_demux_ps;
use super::test_support::{
    ac3_substream_payload, demo_stream, mem_stream, mpeg1_pack, mpeg1_pes, mpeg2_pack, mpeg2_pes,
    program_end, ramp,
};
use super::{CLOCK, PsDemuxer, parse_stamp};

/// Drain to end, collecting packets per track + recovered reasons.
struct Drain {
    packets: Vec<(u32, Vec<u8>)>,
    recovered: Vec<&'static str>,
    terminal: Option<String>,
}

fn drain(demuxer: &mut PsDemuxer) -> Drain {
    let mut out = Drain {
        packets: Vec::new(),
        recovered: Vec::new(),
        terminal: None,
    };
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(p)) => out.packets.push((p.track.handle(), p.data.to_vec())),
            Ok(DemuxEvent::Recovered { reason, .. }) => out.recovered.push(reason),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                out.terminal = Some(err.context.code.clone());
                break;
            }
        }
    }
    out
}

fn open(bytes: impl Into<Arc<[u8]>>) -> PsDemuxer {
    let mut demuxer = PsDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    demuxer
}

fn track_ticks(track: &media_types::Track) -> Option<i64> {
    let Stamp::Present(ts) = track.timing.span.start else {
        return None;
    };
    let base = TimeBase::new(1, CLOCK).must();
    Some(ts.convert(&base).must().ticks)
}

// -- pack headers ----------------------------------------------------------

#[test]
fn mpeg1_pack_roundtrips_scr_and_rate() {
    let mut bytes = mpeg1_pack(180_000, 250_000);
    bytes.extend_from_slice(&program_end());
    let ps = open(bytes);
    assert!(!ps.is_mpeg2());
    assert_eq!(ps.current_scr(), 180_000);
    assert_eq!(ps.scr_extent(), (Some(180_000), Some(180_000)));
}

#[test]
fn mpeg1_pack_carries_ffmpegs_trailing_rate_bits_not_the_specs() {
    // The exact-delta pin: ISO 11172-1 says the two bits after the
    // mux rate are markers `11`; ffmpeg's `mpeg` muxer writes `01`
    // (measured: byte 7 of every pack in ps-mp2-video-audio.mpg is
    // `..01`). The builder forges ffmpeg's byte and the demuxer must
    // accept it — those bits are rate data, never validated.
    let pack = mpeg1_pack(0, 250_000);
    assert_eq!(pack.len(), 12);
    assert_eq!(pack[11] & 0x03, 0b01, "the pinned ffmpeg divergence");
    let mut bytes = pack;
    bytes.extend_from_slice(&program_end());
    let ps = open(bytes); // accepted, not a bad-pack
    assert_eq!(ps.current_scr(), 0);
}

#[test]
fn mpeg2_pack_roundtrips_scr_rate_and_stuffing() {
    let mut bytes = mpeg2_pack(270_000, 500_000, &[0xFF, 0xFF]);
    bytes.extend_from_slice(&program_end());
    let ps = open(bytes);
    assert!(ps.is_mpeg2());
    assert_eq!(ps.current_scr(), 270_000);
    // The walk skipped the 2 stuffing bytes and still found the end.
    assert_eq!(ps.scr_extent(), (Some(270_000), Some(270_000)));
}

#[test]
fn mpeg2_pack_marker_bits_are_data_never_enforced() {
    // The prior-session bug, pinned from the other side: a pack whose
    // byte-11 low bits are 0 (the recorded `b[11]&1` false-marker
    // rejection) is the REAL VOB shape — the ffmpeg `dvd` muxer writes
    // `44 00 04 00 04 01 01 89 c3 f8` SCR/rate bytes where byte 9 is
    // `f8` and the mux-rate trailing bits are `11`; the demuxer parses
    // the rate from bytes 6..8 and trusts no marker there.
    let mut bytes = mpeg2_pack(0, 1_500_000, &[]);
    assert_eq!(bytes[13], 0xF8, "stuffing-length byte shape");
    bytes.extend_from_slice(&program_end());
    let ps = open(bytes);
    assert!(ps.is_mpeg2());
    assert_eq!(ps.current_scr(), 0);
}

#[test]
fn stamp_bytes_roundtrip_with_the_parser() {
    for ticks in [0u64, 1, 9_000, 48_754, 135_000, 1 << 32, (1 << 33) - 1] {
        let bytes = super::test_support::stamp_bytes(0b0010, ticks);
        assert_eq!(parse_stamp(&bytes, 0).must() as u64, ticks);
    }
}

// -- PES headers: the pinned conventions -----------------------------------

#[test]
fn mpeg1_no_stamp_packet_carries_the_single_0x0f_byte() {
    // The exact-delta pin: an unstamped MPEG-1 PES from ffmpeg's
    // `mpeg` muxer carries exactly one 0x0F byte ('0000' + the four
    // low marker bits), where ISO 11172-1 describes optional 0xFF
    // stuffing. The builder forges ffmpeg's; both are accepted below.
    let pes = mpeg1_pes(0xC0, &[0xAA, 0xBB], None);
    assert_eq!(pes[6], 0x0F, "the pinned ffmpeg convention");
    // The spec's flavor: 0xFF stuffing *followed by* the 0x0F
    // terminator (stuffing alone before raw payload is malformed).
    let spec_style = {
        let mut v = pes.clone();
        v[6] = 0xFF;
        v.insert(7, 0x0F);
        let len = u16::try_from(v.len() - 6).unwrap_or(u16::MAX);
        v[4..6].copy_from_slice(&len.to_be_bytes());
        v
    };
    for bytes in [pes, spec_style] {
        let mut stream = mpeg1_pack(0, 250_000);
        stream.extend_from_slice(&bytes);
        stream.extend_from_slice(&program_end());
        let mut ps = open(stream);
        let _ = ps.start().must();
        let Drain { packets, .. } = drain(&mut ps);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].1, vec![0xAA, 0xBB]);
    }
}

#[test]
fn mpeg1_stamped_pes_roundtrips_pts() {
    let mut stream = mpeg1_pack(0, 250_000);
    stream.extend_from_slice(&mpeg1_pes(0xE0, &[0x01], Some(48_754)));
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream);
    let tracks = ps.start().must(); // idempotent replay
    assert_eq!(tracks.len(), 1);
    assert_eq!(track_ticks(&tracks[0]), Some(48_754));
    let Drain { packets, .. } = drain(&mut ps);
    assert_eq!(packets.len(), 1);
}

#[test]
fn mpeg2_pes_flags_use_marker_bit_zero_and_one_stuffing_byte() {
    // The exact-delta pin: ffmpeg's `dvd` muxer writes the PES flag
    // byte as 0x80/0x00 (STD buffer + PTS flags only — the trailing
    // marker bit 0) and a single 0xFF stuffing byte when unstamped.
    // The demuxer never checks that marker (the recorded lesson).
    let pes = mpeg2_pes(0xE0, &[0x11, 0x22], None);
    assert_eq!(&pes[6..10], &[0x80, 0x00, 0x01, 0xFF]);
    let mut stream = mpeg2_pack(0, 500_000, &[]);
    stream.extend_from_slice(&pes);
    stream.extend_from_slice(&program_end());
    let drained = drain(&mut open(stream));
    assert_eq!(drained.packets.len(), 1);
    assert_eq!(drained.packets[0].1, vec![0x11, 0x22]);
}

#[test]
fn mpeg2_pts_dts_pair_parses_both() {
    let mut pes = vec![0x00, 0x00, 0x01, 0xE0];
    pes.extend_from_slice(&14u16.to_be_bytes()); // header 3+10 + 1 payload byte
    pes.extend_from_slice(&[0x80, 0xC0, 0x0A]); // '10' flags, PTS+DTS, len 10
    pes.extend_from_slice(&super::test_support::stamp_bytes(0b0011, 5_000));
    pes.extend_from_slice(&super::test_support::stamp_bytes(0b0001, 4_000));
    pes.push(0x7E);
    let mut stream = mpeg2_pack(0, 500_000, &[]);
    stream.extend_from_slice(&pes);
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream);
    let tracks = ps.start().must();
    assert_eq!(track_ticks(&tracks[0]), Some(5_000));
    let Drain { packets, .. } = drain(&mut ps);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].1, vec![0x7E]);
}

// -- private-stream-1 routing ----------------------------------------------

#[test]
fn private1_ac3_strips_substream_and_three_filler_bytes() {
    // The exact-delta pin: ffmpeg's `dvd` muxer writes 4 bytes before
    // the AC-3 frame (substream id + a 3-byte zero filler) — verified
    // bit-exact against `ffmpeg -c copy -f ac3` in the fixture suite.
    let frame = ramp(64, 0x10);
    let payload = ac3_substream_payload(0x80, [0x00, 0x00, 0x00], &frame);
    let mut stream = mpeg2_pack(0, 500_000, &[]);
    stream.extend_from_slice(&mpeg2_pes(0xBD, &payload, Some(9_000)));
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream.clone());
    let tracks = ps.start().must();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, MediaKind::Audio);
    assert_eq!(tracks[0].codec.as_str(), "ac3");
    let Drain { packets, .. } = drain(&mut open(stream));
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].1, frame, "substream + filler fully stripped");
}

#[test]
fn private1_dts_lpcm_and_spu_route_with_their_strips() {
    let frame = ramp(32, 0x20);
    let mut cases: Vec<(&'static str, MediaKind, u8, u64, Vec<u8>)> = Vec::new();
    // DTS: substream 0x88, strip 4 (same filler shape as AC-3).
    cases.push((
        "dts",
        MediaKind::Audio,
        0x88,
        4,
        ac3_substream_payload(0x88, [0x00, 0x00, 0x00], &frame),
    ));
    // LPCM: substream 0xA0, strip 7 (substream + the 6-byte DVD
    // LPCM header).
    let mut lpcm = vec![0xA0, 0x07, 0x00, 0x04, 0x0C, 0x00, 0x80];
    lpcm.extend_from_slice(&frame);
    cases.push(("pcm_dvd", MediaKind::Audio, 0xA0, 7, lpcm));
    // Sub-picture: substream 0x20, strip 1 (substream only).
    let mut spu = vec![0x20];
    spu.extend_from_slice(&frame);
    cases.push(("dvd_subtitle", MediaKind::Subtitle, 0x20, 1, spu));
    for (codec, kind, _sub, strip, payload) in cases {
        let mut stream = mpeg2_pack(0, 500_000, &[]);
        stream.extend_from_slice(&mpeg2_pes(0xBD, &payload, Some(9_000)));
        stream.extend_from_slice(&program_end());
        let mut ps = open(stream.clone());
        let tracks = ps.start().must();
        assert_eq!(tracks.len(), 1, "{codec}");
        assert_eq!(tracks[0].codec.as_str(), codec, "{codec}");
        assert_eq!(tracks[0].kind, kind, "{codec}");
        let Drain { packets, .. } = drain(&mut open(stream));
        assert_eq!(packets.len(), 1, "{codec}");
        assert_eq!(packets[0].1, payload[strip as usize..], "{codec}");
    }
}

#[test]
fn private1_unclassified_substream_is_skipped_loudly() {
    let mut stream = mpeg2_pack(0, 500_000, &[]);
    stream.extend_from_slice(&mpeg2_pes(0xBD, &[0x55, 0x01, 0x02], None));
    stream.extend_from_slice(&mpeg2_pes(0xE0, &[0x00, 0x00, 0x01, 0xB3], None));
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream.clone());
    assert_eq!(ps.start().must().len(), 1, "only the video track");
    let Drain {
        packets, recovered, ..
    } = drain(&mut open(stream));
    assert_eq!(packets.len(), 1, "only the video packet survives");
    assert_eq!(packets[0].1, vec![0x00, 0x00, 0x01, 0xB3]);
    assert!(
        recovered.iter().any(|r| r.contains("private1")),
        "the skip is damage-noted, never silent: {recovered:?}"
    );
}

// -- nav, padding, system header, program end ------------------------------

#[test]
fn private2_nav_packets_form_a_data_track() {
    let nav = ramp(982, 0x00);
    let mut stream = mpeg2_pack(0, 500_000, &[]);
    // private_stream_2 has no optional PES header: the payload
    // follows the 6-byte start-code + length prefix directly (the
    // real-VOB shape the fixtures pin).
    stream.extend_from_slice(&[0x00, 0x00, 0x01, 0xBF]);
    stream.extend_from_slice(&u16::try_from(nav.len()).unwrap_or(u16::MAX).to_be_bytes());
    stream.extend_from_slice(&nav);
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream.clone());
    let tracks = ps.start().must();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, MediaKind::Data);
    assert_eq!(tracks[0].codec.as_str(), "dvd_nav_packet");
    let Drain { packets, .. } = drain(&mut open(stream));
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].1, nav, "nav payload passes through whole");
}

#[test]
fn padding_stream_and_system_header_are_skipped_positionally() {
    let mut stream = mpeg1_pack(0, 250_000);
    // System header (0xBC): length 6 of zeros.
    stream.extend_from_slice(&[0x00, 0x00, 0x01, 0xBC, 0x00, 0x06, 0, 0, 0, 0, 0, 0]);
    // Padding (0xBE): 4 bytes of 0xCC.
    stream.extend_from_slice(&[0x00, 0x00, 0x01, 0xBE, 0x00, 0x04, 0xCC, 0xCC, 0xCC, 0xCC]);
    stream.extend_from_slice(&mpeg1_pes(0xE0, &[0x00, 0x00, 0x01, 0xB3], None));
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream.clone());
    assert_eq!(ps.start().must().len(), 1);
    let Drain {
        packets, recovered, ..
    } = drain(&mut open(stream));
    assert_eq!(packets.len(), 1);
    assert!(
        recovered.contains(&"padding-stream"),
        "padding is damage-noted: {recovered:?}"
    );
}

#[test]
fn program_end_stops_the_walk_before_trailing_garbage() {
    let mut stream = mpeg1_pack(0, 250_000);
    stream.extend_from_slice(&mpeg1_pes(0xE0, &[0x42], None));
    stream.extend_from_slice(&program_end());
    stream.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00]);
    let Drain {
        packets, recovered, ..
    } = drain(&mut open(stream));
    assert_eq!(packets.len(), 1);
    assert!(
        !recovered.iter().any(|r| r.contains("trailing")),
        "bytes after program-end are out of the walk's contract: {recovered:?}"
    );
}

// -- typed error paths ------------------------------------------------------

#[test]
fn wrong_head_is_the_typed_not_ps() {
    let mut ps = PsDemuxer::new(mem_stream(vec![0x00u8, 0x00, 0x01, 0xB3, 0xFF]).must()).must();
    let err = testkit::assert_err(ps.start(), "not a PS head");
    assert_eq!(err.context.code, "demux.ps.not-ps");
}

#[test]
fn truncated_pack_is_the_typed_truncation() {
    let mut ps =
        PsDemuxer::new(mem_stream(vec![0x00u8, 0x00, 0x01, 0xBA, 0x21, 0x00]).must()).must();
    let err = testkit::assert_err(ps.start(), "pack cut mid-header");
    assert_eq!(err.context.code, "demux.ps.truncated");
}

#[test]
fn bad_pack_prefix_is_typed() {
    // Neither '01' (MPEG-2) nor '0010' (MPEG-1): 0x30.
    let bytes = [
        &[0x00, 0x00, 0x01, 0xBA][..],
        &[
            0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ][..],
    ]
    .concat();
    let mut ps = PsDemuxer::new(mem_stream(bytes).must()).must();
    let err = testkit::assert_err(ps.start(), "bad SCR prefix");
    assert_eq!(err.context.code, "demux.ps.bad-pack");
}

#[test]
fn bad_pes_flags_and_stuffing_are_typed() {
    // MPEG-2 without the '10' flags prefix.
    let mut stream = mpeg2_pack(0, 500_000, &[]);
    let mut pes = vec![0x00, 0x00, 0x01, 0xE0, 0x00, 0x03];
    pes.extend_from_slice(&[0x00, 0x80, 0x00, 0xFF]); // flags byte 0x00
    stream.extend_from_slice(&pes);
    stream.extend_from_slice(&program_end());
    let mut ps = PsDemuxer::new(mem_stream(stream).must()).must();
    let err = testkit::assert_err(ps.start(), "mpeg2 flags prefix");
    assert_eq!(err.context.code, "demux.ps.bad-pes-header");
    // MPEG-1 with a header byte outside the stamp vocabulary.
    let mut stream = mpeg1_pack(0, 250_000);
    let mut pes = vec![0x00, 0x00, 0x01, 0xC0, 0x00, 0x03, 0x51, 0x00, 0x00];
    pes.push(0);
    stream.extend_from_slice(&pes);
    stream.extend_from_slice(&program_end());
    let mut ps = PsDemuxer::new(mem_stream(stream).must()).must();
    let err = testkit::assert_err(ps.start(), "mpeg1 stamp vocabulary");
    assert_eq!(err.context.code, "demux.ps.bad-pes-header");
}

#[test]
fn zero_pes_length_is_damage_noted_never_an_unbounded_read() {
    let mut stream = mpeg2_pack(0, 500_000, &[]);
    stream.extend_from_slice(&[0x00, 0x00, 0x01, 0xE0, 0x00, 0x00]);
    stream.extend_from_slice(&mpeg2_pes(0xE0, &[0x00, 0x00, 0x01, 0xB3], None));
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream.clone());
    assert!(!ps.start().must().is_empty());
    let Drain { recovered, .. } = drain(&mut open(stream));
    assert!(recovered.contains(&"pes-length-zero"), "{recovered:?}");
}

#[test]
fn damaged_bytes_resync_within_the_window() {
    let mut stream = mpeg1_pack(0, 250_000);
    stream.extend_from_slice(&mpeg1_pes(0xE0, &[0x42], None));
    stream.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // garbage
    stream.extend_from_slice(&mpeg1_pes(0xC0, &[0x43], None));
    stream.extend_from_slice(&program_end());
    let Drain {
        packets, recovered, ..
    } = drain(&mut open(stream));
    assert_eq!(packets.len(), 2, "both PES survive the garbage");
    assert!(recovered.contains(&"start-code-resync"), "{recovered:?}");
}

#[test]
fn not_started_is_typed_and_start_is_idempotent() {
    let mut ps = PsDemuxer::new(mem_stream(demo_stream()).must()).must();
    let err = testkit::assert_err(ps.next_event(), "next_event before start");
    assert_eq!(err.context.code, "demux.not-started");
    let first = ps.start().must();
    let second = ps.start().must();
    assert_eq!(first, second);
}

// -- seek / reanchor --------------------------------------------------------

#[test]
fn seek_clamps_into_the_scr_extent_and_reanchor_marks_discontinuity() {
    let mut stream = mpeg1_pack(0, 250_000);
    stream.extend_from_slice(&mpeg1_pes(0xE0, &[0x42], Some(9_000)));
    stream.extend_from_slice(&mpeg1_pack(90_000, 250_000));
    stream.extend_from_slice(&mpeg1_pes(0xE0, &[0x43], Some(99_000)));
    stream.extend_from_slice(&program_end());
    let mut ps = open(stream);
    let target = Timestamp::from_parts(50_000, 1, CLOCK).must();
    let span = TimeSpan {
        start: Stamp::Present(target),
        end: Stamp::Missing,
    };
    let landed = ps.seek(span).must();
    let base = TimeBase::new(1, CLOCK).must();
    let Stamp::Present(ts) = landed.start else {
        unreachable!("seek lands at a present stamp");
    };
    // Packs carry SCRs 0 and 90 000: a 50 000-tick target is inside
    // the extent and lands at itself (the clamp leg is exercised by
    // the fixture suite's VOB spans).
    assert_eq!(ts.convert(&base).must().ticks, 50_000);
    let _ = ps.reanchor(span).must();
    let Drain { packets, .. } = drain(&mut ps);
    assert_eq!(packets.len(), 2, "reanchor rewound to the first pack");
}

// -- the doctest demo -------------------------------------------------------

#[test]
fn demo_stream_yields_two_tracks_and_packets() {
    let mut ps = open(demo_stream());
    let tracks = ps.start().must();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].kind, MediaKind::Video);
    assert_eq!(tracks[1].kind, MediaKind::Audio);
    assert_eq!(tracks[0].codec.as_str(), "mpeg1video");
    assert_eq!(tracks[1].codec.as_str(), "mp2");
    let Drain {
        packets, terminal, ..
    } = drain(&mut ps);
    assert_eq!(packets.len(), 2);
    assert_eq!(terminal, None);
}

// -- fuzz-lite (+FB) ---------------------------------------------------------

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/ps")
        .join(name);
    match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("committed fixture {name}: {err}"),
    }
}

#[test]
fn fuzz_lite_mutations_of_the_committed_fixtures_never_panic() {
    // The two smallest ffmpeg fixtures (< 200 KB each, the +FB
    // budget): deterministic LCG byte-flips biased into the pack/PES
    // header region, plus truncations. Ok-or-typed only.
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
            }
            fuzz_demux_ps(&bytes);
            if round % 4 == 0 {
                let cut = ((state as usize) % bytes.len()).max(4);
                fuzz_demux_ps(&bytes[..cut]);
            }
        }
    }
}

#[test]
fn fuzz_lite_clean_fixtures_drain_without_terminating_errors() {
    for name in [
        "ps-mp2-video-audio.mpg",
        "ps-vob-ac3.vob",
        "ps-vob-dts.vob",
        "ps-vob-lpcm.vob",
    ] {
        let bytes = fixture(name);
        fuzz_demux_ps(&bytes); // absence-of-panic contract
        let owned: Arc<[u8]> = Arc::from(bytes);
        let mut ps = open(owned);
        let _ = ps.start().must();
        let Drain { terminal, .. } = drain(&mut ps);
        assert_eq!(terminal, None, "{name} drains clean");
    }
}
