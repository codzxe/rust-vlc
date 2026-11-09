//! The ASF demuxer's tests (card 6.ASF-a).
//!
//! Three layers:
//!
//! 1. **Built-bytes unit tests** — the `test_support` builders drive
//!    every typed error path and the walk's structural rules
//!    (not-asf, truncation, object sizes, the broadcast gate,
//!    no-streams, WAVEFORMATEX / video-TSD validation, the
//!    non-audio/video stream skip, the declared-count mismatch note).
//! 2. **The fixture differential** — the plan's §1.2 eleven ffmpeg
//!    commands regenerated into the system temp dir (byte-verified
//!    this session against the plan's §4.4 sha256 table) and walked
//!    headless: object tree, track table, codec descriptors, geometry
//!    and WAVEFORMATEX facts, FPO facts — cross-checked against live
//!    ffprobe where ffprobe itself can read the file (stream count,
//!    codec names, rates, dimensions). **Skips loudly** when ffmpeg
//!    is absent (the `mkv_editions_tags` generated-lane precedent).
//!    The fixtures are ASF-d's to commit in-tree; regenerating here
//!    keeps this unit's assertions pinned to real muxer output
//!    without touching the shared fixture tree.
//! 3. **The ASF-b/c vectors** — synthesized Data Object packets
//!    (built by the local packet builder: EC flags, padding, send
//!    time, duration, multi-payload headers, replicated data,
//!    fragments spanning packets) drive the parser, the fragment
//!    assembler and the end-to-end walk; synthesized Simple Index
//!    Objects drive the indexed seek lane, the indexless bounded
//!    linear lane, and the too-slow budget refusal.

use std::process::Command;
use std::sync::Arc;

use media_types::ids::MediaKind;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{Demuxer, DemuxEvent, SeekSupport};

use super::data;
use super::error;
use super::guid;
use super::header::MAX_HEADER_BYTES;
use super::test_support::{
    asf_with_packets, mem_stream, minimal_asf_audio, waveformatex, SimpleIndexSpec,
};
use super::{AsfDemuxer, EXTENSIONS, probe};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => unreachable!("{e}"),
    }
}

fn started(bytes: Arc<[u8]>) -> AsfDemuxer {
    let mut asf = AsfDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let _ = asf.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    asf
}

fn code_of(err: &media_types::EngineError) -> &str {
    err.context.code.as_str()
}

// ---------------------------------------------------------------------------
// Layer 1: built bytes
// ---------------------------------------------------------------------------

const PCM16_WFX: &[u8] = &[
    0x01, 0x00, 0x01, 0x00, 0x11, 0x2b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x10, 0x00,
    0x00, 0x00,
];

/// Expect-Err without `unwrap_err` (the house no-unwrap rule).
fn err_of<T>(r: Result<T, media_types::EngineError>) -> media_types::EngineError {
    match r {
        Ok(_) => unreachable!("expected Err"),
        Err(e) => e,
    }
}

#[test]
fn probe_anchors_on_the_header_guid_only() {
    let mut header = Vec::new();
    header.extend_from_slice(guid::HEADER.as_bytes());
    assert!(probe(&header));
    // The broadcast `$H` prefix shape does not anchor (§2.5).
    let mut broadcast = vec![0x24, 0x48, 0x2f, 0x03, 0, 0, 0, 0, 0, 0x0c, 0x2f, 0x03];
    broadcast.extend_from_slice(guid::HEADER.as_bytes());
    assert!(!probe(&broadcast));
    assert!(!probe(b"RIFF....WAVE"));
    assert!(!probe(&header[..15]));
    assert_eq!(EXTENSIONS, &["asf", "wmv", "wma"]);
}

#[test]
fn minimal_audio_file_walks_to_one_pcm_track() {
    let mut asf = started(minimal_asf_audio(PCM16_WFX));
    let tracks = ok(asf.start());
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
    assert_eq!(tracks[0].kind, MediaKind::Audio);
    let header = asf.header().unwrap_or_else(|| unreachable!("header"));
    assert_eq!(header.declared_children, 2);
    assert_eq!(header.walked_children, 2);
    assert_eq!(header.properties.file_size, 280);
    assert_eq!(header.properties.data_packets_count, 1);
    assert!(!header.properties.is_broadcast());
    assert!(header.properties.declares_seekable());
    let record = &header.streams[0];
    assert_eq!(record.stream_number, 1);
    assert_eq!(record.flags_raw, 0);
    let wfx = record.wave_format.unwrap_or_else(|| unreachable!("wfx"));
    assert_eq!(wfx.tag, 1);
    assert_eq!(wfx.channels, 1);
    assert_eq!(wfx.samples_per_sec, 11_025);
    assert_eq!(wfx.bits_per_sample, 16);
    assert_eq!(wfx.cb_size, 0);
    // The verbatim TSD is the decoder-config seam.
    assert_eq!(&record.tsd_bytes[..], PCM16_WFX);
    // The Data Object was located, not parsed.
    let data = asf.data_object().unwrap_or_else(|| unreachable!("data"));
    assert_eq!(data.packet_count, 0);
    assert!(data.reserved_matches);
    // The PCM host fact (the wav-lane seam).
    assert_eq!(asf.pcm_fact(tracks[0].id), Some((1, 11_025)));
}

#[test]
fn not_asf_bytes_refuse_typed() {
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(&b"RIFF\x00\x00\x00\x00WAVEjunk"[..]))
            .unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.not-asf");
}

#[test]
fn truncated_head_is_truncation_not_not_asf() {
    // A Header GUID prefix cut inside the 30-byte head: the file
    // promised ASF (the wav recognized-magic rule, D-9).
    let mut cut = Vec::new();
    cut.extend_from_slice(guid::HEADER.as_bytes());
    cut.extend_from_slice(&[0x00, 0x00]);
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(cut.as_slice())).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.truncated");
}

#[test]
fn header_size_below_floor_and_overrun_refuse_bad_object_size() {
    // Build a header whose declared size (28) is below the 30-byte
    // floor by patching the built file.
    let bytes = minimal_asf_audio(PCM16_WFX);
    let mut patched = bytes.to_vec();
    patched[16..24].copy_from_slice(&28u64.to_le_bytes());
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(patched.as_slice())).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.bad-object-size");

    // And a header overrunning the stream.
    let bytes = minimal_asf_audio(PCM16_WFX);
    let mut patched = bytes.to_vec();
    patched[16..24].copy_from_slice(&9_999u64.to_le_bytes());
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(patched.as_slice())).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.truncated");
}

#[test]
fn header_above_memory_ceiling_refuses_typed() {
    let bytes = minimal_asf_audio(PCM16_WFX);
    let mut patched = bytes.to_vec();
    patched[16..24].copy_from_slice(&(MAX_HEADER_BYTES + 1).to_le_bytes());
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(patched.as_slice())).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.header-too-large");
}

#[test]
fn broadcast_flags_refuse_typed() {
    // Patch the built file's FPO flags to 0x3 (bit 0 = broadcast —
    // the §2.5 verified encoding of the asf_stream fixture family).
    let bytes = minimal_asf_audio(PCM16_WFX);
    let mut patched = bytes.to_vec();
    let fpo_flags_at = 30 + 24 + 64;
    patched[fpo_flags_at..fpo_flags_at + 4].copy_from_slice(&3u32.to_le_bytes());
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(patched.as_slice())).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.broadcast-unsupported");
}

#[test]
fn zero_channel_waveformatex_refuses_typed() {
    let mut wfx = PCM16_WFX.to_vec();
    wfx[2..4].copy_from_slice(&0u16.to_le_bytes()); // channels = 0
    let mut asf = AsfDemuxer::new(
        mem_stream(minimal_asf_audio(&wfx)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.bad-waveformatex");

    // cbSize overrunning the TSD is the same typed refusal.
    let mut wfx = PCM16_WFX.to_vec();
    wfx[16..18].copy_from_slice(&4u16.to_le_bytes()); // cbSize 4, no tail
    let mut asf = AsfDemuxer::new(
        mem_stream(minimal_asf_audio(&wfx)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.bad-waveformatex");
}

#[test]
fn unknown_waveformatex_tags_pass_through_raw() {
    // Tag 0x7100 (some future codec): the §4.5 map's honest
    // passthrough — the descriptor names the raw signal, the codec
    // layer refuses typed downstream.
    let wfx = waveformatex(0x7100, 2, 48_000, 4, 16);
    let mut asf = started(minimal_asf_audio(&wfx));
    let tracks = ok(asf.start());
    assert_eq!(tracks[0].codec.as_str(), "asf-audio-tag-7100");
    assert!(asf.pcm_fact(tracks[0].id).is_none());
}

#[test]
fn pcm_descriptor_maps_by_bits() {
    for (bits, name) in [
        (8u16, "pcm_u8"),
        (16, "pcm_s16le"),
        (24, "pcm_s24le"),
        (32, "pcm_s32le"),
    ] {
        let wfx = waveformatex(1, 1, 8_000, bits / 8, bits);
        let mut asf = started(minimal_asf_audio(&wfx));
        let tracks = ok(asf.start());
        assert_eq!(tracks[0].codec.as_str(), name, "{bits} bits");
    }
    // Integer PCM at a width the registry does not carry: raw
    // passthrough, never a silent nearest-guess.
    let wfx = waveformatex(1, 1, 8_000, 3, 24);
    let mut asf = started(minimal_asf_audio(&wfx));
    assert_eq!(ok(asf.start())[0].codec.as_str(), "pcm_s24le");
    let wfx = waveformatex(1, 1, 8_000, 6, 48);
    let mut asf = started(minimal_asf_audio(&wfx));
    assert_eq!(ok(asf.start())[0].codec.as_str(), "pcm_s48le");
}

#[test]
fn declared_child_count_mismatch_is_damage_not_failure() {
    // The size walk is authority; a lying count field is noted.
    let bytes = minimal_asf_audio(PCM16_WFX);
    let mut patched = bytes.to_vec();
    patched[24..28].copy_from_slice(&9u32.to_le_bytes()); // declared 9, walked 2
    let asf = started(Arc::from(patched.as_slice()));
    let header = asf.header().unwrap_or_else(|| unreachable!("header"));
    assert_eq!(header.declared_children, 9);
    assert_eq!(header.walked_children, 2);
    assert!(
        header
            .damage
            .events
            .iter()
            .any(|(_, r)| *r == "declared-child-count-mismatch")
    );
}

#[test]
fn non_audio_video_stream_types_skip_with_notes() {
    // Replace the audio stream's type GUID with the command-stream
    // GUID: the SP is skipped as a known non-AV family, and with no
    // AV streams left the header refuses no-streams.
    let bytes = minimal_asf_audio(PCM16_WFX);
    let mut patched = bytes.to_vec();
    let sp_payload_at = 30 + 104 + 24; // FPO child frame, then the SP object frame
    patched[sp_payload_at..sp_payload_at + 16]
        .copy_from_slice(guid::STREAM_TYPE_COMMAND.as_bytes());
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(patched.as_slice())).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.no-streams");
}

#[test]
fn unknown_header_children_skip_by_size() {
    // Splice an unknown-GUID child (48 bytes) between FPO and SP: the
    // walk must skip it by size and still find the stream.
    let bytes = minimal_asf_audio(PCM16_WFX).to_vec();
    let mut unknown = Vec::new();
    unknown.extend_from_slice(&[0xEE; 16]);
    unknown.extend_from_slice(&48u64.to_le_bytes());
    unknown.extend_from_slice(&[0u8; 24]);
    let mut spliced = Vec::new();
    let (fpo_child, rest) = bytes.split_at(30 + 104);
    spliced.extend_from_slice(fpo_child);
    spliced.extend_from_slice(&unknown);
    spliced.extend_from_slice(rest);
    // Fix the header size + declared count to include the splice.
    let header_size = 30 + 104 + 48 + (bytes.len() - 30 - 104 - 50);
    spliced[16..24].copy_from_slice(&(header_size as u64).to_le_bytes());
    spliced[24..28].copy_from_slice(&3u32.to_le_bytes());
    let asf = started(Arc::from(spliced));
    let header = asf.header().unwrap_or_else(|| unreachable!("header"));
    assert_eq!(header.walked_children, 3);
    assert_eq!(header.streams.len(), 1);
    assert_eq!(
        header.skipped_objects.first().map(|(g, _)| g.render()),
        Some("EEEEEEEE-EEEE-EEEE-EEEE-EEEEEEEEEEEE".to_string())
    );
    assert!(
        header
            .damage
            .events
            .iter()
            .any(|(_, r)| *r == "skipped-unknown-header-object")
    );
}

#[test]
fn trait_order_and_post_start_lanes_are_exact() {
    // Before start: the operational refusals.
    let mut asf = AsfDemuxer::new(
        mem_stream(minimal_asf_audio(PCM16_WFX)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    assert_eq!(code_of(&err_of(asf.next_event())), "demux.asf.not-started");
    assert_eq!(
        code_of(&err_of(asf.seek(TimeSpan::UNKNOWN))),
        "demux.asf.not-started"
    );
    assert_eq!(asf.seekability(), SeekSupport::None);

    let tracks = ok(asf.start());
    assert_eq!(tracks.len(), 1);
    // After start: the empty Data Object walks straight to the flush
    // endpoint (a real empty file — never a fake refusal shape) and
    // stays there across repeated drains.
    for _ in 0..3 {
        assert!(matches!(ok(asf.next_event()), DemuxEvent::EndOfStream));
    }
    // The indexless lane: SlowLinear (no Simple Index), and the
    // empty walk rewinds to zero honestly.
    assert_eq!(asf.seekability(), SeekSupport::SlowLinear);
    let landed = ok(asf.seek(TimeSpan::UNKNOWN));
    assert!(
        matches!(&landed.start, Stamp::Present(ts) if ts.ticks == 0),
        "{landed:?}"
    );
    // start is idempotent.
    assert_eq!(ok(asf.start()).len(), 1);
}

#[test]
fn error_constructors_table() {
    // The codes this unit mints (the wav error-table convention).
    let cases = [
        (error::not_asf(b"xx"), "demux.asf.not-asf"),
        (error::truncated(1, 2), "demux.asf.truncated"),
        (error::bad_object_size(3), "demux.asf.bad-object-size"),
        (error::header_too_large(1, 2), "demux.asf.header-too-large"),
        (error::no_streams(), "demux.asf.no-streams"),
        (
            error::broadcast_unsupported(),
            "demux.asf.broadcast-unsupported",
        ),
        (error::encrypted_stream(2), "demux.asf.encrypted-stream"),
        (error::bad_waveformatex(1), "demux.asf.bad-waveformatex"),
        (error::bad_video_info(1), "demux.asf.bad-video-info"),
        (error::not_started(), "demux.asf.not-started"),
        (
            error::packets_not_implemented(),
            "demux.asf.packets-not-implemented",
        ),
        (error::seek_unsupported(), "demux.asf.seek.unsupported"),
        (error::packet_bad_flags(9), "demux.asf.packet.bad-flags"),
        (error::packet_bad_length(9), "demux.asf.packet.bad-length"),
        (
            error::payload_bad_replication(9),
            "demux.asf.payload.bad-replication",
        ),
        (
            error::payload_oversized(1, 2),
            "demux.asf.payload.oversized",
        ),
        (
            error::packet_variable_size(),
            "demux.asf.packet.variable-size-unsupported",
        ),
        (error::seek_out_of_range(), "demux.asf.seek.out-of-range"),
        (error::seek_too_slow(), "demux.asf.seek.too-slow"),
        (error::index_too_large(1, 2), "demux.asf.index-too-large"),
    ];
    for (err, code) in cases {
        assert_eq!(code_of(&err), code);
        assert_eq!(err.stage, media_types::PipelineStage::Demux);
    }
}

// ---------------------------------------------------------------------------
// Layer 3: the ASF-b/c synthesized vectors
// ---------------------------------------------------------------------------

/// One synthesized payload spec (the ffmpeg-verified field set).
struct P {
    stream: u16,
    keyframe: bool,
    mo: u8,
    offset: u32,
    replicated: Option<(u32, u32)>,
    data: Vec<u8>,
}

/// Encode one payload header + data (the `5d` property-flags form:
/// replicated-length byte, offset dword, media-object byte, payload
/// length word).
fn payload_bytes(p: &P) -> Vec<u8> {
    let mut b = Vec::new();
    b.push(
        (if p.keyframe { 0x80 } else { 0x00 }) | u8::try_from(p.stream).unwrap_or(0x7F),
    );
    b.push(p.mo);
    b.extend_from_slice(&p.offset.to_le_bytes());
    match p.replicated {
        Some((size, pts)) => {
            b.push(8);
            b.extend_from_slice(&size.to_le_bytes());
            b.extend_from_slice(&pts.to_le_bytes());
        }
        None => b.push(0),
    }
    b.extend_from_slice(&(p.data.len() as u16).to_le_bytes());
    b.extend_from_slice(&p.data);
    b
}

/// Build one fixed-size data packet: audio-spread EC (82 + 2),
/// packet flags 0x11 (padding field present), property flags 0x5d,
/// padding length, send time, duration, payload count, payloads,
/// then the padding zeros — closing the packet exactly.
fn data_packet(send_ms: u32, duration_ms: u16, payloads: &[P], total: usize) -> Vec<u8> {
    let mut b = Vec::with_capacity(total);
    b.extend_from_slice(&[0x82, 0x00, 0x00]); // Error correction
    b.push(0x11); // Packet flags: type 01 + padding field present
    b.push(0x5d); // Property flags (the ffmpeg constant)
    b.extend_from_slice(&[0x00, 0x00]); // Padding length (patched)
    b.extend_from_slice(&send_ms.to_le_bytes());
    b.extend_from_slice(&duration_ms.to_le_bytes());
    b.push(0x80 | payloads.len() as u8); // Payload count (high bit)
    for p in payloads {
        b.extend_from_slice(&payload_bytes(p));
    }
    let padding = total - b.len();
    assert!(padding <= u16::MAX as usize, "packet overflow");
    b[5..7].copy_from_slice(&(padding as u16).to_le_bytes());
    b.extend(std::iter::repeat_n(0u8, padding));
    assert_eq!(b.len(), total);
    b
}

/// A completed-object payload spec: `data.len()` bytes, one object,
/// presentation time `pts_ms`.
fn object(stream: u16, keyframe: bool, mo: u8, pts_ms: u32, data: &[u8]) -> P {
    P {
        stream,
        keyframe,
        mo,
        offset: 0,
        replicated: Some((data.len() as u32, pts_ms)),
        data: data.to_vec(),
    }
}

fn ms_base() -> TimeBase {
    TimeBase::new(1, 1000).unwrap_or_else(|e| unreachable!("{e}"))
}

fn span_at(ms: i64) -> TimeSpan {
    TimeSpan {
        start: Stamp::Present(Timestamp::new(ms, ms_base())),
        end: Stamp::Missing,
    }
}

#[test]
fn packet_fields_parse_and_close_exactly() {
    let payload = object(1, true, 7, 1234, &[0xAA; 4]);
    let bytes = data_packet(555, 77, &[payload], 48);
    let parsed = match data::parse_packet(&bytes) {
        Ok(p) => p,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(parsed.send_time_ms, 555);
    assert_eq!(parsed.duration_ms, 77);
    assert_eq!(parsed.payload_count, 1);
    let payload = &parsed.payloads[0];
    assert_eq!(payload.stream, 1);
    assert!(payload.keyframe);
    assert_eq!(payload.media_object, 7);
    assert_eq!(payload.offset_into_object, 0);
    assert_eq!(payload.replicated, Some((4, 1234)));
    assert_eq!(payload.length, 4);
    assert_eq!(&bytes[payload.data_range.0..payload.data_range.1], &[0xAA; 4]);
    // The padding closed the packet: header + payload + padding.
    assert_eq!(parsed.padding, 48 - payload.data_range.1 as u32);
}

#[test]
fn multi_payload_packets_parse_in_order() {
    let first = object(1, true, 3, 10, &[0x01; 4]);
    let second = object(2, false, 9, 20, &[0x02; 6]);
    let bytes = data_packet(9, 30, &[first, second], 64);
    let parsed = match data::parse_packet(&bytes) {
        Ok(p) => p,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(parsed.payload_count, 2);
    assert_eq!(parsed.payloads[0].stream, 1);
    assert!(parsed.payloads[0].keyframe);
    assert_eq!(parsed.payloads[0].media_object, 3);
    assert_eq!(parsed.payloads[1].stream, 2);
    assert!(!parsed.payloads[1].keyframe);
    assert_eq!(parsed.payloads[1].media_object, 9);
    assert_eq!(&bytes[parsed.payloads[1].data_range.0..parsed.payloads[1].data_range.1], &[0x02; 6]);
}

#[test]
fn malformed_packets_refuse_typed_never_guess() {
    let good = data_packet(1, 2, &[object(1, true, 1, 3, &[0xAA; 4])], 48);
    // Packet-flags bits 1-0 != 01: an unverified shape.
    let mut bad = good.clone();
    bad[3] = 0x02;
    assert_eq!(
        code_of(&err_of(data::parse_packet(&bad))),
        "demux.asf.packet.bad-flags"
    );
    // Payload-count byte without the high bit.
    let mut bad = good.clone();
    bad[13] = 0x01;
    assert_eq!(
        code_of(&err_of(data::parse_packet(&bad))),
        "demux.asf.packet.bad-flags"
    );
    // A cut packet no longer closes: lengths refuse.
    assert_eq!(
        code_of(&err_of(data::parse_packet(&good[..47]))),
        "demux.asf.packet.bad-length"
    );
    // The compressed 1-byte replication form: no fixture exercises
    // it, refused rather than guessed.
    let mut bad = good.clone();
    bad[20] = 0x01; // The replicated-data length byte of payload 0.
    assert_eq!(
        code_of(&err_of(data::parse_packet(&bad))),
        "demux.asf.payload.bad-replication"
    );
}

#[test]
fn fragments_reassemble_into_whole_objects() {
    let mut assembler = data::ObjectAssembler::new(1);
    // Object 5 spans two packets: 4 bytes then 6 at offset 4.
    let first = match assembler.feed(0, 1, 5, 0, Some((10, 999)), true, &[0x11; 4]) {
        Ok(done) => done,
        Err(e) => unreachable!("{e}"),
    };
    assert!(first.is_none());
    let second = match assembler.feed(0, 1, 5, 4, None, true, &[0x22; 6]) {
        Ok(done) => done,
        Err(e) => unreachable!("{e}"),
    };
    let Some((pts, keyframe, buffer)) = second else {
        unreachable!("object must complete");
    };
    assert_eq!(pts, 999);
    assert!(keyframe);
    assert_eq!(buffer, [[0x11; 4].as_slice(), [0x22; 6].as_slice()].concat());

    // A new media-object number abandons the incomplete predecessor
    // (the container declares the gap by moving on).
    let mut assembler = data::ObjectAssembler::new(1);
    assert!(match assembler.feed(0, 1, 1, 0, Some((8, 5)), true, &[0x33; 2]) {
        Ok(done) => done.is_none(),
        Err(e) => unreachable!("{e}"),
    });
    assert!(match assembler.feed(0, 1, 2, 0, Some((2, 7)), false, &[0x44; 2]) {
        Ok(done) => done.is_none(),
        Err(e) => unreachable!("{e}"),
    });
    let done = match assembler.feed(0, 1, 2, 2, None, false, &[]) {
        Ok(done) => done,
        Err(e) => unreachable!("{e}"),
    };
    assert!(done.is_some());
}

#[test]
fn assembler_refusals_are_typed() {
    // Continuation landing at the wrong offset.
    let mut assembler = data::ObjectAssembler::new(1);
    assert!(matches!(assembler.feed(0, 1, 5, 0, Some((8, 1)), true, &[0x11; 4]), Ok(None)));
    assert_eq!(
        code_of(&err_of(assembler.feed(0, 1, 5, 3, None, true, &[0x22; 4]).map(|_| ()))),
        "demux.asf.packet.bad-length"
    );
    // A first fragment larger than the declared object.
    let mut assembler = data::ObjectAssembler::new(1);
    assert_eq!(
        code_of(&err_of(assembler.feed(0, 1, 9, 0, Some((2, 1)), true, &[0xAA; 3]).map(|_| ()))),
        "demux.asf.packet.bad-length"
    );
    // An object above the memory ceiling (MP §12): the size field is
    // container input, never a promise to allocate.
    let mut assembler = data::ObjectAssembler::new(1);
    assert_eq!(
        code_of(
            &err_of(assembler.feed(0, 1, 1, 0, Some((u32::MAX, 1)), true, &[0]).map(|_| ()))
        ),
        "demux.asf.payload.oversized"
    );
    // A first fragment without the 8-byte replication.
    let mut assembler = data::ObjectAssembler::new(1);
    assert_eq!(
        code_of(&err_of(assembler.feed(0, 1, 1, 0, None, true, &[0u8]).map(|_| ()))),
        "demux.asf.payload.bad-replication"
    );
}

#[test]
fn walk_emits_whole_objects_with_preroll_subtracted_stamps() {
    // A 1 kHz PCM track: sample ticks == milliseconds, so the stamps
    // read directly. Preroll 100 ms rides the payload clock.
    let wfx = waveformatex(1, 1, 1000, 2, 16);
    // Packet 1: object A completes (pts 150) + object B's first
    // fragment (4 of 10 bytes). Packet 2: B's continuation.
    let a = object(1, true, 1, 150, &[0xAA; 4]);
    let b1 = P {
        stream: 1,
        keyframe: false,
        mo: 2,
        offset: 0,
        replicated: Some((10, 250)),
        data: vec![0xBB; 4],
    };
    let b2 = P {
        stream: 1,
        keyframe: false,
        mo: 2,
        offset: 4,
        replicated: None,
        data: vec![0xCC; 6],
    };
    let packets = vec![
        data_packet(0, 20, &[a, b1], 64),
        data_packet(20, 20, &[b2], 64),
    ];
    let mut asf = started(asf_with_packets(&wfx, 100, 64, &packets, None));
    assert_eq!(ok(asf.start()).len(), 1);

    let first = match ok(asf.next_event()) {
        DemuxEvent::Packet(p) => p,
        other => unreachable!("{other:?}"),
    };
    assert_eq!(first.track.handle(), 0);
    assert!(first.keyframe);
    assert_eq!(&first.data[..], &[0xAA; 4]);
    assert!(matches!(&first.pts, Stamp::Present(ts) if ts.ticks == 50)); // 150 - preroll 100

    let second = match ok(asf.next_event()) {
        DemuxEvent::Packet(p) => p,
        other => unreachable!("{other:?}"),
    };
    assert!(!second.keyframe);
    assert_eq!(&second.data[..], &[[0xBB; 4].as_slice(), [0xCC; 6].as_slice()].concat());
    assert!(matches!(&second.pts, Stamp::Present(ts) if ts.ticks == 150)); // 250 - 100

    assert!(matches!(ok(asf.next_event()), DemuxEvent::EndOfStream));
}

#[test]
fn payloads_for_undeclared_streams_are_skipped_not_fatal() {
    let wfx = waveformatex(1, 1, 1000, 2, 16);
    let stray = object(9, true, 1, 10, &[0xAA; 4]); // Header declares stream 1 only.
    let packets = vec![data_packet(0, 20, &[stray], 48)];
    let mut asf = started(asf_with_packets(&wfx, 0, 48, &packets, None));
    assert_eq!(ok(asf.start()).len(), 1);
    assert!(matches!(ok(asf.next_event()), DemuxEvent::EndOfStream));
}

#[test]
fn non_fixed_packet_sizes_refuse_typed() {
    let wfx = waveformatex(1, 1, 1000, 2, 16);
    let packets = vec![data_packet(0, 20, &[object(1, true, 1, 0, &[0xAA; 4])], 48)];
    let bytes = asf_with_packets(&wfx, 0, 48, &packets, None);
    let mut patched = bytes.to_vec();
    // FPO min packet size: header (30) + FPO object header (24) +
    // payload offset 68 (16+8+8+8+8+8+8 file facts... see the FPO
    // field table in `header`'s module docs).
    patched[122..126].copy_from_slice(&111u32.to_le_bytes());
    let mut asf = AsfDemuxer::new(
        mem_stream(Arc::from(patched.as_slice())).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    assert_eq!(
        code_of(&err_of(asf.start())),
        "demux.asf.packet.variable-size-unsupported"
    );
}

#[test]
fn simple_index_seek_reanchors_the_walk() {
    let wfx = waveformatex(1, 1, 1000, 2, 16);
    // Four one-second-spaced objects; the index lands bucket 1 at
    // packet 2.
    let packets: Vec<Vec<u8>> = (0..4)
        .map(|i| data_packet(i * 1000, 1000, &[object(1, true, (i + 1) as u8, i * 1000, &[0xAA; 4])], 48))
        .collect();
    let index = SimpleIndexSpec {
        interval_100ns: 10_000_000, // 1 s (the ffmpeg constant)
        entries: vec![(0, 0), (2, 0)],
    };
    let mut asf = started(asf_with_packets(&wfx, 0, 48, &packets, Some(&index)));
    assert_eq!(ok(asf.start()).len(), 1);
    assert_eq!(asf.seekability(), SeekSupport::Fast { indexed: true });

    let landed = ok(asf.seek(span_at(1500)));
    assert!(matches!(&landed.start, Stamp::Present(ts) if ts.ticks == 1000));
    // The walk resumes at packet 2: pts 2000 then 3000, then EOS.
    for expected in [2000, 3000] {
        match ok(asf.next_event()) {
            DemuxEvent::Packet(p) => {
                assert!(matches!(&p.pts, Stamp::Present(ts) if ts.ticks == expected));
            }
            other => unreachable!("{other:?}"),
        }
    }
    assert!(matches!(ok(asf.next_event()), DemuxEvent::EndOfStream));

    // A target beyond the last bucket clamps to it (the walk's own
    // extent bounds us, never a fabricated position).
    let landed = ok(asf.seek(span_at(999_999)));
    assert!(matches!(&landed.start, Stamp::Present(ts) if ts.ticks == 1000));
    assert!(matches!(ok(asf.next_event()), DemuxEvent::Packet(p) if matches!(&p.pts, Stamp::Present(ts) if ts.ticks == 2000)));
}

#[test]
fn indexless_files_seek_by_bounded_linear_scan() {
    let wfx = waveformatex(1, 1, 1000, 2, 16);
    let packets: Vec<Vec<u8>> = (0..3)
        .map(|i| data_packet(i * 500, 500, &[object(1, true, (i + 1) as u8, i * 500, &[0xAA; 4])], 48))
        .collect();
    let mut asf = started(asf_with_packets(&wfx, 0, 48, &packets, None));
    assert_eq!(ok(asf.start()).len(), 1);
    assert_eq!(asf.seekability(), SeekSupport::SlowLinear);

    // 600 ms: the first packet whose send time reaches it is #2.
    let landed = ok(asf.seek(span_at(600)));
    assert!(matches!(&landed.start, Stamp::Present(ts) if ts.ticks == 1000));
    assert!(matches!(ok(asf.next_event()), DemuxEvent::Packet(p) if matches!(&p.pts, Stamp::Present(ts) if ts.ticks == 1000)));
    assert!(matches!(ok(asf.next_event()), DemuxEvent::EndOfStream));

    // Rewind to zero lands at packet 0.
    let landed = ok(asf.seek(span_at(0)));
    assert!(matches!(&landed.start, Stamp::Present(ts) if ts.ticks == 0));
    assert!(matches!(ok(asf.next_event()), DemuxEvent::Packet(p) if matches!(&p.pts, Stamp::Present(ts) if ts.ticks == 0)));
}

#[test]
fn indexless_seeks_past_the_scan_budget_refuse_typed() {
    let wfx = waveformatex(1, 1, 1000, 2, 16);
    // 5000 minimal packets: past the 4096-packet linear budget.
    let packets: Vec<Vec<u8>> = (0..5000)
        .map(|i| data_packet(i * 10, 10, &[object(1, true, (i % 255 + 1) as u8, i * 10, &[0xAA; 4])], 48))
        .collect();
    let mut asf = started(asf_with_packets(&wfx, 0, 48, &packets, None));
    assert_eq!(ok(asf.start()).len(), 1);
    assert_eq!(
        code_of(&err_of(asf.seek(span_at(5)))),
        "demux.asf.seek.too-slow"
    );
}

// ---------------------------------------------------------------------------
// Layer 2: the ffmpeg fixture differential (regenerated, not committed)
// ---------------------------------------------------------------------------

/// One fixture's expected walk (every number measured this session
/// against the sha256-pinned files — the plan's §4.4 table verified
/// byte-exact on regeneration).
struct FixtureTruth {
    file: &'static str,
    /// The §1.2 generation command (after `-y -v error`).
    generate: &'static str,
    /// Header size.
    header_size: u64,
    /// Declared == walked children.
    children: u32,
    /// (kind, codec, stream_number).
    tracks: &'static [(MediaKind, &'static str, u16)],
    /// FPO: (file_size, data_packets_count, preroll_ms, min/max packet).
    fpo: (u64, u64, u64, u32),
    /// Data Object: (offset, size, packet_count).
    data: (u64, u64, u64),
    /// Trailing Simple Index: (offset, size) — None for audio-only.
    simple_index: Option<(u64, u64)>,
    /// Audio WFX: (tag, channels, rate, align, bits, cb_size).
    wfx: Option<(u16, u16, u32, u16, u16, u16)>,
    /// Video: (width, height, fourcc, bit_count, size_image, extradata).
    video: Option<(u32, u32, &'static str, u16, u32, usize)>,
    /// ffprobe codec names per stream (the cross-check oracle).
    ffprobe_codecs: &'static [&'static str],
}

const FIXTURES: &[FixtureTruth] = &[
    FixtureTruth {
        file: "wma_wmav2_mono8k.wma",
        generate: "-f lavfi -i sine=frequency=440:duration=1:sample_rate=8000 -ac 1 -c:a wmav2 -b:a 32k",
        header_size: 492,
        children: 5,
        tracks: &[(MediaKind::Audio, "wmav2", 1)],
        fpo: (6_942, 2, 3_100, 3_200),
        data: (492, 6_450, 2),
        simple_index: None,
        wfx: Some((0x0161, 1, 8_000, 256, 16, 10)),
        video: None,
        ffprobe_codecs: &["wmav2"],
    },
    FixtureTruth {
        file: "wma_wmav1_st44k.wma",
        generate: "-f lavfi -i sine=frequency=440:duration=2:sample_rate=44100 -ac 2 -c:a wmav1 -b:a 32k",
        header_size: 452,
        children: 5,
        tracks: &[(MediaKind::Audio, "wmav1", 1)],
        fpo: (10_102, 3, 3_100, 3_200),
        data: (452, 9_650, 3),
        simple_index: None,
        wfx: Some((0x0160, 2, 44_100, 185, 16, 4)),
        video: None,
        ffprobe_codecs: &["wmav1"],
    },
    FixtureTruth {
        file: "wmv_wmv2_64x48.wmv",
        generate: "-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v wmv2 -b:v 100k",
        header_size: 587,
        children: 5,
        tracks: &[(MediaKind::Video, "wmv2", 1)],
        fpo: (7_123, 2, 3_100, 3_200),
        data: (587, 6_450, 2),
        simple_index: Some((7_037, 86)),
        wfx: None,
        video: Some((64, 48, "WMV2", 24, 9_216, 4)),
        ffprobe_codecs: &["wmv2"],
    },
    FixtureTruth {
        file: "wmv_wmv1_64x48.wmv",
        generate: "-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v wmv1 -b:v 100k",
        header_size: 583,
        children: 5,
        tracks: &[(MediaKind::Video, "wmv1", 1)],
        fpo: (7_119, 2, 3_100, 3_200),
        data: (583, 6_450, 2),
        simple_index: Some((7_033, 86)),
        wfx: None,
        video: Some((64, 48, "WMV1", 24, 9_216, 0)),
        ffprobe_codecs: &["wmv1"],
    },
    FixtureTruth {
        file: "asf_msmpeg4v3.asf",
        generate: "-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v msmpeg4v3 -b:v 100k",
        header_size: 593,
        children: 5,
        tracks: &[(MediaKind::Video, "msmpeg4v3", 1)],
        fpo: (7_129, 2, 3_100, 3_200),
        data: (593, 6_450, 2),
        simple_index: Some((7_043, 86)),
        wfx: None,
        video: Some((64, 48, "MP43", 24, 9_216, 0)),
        ffprobe_codecs: &["msmpeg4v3"],
    },
    FixtureTruth {
        file: "asf_mjpeg.asf",
        generate: "-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v mjpeg -b:v 200k",
        header_size: 585,
        children: 5,
        tracks: &[(MediaKind::Video, "mjpeg", 1)],
        fpo: (26_321, 8, 3_100, 3_200),
        data: (585, 25_650, 8),
        simple_index: Some((26_235, 86)),
        wfx: None,
        video: Some((64, 48, "MJPG", 24, 9_216, 0)),
        ffprobe_codecs: &["mjpeg"],
    },
    FixtureTruth {
        file: "wmv_wmv2_wma_av.wmv",
        generate: "-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -f lavfi -i sine=frequency=440:duration=1:sample_rate=8000 -ac 1 -c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k",
        header_size: 757,
        children: 6,
        tracks: &[
            (MediaKind::Video, "wmv2", 1),
            (MediaKind::Audio, "wmav2", 2),
        ],
        fpo: (10_499, 3, 3_100, 3_200),
        data: (757, 9_650, 3),
        simple_index: Some((10_407, 92)),
        wfx: Some((0x0161, 1, 8_000, 256, 16, 10)),
        video: Some((64, 48, "WMV2", 24, 9_216, 4)),
        ffprobe_codecs: &["wmv2", "wmav2"],
    },
    FixtureTruth {
        file: "asf_pkt8000.wmv",
        generate: "-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -f lavfi -i sine=frequency=440:duration=1:sample_rate=8000 -ac 1 -c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k -packet_size 8000",
        header_size: 757,
        children: 6,
        tracks: &[
            (MediaKind::Video, "wmv2", 1),
            (MediaKind::Audio, "wmav2", 2),
        ],
        fpo: (16_899, 2, 3_100, 8_000),
        data: (757, 16_050, 2),
        simple_index: Some((16_807, 92)),
        wfx: Some((0x0161, 1, 8_000, 256, 16, 10)),
        video: Some((64, 48, "WMV2", 24, 9_216, 4)),
        ffprobe_codecs: &["wmv2", "wmav2"],
    },
    FixtureTruth {
        file: "asf_pcm_s16le.wma",
        generate: "-f lavfi -i sine=frequency=440:duration=1:sample_rate=8000 -ac 1 -c:a pcm_s16le",
        header_size: 456,
        children: 5,
        tracks: &[(MediaKind::Audio, "pcm_s16le", 1)],
        fpo: (26_106, 8, 3_100, 3_200),
        data: (456, 25_650, 8),
        simple_index: None,
        wfx: Some((0x0001, 1, 8_000, 2, 16, 0)),
        video: None,
        ffprobe_codecs: &["pcm_s16le"],
    },
    FixtureTruth {
        file: "asf_adpcm_ms.wma",
        generate: "-f lavfi -i sine=frequency=440:duration=1:sample_rate=8000 -ac 1 -c:a adpcm_ms -b:a 32k",
        header_size: 486,
        children: 5,
        tracks: &[(MediaKind::Audio, "adpcm_ms", 1)],
        fpo: (6_936, 2, 3_100, 3_200),
        data: (486, 6_450, 2),
        simple_index: None,
        wfx: Some((0x0002, 1, 8_000, 1_024, 4, 32)),
        video: None,
        ffprobe_codecs: &["adpcm_ms"],
    },
];

/// Regenerate (once per session) the §1.2 fixture family into the
/// system temp dir; `None` when ffmpeg is absent (loud skip).
fn fixture_dir() -> Option<std::path::PathBuf> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("SKIP: ffmpeg absent (asf fixture differential)");
        return None;
    }
    let dir = std::env::temp_dir().join("lumen-asf-a-fixtures");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

fn generate(dir: &std::path::Path, truth: &FixtureTruth) -> std::path::PathBuf {
    let path = dir.join(truth.file);
    if path.exists() {
        return path;
    }
    let args: Vec<&str> = truth.generate.split(' ').collect();
    let out = Command::new("ffmpeg")
        .args(["-y", "-v", "error"])
        .args(&args)
        .arg(&path)
        .output()
        .unwrap_or_else(|e| unreachable!("ffmpeg spawn: {e}"));
    assert!(
        out.status.success(),
        "ffmpeg generate {}: {}",
        truth.file,
        String::from_utf8_lossy(&out.stderr)
    );
    path
}

fn read_file(path: &std::path::Path) -> Arc<[u8]> {
    let bytes =
        std::fs::read(path).unwrap_or_else(|e| unreachable!("read {}: {e}", path.display()));
    Arc::from(bytes)
}

#[test]
fn fixture_family_walks_match_the_measured_truth() {
    let Some(dir) = fixture_dir() else {
        return;
    };
    for truth in FIXTURES {
        let path = generate(&dir, truth);
        let bytes = read_file(&path);
        // The §4.4 sha256 pins (first 16 hex) — regeneration is
        // byte-exact or the differential is meaningless.
        let sha = sha256_16(&bytes);
        assert_eq!(sha, expected_sha(truth.file), "{}", truth.file);

        let mut asf = AsfDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{}: new: {e}", truth.file));
        let tracks = match asf.start() {
            Ok(t) => t,
            Err(e) => unreachable!("{}: start: {e}", truth.file),
        };
        let header = asf.header().unwrap_or_else(|| unreachable!("header"));

        // Object tree.
        assert_eq!(header.header_size, truth.header_size, "{}", truth.file);
        assert_eq!(header.declared_children, truth.children, "{}", truth.file);
        assert_eq!(
            header.walked_children as u32, truth.children,
            "{}",
            truth.file
        );
        let data = asf.data_object().unwrap_or_else(|| unreachable!("data"));
        assert_eq!(
            (data.offset, data.size, data.packet_count),
            truth.data,
            "{}",
            truth.file
        );
        assert!(data.reserved_matches, "{}", truth.file);
        match truth.simple_index {
            Some((offset, size)) => assert!(
                header
                    .top_objects
                    .iter()
                    .any(|o| o.guid == guid::SIMPLE_INDEX && o.offset == offset && o.size == size),
                "{}: simple index {offset}/{size} not mapped: {:?}",
                truth.file,
                header.top_objects
            ),
            None => assert!(
                !header
                    .top_objects
                    .iter()
                    .any(|o| o.guid == guid::SIMPLE_INDEX),
                "{}: audio-only file must carry no Simple Index",
                truth.file
            ),
        }

        // Track table.
        assert_eq!(tracks.len(), truth.tracks.len(), "{}", truth.file);
        for (index, (kind, codec, number)) in truth.tracks.iter().enumerate() {
            assert_eq!(tracks[index].kind, *kind, "{}", truth.file);
            assert_eq!(tracks[index].codec.as_str(), *codec, "{}", truth.file);
            assert_eq!(
                header.streams[index].stream_number, *number,
                "{}",
                truth.file
            );
        }

        // FPO facts (the flagged decision: durations advisory, never
        // track timing — asserted as raw facts only).
        let p = header.properties;
        assert_eq!(p.file_size, truth.fpo.0, "{}", truth.file);
        assert_eq!(p.data_packets_count, truth.fpo.1, "{}", truth.file);
        assert_eq!(p.preroll_ms, truth.fpo.2, "{}", truth.file);
        assert_eq!(p.min_packet_size, truth.fpo.3, "{}", truth.file);
        assert_eq!(p.max_packet_size, truth.fpo.3, "{}", truth.file);
        assert!(!p.is_broadcast(), "{}", truth.file);
        assert!(p.declares_seekable(), "{}", truth.file);

        // Codec facts.
        if let Some((tag, channels, rate, align, bits, cb)) = truth.wfx {
            let wfx = header
                .streams
                .iter()
                .find_map(|s| s.wave_format)
                .unwrap_or_else(|| unreachable!("{}: wfx", truth.file));
            assert_eq!(wfx.tag, tag, "{}", truth.file);
            assert_eq!(wfx.channels, channels, "{}", truth.file);
            assert_eq!(wfx.samples_per_sec, rate, "{}", truth.file);
            assert_eq!(wfx.block_align, align, "{}", truth.file);
            assert_eq!(wfx.bits_per_sample, bits, "{}", truth.file);
            assert_eq!(wfx.cb_size, cb, "{}", truth.file);
            // The verbatim TSD blob is the ADPCM decoder-config seam
            // (18 + cb_size bytes of WAVEFORMATEX) — on the SAME
            // audio record the wfx came from (A/V files carry the
            // video SP first: streams[0] is VIDEO there, measured
            // wmv_wmv2_wma_av: streams[0] tsd 55 = VIDEOINFOHEADER).
            let record = header
                .streams
                .iter()
                .find(|s| s.wave_format.is_some())
                .unwrap_or_else(|| unreachable!("{}: audio stream", truth.file));
            assert_eq!(record.tsd_bytes.len(), 18 + cb as usize, "{}", truth.file);
        }
        if let Some((width, height, fourcc, bit_count, size_image, extradata)) = truth.video {
            let video = header
                .streams
                .iter()
                .find_map(|s| s.video_info)
                .unwrap_or_else(|| unreachable!("{}: video", truth.file));
            assert_eq!(video.width, width, "{}", truth.file);
            assert_eq!(video.height, height, "{}", truth.file);
            assert_eq!(video.fourcc(), fourcc, "{}", truth.file);
            assert_eq!(video.bit_count, bit_count, "{}", truth.file);
            assert_eq!(video.size_image, size_image, "{}", truth.file);
            assert_eq!(video.extradata_len, extradata, "{}", truth.file);
        }

        // The §2.2 zeros the plan flagged: SP+52 raw exposure.
        for record in &header.streams {
            assert_eq!(record.flags_raw, 0, "{}", truth.file);
        }

        // The ffmpeg-fixture walk is damage-free (advisory objects
        // agree; nothing skipped in these clean files).
        assert_eq!(
            header.damage.total, 0,
            "{}: {:?}",
            truth.file, header.damage.events
        );

        // ASF-b on real muxer output: drain every event. Per track:
        // something emitted, the first stamp zero-based (ffprobe's
        // own arithmetic — preroll subtracted), stamps monotonic,
        // and the flush endpoint exactly at the end.
        let mut seen = vec![0u64; tracks.len()];
        let mut first_pts = vec![i64::MAX; tracks.len()];
        let mut last_pts = vec![i64::MIN; tracks.len()];
        loop {
            match ok(asf.next_event()) {
                DemuxEvent::Packet(p) => {
                    let slot = p.track.handle() as usize;
                    if let Some(count) = seen.get_mut(slot) {
                        *count += 1;
                    }
                    if let Stamp::Present(ts) = &p.pts {
                        if let Some(first) = first_pts.get_mut(slot) {
                            *first = (*first).min(ts.ticks);
                        }
                        if let Some(last) = last_pts.get_mut(slot) {
                            assert!(
                                ts.ticks >= *last,
                                "{}: track {slot} pts went backwards",
                                truth.file
                            );
                            *last = ts.ticks;
                        }
                    }
                }
                DemuxEvent::EndOfStream => break,
                _ => {}
            }
        }
        for (slot, count) in seen.iter().enumerate() {
            assert!(*count > 0, "{}: track {slot} emitted nothing", truth.file);
            assert_eq!(
                first_pts[slot], 0,
                "{}: track {slot} first pts",
                truth.file
            );
        }

        // ASF-c on the indexed fixtures: seek mid-file, land, and
        // resume the walk (the live ffprobe lane below carries the
        // oracle side of the arithmetic).
        if truth.simple_index.is_some() {
            let landed = ok(asf.seek(span_at(1000)));
            assert!(
                matches!(&landed.start, Stamp::Present(ts) if ts.ticks >= 0),
                "{}: {landed:?}",
                truth.file
            );
            let mut resumed = 0_u64;
            loop {
                match ok(asf.next_event()) {
                    DemuxEvent::Packet(_) => resumed += 1,
                    DemuxEvent::EndOfStream => break,
                    _ => {}
                }
            }
            assert!(resumed > 0, "{}: no packets after seek", truth.file);
        }
    }
}

#[test]
fn fixture_family_cross_checks_against_live_ffprobe() {
    let Some(dir) = fixture_dir() else {
        return;
    };
    if Command::new("ffprobe").arg("-version").output().is_err() {
        eprintln!("SKIP: ffprobe absent (asf ffprobe cross-check)");
        return;
    }
    for truth in FIXTURES {
        let path = generate(&dir, truth);
        let out = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name",
                "-of",
                "csv=p=0",
            ])
            .arg(&path)
            .output()
            .unwrap_or_else(|e| unreachable!("ffprobe spawn: {e}"));
        assert!(
            out.status.success(),
            "ffprobe {}: {}",
            truth.file,
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8_lossy(&out.stdout);
        let mut probed: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
        probed.retain(|l| !l.is_empty());
        // csv=p=0 prints `codec_name` per stream; ours must match
        // ffprobe's stream count AND codec identity exactly.
        assert_eq!(
            probed.len(),
            truth.ffprobe_codecs.len(),
            "{}: {probed:?}",
            truth.file
        );
        for (probed, expected) in probed.iter().zip(truth.ffprobe_codecs) {
            assert_eq!(*probed, *expected, "{}", truth.file);
        }
        // And our walk's track count equals ffprobe's stream count.
        let bytes = read_file(&path);
        let mut asf = AsfDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{}: new: {e}", truth.file));
        let tracks = match asf.start() {
            Ok(t) => t,
            Err(e) => unreachable!("{}: {e}", truth.file),
        };
        assert_eq!(tracks.len(), probed.len(), "{}", truth.file);

        // The ASF-b differential proper: our per-track media-object
        // counts vs ffprobe's demuxed packet counts (live oracle,
        // `-count_packets`).
        let out = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-count_packets",
                "-show_entries",
                "stream=nb_read_packets",
                "-of",
                "csv=p=0",
            ])
            .arg(&path)
            .output()
            .unwrap_or_else(|e| unreachable!("ffprobe spawn: {e}"));
        assert!(
            out.status.success(),
            "ffprobe {}: {}",
            truth.file,
            String::from_utf8_lossy(&out.stderr)
        );
        let counted: Vec<u64> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .map(|l| l.parse().unwrap_or(u64::MAX))
            .collect();
        assert_eq!(counted.len(), tracks.len(), "{}", truth.file);
        let mut ours = vec![0u64; tracks.len()];
        loop {
            match ok(asf.next_event()) {
                DemuxEvent::Packet(p) => {
                    if let Some(slot) = ours.get_mut(p.track.handle() as usize) {
                        *slot += 1;
                    }
                }
                DemuxEvent::EndOfStream => break,
                _ => {}
            }
        }
        for (slot, (mine, oracle)) in ours.iter().zip(counted).enumerate() {
            assert_eq!(
                mine, &oracle,
                "{}: track {slot} media objects vs ffprobe packets",
                truth.file
            );
        }
    }
}

#[test]
fn broadcast_fixture_refuses_typed_where_ffprobe_refuses() {
    let Some(dir) = fixture_dir() else {
        return;
    };
    let path = dir.join("asf_stream_broadcast.wmv");
    if !path.exists() {
        let out = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "lavfi"])
            .arg("-i")
            .arg("testsrc=size=64x48:rate=10:duration=1")
            .args(["-f", "lavfi", "-i"])
            .arg("sine=frequency=440:duration=1:sample_rate=8000")
            .args(["-ac", "1", "-c:v", "wmv2", "-b:v", "100k"])
            .args(["-c:a", "wmav2", "-b:a", "32k", "-f", "asf_stream"])
            .arg(&path)
            .output()
            .unwrap_or_else(|e| unreachable!("ffmpeg spawn: {e}"));
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let bytes = read_file(&path);

    // ffprobe itself refuses the broadcast form (§2.5, verified
    // twice in the plan and re-measured here): the oracle's own
    // answer is refusal.
    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-f", "asf"])
        .arg(&path)
        .output()
        .unwrap_or_else(|e| unreachable!("ffprobe spawn: {e}"));
    assert!(
        !probe.status.success(),
        "ffprobe must refuse the broadcast fixture (§2.5)"
    );

    // The content probe misses (the 12-byte MMS prefix) — honest
    // probe-miss, never a false anchor.
    assert!(!super::probe(&bytes[..16]));

    // Our walk recognizes the prefix shape, parses far enough to read
    // the FPO flags, and refuses the broadcast bit typed — the
    // honest answer, with the reason ffprobe's bare refusal lacks.
    let mut asf = AsfDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match asf.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(code_of(&err), "demux.asf.broadcast-unsupported");
}

/// The §4.4 sha256 pins (first 16 hex characters).
fn expected_sha(file: &str) -> &'static str {
    match file {
        "wma_wmav2_mono8k.wma" => "2f1abf090401e5d4",
        "wma_wmav1_st44k.wma" => "40e3e4b6eb3e707b",
        "wmv_wmv2_64x48.wmv" => "f1803712b502c41a",
        "wmv_wmv1_64x48.wmv" => "3cfac4e86fee00fc",
        "asf_msmpeg4v3.asf" => "66201b9205f26a90",
        "asf_mjpeg.asf" => "1454c0a3a62b640e",
        "wmv_wmv2_wma_av.wmv" => "5bd8c76fa14e31aa",
        "asf_pkt8000.wmv" => "e9ef0bc063ccc02f",
        "asf_pcm_s16le.wma" => "29c7a469c3627a6f",
        "asf_adpcm_ms.wma" => "02ca9cfcf394078b",
        "asf_stream_broadcast.wmv" => "a9db9175da47fe6c",
        other => unreachable!("unpinned fixture {other}"),
    }
}

/// Test-only SHA-256 (FIPS 180-4) — the shared `tests/common`
/// implementation's in-crate twin, pinned against the recorded
/// fixture hashes above (which were measured with the system
/// sha256sum this session).
fn sha256_16(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = bytes.to_vec();
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for block in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for (i, wi) in w.iter().enumerate() {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(*wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut hex = String::new();
    for word in h {
        hex.push_str(&format!("{word:08x}"));
    }
    hex[..16].to_string()
}
