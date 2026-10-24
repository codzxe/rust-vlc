//! Unit tests for the MP4/MOV demuxer (card 5.C1) — in-memory
//! fixtures from [`super::test_support`], every error path pinned to
//! its typed code (no panics, no unwrap-driven assertions on
//! demuxer output).

use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use super::test_support::{
    SampleSpec, TrackSpec, chapter_track, fragmented, progressive, progressive_with_chapters,
    progressive_with_cover,
};
use super::{Mp4Demuxer, probe};

fn video_track(n: usize, dur: u64, syncs: &[bool], cts: &[i64]) -> TrackSpec {
    TrackSpec {
        handler: *b"vide",
        entry: *b"avc1",
        timescale: 9000,
        edit: None,
        chap_ref: None,
        samples: (0..n)
            .map(|i| SampleSpec {
                bytes: vec![0x10 + i as u8; 4],
                dur,
                sync: syncs.get(i).copied().unwrap_or(true),
                cts: cts.get(i).copied().unwrap_or(0),
            })
            .collect(),
    }
}

fn audio_track(n: usize, dur: u64) -> TrackSpec {
    TrackSpec {
        handler: *b"soun",
        entry: *b"mp4a",
        timescale: 44_100,
        edit: None,
        chap_ref: None,
        samples: (0..n)
            .map(|i| SampleSpec {
                bytes: vec![0xA0 + i as u8; 2],
                dur,
                sync: true,
                cts: 0,
            })
            .collect(),
    }
}

fn open(bytes: &[u8]) -> Mp4Demuxer {
    let stream = super::test_support::mem_stream(bytes.to_vec())
        .unwrap_or_else(|e| unreachable!("mem stream: {e}"));
    Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("construct: {e}"))
}

fn drain(demuxer: &mut Mp4Demuxer) -> (usize, Vec<media_types::Packet>) {
    let mut packets = Vec::new();
    let mut events = 0;
    while let Ok(event) = demuxer.next_event() {
        events += 1;
        match event {
            DemuxEvent::Packet(p) => packets.push(p),
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
        assert!(events < 10_000, "event budget");
    }
    (events, packets)
}

trait ErrOf<T> {
    fn err_of(self, what: &str) -> media_types::EngineError;
}

impl<T> ErrOf<T> for Result<T, media_types::EngineError> {
    fn err_of(self, what: &str) -> media_types::EngineError {
        match self {
            Err(err) => err,
            Ok(_) => unreachable!("{what}: a typed error was required"),
        }
    }
}

fn code_of(err: &media_types::EngineError) -> &str {
    err.context.code.as_str()
}

// ---- probe ------------------------------------------------------

#[test]
fn probe_accepts_known_brands_and_qt_heads() {
    let ftyp = |major: &[u8; 4], compat: &[&[u8; 4]]| {
        let mut v = Vec::new();
        v.extend_from_slice(&((8 + 8 + 4 * (compat.len() + 1)) as u32).to_be_bytes());
        v.extend_from_slice(b"ftyp");
        v.extend_from_slice(major);
        v.extend_from_slice(&0u32.to_be_bytes());
        v.extend_from_slice(major);
        for c in compat {
            v.extend_from_slice(*c);
        }
        v
    };
    assert!(probe(&ftyp(b"isom", &[b"iso2", b"mp41"])));
    assert!(probe(&ftyp(b"mp42", &[b"isom"])));
    assert!(probe(&ftyp(b"qt  ", &[b"qt  "])));
    assert!(probe(&ftyp(b"3gp5", &[b"3gp4"])));
    assert!(probe(&ftyp(b"zzzz", &[b"isom"]))); // compat brand known
    assert!(probe(b"\x00\x00\x00\x08moov")); // pre-ftyp QuickTime
    assert!(!probe(&ftyp(b"zzzz", &[b"zzzz"]))); // no known brand
    assert!(!probe(b"\x00\x00\x00\x08ftyp")); // brands cut short
    assert!(!probe(&[0, 1, 2]));
}

// ---- progressive ------------------------------------------------

#[test]
fn progressive_single_track_demuxes_samples_in_order() {
    let track = video_track(4, 3000, &[true, false, false, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].codec.as_str(), "h264");
    assert!(!mp4.is_fragmented());
    assert_eq!(mp4.seekability(), SeekSupport::Fast { indexed: true });
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 4);
    // DTS ladder from stts deltas.
    for (i, p) in packets.iter().enumerate() {
        let expect = Timestamp::new(
            (i as u64 * 3000) as i64,
            TimeBase::new(1, 9000).unwrap_or_else(|e| unreachable!("base: {e}")),
        );
        match &p.dts {
            Stamp::Present(ts) => assert_eq!(ts.ticks, expect.ticks, "dts {i}"),
            Stamp::Missing => unreachable!("dts missing"),
        }
    }
    // Keyframes follow stss.
    assert!(packets[0].keyframe);
    assert!(!packets[1].keyframe);
    assert!(!packets[2].keyframe);
    assert!(packets[3].keyframe);
    // Payloads are the built bytes.
    assert_eq!(&packets[0].data[..], &[0x10; 4]);
    assert_eq!(&packets[3].data[..], &[0x13; 4]);
}

#[test]
fn progressive_two_tracks_interleave_by_file_offset() {
    let v = video_track(3, 3000, &[true, false, false], &[0]);
    let a = audio_track(3, 1024);
    let bytes = progressive(&[v, a], b"isom");
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].codec.as_str(), "h264");
    assert_eq!(tracks[1].codec.as_str(), "aac");
    let (_, packets) = drain(&mut mp4);
    // The builder writes video samples before audio: file order.
    assert_eq!(packets.len(), 6);
    assert_eq!(packets[0].track.handle(), 0);
    assert_eq!(packets[3].track.handle(), 1);
}

#[test]
fn ctts_shifts_pts_off_dts() {
    let track = video_track(3, 3000, &[true, false, false], &[0, -3000, 3000]);
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    let pts = |p: &media_types::Packet| match &p.pts {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("pts missing"),
    };
    assert_eq!(pts(&packets[0]), 0);
    assert_eq!(pts(&packets[1]), 0);
    assert_eq!(pts(&packets[2]), 6000 + 3000);
}

#[test]
fn elst_empty_edit_delays_and_media_time_trims() {
    let mut track = video_track(2, 3000, &[true, true], &[0]);
    track.edit = Some((1_000, -1)); // empty edit: 1000-tick dwell
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    let pts = |p: &media_types::Packet| match &p.pts {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("pts missing"),
    };
    assert_eq!(pts(&packets[0]), 1_000);
    assert_eq!(pts(&packets[1]), 4_000);

    let mut track = video_track(2, 3000, &[true, true], &[0]);
    track.edit = Some((6_000, 1_000)); // media_time 1000: trim
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    let pts = |p: &media_types::Packet| match &p.pts {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("pts missing"),
    };
    assert_eq!(pts(&packets[0]), 0 - 1_000);
    assert_eq!(pts(&packets[1]), 3_000 - 1_000);
}

#[test]
fn absent_stss_means_all_sync() {
    let track = video_track(3, 100, &[true, true, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    assert!(packets.iter().all(|p| p.keyframe));
}

// ---- fragmented -------------------------------------------------

#[test]
fn fragmented_demuxes_moofs_with_tfdt_ladder() {
    let v = video_track(
        8,
        1024,
        &[true, false, false, false, true, false, false, false],
        &[0],
    );
    let bytes = fragmented(&[v], 4, 2);
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    assert!(mp4.is_fragmented());
    assert_eq!(mp4.seekability(), SeekSupport::SlowLinear);
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 8);
    // tfdt ladder: frag f starts at f*2048.
    let dts = |p: &media_types::Packet| match &p.dts {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("dts missing"),
    };
    assert_eq!(dts(&packets[0]), 0);
    assert_eq!(dts(&packets[1]), 1024);
    assert_eq!(dts(&packets[2]), 2048);
    assert_eq!(dts(&packets[4]), 4096);
    // Sync pattern: sample 0 and 4.
    assert!(packets[0].keyframe);
    assert!(!packets[1].keyframe);
    assert!(packets[4].keyframe);
}

#[test]
fn fragmented_two_tracks_per_moof() {
    let v = video_track(4, 1024, &[true, false, true, false], &[0]);
    let a = audio_track(4, 1024);
    let bytes = fragmented(&[v, a], 2, 2);
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 2);
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 8);
    // Per moof: both tracks' samples, DTS order.
    assert_eq!(packets[0].track.handle(), 0);
    assert_eq!(packets[1].track.handle(), 1);
    assert_eq!(packets[2].track.handle(), 0);
    assert_eq!(packets[3].track.handle(), 1);
}

// ---- chapters ---------------------------------------------------

#[test]
fn chap_tref_chapters_surface_with_titles() {
    let main = video_track(2, 3000, &[true, true], &[0]);
    let chapter = chapter_track(1000, &[(1000, "Intro"), (1000, "Middle")]);
    let bytes = progressive_with_chapters(&main, &chapter);
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    // The chapter track is referenced: hidden from the track list.
    assert_eq!(tracks.len(), 1);
    let chapters = mp4.chapters();
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].title, "Intro");
    assert_eq!(chapters[1].title, "Middle");
    // DTS ladder: 0 ticks and 1000 ticks at 1000 timescale = 1 s.
    assert_eq!(chapters[0].start.0, 0);
    assert_eq!(chapters[1].start.0, 1_000_000_000);
    // The chapter trak is metadata, not media: its text samples
    // never surface as packets (the ffmpeg discard precedent —
    // ffprobe reads zero packets off the text trak).
    let (_, packets) = drain(&mut mp4);
    assert!(!packets.is_empty());
    assert!(
        packets.iter().all(|p| p.track.handle() == 0),
        "chapter samples leaked onto a hidden handle"
    );
}

// ---- attached pictures (udta cover art) -------------------------

#[test]
fn cover_art_surfaces_as_a_still_image_video_track() {
    // The corpus-0029 shape: an M4A whose only real trak is audio,
    // with the picture in `udta/meta/ilst/covr/data`.
    let audio = audio_track(2, 1024);
    let jpeg: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 1, 2, 3, 4, 5, 6, 7];
    let bytes = progressive_with_cover(&[audio], b"M4A ", 13, jpeg);
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 2, "the cover is a surfaced track");
    assert_eq!(tracks[0].codec.as_str(), "aac");
    assert_eq!(tracks[1].codec.as_str(), "mjpeg");
    assert_eq!(tracks[1].kind, media_types::MediaKind::Video);
    assert!(tracks[1].flags.still_image, "cover traks are still-image");
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 3);
    let covers: Vec<&media_types::Packet> =
        packets.iter().filter(|p| p.track.handle() == 1).collect();
    assert_eq!(covers.len(), 1, "exactly one cover packet");
    assert!(covers[0].keyframe);
    assert_eq!(covers[0].data.as_ref(), jpeg, "the picture bytes verbatim");
    // The media trak still walks in file order behind the header-time
    // cover packet.
    let audio_packets: Vec<&media_types::Packet> =
        packets.iter().filter(|p| p.track.handle() == 0).collect();
    assert_eq!(audio_packets.len(), 2);
    assert_eq!(audio_packets[0].data.as_ref(), &[0xA0, 0xA0]);
    assert_eq!(audio_packets[1].data.as_ref(), &[0xA1, 0xA1]);
}

#[test]
fn png_cover_and_unknown_cover_types_map_or_note() {
    // Type 14 = PNG — a real descriptor, surfaced like the JPEG lane.
    let audio = audio_track(1, 1024);
    let png: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 9, 9];
    let bytes = progressive_with_cover(std::slice::from_ref(&audio), b"M4A ", 14, png);
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[1].codec.as_str(), "png");
    let (_, packets) = drain(&mut mp4);
    let covers: Vec<&media_types::Packet> =
        packets.iter().filter(|p| p.track.handle() == 1).collect();
    assert_eq!(covers.len(), 1);
    assert_eq!(covers[0].data.as_ref(), png);

    // An unknown well-known type: damage-noted and skipped — the
    // audio still demuxes, no fake codec is guessed.
    let bytes = progressive_with_cover(&[audio], b"M4A ", 27, &[1, 2, 3, 4]);
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1, "unknown cover types surface no track");
    let mut recovered = false;
    let mut packets = 0usize;
    loop {
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(_) => packets += 1,
            DemuxEvent::Recovered { reason, .. } => {
                assert_eq!(reason, "cover-type-unsupported");
                recovered = true;
            }
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    assert!(recovered, "the skipped cover is damage-noted");
    assert_eq!(packets, 1);
}

#[test]
fn cover_and_chapters_keep_packet_handles_aligned_with_tracks() {
    // The gap-alignment case: a hidden chapter trak between the media
    // trak and the appended cover — packets must still carry exactly
    // the handles of the exposed list (0 = audio, 1 = cover).
    let mut audio = audio_track(1, 1024);
    audio.chap_ref = Some(2); // reference the chapter trak (id 2)
    let chapter = chapter_track(1000, &[(1000, "Intro")]);
    let jpeg: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 7, 7, 7];
    let udta_tracks = [audio, chapter];
    let bytes = progressive_with_cover(&udta_tracks, b"isom", 13, jpeg);
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 2, "chapter hidden, cover exposed");
    assert_eq!(mp4.chapters().len(), 1);
    assert_eq!(tracks[0].codec.as_str(), "aac");
    assert_eq!(tracks[1].codec.as_str(), "mjpeg");
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 2);
    assert!(packets.iter().all(|p| p.track.handle() <= 1));
    let cover = packets
        .iter()
        .find(|p| p.track.handle() == 1)
        .unwrap_or_else(|| unreachable!("cover packet on the exposed handle"));
    assert_eq!(cover.data.as_ref(), jpeg);
}

// ---- seek -------------------------------------------------------

fn span_at(nanos: u64) -> TimeSpan {
    TimeSpan {
        start: Stamp::Present(
            Timestamp::from_parts(nanos as i64, 1, 1_000_000_000)
                .unwrap_or_else(|e| unreachable!("ts: {e}")),
        ),
        end: Stamp::Missing,
    }
}

#[test]
fn progressive_seek_lands_on_keyframe() {
    // 6 samples × 1000 ticks @ 9000 Hz ≈ 111 ms each.
    let track = video_track(6, 1000, &[true, false, false, false, true, false], &[0]);
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    // Target sample 3 (0.39 s): previous keyframe is sample 0.
    let landed = mp4
        .seek(span_at(390_000_000))
        .unwrap_or_else(|e| unreachable!("seek: {e}"));
    match landed.start {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 0),
        Stamp::Missing => unreachable!("landing missing"),
    }
    // Target sample 5 (0.58 s): previous keyframe is sample 4.
    let landed = mp4
        .seek(span_at(580_000_000))
        .unwrap_or_else(|e| unreachable!("seek: {e}"));
    match landed.start {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 4000),
        Stamp::Missing => unreachable!("landing missing"),
    }
    // Drain: resumes at the keyframe, not the target.
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 2);
    assert!(packets[0].keyframe);
    // Out of range is the typed refusal.
    let err = mp4.seek(span_at(999_999_999)).err_of("out of range");
    assert_eq!(code_of(&err), "demux.mp4.seek.out-of-range");
}

#[test]
fn fragmented_seek_scans_moofs() {
    let v = video_track(
        8,
        1000,
        &[true, false, false, false, true, false, false, false],
        &[0],
    );
    let bytes = fragmented(&[v], 4, 2);
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    // Frag 2 (samples 4..6, dts 4000 @ 9000 Hz ≈ 444 ms) holds the
    // second sync: a 450 ms target lands there.
    let landed = mp4
        .seek(span_at(450_000_000))
        .unwrap_or_else(|e| unreachable!("seek: {e}"));
    match landed.start {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 4000),
        Stamp::Missing => unreachable!("landing missing"),
    }
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 4);
    assert!(packets[0].keyframe);
}

// ---- error paths (each AC shape, typed, no panics) --------------

#[test]
fn not_mp4_is_typed_refusal() {
    let mut mp4 = open(b"RIFF\x00\x00\x00\x00WAVEjunkjunkjunk");
    let err = mp4.start().err_of("not mp4");
    assert_eq!(code_of(&err), "demux.mp4.not-mp4");
}

#[test]
fn missing_moov_is_typed() {
    // A known-brand ftyp followed by garbage: probe passes, the scan
    // finds no moov.
    let mut file = Vec::new();
    file.extend_from_slice(&ftyp_bytes());
    file.extend_from_slice(&[0xAB; 64]);
    let mut mp4 = open(&file);
    let err = mp4.start().err_of("no moov");
    assert_eq!(code_of(&err), "demux.mp4.no-moov");
}

fn ftyp_bytes() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&32u32.to_be_bytes());
    v.extend_from_slice(b"ftyp");
    v.extend_from_slice(b"isom");
    v.extend_from_slice(&0u32.to_be_bytes());
    v.extend_from_slice(b"isom");
    v.extend_from_slice(b"iso2");
    v
}

#[test]
fn truncated_box_header_is_typed_truncation() {
    let track = video_track(2, 100, &[true, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    // Cut mid-file (inside moov): the moov extent exceeds the stream.
    let cut = &bytes[..bytes.len() / 2];
    let mut mp4 = open(cut);
    let err = mp4.start().err_of("truncated");
    assert_eq!(code_of(&err), "demux.mp4.truncated");
}

#[test]
fn size_zero_convention_extends_to_stream_end() {
    let track = video_track(2, 100, &[true, true], &[0]);
    let mut bytes = Vec::from(progressive(&[track], b"isom").as_ref());
    // Rewrite the mdat header size to 0 ("to EOF") — still demuxes.
    let mut at = None;
    for i in 0..bytes.len().saturating_sub(4) {
        if &bytes[i..i + 4] == b"mdat" {
            at = Some(i - 4);
        }
    }
    let Some(at) = at else {
        unreachable!("no mdat")
    };
    bytes[at..at + 4].copy_from_slice(&0u32.to_be_bytes());
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 2);
}

#[test]
fn size_one_reads_64bit_largesize() {
    let track = video_track(2, 100, &[true, true], &[0]);
    let bytes = progressive(&[track], b"isom").to_vec();
    // Rewrite the mdat to a 64-bit-sized box: size=1 + largesize.
    let mut at = None;
    for i in 0..bytes.len().saturating_sub(4) {
        if &bytes[i..i + 4] == b"mdat" {
            at = Some(i - 4);
        }
    }
    let Some(at) = at else {
        unreachable!("no mdat")
    };
    let total = bytes.len() as u64;
    let mut head = Vec::new();
    head.extend_from_slice(&1u32.to_be_bytes());
    head.extend_from_slice(b"mdat");
    head.extend_from_slice(&(total - at as u64).to_be_bytes());
    let payload_at = at + 8;
    let payload = bytes[payload_at..].to_vec();
    let mut rebuilt = bytes[..at].to_vec();
    rebuilt.extend_from_slice(&head);
    rebuilt.extend_from_slice(&payload);
    let mut mp4 = open(&rebuilt);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 2);
}

#[test]
fn child_beyond_parent_is_typed() {
    let track = video_track(2, 100, &[true, true], &[0]);
    let mut bytes = progressive(&[track], b"isom").to_vec();
    // Inflate the moov's declared size so a child claims past its
    // parent (stco entries then "extend" the moov): instead, shrink
    // the stbl's parent — find "stbl" and set its size to 8+4.
    let mut at = None;
    for i in 0..bytes.len().saturating_sub(4) {
        if &bytes[i..i + 4] == b"stbl" {
            at = Some(i - 4);
        }
    }
    let Some(at) = at else {
        unreachable!("no stbl")
    };
    // Set stbl size to 12 (room for stsd's first bytes only) — the
    // stsd child then overruns the parent (damage-noted per trak;
    // the file then dies typed at no-tracks or the overrun itself).
    bytes[at..at + 4].copy_from_slice(&12u32.to_be_bytes());
    let mut mp4 = open(&bytes);
    let err = mp4.start().err_of("overrun");
    let code = code_of(&err);
    assert!(
        code == "demux.mp4.box-overruns-parent"
            || code == "demux.mp4.truncated"
            || code == "demux.mp4.no-tracks",
        "got {code}"
    );

    // The direct shape: inflate the trak's declared size past the
    // moov's end — the moov walk must refuse typed, not truncate.
    let track2 = video_track(2, 100, &[true, true], &[0]);
    let mut bytes2 = Vec::from(progressive(&[track2], b"isom").as_ref());
    let mut trak_at = None;
    for i in 0..bytes2.len().saturating_sub(4) {
        if &bytes2[i..i + 4] == b"trak" {
            trak_at = Some(i - 4);
        }
    }
    let Some(trak_at) = trak_at else {
        unreachable!("no trak")
    };
    let size = u32::from_be_bytes([
        bytes2[trak_at],
        bytes2[trak_at + 1],
        bytes2[trak_at + 2],
        bytes2[trak_at + 3],
    ]);
    bytes2[trak_at..trak_at + 4].copy_from_slice(&(size + 64).to_be_bytes());
    let mut mp4 = open(&bytes2);
    let err = mp4.start().err_of("child > parent");
    assert_eq!(code_of(&err), "demux.mp4.box-overruns-parent");
}

#[test]
fn zero_sample_tables_in_progressive_is_typed() {
    // A progressive file whose stsz/stco declare zero entries: no
    // sample table, nothing to demux — the typed refusal.
    let v = video_track(4, 100, &[true, false, true, false], &[0]);
    let mut bytes = Vec::from(progressive(&[v], b"isom").as_ref());
    let mut patch = |tag: &[u8; 4], field_at: usize| {
        let mut at = None;
        for i in 0..bytes.len().saturating_sub(4) {
            if &bytes[i..i + 4] == tag {
                at = Some(i - 4); // box start
            }
        }
        let Some(start) = at else {
            unreachable!("no {tag:?} box")
        };
        // entry count sits `field_at` bytes into the box payload
        // (8 header + version/flags + box-specific prefix).
        bytes[start + field_at..start + field_at + 4].copy_from_slice(&0u32.to_be_bytes());
    };
    patch(b"stsz", 8 + 8); // v/f(4) + fixed(4) + count
    patch(b"stco", 8 + 4); // v/f(4) + count
    let mut mp4 = open(&bytes);
    let err = mp4.start().err_of("no tables");
    let code = code_of(&err);
    assert!(
        code == "demux.mp4.no-sample-table" || code == "demux.mp4.no-tracks",
        "got {code}"
    );
}

#[test]
fn unknown_boxes_between_ftyp_and_moov_are_tolerated() {
    let track = video_track(1, 100, &[true], &[0]);
    let bytes = Vec::from(progressive(&[track], b"isom").as_ref());
    // Splice an unknown top-level box between ftyp (ends at 36) and
    // the moov.
    let mut junk = Vec::new();
    junk.extend_from_slice(&16u32.to_be_bytes());
    junk.extend_from_slice(b"junk");
    junk.extend_from_slice(&[0xEE; 8]);
    let mut spliced = bytes[..36].to_vec();
    spliced.extend_from_slice(&junk);
    spliced.extend_from_slice(&bytes[36..]);
    let mut mp4 = open(&spliced);
    mp4.start()
        .unwrap_or_else(|e| unreachable!("start with junk box: {e}"));
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 1);
}

#[test]
fn truncated_sample_read_is_typed_after_start() {
    let track = video_track(2, 100, &[true, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    // Cut the tail so sample 2 runs past the stream end.
    let cut = &bytes[..bytes.len() - 1];
    let mut mp4 = open(cut);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    // Sample 1 is fully present and delivered; sample 2 runs one
    // byte past the stream end.
    assert!(matches!(mp4.next_event(), Ok(DemuxEvent::Packet(_))));
    let err = mp4.next_event().err_of("truncated sample");
    assert_eq!(code_of(&err), "demux.mp4.truncated");
    // Sticky: the error repeats.
    let err2 = mp4.next_event().err_of("sticky");
    assert_eq!(code_of(&err2), "demux.mp4.truncated");
}

#[test]
fn not_started_calls_are_typed() {
    let track = video_track(1, 100, &[true], &[0]);
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    let err = mp4.next_event().err_of("not started");
    assert_eq!(code_of(&err), "demux.mp4.not-started");
    let err = mp4.seek(span_at(0)).err_of("not started");
    assert_eq!(code_of(&err), "demux.mp4.not-started");
}

#[test]
fn mov_quirks_wide_and_qt_brand_open() {
    let track = video_track(2, 100, &[true, true], &[0]);
    let bytes = progressive(&[track], b"qt  ");
    // Prefix a wide box (the classic MOV pre-padding).
    let mut wide = Vec::new();
    wide.extend_from_slice(&8u32.to_be_bytes());
    wide.extend_from_slice(b"wide");
    let mut file = wide;
    file.extend_from_slice(&bytes[..]);
    let mut mp4 = open(&file);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    assert_eq!(mp4.major_brand(), b"qt  ");
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 2);
}

#[test]
fn sowt_entry_maps_to_pcm_s16le() {
    let mut a = audio_track(2, 1024);
    a.entry = *b"sowt";
    let bytes = progressive(&[a], b"qt  ");
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
}

#[test]
fn threegp_brand_and_samr_entry_open() {
    let mut a = audio_track(2, 160);
    a.entry = *b"samr";
    let bytes = progressive(&[a], b"3gp5");
    let mut mp4 = open(&bytes);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks[0].codec.as_str(), "amr_nb");
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 2);
}

#[test]
fn seek_failure_leaves_state_usable_d4() {
    let track = video_track(4, 1000, &[true, false, false, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    // Consume two packets, then a failing seek must not kill the walk.
    let mut seen = 0;
    for _ in 0..2 {
        if let DemuxEvent::Packet(_) = mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            seen += 1;
        }
    }
    assert_eq!(seen, 2);
    let err = mp4.seek(span_at(999_999_999)).err_of("oor");
    assert_eq!(code_of(&err), "demux.mp4.seek.out-of-range");
    // The stream continues where it was: the remaining 2 packets.
    let (_, packets) = drain(&mut mp4);
    assert_eq!(packets.len(), 2);
}

// ---- +FB (fuzz-lite, deterministic in-crate sweep) --------------

#[test]
fn fuzz_lite_mutations_never_panic() {
    let track = video_track(4, 100, &[true, false, false, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    // Truncation prefixes.
    for cut in [0usize, 7, 8, 32, 64, 128, bytes.len() / 2] {
        super::fuzz::fuzz_demux_mp4(&bytes[..cut.min(bytes.len())]);
    }
    // Single-bit flips at stride.
    let mut mutated = bytes.to_vec();
    for i in (0..mutated.len()).step_by(37) {
        mutated[i] ^= 0x80;
        super::fuzz::fuzz_demux_mp4(&mutated);
        mutated[i] ^= 0x80;
    }
    // Garbage tail.
    let mut garbage = bytes.to_vec();
    garbage.extend_from_slice(&[0xFF; 512]);
    super::fuzz::fuzz_demux_mp4(&garbage);
}

#[test]
fn fuzz_lite_fragment_mutations_never_panic() {
    let v = video_track(4, 1000, &[true, false, true, false], &[0]);
    let bytes = fragmented(&[v], 2, 2);
    for cut in [0usize, 9, 64, bytes.len() / 3, bytes.len() - 4] {
        super::fuzz::fuzz_demux_mp4(&bytes[..cut.min(bytes.len())]);
    }
    let mut mutated = bytes.to_vec();
    for i in (0..mutated.len()).step_by(53) {
        mutated[i] ^= 0x01;
        super::fuzz::fuzz_demux_mp4(&mutated);
        mutated[i] ^= 0x01;
    }
}
