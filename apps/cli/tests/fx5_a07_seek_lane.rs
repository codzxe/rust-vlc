//! FX5-A07 — the engine seek lane's host half, verified end-to-end
//! (the wave-20 F-001 row): the engine's ONLY in-session seek entry is
//! the demuxer's `reanchor` (playback-engine pipeline.rs:1499 →
//! `SelectDemuxer::reanchor` (demux_host.rs:606) → the inner demuxer),
//! and the terminals ride `EngineEventKind::Seeked`/`SeekFailed` →
//! `timing.seek-completed` / `timing.seek-failed` (engine event.rs
//! :547/:551).
//!
//! What is pinned here, per container lane, through the REAL open path
//! (`lumen_cli::demux_host::open_source` — the same wrapper object the
//! engine's `SessionConfig` receives, host.rs:972-978):
//!
//! - **WAV** (the inner demuxer that DOES implement `reanchor` —
//!   demux/wav mod.rs:1095; the W20-A05 executed reference): the pure
//!   `seek` query and the acting `reanchor` agree on one landing
//!   through the wrapper, a successful backward reanchor clears the
//!   open-time skip watermark (demux_host.rs:616), and a
//!   `playback.seek` through the full CLI dispatch mints exactly one
//!   `timing.seek-completed` at the actual landing — the exact
//!   machinery the MP4 lane rides, proven live with an inner that can
//!   land.
//! - **MP4** (the double-wrapper lane: `SelectDemuxer` → the
//!   `StartedMp4` start-idempotence adapter (containers/mp4.rs:155)
//!   → `Mp4Demuxer`): the capability truth forwards through BOTH
//!   hops — `Fast { indexed: true }` (mp4/mod.rs:2063) — the
//!   "seekable: true" half of the executed F-001, and the acting
//!   `reanchor` call itself reaches the inner demuxer through both
//!   hops and lands (mp4/mod.rs:2086, FX5-A01's F1.3 implementation).
//!   The failed half of the executed F-001 was the inner's
//!   trait-default `reanchor` refusal alone; nothing in the wrapper
//!   lane swallowed either direction.

use std::path::PathBuf;

use commands::catalog::types::{OpenOptions, SeekTarget, SourceSpec, Timestamp};
use commands::envelope::RequestId;
use commands::{Command, EventKind, PlaybackState, WireRequest};
use lumen_cli::demux_host::{open_source, OpenedMedia};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use media_types::time::{Stamp, TimeSpan};
use plugin_api::{DemuxEvent, SeekSupport};

/// A committed fixture under the root `tests/fixtures`.
fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

/// The WAV lane's fixture: 8_820 stereo s16 frames @ 44.1 kHz = 200 ms
/// declared (one `Fast`-but-indexless container whose `reanchor` is
/// real — the demux/wav reference).
fn wav_fixture() -> PathBuf {
    fixture("generated/valid/wav_pcm_s16le_44k1_st.wav")
}

/// The MP4 lane's fixture: h264 + aac, progressive tables.
fn mp4_fixture() -> PathBuf {
    fixture("generated/valid/mp4_h264_aac.mp4")
}

/// Open the WAV lane through the real CLI open path (probe → container
/// host → the wrapped demuxer the engine would receive).
fn open_wav(start_time_us: Option<i64>) -> OpenedMedia {
    match open_source(
        &SourceSpec::File {
            path: wav_fixture(),
        },
        start_time_us,
        false,
        false,
    ) {
        Ok(opened) => opened,
        Err(err) => unreachable!("{err}"),
    }
}

/// A present stamp's ticks (the lane's single track base).
fn stamp_ticks(stamp: &Stamp) -> i64 {
    match stamp {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("the lane's stamps are present"),
    }
}

/// A present stamp's nanoseconds (base-agnostic; saturating on the
/// theoretical `u64` overflow).
fn stamp_ns(stamp: &Stamp) -> i64 {
    match stamp {
        Stamp::Present(ts) => match ts.as_nanos() {
            Ok(ns) => i64::try_from(ns.0).unwrap_or(i64::MAX),
            Err(err) => unreachable!("{err:?}"),
        },
        Stamp::Missing => unreachable!("the lane's landings are present"),
    }
}

/// Dispatch one command, asserting the envelope's success.
fn dispatch(host: &mut CliHost, request_id: u64, command: Command) {
    let response = lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(request_id),
            command,
        },
    );
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("command failed: {error}"),
    }
}

/// The pure `seek` query and the engine's acting `reanchor` agree on
/// one keyframe-safe landing through the wrapped lane object, and the
/// media continues from the landing — the wrapper forwards the acting
/// half to the real container demuxer verbatim (the same wrapper the
/// engine holds; the engine's only seek entry, pipeline.rs:1499).
#[test]
fn fx5_a07_lane_pure_query_and_reanchor_agree_on_a_real_wrapper_lane() {
    let opened = open_wav(None);
    let mut demuxer = opened.demuxer;
    let track = &opened.facts.tracks[0];
    // The engine's step-1 capability consult (`pipeline.seekability`)
    // reads the inner truth through the wrapper: WAV's own
    // format-inherent `Fast` — forwarded, not invented.
    assert_eq!(demuxer.seekability(), SeekSupport::Fast { indexed: false });
    // A target at half the declared extent, in the track's own base.
    let end_ticks = match &track.timing.span.end {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("the WAV fixture declares its sample-frame extent"),
    };
    let target = TimeSpan {
        start: Stamp::Present(media_types::time::Timestamp::new(
            end_ticks / 2,
            track.timing.base,
        )),
        end: Stamp::Missing,
    };
    // The pure query (the `--start-time` validation's own seam)…
    let queried = match demuxer.seek(target) {
        Ok(span) => span,
        Err(err) => unreachable!("{err}"),
    };
    // …and the acting half the engine calls at step 4: one landing.
    let landed = match demuxer.reanchor(target) {
        Ok(span) => span,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(
        stamp_ticks(&queried.start),
        stamp_ticks(&landed.start),
        "the query and the acting half report one landing"
    );
    let landing_ticks = stamp_ticks(&landed.start);
    assert!(
        landing_ticks <= end_ticks / 2,
        "the landing is the keyframe-safe position ≤ the target"
    );
    // The media continues from the landing: every surviving packet
    // presents at/after it, and the tail still delivers.
    let mut packets = 0u32;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                packets += 1;
                match &packet.pts {
                    Stamp::Present(ts) => assert!(
                        ts.ticks >= landing_ticks,
                        "a pre-landing timestamp reappeared: {} < {}",
                        ts.ticks,
                        landing_ticks
                    ),
                    Stamp::Missing => unreachable!("WAV packets carry present stamps"),
                }
            }
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(_) => {}
            Err(err) => unreachable!("{err}"),
        }
    }
    assert!(packets > 0, "the tail of the media must still deliver");
}

/// A successful backward reanchor clears the open-time skip watermark
/// (demux_host.rs:616): `--start-time 150 ms` of the 200 ms fixture,
/// then a reanchor to the quarter extent — the landing packet sits
/// BELOW the watermark and still flows. A surviving watermark would
/// have dropped it (the pre-FX5-A01 starved-backward-seek shape).
#[test]
fn fx5_a07_lane_reanchor_clears_the_start_time_watermark() {
    let opened = open_wav(Some(150_000));
    assert_eq!(opened.start_position_ns, 150_000_000);
    let mut demuxer = opened.demuxer;
    let track = &opened.facts.tracks[0];
    let end_ticks = match &track.timing.span.end {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("the WAV fixture declares its sample-frame extent"),
    };
    // A backward reanchor below the 150 ms watermark: the quarter extent.
    let target = TimeSpan {
        start: Stamp::Present(media_types::time::Timestamp::new(
            end_ticks / 4,
            track.timing.base,
        )),
        end: Stamp::Missing,
    };
    let landed = match demuxer.reanchor(target) {
        Ok(span) => span,
        Err(err) => unreachable!("{err}"),
    };
    let landing_ticks = stamp_ticks(&landed.start);
    // The landing packet flows BELOW the watermark: the successful
    // reanchor cleared the open-time filter at the landing.
    match demuxer.next_event() {
        Ok(DemuxEvent::Packet(packet)) => match &packet.pts {
            Stamp::Present(ts) => {
                assert!(
                    ts.ticks < end_ticks * 3 / 4,
                    "the early landing packet flows — the watermark was cleared: {}",
                    ts.ticks
                );
                assert!(
                    ts.ticks >= landing_ticks,
                    "the stream resumes at the landing: {} < {}",
                    ts.ticks,
                    landing_ticks
                );
            }
            Stamp::Missing => unreachable!("WAV packets carry present stamps"),
        },
        other => unreachable!("the landing packet must flow: {other:?}"),
    }
}

/// The MP4 double wrapper (SelectDemuxer → StartedMp4 → Mp4Demuxer)
/// reports the inner demuxer's capability through BOTH hops:
/// `Fast { indexed: true }` (the progressive sample tables,
/// mp4/mod.rs:2063) — the "seekable: true" the audit's executed F-001
/// reported while every seek still failed. The wrapper swallows
/// neither the capability (this test) nor the inner's typed
/// `reanchor` refusal (the demux_host unit test) — the executed
/// failure was the inner's missing `reanchor` alone (FX5-A01's row).
#[test]
fn fx5_a07_lane_seekability_reports_the_inner_demuxer_through_both_wrappers() {
    let opened = match open_source(
        &SourceSpec::File {
            path: mp4_fixture(),
        },
        None,
        false,
        false,
    ) {
        Ok(opened) => opened,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(
        opened.demuxer.seekability(),
        SeekSupport::Fast { indexed: true },
        "the double wrapper forwards the inner's declared capability"
    );
}

/// The acting half reaches the MP4 demuxer through BOTH wrappers — the
/// F1.3 lane FX5-A07 exists to verify: `SelectDemuxer::reanchor`
/// (demux_host.rs:606) → the `StartedMp4` adapter
/// (containers/mp4.rs:173) → `Mp4Demuxer::reanchor`
/// (mp4/mod.rs:2086, FX5-A01's implementation). The pure query and the
/// acting call agree on one keyframe-safe landing ≤ the target, and
/// the media still delivers through the wrapper afterwards.
#[test]
fn fx5_a07_lane_reanchor_reaches_the_mp4_demuxer_through_both_wrappers() {
    let opened = match open_source(
        &SourceSpec::File {
            path: mp4_fixture(),
        },
        None,
        false,
        false,
    ) {
        Ok(opened) => opened,
        Err(err) => unreachable!("{err}"),
    };
    let mut demuxer = opened.demuxer;
    assert_eq!(demuxer.seekability(), SeekSupport::Fast { indexed: true });
    // A target at half the declared extent (ns; the demuxer's preamble
    // converts any base).
    let duration_us = match opened.facts.duration_us() {
        Some(us) => us,
        None => unreachable!("the fixture declares its extent"),
    };
    let target = TimeSpan {
        start: Stamp::Present(media_types::time::Timestamp::new(
            duration_us.saturating_mul(500),
            media_clock::NANO_BASE,
        )),
        end: Stamp::Missing,
    };
    let queried = match demuxer.seek(target) {
        Ok(span) => span,
        Err(err) => unreachable!("{err}"),
    };
    let landed = match demuxer.reanchor(target) {
        Ok(span) => span,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(
        stamp_ns(&queried.start),
        stamp_ns(&landed.start),
        "the pure query and the acting half agree on one landing"
    );
    let landing_ns = stamp_ns(&landed.start);
    assert!(
        landing_ns >= 0 && landing_ns <= duration_us * 1000 / 2,
        "the landing is the keyframe-safe position ≤ the half-extent target: {landing_ns} ns"
    );
    // The stream still delivers through the wrapper from the landing.
    let mut packets = 0u32;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(_)) => packets += 1,
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(_) => {}
            Err(err) => unreachable!("{err}"),
        }
    }
    assert!(packets > 0, "the tail of the media must still deliver");
}

/// The success terminal fires on a wrapper lane that can land: a
/// `playback.seek 50%` through the full CLI dispatch (host.rs
/// dispatch_seek → the engine's seven-step flow) mints exactly one
/// `timing.seek-completed` at the ACTUAL landing — half of the
/// 200 ms fixture at the sample-frame landing — and the clock
/// re-anchors there. This is the W20-A05 executed WAV reference made
/// in-tree: the same machinery the MP4 lane rides, with an inner
/// `reanchor` that works.
#[test]
fn fx5_a07_lane_seek_emits_the_completed_terminal_on_a_wrapper_lane_that_lands() {
    let mut host = CliHost::new();
    dispatch(
        &mut host,
        1,
        Command::SessionOpen {
            source: SourceSpec::File {
                path: wav_fixture(),
            },
            options: OpenOptions::default(),
        },
    );
    host.settle();
    assert_eq!(
        host.state(),
        PlaybackState::Playing,
        "the seek needs a live Playing session"
    );
    dispatch(
        &mut host,
        2,
        Command::PlaybackSeek {
            target: SeekTarget::Percent { fraction: 0.5 },
        },
    );
    // The terminal: the landing truth (µs), never the request.
    let completed = host.events().iter().find_map(|event| match event.kind {
        EventKind::TimingSeekCompleted { position, .. } => Some(position),
        _ => None,
    });
    assert_eq!(
        completed,
        Some(Timestamp(100_000)),
        "half of the 200 ms fixture, at the sample-frame landing"
    );
    assert_eq!(
        host.position_us(),
        100_000,
        "the clock re-anchored on the first audio frame's own timestamp"
    );
    assert!(
        !host
            .events()
            .iter()
            .any(|event| matches!(event.kind, EventKind::TimingSeekFailed { .. })),
        "a landing seek fabricates no failure terminal"
    );
}
