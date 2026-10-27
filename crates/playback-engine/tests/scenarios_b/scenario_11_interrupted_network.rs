//! Scenario 11 — **interrupted network source** (doc 10 §6.4 row
//! 11), driven through the command layer.
//!
//! Pass criteria: `Buffering` → resume or typed error; no hang
//! (deadlines fire).
//!
//! The honest v0 mapping (recorded for the Lead):
//!
//! - Real URL sources (`http(s)://…`) answer the production typed
//!   refusal `input.unsupported-for-host` — the streaming crate is
//!   Phase 6 (doc 05 §5.2's "capability gaps are typed, not
//!   silent"). Asserted here so the wire truth is pinned.
//! - The network DATA path is simulated with the scripted access
//!   source over the parent fixture's bytes (the card's
//!   "testkit synthetic source with scripted truncation"; the
//!   local-fake shape is documented in `host.rs`): two scripts —
//!   the **truncate** script (the mid-stream cut: the source's
//!   visible length drops under the reader, exactly a stalled
//!   download that will never deliver more) and the **reset**
//!   script (the next read fails with the typed mid-read I/O
//!   error, the connection reset).
//!
//! Observed outcomes (pinned by the tuning run; the containment
//! semantics are ED-FIX-3, Lead-verified 2026-10-22):
//!
//! - **truncate** — the open lands, the session plays the delivered
//!   prefix, then ends CLEANLY: `Playing → Ended`, exactly one
//!   `playback.end-of-media`, exactly one NON-FATAL
//!   `playback.error` (the coalesced damage accounting). No hang:
//!   the terminal arrives within the pump bound. This is the
//!   "resume"-shaped arm in v0: there is no rebuffer-and-wait
//!   because a local-source cut is final — the honest equivalent of
//!   the network row's "resume or typed error" is "bounded play-out
//!   to a clean end".
//! - **reset** — the open lands, playback starts, the failed read
//!   surfaces as a typed FATAL: `Playing → Error` with exactly one
//!   fatal `playback.error` (stage `input`, code `inaccessible` —
//!   doc 01 §3.4: `Inaccessible` covers sources that cannot be
//!   opened *or read*). Bounded pumps, never a hang.
//!
//! Handoff (3.C3 recovery — environment owns the retry window →
//! typed `network` error for real streaming sources; 6.D2 owns the
//! reconnect): when the streaming crate lands, this scenario's
//! scripts move to its access layer and the retry/deadline machinery
//! gets its own assertions.

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, ErrorCode, PlaybackState, Stage};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{ScenarioHost, count_events, dispatch, error_events};

/// The scripted parent's URL (the synthetic network source).
const SCRIPTED: &str = "script://avi-parent";

/// The documented mid-movi cut (the parent structure map's frame-30
/// `00dc` chunk — the same offset as the committed `trunc3` class-B
/// derivative).
const CUT: u64 = 212_512;

/// Open the scripted source into the **mid-stream parked hold** the
/// mid-play faults need: tight queue caps (4 packets per limb) plus
/// an immediate pause. This v0 engine has no wall-clock pacing — the
/// synthetic sink's sample counter self-feeds, so a `Playing` settle
/// drains the whole stream inside its own pumps and an armed fault
/// would never fire. `Paused` parks decode (doc 03 §4.6's pause row:
/// the demuxer fills to the watermark, then stops), which holds the
/// demuxer genuinely mid-file: the cut/reset below bites a live
/// mid-stream download, not an already-drained one.
fn open_scripted_parked(host: &mut ScenarioHost) {
    host.pace_next_open(4);
    let response = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: SCRIPTED.to_string(),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the scripted source opens: {:?}",
        response.result
    );
    // Land the pause BEFORE any settle-style pumping can race the
    // sink forward (the buffered pause, card 3.C1).
    let pause = dispatch(host, Command::PlaybackPause);
    assert!(pause.result.is_ok(), "the pause lands: {:?}", pause.result);
    assert_eq!(host.state(), PlaybackState::Paused, "parked mid-stream");
    // The honest guard: the demuxer is parked before the cut — the
    // mid-play fault arming is real, not a fiction.
    let (reads, bytes) = host.script().must().read_stats();
    assert!(
        bytes < CUT,
        "the parked demuxer sits before the cut (reads {reads}, \
         bytes {bytes} >= {CUT}) — the fault arming would never fire"
    );
    let before_frames = host.sink().map(|sink| sink.frames_accepted()).must();
    assert!(
        before_frames < 120,
        "the park held playback mid-stream ({before_frames} frames)"
    );
}

fn open_scripted(host: &mut ScenarioHost) {
    let response = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: SCRIPTED.to_string(),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the scripted source opens: {:?}",
        response.result
    );
    host.settle();
    assert_eq!(
        host.state(),
        PlaybackState::Playing,
        "the scripted source plays"
    );
}

/// Real URL sources answer the production typed refusal (the
/// streaming crate is Phase 6) — with the scheme detail attached.
#[test]
fn url_sources_refuse_the_phase_named() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: "https://example.org/stream.mp4".to_string(),
            },
            options: OpenOptions::default(),
        },
    );
    match response.result {
        Err(error) => {
            assert_eq!(error.stage, Stage::Input, "{error}");
            assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}");
            assert_eq!(
                error.details.get("scheme").map(String::as_str),
                Some("https"),
                "{error}"
            );
            assert!(
                error.message.contains("Phase 6"),
                "the refusal names the owning phase: {error}"
            );
        }
        Ok(_) => unreachable!("a URL source must refuse in this build"),
    }
    assert!(host.session_id().is_none());
    assert_eq!(host.state(), PlaybackState::Idle);
}

/// The **truncate** script: parked mid-stream, the source's visible
/// length drops to a documented mid-movi offset (the parent's
/// structure map: the frame-30 `00dc` chunk, offset 212_512 — the
/// same cut as the committed `trunc3` class-B derivative) and the
/// session resumes. The containment contract holds end-to-end and
/// nothing hangs.
#[test]
fn truncated_midstream_source_ends_cleanly_within_deadline() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    open_scripted_parked(&mut host);

    let before_frames = host.sink().map(|sink| sink.frames_accepted()).must();
    // The mid-stream cut: the download stops delivering at `CUT`
    // bytes (documented offset — the structure map's frame-30 chunk).
    host.script().must().shrink_to(CUT);
    // Resume: the parked session plays on into the cut.
    let play = dispatch(&mut host, Command::PlaybackPlay);
    assert!(play.result.is_ok(), "resumes: {:?}", play.result);

    let (pumps, terminal) = host.run_until_terminal(512);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the cut ends cleanly (pumps {pumps})"
    );
    let errors = error_events(&host);
    let damage: Vec<_> = errors.iter().filter(|(_, fatal, _)| !fatal).collect();
    let fatal: Vec<_> = errors.iter().filter(|(_, fatal, _)| *fatal).collect();
    assert_eq!(
        damage.len(),
        1,
        "exactly one non-fatal damage accounting ({pumps} pumps): {errors:?}"
    );
    assert!(
        damage[0].2.contains("recovered") && damage[0].2.contains("damaged"),
        "the damage accounting sentence: {damage:?}"
    );
    assert!(fatal.is_empty(), "the cut is never fatal: {errors:?}");
    assert_eq!(
        count_events(&host, "playback.end-of-media"),
        1,
        "exactly one end-of-media"
    );
    let frames = host.sink().map(|sink| sink.frames_accepted()).must();
    assert!(
        frames > before_frames,
        "the delivered prefix played out ({before_frames} → {frames})"
    );
    assert!(
        frames < 120,
        "the cut actually bit: {frames} of 120 frames delivered"
    );
    // The visible length is the truth the source reports now.
    assert_eq!(host.script().must().visible_len(), CUT);

    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}

/// The **reset** script: parked mid-stream, the next read fails
/// with the typed mid-read I/O failure and the session resumes. A
/// typed fatal lands (`Playing → Error`), bounded — never a hang,
/// never a panic.
#[test]
fn reset_midstream_source_fails_typed_within_deadline() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    open_scripted_parked(&mut host);

    // The connection reset: the very next read fails typed.
    host.script().must().fail_next_read();
    // Resume: the parked session plays into the failed read.
    let play = dispatch(&mut host, Command::PlaybackPlay);
    assert!(play.result.is_ok(), "resumes: {:?}", play.result);

    let (pumps, terminal) = host.run_until_terminal(512);
    assert_eq!(
        terminal,
        PlaybackState::Error,
        "the reset is a typed fatal (pumps {pumps})"
    );
    let errors = error_events(&host);
    let fatal: Vec<_> = errors.iter().filter(|(_, fatal, _)| *fatal).collect();
    assert_eq!(fatal.len(), 1, "exactly one fatal error event: {errors:?}");
    assert_eq!(
        fatal[0].0.as_str(),
        "input.inaccessible",
        "the typed fatal: {fatal:?}"
    );
    // The fatal error event is ordered BEFORE the state change to
    // Error (doc 05 §3.3) — the state trail ends with → Error. The
    // from-state is the buffering phase the reset happened to land
    // in (the 6.FIX-pcm queue-admission change shifted the episode
    // timing: the resumed session sits in a rebuffer when the failed
    // read hits — `Buffering → Error`; a reset landing mid-play
    // answers `Playing → Error`. Both are the D-4 typed fatal; the
    // contract is the terminal, not the phase it interrupted).
    let trail = host.state_history();
    let last = trail.last().copied();
    assert!(
        matches!(
            last,
            Some((
                PlaybackState::Playing | PlaybackState::Buffering,
                PlaybackState::Error
            ))
        ),
        "the trail: {trail:?}"
    );
    // The host recovers: a close settles the dead session and the
    // next open works (no poisoned state after a network fatal).
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(
        close.result.is_ok(),
        "closes after the fatal: {:?}",
        close.result
    );
    assert_eq!(host.state(), PlaybackState::Idle);
    open_scripted(&mut host);
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(
        close.result.is_ok(),
        "the reopened session closes: {:?}",
        close.result
    );
}
