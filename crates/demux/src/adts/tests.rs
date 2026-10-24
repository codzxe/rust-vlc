//! The ADTS/ADIF unit suite (card 5.C2b): header round-trips, frame
//! walk arithmetic, every typed error path, the ADIF CBR walk, and
//! the committed `+FB` fuzz-lite derivatives.

use std::sync::Arc;

use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

use super::test_support::{adif_stream, adts_frame, id3v2_tag, mem_stream};
use super::{AdtsDemuxer, AdtsHeader, SAMPLE_RATES};

/// Drain to end, collecting (packets, recovered, discontinuities,
/// terminal error code).
struct Drain {
    packets: Vec<media_types::Packet>,
    recovered: Vec<(u64, &'static str)>,
    discontinuities: usize,
    terminal: Option<String>,
}

fn drain(demuxer: &mut AdtsDemuxer) -> Drain {
    let mut out = Drain {
        packets: Vec::new(),
        recovered: Vec::new(),
        discontinuities: 0,
        terminal: None,
    };
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(p)) => out.packets.push(p),
            Ok(DemuxEvent::Recovered { skipped, reason }) => out.recovered.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => out.discontinuities += 1,
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                out.terminal = Some(err.context.code.clone());
                break;
            }
        }
    }
    out
}

fn open(bytes: impl Into<Arc<[u8]>>) -> AdtsDemuxer {
    let mut demuxer = AdtsDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    demuxer
}

// -- header parsing --------------------------------------------------------

#[test]
fn header_parse_roundtrips_the_field_set() {
    let frame = adts_frame(1, 4, 2, 1, 100, None);
    let head = AdtsHeader::parse(&frame).must();
    assert!(!head.mpeg2);
    assert!(head.protection_absent);
    assert_eq!(head.profile, 1);
    assert_eq!(head.sampling_index, 4);
    assert!(!head.private_bit);
    assert_eq!(head.channel_config, 2);
    assert!(head.original_copy);
    assert!(head.home);
    assert_eq!(head.frame_length, u16::try_from(frame.len()).must());
    assert_eq!(head.buffer_fullness, 0x7FF);
    assert_eq!(head.blocks_minus_one, 0);
    assert_eq!(head.samples(), 1024);
    assert_eq!(head.sample_rate(), Some(44_100));
    assert_eq!(head.header_len(), 7);
    assert_eq!(head.descriptor().as_str(), "aac-lc");

    let protected = adts_frame(0, 0, 1, 4, 90, Some(false));
    let head = AdtsHeader::parse(&protected).must();
    assert!(!head.protection_absent);
    assert_eq!(head.header_len(), 9);
    assert_eq!(head.blocks_minus_one, 3);
    assert_eq!(head.samples(), 4096); // 4 raw data blocks
    assert_eq!(head.sample_rate(), Some(96_000));
    assert_eq!(head.descriptor().as_str(), "aac-main");
}

#[test]
fn probe_detects_adts_adif_and_rejects_the_rest() {
    assert!(super::probe(&[0xFF, 0xF1, 0x44, 0x40]));
    assert!(super::probe(b"ADIF\x00"));
    assert!(!super::probe(&[0xFF, 0x0F, 0x00, 0x00])); // no syncword
    assert!(!super::probe(b"RIFF"));
    assert!(!super::probe(b"\xFF"));
}

#[test]
fn rate_table_matches_the_spec() {
    assert_eq!(SAMPLE_RATES[0], 96_000);
    assert_eq!(SAMPLE_RATES[1], 88_200);
    assert_eq!(SAMPLE_RATES[11], 8_000);
    assert_eq!(SAMPLE_RATES[12], 7_350);
}

// -- the clean walk ---------------------------------------------------------

#[test]
fn clean_walk_stamps_frame_arithmetic() {
    let mut bytes = Vec::new();
    for blocks in [1u8, 1, 2, 1] {
        bytes.extend_from_slice(&adts_frame(1, 4, 2, blocks, 100, None));
    }
    let mut demuxer = open(bytes);
    let info = demuxer.stream_info().must();
    assert_eq!(
        (info.format, info.rate, info.channels, info.profile),
        ("adts", 44_100, 2, 1)
    );
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 4);
    assert!(out.terminal.is_none());
    assert!(out.recovered.is_empty());
    // pts = cumulative samples; duration = samples/rate; the third
    // frame carries 2 blocks (2048 samples).
    let expected_pts = [0i64, 1024, 2048, 4096];
    let expected_dur = [1024u64, 1024, 2048, 1024];
    for (i, packet) in out.packets.iter().enumerate() {
        match packet.pts {
            Stamp::Present(t) => {
                assert_eq!(t.ticks, expected_pts[i], "packet {i}");
                assert_eq!(t.base.as_parts(), (1, 44_100));
            }
            Stamp::Missing => unreachable!("packet {i} is stamped"),
        }
        let media_types::time::Duration::Known(ns) = packet.duration.must() else {
            unreachable!("packet {i} has a duration");
        };
        let want = u128::from(expected_dur[i]) * 1_000_000_000u128 / 44_100u128;
        assert_eq!(u128::from(ns.0), want, "packet {i} duration");
        assert!(!packet.discontinuity);
        assert!(packet.keyframe);
    }
    assert_eq!(demuxer.frames_demuxed(), 4);
    // Duration after the drain: 5120 samples at 44.1 kHz.
    assert_eq!(
        demuxer.track_duration(),
        Some(media_types::time::Duration::Known(
            media_types::time::Nanoseconds(116_099_773)
        ))
    );
}

#[test]
fn id3_head_and_interframe_tags_are_skipped() {
    let mut bytes = id3v2_tag(&[0xAB; 300]);
    bytes.extend_from_slice(&adts_frame(1, 4, 2, 1, 100, None));
    let mut mid = id3v2_tag(&[0xCD; 64]);
    bytes.append(&mut mid);
    bytes.extend_from_slice(&adts_frame(1, 4, 2, 1, 100, None));
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 2);
    assert!(out.terminal.is_none());
    assert_eq!(demuxer.id3_skipped(), 310 + 74);
}

#[test]
fn pool_memory_stays_bounded() {
    // Far more frames than pool slots: the resident bytes must stay
    // at the two-slot ceiling, never one allocation per packet.
    let mut bytes = Vec::new();
    for _ in 0..64 {
        bytes.extend_from_slice(&adts_frame(1, 4, 2, 1, 200, None));
    }
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 64);
    assert!(demuxer.pool_bytes() <= 2 * super::MAX_FRAME_BYTES as usize);
    assert!(demuxer.pool_bytes() > 0);
}

// -- the typed error paths (card step 4: test EACH) ------------------------

#[test]
fn lost_sync_mid_stream_resyncs_with_recovered_events() {
    let mut bytes = Vec::new();
    let f1 = adts_frame(1, 4, 2, 1, 100, None);
    let f2 = adts_frame(1, 4, 2, 1, 100, None);
    let f3 = adts_frame(1, 4, 2, 1, 100, None);
    bytes.extend_from_slice(&f1);
    let mut damaged = f2.clone();
    damaged[0] = 0x12; // syncword smashed
    bytes.extend_from_slice(&damaged);
    bytes.extend_from_slice(&f3);
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    // f2 is skipped (Recovered), f1 + f3 demux; the post-gap packet
    // carries the discontinuity mark.
    assert_eq!(out.packets.len(), 2);
    assert_eq!(out.recovered.len(), 1);
    assert_eq!(out.recovered[0].0, u64::try_from(f2.len()).must());
    assert!(out.packets[1].discontinuity);
    assert!(out.terminal.is_none());
    // The clock counts emitted samples only (ffmpeg's raw-ADTS
    // arithmetic): f2's 1024 samples are a real hole, exposed by the
    // discontinuity mark — never smoothed into the timeline.
    match out.packets[1].pts {
        Stamp::Present(t) => assert_eq!(t.ticks, 1024),
        Stamp::Missing => unreachable!("f3 is stamped"),
    }
}

#[test]
fn truncated_frame_is_the_typed_truncation() {
    let frame = adts_frame(1, 4, 2, 1, 100, None);
    let mut bytes = frame.clone();
    bytes.extend_from_slice(&frame);
    let cut: Vec<u8> = bytes[..bytes.len() - 40].to_vec(); // 2nd frame short
    let mut demuxer = AdtsDemuxer::new(mem_stream(cut).must()).must();
    demuxer.start().must();
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 1);
    assert_eq!(out.terminal.as_deref(), Some("demux.adts.truncated"));
    // Sticky: the error replays.
    let again = demuxer.next_event();
    assert_eq!(
        again.err().map(|e| e.context.code),
        Some("demux.adts.truncated".to_string())
    );
}

#[test]
fn truncated_first_frame_is_the_typed_truncation_at_start() {
    let frame = adts_frame(1, 4, 2, 1, 100, None); // 107 declared
    let cut: Vec<u8> = frame[..60].to_vec(); // header intact, payload cut
    let mut demuxer = AdtsDemuxer::new(mem_stream(cut).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.truncated");
}

#[test]
fn sampling_index_fifteen_is_typed() {
    let frame = adts_frame(1, 15, 2, 1, 100, None);
    let mut demuxer = AdtsDemuxer::new(mem_stream(frame).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.bad-sampling-index");
}

#[test]
fn channel_config_zero_is_typed() {
    let frame = adts_frame(1, 4, 0, 1, 100, None);
    let mut demuxer = AdtsDemuxer::new(mem_stream(frame).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.bad-channel-config");
}

#[test]
fn crc_mismatch_is_typed_and_good_crc_walks() {
    let bad = adts_frame(1, 4, 2, 1, 100, Some(true));
    let mut demuxer = AdtsDemuxer::new(mem_stream(bad).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.crc-mismatch");

    let good = adts_frame(1, 4, 2, 1, 100, Some(false));
    let mut demuxer = open(good);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 1);
    assert_eq!(out.packets[0].data.len(), 109); // 9-byte header + payload
    assert!(out.terminal.is_none());
}

#[test]
fn frame_shorter_than_its_header_is_typed_bad_header() {
    // flen field forced to 0 (< the 7-byte header): impossible
    // structure at the first frame.
    let mut frame = adts_frame(1, 4, 2, 1, 100, None);
    frame[3] &= 0xFC; // flen[12:11] = 0
    frame[4] = 0; // flen[10:3] = 0
    frame[5] &= 0x1F; // flen[2:0] = 0 → flen = 6
    let mut demuxer = AdtsDemuxer::new(mem_stream(frame).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.bad-header");
}

#[test]
fn garbage_head_without_sync_is_the_typed_not_adts() {
    let mut demuxer = AdtsDemuxer::new(mem_stream(vec![0x00; 512]).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.not-adts");
    // Not even an ID3-tagged one.
    let mut bytes = id3v2_tag(&[0x11; 32]);
    bytes.extend_from_slice(&[0x00; 64]);
    let mut demuxer = AdtsDemuxer::new(mem_stream(bytes).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.not-adts");
}

#[test]
fn nonzero_layer_head_is_not_adts() {
    // A syncword whose layer bits are nonzero is not an ADTS claim;
    // with no other candidate the file is typed not-adts.
    let mut bytes = vec![0xFF, 0xF4, 0x44, 0x40, 0x00, 0x4C, 0x61];
    bytes.extend_from_slice(&[0x00; 64]);
    let mut demuxer = AdtsDemuxer::new(mem_stream(bytes).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.not-adts");
}

#[test]
fn trailing_fragment_and_lost_sync_at_end_are_clean() {
    let mut bytes = adts_frame(1, 4, 2, 1, 100, None);
    bytes.extend_from_slice(&[0x01, 0x02, 0x03]);
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 1);
    assert!(out.terminal.is_none());
    assert_eq!(out.recovered.len(), 1);
    assert_eq!(out.recovered[0].0, 3);

    let mut bytes = adts_frame(1, 4, 2, 1, 100, None);
    bytes.extend_from_slice(&[0x00; 128]); // no syncword until EOF
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 1);
    assert!(out.terminal.is_none());
    assert_eq!(out.recovered.len(), 1);
    assert_eq!(out.recovered[0].0, 128);
}

#[test]
fn not_started_is_typed() {
    let frame = adts_frame(1, 4, 2, 1, 100, None);
    let mut demuxer = AdtsDemuxer::new(mem_stream(frame).must()).must();
    let err = demuxer.next_event().err().must();
    assert_eq!(err.context.code, "demux.adts.not-started");
    let err = demuxer
        .reanchor(media_types::time::TimeSpan::UNKNOWN)
        .err()
        .must();
    assert_eq!(err.context.code, "demux.adts.not-started");
}

#[test]
fn mid_stream_parameter_change_is_a_discontinuity() {
    let mut bytes = adts_frame(1, 4, 2, 1, 100, None);
    bytes.extend_from_slice(&adts_frame(1, 4, 1, 1, 100, None)); // channels 2→1
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 2);
    assert_eq!(out.discontinuities, 1);
    assert!(out.packets[1].discontinuity);
    // The clock keeps the track's base (the arithmetic stays
    // explicit — 2 emitted frames — while the change is exposed).
    match out.packets[1].pts {
        Stamp::Present(t) => assert_eq!(t.ticks, 1024),
        Stamp::Missing => unreachable!("stamped"),
    }
}

// -- ADIF -------------------------------------------------------------------

#[test]
fn adif_cbr_walk_matches_the_block_arithmetic() {
    let frames = 7;
    let frame_len = 100;
    let bytes = adif_stream(3, 2, frames, frame_len);
    let mut demuxer = open(bytes.clone());
    let info = demuxer.stream_info().must();
    assert_eq!(
        (info.format, info.rate, info.channels, info.profile),
        ("adif", 48_000, 2, 1)
    );
    assert_eq!(demuxer.stream_info().must().profile, 1);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), frames);
    assert!(out.terminal.is_none());
    // The header size is format-derived; every block is frame_len
    // bytes of the deterministic ramp.
    let head_len = bytes.len() - frames * frame_len;
    assert_eq!(head_len, 14, "the minimal single-program PCE header");
    for (f, packet) in out.packets.iter().enumerate() {
        match packet.pts {
            Stamp::Present(t) => assert_eq!(t.ticks, i64::try_from(f).must() * 1024),
            Stamp::Missing => unreachable!("block {f} is stamped"),
        }
        assert_eq!(packet.data.len(), frame_len);
        for (i, byte) in packet.data.as_ref().iter().enumerate() {
            assert_eq!(*byte, u8::try_from((f * 31 + i) % 251).must());
        }
        assert!(!packet.discontinuity);
    }
    assert_eq!(
        demuxer.track_duration().must(),
        media_types::time::Duration::Known(media_types::time::Nanoseconds(149_333_333))
    );
}

#[test]
fn adts_and_adif_agree_on_the_same_payload() {
    // The same audio essence, two containers: uniform 100-byte blocks
    // of the ADIF ramp, wrapped once as ADTS frames (headers stripped
    // at comparison) and once as a CBR ADIF stream.
    let frames = 10;
    let frame_len = 100;
    let expected: Vec<u8> = (0..frames)
        .flat_map(|f| (0..frame_len).map(move |i| u8::try_from((f * 31 + i) % 251).must()))
        .collect();

    let mut adts = Vec::new();
    for f in 0..frames {
        let mut frame = adts_frame(1, 3, 2, 1, frame_len, None);
        frame[7..].copy_from_slice(&expected[f * frame_len..(f + 1) * frame_len]);
        adts.extend_from_slice(&frame);
    }
    let mut demuxer = open(adts);
    let out = drain(&mut demuxer);
    let mut stripped = Vec::new();
    for packet in &out.packets {
        stripped.extend_from_slice(&packet.data.as_ref()[7..]);
    }
    assert_eq!(stripped, expected, "ADTS strips to the essence");

    let adif = adif_stream(3, 2, frames, frame_len);
    let mut demuxer = open(adif);
    let out = drain(&mut demuxer);
    let mut plain = Vec::new();
    for packet in &out.packets {
        plain.extend_from_slice(packet.data.as_ref());
    }
    assert_eq!(plain, expected, "ADIF walks the same essence");
}

/// A minimal hand-built bit writer for the malformed ADIF fixtures.
struct Bits(Vec<bool>);
impl Bits {
    fn push(&mut self, value: u64, n: usize) {
        for i in (0..n).rev() {
            self.0.push(value >> i & 1 == 1);
        }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in self.0.chunks(8) {
            let mut byte = 0u8;
            for (i, bit) in chunk.iter().enumerate() {
                byte |= u8::from(*bit) << (7 - i);
            }
            out.push(byte);
        }
        out
    }
}

/// One ADIF header with configurable flavor + a data region.
fn adif_raw(vbr: bool, num_pce: u8, sampling_index: u8, front: u16, bitrate: u32) -> Vec<u8> {
    let mut bits = Bits(Vec::new());
    bits.push(u64::from(u32::from_be_bytes(*b"ADIF")), 32);
    bits.push(0, 1); // copyright_id_present
    bits.push(1, 1); // original_copy
    bits.push(0, 1); // home
    bits.push(u64::from(vbr), 1); // bitstream_type
    bits.push(u64::from(bitrate), 23);
    bits.push(u64::from(num_pce), 4);
    if vbr {
        bits.push(0, 20); // adif_buffer_fullness
    }
    for _ in 0..=num_pce {
        bits.push(0, 4); // element_instance_tag
        bits.push(1, 2); // profile
        bits.push(u64::from(sampling_index), 4);
        bits.push(u64::from(front), 4);
        bits.push(0, 4 + 4 + 2 + 3 + 4); // side/back/lfe/assoc/cc
        bits.push(0, 3); // no mixdowns
        for _ in 0..front {
            bits.push(1, 1); // front_element_is_cpe
            bits.push(0, 4); // tag
        }
        while !bits.0.len().is_multiple_of(8) {
            bits.push(0, 1);
        }
        bits.push(0, 8); // comment_field_bytes
    }
    let mut out = bits.bytes();
    out.extend_from_slice(&[0x55; 256]); // the raw data region
    out
}

#[test]
fn adif_vbr_is_the_typed_refusal() {
    let bytes = adif_raw(true, 0, 4, 1, 0);
    let mut demuxer = AdtsDemuxer::new(mem_stream(bytes).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.adif-vbr");
}

#[test]
fn adif_multi_program_is_the_typed_refusal() {
    let bytes = adif_raw(false, 1, 4, 1, 128_000);
    let mut demuxer = AdtsDemuxer::new(mem_stream(bytes).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.adif-multi-program");
}

#[test]
fn adif_bad_params_are_typed() {
    for (index, front, bitrate) in [(15u8, 1u16, 128_000u32), (4, 0, 128_000), (4, 1, 0)] {
        let bytes = adif_raw(false, 0, index, front, bitrate);
        let mut demuxer = AdtsDemuxer::new(mem_stream(bytes).must()).must();
        let err = demuxer.start().err().must();
        assert_eq!(err.context.code, "demux.adts.adif-bad-params");
    }
}

#[test]
fn adif_header_without_data_is_typed() {
    let bytes = adif_raw(false, 0, 4, 1, 128_000);
    let cut: Vec<u8> = bytes[..bytes.len() - 256].to_vec();
    let mut demuxer = AdtsDemuxer::new(mem_stream(cut).must()).must();
    let err = demuxer.start().err().must();
    assert_eq!(err.context.code, "demux.adts.adif-no-data");
}

#[test]
fn adif_partial_final_block_is_marked() {
    // A CBR stream whose data region is not block-aligned: the final
    // packet is the remainder, marked discontinuous.
    let mut bytes = adif_stream(3, 2, 3, 100);
    bytes.truncate(bytes.len() - 40);
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 3);
    assert_eq!(out.packets[2].data.len(), 60);
    assert!(out.packets[2].discontinuity);
    assert_eq!(out.discontinuities, 1);
    assert!(out.terminal.is_none());
}

#[test]
fn adif_seek_is_format_inherent_arithmetic() {
    use media_types::time::{TimeBase, Timestamp};
    let bytes = adif_stream(3, 2, 16, 100);
    let mut demuxer = open(bytes);
    assert_eq!(
        demuxer.seekability(),
        plugin_api::SeekSupport::Fast { indexed: false }
    );
    let base = TimeBase::new(1, 48_000).must();
    let target = media_types::time::TimeSpan {
        start: Stamp::Present(Timestamp::new(3 * 1024, base)),
        end: Stamp::Missing,
    };
    let landing = demuxer.seek(target).must();
    match landing.start {
        Stamp::Present(t) => assert_eq!(t.ticks, 3 * 1024),
        Stamp::Missing => unreachable!("landing is stamped"),
    }
    demuxer.reanchor(target).must();
    match demuxer.next_event().must() {
        DemuxEvent::Discontinuity => {}
        other => unreachable!("reanchor marks the splice: {other:?}"),
    }
    match demuxer.next_event().must() {
        DemuxEvent::Packet(p) => match p.pts {
            Stamp::Present(t) => assert_eq!(t.ticks, 3 * 1024),
            Stamp::Missing => unreachable!(),
        },
        other => unreachable!("reanchor lands on a packet: {other:?}"),
    }
}

// -- seek (ADTS) -------------------------------------------------------------

#[test]
fn adts_seek_roundtrip_lands_at_or_before_the_target() {
    use media_types::time::{TimeBase, Timestamp};
    let mut bytes = Vec::new();
    for _ in 0..16 {
        bytes.extend_from_slice(&adts_frame(1, 4, 2, 1, 100, None));
    }
    let mut demuxer = open(bytes);
    assert_eq!(demuxer.seekability(), plugin_api::SeekSupport::SlowLinear);
    let base = TimeBase::new(1, 44_100).must();
    let target = media_types::time::TimeSpan {
        start: Stamp::Present(Timestamp::new(5 * 1024 + 512, base)),
        end: Stamp::Missing,
    };
    let landing = demuxer.seek(target).must();
    match landing.start {
        Stamp::Present(t) => assert_eq!(t.ticks, 5 * 1024, "at-or-before"),
        Stamp::Missing => unreachable!("landing is stamped"),
    }
    demuxer.reanchor(target).must();
    match demuxer.next_event().must() {
        DemuxEvent::Discontinuity => {}
        other => unreachable!("reanchor marks the splice: {other:?}"),
    }
    match demuxer.next_event().must() {
        DemuxEvent::Packet(p) => match p.pts {
            Stamp::Present(t) => assert_eq!(t.ticks, 5 * 1024),
            Stamp::Missing => unreachable!(),
        },
        other => unreachable!("reanchor lands on a packet: {other:?}"),
    }
    let out = drain(&mut demuxer);
    assert_eq!(
        out.packets.len(),
        16 - 5 - 1,
        "remaining after the consumed one"
    );
}

#[test]
fn adts_seek_past_the_end_lands_at_the_last_frame() {
    // Ogg parity: at-or-before semantics clamp a past-the-end target
    // to the last frame (the ADIF lane is the one that refuses —
    // block arithmetic knows the exact block count).
    use media_types::time::{TimeBase, Timestamp};
    let bytes = adts_frame(1, 4, 2, 1, 100, None);
    let mut demuxer = open(bytes);
    let base = TimeBase::new(1, 44_100).must();
    let target = media_types::time::TimeSpan {
        start: Stamp::Present(Timestamp::new(1_000_000, base)),
        end: Stamp::Missing,
    };
    let landing = demuxer.seek(target).must();
    match landing.start {
        Stamp::Present(t) => assert_eq!(t.ticks, 0, "clamped to the only frame"),
        Stamp::Missing => unreachable!("landing is stamped"),
    }
}

// -- +FB fuzz-lite over the committed derivatives ---------------------------

#[test]
fn committed_mutation_fixtures_are_typed_or_recover_never_panics() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/adts");
    for name in [
        "0059-trunc-4k.bin",
        "0110-trunc-8k.bin",
        "0059-flip-idx15.bin",
        "0059-zero-range.bin",
    ] {
        let path = std::path::Path::new(dir).join(name);
        let bytes = std::fs::read(&path).must();
        assert!(!bytes.is_empty(), "{name} exists");
        super::fuzz::fuzz_demux_adts(&bytes);
    }
}
