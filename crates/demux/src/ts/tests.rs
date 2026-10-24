//! Unit tests for the TS packet layer + PSI (card 5.I1a).

use media_types::PipelineStage;
use media_types::ids::MediaKind;
use plugin_api::{DemuxEvent, Demuxer};

use crate::ts::psi::{
    TID_CAT, TID_PAT, TID_PMT, TID_SDT, crc32_mpeg, dvb_text, parse_section, section_crc_ok,
};
use crate::ts::test_support::{
    mem_stream, pat_body, pmt_body, psi_packets, psi_section, psi_section_full, psi_section_tuned,
    sdt_body, single_program_ts, stream_of, ts_packet, ts_packet_tuned,
};
use crate::ts::{PACKET_BYTES, TsDemuxer, TsPacket, probe};

// ---- CRC-32/MPEG-2 ---------------------------------------------------------

#[test]
fn crc32_matches_the_corpus_shape() {
    // A whole section with its CRC appended hashes to zero — the
    // property the corpus sections were checked against before any
    // Rust was written.
    let section = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    assert!(section_crc_ok(&section));
    let mut broken = section.clone();
    let last = broken.len() - 1;
    broken[last] ^= 0x01;
    assert!(!section_crc_ok(&broken));
    // The known-check-value lane (poly 0x04C11DB7, init all-ones,
    // no reflection): 32 zero bytes → 0x4E08BFB4? — the corpus's own
    // section CRCs are the authority here; pin one exactly.
    assert_eq!(crc32_mpeg(&[]), 0xFFFF_FFFF);
}

// ---- PSI section parsing ---------------------------------------------------

#[test]
fn pat_parses_program_entries() {
    let section = psi_section(TID_PAT, 0x1234, 3, &pat_body(&[(1, 0x1000), (2, 0x1100)]));
    match parse_section(&section) {
        Ok(crate::ts::psi::PsiSection::Pat(pat)) => {
            assert_eq!(pat.transport_stream_id, 0x1234);
            assert_eq!(pat.entries.len(), 2);
            assert_eq!(pat.entries[0].program_number, 1);
            assert_eq!(pat.entries[0].pid, 0x1000);
            assert_eq!(pat.entries[1].program_number, 2);
            assert_eq!(pat.entries[1].pid, 0x1100);
        }
        other => unreachable!("PAT expected, got {other:?}"),
    }
}

#[test]
fn pmt_parses_pcr_and_es_loop() {
    let iso639 = [0x0A_u8, 0x04, b'e', b'n', b'g', 0x00];
    let section = psi_section(
        TID_PMT,
        7,
        0,
        &pmt_body(
            0x100,
            &[0x09, 0x00],
            &[(0x1B, 0x101, &[]), (0x0F, 0x102, &iso639)],
        ),
    );
    match parse_section(&section) {
        Ok(crate::ts::psi::PsiSection::Pmt(pmt)) => {
            assert_eq!(pmt.program_number, 7);
            assert_eq!(pmt.pcr_pid, 0x100);
            assert_eq!(pmt.program_info, vec![0x09, 0x00]);
            assert_eq!(pmt.streams.len(), 2);
            assert_eq!(pmt.streams[0].stream_type, 0x1B);
            assert_eq!(pmt.streams[0].pid, 0x101);
            assert_eq!(pmt.streams[1].stream_type, 0x0F);
            assert_eq!(pmt.streams[1].pid, 0x102);
            assert_eq!(pmt.streams[1].es_info, iso639.to_vec());
        }
        other => unreachable!("PMT expected, got {other:?}"),
    }
}

#[test]
fn sdt_parses_service_names_the_5_byte_loop_header() {
    // The regression the card warns about: the service-loop header
    // is 5 bytes (service_id + flags + descriptors_loop_length).
    // Reading 6 would eat the 0x48 tag and lose the name.
    let section = psi_section(
        TID_SDT,
        1,
        0,
        &sdt_body(
            0x1234,
            &[(1, "FFmpeg", "Service01"), (2, "Provider B", "Named")],
        ),
    );
    match parse_section(&section) {
        Ok(crate::ts::psi::PsiSection::Sdt(sdt)) => {
            assert_eq!(sdt.transport_stream_id, 1);
            assert_eq!(sdt.original_network_id, 0x1234);
            assert_eq!(sdt.services.len(), 2);
            assert_eq!(sdt.services[0].service_id, 1);
            assert_eq!(sdt.services[0].service_type, 0x01);
            assert_eq!(sdt.services[0].provider.as_deref(), Some("FFmpeg"));
            assert_eq!(sdt.services[0].name.as_deref(), Some("Service01"));
            assert_eq!(sdt.services[1].name.as_deref(), Some("Named"));
        }
        other => unreachable!("SDT expected, got {other:?}"),
    }
}

#[test]
fn cat_and_future_and_short_sections_skip() {
    // CAT: consumed, not parsed.
    let cat = psi_section(TID_CAT, 1, 0, &[0x09, 0x04, 0xFF, 0xFF, 0xFF, 0xFF]);
    match parse_section(&cat) {
        Ok(crate::ts::psi::PsiSection::Skipped { table_id, bytes }) => {
            assert_eq!(table_id, TID_CAT);
            assert_eq!(bytes, cat.len());
        }
        other => unreachable!("CAT skip expected, got {other:?}"),
    }
    // current_next_indicator == 0: a "future" version.
    let future = psi_section_tuned(TID_PAT, 1, 1, &pat_body(&[(1, 0x1000)]), false);
    match parse_section(&future) {
        Ok(crate::ts::psi::PsiSection::Skipped { .. }) => {}
        other => unreachable!("future skip expected, got {other:?}"),
    }
    // Short-form: no fixed header, no CRC, skipped by table id.
    let short = psi_section_full(TID_SDT, 0, 0, &[0x11, 0x22, 0x33, 0x44, 0x55], true, false);
    match parse_section(&short) {
        Ok(crate::ts::psi::PsiSection::Skipped { table_id, bytes }) => {
            assert_eq!(table_id, TID_SDT);
            assert_eq!(bytes, 8);
        }
        other => unreachable!("short-form skip expected, got {other:?}"),
    }
}

#[test]
fn crc_mismatch_is_typed() {
    let mut section = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    let last = section.len() - 1;
    section[last] ^= 0xFF;
    match parse_section(&section) {
        Err(err) => {
            assert_eq!(err.stage, PipelineStage::Demux);
            assert_eq!(err.context.code, "demux.ts.psi.crc-mismatch");
        }
        Ok(_) => unreachable!("CRC mismatch must fail typed"),
    }
}

#[test]
fn truncated_and_bad_structure_are_typed() {
    // The section declares more than it carries.
    let section = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    match parse_section(&section[..section.len() - 2]) {
        Err(err) => assert_eq!(err.context.code, "demux.ts.psi.truncated"),
        Ok(_) => unreachable!("truncation must fail typed"),
    }
    // Impossible arithmetic: a PAT body not a multiple of 4.
    let mut bad = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    let slen = 9 + 5;
    bad[1] = 0xB0 | (slen >> 8) as u8;
    bad[2] = (slen & 0xFF) as u8;
    let body_start = 8;
    let crc = crc32_mpeg(&bad[..body_start + 5]);
    bad.truncate(body_start + 5);
    bad.extend_from_slice(&crc.to_be_bytes());
    match parse_section(&bad) {
        Err(err) => assert_eq!(err.context.code, "demux.ts.psi.bad-structure"),
        Ok(_) => unreachable!("stride mismatch must fail typed"),
    }
    // Too small to be a section at all.
    match parse_section(&[0x00, 0xB0]) {
        Err(err) => assert_eq!(err.context.code, "demux.ts.psi.not-psi"),
        Ok(_) => unreachable!("stub must fail typed"),
    }
}

#[test]
fn dvb_text_prefixes_and_composition() {
    assert_eq!(dvb_text(b"Service01"), "Service01");
    assert_eq!(dvb_text(b""), "");
    // ISO 6937 selector + a composed acute-e.
    assert_eq!(dvb_text(&[0x05, 0xC1, 0x45]), "É");
    // UTF-8 selector.
    assert_eq!(dvb_text(&[0x15, 0xC3, 0x89]), "É");
    // ISO 10646 selector + 2 code bytes + UTF-16-BE.
    assert_eq!(dvb_text(&[0x10, 0x00, 0x0B, 0x00, 0xC9]), "É");
    // Latin-1 fallback for an unknown composition pair.
    assert_eq!(dvb_text(&[0xC1, 0x77]), "\u{c1}w");
}

// ---- packet layer ----------------------------------------------------------

#[test]
fn packet_fields_parse() {
    let payload = [0xAB_u8; 100];
    let bytes = ts_packet(0x1ABC, true, 9, Some((0x1_2345_6789, 0x1FF)), &payload);
    match TsPacket::parse(&bytes) {
        Ok(p) => {
            assert!(p.payload_unit_start);
            assert!(!p.transport_error);
            assert!(!p.transport_priority);
            assert_eq!(p.pid, 0x1ABC);
            assert_eq!(p.scrambling, 0);
            assert_eq!(p.adaptation_field_control, 0b11);
            assert_eq!(p.continuity_counter, 9);
            let af = p.adaptation.unwrap_or_else(|| unreachable!("AF expected"));
            let pcr = af.pcr.unwrap_or_else(|| unreachable!("PCR expected"));
            assert_eq!(pcr.base, 0x1_2345_6789);
            assert_eq!(pcr.extension, 0x1FF);
            // The payload runs to the packet's end (184 − the 8-byte
            // AF); the PES length itself is decode-domain.
            assert_eq!(p.payload.len(), 188 - 4 - 8);
            assert_eq!(p.payload[0], 0xAB);
            assert_eq!(p.payload[99], 0xAB);
        }
        Err(e) => unreachable!("parse failed: {e:?}"),
    }
}

#[test]
fn tei_and_scrambling_and_af_overrun() {
    let bytes = ts_packet_tuned(0x10, false, 0, None, &[0x00; 32], true, false);
    match TsPacket::parse(&bytes) {
        Ok(p) => assert!(p.transport_error),
        Err(e) => unreachable!("parse failed: {e:?}"),
    }
    // A scrambling-control value round-trips raw.
    let mut scrambled = ts_packet(0x20, false, 0, None, &[0x11; 40]);
    scrambled[3] |= 0xC0;
    match TsPacket::parse(&scrambled) {
        Ok(p) => assert_eq!(p.scrambling, 3),
        Err(e) => unreachable!("parse failed: {e:?}"),
    }
    // An adaptation field declaring more than the packet carries is
    // the typed bad-packet path.
    let mut overrun = ts_packet(0x30, false, 0, Some((0, 0)), &[0x00; 10]);
    overrun[4] = 200; // AF length 200 > 183
    match TsPacket::parse(&overrun) {
        Err(err) => assert_eq!(err.context.code, "demux.ts.bad-packet"),
        Ok(_) => unreachable!("AF overrun must fail typed"),
    }
}

#[test]
fn probe_requires_synced_stride() {
    assert!(probe(&[0x47]));
    let mut stream = single_program_ts();
    assert!(probe(&stream));
    stream[0] = 0x00;
    assert!(!probe(&stream));
}

// ---- the demuxer over a synthetic stream ----------------------------------

fn demux(bytes: &[u8]) -> TsDemuxer {
    match mem_stream(bytes.to_vec()) {
        Ok(stream) => match TsDemuxer::new(stream) {
            Ok(d) => d,
            Err(e) => unreachable!("construct failed: {e:?}"),
        },
        Err(e) => unreachable!("stream failed: {e:?}"),
    }
}

fn drain(d: &mut TsDemuxer) -> crate::ts::TsStats {
    loop {
        match d.next_event() {
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(_) => {}
            Err(e) => unreachable!("walk failed: {e:?}"),
        }
    }
    d.stats()
}

#[test]
fn single_program_surface() {
    let mut d = demux(&single_program_ts());
    let tracks = match d.start() {
        Ok(t) => t,
        Err(e) => unreachable!("start failed: {e:?}"),
    };
    // Two tracks (video h264 + audio aac), one program, SDT names.
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].kind, MediaKind::Video);
    assert_eq!(tracks[0].codec.as_str(), "h264");
    assert_eq!(tracks[1].kind, MediaKind::Audio);
    assert_eq!(tracks[1].codec.as_str(), "aac");
    let progs = d.programs();
    assert_eq!(progs.len(), 1);
    assert_eq!(progs[0].number, 1);
    assert_eq!(progs[0].pmt_pid, 0x1000);
    assert_eq!(progs[0].pcr_pid, Some(0x100));
    assert_eq!(progs[0].service_name.as_deref(), Some("Service01"));
    assert_eq!(progs[0].service_provider.as_deref(), Some("FFmpeg"));
    assert_eq!(progs[0].streams.len(), 2);
    assert_eq!(d.transport_stream_id(), Some(1));
    assert_eq!(d.original_network_id(), Some(1));
    assert_eq!(d.track_of_pid(0x100).map(|t| t.handle()), Some(0));
    assert_eq!(d.track_of_pid(0x101).map(|t| t.handle()), Some(1));
    // The PES seam sees the whole stream (rewind after the scan).
    let stats = drain(&mut d);
    assert_eq!(stats.pes_packets, 3);
    assert_eq!(stats.psi_sections, 3); // the replayed SDT+PAT+PMT
    assert_eq!(stats.cc_jumps, 0);
    assert!(stats.pes_bytes > 0);
}

#[test]
fn multi_program_surface() {
    let pat = psi_section(
        TID_PAT,
        9,
        0,
        &pat_body(&[(1, 0x1000), (2, 0x1100), (0, 0x0010)]),
    );
    let pmt1 = psi_section(TID_PMT, 1, 0, &pmt_body(0x100, &[], &[(0x1B, 0x100, &[])]));
    let pmt2 = psi_section(
        TID_PMT,
        2,
        0,
        &pmt_body(0x200, &[], &[(0x03, 0x200, &[]), (0x24, 0x201, &[])]),
    );
    let sdt = psi_section(
        0x42,
        9,
        0,
        &sdt_body(0x11, &[(1, "P1", "One"), (2, "P2", "Two")]),
    );
    let mut packets = Vec::new();
    packets.extend(psi_packets(0x0011, 0, &[sdt]));
    packets.extend(psi_packets(0x0000, 0, &[pat]));
    packets.extend(psi_packets(0x1000, 0, &[pmt1]));
    packets.extend(psi_packets(0x1100, 0, &[pmt2]));
    let mut d = demux(&stream_of(&packets));
    let tracks = match d.start() {
        Ok(t) => t,
        Err(e) => unreachable!("start failed: {e:?}"),
    };
    assert_eq!(tracks.len(), 3);
    let progs = d.programs();
    assert_eq!(progs.len(), 2); // the NIT pointer is not a program
    assert_eq!(progs[0].number, 1);
    assert_eq!(progs[0].service_name.as_deref(), Some("One"));
    assert_eq!(progs[1].number, 2);
    assert_eq!(progs[1].service_name.as_deref(), Some("Two"));
    assert_eq!(progs[1].streams[0].descriptor().as_str(), "mp3");
    assert_eq!(progs[1].streams[1].descriptor().as_str(), "hevc");
}

#[test]
fn sections_spanning_packets_and_multi_section_payloads() {
    // A PMT big enough to split across two packets (40 ES entries →
    // a 220-byte section): the first packet carries the pointer field
    // plus a full 183-byte payload of section bytes, the second the
    // remaining 37.
    let streams: Vec<(u8, u16, &[u8])> =
        (0..40u16).map(|i| (0x1B_u8, 0x200 + i, &[][..])).collect();
    let pmt = psi_section(TID_PMT, 1, 0, &pmt_body(0x100, &[], &streams));
    assert!(pmt.len() > 184);
    let mut first = vec![0x00_u8]; // pointer field
    first.extend_from_slice(&pmt[..183]);
    let second = pmt[183..].to_vec();
    let pat = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    let sdt = psi_section(0x42, 1, 0, &sdt_body(1, &[(1, "FFmpeg", "Service01")]));
    let mut packets = psi_packets(0x0011, 0, &[sdt]);
    packets.extend(psi_packets(0x0000, 0, &[pat]));
    packets.push(ts_packet(0x1000, true, 0, None, &first));
    packets.push(ts_packet(0x1000, false, 1, None, &second));
    let mut d = demux(&stream_of(&packets));
    match d.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 40),
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    assert_eq!(d.programs().len(), 1);
    assert_eq!(d.programs()[0].streams.len(), 40);
}

// ---- damage paths (each tested, all typed/no panics) -----------------------

#[test]
fn no_pat_is_typed() {
    // A synced TS with only PES-looking packets: no PAT anywhere.
    let packets = vec![
        ts_packet(0x0100, true, 0, None, &[0x00; 100]),
        ts_packet(0x0100, false, 1, None, &[0x00; 100]),
    ];
    let mut d = demux(&stream_of(&packets));
    match d.start() {
        Err(err) => assert_eq!(err.context.code, "demux.ts.no-pat"),
        Ok(_) => unreachable!("PAT-less stream must refuse"),
    }
    // Sticky: the refusal replays.
    match d.start() {
        Err(err) => assert_eq!(err.context.code, "demux.ts.no-pat"),
        Ok(_) => unreachable!("sticky refusal expected"),
    }
}

#[test]
fn not_ts_is_typed() {
    let mut d = demux(&[0x00_u8; 2048]);
    match d.start() {
        Err(err) => assert_eq!(err.context.code, "demux.ts.not-ts"),
        Ok(_) => unreachable!("garbage must refuse"),
    }
}

#[test]
fn bad_sync_resyncs_with_recovery_event() {
    // Three packets with garbage spliced before the third.
    let good = single_program_ts();
    let mut bytes = good[..2 * PACKET_BYTES].to_vec();
    bytes.extend_from_slice(&[0xFF; 57]);
    bytes.extend_from_slice(&good[2 * PACKET_BYTES..]);
    let mut d = demux(&bytes);
    match d.start() {
        Ok(t) => assert_eq!(t.len(), 2),
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let mut recovered = 0;
    let mut discontinuities = 0;
    loop {
        match d.next_event() {
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Recovered { skipped, .. }) => {
                assert_eq!(skipped, 57);
                recovered += 1;
            }
            Ok(DemuxEvent::Discontinuity) => discontinuities += 1,
            Ok(_) => {}
            Err(e) => unreachable!("walk failed: {e:?}"),
        }
    }
    assert_eq!(recovered, 1);
    // The resync landed on the PMT packet's true boundary, and the
    // replay restarted the continuity state — no spurious jump.
    assert_eq!(discontinuities, 0);
    assert_eq!(d.stats().sync_errors, 1);
    assert_eq!(d.stats().resync_bytes, 57);
}

#[test]
fn unresyncable_garbage_is_typed() {
    // A solid zero wall after the head: the bounded resync scan
    // refuses, typed.
    let mut bytes = single_program_ts();
    bytes.extend_from_slice(&[0x00; 4096]);
    let mut d = demux(&bytes);
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    match d.next_event() {
        Err(err) => assert_eq!(err.context.code, "demux.ts.resync-failed"),
        Ok(_) => unreachable!("unresyncable wall must refuse"),
    }
}

#[test]
fn truncated_psi_section_drops_and_continues() {
    // A PAT PUSI packet whose section_length declares more than the
    // stream ever delivers, followed by a complete valid PAT: the
    // partial is dropped (damage), the complete one applies.
    let partial = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    let mut cut = partial[..10].to_vec();
    cut[1] = 0xB0 | ((partial.len() - 3) >> 8) as u8;
    cut[2] = ((partial.len() - 3) & 0xFF) as u8;
    let good = psi_section(TID_PAT, 1, 1, &pat_body(&[(1, 0x1000)]));
    let pmt = psi_section(TID_PMT, 1, 0, &pmt_body(0x100, &[], &[(0x1B, 0x100, &[])]));
    let sdt = psi_section(0x42, 1, 0, &sdt_body(1, &[(1, "FFmpeg", "Service01")]));
    let mut packets = Vec::new();
    packets.push(ts_packet(0x0000, true, 0, None, &cut));
    packets.extend(psi_packets(0x0000, 1, &[good]));
    packets.extend(psi_packets(0x1000, 0, &[pmt]));
    packets.extend(psi_packets(0x0011, 0, &[sdt]));
    let mut d = demux(&stream_of(&packets));
    match d.start() {
        Ok(t) => assert_eq!(t.len(), 1),
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let stats = drain(&mut d);
    assert!(stats.psi_partial_drops >= 1);
    assert_eq!(d.programs().len(), 1);
}

#[test]
fn crc_damaged_psi_is_dropped_not_fatal() {
    let mut pat = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    let last = pat.len() - 1;
    pat[last] ^= 0x80;
    let good = psi_section(TID_PAT, 1, 1, &pat_body(&[(1, 0x1000)]));
    let pmt = psi_section(TID_PMT, 1, 0, &pmt_body(0x100, &[], &[(0x1B, 0x100, &[])]));
    let mut packets = psi_packets(0x0000, 0, &[pat, good]);
    packets.extend(psi_packets(0x1000, 0, &[pmt]));
    let mut d = demux(&stream_of(&packets));
    // The scan tolerates the CRC failure and takes the good PAT;
    // no SDT is fine (names stay None).
    match d.start() {
        Ok(t) => assert_eq!(t.len(), 1),
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let stats = drain(&mut d);
    assert!(stats.psi_crc_failures >= 1);
    assert_eq!(d.programs()[0].service_name, None);
}

#[test]
fn continuity_jump_and_duplicate_are_tolerant_and_counted() {
    let base = single_program_ts();
    // Splice a duplicate packet, then jump the counter by 2.
    let mut packets: Vec<Vec<u8>> = base.chunks(PACKET_BYTES).map(<[u8]>::to_vec).collect();
    let dup = packets[3].clone();
    packets.insert(4, dup);
    let jumped = ts_packet(0x100, false, 5, None, &[0x07; 100]); // expected 4
    packets.insert(6, jumped);
    let mut bytes = stream_of(&packets);
    let mut d = demux(&bytes);
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let stats = drain(&mut d);
    assert!(stats.cc_duplicates >= 1);
    assert!(stats.cc_jumps >= 1);
    bytes.clear();
}

#[test]
fn tei_packets_are_skipped_and_counted() {
    let mut packets: Vec<Vec<u8>> = single_program_ts()
        .chunks(PACKET_BYTES)
        .map(<[u8]>::to_vec)
        .collect();
    let mut tei = packets[4].clone();
    tei[1] |= 0x80;
    packets[4] = tei;
    let mut d = demux(&stream_of(&packets));
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let stats = drain(&mut d);
    assert_eq!(stats.tei_packets, 1);
}

#[test]
fn cat_sections_skip_with_a_note() {
    let cat = psi_section(TID_CAT, 1, 0, &[0x09, 0x04, 0xFF, 0xFF, 0xFF, 0xFF]);
    let pat = psi_section(TID_PAT, 1, 0, &pat_body(&[(1, 0x1000)]));
    let pmt = psi_section(TID_PMT, 1, 0, &pmt_body(0x100, &[], &[(0x1B, 0x100, &[])]));
    let sdt = psi_section(0x42, 1, 0, &sdt_body(1, &[(1, "FFmpeg", "Service01")]));
    let mut packets = Vec::new();
    packets.extend(psi_packets(0x0011, 0, &[sdt]));
    packets.extend(psi_packets(0x0001, 0, &[cat.clone(), cat]));
    packets.extend(psi_packets(0x0000, 0, &[pat]));
    packets.extend(psi_packets(0x1000, 0, &[pmt]));
    let mut d = demux(&stream_of(&packets));
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let stats = drain(&mut d);
    assert!(stats.cat_skips >= 2);
    assert!(
        d.damage_events()
            .iter()
            .any(|(_, reason)| *reason == "ts: CAT section skipped (conditional access)")
    );
}

#[test]
fn af_discontinuity_indicator_raises_the_event() {
    let base = single_program_ts();
    let mut packets: Vec<Vec<u8>> = base.chunks(PACKET_BYTES).map(<[u8]>::to_vec).collect();
    let flagged = ts_packet_tuned(0x100, true, 6, Some((90_000, 0)), &[0x00; 100], false, true);
    packets.push(flagged);
    let mut d = demux(&stream_of(&packets));
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let mut discontinuities = 0;
    loop {
        match d.next_event() {
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Discontinuity) => discontinuities += 1,
            Ok(_) => {}
            Err(e) => unreachable!("walk failed: {e:?}"),
        }
    }
    assert!(discontinuities >= 1);
}

#[test]
fn partial_tail_packet_is_a_note_not_an_error() {
    let mut bytes = single_program_ts();
    bytes.extend_from_slice(&[0x47, 0x41, 0x00, 0x10, 0xFF]); // 5-byte tail
    let mut d = demux(&bytes);
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let mut saw_note = false;
    loop {
        match d.next_event() {
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Recovered { skipped, .. }) => {
                if skipped == 5 {
                    saw_note = true;
                }
            }
            Ok(_) => {}
            Err(e) => unreachable!("walk failed: {e:?}"),
        }
    }
    assert!(saw_note);
}

#[test]
fn pes_seam_receives_payloads_and_errors_propagate() {
    use crate::ts::PesConsumer;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Collector {
        calls: Arc<AtomicU64>,
        bytes: Arc<AtomicU64>,
    }
    impl PesConsumer for Collector {
        fn pes_payload(
            &mut self,
            _pid: u16,
            _pusi: bool,
            payload: &[u8],
        ) -> Result<(), media_types::EngineError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.bytes
                .fetch_add(payload.len() as u64, Ordering::Relaxed);
            Ok(())
        }
    }
    let calls = Arc::new(AtomicU64::new(0));
    let bytes = Arc::new(AtomicU64::new(0));
    let mut d = demux(&single_program_ts());
    d.set_pes_consumer(Box::new(Collector {
        calls: Arc::clone(&calls),
        bytes: Arc::clone(&bytes),
    }));
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    let _ = drain(&mut d);
    assert_eq!(calls.load(Ordering::Relaxed), 3);
    assert!(bytes.load(Ordering::Relaxed) > 0);
}

#[test]
fn seek_refuses_typed() {
    let mut d = demux(&single_program_ts());
    match d.start() {
        Ok(_) => {}
        Err(e) => unreachable!("start failed: {e:?}"),
    }
    use plugin_api::SeekSupport;
    assert_eq!(d.seekability(), SeekSupport::None);
    match d.seek(media_types::time::TimeSpan::UNKNOWN) {
        Err(err) => assert_eq!(err.context.code, "demux.ts.seek.unsupported"),
        Ok(_) => unreachable!("seek must refuse"),
    }
}

#[test]
fn next_event_before_start_is_typed() {
    let mut d = demux(&single_program_ts());
    match d.next_event() {
        Err(err) => assert_eq!(err.context.code, "demux.ts.not-started"),
        Ok(_) => unreachable!("before-start must refuse"),
    }
}
