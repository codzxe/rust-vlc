//! Scenario 14 — **short media** (doc 10 §6.4 row 14), driven through
//! the command layer.
//!
//! Pass criteria: edge cases per doc 03 §4.6 (the `Ended` vs
//! `Buffering` race): a clip whose whole payload fits under the
//! high-watermark must **start with what exists** — `EOF before
//! watermark` answers immediate `Playing`, then a quick drain to
//! `Ended` — never a session stuck in `Buffering` waiting for a
//! watermark that EOF has already made unreachable.
//!
//! The fixtures (per the card: 1-frame fixtures; doc 10's driver row
//! names the 0.1 s clip):
//!
//! - **Real tiny AVIs**, built at load time from the committed class-A
//!   parent by documented byte surgery (`host::TempDir::
//!   build_short_avi`): a 1-frame clip (1 video + 1 audio chunk,
//!   33.3 ms) and the 0.1 s clip (3 frame pairs). They are
//!   **well-formed** — declared counts patched to the honest short
//!   numbers — so the pass shape is a CLEAN play-out: `Playing →
//!   Ended`, exactly one `playback.end-of-media`, **zero** error
//!   events (damage is scenario 9's corrupt matrix, not shortness).
//! - **Scripted timelines** (the demux-stage view): a 1-packet
//!   timeline (the same race at the fake-demux seam) and the
//!   zero-packet timeline (EOF before *any* packet — the degenerate
//!   short edge; the observed, pinned behavior is documented below).
//!
//! Determinism: every observable is a pure function of the pump count
//! (the poll model); the tiny AVIs are pure functions of the committed
//! parent bytes. No wall-clock waiting anywhere.

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{ScenarioHost, TempDir, count_events, dispatch, error_events};

/// The doc 03 §4.6 short-media rule, asserted on the observed trail:
/// the session reached `Playing` (immediate start — never a terminal
/// `Buffering`), and every `Buffering` episode it did enter was left
/// **to `Playing`** ("start with what exists"), never to `Ended`
/// directly from under the watermark race, and never left at all.
fn assert_short_media_trail(host: &ScenarioHost, pumps: usize) {
    let trail = host.state_history();
    assert_eq!(
        trail.last().copied(),
        Some((PlaybackState::Playing, PlaybackState::Ended)),
        "a short clip ends via Playing → Ended ({pumps} pumps): {trail:?}"
    );
    for pair in trail.windows(2) {
        if pair[0].1 == PlaybackState::Buffering {
            assert_eq!(
                pair[1],
                (PlaybackState::Buffering, PlaybackState::Playing),
                "an EOF-before-watermark episode starts with what exists: {trail:?}"
            );
        }
    }
}

/// Open `path`, drive to the terminal, and assert the full clean
/// short-media shape. Answers the declared duration (µs) for the
/// caller's pinned range check.
fn play_short_file(
    host: &mut ScenarioHost,
    path: std::path::PathBuf,
    frames: u64,
    name: &str,
) -> u64 {
    let open = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "{name}: the short clip opens: {:?}",
        open.result
    );
    host.settle();
    let settled = host.state();
    assert!(
        matches!(settled, PlaybackState::Playing | PlaybackState::Ended),
        "{name}: a short clip settles into an active or already-drained \
         state, never a stuck Buffering (got {settled:?})"
    );

    // The honest duration while the session is live (µs; both tracks
    // declare frames/30 s — the patched strh spans).
    let duration = match dispatch(host, Command::MediaGetDuration).result {
        Ok(CommandOutcome::Completed(commands::CommandResult::Duration {
            duration: commands::catalog::types::Duration::Known { time },
        })) => u64::try_from(time.0).must(),
        other => unreachable!("{name}: the short clip's duration is known: {other:?}"),
    };

    let (pumps, terminal) = host.run_until_terminal(256);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "{name}: the short clip reaches Ended ({pumps} pumps)"
    );
    assert_eq!(
        count_events(host, "playback.end-of-media"),
        1,
        "{name}: exactly one end-of-media"
    );
    let errors = error_events(host);
    assert!(
        errors.is_empty(),
        "{name}: a well-formed short clip reports no errors: {errors:?}"
    );
    let delivered = host.sink().map(|sink| sink.frames_accepted()).must();
    assert_eq!(
        delivered, frames,
        "{name}: every declared frame delivered ({delivered} of {frames}, {pumps} pumps)"
    );
    assert_short_media_trail(host, pumps);

    let close = dispatch(host, Command::SessionClose);
    assert!(close.result.is_ok(), "{name}: closes: {:?}", close.result);
    duration
}

/// The 1-frame clip and the 0.1 s clip: both play clean to `Ended`
/// with the declared frame count, honest duration, and the §4.6
/// no-stuck-buffering trail.
#[test]
fn one_frame_and_tenth_second_clips_end_cleanly() {
    let _serial = serialize_tests();
    let scratch = TempDir::new("s14");

    // The 1-frame fixture: 1 video + 1 audio chunk = 2 frames, 33.3 ms.
    let one_frame = scratch.build_short_avi("short-1frame.avi", 1);
    let mut host = ScenarioHost::new();
    let duration = play_short_file(&mut host, one_frame, 2, "1-frame");
    assert!(
        (33_000..=33_400).contains(&duration),
        "the 1-frame clip's declared duration is one frame ({duration} µs)"
    );

    // The 0.1 s clip: 3 frame pairs = 6 frames, 3/30 s.
    let tenth = scratch.build_short_avi("short-0p1s.avi", 3);
    let mut host = ScenarioHost::new();
    let duration = play_short_file(&mut host, tenth, 6, "0.1s-clip");
    assert!(
        (99_900..=100_100).contains(&duration),
        "the 0.1 s clip's declared duration is 0.1 s ({duration} µs)"
    );
}

/// The scripted 1-packet timeline: the same §4.6 race at the demux
/// stage seam — one packet, then EOF; `Playing` then a quick drain to
/// `Ended`, one frame delivered, one end-of-media, no errors.
#[test]
fn one_packet_timeline_ends_cleanly() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    let open = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: "timeline://1x33333".to_string(),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "the 1-packet timeline opens: {:?}",
        open.result
    );
    host.settle();
    let (pumps, terminal) = host.run_until_terminal(128);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the 1-packet timeline reaches Ended ({pumps} pumps)"
    );
    assert_eq!(
        host.sink().map(|sink| sink.audio_accepted()).must(),
        1,
        "exactly one frame delivered"
    );
    assert_eq!(count_events(&host, "playback.end-of-media"), 1);
    assert!(
        error_events(&host).is_empty(),
        "a clean 1-packet timeline reports no errors"
    );
    assert_short_media_trail(&host, pumps);
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}

/// The zero-packet timeline: EOF before *any* packet — the degenerate
/// short edge. Pinned observed behavior (doc 03 §4.6's "Demuxing | EOF
/// before watermark → Playing → quick drain to `Ended`" holds): the
/// session opens, never plays a frame, and still ends cleanly with
/// exactly one end-of-media and zero errors — an honest empty media,
/// never a hang and never a stuck `Buffering`.
#[test]
fn zero_packet_timeline_ends_cleanly_with_no_frames() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    let open = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: "timeline://0x33333".to_string(),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "the zero-packet timeline opens: {:?}",
        open.result
    );
    let (pumps, terminal) = host.run_until_terminal(128);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "empty media still reaches a legal terminal ({pumps} pumps)"
    );
    assert_eq!(
        host.sink().map(|sink| sink.frames_accepted()).must(),
        0,
        "nothing was ever decoded"
    );
    assert_eq!(
        count_events(&host, "playback.end-of-media"),
        1,
        "exactly one end-of-media"
    );
    assert!(
        error_events(&host).is_empty(),
        "an empty timeline is not an error: {:?}",
        error_events(&host)
    );
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
