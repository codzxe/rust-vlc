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

fn strf_video(width: u32, height: i32) -> Vec<u8> {
    let mut v = Vec::with_capacity(40);
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&width.to_le_bytes());
    // biHeight is a SIGNED field: negative = bottom-up DIB.
    v.extend_from_slice(&height.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(b"MJPG");
    v.extend_from_slice(&[0u8; 20]);
    v
}

fn strf_pcm(channels: u16, rate: u32, block_align: u16, bits: u16) -> Vec<u8> {
    strf_wave(1, channels, rate, block_align, bits) // WAVE_FORMAT_PCM
}

/// A raw WAVEFORMATEX `strf` with an explicit format tag (1 = PCM,
/// 3 = IEEE float, anything else = adversarial).
fn strf_wave(tag: u16, channels: u16, rate: u32, block_align: u16, bits: u16) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&tag.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * u32::from(channels) * u32::from(bits / 8)).to_le_bytes());
    v.extend_from_slice(&block_align.to_le_bytes());
    v.extend_from_slice(&bits.to_le_bytes());
    v
}

/// `KSDATAFORMAT_SUBTYPE_PCM` / `..._IEEE_FLOAT` (the SubFormat GUIDs
/// WAVEFORMATEXTENSIBLE carries at byte 24 — byte-for-byte what
/// ffmpeg's AVI muxer writes for multi-channel PCM).
const GUID_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];
const GUID_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

/// The 40-byte WAVEFORMATEXTENSIBLE `strf` ffmpeg writes: 18-byte
/// WAVEFORMATEX (tag `0xFFFE`, `cbSize` 22) + valid bits + channel
/// mask + SubFormat GUID. `valid_bits` and `mask` mirror real muxer
/// output (container width; a plain channel-mask bitmap).
fn strf_extensible(
    guid: [u8; 16],
    channels: u16,
    rate: u32,
    bits: u16,
    valid_bits: u16,
    mask: u32,
) -> Vec<u8> {
    let block_align = channels * (bits / 8);
    let mut v = strf_wave(0xFFFE, channels, rate, block_align, bits);
    v.extend_from_slice(&22u16.to_le_bytes()); // cbSize
    v.extend_from_slice(&valid_bits.to_le_bytes());
    v.extend_from_slice(&mask.to_le_bytes());
    v.extend_from_slice(&guid);
    v
}

/// The streams a built file carries.
enum Stream {
    Video {
        scale: u32,
        rate: u32,
        length: u32,
        width: u32,
        /// Signed `biHeight` (negative exercises the bottom-up DIB
        /// acceptance).
        height: i32,
    },
    /// A video stream with raw `strf` bytes (degenerate-format
    /// adversarial cases: short strf, zero dimensions, …).
    VideoRaw {
        scale: u32,
        rate: u32,
        length: u32,
        strf: Vec<u8>,
    },
    Audio {
        scale: u32,
        rate: u32,
        length: u32,
        ssize: u32,
        strf: Vec<u8>,
    },
    /// A strl whose `strh` chunk is shorter than the 56-byte header.
    ShortStrh,
    /// A real-but-unsupported stream (txts/mids/mjpa/…): strl
    /// present, outside the v0 codec filter.
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
            Stream::VideoRaw {
                scale,
                rate,
                length,
                ref strf,
            } => {
                chunk(
                    &mut out,
                    b"strh",
                    &strh(b"vids", b"MJPG", scale, rate, length, 0),
                );
                chunk(&mut out, b"strf", strf);
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
            Stream::ShortStrh => {
                // A strh declared (and written) one byte short.
                chunk(
                    &mut out,
                    b"strh",
                    &strh(b"vids", b"MJPG", 1, 30, 1, 0)[..55],
                );
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
            sample: header::PcmSample::S16Le,
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
    chunk(&mut out, b"strn", b"Lumen test stream\x00"); // lumen-allow-literal: test fixture value — round-trip data, not display identity
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
    assert_eq!(tracks[0].title.as_deref(), Some("Lumen test stream")); // lumen-allow-literal: test fixture value — round-trip data, not display identity
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

// ---------------------------------------------------------------------------
// Per-stream containment (D-3/D-9: a degenerate strl skips its stream,
// never the file — Lead decision ED-e; the old abort behavior was the
// audited violation M6)
// ---------------------------------------------------------------------------

#[test]
fn zero_rate_or_scale_skips_the_stream_not_the_file() {
    // The first strl carries a zero dwScale (no time base possible);
    // the second is a healthy video stream. The file must open with
    // the surviving stream and a damage record for the skipped one.
    let built = build(
        &[
            Stream::Video {
                scale: 0,
                rate: 30,
                length: 1,
                width: 64,
                height: 48,
            },
            video_stream(),
        ],
        &[(b"01dc", &b"x"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("a bad sibling stream must not kill the file: {err}"),
    };
    assert_eq!(tracks.len(), 1, "only the healthy stream survives");
    assert_eq!(
        tracks[0].id.handle(),
        1,
        "ordinals stay aligned with strl order"
    );
    // The skip is observable first.
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 0, "a decision, not a byte range");
            assert_eq!(reason, "zero-rate-or-scale");
        }
        other => unreachable!("damage surfaces first, got {other:?}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].track.handle(), 1);
}

#[test]
fn all_streams_degenerate_is_typed_no_supported_streams() {
    // Every stream bad → the honest typed refusal (not a fabricated
    // empty session, and not a per-stream error masquerading as one).
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
        Ok(_) => unreachable!("all-bad streams must refuse"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.no-supported-streams");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::Unsupported { .. }
    ));
    assert_eq!(err.stage, media_types::PipelineStage::Demux);
}

#[test]
fn negative_biheight_accepts_bottom_up_dib_with_damage() {
    // biHeight = -480: the bottom-up DIB convention (ubiquitous in
    // real-world files) — the stream is accepted with height 480 and a
    // damage note recording the flip.
    let built = build(
        &[Stream::Video {
            scale: 1,
            rate: 30,
            length: 1,
            width: 320,
            height: -480,
        }],
        &[(b"00dc", &b"x"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("bottom-up DIB must be accepted: {err}"),
    };
    assert_eq!(tracks.len(), 1);
    assert_eq!(
        avi.stream_format(tracks[0].id),
        Some(header::StreamFormat::MotionJpeg {
            width: 320,
            height: 480
        }),
        "the absolute height is the frame geometry"
    );
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 0);
            assert_eq!(reason, "negative-biheight-bottom-up");
        }
        other => unreachable!("the flip note surfaces first, got {other:?}"),
    }
    assert_eq!(drain_packets(&mut avi).len(), 1);
}

#[test]
fn short_strh_skips_the_stream_siblings_survive() {
    let built = build(
        &[Stream::ShortStrh, audio_stream()],
        &[(b"01wb", &b"abcd"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("containment: {err}"),
    };
    assert_eq!(tracks.len(), 1, "the audio stream survives");
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(reason, "strh-too-short");
            assert_eq!(skipped, 0);
        }
        other => unreachable!("damage first, got {other:?}"),
    }
    assert_eq!(drain_packets(&mut avi).len(), 1);
}

#[test]
fn short_strf_skips_the_stream_siblings_survive() {
    // A 39-byte video strf (BITMAPINFOHEADER needs 40) and a 15-byte
    // audio strf (WAVEFORMATEX needs 16): both streams skipped with
    // damage, a healthy sibling survives each.
    let short_video_strf = strf_video(64, 48)[..39].to_vec();
    let short_audio_strf = strf_pcm(2, 48_000, 4, 16)[..15].to_vec();
    for (streams, expected_track) in [
        (
            vec![
                Stream::VideoRaw {
                    scale: 1,
                    rate: 30,
                    length: 1,
                    strf: short_video_strf.clone(),
                },
                audio_stream(),
            ],
            1u32,
        ),
        (
            vec![
                video_stream(),
                Stream::Audio {
                    scale: 1,
                    rate: 48_000,
                    length: 4_000,
                    ssize: 4,
                    strf: short_audio_strf.clone(),
                },
            ],
            0u32,
        ),
    ] {
        let built = build(
            &streams,
            &[(b"00dc", &b"x"[..]), (b"01wb", &b"abcd"[..])],
            false,
        );
        let mut avi = demuxer_over(built.bytes);
        let tracks = match avi.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("containment: {err}"),
        };
        assert_eq!(tracks.len(), 1, "the healthy sibling survives");
        assert_eq!(tracks[0].id.handle(), expected_track);
        match avi.next_event() {
            Ok(DemuxEvent::Recovered { skipped, reason }) => {
                assert_eq!(reason, "strf-too-short");
                assert_eq!(skipped, 0);
            }
            other => unreachable!("damage first, got {other:?}"),
        }
        let packets = drain_packets(&mut avi);
        assert!(
            packets.iter().all(|p| p.track.handle() == expected_track),
            "only the surviving stream's chunks deliver"
        );
    }
}

#[test]
fn channels_over_ceiling_skips_the_stream_siblings_survive() {
    // nChannels = 0xFFFF: corruption, not a layout — skipped with
    // damage, never fabricated (V11).
    let built = build(
        &[
            video_stream(),
            Stream::Audio {
                scale: 1,
                rate: 1_000,
                length: 100,
                ssize: 4,
                strf: strf_pcm(65535, 1_000, 4, 16),
            },
        ],
        &[(b"00dc", &b"x"[..]), (b"01wb", &b"abcd"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("containment: {err}"),
    };
    assert_eq!(tracks.len(), 1, "the video stream survives");
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(reason, "audio-channels-over-ceiling");
            assert_eq!(skipped, 0);
        }
        other => unreachable!("damage first, got {other:?}"),
    }
    let packets = drain_packets(&mut avi);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].track.handle(), 0);
}

// ---------------------------------------------------------------------------
// The WAVE PCM family: tag/width matrix, WAVEFORMATEXTENSIBLE, refusals.
// (Registered follow-up "pcm stream-recognition gap": the randomized
// corpus showed ffmpeg's AVI muxer emitting 0xFFFE extensible strf for
// most multi-channel PCM, and the float tags carrying a descriptor that
// lied about signedness.)
// ---------------------------------------------------------------------------

/// Drive one built AVI and return (tracks, format of the audio track,
/// all events' damage reasons in order, packets).
#[allow(clippy::type_complexity)]
fn start_and_inspect(
    built: BuiltAvi,
) -> (
    Vec<media_types::Track>,
    Option<header::StreamFormat>,
    Vec<&'static str>,
    Vec<media_types::Packet>,
) {
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("containment: {err}"),
    };
    let audio_format = tracks
        .iter()
        .find(|t| matches!(t.kind, MediaKind::Audio))
        .and_then(|t| avi.stream_format(t.id));
    let mut reasons = Vec::new();
    let mut packets = Vec::new();
    loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(p)) => packets.push(p),
            Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("clean drain: {err}"),
        }
    }
    (tracks, audio_format, reasons, packets)
}

/// One audio stream spec plus a movi payload of four whole
/// sample-frames in that stream's width (the payload is returned so
/// the caller can borrow it into `build`).
fn audio_case(strf: Vec<u8>, rate: u32, ssize: u32) -> (Stream, Vec<u8>) {
    let payload: Vec<u8> = (0..(ssize as usize) * 4)
        .map(|i| (0x30 + (i % 10)) as u8)
        .collect();
    (
        Stream::Audio {
            scale: 1,
            rate,
            length: 4,
            ssize,
            strf,
        },
        payload,
    )
}

#[test]
fn pcm_wave_tag_matrix_maps_every_width_and_kind() {
    // The plain-tag family: tag 1 (int) 8/16/24/32-bit and tag 3
    // (IEEE float) 32/64-bit — each recognized, described with the
    // registry vocabulary, and packet-extracted with the sample
    // ladder (PTS advances by whole sample-frames).
    for (tag, bits, sample, descriptor) in [
        (1u16, 8u16, header::PcmSample::U8, "pcm_u8"),
        (1, 16, header::PcmSample::S16Le, "pcm_s16le"),
        (1, 24, header::PcmSample::S24Le, "pcm_s24le"),
        (1, 32, header::PcmSample::S32Le, "pcm_s32le"),
        (3, 32, header::PcmSample::F32Le, "pcm_f32le"),
        (3, 64, header::PcmSample::F64Le, "pcm_f64le"),
    ] {
        let bytes_per_sample = bits / 8;
        let (stream, payload) = audio_case(
            strf_wave(tag, 2, 8_000, 2 * bytes_per_sample, bits),
            8_000,
            u32::from(2 * bytes_per_sample),
        );
        let built = build(
            &[stream],
            &[(b"00wb", payload.as_slice()), (b"00wb", payload.as_slice())],
            false,
        );
        let (tracks, format, reasons, packets) = start_and_inspect(built);
        assert_eq!(tracks.len(), 1, "tag {tag} bits {bits}: track kept");
        assert_eq!(tracks[0].codec.as_str(), descriptor);
        assert_eq!(
            format,
            Some(header::StreamFormat::Pcm {
                channels: 2,
                samples_per_sec: 8_000,
                sample,
                block_align: 2 * (bits / 8)
            }),
            "tag {tag} bits {bits}"
        );
        assert!(reasons.is_empty(), "tag {tag} bits {bits}: no damage");
        assert_eq!(packets.len(), 2);
        // PTS is each chunk's ladder position (start, AVI convention):
        // 0, then 4 sample-frames later on the 1/8000 base.
        let pts: Vec<i64> = packets
            .iter()
            .map(|p| match &p.pts {
                media_types::Stamp::Present(ts) => ts.ticks,
                media_types::Stamp::Missing => unreachable!("ladder stamps by construction"),
            })
            .collect();
        assert_eq!(pts, vec![0, 4], "tag {tag} bits {bits}");
    }
}

#[test]
fn wave_format_extensible_resolves_subformat_guids() {
    // The 40-byte WAVEFORMATEXTENSIBLE ffmpeg's AVI muxer writes for
    // multi-channel PCM: int and float GUIDs both resolve to the same
    // vocabulary the plain tags map to (the corpus's s24le 8-channel
    // and f32le 4-channel shapes, reproduced byte-level).
    for (guid, bits, sample, descriptor, channels, mask) in [
        (
            GUID_PCM,
            24u16,
            header::PcmSample::S24Le,
            "pcm_s24le",
            8u16,
            0x63f,
        ),
        (GUID_PCM, 8, header::PcmSample::U8, "pcm_u8", 4, 0x107),
        (GUID_PCM, 16, header::PcmSample::S16Le, "pcm_s16le", 1, 0x4),
        (
            GUID_IEEE_FLOAT,
            32,
            header::PcmSample::F32Le,
            "pcm_f32le",
            4,
            0x107,
        ),
        (
            GUID_IEEE_FLOAT,
            64,
            header::PcmSample::F64Le,
            "pcm_f64le",
            2,
            0x3,
        ),
    ] {
        let block_align = channels * (bits / 8);
        let (stream, payload) = audio_case(
            strf_extensible(guid, channels, 11_025, bits, bits, mask),
            11_025,
            u32::from(block_align),
        );
        let built = build(
            &[stream],
            &[(b"00wb", payload.as_slice()), (b"00wb", payload.as_slice())],
            false,
        );
        let (tracks, format, reasons, packets) = start_and_inspect(built);
        assert_eq!(tracks.len(), 1, "extensible bits {bits} ch {channels}");
        assert_eq!(tracks[0].codec.as_str(), descriptor);
        assert_eq!(
            format,
            Some(header::StreamFormat::Pcm {
                channels,
                samples_per_sec: 11_025,
                sample,
                block_align
            }),
            "extensible bits {bits} ch {channels}"
        );
        assert!(
            reasons.is_empty(),
            "extensible bits {bits} ch {channels}: no damage"
        );
        assert_eq!(packets.len(), 2, "extensible bits {bits} ch {channels}");
    }
}

#[test]
fn impossible_pcm_combinations_skip_the_stream_siblings_survive() {
    // Typed refusals for combinations the WAVE PCM family cannot
    // express — each skips only its stream (D-3 containment) with a
    // distinct observable reason.
    for (strf, reason) in [
        // Tag 1 with 20-bit samples: not a PCM container width.
        (strf_wave(1, 2, 5, 20, 20), "bad-pcm-format"),
        // Tag 3 with 8-bit samples: IEEE float is 32/64-bit only.
        (strf_wave(3, 2, 4, 8, 8), "float-pcm-bit-width-unsupported"),
        // Tag 3 with 16-bit samples.
        (strf_wave(3, 2, 4, 4, 16), "float-pcm-bit-width-unsupported"),
        // A real non-PCM tag (0x0011 IMA ADPCM).
        (strf_wave(0x0011, 2, 4, 1, 4), "unsupported-audio-format"),
        // Extensible whose SubFormat GUID names something else (the
        // μ-law GUID {00000007-...}).
        (
            strf_extensible(
                [
                    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00,
                    0x38, 0x9b, 0x71,
                ],
                2,
                8_000,
                16,
                16,
                0x3,
            ),
            "unsupported-audio-format",
        ),
        // Extensible declared but the chunk too short to carry the
        // GUID (a 16-byte strf with tag 0xFFFE).
        (strf_wave(0xFFFE, 2, 4, 4, 16), "extensible-strf-truncated"),
    ] {
        let built = build(
            &[
                video_stream(),
                Stream::Audio {
                    scale: 1,
                    rate: 8_000,
                    length: 4,
                    ssize: 4,
                    strf,
                },
            ],
            &[(b"00dc", &b"frame"[..]), (b"01wb", &b"abcd"[..])],
            false,
        );
        let mut avi = demuxer_over(built.bytes);
        let tracks = match avi.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("containment: {err}"),
        };
        assert_eq!(tracks.len(), 1, "the video sibling survives {reason}");
        match avi.next_event() {
            Ok(DemuxEvent::Recovered {
                skipped,
                reason: got,
            }) => {
                assert_eq!(got, reason);
                assert_eq!(skipped, 0);
            }
            other => unreachable!("damage first for {reason}, got {other:?}"),
        }
        let packets = drain_packets(&mut avi);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].track.handle(), 0);
    }
}

#[test]
fn float_pcm_without_sample_ladder_skips_the_stream() {
    // dwSampleSize = 0 on a float stream: no honest chunk→sample
    // ladder either — the same typed skip the int path has always
    // taken (previously float skipped this check; tightening it is
    // required for honest PTS on the newly recognized streams).
    let built = build(
        &[
            video_stream(),
            Stream::Audio {
                scale: 1,
                rate: 8_000,
                length: 4,
                ssize: 0,
                strf: strf_wave(3, 1, 8_000, 4, 32),
            },
        ],
        &[(b"00dc", &b"frame"[..]), (b"01wb", &b"abcd"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("containment: {err}"),
    };
    assert_eq!(tracks.len(), 1);
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { reason, .. }) => {
            assert_eq!(reason, "pcm-sample-size-zero");
        }
        other => unreachable!("damage first, got {other:?}"),
    }
}

#[test]
fn block_align_disagreements_are_damage_notes_not_skips() {
    // A block_align that disagrees with channels × width (and a
    // dwSampleSize agreeing with that lie) is real-world damage the
    // stream survives: the strh dwSampleSize remains the ladder, the
    // descriptor remains the truth, and both disagreements surface as
    // damage notes.
    let built = build(
        &[Stream::Audio {
            scale: 1,
            rate: 8_000,
            length: 4,
            ssize: 5,
            strf: strf_wave(1, 2, 8_000, 5, 16),
        }],
        &[(b"00wb", &b"abcdefghij"[..]), (b"00wb", &b"klmnopqrst"[..])],
        false,
    );
    let (tracks, _format, reasons, packets) = start_and_inspect(built);
    assert_eq!(tracks.len(), 1, "the stream survives the lies");
    assert!(reasons.contains(&"block-align-disagrees-with-format"));
    // The stream still delivers; the ladder advances by the strh's
    // dwSampleSize (5), not the lying block_align: 10 bytes = 2
    // sample-frames, so the second chunk lands on tick 2.
    assert_eq!(packets.len(), 2);
    let pts: Vec<i64> = packets
        .iter()
        .map(|p| match &p.pts {
            media_types::Stamp::Present(ts) => ts.ticks,
            media_types::Stamp::Missing => unreachable!("ladder stamps by construction"),
        })
        .collect();
    assert_eq!(pts, vec![0, 2]);
}

#[test]
fn mjpa_handler_is_filtered_like_any_unsupported_handler() {
    // Stage agreement (V8): the probe does not hint mjpeg for mjpa —
    // and the demuxer's codec filter skips the handler exactly like
    // any other unsupported one, with the audio sibling surviving.
    let built = build(
        &[
            Stream::Other {
                fcc: b"vids",
                handler: b"mjpa",
            },
            audio_stream(),
        ],
        &[(b"00dc", &b"x"[..]), (b"01wb", &b"abcd"[..])],
        false,
    );
    let mut avi = demuxer_over(built.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("containment: {err}"),
    };
    assert_eq!(tracks.len(), 1, "only the PCM stream survives");
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { reason, .. }) => {
            assert_eq!(reason, "unsupported-video-handler");
        }
        other => unreachable!("damage first, got {other:?}"),
    }
    assert_eq!(drain_packets(&mut avi).len(), 1);
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
fn rec_list_overrunning_parent_bound_clamps_with_damage() {
    // A `rec ` list whose declared end runs past the movi list (into
    // the idx1 region — V6): clamped to the bound with damage, never a
    // fatal bad-structure; the chunks inside still deliver.
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
    chunk(&mut out, b"strh", &strh(b"vids", b"MJPG", 1, 30, 3, 0));
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
    chunk(&mut out, b"00dc", b"t0");
    // The rec's honest size (form + children), then the lie: 20 bytes
    // past the movi list end (the idx1 that follows makes those bytes
    // real file bytes).
    let rec_size = (out.len() - rec_size_pos - 4) as u32;
    out[rec_size_pos..rec_size_pos + 4].copy_from_slice(&(rec_size + 20).to_le_bytes());
    end_sized(&mut out, movi_size_pos);
    // A minimal idx1 so bytes exist past the movi end.
    chunk(&mut out, b"idx1", &[]);
    end_sized(&mut out, riff_size_pos);

    let mut avi = demuxer_over(out);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("clamped rec is not fatal: {err}"),
    }
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(reason, "rec-extends-past-list-bound");
            assert_eq!(skipped, 20);
        }
        other => unreachable!("the rec clamp surfaces first, got {other:?}"),
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

#[test]
fn lying_idx1_size_clamps_to_available_bytes() {
    // V5: an idx1 declaring 0xFFFFFFF0 bytes with only 3 real entries
    // (48 bytes) in the stream — clamp before the cap check: a small
    // salvaged index plus damage, not a fatal over-cap abort.
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
        ],
        true,
    );
    let mut bytes = built.bytes;
    // The idx1 chunk header sits at the end; its size field is the
    // last chunk header before EOF.
    let idx1_pos = bytes.len() - 8 - 3 * 16;
    assert_eq!(&bytes[idx1_pos..idx1_pos + 4], b"idx1");
    bytes[idx1_pos + 4..idx1_pos + 8].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("the lying size is clamped, not fatal: {err}"),
    }
    // The clamp is damage-counted…
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(reason, "idx1-size-past-available");
            assert_eq!(
                skipped,
                u64::from(0xFFFF_FFF0u32) - 48,
                "declared minus the bytes that exist"
            );
        }
        other => unreachable!("damage first, got {other:?}"),
    }
    // …and the salvaged index still works: Fast seekability, packets
    // flow, an indexed seek lands on the target keyframe.
    assert_eq!(
        avi.seekability(),
        SeekSupport::Fast { indexed: true },
        "the salvaged index is a real index"
    );
    assert_eq!(drain_packets(&mut avi).len(), 3);
}

#[test]
fn absolute_idx1_offsets_are_autodetected_and_seek() {
    // V4 (Lead decision ED-d, FFmpeg behavior): a legacy writer stored
    // absolute file positions in dwOffset. The parser probes the first
    // entry against both bases, accepts the absolute interpretation
    // (the chunk ids at the implied positions match), records the
    // variant as a damage note — and the index seeks correctly.
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
            (b"00dc", &b"b"[..]),
            (b"00dc", &b"c"[..]),
        ],
        true,
    );
    let mut bytes = built.bytes;
    let idx1_pos = bytes.len() - 8 - 3 * 16;
    assert_eq!(&bytes[idx1_pos..idx1_pos + 4], b"idx1");
    // Rewrite every dwOffset as an absolute file position.
    for entry in 0..3 {
        let at = idx1_pos + 8 + entry * 16 + 8;
        let rel = u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        let absolute = u64::from(rel) + built.movi_form_pos;
        bytes[at..at + 4]
            .copy_from_slice(&u32::try_from(absolute).unwrap_or(u32::MAX).to_le_bytes());
    }
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("absolute offsets are a variant, not corruption: {err}"),
    }
    // The variant is recorded, never silent.
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(reason, "idx1-absolute-offsets");
            assert_eq!(skipped, 0, "a decision, not a byte range");
        }
        other => unreachable!("the variant note surfaces first, got {other:?}"),
    }
    // The index is real: Fast seekability and a landing on the
    // keyframe at/below the target (tick 1 for 40 ms).
    assert_eq!(avi.seekability(), SeekSupport::Fast { indexed: true });
    let landing = match avi.seek(span_at_nanos(40_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("indexed seek over absolute offsets: {err}"),
    };
    match landing.start.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 1, "lands on frame 1"),
        Err(_) => unreachable!("landing start present"),
    }
    assert_eq!(drain_packets(&mut avi).len(), 3);
}

#[test]
fn idx1_damage_coalesces_per_reason() {
    // V4c: 1000 entries all pointing outside the movi — ONE damage
    // event for the reason (the byte total carries the count: 16
    // bytes per entry), not 1000 events (the audited 524k-event
    // storm on a 2-hour file).
    const LYING_ENTRIES: usize = 1000;
    let built = build(&[video_stream()], &[(b"00dc", &b"a"[..])], false);
    let mut bytes = built.bytes;
    let mut idx: Vec<u8> = Vec::with_capacity(16 * LYING_ENTRIES);
    for _ in 0..LYING_ENTRIES {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&0x10u32.to_le_bytes());
        // ~1 GiB past the movi: outside any real file's data region.
        idx.extend_from_slice(&0x4000_0000u32.to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());
    }
    let idx1_offset = bytes.len();
    chunk(&mut bytes, b"idx1", &idx);
    let grow = (bytes.len() - idx1_offset) as u32;
    patch_size(&mut bytes, 4, grow);

    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("a lying index never kills the file: {err}"),
    }
    let mut events_for_reason = 0;
    let mut packets = 0;
    let terminal = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(_)) => packets += 1,
            Ok(DemuxEvent::Recovered { skipped, reason }) => {
                if reason == "idx1-entry-outside-movi" {
                    events_for_reason += 1;
                    assert_eq!(
                        skipped,
                        (16 * LYING_ENTRIES) as u64,
                        "the byte total carries the entry count"
                    );
                }
            }
            Ok(DemuxEvent::EndOfStream) => break "eos".to_string(),
            Ok(_) => {}
            Err(err) => break err.context.code.clone(),
        }
    };
    assert_eq!(terminal, "eos");
    assert_eq!(packets, 1, "the movi walk is the packet-extraction truth");
    assert_eq!(
        events_for_reason, 1,
        "one coalesced event regardless of entry count"
    );
    assert_eq!(
        avi.damage_total(),
        (16 * LYING_ENTRIES) as u64,
        "the damage total still counts every dropped entry"
    );
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
    // A cut too short to carry the RIFF magic at all never promised
    // AVI: honestly "not an AVI" (a zero-byte source is refused even
    // earlier, at Open, by the source-capability check).
    for cut in [1usize, 3] {
        let mut avi = demuxer_over(bytes[..cut].to_vec());
        let err = match avi.start() {
            Ok(_) => unreachable!("cut at {cut} must be an error"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.avi.not-avi", "cut {cut}");
    }
    // A cut inside the RIFF head of a file that DOES say RIFF is
    // truncation, not "not an AVI" (V9: the magic is a promise the
    // bytes did not keep).
    for cut in [4usize, 5, 11] {
        let mut avi = demuxer_over(bytes[..cut].to_vec());
        let err = match avi.start() {
            Ok(_) => unreachable!("cut at {cut} must be an error"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.avi.truncated", "cut {cut}");
        match err.kind {
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated { expected, found },
            } => {
                assert_eq!(expected, 12, "cut {cut}: the RIFF head size");
                assert_eq!(found, cut as u64, "cut {cut}");
            }
            other => unreachable!("truncated flavor at cut {cut}, got {other:?}"),
        }
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
fn truncated_movi_payload_delivers_prefix_damage_then_eos() {
    // A cut inside a chunk payload (V3a): the available prefix is
    // delivered, the missing bytes are counted, and the stream ends
    // with EndOfStream — the old sticky-fatal truncation was a
    // whole-file abort for a recoverable cut (Lead decision ED-e).
    let built = build(
        &[video_stream()],
        &[(b"00dc", &b"v0"[..]), (b"00dc", &[0x42; 100])],
        false,
    );
    // Cut inside the second chunk's payload, 40 of 100 bytes in.
    let cut = (built.chunk_positions[1] + 8 + 40) as usize;
    let mut avi = demuxer_over(built.bytes[..cut].to_vec());
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("header intact: {err}"),
    }
    // Drain every event in order (damage records included).
    let mut events = Vec::new();
    loop {
        match avi.next_event() {
            Ok(DemuxEvent::EndOfStream) => {
                events.push("eos".to_string());
                break;
            }
            Ok(DemuxEvent::Packet(packet)) => {
                events.push(format!("packet:{}", packet.data.len()));
            }
            Ok(DemuxEvent::Recovered { skipped, reason }) => {
                events.push(format!("recovered:{skipped}:{reason}"));
            }
            Ok(_) => {}
            Err(err) => unreachable!("containment, not a fatal: {err}"),
        }
    }
    // The parse-time clamps surface first (RIFF and movi both declared
    // more than the cut stream carries)…
    assert_eq!(
        events[0],
        format!(
            "recovered:{}:riff-extends-past-stream",
            built.bytes.len() as u64 - cut as u64
        )
    );
    assert!(
        events[1].starts_with("recovered:") && events[1].ends_with(":movi-extends-past-riff"),
        "movi clamp damage: {:?}",
        events[1]
    );
    // …then the surviving packet, the walk's own clamp record for the
    // cut chunk, the delivered prefix, and the terminal EOS.
    assert_eq!(events[2], "packet:2");
    assert_eq!(events[3], "recovered:60:chunk-payload-past-eof");
    assert_eq!(events[4], "packet:40");
    assert_eq!(events[5], "eos");
    assert_eq!(events.len(), 6);
    // EOS is idempotent afterwards (not sticky-fatal).
    for _ in 0..3 {
        match avi.next_event() {
            Ok(DemuxEvent::EndOfStream) => {}
            other => unreachable!("EOS idempotent, got {other:?}"),
        }
    }
}

#[test]
fn cut_exactly_at_a_chunk_payload_start_is_damage_then_eos() {
    // Zero payload bytes available: nothing to deliver — damage plus
    // the terminal EOS, never a fabricated empty packet.
    let built = build(
        &[video_stream()],
        &[(b"00dc", &b"v0"[..]), (b"00dc", &[0x42; 100])],
        false,
    );
    let cut = (built.chunk_positions[1] + 8) as usize;
    let mut avi = demuxer_over(built.bytes[..cut].to_vec());
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("header intact: {err}"),
    }
    let mut recovered = 0;
    let mut packets = 0;
    let terminal = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(_)) => packets += 1,
            Ok(DemuxEvent::Recovered { skipped, reason }) => {
                if reason == "chunk-payload-past-eof" {
                    assert_eq!(skipped, 100, "the whole declared payload is gone");
                }
                recovered += 1;
            }
            Ok(DemuxEvent::EndOfStream) => break "eos".to_string(),
            Ok(_) => {}
            Err(err) => break err.context.code.clone(),
        }
    };
    assert_eq!(terminal, "eos");
    assert_eq!(packets, 1, "only the first chunk survives");
    assert!(recovered >= 3, "riff + movi clamps and the chunk clamp");
}

#[test]
fn chunk_size_past_movi_bound_clamps_and_continues() {
    // A mid-movi chunk whose declared size runs past the movi list end
    // while the file continues (the idx1 follows — V3b): the payload
    // is clamped to the bound, the clamp is damage-counted, and the
    // walk resumes at the bound — no silent early EOS, no fatal.
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"v0"[..]),
            (b"00dc", &[0x42; 60]),
            (b"00dc", &b"v2"[..]),
        ],
        true,
    );
    let mut bytes = built.bytes;
    // movi list end: right after the last chunk (2-byte payload, even
    // → no pad).
    let movi_end = built.chunk_positions[2] + 8 + 2;
    let liar = built.chunk_positions[1];
    let in_movi = movi_end - (liar + 8);
    // Declare 40 bytes more than the movi holds (into the idx1
    // region, still inside the file).
    let lying_size = in_movi + 40;
    bytes[(liar + 4) as usize..(liar + 8) as usize]
        .copy_from_slice(&u32::try_from(lying_size).unwrap_or(u32::MAX).to_le_bytes());
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("containment: {err}"),
    }
    let mut clamped_packet: Option<usize> = None;
    let mut clamp_damage = 0;
    let terminal = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                if packet.data.as_slice() == b"v0" {
                    assert_eq!(packet.data.len(), 2);
                } else {
                    clamped_packet = Some(packet.data.len());
                }
            }
            Ok(DemuxEvent::Recovered { skipped, reason }) => {
                if reason == "chunk-payload-past-list-bound" {
                    clamp_damage += skipped;
                }
            }
            Ok(DemuxEvent::EndOfStream) => break "eos".to_string(),
            Ok(_) => {}
            Err(err) => break err.context.code.clone(),
        }
    };
    assert_eq!(terminal, "eos", "the clamp is not fatal");
    // The lying chunk delivered exactly the bytes the movi actually
    // holds for it; the 40 over-declared bytes are counted damage; the
    // positionally-swallowed third chunk is lost but observable
    // through the same clamp record.
    assert_eq!(clamped_packet, Some(in_movi as usize));
    assert_eq!(clamp_damage, 40);
}

// ---------------------------------------------------------------------------
// W2-07: clamp + resync of damage-induced cursor jumps past the walk
// bound (the zero-mid corpus class — a garbage size read off
// misaligned bytes must not forfeit the playable tail).
// ---------------------------------------------------------------------------

/// Smash the chunk header at `pos` into the `zero-mid` damage shape:
/// zeroed fourcc + a garbage size (the exact byte shapes W1-03 traced
/// in the two filed corpus siblings).
fn smash_chunk_header(bytes: &mut [u8], pos: u64, garbage_size: u32) {
    let pos = pos as usize;
    bytes[pos..pos + 4].copy_from_slice(&[0, 0, 0, 0]);
    bytes[pos + 4..pos + 8].copy_from_slice(&garbage_size.to_le_bytes());
}

/// The damage ledger a walk can report: `(skipped, reason)` pairs in
/// order, the packet payloads, and the typed terminal.
type DamageLedger = (
    Vec<(u64, &'static str)>,
    Vec<Vec<u8>>,
    Result<(), media_types::EngineError>,
);

/// Drain, capturing the `(skipped, reason)` damage pairs in order and
/// the packet payloads (the resync flavors are pinned by name, not
/// just by count).
fn drain_with_damage(avi: &mut AviDemuxer) -> DamageLedger {
    let mut damage = Vec::new();
    let mut payloads = Vec::new();
    let terminal = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => payloads.push(packet.data.as_slice().to_vec()),
            Ok(DemuxEvent::Recovered { skipped, reason }) => damage.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break Ok(()),
            Err(err) => break Err(err),
        }
    };
    (damage, payloads, terminal)
}

/// The two-stream six-chunk body the resync tests damage: v0, a0,
/// (damaged v1), a1, v2, a2 — the damaged chunk is the exact
/// unclamped-jump-then-valid-tail shape. Returns the chunk positions
/// and the damaged bytes.
fn resync_body(garbage_size: u32) -> (Vec<u64>, Vec<u8>) {
    let built = build(
        &[video_stream(), audio_stream()],
        &[
            (b"00dc", &b"v0"[..]),
            (b"01wb", &[0xAA; 4]),
            (b"00dc", &[0x42; 6]),
            (b"01wb", &[0xBB; 4]),
            (b"00dc", &[0x43; 6]),
            (b"01wb", &[0xCC; 4]),
        ],
        true,
    );
    let mut bytes = built.bytes;
    smash_chunk_header(&mut bytes, built.chunk_positions[2], garbage_size);
    (built.chunk_positions, bytes)
}

#[test]
fn garbage_chunk_jump_past_bound_resyncs_via_idx1() {
    // The 0136 filing's byte shape: after the damaged chunk, the
    // idx1's next entry is the ladder back onto real structure — the
    // tail (a1, v2, a2) is recovered, the loss is damage-recorded,
    // and the walk still ends EOS.
    let garbage_size = 125_437_440u32; // 0x077A_0600, the traced value
    let (chunk_positions, bytes) = resync_body(garbage_size);
    let damaged = chunk_positions[2];
    let landing = chunk_positions[3];
    // The movi list ends right after the last chunk (4-byte payload).
    let movi_end = chunk_positions[5] + 8 + 4;
    let next = damaged + 8 + u64::from(garbage_size); // even size, no pad
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 2),
        Err(err) => unreachable!("start: {err}"),
    }
    let (damage, payloads, terminal) = drain_with_damage(&mut avi);
    assert_eq!(
        damage,
        vec![
            (u64::from(garbage_size), "unknown-chunk-in-movi"),
            (next - movi_end, "chunk-jump-past-list-bound"),
            (landing - damaged, "movi-resync-idx1"),
        ],
        "the refused jump, then the idx1 landing, in the walk's order"
    );
    assert_eq!(
        payloads,
        vec![
            b"v0".to_vec(),
            vec![0xAA; 4],
            vec![0xBB; 4],
            vec![0x43; 6],
            vec![0xCC; 4],
        ],
        "the damaged chunk alone is lost; the tail recovers"
    );
    assert!(terminal.is_ok(), "the resync is not fatal: {terminal:?}");
}

#[test]
fn garbage_chunk_jump_past_bound_resyncs_by_scan_without_idx1() {
    // The same shape with no idx1: the bounded signature scan (the
    // ffmpeg avi_sync equivalent) finds the first chunk header the
    // walk itself would accept — the 0268 filing's garbage value.
    let garbage_size = 4_286_905_213u32; // 0xFFBEFFFF-ish, odd
    let (chunk_positions, mut bytes) = resync_body(garbage_size);
    // Drop the idx1: truncate the file right after the movi list and
    // shrink the RIFF size to match (an honest no-index file).
    let movi_end = (chunk_positions[5] + 8 + 4) as usize;
    bytes.truncate(movi_end);
    // The dropped idx1 chunk: 8 header bytes + 6 16-byte entries.
    patch_size(&mut bytes, 4, (8u32 + 6 * 16).wrapping_neg());
    let damaged = chunk_positions[2];
    let landing = chunk_positions[3];
    let next = damaged + 8 + u64::from(garbage_size) + 1; // odd size, one pad
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 2),
        Err(err) => unreachable!("start: {err}"),
    }
    let (damage, payloads, terminal) = drain_with_damage(&mut avi);
    assert_eq!(
        damage,
        vec![
            (u64::from(garbage_size), "unknown-chunk-in-movi"),
            (next - movi_end as u64, "chunk-jump-past-list-bound"),
            (landing - damaged, "movi-resync-scan"),
        ],
        "the scan landing carries the resync's own reason"
    );
    assert_eq!(
        payloads,
        vec![
            b"v0".to_vec(),
            vec![0xAA; 4],
            vec![0xBB; 4],
            vec![0x43; 6],
            vec![0xCC; 4],
        ]
    );
    assert!(terminal.is_ok(), "recovered walk ends EOS: {terminal:?}");
}

#[test]
fn garbage_chunk_jump_without_landing_is_typed_never_silent_eos() {
    // The damaged chunk is the LAST one and its payload carries no
    // acceptable signature: no idx1 entry follows, the scan window is
    // exhausted — the walk refuses with the typed W2-07 terminal
    // instead of the old silent EOS (which could be forfeiting
    // playable bytes). The intact prefix still delivered.
    let garbage_size = 125_437_440u32;
    let built = build(
        &[video_stream(), audio_stream()],
        &[
            (b"00dc", &b"v0"[..]),
            (b"01wb", &[0xAA; 4]),
            (b"00dc", &[0x42; 6]),
        ],
        true,
    );
    let mut bytes = built.bytes;
    let damaged = built.chunk_positions[2];
    smash_chunk_header(&mut bytes, damaged, garbage_size);
    let movi_end = built.chunk_positions[2] + 8 + 6;
    let next = damaged + 8 + u64::from(garbage_size);
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("start: {err}"),
    }
    let (damage, payloads, terminal) = drain_with_damage(&mut avi);
    assert_eq!(
        payloads,
        vec![b"v0".to_vec(), vec![0xAA; 4]],
        "the undamaged prefix still delivers"
    );
    let err = match terminal {
        Ok(()) => unreachable!("no landing exists — the walk must refuse loudly"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.walk-resync-failed");
    assert!(
        matches!(err.kind, media_types::ErrorKind::Corrupt { .. }),
        "structure corruption flavor: {:?}",
        err.kind
    );
    // The terminal's own damage (the refused jump) is observable via
    // the damage log; like every fatal walk error, the queued notes
    // die with the error instead of surfacing as events — the typed
    // terminal IS the loud record (the pre-existing sticky pattern).
    assert_eq!(
        damage,
        Vec::<(u64, &'static str)>::new(),
        "no events surface past the terminal error"
    );
    assert_eq!(
        avi.damage_total(),
        u64::from(garbage_size) + (next - movi_end),
        "the refused jump distance is counted"
    );
}

#[test]
fn garbage_chunk_jump_inside_rec_hands_back_to_parent() {
    // The damage sits inside a `rec ` list whose remainder carries no
    // landing: the position hands back to the parent walk at the
    // list's end (the damaged list's tail surrendered, counted), and
    // the parent's chunks after the rec still deliver.
    let garbage_size = 125_437_440u32;
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
    // The damaged chunk: physically 8 header + 6 filler bytes.
    let damaged = out.len() as u64;
    chunk(&mut out, &[0, 0, 0, 0], &[0x42; 6]);
    let rec_end = out.len() as u64;
    let garbage = garbage_size.to_le_bytes();
    out[damaged as usize + 4..damaged as usize + 8].copy_from_slice(&garbage);
    end_sized(&mut out, rec_size_pos);
    chunk(&mut out, b"00dc", b"t0");
    end_sized(&mut out, movi_size_pos);
    end_sized(&mut out, riff_size_pos);

    let mut avi = demuxer_over(out);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("start: {err}"),
    }
    let (damage, payloads, terminal) = drain_with_damage(&mut avi);
    let next = damaged + 8 + u64::from(garbage_size); // even, no pad
    assert_eq!(
        damage,
        vec![
            (u64::from(garbage_size), "unknown-chunk-in-movi"),
            (next - rec_end, "chunk-jump-past-list-bound"),
            (rec_end - damaged, "movi-resync-parent"),
        ],
        "the rec's damaged tail is surrendered to the parent walk"
    );
    assert_eq!(payloads, vec![b"r0".to_vec(), b"t0".to_vec()]);
    assert!(terminal.is_ok(), "recovered walk ends EOS: {terminal:?}");
}

#[test]
fn resync_budget_exhaustion_is_the_typed_terminal_not_an_endless_walk() {
    // A corruption-dense `movi`: MORE damaged cursor jumps past the
    // bound than the MAX_WALK_RESYNCS budget (33 vs 32). The first 32
    // land via the idx1 ladder (the walk keeps recovering, exactly
    // the zero-mid shape repeated); the 33rd refuses with the typed
    // W2-07 terminal — the budget is what bounds the walk's
    // worst-case work on adversarial files (D-3: bounded damage,
    // never an endless skip).
    let garbage_size = 125_437_440u32;
    // c0 valid; c1..=c33 damaged (33 chunks); c34..=c36 a valid tail
    // the refused walk must never reach. Every payload is 4 bytes, so
    // every chunk occupies exactly 12 bytes (8 header + 4 payload,
    // even — no pad).
    let mut specs: Vec<MoviChunk<'_>> = Vec::with_capacity(37);
    specs.push((b"00dc", &b"head"[..]));
    for _ in 0..33 {
        specs.push((b"00dc", &b"gggg"[..]));
    }
    for _ in 0..3 {
        specs.push((b"00dc", &b"tail"[..]));
    }
    let built = build(&[video_stream()], &specs, true);
    let mut bytes = built.bytes;
    for &pos in &built.chunk_positions[1..=33] {
        smash_chunk_header(&mut bytes, pos, garbage_size);
    }
    let mut avi = demuxer_over(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("start: {err}"),
    }
    let (damage, payloads, terminal) = drain_with_damage(&mut avi);
    assert_eq!(
        payloads,
        vec![b"head".to_vec()],
        "the damaged run delivers nothing; the tail stays unreached"
    );
    let err = match terminal {
        Ok(()) => unreachable!("the budget is spent — the walk must refuse loudly"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.walk-resync-failed");
    assert!(
        matches!(err.kind, media_types::ErrorKind::Corrupt { .. }),
        "structure corruption flavor: {:?}",
        err.kind
    );
    // The per-resync `Recovered` records die pending with the terminal
    // error (the pre-existing sticky pattern — the typed terminal IS
    // the loud record; same shape the no-landing test pins).
    assert!(
        damage.is_empty(),
        "no events surface past the terminal error: {damage:?}"
    );
    // The budget proof is the exact damage ledger. Per damaged chunk
    // k = 1..=33 (positions 12 bytes apart, the bound at chunk 36's
    // end): the garbage size, the refused jump distance (12k + G -
    // 436), and — for the MAX_WALK_RESYNCS landings that happened
    // before the 33rd refusal — the 12-byte span to the next chunk:
    //   33*G + (33*G - 7656) + 32*12  ==  66*G - 7272
    // One landing more (budget bypassed) or fewer (early refusal)
    // shifts the total by the 12-byte span — the arithmetic pins the
    // budget as spent to exactly MAX_WALK_RESYNCS.
    assert_eq!(
        avi.damage_total(),
        66 * u64::from(garbage_size) - 7272,
        "every refusal and every landing is counted"
    );
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
