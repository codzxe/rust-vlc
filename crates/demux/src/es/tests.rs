//! In-crate tests for the raw ES demuxers (card 5.I3a): synthetic
//! framing truth, the typed error paths, and the fuzz-lite survival
//! sweep. The real-corpus differential against ffprobe lives in
//! `tests/es_corpus.rs` (it needs `input`'s file stack).

use plugin_api::{DemuxEvent, Demuxer};

use super::annexb::{AnnexbCodec, AnnexbVideoDemuxer};
use super::mp3::RawMp3Demuxer;

/// Boxed in-memory source (the crate's shared test seam).
fn mem(bytes: &[u8]) -> Box<dyn plugin_api::SourceStream> {
    let owned: std::sync::Arc<[u8]> = std::sync::Arc::from(bytes);
    match crate::avi::test_support::mem_stream(owned) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem stream never fails: {err:?}"),
    }
}

/// Push a 4-byte start code + `nal` (header byte(s) + payload).
fn push_nal(out: &mut Vec<u8>, nal: &[u8]) {
    out.extend_from_slice(&[0, 0, 0, 1]);
    out.extend_from_slice(nal);
}

/// An H.264 slice NAL: type 1/5 header + a payload whose first bit
/// encodes `first_mb_in_slice` (`ue(0)` is a lone 1-bit).
fn h264_slice(idr: bool, first_mb_zero: bool) -> Vec<u8> {
    let header = if idr { 0x65 } else { 0x41 };
    let first = if first_mb_zero { 0x80 } else { 0x00 };
    vec![header, first, 0x33, 0x77]
}

/// One drained walk: packets, event labels, terminal error.
type Walk = (
    Vec<media_types::Packet>,
    Vec<String>,
    Option<media_types::EngineError>,
);

/// Run a demuxer to its end, collecting packets (and events).
fn drain(demux: &mut dyn Demuxer) -> Result<Walk, media_types::EngineError> {
    let tracks = demux.start()?;
    let mut packets = Vec::new();
    let mut events = Vec::new();
    let mut err = None;
    for track in &tracks {
        events.push(format!(
            "track:{}:{}",
            track.id.handle(),
            track.codec.as_str()
        ));
    }
    loop {
        match demux.next_event() {
            Ok(DemuxEvent::Packet(p)) => packets.push(p),
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Recovered { skipped, reason }) => {
                events.push(format!("recovered:{skipped}:{reason}"));
            }
            Ok(DemuxEvent::Discontinuity) => events.push("discontinuity".to_string()),
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }
    Ok((packets, events, err))
}

#[test]
fn h264_aus_follow_the_nal_type_rule() {
    let mut stream = Vec::new();
    push_nal(&mut stream, &[0x67, 0x42]); // SPS (type 7)
    push_nal(&mut stream, &[0x68, 0xCE]); // PPS (type 8)
    push_nal(&mut stream, &[0x06, 0x01]); // SEI (type 6)
    push_nal(&mut stream, &h264_slice(true, true)); // IDR: AU 1
    push_nal(&mut stream, &h264_slice(false, true)); // P: AU 2
    push_nal(&mut stream, &h264_slice(false, false)); // 2nd slice, same picture
    push_nal(&mut stream, &h264_slice(false, true)); // P: AU 3
    push_nal(&mut stream, &[0x67, 0x42]); // mid SPS after VCL → AU 4
    push_nal(&mut stream, &[0x68, 0xCE]); // PPS (stays in AU 4)
    push_nal(&mut stream, &h264_slice(false, true)); // AU 4's slice
    let mut demux = AnnexbVideoDemuxer::new(AnnexbCodec::H264, mem(&stream))
        .unwrap_or_else(|e| unreachable!("{e:?}"));
    let (packets, events, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert!(err.is_none(), "{err:?}");
    assert_eq!(events.len(), 1, "one video track: {events:?}");
    assert_eq!(events[0], "track:0:h264");
    assert_eq!(packets.len(), 4, "IDR+P | P(2 slices) | SPS+PPS+P");
    // AU 1 = 4 start codes (16 B) + SPS 2 + PPS 2 + SEI 2 + slice 4.
    assert_eq!(packets[0].data.len(), 4 * 4 + 2 + 2 + 2 + 4);
    assert!(packets[0].keyframe, "AU 1 holds the IDR");
    assert!(!packets[1].keyframe);
    assert!(!packets[2].keyframe);
    assert!(!packets[3].keyframe, "AU 4 is a non-IDR slice");
    // Contiguous coverage: no bytes dropped between AUs.
    let total: usize = packets.iter().map(|p| p.data.len()).sum();
    assert_eq!(total, stream.len());
    assert_eq!(demux.aus_demuxed(), 4);
    // Timing honesty: raw Annex-B carries no stamps.
    assert_eq!(packets[0].pts, media_types::time::Stamp::Missing);
    assert_eq!(packets[0].duration, None);
}

#[test]
fn hevc_aus_group_on_first_slice_flag_and_vps() {
    let mut stream = Vec::new();
    // 2-byte HEVC headers: (type << 1) | 0, flags byte with the
    // first_slice_segment_in_pic_flag in bit 7.
    push_nal(&mut stream, &[0x40, 0x01]); // VPS (32)
    push_nal(&mut stream, &[0x42, 0x01]); // SPS (33)
    push_nal(&mut stream, &[0x44, 0x01]); // PPS (34)
    push_nal(&mut stream, &[0x4E, 0x80, 0x11]); // prefix SEI (39)
    push_nal(&mut stream, &[0x26, 0x80, 0x55]); // IRAP_W_RADL (19): AU 1
    push_nal(&mut stream, &[0x02, 0x80, 0x55]); // TRAIL_R (1): AU 2
    push_nal(&mut stream, &[0x02, 0x00, 0x55]); // 2nd segment, same picture
    push_nal(&mut stream, &[0x40, 0x01]); // mid VPS after VCL → AU 3
    push_nal(&mut stream, &[0x02, 0x80, 0x55]); // AU 3's slice
    let mut demux = AnnexbVideoDemuxer::new(AnnexbCodec::Hevc, mem(&stream))
        .unwrap_or_else(|e| unreachable!("{e:?}"));
    let (packets, events, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert!(err.is_none(), "{err:?}");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "track:0:hevc");
    assert_eq!(packets.len(), 3, "IRAP | TRAIL(2 segments) | VPS+TRAIL");
    assert!(packets[0].keyframe, "AU 1 is IRAP");
    assert!(!packets[1].keyframe);
    assert!(!packets[2].keyframe, "AU 3's slice is TRAIL_R → not IRAP");
    let total: usize = packets.iter().map(|p| p.data.len()).sum();
    assert_eq!(total, stream.len());
}

#[test]
fn h264_no_sync_is_typed_unsupported_after_scan() {
    let garbage = vec![0xA5u8; 512];
    let mut demux = AnnexbVideoDemuxer::new(AnnexbCodec::H264, mem(&garbage))
        .unwrap_or_else(|e| unreachable!("{e:?}"));
    let err = demux
        .start()
        .err()
        .unwrap_or_else(|| unreachable!("garbage must refuse"));
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::Unsupported { .. }
    ));
    assert_eq!(err.context.code, "demux.es.no-sync");
    // And it is sticky.
    let err2 = demux
        .start()
        .err()
        .unwrap_or_else(|| unreachable!("sticky refusal"));
    assert_eq!(err2.context.code, "demux.es.no-sync");
}

#[test]
fn empty_source_refuses_at_construction() {
    let err = AnnexbVideoDemuxer::new(AnnexbCodec::H264, mem(&[]))
        .err()
        .unwrap_or_else(|| unreachable!("empty must refuse"));
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::Unsupported { .. }
    ));
    let err = RawMp3Demuxer::new(mem(&[]))
        .err()
        .unwrap_or_else(|| unreachable!("empty must refuse"));
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::Unsupported { .. }
    ));
}

#[test]
fn next_event_before_start_is_typed() {
    let mut demux = AnnexbVideoDemuxer::new(AnnexbCodec::H264, mem(&[0, 0, 0, 1, 0x67, 0x42]))
        .unwrap_or_else(|e| unreachable!("{e:?}"));
    let err = demux
        .next_event()
        .err()
        .unwrap_or_else(|| unreachable!("contract order"));
    assert_eq!(err.context.code, "demux.es.not-started");
}

/// The canonical MPEG-1 L3 128 kbps 44.1 kHz stereo header (frame
/// size 417, 1152 samples) — the same fixture shape the codecs-side
/// mp3 module pins.
const MP3_HEAD: [u8; 4] = [0xFF, 0xFB, 0x90, 0x00];

fn mp3_frames(n: usize, payload: u8) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend_from_slice(&MP3_HEAD);
        out.resize(out.len() + 417 - 4, payload);
    }
    out
}

#[test]
fn mp3_walks_frames_with_exact_sample_clock() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&crate::adts::test_support::id3v2_tag(b"tagged"));
    bytes.extend_from_slice(&mp3_frames(3, 0x5A));
    let mut demux = RawMp3Demuxer::new(mem(&bytes)).unwrap_or_else(|e| unreachable!("{e:?}"));
    let (packets, _events, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert!(err.is_none(), "{err:?}");
    assert_eq!(packets.len(), 3);
    assert_eq!(
        demux.id3_skipped(),
        16,
        "10-byte tag header + 6-byte payload"
    );
    assert_eq!(demux.frames_demuxed(), 3);
    for (i, p) in packets.iter().enumerate() {
        assert_eq!(p.data.len(), 417);
        assert_eq!(p.data[0..4], MP3_HEAD);
        let pts = match p.pts {
            media_types::time::Stamp::Present(t) => t,
            other => unreachable!("mp3 pts are present: {other:?}"),
        };
        assert_eq!(pts.ticks, (i as i64) * 1152);
        assert!(p.keyframe);
    }
    let head = demux
        .head()
        .unwrap_or_else(|| unreachable!("start pinned it"));
    assert_eq!(head.sample_rate, 44_100);
    assert_eq!(head.samples_per_frame, 1152);
    assert_eq!(super::mp3::descriptor(head.layer).as_str(), "mp3");
}

#[test]
fn mp3_truncated_final_frame_is_typed_sticky() {
    let mut bytes = mp3_frames(2, 0x5A);
    bytes.truncate(bytes.len() - 100); // cut inside frame 2
    let mut demux = RawMp3Demuxer::new(mem(&bytes)).unwrap_or_else(|e| unreachable!("{e:?}"));
    let (packets, _events, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert_eq!(packets.len(), 1, "frame 1 emitted, frame 2 refused");
    let err = err.unwrap_or_else(|| unreachable!("the overrun must refuse typed"));
    assert_eq!(err.context.code, "demux.es.truncated");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::Truncated { .. }
        }
    ));
    // Sticky: the next call replays the same shape.
    let again = demux
        .next_event()
        .err()
        .unwrap_or_else(|| unreachable!("sticky"));
    assert_eq!(again.context.code, "demux.es.truncated");
}

#[test]
fn mp3_no_sync_is_typed_unsupported_after_scan() {
    let mut demux =
        RawMp3Demuxer::new(mem(&[0x11u8; 300])).unwrap_or_else(|e| unreachable!("{e:?}"));
    let err = demux
        .start()
        .err()
        .unwrap_or_else(|| unreachable!("garbage must refuse"));
    assert_eq!(err.context.code, "demux.es.no-sync");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::Unsupported { .. }
    ));
}

#[test]
fn mp3_midstream_garbage_resyncs_with_recovered_event() {
    let mut bytes = mp3_frames(1, 0x5A);
    bytes.extend_from_slice(&[0xEE; 7]); // garbage between frames
    bytes.extend_from_slice(&mp3_frames(1, 0x5A));
    let mut demux = RawMp3Demuxer::new(mem(&bytes)).unwrap_or_else(|e| unreachable!("{e:?}"));
    let (packets, events, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert!(err.is_none(), "{err:?}");
    assert_eq!(packets.len(), 2);
    assert!(events.iter().any(|e| e.starts_with("recovered:7:")));
}

#[test]
fn aac_lane_is_the_landed_adts_demuxer() {
    let bytes = crate::adts::test_support::demo_stream();
    let stream = match crate::avi::test_support::mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem stream never fails: {err:?}"),
    };
    let mut demux = super::aac::open(stream).unwrap_or_else(|e| unreachable!("{e:?}"));
    let (packets, _events, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert!(err.is_none(), "{err:?}");
    assert_eq!(packets.len(), 3, "the landed demo stream: 3 frames");
    assert_eq!(demux.frames_demuxed(), 3);
    assert!(super::aac::probe(&[0xFF, 0xF1]));
}

/// Fuzz-lite (`+FB`, doc 04 §8.5): a seeded LCG mutates a small
/// real-shaped stream (flips, truncations, wild headers, bursts);
/// every outcome must be Ok-or-typed — no panic, no hang. The mutated
/// corpus stays under 200 KB by construction.
#[test]
fn fuzz_lite_mutations_never_panic_or_hang() {
    let mut base = Vec::new();
    push_nal(&mut base, &[0x67, 0x42]);
    push_nal(&mut base, &[0x68, 0xCE]);
    push_nal(&mut base, &h264_slice(true, true));
    push_nal(&mut base, &h264_slice(false, true));
    base.extend_from_slice(&mp3_frames(3, 0x5A));
    let base_len = base.len();
    assert!(base_len < 200_000, "fuzz corpus stays small ({base_len})");

    let mut seed = 0x5EED_1A55_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for round in 0..512 {
        let mut bytes = base.clone();
        let mutations = (next() % 6) + 1;
        for _ in 0..mutations {
            match next() % 4 {
                0 => {
                    let at = (next() as usize) % bytes.len();
                    bytes[at] ^= 0xFF; // flip
                }
                1 => {
                    let at = (next() as usize) % bytes.len();
                    bytes[at] = u8::try_from(next() % 256).unwrap_or(0xA5); // wild byte
                }
                2 => {
                    bytes.truncate((next() as usize) % (bytes.len() + 1)); // truncation
                }
                _ => {
                    let at = (next() as usize) % bytes.len().max(1);
                    let burst = ((next() % 16) + 1) as usize;
                    for i in 0..burst {
                        if let Some(slot) = bytes.get_mut(at + i) {
                            *slot = 0x00; // zero burst (start-code fodder)
                        }
                    }
                }
            }
        }
        // The mutation must bite.
        assert!(round < 512);
        let mut h264 = match AnnexbVideoDemuxer::new(AnnexbCodec::H264, mem(&bytes)) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let _ = drain(&mut h264);
        let mut hevc = match AnnexbVideoDemuxer::new(AnnexbCodec::Hevc, mem(&bytes)) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let _ = drain(&mut hevc);
        let mut mp3 = match RawMp3Demuxer::new(mem(&bytes)) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let _ = drain(&mut mp3);
    }
}
