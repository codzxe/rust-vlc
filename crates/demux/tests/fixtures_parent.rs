//! The parent-fixture acceptance suite (card 2.B2 AC): the generated
//! AVI demuxes **fully** through the real file-access stack — every
//! video packet hash-matches the expectation's per-frame sha256, the
//! concatenated PCM hash-matches the stream digest, and the
//! track/duration metadata matches the writer-declared truth.

mod common;

use common::{open_via_input, parse_expectation, sha256_hex};
use demux::avi::AviDemuxer;
use media_types::ids::MediaKind;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

const PARENT: &str = "generated/valid/avi_mjpeg_pcm.avi";
const PARENT_EXPECTATION: &str = "expectations/valid/avi_mjpeg_pcm.avi.toml";

fn open_parent() -> AviDemuxer {
    let path = testkit::fixture_path(PARENT).must();
    let stream = open_via_input(&path).must();
    match AviDemuxer::new(stream) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("parent opens through the access stack: {err}"),
    }
}

#[test]
fn sha256_impl_matches_the_recorded_file_digest() {
    // Pin the test-side SHA-256 against the corpus's committed
    // whole-file hash before any digest assertions rely on it.
    let bytes = testkit::fixture_bytes(PARENT).must();
    let expect = parse_expectation(PARENT_EXPECTATION);
    assert_eq!(sha256_hex(&bytes), expect.sha256);
    assert_eq!(bytes.len() as u64, expect.size_bytes);
}

#[test]
fn parent_track_table_matches_the_expectation() {
    let expect = parse_expectation(PARENT_EXPECTATION);
    let mut avi = open_parent();
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("parent start: {err}"),
    };
    assert_eq!(tracks.len() as u64, expect.track_count);

    let video = &tracks[0];
    assert_eq!(video.kind, MediaKind::Video);
    assert_eq!(
        video.codec.as_str(),
        expect.video_codec.as_deref().unwrap_or("")
    );
    assert_eq!(
        avi.stream_format(video.id),
        Some(demux::avi::header::StreamFormat::MotionJpeg {
            width: expect.video_width.unwrap_or(0),
            height: expect.video_height.unwrap_or(0),
        })
    );
    // The expectation records frames-per-second (num = 30, den = 1);
    // the track base is its reciprocal (one tick = 1/30 s).
    let (num, den) = video.timing.base.as_parts();
    let (fps_num, fps_den) = expect.video_fps.unwrap_or((0, 0));
    assert_eq!((num, den), (fps_den, fps_num));

    let audio = &tracks[1];
    assert_eq!(audio.kind, MediaKind::Audio);
    assert_eq!(
        audio.codec.as_str(),
        expect.audio_codec.as_deref().unwrap_or("")
    );
    match avi.stream_format(audio.id) {
        Some(demux::avi::header::StreamFormat::Pcm {
            channels,
            samples_per_sec,
            ..
        }) => {
            assert_eq!(
                u32::from(channels),
                u64::from(expect.audio_channels.unwrap_or(0)) as u32
            );
            assert_eq!(samples_per_sec, expect.audio_rate.unwrap_or(0));
        }
        other => unreachable!("audio format, got {other:?}"),
    }

    // Duration: the video track's declared span is 2_000_000 µs.
    let duration = match (
        video.timing.span.start.timestamp(),
        video.timing.span.end.timestamp(),
    ) {
        (Ok(start), Ok(end)) => {
            let base = TimeBase::new(1, 1_000_000).must();
            match end.convert(&base) {
                Ok(end_ns) => {
                    end_ns.ticks
                        - match start.convert(&base) {
                            Ok(s) => s.ticks,
                            Err(err) => unreachable!("{err}"),
                        }
                }
                Err(err) => unreachable!("{err}"),
            }
        }
        _ => unreachable!("video span is fully present"),
    };
    assert_eq!(duration as u64, expect.duration_us);
}

#[test]
fn parent_demuxes_fully_per_frame_hash_and_pcm_digest() {
    let expect = parse_expectation(PARENT_EXPECTATION);
    let mut avi = open_parent();
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("parent start: {err}"),
    };
    assert_eq!(tracks.len(), 2);

    let mut video_hashes: Vec<String> = Vec::new();
    let mut pcm = Vec::new();
    let mut video_pts: Vec<i64> = Vec::new();
    let mut audio_pts: Vec<i64> = Vec::new();
    let mut recovered = 0u32;
    loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => match packet.track.handle() {
                0 => {
                    video_hashes.push(sha256_hex(packet.data.as_slice()));
                    match packet.pts {
                        Stamp::Present(ts) => video_pts.push(ts.ticks),
                        Stamp::Missing => unreachable!("video pts present"),
                    }
                }
                1 => {
                    pcm.extend_from_slice(packet.data.as_slice());
                    match packet.pts {
                        Stamp::Present(ts) => audio_pts.push(ts.ticks),
                        Stamp::Missing => unreachable!("audio pts present"),
                    }
                }
                other => unreachable!("parent has streams 0/1, got {other}"),
            },
            Ok(DemuxEvent::Recovered { .. }) => recovered += 1,
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Discontinuity) => {}
            Err(err) => unreachable!("parent demuxes cleanly: {err}"),
        }
    }

    // Every video packet hash-matches, in order.
    assert_eq!(
        video_hashes.len() as u64,
        expect.video_sample_count,
        "frame count"
    );
    assert_eq!(video_hashes.len(), expect.video_sample_sha256.len());
    for (index, (got, want)) in video_hashes
        .iter()
        .zip(&expect.video_sample_sha256)
        .enumerate()
    {
        assert_eq!(got, want, "frame {index} sha256");
    }
    // Concatenated PCM hash-matches the stream digest.
    assert_eq!(pcm.len() as u64, expect.audio_stream_bytes);
    assert_eq!(sha256_hex(&pcm), expect.audio_stream_sha256);
    // The parent is a clean container: no bounded recovery happened.
    assert_eq!(recovered, 0, "parent demuxes with zero damage");
    assert_eq!(avi.damage_total(), 0);

    // PTS ladders: video frame i at tick i (1/30 base), audio chunk i
    // at tick i*1470 (1/44100 base) — monotone within a track by
    // construction of the interleave.
    assert_eq!(video_pts, (0..60).collect::<Vec<i64>>());
    let audio_want: Vec<i64> = (0..60).map(|i| i * 1470).collect();
    assert_eq!(audio_pts, audio_want);
}

#[test]
fn parent_seek_targets_land_within_tolerance() {
    let expect = parse_expectation(PARENT_EXPECTATION);
    let mut avi = open_parent();
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("parent start: {err}"),
    }
    let nanos = TimeBase::new(1, 1_000_000_000).must();
    for target_us in &expect.seek_targets_us {
        let target = TimeSpan {
            start: Stamp::Present(Timestamp::new(*target_us as i64 * 1_000, nanos)),
            end: Stamp::Missing,
        };
        let landing = match avi.seek(target) {
            Ok(span) => span,
            Err(err) => unreachable!("seek {target_us}µs: {err}"),
        };
        let landed_us = match landing.start.timestamp() {
            Ok(ts) => match ts.as_nanos() {
                Ok(ns) => ns.0 / 1_000,
                Err(err) => unreachable!("{err}"),
            },
            Err(_) => unreachable!("landing start present"),
        };
        let delta = landed_us.abs_diff(*target_us);
        assert!(
            delta <= expect.seek_tolerance_us,
            "seek {target_us}µs landed at {landed_us}µs (delta {delta}µs > tolerance {}µs)",
            expect.seek_tolerance_us
        );
        assert!(
            landed_us <= *target_us,
            "seek lands at-or-below the target (keyframe-safe)"
        );
    }
}

#[test]
fn parent_events_survive_the_fuzz_harness() {
    // The +FB harness over the real corpus bytes: construct, drain,
    // probe seeks — no panic, no hang (the libFuzzer campaign itself
    // is CI wiring; this pins the harness logic on real media).
    let bytes = testkit::fixture_bytes(PARENT).must();
    demux::avi::fuzz::fuzz_demux_avi(&bytes);
}

#[test]
fn parent_reanchor_continues_the_stream_within_tolerance() {
    // The acting half of the seek over the REAL corpus (card 3.A1):
    // every recorded target re-anchors within the expectation's
    // tolerance, the stream continues from the landing (video packets
    // match the recorded per-frame hashes from the landing onward),
    // and a past-end target is a typed refusal that leaves the stream
    // playable at its position (D4).
    let expect = parse_expectation(PARENT_EXPECTATION);
    let nanos = TimeBase::new(1, 1_000_000_000).must();
    for target_us in &expect.seek_targets_us {
        let target = TimeSpan {
            start: Stamp::Present(Timestamp::new(*target_us as i64 * 1_000, nanos)),
            end: Stamp::Missing,
        };
        // The from-start truth for this target: re-open and index the
        // video packets by their per-frame hashes.
        let mut fresh = open_parent();
        match fresh.start() {
            Ok(_) => {}
            Err(err) => unreachable!("parent start: {err}"),
        }
        let mut truth_hashes: Vec<String> = Vec::new();
        loop {
            match fresh.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if matches!(packet.track.handle(), 0) {
                        truth_hashes.push(sha256_hex(packet.data.as_slice()));
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("clean drain: {err}"),
            }
        }
        assert_eq!(
            truth_hashes.len() as u64,
            expect.video_sample_count,
            "the parent still demuxes fully"
        );

        let mut avi = open_parent();
        match avi.start() {
            Ok(_) => {}
            Err(err) => unreachable!("parent start: {err}"),
        }
        let landing = match avi.reanchor(target) {
            Ok(span) => span,
            Err(err) => unreachable!("reanchor {target_us}µs: {err}"),
        };
        let landed_us = match landing.start.timestamp() {
            Ok(ts) => match ts.as_nanos() {
                Ok(ns) => ns.0 / 1_000,
                Err(err) => unreachable!("{err}"),
            },
            Err(_) => unreachable!("landing start present"),
        };
        let delta = landed_us.abs_diff(*target_us);
        assert!(
            delta <= expect.seek_tolerance_us,
            "reanchor {target_us}µs landed at {landed_us}µs (delta {delta}µs > tolerance {}µs)",
            expect.seek_tolerance_us
        );
        assert!(
            landed_us <= *target_us,
            "reanchor lands at-or-below the target (keyframe-safe)"
        );

        // The stream continues from the landing: the remaining video
        // packets' hashes are exactly the from-start tail.
        let mut resumed_hashes: Vec<String> = Vec::new();
        loop {
            match avi.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if matches!(packet.track.handle(), 0) {
                        resumed_hashes.push(sha256_hex(packet.data.as_slice()));
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("post-seek drain: {err}"),
            }
        }
        // Where the landing sits in the from-start frame order: the
        // landing time is the landing frame's own pts, so its index is
        // ceil(landed_us × fps) — the µs truncation of a fractional
        // frame time (33 333.33 µs @ 30 fps) must round back up to the
        // frame it belongs to.
        let fps = expect.video_fps.must();
        let numer = landed_us.saturating_mul(u64::from(fps.0));
        let denom = 1_000_000u64.saturating_mul(u64::from(fps.1));
        let landing_frame = numer.div_ceil(denom).min(expect.video_sample_count);
        assert_eq!(
            resumed_hashes,
            truth_hashes[landing_frame as usize..],
            "the post-seek video stream is the from-start tail from the landing"
        );

        // Past the declared end: typed refusal, the stream stays
        // playable at its (now end) position (D4).
        let past = TimeSpan {
            start: Stamp::Present(Timestamp::new(
                (expect.duration_us as i64 + 10_000_000) * 1_000,
                nanos,
            )),
            end: Stamp::Missing,
        };
        let err = match avi.reanchor(past) {
            Ok(_) => unreachable!("past-end reanchor is typed"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.avi.seek.out-of-range");
        match avi.next_event() {
            Ok(DemuxEvent::EndOfStream) => {}
            Ok(other) => unreachable!("exhausted stream stays exhausted, got {other:?}"),
            Err(err) => unreachable!("still parseable: {err}"),
        }
    }
}
