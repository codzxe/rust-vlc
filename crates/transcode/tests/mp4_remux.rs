//! The card 7.E2 MP4-remux acceptance suite (issue #44 AC): the
//! [`Remuxer`] one-call assembly copies a committed H.264/AAC fixture
//! into a fresh MP4, and the copy is verified by OUR own demuxer —
//! per-track packet streams compared field-by-field (payload bytes,
//! DTS, PTS, keyframe flags, durations, counts). Plus the honesty
//! refusals: a codec outside the stream-copy set fails typed, a
//! config naming a track the source does not expose fails typed, an
//! empty config list is refused at submit, and a mid-remux cancel
//! leaves a *playable honest prefix* (a structurally valid MP4 whose
//! sample tables describe exactly the samples that made it).
//!
//! Fixtures: the committed ffmpeg encodes under
//! `tests/fixtures/generated/valid/` — the progressive pair
//! (`mp4_h264_aac.mp4`), the fragmented twin (`mp4_frag.mp4`), the
//! edit-list shape (`mp4_edts.mp4`) and the outside-the-set pair
//! (`mp4_mjpeg_pcm.mp4` — MJPEG video + PCM audio, both refused).

use std::io::{Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use demux::mp4::Mp4Demuxer;
use media_types::ids::SourceUri;
use media_types::ids::{CodecDescriptor, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration as MediaDuration, Stamp};
use media_types::{EngineError, GenerationId, Packet, Track};
use plugin_api::{Access, DemuxEvent, Demuxer, SeekOrigin, SourcePosition, SourceStream};
use testkit::{Must, fixture_path};
use transcode::mp4mux::Mp4TrackConfig;
use transcode::record::Remuxer;
use transcode::{JobLimits, JobOutcome};

// ---------------------------------------------------------------------------
// Shared plumbing (the record suite's shapes, reproduced test-locally —
// integration suites do not share code across files)
// ---------------------------------------------------------------------------

/// A `Write + Seek` sink into a shared buffer (the readable twin).
struct SharedCursor {
    twin: Arc<Mutex<Vec<u8>>>,
    pos: usize,
}

fn sink_pair() -> (Box<dyn transcode::WriteSeek>, Arc<Mutex<Vec<u8>>>) {
    let twin: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    (
        Box::new(SharedCursor {
            twin: Arc::clone(&twin),
            pos: 0,
        }),
        twin,
    )
}

impl Write for SharedCursor {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.twin.lock().unwrap_or_else(|p| p.into_inner());
        if self.pos + bytes.len() > guard.len() {
            guard.resize(self.pos + bytes.len(), 0);
        }
        guard[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for SharedCursor {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        match to {
            SeekFrom::Start(n) => self.pos = n as usize,
            SeekFrom::Current(n) => {
                self.pos = usize::try_from(self.pos as i64 + n).unwrap_or(self.pos);
            }
            SeekFrom::End(n) => {
                let len = self.twin.lock().unwrap_or_else(|p| p.into_inner()).len() as i64;
                self.pos = usize::try_from(len + n).unwrap_or(self.pos);
            }
        }
        Ok(self.pos as u64)
    }
}

/// An in-memory `SourceStream` (size-known, seekable).
struct MemStream {
    uri: SourceUri,
    data: Arc<Vec<u8>>,
    pos: usize,
}

impl MemStream {
    fn over(bytes: Vec<u8>) -> MemStream {
        MemStream {
            uri: SourceUri::parse("memory://mp4-remux-suite").must(),
            data: Arc::new(bytes),
            pos: 0,
        }
    }
}

impl SourceStream for MemStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }
    fn size(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }
    fn supports_seek(&self) -> bool {
        true
    }
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let end = (self.pos + len).min(self.data.len());
        let out = self.data[self.pos..end].to_vec();
        self.pos = end;
        Ok(out)
    }
    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let end = (self.pos + out.len()).min(self.data.len());
        out[..end - self.pos].copy_from_slice(&self.data[self.pos..end]);
        let got = end - self.pos;
        self.pos = end;
        Ok(got)
    }
    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target: u64 = match origin {
            SeekOrigin::Start(to) => to,
            SeekOrigin::Current(delta) => match (self.pos as i64).checked_add(delta) {
                Some(n) if n >= 0 => n as u64,
                _ => return Err(mem_seek_error()),
            },
        };
        if target > self.data.len() as u64 {
            return Err(mem_seek_error());
        }
        self.pos = target as usize;
        Ok(SourcePosition(target))
    }
    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos as u64)
    }
}

fn mem_seek_error() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Open,
        media_types::ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new("transcode.test.mem-seek-out-of-range"),
    )
}

fn open_fixture(relative: &str) -> Mp4Demuxer {
    let path = fixture_path(relative).must();
    let uri = SourceUri::parse(&format!("file://{}", path.to_str().unwrap_or_default())).must();
    let stream = input::FileAccess::new().open(&uri).must();
    Mp4Demuxer::new(stream).must()
}

/// One flattened packet row for the stream comparisons.
struct FlatPacket {
    track: TrackId,
    dts: i64,
    pts: i64,
    keyframe: bool,
    duration_nanos: Option<u64>,
    data: Vec<u8>,
}

fn ticks(stamp: &Stamp) -> i64 {
    match stamp {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => i64::MIN,
    }
}

fn duration_nanos(packet: &Packet) -> Option<u64> {
    match packet.duration {
        Some(MediaDuration::Known(nanos)) => Some(nanos.0),
        _ => None,
    }
}

/// Start the demuxer and flatten every packet, in demux order.
fn demux_all(demuxer: &mut Mp4Demuxer) -> (Vec<Track>, Vec<FlatPacket>) {
    let tracks = demuxer.start().must();
    let mut packets = Vec::new();
    loop {
        match demuxer.next_event().must() {
            DemuxEvent::Packet(packet) => packets.push(FlatPacket {
                track: packet.track,
                dts: ticks(&packet.dts),
                pts: ticks(&packet.pts),
                keyframe: packet.keyframe,
                duration_nanos: duration_nanos(&packet),
                data: packet.data.as_slice().to_vec(),
            }),
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    (tracks, packets)
}

/// The per-track packet subsequence (demux order preserved).
fn of_track(packets: &[FlatPacket], track: TrackId) -> Vec<&FlatPacket> {
    packets.iter().filter(|p| p.track == track).collect()
}

/// One h264/aac config per named track, sample entries harvested from
/// the OPEN demuxer the same source will re-open through (the host
/// wiring point the docs describe).
fn config_for(demuxer: &Mp4Demuxer, track: &Track) -> Mp4TrackConfig {
    Mp4TrackConfig {
        track: track.id,
        sample_entry: SharedBytes::from_vec(
            demuxer.sample_entry_bytes(track.id).unwrap_or(&[]).to_vec(),
        ),
    }
}

/// The h264 and aac tracks of a started fixture (moov order).
fn h264_aac(tracks: &[Track]) -> (Track, Track) {
    let video = tracks
        .iter()
        .find(|t| t.codec.as_str() == "h264")
        .unwrap_or_else(|| unreachable!("no h264 track"));
    let audio = tracks
        .iter()
        .find(|t| t.codec.as_str() == "aac")
        .unwrap_or_else(|| unreachable!("no aac track"));
    (video.clone(), audio.clone())
}

// ---------------------------------------------------------------------------
// AC: remuxed output verified by our own demuxer, packets compared
// ---------------------------------------------------------------------------

/// The exact-comparison shape: payload bytes, stamps, keyframe flags,
/// durations and counts, per track, plus the container facts the
/// remux claims to preserve (timescale, codec descriptors).
#[test]
fn progressive_source_remuxes_and_redemuxes_packet_identical() {
    const WHAT: &str = "generated/valid/mp4_h264_aac.mp4";
    // The verification side: the source's own demux.
    let mut source = open_fixture(WHAT);
    let (tracks, baseline) = demux_all(&mut source);
    let (video, audio) = h264_aac(&tracks);
    let configs = [config_for(&source, &video), config_for(&source, &audio)];

    // The remux: a fresh source, the one-call assembly.
    let (sink, twin) = sink_pair();
    let job = Remuxer::new(Box::new(open_fixture(WHAT)), sink)
        .track(configs[0].clone())
        .track(configs[1].clone())
        .remux(JobLimits::default())
        .must();
    let stats = match job.wait() {
        JobOutcome::Completed(stats) => stats,
        other => unreachable!("remux completes, got {other:?}"),
    };
    // Both tracks' payload bytes landed (the mdat truth).
    let expect_bytes: u64 = baseline.iter().map(|p| p.data.len() as u64).sum();
    assert_eq!(stats.bytes_written, expect_bytes);

    // Our demuxer accepts the copy…
    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    assert!(&output[4..8] == b"ftyp", "the copy is an MP4 file");
    let mut round = Mp4Demuxer::new(Box::new(MemStream::over(output))).must();
    let (round_tracks, round_packets) = demux_all(&mut round);
    assert!(!round.is_fragmented(), "the copy is progressive");
    // …with the same two tracks, codecs and timescales preserved.
    assert_eq!(round_tracks.len(), 2);
    assert_eq!(round_tracks[0].codec.as_str(), "h264");
    assert_eq!(round_tracks[1].codec.as_str(), "aac");
    assert_eq!(round_tracks[0].timing.base, video.timing.base);
    assert_eq!(round_tracks[1].timing.base, audio.timing.base);

    // …and the per-track packet streams compare field-for-field.
    // The fresh movie re-anchors each track's timeline at its first
    // sample DTS (the muxer's documented honesty table — an edit-list
    // source can surface a negative first DTS), so stamps compare
    // relative to that origin; payload bytes, sync flags, durations
    // and counts are exact.
    for track in [&video, &audio] {
        let want = of_track(&baseline, track.id);
        let got = of_track(&round_packets, track.id);
        assert_eq!(
            want.len(),
            got.len(),
            "track {} sample count",
            track.id.handle()
        );
        let origin = want.first().map(|p| p.dts).unwrap_or(0);
        assert_eq!(
            got.first().map(|p| p.dts),
            Some(0i64),
            "track {} origin",
            track.id.handle()
        );
        for (i, (w, g)) in want.iter().zip(got.iter()).enumerate() {
            assert_eq!(
                w.data,
                g.data,
                "track {} sample {} payload",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.dts - origin,
                g.dts,
                "track {} sample {} dts",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.pts - origin,
                g.pts,
                "track {} sample {} pts",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.keyframe,
                g.keyframe,
                "track {} sample {} sync flag",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.duration_nanos,
                g.duration_nanos,
                "track {} sample {} duration",
                track.id.handle(),
                i
            );
        }
    }
}

/// The fragmented source remuxes into a PROGRESSIVE copy: the moof
/// walk's packets land under one classic sample-table moov (the
/// capability the muxer claims — fragmented input, progressive
/// output).
#[test]
fn fragmented_source_remuxes_into_a_progressive_output() {
    const WHAT: &str = "generated/valid/mp4_frag.mp4";
    let mut source = open_fixture(WHAT);
    let (tracks, baseline) = demux_all(&mut source);
    assert!(source.is_fragmented(), "the fixture is fragmented");
    let (video, audio) = h264_aac(&tracks);

    let (sink, twin) = sink_pair();
    let job = Remuxer::new(Box::new(open_fixture(WHAT)), sink)
        .track(config_for(&source, &video))
        .track(config_for(&source, &audio))
        .remux(JobLimits::default())
        .must();
    match job.wait() {
        JobOutcome::Completed(_) => {}
        other => unreachable!("fragmented remux completes, got {other:?}"),
    }

    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let mut round = Mp4Demuxer::new(Box::new(MemStream::over(output))).must();
    let (round_tracks, round_packets) = demux_all(&mut round);
    assert!(!round.is_fragmented(), "the copy is progressive");
    assert_eq!(round_tracks.len(), 2);
    // Payloads and sync flags are exact; stamps compare relative to
    // each track's first sample (the fresh movie's re-anchored
    // timeline — the muxer's documented honesty table).
    for track in [&video, &audio] {
        let want = of_track(&baseline, track.id);
        let got = of_track(&round_packets, track.id);
        assert_eq!(want.len(), got.len(), "track {} count", track.id.handle());
        let origin = want.first().map(|p| p.dts).unwrap_or(0);
        for (i, (w, g)) in want.iter().zip(got.iter()).enumerate() {
            assert_eq!(
                w.data,
                g.data,
                "track {} sample {} payload",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.keyframe,
                g.keyframe,
                "track {} sample {} sync",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.dts - origin,
                g.dts,
                "track {} sample {} re-anchored dts",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.pts - w.dts,
                g.pts - g.dts,
                "track {} sample {} composition offset",
                track.id.handle(),
                i
            );
        }
    }
}

/// The edit-list source re-anchors on the fresh movie timeline: the
/// copy's first sample DTS is the media zero, composition offsets
/// survive, payloads are exact.
#[test]
fn edit_list_source_re_anchors_the_media_timeline() {
    const WHAT: &str = "generated/valid/mp4_edts.mp4";
    let mut source = open_fixture(WHAT);
    let (tracks, baseline) = demux_all(&mut source);
    let (video, audio) = h264_aac(&tracks);

    let (sink, twin) = sink_pair();
    let job = Remuxer::new(Box::new(open_fixture(WHAT)), sink)
        .track(config_for(&source, &video))
        .track(config_for(&source, &audio))
        .remux(JobLimits::default())
        .must();
    match job.wait() {
        JobOutcome::Completed(_) => {}
        other => unreachable!("edts remux completes, got {other:?}"),
    }

    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let mut round = Mp4Demuxer::new(Box::new(MemStream::over(output))).must();
    let (_, round_packets) = demux_all(&mut round);
    for track in [&video, &audio] {
        let want = of_track(&baseline, track.id);
        let got = of_track(&round_packets, track.id);
        assert_eq!(want.len(), got.len(), "track {} count", track.id.handle());
        let origin = want.first().map(|p| p.dts).unwrap_or(0);
        // The fresh timeline starts at the first sample.
        assert_eq!(
            got.first().map(|p| p.dts),
            Some(0i64),
            "track {} origin",
            track.id.handle()
        );
        for (i, (w, g)) in want.iter().zip(got.iter()).enumerate() {
            assert_eq!(
                w.data,
                g.data,
                "track {} sample {} payload",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.dts - origin,
                g.dts,
                "track {} sample {} dts",
                track.id.handle(),
                i
            );
            assert_eq!(
                w.pts - w.dts,
                g.pts - g.dts,
                "track {} sample {} cts",
                track.id.handle(),
                i
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Honesty refusals
// ---------------------------------------------------------------------------

/// A codec outside the stream-copy set fails the job typed — the
/// remux never transcodes and never writes a track it cannot copy.
#[test]
fn remux_refuses_codecs_outside_the_stream_copy_set() {
    const WHAT: &str = "generated/valid/mp4_mjpeg_pcm.mp4";
    let mut source = open_fixture(WHAT);
    let (tracks, _) = demux_all(&mut source);
    // The fixture's tracks are MJPEG video + PCM audio — both outside
    // {h264, aac}; the FIRST config's cross-check refuses.
    let first = tracks
        .first()
        .unwrap_or_else(|| unreachable!("fixture has tracks"))
        .clone();
    let (sink, _twin) = sink_pair();
    let job = Remuxer::new(Box::new(open_fixture(WHAT)), sink)
        .track(config_for(&source, &first))
        .remux(JobLimits::default())
        .must();
    let err = match job.wait() {
        JobOutcome::Failed(err) => err,
        other => unreachable!("mjpeg remux fails, got {other:?}"),
    };
    assert_eq!(err.context.code, "transcode.mp4.codec-unsupported");
    assert_eq!(err.stage, media_types::PipelineStage::Transcode);
}

/// A config naming a track the source does not expose fails typed at
/// the cross-check (never a silent empty track in the output).
#[test]
fn remux_refuses_a_config_naming_a_missing_track() {
    const WHAT: &str = "generated/valid/mp4_h264_aac.mp4";
    let mut source = open_fixture(WHAT);
    let (tracks, _) = demux_all(&mut source);
    let (video, _audio) = h264_aac(&tracks);
    let (sink, _twin) = sink_pair();
    let job = Remuxer::new(Box::new(open_fixture(WHAT)), sink)
        .track(config_for(&source, &video))
        .track(Mp4TrackConfig {
            track: TrackId::new(9),
            sample_entry: SharedBytes::from_vec(vec![0u8; 8]),
        })
        .remux(JobLimits::default())
        .must();
    let err = match job.wait() {
        JobOutcome::Failed(err) => err,
        other => unreachable!("missing-track config fails, got {other:?}"),
    };
    assert_eq!(err.context.code, "transcode.mp4.config-track-missing");
}

/// The empty config list is refused at submit (synchronously, typed)
/// — a remux with no tracks is not a job.
#[test]
fn submit_refuses_an_empty_remux() {
    let (sink, _twin) = sink_pair();
    let err = Remuxer::new(
        Box::new(open_fixture("generated/valid/mp4_h264_aac.mp4")),
        sink,
    )
    .remux(JobLimits::default())
    .err()
    .unwrap_or_else(|| unreachable!("empty remux is refused at submit"));
    assert_eq!(err.context.code, "transcode.mp4.tracks-empty");
}

// ---------------------------------------------------------------------------
// Cancel: honest partial output (a playable MP4 prefix)
// ---------------------------------------------------------------------------

/// One scripted demuxer step: a packet on one of two tracks; dropping
/// the sender says end-of-stream.
enum Step {
    Packet(TrackId, Vec<u8>),
}

/// An mpsc-backed two-track `Demuxer` the test drives packet-by-packet
/// — holding the sender parks the pump mid-flight, the observation
/// point a mid-remux cancel needs.
struct ScriptedDemuxer {
    rx: std::sync::mpsc::Receiver<Step>,
    tracks: Vec<Track>,
}

impl Demuxer for ScriptedDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        match self.rx.recv() {
            Ok(Step::Packet(track, bytes)) => Ok(DemuxEvent::Packet(Packet {
                track,
                pts: Stamp::Missing,
                dts: Stamp::Present(media_types::time::Timestamp::new(
                    tick_of(track),
                    media_types::time::TimeBase::new(1, 900).must(),
                )),
                duration: None,
                data: SharedBytes::from_vec(bytes),
                keyframe: true,
                discontinuity: false,
                generation: GenerationId::FIRST,
            })),
            Err(_) => Ok(DemuxEvent::EndOfStream),
        }
    }
    fn seek(
        &mut self,
        _target: media_types::time::TimeSpan,
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        Ok(media_types::time::TimeSpan::UNKNOWN)
    }
}

/// A per-track DTS ladder (video every 300 ticks, audio every 100 —
/// monotone per track, interleaved across tracks).
fn tick_of(track: TrackId) -> i64 {
    if track.handle() == 0 { 300 } else { 100 }
}

fn scripted_tracks() -> Vec<Track> {
    let shape = |id: u32, kind: media_types::MediaKind, codec: &str| Track {
        id: TrackId::new(id),
        stream: media_types::ids::StreamId::new(id),
        kind,
        language: None,
        codec: CodecDescriptor::new(codec).must(),
        title: None,
        flags: media_types::ids::TrackFlags::NONE,
        timing: media_types::time::TrackTiming {
            span: media_types::time::TimeSpan::UNKNOWN,
            base: media_types::time::TimeBase::new(1, 900).must(),
            gaps: Vec::new(),
        },
    };
    vec![
        shape(0, media_types::MediaKind::Video, "h264"),
        shape(1, media_types::MediaKind::Audio, "aac"),
    ]
}

/// A minimal but structurally real VisualSampleEntry payload (the
/// 8-byte header's body: the 6 reserved + 2 data-reference-index
/// bytes, then the VisualSampleEntry fields through width/height).
fn visual_sample_entry() -> SharedBytes {
    let mut entry = vec![0u8; 78];
    // width/height at payload offsets 24/26 (the muxer's own reader).
    entry[24..26].copy_from_slice(&64u16.to_be_bytes());
    entry[26..28].copy_from_slice(&48u16.to_be_bytes());
    SharedBytes::from_vec(entry)
}

/// A minimal AudioSampleEntry payload (same 8-byte prologue shape).
fn audio_sample_entry() -> SharedBytes {
    let mut entry = vec![0u8; 28];
    // channel count / sample size at payload offsets 16/18.
    entry[16..18].copy_from_slice(&2u16.to_be_bytes());
    entry[18..20].copy_from_slice(&16u16.to_be_bytes());
    // 44100 << 16 at 20..24 (the classic 16.16 rate field).
    entry[20..24].copy_from_slice(&(44_100u32 << 16).to_be_bytes());
    SharedBytes::from_vec(entry)
}

/// Bounded poll: run `probe` until it answers true or the budget dies.
fn eventually(probe: impl Fn() -> bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !probe() {
        if Instant::now() > deadline {
            unreachable!("timed out waiting for {what}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn mid_remux_cancel_leaves_an_honest_playable_prefix() {
    const PACKET: usize = 512;
    let (tx, rx) = std::sync::mpsc::channel::<Step>();
    let (sink, twin) = sink_pair();
    let job = Remuxer::new(
        Box::new(ScriptedDemuxer {
            rx,
            tracks: scripted_tracks(),
        }),
        sink,
    )
    .track(Mp4TrackConfig {
        track: TrackId::new(0),
        sample_entry: visual_sample_entry(),
    })
    .track(Mp4TrackConfig {
        track: TrackId::new(1),
        sample_entry: audio_sample_entry(),
    })
    .remux(JobLimits::default())
    .must();
    // Six packets per track, interleaved, then HOLD the sender: the
    // pump parks inside next_event once the channel empties —
    // mid-remux, exactly.
    for step in 0..6u32 {
        tx.send(Step::Packet(TrackId::new(0), vec![step as u8; PACKET]))
            .must();
        tx.send(Step::Packet(
            TrackId::new(1),
            vec![0x40 + step as u8; PACKET],
        ))
        .must();
    }
    eventually(
        || job.stats().bytes_written >= PACKET as u64,
        "at least one remuxed packet",
    );
    job.cancel();
    drop(tx);

    let stats = match job.wait() {
        JobOutcome::Cancelled(stats) => stats,
        other => unreachable!("mid-remux stop cancels, got {other:?}"),
    };
    assert_eq!(job.state(), transcode::JobState::Cancelled);
    // A truthful prefix: whole packets only, somewhere between the
    // observed packet and all twelve.
    assert!(
        stats.bytes_written >= PACKET as u64 && stats.bytes_written <= 12 * PACKET as u64,
        "prefix bytes {}",
        stats.bytes_written
    );
    assert_eq!(stats.bytes_written % PACKET as u64, 0, "whole packets");

    // The partial file is a structurally valid MP4 whose tables tell
    // the truth about the prefix (the honest-partial contract).
    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    assert!(&output[4..8] == b"ftyp", "the prefix is an MP4 file");
    let mut round = Mp4Demuxer::new(Box::new(MemStream::over(output))).must();
    let (round_tracks, round_packets) = demux_all(&mut round);
    assert_eq!(round_tracks.len(), 2, "both tracks exist in the prefix");
    for id in [TrackId::new(0), TrackId::new(1)] {
        let got = of_track(&round_packets, id);
        assert!(!got.is_empty(), "track {} has samples", id.handle());
        assert!(got.len() <= 6, "track {} prefix ≤ six samples", id.handle());
        // Every landed sample is one of the scripted payloads.
        for (i, p) in got.iter().enumerate() {
            let salt = p.data[0];
            assert_eq!(
                p.data,
                vec![salt; PACKET],
                "track {} sample {} whole",
                id.handle(),
                i
            );
        }
        // Whole packets only: the size table cannot describe a tear.
        assert!(
            got.iter().all(|p| p.data.len() == PACKET),
            "track {} samples are whole packets",
            id.handle()
        );
    }
    // The bytes the job reported are exactly the bytes the tables
    // describe (mdat size honesty).
    let landed: usize = round_packets.iter().map(|p| p.data.len()).sum();
    assert_eq!(landed as u64, stats.bytes_written, "tables = stats");
}
