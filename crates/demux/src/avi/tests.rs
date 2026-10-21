//! Unit tests for the AVI demuxer core — in-memory containers built by
//! the same byte-level rules the fixture writer uses (the committed
//! corpus in `tests/` is the end-to-end conformance layer on top of
//! these).

use super::test_support;
use super::*;
use crate::error;
use media_types::ids::MediaKind;
use media_types::time::{TimeBase, Timestamp};
use plugin_api::{DemuxEvent, Demuxer, SourceStream};

// ---------------------------------------------------------------------------
// A tiny AVI builder (mirrors tools/xtask fixtures.rs byte-for-byte
// rules: WORD-aligned chunks, patched LIST/RIFF sizes, idx1 offsets
// relative to the `movi` form FourCC).
// ---------------------------------------------------------------------------

fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) -> usize {
    let offset = out.len();
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
    offset
}

fn end_sized(out: &mut [u8], size_pos: usize) {
    let size = out.len() - size_pos - 4;
    out[size_pos..size_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
}

fn avih(flags: u32, streams: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    for value in [
        33_333u32, // dwMicroSecPerFrame
        0,         // dwMaxBytesPerSec
        0,         // dwPaddingGranularity
        flags, 60, // dwTotalFrames
        0,  // dwInitialFrames
        streams, 0, // dwSuggestedBufferSize
        320, 240,
    ] {
        v.extend_from_slice(&value.to_le_bytes());
    }
    v.extend_from_slice(&[0u8; 16]);
    v
}

/// `strh` with the v0-relevant fields; everything else zeroed.
fn strh(
    fcc: &[u8; 4],
    handler: &[u8; 4],
    scale: u32,
    rate: u32,
    length: u32,
    ssize: u32,
) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    v.extend_from_slice(fcc);
    v.extend_from_slice(handler);
    v.extend_from_slice(&[0u8; 12]); // flags, priority, language, initial
    v.extend_from_slice(&scale.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // dwStart
    v.extend_from_slice(&length.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // dwSuggestedBufferSize
    v.extend_from_slice(&0u32.to_le_bytes()); // dwQuality
    v.extend_from_slice(&ssize.to_le_bytes()); // dwSampleSize
    v.extend_from_slice(&[0u8; 8]); // rcFrame
    v
}

fn strf_video(width: u32, height: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(40);
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(b"MJPG");
    v.extend_from_slice(&[0u8; 20]);
    v
}

fn strf_pcm(channels: u16, rate: u32, block_align: u16, bits: u16) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * u32::from(channels) * u32::from(bits / 8)).to_le_bytes());
    v.extend_from_slice(&block_align.to_le_bytes());
    v.extend_from_slice(&bits.to_le_bytes());
    v
}

/// The streams a built file carries.
enum Stream {
    Video {
        scale: u32,
        rate: u32,
        length: u32,
        width: u32,
        height: u32,
    },
    Audio {
        scale: u32,
        rate: u32,
        length: u32,
        ssize: u32,
        strf: Vec<u8>,
    },
    /// A real-but-unsupported stream (txts/mids/…): strl present,
    /// outside the v0 codec filter.
    Other {
        fcc: &'static [u8; 4],
        handler: &'static [u8; 4],
    },
}

fn video_stream() -> Stream {
    Stream::Video {
        scale: 1,
        rate: 30,
        length: 3,
        width: 320,
        height: 240,
    }
}

fn audio_stream() -> Stream {
    Stream::Audio {
        scale: 1,
        rate: 48_000,
        length: 4_000,
        ssize: 4,
        strf: strf_pcm(2, 48_000, 4, 16),
    }
}

/// One movi payload chunk spec: (tag, bytes).
type MoviChunk<'a> = (&'static [u8; 4], &'a [u8]);

struct BuiltAvi {
    bytes: Vec<u8>,
    /// Absolute positions of the movi chunks (assertions).
    chunk_positions: Vec<u64>,
    movi_form_pos: u64,
}

fn build(streams: &[Stream], movi_chunks: &[MoviChunk<'_>], with_idx1: bool) -> BuiltAvi {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    let riff_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"AVI ");

    // LIST hdrl: avih + one LIST strl per stream.
    out.extend_from_slice(b"LIST");
    let hdrl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"hdrl");
    chunk(&mut out, b"avih", &avih(0x110, streams.len() as u32));
    for stream in streams {
        out.extend_from_slice(b"LIST");
        let strl_size_pos = out.len();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"strl");
        match *stream {
            Stream::Video {
                scale,
                rate,
                length,
                width,
                height,
            } => {
                chunk(
                    &mut out,
                    b"strh",
                    &strh(b"vids", b"MJPG", scale, rate, length, 0),
                );
                chunk(&mut out, b"strf", &strf_video(width, height));
            }
            Stream::Audio {
                scale,
                rate,
                length,
                ssize,
                ref strf,
            } => {
                chunk(
                    &mut out,
                    b"strh",
                    &strh(b"auds", &[0; 4], scale, rate, length, ssize),
                );
                chunk(&mut out, b"strf", strf);
            }
            Stream::Other { fcc, handler } => {
                chunk(&mut out, b"strh", &strh(fcc, handler, 1, 1, 0, 0));
            }
        }
        end_sized(&mut out, strl_size_pos);
    }
    end_sized(&mut out, hdrl_size_pos);

    // LIST movi: the given chunks in order.
    let movi_offset = out.len();
    out.extend_from_slice(b"LIST");
    let movi_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"movi");
    let movi_form_pos = (movi_offset + 8) as u64;
    let mut chunk_positions = Vec::new();
    for (tag, data) in movi_chunks {
        chunk_positions.push(chunk(&mut out, tag, data) as u64);
    }
    end_sized(&mut out, movi_size_pos);

    if with_idx1 {
        let mut idx = Vec::new();
        for ((tag, data), pos) in movi_chunks.iter().zip(&chunk_positions) {
            idx.extend_from_slice(*tag);
            idx.extend_from_slice(&0x10u32.to_le_bytes()); // AVIIF_KEYFRAME
            idx.extend_from_slice(&((*pos - movi_form_pos) as u32).to_le_bytes());
            idx.extend_from_slice(&(data.len() as u32).to_le_bytes());
        }
        chunk(&mut out, b"idx1", &idx);
    }
    end_sized(&mut out, riff_size_pos);

    BuiltAvi {
        bytes: out,
        chunk_positions,
        movi_form_pos,
    }
}

fn demuxer_over(bytes: Vec<u8>) -> AviDemuxer {
    match AviDemuxer::new(match test_support::mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem stream: {err}"),
    }) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("demuxer over memory: {err}"),
    }
}

/// Drain every event; return the packets (asserting no error — tests
/// that expect errors call `next_event` themselves).
fn drain_packets(demuxer: &mut AviDemuxer) -> Vec<media_types::Packet> {
    let mut packets = Vec::new();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => packets.push(packet),
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(_) => {}
            Err(err) => unreachable!("clean drain: {err}"),
        }
    }
    packets
}

// ---------------------------------------------------------------------------
// Header / track table
// ---------------------------------------------------------------------------

#[test]
fn two_stream_start_reports_the_track_table() {
    let built = build(
        &[video_stream(), audio_stream()],
        &[
            (b"00dc", &b"frame-zero"[..]),
            (b"01wb", &b"abcd"[..]),
            (b"00dc", &b"frame-one"[..]),
            (b"01wb", &b"efgh"[..]),
        ],
        true,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("start: {err}"),
    };
    assert_eq!(tracks.len(), 2);

    let video = &tracks[0];
    assert_eq!(video.id.handle(), 0);
    assert_eq!(video.kind, MediaKind::Video);
    assert_eq!(video.codec.as_str(), "mjpeg");
    let (num, den) = video.timing.base.as_parts();
    assert_eq!((num, den), (1, 30));
    // Span: 3 declared frames from 0 → end tick 3 on the 1/30 base.
    match (
        video.timing.span.start.timestamp(),
        video.timing.span.end.timestamp(),
    ) {
        (Ok(a), Ok(b)) => {
            assert_eq!(a.ticks, 0);
            assert_eq!(b.ticks, 3);
        }
        _ => unreachable!("video span stamps are present"),
    }

    let audio = &tracks[1];
    assert_eq!(audio.id.handle(), 1);
    assert_eq!(audio.kind, MediaKind::Audio);
    assert_eq!(audio.codec.as_str(), "pcm_s16le");
    let (num, den) = audio.timing.base.as_parts();
    assert_eq!((num, den), (1, 48_000));

    // start is idempotent (same snapshot, no re-parse side effects).
    let again = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("start again: {err}"),
    };
    assert_eq!(again.len(), 2);

    // The format seam for the decoder cards.
    assert_eq!(
        avi.stream_format(tracks[0].id),
        Some(header::StreamFormat::MotionJpeg {
            width: 320,
            height: 240
        })
    );
    assert_eq!(
        avi.stream_format(tracks[1].id),
        Some(header::StreamFormat::Pcm {
            channels: 2,
            samples_per_sec: 48_000,
            bits_per_sample: 16,
            block_align: 4
        })
    );
    assert_eq!(avi.stream_format(media_types::ids::TrackId::new(9)), None);
    // Advisory avih fields are exposed but never trusted.
    assert_eq!(avi.main_header().streams, 2);
    assert_eq!(avi.main_header().flags, 0x110);
}

#[test]
fn strn_title_surfaces_on_the_track() {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    let riff_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(b"LIST");
    let hdrl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"hdrl");
    chunk(&mut out, b"avih", &avih(0, 1));
    out.extend_from_slice(b"LIST");
    let strl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"strl");
    chunk(&mut out, b"strh", &strh(b"vids", b"MJPG", 1, 30, 1, 0));
    chunk(&mut out, b"strf", &strf_video(64, 48));
    chunk(&mut out, b"strn", b"Lumen test stream\x00");
    end_sized(&mut out, strl_size_pos);
    end_sized(&mut out, hdrl_size_pos);
    out.extend_from_slice(b"LIST");
    let movi_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"movi");
    chunk(&mut out, b"00dc", b"f");
    end_sized(&mut out, movi_size_pos);
    end_sized(&mut out, riff_size_pos);

    let mut avi = demuxer_over(out);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("start: {err}"),
    };
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title.as_deref(), Some("Lumen test stream"));
}

#[test]
fn unsupported_stream_types_are_filtered_with_damage() {
    let built = build(
        &[
            video_stream(),
            Stream::Other {
                fcc: b"txts",
                handler: &[0; 4],
            },
        ],
        &[(b"00dc", &b"frame-zero"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("start: {err}"),
    };
    assert_eq!(tracks.len(), 1, "only the MJPEG stream survives the filter");
    // The skip is observable: damage was counted.
    assert!(
        avi.damage_total() == 0,
        "a decision, not bytes: 0 skipped bytes"
    );
    // The first event is the Recovered record of the filter decision.
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 0);
            assert_eq!(reason, "unsupported-stream-type");
        }
        other => unreachable!("damage surfaces first, got {other:?}"),
    }
}

#[test]
fn all_streams_unsupported_is_typed() {
    let built = build(
        &[Stream::Other {
            fcc: b"mids",
            handler: &[0; 4],
        }],
        &[(b"00dc", &b"x"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let err = match avi.start() {
        Ok(_) => unreachable!("no supported streams must be an error"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.no-supported-streams");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::Unsupported { .. }
    ));
}

#[test]
fn zero_rate_or_scale_is_bad_stream_params() {
    let built = build(
        &[Stream::Video {
            scale: 0,
            rate: 30,
            length: 1,
            width: 64,
            height: 48,
        }],
        &[(b"00dc", &b"x"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let err = match avi.start() {
        Ok(_) => unreachable!("zero scale must be an error"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.bad-stream-params");
    assert_eq!(err.stage, media_types::PipelineStage::Demux);
}

// ---------------------------------------------------------------------------
// movi walk
// ---------------------------------------------------------------------------

#[test]
fn packets_carry_the_avi_ladder_timing() {
    let built = build(
        &[video_stream(), audio_stream()],
        &[
            (b"00dc", &b"v0"[..]),
            (b"01wb", &[0u8; 8]), // 2 sample frames of block-align 4
            (b"00dc", &b"v1"[..]),
            (b"01wb", &[0u8; 4]), // 1 sample frame
            (b"00dc", &b"v2"[..]),
        ],
        true,
    );
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 2),
        Err(err) => unreachable!("start: {err}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 5);

    let video_base = TimeBase::new(1, 30).unwrap_or_else(|err| unreachable!("{err}"));

    // Video: one tick per chunk, all keyframes.
    for (index, packet) in packets.iter().filter(|p| p.track.handle() == 0).enumerate() {
        let pts = match packet.pts {
            media_types::time::Stamp::Present(ts) => ts,
            _ => unreachable!("video pts present"),
        };
        assert_eq!(pts.ticks, index as i64, "video ladder");
        assert_eq!(pts.base, video_base);
        assert_eq!(packet.dts, packet.pts, "no B-frames: DTS mirrors PTS");
        assert!(packet.keyframe, "idx1 flags every entry a keyframe");
        let duration = match packet.duration {
            Some(media_types::Duration::Known(ns)) => ns,
            other => unreachable!("video duration known, got {other:?}"),
        };
        assert_eq!(duration.0, 33_333_333, "1/30 s in ns");
    }

    // Audio: ticks advance by sample frames (8 bytes / 4 = 2, then 1).
    let audio_pts: Vec<i64> = packets
        .iter()
        .filter(|p| p.track.handle() == 1)
        .map(|p| match p.pts {
            media_types::time::Stamp::Present(ts) => ts.ticks,
            _ => unreachable!("audio pts present"),
        })
        .collect();
    assert_eq!(audio_pts, vec![0, 2], "audio ladder in sample frames");
    let audio = &packets[1];
    assert_eq!(audio.track.handle(), 1);
    match audio.duration {
        Some(media_types::Duration::Known(ns)) => {
            assert_eq!(ns.0, 41_667, "2/48000 s rounded to nearest ns");
        }
        other => unreachable!("audio duration known, got {other:?}"),
    }
    assert!(audio.keyframe);
}

#[test]
fn odd_payloads_strip_the_word_pad() {
    // 5-byte video payload → one pad byte in the file, none in the
    // packet; the following chunk still lands where the file says.
    let built = build(
        &[video_stream()],
        &[(b"00dc", &b"abcde"[..]), (b"00dc", &b"fg"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].data.as_slice(), b"abcde");
    assert_eq!(packets[1].data.as_slice(), b"fg");
    assert_eq!(avi.damage_total(), 0, "pad handling is not damage");
}

#[test]
fn rec_lists_are_transparent() {
    // Chunks grouped in one `rec ` list inside movi.
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    let riff_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(b"LIST");
    let hdrl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"hdrl");
    chunk(&mut out, b"avih", &avih(0, 1));
    out.extend_from_slice(b"LIST");
    let strl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"strl");
    chunk(&mut out, b"strh", &strh(b"vids", b"MJPG", 1, 30, 2, 0));
    chunk(&mut out, b"strf", &strf_video(64, 48));
    end_sized(&mut out, strl_size_pos);
    end_sized(&mut out, hdrl_size_pos);
    out.extend_from_slice(b"LIST");
    let movi_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"movi");
    out.extend_from_slice(b"LIST");
    let rec_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"rec ");
    chunk(&mut out, b"00dc", b"r0");
    chunk(&mut out, b"00dc", b"r1");
    end_sized(&mut out, rec_size_pos);
    chunk(&mut out, b"00dc", b"t0");
    end_sized(&mut out, movi_size_pos);
    end_sized(&mut out, riff_size_pos);

    let mut avi = demuxer_over(out);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("start: {err}"),
    }
    let packets = drain_packets(&mut avi);
    let payloads: Vec<&[u8]> = packets.iter().map(|p| p.data.as_slice()).collect();
    assert_eq!(payloads, vec![&b"r0"[..], &b"r1"[..], &b"t0"[..]]);
}

#[test]
fn unknown_movi_chunks_skip_with_damage() {
    let built = build(&[video_stream()], &[(b"00dc", &b"v0"[..])], false);
    let mut bytes = built.bytes;
    // Splice a JUNK chunk between the movi form and the first packet
    // chunk: rebuild by locating movi form then inserting.
    let movi_form = built.movi_form_pos as usize + 4; // first chunk would start here
    let mut spliced = bytes[..movi_form].to_vec();
    chunk(&mut spliced, b"JUNK", &[0u8; 12]);
    spliced.extend_from_slice(&bytes[movi_form..]);
    // Sizes still honest? The movi/riff sizes grew by the splice; patch both.
    let grow = (spliced.len() - bytes.len()) as u32;
    patch_size(&mut spliced, 4, grow);
    // movi LIST size field sits at (movi_form - 8 + 4).
    let movi_list_size_pos = built.movi_form_pos as usize - 4;
    patch_size(&mut spliced, movi_list_size_pos, grow);
    bytes = spliced;

    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 12);
            assert_eq!(reason, "unknown-chunk-in-movi");
        }
        other => unreachable!("damage first, got {other:?}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].data.as_slice(), b"v0");
}

/// Add `grow` to the u32le size field at `size_pos`.
fn patch_size(bytes: &mut [u8], size_pos: usize, grow: u32) {
    let current = u32::from_le_bytes([
        bytes[size_pos],
        bytes[size_pos + 1],
        bytes[size_pos + 2],
        bytes[size_pos + 3],
    ]);
    let patched = current.wrapping_add(grow);
    bytes[size_pos..size_pos + 4].copy_from_slice(&patched.to_le_bytes());
}

#[test]
fn pcm_misalignment_is_damage_not_silence() {
    // Audio chunk of 6 bytes with block align 4: 1 sample frame + a
    // 2-byte stray — counted, packet still emitted with floored ticks.
    let built = build(&[audio_stream()], &[(b"00wb", &[0xAA; 6])], false);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 2);
            assert_eq!(reason, "pcm-chunk-not-sample-aligned");
        }
        other => unreachable!("damage first, got {other:?}"),
    }
    match avi.next_event() {
        Ok(DemuxEvent::Packet(packet)) => {
            assert_eq!(packet.data.as_slice(), &[0xAA; 6]);
            match packet.pts {
                media_types::time::Stamp::Present(ts) => assert_eq!(ts.ticks, 0),
                _ => unreachable!("pts present"),
            }
        }
        other => unreachable!("then the packet, got {other:?}"),
    }
    assert_eq!(avi.damage_total(), 2);
}

#[test]
fn idx1_keyframe_flag_is_the_truth_when_present() {
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
        ],
        false,
    );
    // Hand-build an idx1 whose middle entry is NOT a keyframe.
    let mut bytes = built.bytes;
    let mut idx = Vec::new();
    for (index, pos) in built.chunk_positions.iter().enumerate() {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&if index == 1 { 0u32 } else { 0x10u32 }.to_le_bytes());
        idx.extend_from_slice(&((*pos - built.movi_form_pos) as u32).to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());
    }
    let idx1_offset = bytes.len();
    chunk(&mut bytes, b"idx1", &idx);
    let grow = (bytes.len() - idx1_offset) as u32;
    patch_size(&mut bytes, 4, grow);

    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 3);
    assert!(packets[0].keyframe);
    assert!(!packets[1].keyframe, "idx1 says frame 1 is not a keyframe");
    assert!(packets[2].keyframe);
}

// ---------------------------------------------------------------------------
// Errors: typed flavors and stickiness
// ---------------------------------------------------------------------------

#[test]
fn broken_magic_is_typed_not_avi() {
    let mut bytes = build(&[video_stream()], &[(b"00dc", &b"x"[..])], false).bytes;
    bytes[0] = b'X';
    let mut avi = demuxer_over(bytes);
    let err = match avi.start() {
        Ok(_) => unreachable!("broken magic must be an error"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.not-avi");
    match err.kind {
        media_types::ErrorKind::Unsupported { format } => {
            assert_eq!(format.and_then(|f| f.name).as_deref(), Some("avi"));
        }
        other => unreachable!("unsupported kind, got {other:?}"),
    }
}

#[test]
fn truncated_head_is_typed_at_the_boundary() {
    let bytes = build(&[video_stream()], &[(b"00dc", &b"x"[..])], false).bytes;
    // A prefix too short to even read the 12-byte RIFF head is
    // honestly "not an AVI", not "truncated".
    for cut in [4usize, 11] {
        let mut avi = demuxer_over(bytes[..cut].to_vec());
        let err = match avi.start() {
            Ok(_) => unreachable!("cut at {cut} must be an error"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.avi.not-avi", "cut {cut}");
    }
    // Structural cuts inside the header are typed truncation at the
    // exact boundary the structure needed.
    for cut in [12usize, 20, 30, 44] {
        let mut avi = demuxer_over(bytes[..cut].to_vec());
        let err = match avi.start() {
            Ok(_) => unreachable!("cut at {cut} must be an error"),
            Err(err) => err,
        };
        match err.kind {
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated { .. },
            } => {}
            other => unreachable!("truncated flavor at cut {cut}, got {other:?}"),
        }
        assert_eq!(err.stage, media_types::PipelineStage::Demux);
        // Sticky: the same failure replays with the same shape.
        let replay = match avi.start() {
            Ok(_) => unreachable!("sticky failure replays"),
            Err(err) => err,
        };
        assert_eq!(replay.context.code, err.context.code);
    }
}

#[test]
fn truncated_movi_payload_errors_and_sticks() {
    let built = build(
        &[video_stream()],
        &[(b"00dc", &b"v0"[..]), (b"00dc", &[0x42; 100])],
        false,
    );
    // Cut inside the second chunk's payload.
    let cut = (built.chunk_positions[1] + 8 + 40) as usize;
    let mut avi = demuxer_over(built.bytes[..cut].to_vec());
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("header intact: {err}"),
    }
    // Drain manually: the cut is expected to end the stream with the
    // typed truncation, so a clean-drain helper is wrong here.
    let mut packets = Vec::new();
    let err = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => packets.push(packet),
            Ok(DemuxEvent::EndOfStream) => unreachable!("cut payload must error"),
            Ok(_) => {}
            Err(err) => break err,
        }
    };
    assert_eq!(packets.len(), 1, "first frame survives");
    match err.kind {
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::Truncated { expected, found },
        } => {
            assert_eq!(expected, built.chunk_positions[1] + 8 + 100);
            assert_eq!(found, cut as u64);
        }
        other => unreachable!("truncated flavor, got {other:?}"),
    }
    // Sticky replay, byte-identical shape.
    let replay = match avi.next_event() {
        Ok(_) => unreachable!("sticky failure replays"),
        Err(err) => err,
    };
    assert_eq!(replay.kind, err.kind);
    assert_eq!(replay.context.code, err.context.code);
}

#[test]
fn missing_header_and_missing_movi_are_distinct_typed_errors() {
    // RIFF/AVI with only a movi: no hdrl.
    let built = build(&[video_stream()], &[(b"00dc", &b"x"[..])], false);
    let hdrl_start = 12;
    let movi_list_start = built
        .chunk_positions
        .first()
        .copied()
        .unwrap_or(0)
        .saturating_sub(12);
    let _ = movi_list_start;
    let mut bytes = built.bytes.clone();
    // Remove the hdrl LIST by shifting: simply build a file whose
    // top-level walk sees movi first (drop bytes 12..movi_offset).
    let movi_offset = (built.movi_form_pos - 8) as usize;
    let mut no_hdrl = bytes[..hdrl_start].to_vec();
    no_hdrl.extend_from_slice(&bytes[movi_offset..]);
    let shrink = (movi_offset - hdrl_start) as u32;
    patch_size(&mut no_hdrl, 4, shrink.wrapping_neg());
    let mut avi = demuxer_over(no_hdrl);
    let err = match avi.start() {
        Ok(_) => unreachable!("no hdrl must be an error"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.no-header");

    // RIFF/AVI with only an hdrl: no movi.
    bytes.truncate(movi_offset);
    let shrink = (built.bytes.len() - movi_offset) as u32;
    patch_size(&mut bytes, 4, shrink.wrapping_neg());
    let mut avi = demuxer_over(bytes);
    let err = match avi.start() {
        Ok(_) => unreachable!("no movi must be an error"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.no-movi");
}

#[test]
fn methods_before_start_are_typed_not_panics() {
    let built = build(&[video_stream()], &[(b"00dc", &b"x"[..])], false);
    let mut avi = demuxer_over(built.bytes);
    let code = "demux.not-started";
    let err = match avi.next_event() {
        Ok(_) => unreachable!("next_event before start is typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, code);
    let err = match avi.seek(media_types::time::TimeSpan::UNKNOWN) {
        Ok(_) => unreachable!("seek before start is typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, code);
}

#[test]
fn unusable_sources_are_rejected_at_construction() {
    use media_types::ids::SourceUri;

    struct Broken {
        uri: SourceUri,
        size: Option<u64>,
        seek: bool,
    }
    impl SourceStream for Broken {
        fn uri(&self) -> &SourceUri {
            &self.uri
        }
        fn size(&self) -> Option<u64> {
            self.size
        }
        fn supports_seek(&self) -> bool {
            self.seek
        }
        fn read(&mut self, _len: usize) -> Result<Vec<u8>, media_types::EngineError> {
            Ok(Vec::new())
        }
        fn seek(
            &mut self,
            _origin: plugin_api::SeekOrigin,
        ) -> Result<plugin_api::SourcePosition, media_types::EngineError> {
            Ok(plugin_api::SourcePosition(0))
        }
        fn position(&self) -> plugin_api::SourcePosition {
            plugin_api::SourcePosition(0)
        }
    }
    let uri = match SourceUri::parse("memory://broken") {
        Ok(uri) => uri,
        Err(err) => unreachable!("{err}"),
    };
    for (size, seek, code) in [
        (None, true, "demux.source.unknown-length"),
        (Some(42), false, "demux.source.unseekable"),
        (Some(0), true, "demux.source.empty"),
    ] {
        let err = match AviDemuxer::new(Box::new(Broken {
            uri: uri.clone(),
            size,
            seek,
        })) {
            Ok(_) => unreachable!("broken source must be rejected"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, code);
        assert_eq!(err.stage, media_types::PipelineStage::Open);
    }
}

// ---------------------------------------------------------------------------
// Seek
// ---------------------------------------------------------------------------

fn span_at_nanos(nanos: i64) -> media_types::time::TimeSpan {
    let base = TimeBase::new(1, 1_000_000_000).unwrap_or_else(|err| unreachable!("{err}"));
    media_types::time::TimeSpan {
        start: media_types::time::Stamp::Present(Timestamp::new(nanos, base)),
        end: media_types::time::Stamp::Missing,
    }
}

#[test]
fn seek_lands_on_last_keyframe_at_or_below_target() {
    // 4 video frames at 30 fps: ticks 0..3; frame 1 marked non-key.
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
            (b"00dc", &b"d"[..]),
        ],
        false,
    );
    let mut bytes = built.bytes;
    let mut idx = Vec::new();
    for (index, pos) in built.chunk_positions.iter().enumerate() {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&if index == 1 { 0u32 } else { 0x10u32 }.to_le_bytes());
        idx.extend_from_slice(&((*pos - built.movi_form_pos) as u32).to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());
    }
    let idx1_offset = bytes.len();
    chunk(&mut bytes, b"idx1", &idx);
    let grow = (bytes.len() - idx1_offset) as u32;
    patch_size(&mut bytes, 4, grow);

    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // 0 ms → frame 0.
    let landing = match avi.seek(span_at_nanos(0)) {
        Ok(span) => span,
        Err(err) => unreachable!("seek 0: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 0),
        Err(_) => unreachable!("landing start present"),
    }
    // 45 ms → tick 1.35 → tick 1: frame 1 is not a keyframe, so the
    // landing is frame 0 (the last keyframe at-or-below).
    let landing = match avi.seek(span_at_nanos(45_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("seek 45ms: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 0, "non-keyframe skipped"),
        Err(_) => unreachable!("landing start present"),
    }
    // 65 ms → tick 1.95 → tick 2: a keyframe — lands on it.
    let landing = match avi.seek(span_at_nanos(65_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("seek 65ms: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 2),
        Err(_) => unreachable!("landing start present"),
    }
    // 100 ms → frame 3? No: 100 ms = tick 3 exactly → the keyframe at 3.
    let landing = match avi.seek(span_at_nanos(100_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("seek 100ms: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 3),
        Err(_) => unreachable!("landing start present"),
    }
    // The span end is the declared track end (start + length).
    match landing.end.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 3),
        Err(_) => unreachable!("landing end present"),
    }
    // Landing ticks are on the stream's own base.
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(
            ts.base,
            TimeBase::new(1, 30).unwrap_or_else(|e| unreachable!("{e}"))
        ),
        Err(_) => unreachable!(),
    }
}

#[test]
fn seek_without_idx1_walks_the_movi() {
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
        ],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // 50 ms → tick 1.5 → rounds (ties away from zero) to tick 2; the
    // last chunk with pts ≤ 2 is tick 2.
    let landing = match avi.seek(span_at_nanos(50_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("linear seek: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 2),
        Err(_) => unreachable!("landing start present"),
    }
    // And the demuxer still walks from the beginning (seek is a pure
    // query): draining yields all three frames.
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 3);
}

#[test]
fn seek_out_of_range_is_typed() {
    let built = build(
        &[video_stream()],
        &[(b"00dc", &b"a"[..]), (b"00dc", &b"b"[..])],
        true,
    );
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // Past the declared end (length 3 frames declared, 2 present is
    // fine for the range check only against the DECLARED end: use a
    // clearly-past target).
    let err = match avi.seek(span_at_nanos(10_000_000_000)) {
        Ok(_) => unreachable!("past-end target is typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.seek.out-of-range");
    // Negative target.
    let negative = {
        let base = TimeBase::new(1, 1_000_000_000).unwrap_or_else(|e| unreachable!("{e}"));
        media_types::time::TimeSpan {
            start: media_types::time::Stamp::Present(Timestamp::new(-1, base)),
            end: media_types::time::Stamp::Missing,
        }
    };
    let err = match avi.seek(negative) {
        Ok(_) => unreachable!("negative target is typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.seek.out-of-range");
    // Missing stamp.
    let err = match avi.seek(media_types::time::TimeSpan::UNKNOWN) {
        Ok(_) => unreachable!("unknown span is typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.seek.out-of-range");
}

#[test]
fn seek_does_not_move_the_read_position() {
    // A position-recording wrapper around the in-memory stream.
    use std::sync::{Arc, Mutex};

    struct Tracked {
        inner: test_support::BytesStream,
        /// The stream position observed at every operation entry —
        /// comparing seek *targets* would miss read-driven movement.
        positions: Arc<Mutex<Vec<u64>>>,
    }
    impl Tracked {
        fn note_exit(&self) {
            if let Ok(mut log) = self.positions.lock() {
                log.push(self.inner.position().0);
            }
        }
    }
    impl SourceStream for Tracked {
        fn uri(&self) -> &media_types::ids::SourceUri {
            self.inner.uri()
        }
        fn size(&self) -> Option<u64> {
            self.inner.size()
        }
        fn supports_seek(&self) -> bool {
            self.inner.supports_seek()
        }
        fn read(&mut self, len: usize) -> Result<Vec<u8>, media_types::EngineError> {
            let outcome = self.inner.read(len);
            self.note_exit();
            outcome
        }
        fn seek(
            &mut self,
            origin: plugin_api::SeekOrigin,
        ) -> Result<plugin_api::SourcePosition, media_types::EngineError> {
            let outcome = self.inner.seek(origin);
            self.note_exit();
            outcome
        }
        fn position(&self) -> plugin_api::SourcePosition {
            self.inner.position()
        }
    }

    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
        ],
        false,
    );
    let positions = Arc::new(Mutex::new(Vec::new()));
    let tracked = Tracked {
        inner: match test_support::BytesStream::new(built.bytes.into()) {
            Ok(inner) => inner,
            Err(err) => unreachable!("{err}"),
        },
        positions: Arc::clone(&positions),
    };
    let mut avi = match AviDemuxer::new(Box::new(tracked)) {
        Ok(avi) => avi,
        Err(err) => unreachable!("{err}"),
    };
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let _ = drain_packets(&mut avi);
    let before = {
        let log = match positions.lock() {
            Ok(log) => log,
            Err(_) => unreachable!("position log lock"),
        };
        log.last().copied()
    };
    match avi.seek(span_at_nanos(50_000_000)) {
        Ok(_) => {}
        Err(err) => unreachable!("seek: {err}"),
    }
    let after = {
        let log = match positions.lock() {
            Ok(log) => log,
            Err(_) => unreachable!("position log lock"),
        };
        log.last().copied()
    };
    assert_eq!(
        before, after,
        "the linear fallback restores the read position before returning"
    );
}

// ---------------------------------------------------------------------------
// Recognition-is-authority recovery
// ---------------------------------------------------------------------------

#[test]
fn lying_hdrl_size_does_not_swallow_the_movi() {
    let built = build(&[video_stream()], &[(b"00dc", &b"a"[..])], false);
    let mut bytes = built.bytes;
    // The hdrl LIST size field sits at 16; lie hugely (covers the movi).
    bytes[16..20].copy_from_slice(&0x7FFF_0000u32.to_le_bytes());
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("recognition recovers the movi: {err}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].data.as_slice(), b"a");
}

#[test]
fn riff_extent_clamps_the_walk_to_real_bytes() {
    let built = build(&[video_stream()], &[(b"00dc", &b"a"[..])], false);
    let clean = built.bytes.clone();
    let mut bytes = built.bytes;
    // Append garbage past the RIFF extent: invisible to the walk.
    bytes.extend_from_slice(&[0xEE; 64]);
    let mut avi = demuxer_over(bytes.clone());
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("garbage past the extent is skipped: {err}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 1);
    assert_eq!(avi.damage_total(), 0);

    // Shrink the RIFF size below the movi: the movi is cut → typed
    // truncation at the walk boundary (not a silent empty EOF).
    // Rebuild from the clean parent bytes (the garbage-appended copy
    // above must not leak into the shrink arithmetic).
    let cut = (built.movi_form_pos + 4) as usize; // movi has no complete chunk
    let mut small = clean[..cut].to_vec();
    let shrink = (clean.len() - cut) as u32;
    patch_size(&mut small, 4, shrink.wrapping_neg());
    let mut avi = demuxer_over(small);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "header still parses"),
        Err(err) => unreachable!("start: {err}"),
    }
    // The clamp damage surfaces first; the truncation error follows.
    let err = loop {
        match avi.next_event() {
            Ok(DemuxEvent::EndOfStream) => unreachable!("clamped movi must be a typed truncation"),
            Ok(_) => {}
            Err(err) => break err,
        }
    };
    match err.kind {
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::Truncated { expected, found },
        } => {
            assert!(expected > found);
        }
        other => unreachable!("truncated flavor, got {other:?}"),
    }
}

#[test]
fn duplicate_movi_and_duplicate_idx1_count_as_damage() {
    let built = build(&[video_stream()], &[(b"00dc", &b"a"[..])], true);
    let mut bytes = built.bytes;
    // Append a second (empty) movi LIST and a second idx1 past the
    // extent — both must be skipped, not double-read.
    let extra_start = bytes.len();
    chunk(&mut bytes, b"LIST", &[]);
    let _ = extra_start;
    // Recompute: a LIST with form 'movi' needs the form inside payload.
    bytes.truncate(extra_start);
    let mut movi_dup = Vec::new();
    movi_dup.extend_from_slice(b"movi");
    chunk(&mut bytes, b"LIST", &movi_dup);
    chunk(&mut bytes, b"idx1", &[0u8; 32]);
    let grow = (bytes.len() - extra_start) as u32;
    patch_size(&mut bytes, 4, grow);

    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        avi.damage_total(),
        36,
        "duplicate movi (4) + duplicate idx1 (32) payload bytes"
    );
}

#[test]
fn eos_is_idempotent_and_terminates() {
    let built = build(&[video_stream()], &[(b"00dc", &b"a"[..])], false);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 1);
    for _ in 0..3 {
        match avi.next_event() {
            Ok(DemuxEvent::EndOfStream) => {}
            Ok(other) => unreachable!("EOS idempotent, got {other:?}"),
            Err(err) => unreachable!("EOS idempotent, got {err}"),
        }
    }
}

#[test]
fn error_module_constructors_stay_demux_staged() {
    // A cross-check tied to this module's usage sites: every error the
    // walk can mint carries the Demux stage and a stable code.
    let err = error::no_movi();
    assert_eq!(err.stage, media_types::PipelineStage::Demux);
    assert_eq!(err.context.code, "demux.avi.no-movi");
    let err = error::not_started();
    assert_eq!(err.stage, media_types::PipelineStage::Demux);
    assert_eq!(err.context.code, "demux.not-started");
}

// ---------------------------------------------------------------------------
// Re-anchor (card 3.A1: doc 03 §6 step 4 — the acting half of a seek)
// ---------------------------------------------------------------------------

/// A video stream with an explicit frame count (the shared
/// `video_stream()` pins length 3).
fn video_stream_of(length: u32) -> Stream {
    Stream::Video {
        scale: 1,
        rate: 30,
        length,
        width: 320,
        height: 240,
    }
}

/// The from-start truth for one file: every packet's (stream, ticks).
fn from_start_truth(
    streams: &[Stream],
    movi_chunks: &[MoviChunk<'_>],
    with_idx1: bool,
) -> Vec<(u32, i64)> {
    let built = build(streams, movi_chunks, with_idx1);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    drain_packets(&mut avi)
        .into_iter()
        .map(|packet| {
            let ts = match packet.pts {
                media_types::time::Stamp::Present(ts) => ts,
                media_types::time::Stamp::Missing => unreachable!("stamps present by D1"),
            };
            (packet.track.handle(), ts.ticks)
        })
        .collect()
}

#[test]
fn reanchor_moves_the_stream_and_packets_continue_from_the_landing() {
    // 4 video frames at 30 fps (ticks 0..3), indexed, frame 1 non-key
    // (same layout as the pure-query test above).
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
            (b"00dc", &b"d"[..]),
        ],
        false,
    );
    let mut bytes = built.bytes;
    let mut idx = Vec::new();
    for (index, pos) in built.chunk_positions.iter().enumerate() {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&if index == 1 { 0u32 } else { 0x10u32 }.to_le_bytes());
        idx.extend_from_slice(&((*pos - built.movi_form_pos) as u32).to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());
    }
    let idx1_offset = bytes.len();
    chunk(&mut bytes, b"idx1", &idx);
    let grow = (bytes.len() - idx1_offset) as u32;
    patch_size(&mut bytes, 4, grow);

    let truth = from_start_truth(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
            (b"00dc", &b"d"[..]),
        ],
        false,
    );
    // (The truth was rebuilt over the un-indexed layout; re-derive it
    // over the indexed bytes for exactness.)
    let mut truth_indexed = truth;
    truth_indexed.clear();
    {
        let mut probe = demuxer_over(bytes.clone());
        match probe.start() {
            Ok(_) => {}
            Err(err) => unreachable!("start: {err}"),
        }
        for packet in drain_packets(&mut probe) {
            match packet.pts {
                media_types::time::Stamp::Present(ts) => {
                    truth_indexed.push((packet.track.handle(), ts.ticks));
                }
                media_types::time::Stamp::Missing => unreachable!("D1"),
            }
        }
    }
    assert_eq!(truth_indexed, vec![(0, 0), (0, 1), (0, 2), (0, 3)]);

    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // Consume the first two frames, then re-anchor to 65 ms → tick 1.95
    // → the keyframe at tick 2 (frame 1 is not a keyframe).
    let _ = drain_through(&mut avi, 2);
    let landing = match avi.reanchor(span_at_nanos(65_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("reanchor: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 2, "lands on the keyframe at/below the target"),
        Err(_) => unreachable!("landing start present"),
    }
    // The stream continues from the landing with exactly the
    // from-start timestamps (the ladder replay is exact).
    let resumed = drain_packets(&mut avi);
    assert_eq!(resumed.len(), 2, "frames 2 and 3 remain");
    let resumed_truth: Vec<(u32, i64)> = resumed
        .into_iter()
        .map(|packet| match packet.pts {
            media_types::time::Stamp::Present(ts) => (packet.track.handle(), ts.ticks),
            media_types::time::Stamp::Missing => unreachable!("D1"),
        })
        .collect();
    assert_eq!(resumed_truth, vec![(0, 2), (0, 3)]);
}

/// Consume exactly `count` packets (ignoring container signals).
fn drain_through(demuxer: &mut AviDemuxer, count: usize) -> Vec<media_types::Packet> {
    let mut packets = Vec::new();
    while packets.len() < count {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => packets.push(packet),
            Ok(DemuxEvent::EndOfStream) => unreachable!("needed {count} packets"),
            Ok(_) => {}
            Err(err) => unreachable!("clean drain: {err}"),
        }
    }
    packets
}

#[test]
fn reanchor_on_interleaved_streams_keeps_both_ladders_true() {
    // Video (30 fps) and audio (48 kHz, 4-byte frames) interleaved:
    // both ladders must continue with the from-start ticks.
    let chunks: Vec<MoviChunk<'_>> = (0..6)
        .flat_map(|index| {
            let video: MoviChunk<'_> = (b"00dc", &b"v"[..]);
            let audio: MoviChunk<'_> = (b"01wb", &b"aaaa"[..]);
            [video, audio].map(|chunk| {
                let _ = index;
                chunk
            })
        })
        .collect();
    let streams = [video_stream_of(6), audio_stream()];
    let truth = from_start_truth(&streams, &chunks, true);

    let built = build(&streams, &chunks, true);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // Consume v0..v1 / a0..a1 (4 packets), then re-anchor to the
    // video tick 2 (66.67 ms) — the landing is the v2 chunk.
    let _ = drain_through(&mut avi, 4);
    let landing = match avi.reanchor(span_at_nanos(66_666_667)) {
        Ok(span) => span,
        Err(err) => unreachable!("reanchor: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 2, "video landing at tick 2"),
        Err(_) => unreachable!("landing start present"),
    }
    let resumed = drain_packets(&mut avi);
    let resumed_truth: Vec<(u32, i64)> = resumed
        .into_iter()
        .map(|packet| match packet.pts {
            media_types::time::Stamp::Present(ts) => (packet.track.handle(), ts.ticks),
            media_types::time::Stamp::Missing => unreachable!("D1"),
        })
        .collect();
    // The truth tail from the v2 chunk onward: v2, a2, v3, a3, v4, a4, v5, a5.
    let expected: Vec<(u32, i64)> = truth[4..].to_vec();
    assert_eq!(resumed_truth, expected);
    assert_eq!(resumed_truth.first().copied(), Some((0, 2)));
}

#[test]
fn reanchor_without_idx1_scans_and_continues() {
    let chunks: Vec<MoviChunk<'_>> = (0..3).map(|_| (b"00dc", &b"x"[..])).collect();
    let streams = [video_stream_of(3)];
    let truth = from_start_truth(&streams, &chunks, false);
    let built = build(&streams, &chunks, false);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let _ = drain_through(&mut avi, 1);
    // 50 ms → tick 1.5 → the last chunk with pts ≤ 2 is tick 2.
    let landing = match avi.reanchor(span_at_nanos(50_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("reanchor (scan): {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 2),
        Err(_) => unreachable!("landing start present"),
    }
    let resumed = drain_packets(&mut avi);
    let resumed_truth: Vec<(u32, i64)> = resumed
        .into_iter()
        .map(|packet| match packet.pts {
            media_types::time::Stamp::Present(ts) => (packet.track.handle(), ts.ticks),
            media_types::time::Stamp::Missing => unreachable!("D1"),
        })
        .collect();
    assert_eq!(resumed_truth, vec![truth[2]]);
}

#[test]
fn reanchor_failure_leaves_the_demuxer_at_its_position_d4() {
    let chunks: Vec<MoviChunk<'_>> = (0..3).map(|_| (b"00dc", &b"x"[..])).collect();
    let streams = [video_stream_of(3)];
    let truth = from_start_truth(&streams, &chunks, true);
    let built = build(&streams, &chunks, true);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let _ = drain_through(&mut avi, 1);
    // Past the declared end: the typed refusal, with nothing moved.
    let err = match avi.reanchor(span_at_nanos(10_000_000_000)) {
        Ok(_) => unreachable!("past-end reanchor is typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.seek.out-of-range");
    // D4: the demuxer continues from its pre-seek position — the
    // remaining packets arrive with their original timestamps.
    let resumed = drain_packets(&mut avi);
    let resumed_truth: Vec<(u32, i64)> = resumed
        .into_iter()
        .map(|packet| match packet.pts {
            media_types::time::Stamp::Present(ts) => (packet.track.handle(), ts.ticks),
            media_types::time::Stamp::Missing => unreachable!("D1"),
        })
        .collect();
    assert_eq!(resumed_truth, truth[1..].to_vec());
}

#[test]
fn reanchor_from_eof_revives_the_stream() {
    let chunks: Vec<MoviChunk<'_>> = (0..3).map(|_| (b"00dc", &b"x"[..])).collect();
    let streams = [video_stream_of(3)];
    let built = build(&streams, &chunks, true);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    assert_eq!(drain_packets(&mut avi).len(), 3);
    match avi.next_event() {
        Ok(DemuxEvent::EndOfStream) => {}
        other => unreachable!("EOS after drain, got {other:?}"),
    }
    // A backward seek must revive the exhausted stream.
    let landing = match avi.reanchor(span_at_nanos(0)) {
        Ok(span) => span,
        Err(err) => unreachable!("reanchor from EOS: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 0),
        Err(_) => unreachable!("landing start present"),
    }
    assert_eq!(drain_packets(&mut avi).len(), 3, "the stream plays again");
}

#[test]
fn reanchor_clears_pending_events_stale_packets_never_reappear() {
    // An unknown chunk inside the movi queues a `Recovered` damage
    // record (and reorders the next packet behind it); a re-anchor
    // past that point must drop both — pre-seek events never
    // reappear (doc 03 §5's stale rule applied at the demuxer).
    let built = build(
        &[video_stream_of(3)],
        &[
            (b"00dc", &b"a"[..]),
            (b"JUNK", &b"j"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
        ],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // Packet a, then the damage record (packet b waits behind it in
    // the pending queue — the emit-reorder contract).
    match avi.next_event() {
        Ok(DemuxEvent::Packet(packet)) => assert_eq!(packet.data.as_slice(), b"a"),
        other => unreachable!("first packet, got {other:?}"),
    }
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { .. }) => {}
        other => unreachable!("damage record, got {other:?}"),
    }
    // Re-anchor to tick 2 (the `c` frame): the pending packet `b`
    // (old position) must be dropped, not delivered after the seek.
    let landing = match avi.reanchor(span_at_nanos(66_666_667)) {
        Ok(span) => span,
        Err(err) => unreachable!("reanchor: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 2),
        Err(_) => unreachable!("landing start present"),
    }
    let resumed = drain_packets(&mut avi);
    assert_eq!(resumed.len(), 1, "only frame c remains");
    assert_eq!(resumed[0].data.as_slice(), b"c");
    match resumed[0].pts {
        media_types::time::Stamp::Present(ts) => assert_eq!(ts.ticks, 2),
        media_types::time::Stamp::Missing => unreachable!("D1"),
    }
}

#[test]
fn reanchor_at_end_lands_last_and_past_end_is_typed() {
    let chunks: Vec<MoviChunk<'_>> = (0..4).map(|_| (b"00dc", &b"x"[..])).collect();
    let streams = [video_stream_of(4)];
    let built = build(&streams, &chunks, true);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // At the declared end (tick 4 = 133.33 ms): the last keyframe at
    // or below it is frame 3 (pts 3 ≤ 4).
    let landing = match avi.reanchor(span_at_nanos(133_333_334)) {
        Ok(span) => span,
        Err(err) => unreachable!("at-end reanchor: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 3, "at-end lands the last keyframe"),
        Err(_) => unreachable!("landing start present"),
    }
    // Past the declared end: typed refusal, nothing moved.
    let err = match avi.reanchor(span_at_nanos(200_000_000)) {
        Ok(_) => unreachable!("past-end is typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.seek.out-of-range");
}

#[test]
fn reanchor_landing_agrees_with_the_pure_query() {
    let chunks: Vec<MoviChunk<'_>> = (0..4).map(|_| (b"00dc", &b"x"[..])).collect();
    let streams = [video_stream_of(4)];
    let built = build(&streams, &chunks, true);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let queried = match avi.seek(span_at_nanos(70_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("query: {err}"),
    };
    let landed = match avi.reanchor(span_at_nanos(70_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("reanchor: {err}"),
    };
    let (qa, la) = match (queried.start.timestamp(), landed.start.timestamp()) {
        (Ok(a), Ok(b)) => (a, b),
        _ => unreachable!("both present"),
    };
    assert_eq!(qa.ticks, la.ticks);
    assert_eq!(qa.base, la.base);
}

#[test]
fn slow_linear_scan_is_bounded_and_typed() {
    // 1002 unindexed video packets: a far target needs more than the
    // doc 03 §6 SlowLinear budget (1000 packets) — typed TooSlow with
    // the position preserved (D4). The pure query stays unbounded
    // (the landed 2.B2 behavior).
    let chunks: Vec<MoviChunk<'_>> = (0..1002).map(|_| (b"00dc", &b"x"[..])).collect();
    let streams = [video_stream_of(1002)];
    let built = build(&streams, &chunks, false);
    let mut avi = demuxer_over(built.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    // A near target stays inside the budget and re-anchors fine.
    match avi.reanchor(span_at_nanos(100_000_000)) {
        Ok(_) => {}
        Err(err) => unreachable!("near target within budget: {err}"),
    }
    // Back to the start, then a far target: the scan must refuse.
    match avi.reanchor(span_at_nanos(0)) {
        Ok(_) => {}
        Err(err) => unreachable!("restart: {err}"),
    }
    // 33.4 s == the declared end tick (1002): in range, but the scan
    // must walk every packet chunk to answer it — past the budget.
    let err = match avi.reanchor(span_at_nanos(33_400_000_000)) {
        Ok(_) => unreachable!("far target exceeds the scan budget"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.seek.too-slow");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::ResourceLimit { .. }
    ));
    // D4: the demuxer is still at its position (tick 0) and playable.
    let packet = match avi.next_event() {
        Ok(DemuxEvent::Packet(packet)) => packet,
        other => unreachable!("still playable, got {other:?}"),
    };
    match packet.pts {
        media_types::time::Stamp::Present(ts) => assert_eq!(ts.ticks, 0),
        media_types::time::Stamp::Missing => unreachable!("D1"),
    }
    // The pure query is unbounded (landed behavior, unchanged): the
    // same end-target the capped reanchor refused answers fine —
    // walking all 1002 packets is the query's prerogative.
    let end_query = match avi.seek(span_at_nanos(33_400_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("the pure query is unbounded: {err}"),
    };
    match end_query.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 1001, "the last packet at/below the end"),
        Err(_) => unreachable!("landing start present"),
    }
    // Past the declared end the range check still refuses.
    match avi.seek(span_at_nanos(60_000_000_000)) {
        Ok(_) => unreachable!("past-end is typed"),
        Err(err) => assert_eq!(err.context.code, "demux.avi.seek.out-of-range"),
    }
}

#[test]
fn seekability_reports_the_container_shape() {
    let chunks: Vec<MoviChunk<'_>> = (0..2).map(|_| (b"00dc", &b"x"[..])).collect();
    let streams = [video_stream_of(2)];
    let indexed = build(&streams, &chunks, true);
    let mut avi = demuxer_over(indexed.bytes);
    assert_eq!(avi.seekability(), SeekSupport::None, "not started");
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    assert_eq!(
        avi.seekability(),
        SeekSupport::Fast { indexed: true },
        "idx1 present"
    );

    let unindexed = build(&streams, &chunks, false);
    let mut avi = demuxer_over(unindexed.bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    assert_eq!(
        avi.seekability(),
        SeekSupport::SlowLinear,
        "no idx1, seekable source"
    );
}
