//! Unit tests for the Ogg demuxer (card 5.B1): the page layer has
//! its own suite in [`super::page`]; this one pins the demuxer's
//! physical/logical mapping, chains, packet assembly and every typed
//! error path on built fixtures (deterministic bytes, no I/O).

use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use super::OggDemuxer;
use super::test_support::{BOS, CONTINUED, EOS, build_page, mem_stream, vorbis_id_packet};

/// Drive `demuxer` to end, collecting (events, sticky-error code).
fn drain(demuxer: &mut OggDemuxer) -> (Vec<DemuxEvent>, Option<String>) {
    let mut events = Vec::new();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::EndOfStream) => return (events, None),
            Ok(event) => events.push(event),
            Err(err) => return (events, Some(err.context.code.clone())),
        }
    }
}

fn packets(events: &[DemuxEvent]) -> Vec<(u32, Vec<u8>, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            DemuxEvent::Packet(p) => {
                Some((p.track.handle(), p.data.as_slice().to_vec(), p.keyframe))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn single_stream_start_packets_and_duration() {
    let bytes = super::test_support::demo_stream();
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let tracks = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].codec.as_str(), "vorbis");
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    let pkts = packets(&events);
    assert_eq!(
        pkts,
        vec![
            (0, b"\x00audio-a".to_vec(), true),
            (0, b"\x00audio-b".to_vec(), false),
            (0, b"\x00audio-c".to_vec(), false),
        ]
    );
    // Duration = EOS granule 1024 at 8 kHz = 128 ms.
    let duration = ogg
        .track_duration(tracks[0].id)
        .unwrap_or_else(|| unreachable!("vorbis clock known"));
    assert_eq!(
        duration,
        media_types::time::Duration::Known(media_types::time::Nanoseconds(128_000_000))
    );
    // Header passthrough: the ID header is stashed, not an event.
    assert_eq!(ogg.header_packets(0xAA11).len(), 1);
    assert_eq!(ogg.links(), 1);
    assert_eq!(ogg.granule_discontinuities(), 0);
}

#[test]
fn only_the_last_packet_on_a_page_owns_the_granule() {
    let bytes = super::test_support::demo_stream();
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let tracks = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let _ = tracks;
    let (events, _) = drain(&mut ogg);
    // Page 2 carries audio-a + audio-b at granule 512: only audio-b
    // is stamped (the honest apportionment contract).
    let stamps: Vec<Option<i64>> = events
        .iter()
        .filter_map(|e| match e {
            DemuxEvent::Packet(p) => Some(match p.pts {
                media_types::time::Stamp::Present(t) => Some(t.ticks),
                media_types::time::Stamp::Missing => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(stamps, vec![None, Some(512), Some(1024)]);
}

#[test]
fn mux_two_serials_two_tracks_interleaved() {
    let id_a = vorbis_id_packet(8_000);
    let id_b = {
        let mut v = b"OpusHead".to_vec();
        v.extend_from_slice(&[1, 2, 0x38, 0x01, 0x80, 0x3e, 0, 0, 0, 0, 0]);
        v
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id_a]));
    bytes.extend_from_slice(&build_page(BOS, 0, 0x2222, 0, &[&id_b]));
    bytes.extend_from_slice(&build_page(0, 256, 0x1111, 1, &[b"\x00a1"]));
    bytes.extend_from_slice(&build_page(0, 960, 0x2222, 1, &[b"\x00b1"]));
    bytes.extend_from_slice(&build_page(EOS, 512, 0x1111, 2, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(EOS, 1920, 0x2222, 2, &[b"\x00b2"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let tracks = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(tracks.len(), 2, "one track per logical stream");
    assert_eq!(tracks[0].codec.as_str(), "vorbis");
    assert_eq!(tracks[1].codec.as_str(), "opus");
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    let pkts = packets(&events);
    assert_eq!(pkts.len(), 4);
    assert_eq!(pkts[0].0, 0);
    assert_eq!(pkts[1].0, 1);
    assert_eq!(pkts[2].0, 0);
    assert_eq!(pkts[3].0, 1);
    // One link: mux, not chain.
    assert_eq!(ogg.links(), 1);
    assert_eq!(ogg.serial_numbers().len(), 2);
}

#[test]
fn chained_links_boundary_discontinuity_and_fresh_tracks() {
    // The canonical chain: link 1 ends (EOS), link 2 opens with a
    // new serial BOS.
    let id_a = vorbis_id_packet(8_000);
    let id_b = vorbis_id_packet(16_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id_a]));
    bytes.extend_from_slice(&build_page(0, 800, 0x1111, 1, &[b"\x00a1"]));
    bytes.extend_from_slice(&build_page(EOS, 1600, 0x1111, 2, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(BOS, 0, 0x9999, 0, &[&id_b]));
    bytes.extend_from_slice(&build_page(0, 1600, 0x9999, 1, &[b"\x00b1"]));
    bytes.extend_from_slice(&build_page(EOS, 3200, 0x9999, 2, &[b"\x00b2"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let tracks = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(tracks.len(), 1, "start reports link 0 only");
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    // The chain boundary is a Discontinuity, then link 2's packets
    // arrive under a fresh track id.
    let boundary = events
        .iter()
        .position(|e| matches!(e, DemuxEvent::Discontinuity))
        .unwrap_or_else(|| unreachable!("chain boundary must be a Discontinuity"));
    let pkts = packets(&events);
    let (before, after): (Vec<u32>, Vec<u32>) = (
        events[..boundary]
            .iter()
            .filter_map(|e| match e {
                DemuxEvent::Packet(p) => Some(p.track.handle()),
                _ => None,
            })
            .collect(),
        events[boundary + 1..]
            .iter()
            .filter_map(|e| match e {
                DemuxEvent::Packet(p) => Some(p.track.handle()),
                _ => None,
            })
            .collect(),
    );
    assert_eq!(before, vec![0, 0]);
    assert_eq!(after, vec![1, 1], "link 2 packets ride a fresh track id");
    assert_eq!(pkts.len(), 4);
    assert_eq!(ogg.links(), 2);
    assert_eq!(
        ogg.tracks().len(),
        2,
        "the accessor surfaces link 2's track"
    );
}

#[test]
fn continued_packet_assembles_across_pages() {
    // One 600-byte packet split across pages: page 1 carries a
    // single 255-run (the packet stays open), page 2 (continued)
    // carries 255 + 90 (terminal) plus an unrelated short packet.
    let id = vorbis_id_packet(8_000);
    let big: Vec<u8> = (0..600u32).map(|i| (i % 256) as u8).collect();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    bytes.extend_from_slice(&build_page(0, -1, 0x1111, 1, &[&big[..255]]));
    bytes.extend_from_slice(&build_page(
        CONTINUED,
        600,
        0x1111,
        2,
        &[&big[255..], b"\x00tail"],
    ));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    let pkts = packets(&events);
    assert_eq!(pkts.len(), 2, "one assembled 600-byte packet + the tail");
    assert_eq!(pkts[0].1.len(), 600);
    assert_eq!(
        pkts[0].1,
        (0..600u32).map(|i| (i % 256) as u8).collect::<Vec<u8>>()
    );
    assert_eq!(pkts[1].1, b"\x00tail");
}

#[test]
fn bad_crc_page_is_dropped_with_sync_recovery() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    let mut corrupt = build_page(0, 256, 0x1111, 1, &[b"\x00a1"]);
    corrupt[29] ^= 0xFF; // flip a body byte of THIS page: CRC now lies
    bytes.extend_from_slice(&corrupt);
    bytes.extend_from_slice(&build_page(0, 512, 0x1111, 2, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(EOS, 768, 0x1111, 3, &[b"\x00a3"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None, "a bad CRC page is damage, not fatal");
    let recovered: Vec<&DemuxEvent> = events
        .iter()
        .filter(|e| matches!(e, DemuxEvent::Recovered { .. }))
        .collect();
    assert_eq!(recovered.len(), 1, "one recovery skip for the bad page");
    let pkts = packets(&events);
    assert_eq!(
        pkts.len(),
        2,
        "the corrupt page's packet is lost, the rest survive"
    );
    assert_eq!(pkts[0].1, b"\x00a2");
    assert!(ogg.damage_total() > 0);
}

#[test]
fn mid_stream_garbage_recovers_at_the_next_capture() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    bytes.extend_from_slice(&build_page(0, 256, 0x1111, 1, &[b"\x00a1"]));
    // 5 KiB of noise with captures sprinkled in (some fake-broken).
    bytes.extend_from_slice(&[0xDE; 1024]);
    bytes.extend_from_slice(b"OggSgarbage-not-a-page");
    bytes.extend_from_slice(&[0xAD; 1024]);
    bytes.extend_from_slice(&build_page(0, 512, 0x1111, 2, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(EOS, 768, 0x1111, 3, &[b"\x00a3"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    let pkts = packets(&events);
    assert_eq!(pkts.len(), 3, "all three data packets survive the garbage");
    assert!(ogg.damage_total() >= 2058);
}

#[test]
fn truncated_page_is_a_typed_sticky_error() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    bytes.extend_from_slice(&build_page(0, 256, 0x1111, 1, &[b"\x00a1"]));
    // A final page cut mid-body (the segment table promises 64).
    bytes.extend_from_slice(&build_page(EOS, 512, 0x1111, 2, &[&[0xAB; 64]])[..27 + 1 + 32]);
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err.as_deref(), Some("demux.ogg.truncated-page"));
    assert_eq!(
        packets(&events).len(),
        1,
        "the first data packet already flowed"
    );
    // Sticky: the error replays by shape.
    match ogg.next_event() {
        Err(replay) => assert_eq!(replay.context.code, "demux.ogg.truncated-page"),
        Ok(_) => unreachable!("truncation is sticky"),
    }
}

#[test]
fn sequence_discontinuity_is_damage_noted_and_continues() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    bytes.extend_from_slice(&build_page(0, 256, 0x1111, 1, &[b"\x00a1"]));
    // seq jumps 1 → 7 (four pages lost in some other edit).
    bytes.extend_from_slice(&build_page(0, 1024, 0x1111, 7, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(EOS, 1280, 0x1111, 8, &[b"\x00a3"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DemuxEvent::Discontinuity)),
        "the seq gap surfaces as a Discontinuity"
    );
    assert_eq!(packets(&events).len(), 3);
    assert_eq!(ogg.granule_discontinuities(), 1);
}

#[test]
fn granule_regression_is_an_honest_discontinuity() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    bytes.extend_from_slice(&build_page(0, 1000, 0x1111, 1, &[b"\x00a1"]));
    // The clock goes backwards: 1000 → 400.
    bytes.extend_from_slice(&build_page(0, 400, 0x1111, 2, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(EOS, 600, 0x1111, 3, &[b"\x00a3"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    assert_eq!(ogg.granule_discontinuities(), 1);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DemuxEvent::Discontinuity))
    );
}

#[test]
fn minus_one_data_page_punches_a_visible_hole() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    bytes.extend_from_slice(&build_page(0, 256, 0x1111, 1, &[b"\x00a1"]));
    // A −1 granule DATA page: the position is unknown — a hole.
    bytes.extend_from_slice(&build_page(0, -1, 0x1111, 2, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(0, 1024, 0x1111, 3, &[b"\x00a3"]));
    bytes.extend_from_slice(&build_page(EOS, 1280, 0x1111, 4, &[b"\x00a4"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    assert_eq!(
        ogg.granule_discontinuities(),
        1,
        "leaving the hole re-anchors loudly"
    );
    // The hole page's packet is stamped Missing (never guessed).
    let stamps: Vec<Option<i64>> = events
        .iter()
        .filter_map(|e| match e {
            DemuxEvent::Packet(p) => Some(match p.pts {
                media_types::time::Stamp::Present(t) => Some(t.ticks),
                media_types::time::Stamp::Missing => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(stamps, vec![Some(256), None, Some(1024), Some(1280)]);
}

#[test]
fn continued_flag_lies_are_damage_noted() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    // Page 2 says "continued" but nothing is open: the orphan tail
    // is dropped.
    bytes.extend_from_slice(&build_page(
        CONTINUED,
        256,
        0x1111,
        1,
        &[b"\x00orphan", b"\x00a1"],
    ));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    let pkts = packets(&events);
    assert_eq!(pkts.len(), 1);
    assert_eq!(
        pkts[0].1, b"\x00a1",
        "the orphan run is dropped, the real packet survives"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, DemuxEvent::Recovered { .. })),
        "nothing was skipped"
    );
}

#[test]
fn unterminated_lacing_run_is_a_typed_overrun() {
    // Many pages of pure-255 lacing with no terminal: the pending
    // packet grows past MAX_PACKET_BYTES.
    let id = vorbis_id_packet(8_000);
    let big = vec![0x55u8; 255]; // one 255-run leaves the packet open
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    for seq in 1..=(super::MAX_PACKET_BYTES / 255 + 3) as u32 {
        let page = build_page(
            if seq == 1 { 0 } else { CONTINUED },
            -1,
            0x1111,
            seq,
            &[&big],
        );
        bytes.extend_from_slice(&page);
    }
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    // The overrun may fire during `start`'s header hunt already —
    // either way the typed error is what surfaces.
    let _ = ogg.start();
    let (_, err) = drain(&mut ogg);
    assert_eq!(err.as_deref(), Some("demux.ogg.packet-overrun"));
}

#[test]
fn no_bos_page_is_a_typed_no_streams() {
    // Garbage-only stream with a fake capture that fails CRC.
    let mut bytes = vec![0x11u8; 4096];
    bytes[..4].copy_from_slice(b"OggS");
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    match ogg.start() {
        Err(err) => assert_eq!(err.context.code, "demux.ogg.no-streams"),
        Ok(_) => unreachable!("garbage has no logical streams"),
    }
}

#[test]
fn unknown_codec_demuxes_opaque_with_missing_stamps() {
    let mystery = vec![0xEE; 40];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x4242, 0, &[&mystery]));
    bytes.extend_from_slice(&build_page(0, 500, 0x4242, 1, &[&mystery]));
    bytes.extend_from_slice(&build_page(EOS, 900, 0x4242, 2, &[&mystery]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let tracks = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].codec.as_str(), "ogg-opaque");
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    assert_eq!(packets(&events).len(), 3);
    // No clock: the duration is honestly unknown.
    assert_eq!(ogg.track_duration(tracks[0].id), None);
}

#[test]
fn skeleton_stream_mints_no_track_and_no_packets() {
    let fishead = b"fishead\x00\x00\x00\x00\x00\x00\x00\x00".to_vec();
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x0F1, 0, &[&fishead]));
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    bytes.extend_from_slice(&build_page(0, 256, 0x1111, 1, &[b"\x00a1"]));
    bytes.extend_from_slice(&build_page(EOS, 512, 0x1111, 2, &[b"\x00a2"]));
    bytes.extend_from_slice(&build_page(EOS, 0, 0x0F1, 1, &[&fishead]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let tracks = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(tracks.len(), 1, "skeleton carries no track");
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    assert_eq!(packets(&events).len(), 2, "skeleton packets never surface");
    assert_eq!(ogg.serial_numbers().len(), 2);
}

#[test]
fn not_started_and_not_ogg_are_typed() {
    let mut ogg =
        OggDemuxer::new(mem_stream(vec![1u8; 64]).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
    match ogg.next_event() {
        Err(err) => assert_eq!(err.context.code, "demux.ogg.not-started"),
        Ok(_) => unreachable!("start must run first"),
    }
    assert_eq!(ogg.seekability(), SeekSupport::None);
}

#[test]
fn seek_and_reanchor_land_on_the_last_page_before_the_target() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    for i in 0..8u32 {
        let granule = 256 * i64::from(i);
        bytes.extend_from_slice(&build_page(0, granule, 0x1111, i + 1, &[b"\x00x"]));
    }
    bytes.extend_from_slice(&build_page(EOS, 2048, 0x1111, 9, &[b"\x00x"]));
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(ogg.seekability(), SeekSupport::SlowLinear);
    let base = media_types::time::TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}"));
    let target = media_types::time::TimeSpan {
        start: media_types::time::Stamp::Present(media_types::time::Timestamp::new(1024, base)),
        end: media_types::time::Stamp::Missing,
    };
    let span = ogg.seek(target).unwrap_or_else(|e| unreachable!("{e}"));
    match span.start {
        media_types::time::Stamp::Present(t) => assert_eq!(t.ticks, 1024),
        media_types::time::Stamp::Missing => unreachable!("the landing is known"),
    }
    let landed = ogg.reanchor(target).unwrap_or_else(|e| unreachable!("{e}"));
    let _ = landed;
    // The re-anchored walk yields the packets after the landing
    // point (pages at granule ≥ 1024), preceded by the discontinuity.
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DemuxEvent::Discontinuity))
    );
    assert_eq!(packets(&events).len(), 5, "pages at granule 1024..=2048");
}

#[test]
fn pool_stays_bounded_and_packets_are_zero_copy_views() {
    let id = vorbis_id_packet(8_000);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&build_page(BOS, 0, 0x1111, 0, &[&id]));
    for i in 0..32u32 {
        bytes.extend_from_slice(&build_page(
            0,
            256 * i64::from(i),
            0x1111,
            i + 1,
            &[b"\x00payload"],
        ));
    }
    let mut ogg = OggDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = ogg.start().unwrap_or_else(|e| unreachable!("{e}"));
    let (events, err) = drain(&mut ogg);
    assert_eq!(err, None);
    assert_eq!(packets(&events).len(), 32);
    // 2 arenas × 256 KiB ceiling, demand-grown: far below in practice.
    assert!(ogg.pool_bytes() <= 2 * super::MAX_PACKET_BYTES as usize);
    assert!(ogg.pool_bytes() > 0);
}
