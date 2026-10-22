//! The card 3.A3 **real-fixture** acceptance test (the AC's "switch
//! tests on multi-track fixtures with event-sequence assertions";
//! design W1-17 S7 case 12): the committed multi-track parent
//! `avi_mjpeg_2pcm.avi` (W2-FIX-MT) opened through the **real**
//! byte-source → demuxer → decoder stack — `input`'s `FileAccess`,
//! `demux`'s `AviDemuxer`, `codecs`' `MjpegDecoder`/`PcmDecoder` —
//! and driven through a mid-play audio track switch with the full
//! ordered event-sequence assertions of the fake-harness happy path
//! (`tests/track_switch.rs` case 1).
//!
//! # Why a dev-dependency composition (and not the fakes)
//!
//! The engine crate is trait-level by contract (no concrete demux or
//! codec crate in `[dependencies]`); this suite is the **test-side**
//! composition proof — the same pattern `demux` uses for its
//! `input` dev-edge (card 2.B2's "prove the 2.A1+2.B2 composition at
//! the plugin-api seam") and `codecs` for its corpus edge. The
//! boundaries manifest carries the Lead flag (dev edges are exempt
//! from layer ordering by its own rules).
//!
//! # The recorded truth
//!
//! Every number this test switches on is **read from the fixture's
//! expectation TOML** (`[switch_plan]`), never transcribed: the
//! switch point, the from/to track ids, and the post-switch audio
//! shape (44.1 kHz mono `pcm_u8` vs the wired default's 48 kHz
//! stereo `pcm_s16le` — the deliberately distinct shapes that make a
//! **wrong-track decode observable at the sink** as a format change).
//!
//! # The proof, in order (one switch, then the recycle round-trip)
//!
//! 1. open → `Playing` on the real container;
//! 2. play to the recorded switch point, then
//!    `tracks.set-audio-track { id: to_track }`;
//! 3. the mint→drain→re-prime→`tracks.changed` sequence: receipt
//!    (`Accepted` at the post-bump generation) → `state-changed
//!    playing→playing` → the new track's first frame at the sink
//!    stamped with the switch generation (the re-prime fact) → the
//!    `tracks.changed` terminal under the switch's **own** generation
//!    with the causing request as origin;
//! 4. the drain: zero old-track frames past the boundary (every
//!    pre-switch from-track frame carries a pre-mint stamp, and the
//!    last one is the frame already decoded when the switch landed),
//!    and the new limb filled **forward** from the demuxer's
//!    position;
//! 5. video kept decoding through the audio switch (the unchanged
//!    kind's generation floor — the S4.2 proof on a real container);
//! 6. the clock re-anchored on the new track's first frame's own pts;
//! 7. switch **back** to the original track: the real PCM decoder
//!    instances round-trip through the per-session decoder cache —
//!    build counts stay at one per track, `decoders_reused` counts
//!    the re-activation, and the sink's audio shape returns to the
//!    48 kHz stereo form under the second switch generation;
//! 8. the media ends honestly (`Ended`) after both switches.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{Decoder, DecoderConfig};
use commands::envelope::{Generation, RequestId};
use demux::avi::header::StreamFormat;
use demux::avi::AviDemuxer;
use media_clock::{MonotonicSource, PlaybackClock};
use media_types::frame::SampleFormat;
use media_types::{
    EngineError, Frame, MediaKind, Nanoseconds, SourceUri, Stamp, TimeBase, Timestamp, Track,
    TrackId,
};
use packet_queue::QueueLimits;
use playback_engine::state::SessionState;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput,
    FrameConsumer, QueuePlan, ReceiptResult, SessionCommandKind, SessionConfig, SessionHandle,
    TickBudget, TrackSelector,
};
use plugin_api::{Access, Demuxer};
use testkit::Must;

const FIXTURE: &str = "generated/valid/avi_mjpeg_2pcm.avi";
const EXPECTATION: &str = "expectations/valid/avi_mjpeg_2pcm.avi.toml";

// ---------------------------------------------------------------------------
// The recorded switch plan (parsed from the expectation TOML — the
// committed truth, never a transcribed copy)
// ---------------------------------------------------------------------------

/// The `[switch_plan]` section of the expectation file.
struct SwitchPlan {
    /// Where the switch lands (media time, ns).
    switch_at_ns: i64,
    /// The wired-default audio track (the first audio stream).
    from_id: TrackId,
    /// The switch target (the distinct-shape second audio stream).
    to_id: TrackId,
    /// The post-switch audio shape (the wrong-track-decode oracle).
    post_sample: SampleFormat,
    post_rate: u32,
    post_channels: u32,
}

/// Parse the integers of the expectation's TOML subset (`1_000_000`
/// digit grouping included) — a line scanner, no dependency.
fn toml_int(value: &str) -> i64 {
    value
        .trim()
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect::<String>()
        .parse()
        .must()
}

fn toml_quoted(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string()
}

fn parse_switch_plan() -> SwitchPlan {
    let text = std::fs::read_to_string(testkit::fixture_path(EXPECTATION).must()).must();
    let mut switch_at_ns = None;
    let mut from_id = None;
    let mut to_id = None;
    let mut post_sample = None;
    let mut post_rate = None;
    let mut post_channels = None;
    let mut section = "";
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            section = line.trim_start_matches('[').trim_end_matches(']');
            continue;
        }
        if section != "switch_plan" {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "switch_at_us" => switch_at_ns = Some(toml_int(value) * 1_000),
            "from_track_id" => from_id = Some(TrackId::new(toml_int(value) as u32)),
            "to_track_id" => to_id = Some(TrackId::new(toml_int(value) as u32)),
            "post_switch" => {
                // post_switch = { codec = "pcm_u8", rate = 44100, channels = 1 }
                let inner = value.trim().trim_start_matches('{').trim_end_matches('}');
                for pair in inner.split(',') {
                    let Some((field, field_value)) = pair.split_once('=') else {
                        continue;
                    };
                    match field.trim() {
                        "codec" => {
                            post_sample = Some(match toml_quoted(field_value).as_str() {
                                "pcm_u8" => SampleFormat::U8,
                                "pcm_s16le" => SampleFormat::S16,
                                other => unreachable!("known post-switch codec: {other}"),
                            });
                        }
                        "rate" => post_rate = Some(toml_int(field_value) as u32),
                        "channels" => post_channels = Some(toml_int(field_value) as u32),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    SwitchPlan {
        switch_at_ns: switch_at_ns.must(),
        from_id: from_id.must(),
        to_id: to_id.must(),
        post_sample: post_sample.must(),
        post_rate: post_rate.must(),
        post_channels: post_channels.must(),
    }
}

// ---------------------------------------------------------------------------
// The host halves (virtual clock + collecting sink — the track_switch
// harness shapes, over the real demux/decode stack)
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps, no wall
/// clock — clock-position assertions are deterministic).
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The collecting sink: every delivered frame with its track, in
/// delivery order (the wrong-track-decode observability seam).
#[derive(Default)]
struct Collector {
    frames: Mutex<Vec<(TrackId, Frame)>>,
}

impl FrameConsumer for Collector {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((track, frame));
        Ok(())
    }
}

/// The real decoder provider (the `pipeline_host` registry-shim
/// shape, test-side): MJPEG as-is, PCM seeded with the `strf` facts
/// harvested from the demuxer before it moved into the engine (the
/// `PcmDecoder::with_format` seam — the engine's wire passes no
/// WAVEFORMATEX `extra_data`), with per-track build counting (the
/// recycling oracle).
struct RealProvider {
    pcm_facts: HashMap<u32, (u32, u32)>,
    builds: Mutex<Vec<(TrackId, u32)>>,
}

impl RealProvider {
    fn build_count(&self, track: u32) -> u32 {
        self.builds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|(id, _)| id.handle() == track)
            .map(|(_, count)| *count)
            .sum()
    }
}

impl DecoderProvider for RealProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        {
            let mut builds = self
                .builds
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match builds.iter_mut().find(|(id, _)| *id == track.id) {
                Some((_, count)) => *count += 1,
                None => builds.push((track.id, 1)),
            }
        }
        match track.kind {
            MediaKind::Video => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            MediaKind::Audio => {
                let Some((channels, rate)) = self.pcm_facts.get(&track.id.handle()).copied() else {
                    unreachable!("pcm facts harvested for track {}", track.id.handle());
                };
                let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)?;
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            _ => unreachable!("the fixture carries A/V tracks only"),
        }
    }
}

// ---------------------------------------------------------------------------
// Pacing
// ---------------------------------------------------------------------------

/// The queue caps that keep the demuxer honest for a mid-play switch
/// on this fixture's shape (100 ms interleave granularity, 2 s total):
/// both duration caps sit at a few packets so (a) the prime
/// watermarks (cap − cap/4) are reachable EARLY — a 2 s stream must
/// prime long before it exhausts — and (b) the demuxer never stands
/// more than ~400 ms ahead of the decoded position, so the switch
/// target still has media ahead of the cursor when the recorded
/// switch point is reached. The video limb still banks frames (four)
/// across the audio switch — enough to prove the unchanged kind keeps
/// decoding through the mint without racing the media to EOF.
fn fixture_queues() -> QueuePlan {
    let audio = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(400_000_000),
    };
    let video = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(8 << 20).must(),
        max_duration: Nanoseconds(400_000_000),
    };
    QueuePlan::new(audio, video)
}

/// One packet per limb per poll: the prime watermarks are evaluated
/// AFTER the decode slice, so the slice must stay at or below a
/// quarter of the queue caps (the high watermark's distance from the
/// cap) for the queues to still read `high` post-decode — with
/// 100 ms-granularity packets and 400 ms caps that means one packet
/// per limb. Each poll then advances the media by a bounded,
/// assertable step (~100–200 ms), and the queue never drains to its
/// low watermark while `Playing` (no spurious playing→buffering
/// flash).
fn fixture_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 1,
        flush_calls_per_limb: 8,
    }
}

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

struct Fixture {
    engine: Engine,
    handle: SessionHandle,
    collector: Arc<Collector>,
    clock: Arc<media_clock::MediaClock>,
    provider: Arc<RealProvider>,
    /// The demuxed track list (kept for id-derived assertions — the
    /// video track's id is read from the container, not assumed).
    tracks: Vec<Track>,
}

fn open_fixture() -> Fixture {
    let path = testkit::fixture_path(FIXTURE).must();
    let uri = SourceUri::parse(&format!("file://{}", path.to_str().must())).must();
    let stream = input::FileAccess::new().open(&uri).must();
    let mut avi = AviDemuxer::new(stream).must();
    let tracks = avi.start().must();
    // Harvest the PCM `strf` facts before the demuxer moves into the
    // engine (start() is cached, so the engine's own probe re-reads
    // the same list).
    let mut pcm_facts = HashMap::new();
    for track in &tracks {
        if let Some(StreamFormat::Pcm {
            channels,
            samples_per_sec,
            ..
        }) = avi.stream_format(track.id)
        {
            pcm_facts.insert(track.id.handle(), (u32::from(channels), samples_per_sec));
        }
    }
    let collector = Arc::new(Collector::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(SharedNow(AtomicU64::new(0))),
            Timestamp::new(0, TimeBase::new(1, 1_000_000).must()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let provider = Arc::new(RealProvider {
        pcm_facts,
        builds: Mutex::new(Vec::new()),
    });
    let mut config = SessionConfig::new(
        uri,
        Box::new(avi),
        Arc::clone(&provider) as Arc<dyn DecoderProvider>,
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        Arc::clone(&clock),
    );
    config.queues = fixture_queues();
    config.budget = fixture_budget();
    config.throttle = playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    };
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    Fixture {
        engine,
        handle,
        collector,
        clock,
        provider,
        tracks,
    }
}

fn stamp_ns(stamp: &Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(Nanoseconds(ns)) => Some(i64::try_from(ns).unwrap_or(i64::MAX)),
        Err(_) => None,
    }
}

fn frames_snapshot(collector: &Collector) -> Vec<(TrackId, Frame)> {
    collector
        .frames
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn frames_of<'a>(frames: &'a [(TrackId, Frame)], track: TrackId) -> Vec<&'a Frame> {
    frames
        .iter()
        .filter(|(id, _)| *id == track)
        .map(|(_, frame)| frame)
        .collect()
}

/// The fixture's video track id (read from the demuxed list).
fn video_track(fixture: &Fixture) -> TrackId {
    fixture
        .tracks
        .iter()
        .find(|track| track.kind == MediaKind::Video)
        .must()
        .id
}

/// One poll, with the outbox drained (the helpers keep the output
/// stream empty between observation windows, so a window's ordering
/// assertions see only that window's facts).
fn step(fixture: &mut Fixture) {
    fixture.engine.poll();
    let _ = fixture.engine.take_outputs();
}

fn poll_until(
    fixture: &mut Fixture,
    predicate: impl Fn(SessionState) -> bool,
    limit: usize,
    context: &str,
) {
    step(fixture);
    for _ in 1..limit {
        if predicate(fixture.engine.state()) {
            return;
        }
        step(fixture);
    }
    assert!(
        predicate(fixture.engine.state()),
        "{context}: state is {} after {limit} polls",
        fixture.engine.state()
    );
}

/// Poll until the collector holds an audio frame of `track` at or
/// after `pts_ns` (the decoded position crossing a media time).
fn poll_until_audio_at(
    fixture: &mut Fixture,
    track: TrackId,
    pts_ns: i64,
    limit: usize,
    context: &str,
) {
    let reached = |fixture: &Fixture| {
        frames_snapshot(&fixture.collector).iter().any(|(id, frame)| {
            *id == track && stamp_ns(&frame.pts()).is_some_and(|p| p >= pts_ns)
        })
    };
    step(fixture);
    for _ in 1..limit {
        if reached(fixture) {
            return;
        }
        step(fixture);
    }
    let seen = frames_snapshot(&fixture.collector)
        .iter()
        .filter(|(id, _)| *id == track)
        .count();
    assert!(
        reached(fixture),
        "{context}: no audio frame of track {} at or after {pts_ns} ns within \
         {limit} polls ({seen} frames of the track seen)",
        track.handle()
    );
}

/// Poll until a `tracks.changed` terminal appears, returning the
/// accumulated output stream from the first poll on (the receipt may
/// land a poll or two before the terminal — the fresh queue's
/// flush-gap consumes the first decode slice — so the window is the
/// whole span, preserving the causal order across polls; every
/// helper drains the outbox, so the accumulation starts clean).
fn poll_until_terminal(fixture: &mut Fixture, limit: usize, context: &str) -> Vec<EngineOutput> {
    let mut stream = Vec::new();
    for _ in 0..limit {
        fixture.engine.poll();
        let outputs = fixture.engine.take_outputs();
        let done = outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        });
        stream.extend(outputs);
        if done {
            return stream;
        }
    }
    unreachable!("{context}: no tracks.changed terminal within {limit} polls");
}

fn selection_terms(
    outputs: &[EngineOutput],
) -> Vec<(Generation, MediaKind, Option<TrackId>, Option<RequestId>)> {
    outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                generation,
                origin,
                kind:
                    EngineEventKind::TrackSelectionChanged {
                        track_kind,
                        selected,
                    },
                ..
            }) => Some((*generation, *track_kind, *selected, *origin)),
            _ => None,
        })
        .collect()
}

fn receipt_for(outputs: &[EngineOutput], request: RequestId) -> CommandReceipt {
    outputs
        .iter()
        .find_map(|output| match output {
            EngineOutput::Receipt(receipt) if receipt.request == request => Some(*receipt),
            _ => None,
        })
        .must()
}

fn index_of(outputs: &[EngineOutput], predicate: impl Fn(&EngineOutput) -> bool) -> Option<usize> {
    outputs.iter().position(predicate)
}

// ---------------------------------------------------------------------------
// The AC
// ---------------------------------------------------------------------------

/// The full real-fixture switch story: the recorded plan, played on
/// the real container through the real decoders, twice (there and
/// back — the recycle round-trip).
#[test]
fn real_fixture_audio_switch_follows_the_recorded_plan() {
    let plan = parse_switch_plan();
    let mut fixture = open_fixture();
    let video = video_track(&fixture);

    poll_until(&mut fixture, |s| s == SessionState::Playing, 100, "prime");
    let stats_after_open = fixture.engine.stats();
    assert_eq!(
        stats_after_open.decoders_built, 2,
        "video + the default audio limb wired (one limb per kind)"
    );
    assert_eq!(stats_after_open.decoders_reused, 0);
    assert_eq!(fixture.provider.build_count(plan.from_id.handle()), 1);
    assert_eq!(fixture.provider.build_count(plan.to_id.handle()), 0);

    // ---- Mid-play: play to the recorded switch point. ----
    poll_until_audio_at(
        &mut fixture,
        plan.from_id,
        plan.switch_at_ns,
        100,
        "play to the recorded switch point",
    );
    let frames_at_switch = frames_snapshot(&fixture.collector);
    let video_at_switch = frames_of(&frames_at_switch, video).len();
    let last_from_pts = frames_of(&frames_at_switch, plan.from_id)
        .iter()
        .filter_map(|frame| stamp_ns(&frame.pts()))
        .max()
        .must();

    // The wired default delivered its shape and only its shape up to
    // the switch point: the unwired second audio track decoded
    // nothing (a decode of the wrong track would show at the sink as
    // the from-track's shape on the to-track's id — or vice versa).
    let from_frames = frames_of(&frames_at_switch, plan.from_id);
    assert!(
        !from_frames.is_empty(),
        "the default audio limb decoded on the way to the switch point"
    );
    for frame in &from_frames {
        match frame {
            Frame::Audio(audio) => {
                assert_eq!(audio.format.channels, 2);
                assert_eq!(audio.format.rate, 48_000);
                assert_eq!(audio.format.sample, SampleFormat::S16);
            }
            other => unreachable!("an audio limb delivers audio: {other:?}"),
        }
    }
    assert!(
        frames_of(&frames_at_switch, plan.to_id).is_empty(),
        "the unwired second audio track decoded nothing before the switch"
    );
    let switch_generation = fixture.engine.generation().must().0 + 1;

    // ---- The switch: mint → drain → re-prime → tracks.changed. ----
    let request = RequestId(41);
    fixture
        .handle
        .set_audio_track(request, TrackSelector::Id { id: plan.to_id })
        .must();
    let outputs = poll_until_terminal(&mut fixture, 50, "the switch completes");

    // (a) The receipt precedes every event it caused and carries the
    // post-bump generation.
    let receipt_index = index_of(
        &outputs,
        |output| matches!(output, EngineOutput::Receipt(r) if r.request == request),
    )
    .must();
    let receipt = receipt_for(&outputs, request);
    assert_eq!(receipt.command, SessionCommandKind::SelectAudio);
    match receipt.result {
        ReceiptResult::Accepted { generation, .. } => {
            assert_eq!(generation, Generation(switch_generation));
        }
        other => unreachable!("the switch accepts: {other:?}"),
    }
    // (b) The C8 confirmation follows the receipt: playing→playing.
    let confirm_index = index_of(&outputs, |output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged {
                    to: SessionState::Playing,
                    ..
                },
                ..
            })
        )
    })
    .must();
    assert!(
        receipt_index < confirm_index,
        "receipt precedes the state-changed confirmation"
    );
    // (c) The terminal: exactly one, audio, the target track, under
    // the switch's own generation, caused by the request.
    assert_eq!(
        selection_terms(&outputs),
        vec![(
            Generation(switch_generation),
            MediaKind::Audio,
            Some(plan.to_id),
            Some(request)
        )],
        "exactly one tracks.changed, under the switch's own generation"
    );
    // (d) No limb ended at the retire (a retired limb is not "newly
    // drained" — no end-of-track fires for it).
    assert!(
        !outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::EndOfTrack { .. },
                    ..
                })
            )
        }),
        "the retired limb fired no end-of-track"
    );

    // ---- The sink truth. ----
    let frames = frames_snapshot(&fixture.collector);
    let to_frames = frames_of(&frames, plan.to_id);
    let first_new = to_frames.first().must();
    // (e) The re-prime: the new track's first frame follows the
    // switch generation (doc 03 §8's pass condition).
    assert_eq!(
        first_new.generation().counter(),
        switch_generation,
        "the new track's first frame follows the switch generation"
    );
    // (f) Wrong-track-decode observability: every new-track frame
    // carries the recorded post-switch shape — never the old one.
    for frame in &to_frames {
        match frame {
            Frame::Audio(audio) => {
                assert_eq!(audio.format.sample, plan.post_sample);
                assert_eq!(audio.format.rate, plan.post_rate);
                assert_eq!(audio.format.channels, plan.post_channels);
            }
            other => unreachable!("an audio limb delivers audio: {other:?}"),
        }
    }
    // (g) The drain: no old-track frame past the boundary — every
    // from-track frame carries a pre-mint stamp, and the newest
    // from-track frame is the one already decoded when the switch
    // landed (the retired limb's queued residents died at the flush).
    let from_after = frames_of(&frames, plan.from_id);
    assert!(
        from_after
            .iter()
            .all(|frame| frame.generation().counter() < switch_generation),
        "no old-track frame past the switch boundary"
    );
    let last_from_after = from_after
        .iter()
        .filter_map(|frame| stamp_ns(&frame.pts()))
        .max()
        .must();
    assert_eq!(
        last_from_after, last_from_pts,
        "the retired limb's queued residents never decoded"
    );
    // (h) The new limb filled forward from the demuxer's position
    // (never rewound past the recorded switch point).
    let first_new_pts = stamp_ns(&first_new.pts()).must();
    assert!(
        first_new_pts >= plan.switch_at_ns,
        "the new limb filled forward ({first_new_pts} ns >= {} ns)",
        plan.switch_at_ns
    );
    // (i) The unchanged kind kept decoding through the mint: video
    // frames kept arriving past the switch (the per-limb generation
    // floor — the S4.2 proof on the real container).
    let video_after = frames_of(&frames, video).len();
    assert!(
        video_after > video_at_switch,
        "video kept decoding through the audio switch ({video_after} > {video_at_switch})"
    );
    // (j) The clock re-anchored on the new track's first frame's own
    // pts (anchor on truth, never the freeze position).
    let clock_ns = match fixture.clock.now().as_nanos() {
        Ok(Nanoseconds(ns)) => i64::try_from(ns).unwrap_or(i64::MAX),
        Err(_) => 0,
    };
    assert_eq!(clock_ns, first_new_pts, "the clock anchors on the landing truth");

    // ---- The recycle round-trip: switch back to the original. ----
    let back_generation = fixture.engine.generation().must().0 + 1;
    let back_request = RequestId(42);
    fixture
        .handle
        .set_audio_track(back_request, TrackSelector::Id { id: plan.from_id })
        .must();
    let back_outputs = poll_until_terminal(&mut fixture, 50, "the switch-back completes");
    assert_eq!(
        selection_terms(&back_outputs),
        vec![(
            Generation(back_generation),
            MediaKind::Audio,
            Some(plan.from_id),
            Some(back_request)
        )],
        "the switch-back terminal, under its own generation"
    );

    // The decoder cache round-trip on the REAL decoders: each track's
    // decoder was built exactly once (the cache served the
    // re-activations) — D-1's no-fresh-context-per-switch.
    assert_eq!(
        fixture.provider.build_count(plan.from_id.handle()),
        1,
        "the from-track PCM decoder was never rebuilt"
    );
    assert_eq!(
        fixture.provider.build_count(plan.to_id.handle()),
        1,
        "the to-track PCM decoder was built exactly once"
    );
    assert_eq!(
        fixture.provider.build_count(video.handle()),
        1,
        "the MJPEG limb never rebuilt"
    );
    let stats = fixture.engine.stats();
    assert_eq!(
        stats.decoders_built, 3,
        "video + both audio decoders, once each"
    );
    assert_eq!(stats.decoders_reused, 1, "the switch-back was a cache hit");

    // The sink shape returned to the from-track's, under the second
    // switch generation, filled forward from the demuxer's position.
    let frames = frames_snapshot(&fixture.collector);
    let first_back = frames_of(&frames, plan.from_id)
        .into_iter()
        .find(|frame| frame.generation().counter() == back_generation)
        .must();
    match first_back {
        Frame::Audio(audio) => {
            assert_eq!(audio.format.channels, 2);
            assert_eq!(audio.format.rate, 48_000);
            assert_eq!(audio.format.sample, SampleFormat::S16);
        }
        other => unreachable!("an audio limb delivers audio: {other:?}"),
    }
    let first_back_pts = stamp_ns(&first_back.pts()).must();
    assert!(
        first_back_pts >= first_new_pts,
        "the re-activation filled forward from the demuxer's position"
    );
    let clock_ns = match fixture.clock.now().as_nanos() {
        Ok(Nanoseconds(ns)) => i64::try_from(ns).unwrap_or(i64::MAX),
        Err(_) => 0,
    };
    assert_eq!(
        clock_ns, first_back_pts,
        "the clock re-anchored on the re-activation's landing truth"
    );

    // ---- The media ends honestly after both switches. ----
    poll_until(&mut fixture, |s| s == SessionState::Ended, 100, "run to ended");
    let frames = frames_snapshot(&fixture.collector);
    assert!(
        frames_of(&frames, video).len() >= 18,
        "the video limb decoded (nearly) the whole stream"
    );
    assert!(
        !frames_of(&frames, plan.to_id).is_empty() && !frames_of(&frames, plan.from_id).is_empty(),
        "both audio tracks delivered across the switches"
    );
}

// ---------------------------------------------------------------------------
// Sanity pins for the harness itself (D-5: the parsing is part of
// the proof surface)
// ---------------------------------------------------------------------------

/// The expectation parser reads the recorded plan, not a copy: the
/// committed values for the multi-track parent, as written.
#[test]
fn the_recorded_switch_plan_parses() {
    let plan = parse_switch_plan();
    assert_eq!(plan.switch_at_ns, 1_000_000_000);
    assert_eq!(plan.from_id, TrackId::new(1));
    assert_eq!(plan.to_id, TrackId::new(2));
    assert_eq!(plan.post_sample, SampleFormat::U8);
    assert_eq!(plan.post_rate, 44_100);
    assert_eq!(plan.post_channels, 1);
}

/// The decoder cache dies with its session (epoch invariant S4 — no
/// cross-source reuse): closing and re-opening the same file builds
/// fresh decoders, never carries instances across.
#[test]
fn the_decoder_cache_dies_with_the_session() {
    let plan = parse_switch_plan();
    let mut first = open_fixture();
    poll_until(&mut first, |s| s == SessionState::Playing, 100, "prime");
    first
        .handle
        .set_audio_track(RequestId(7), TrackSelector::Id { id: plan.to_id })
        .must();
    let _ = poll_until_terminal(&mut first, 50, "the switch completes");
    assert_eq!(first.provider.build_count(plan.to_id.handle()), 1);
    // The v0 exit protocol: stop from playback, close from the
    // settled state — then the cache died with the session.
    first.handle.stop(RequestId(8)).must();
    poll_until(&mut first, |s| s == SessionState::Stopped, 50, "stop settles");
    first.handle.close(RequestId(9)).must();
    poll_until(&mut first, |s| s == SessionState::Idle, 50, "close settles");
    drop(first);

    let mut second = open_fixture();
    poll_until(&mut second, |s| s == SessionState::Playing, 100, "re-prime");
    // A fresh session built its own decoders (no cross-session
    // instance reuse — the cache is per-pipeline by construction).
    assert_eq!(second.provider.build_count(plan.from_id.handle()), 1);
    assert_eq!(second.engine.stats().decoders_reused, 0);
}
