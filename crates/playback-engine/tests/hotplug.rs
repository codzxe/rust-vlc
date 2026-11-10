//! Card 7.C1 — the device-hotplug acceptance suite (issue #42):
//! simulated device events against the REAL engine.
//!
//! # What these tests prove (the card's AC, verbatim)
//!
//! *"hotplug tests with simulated device events"* — a device that
//! *vanishes mid-playback* → the engine **renegotiates** (suspend
//! in-pipeline, bounded reopen window) → **resumes** on the
//! re-acquired/new device — **no panic, no restart**:
//!
//! - the suspend is the session's own `Pause` row applied between
//!   polls (no teardown, no epoch bump, no generation bump — the
//!   no-restart proof asserts all three ids stay identical);
//! - the resume is the `Play` row: the clock continues from its own
//!   frozen value, so playback continues at the **remembered
//!   position**, never from zero;
//! - the wire shows cause-then-effect (doc 05 §3.3): `audio.
//!   device-lost` before `state-changed → paused`, `audio.
//!   device-changed` before `state-changed → playing`;
//! - the reopened device may be a **different** one (the new-device
//!   migration case);
//! - window exhaustion is a **non-fatal** suspension notice — the
//!   session stays alive at the remembered position, closeable like
//!   any paused session (a fatal would kill a session whose media is
//!   fine — the recovery doctrine's own words);
//! - repeated loss is tolerated (each episode gets a fresh window);
//! - the renderer's capability report relays onto the engine's one
//!   wire event stream (`video.backend-changed`), including the
//!   startup report replayed at the first open.
//!
//! # The simulated device (the mission's sanctioned test double)
//!
//! No production audio seam in this build can lose a device (the
//! null device cannot vanish — the scenario-12 honesty note). The
//! simulated event is the **typed fact a host's seam would report**:
//! the test calls `Engine::observe_audio_device_lost` exactly where
//! an OS callback would. The engine under test is the real one; only
//! the *origin* of the observation is simulated — the same doctrine
//! as the platform crates' event-queue tests.
//!
//! # Time
//!
//! Virtual throughout (doc 03 §8): one shared atomic clock drives
//! the media clock and the hotplug windows; no sleeps anywhere.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64 as SharedNs, Ordering};

use common::{Collector, SharedNow, base};
use media_clock::{MonotonicSource, PlaybackClock};
use media_types::MediaKind;
use media_types::frame::AudioFormat;
use media_types::{EngineError, PipelineStage, Rate, Timestamp};
use playback_engine::{
    DrainStaleness, Engine, EngineEvent, EngineEventKind, EngineOutput, GenerationPair,
    HotplugStep, PipelineGeneration, SessionEpoch, SessionHandle, SessionState,
};
use plugin_api::DemuxEvent;
use testkit::Must;

use commands::envelope::{Generation, SessionId};
use commands::{EventKind, PlaybackState};

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

/// One hotplug test session: the real engine over a scripted audio
/// media, a shared virtual clock, and the drained wire trail.
struct HotplugFixture {
    engine: Engine,
    handle: SessionHandle,
    now: Arc<SharedNow>,
    clock: Arc<media_clock::MediaClock>,
    sink: Arc<Collector>,
    /// The drained wire events, in order.
    wire: Vec<EventKind>,
}

/// Advance virtual time by whole milliseconds and poll once per step
/// (the host's natural cadence: poll, observe, repeat).
fn run_ms(fixture: &mut HotplugFixture, ms: u64, step_ms: u64) {
    let mut left = ms;
    while left > 0 {
        let step = step_ms.min(left);
        fixture.now.0.fetch_add(step * 1_000_000, Ordering::Relaxed);
        fixture.engine.poll();
        drain_wire(fixture);
        left -= step;
    }
}

/// Drain the engine's outputs into the wire trail (events convert;
/// receipts are counted away — none of these scenarios submit
/// commands after the open).
fn drain_wire(fixture: &mut HotplugFixture) {
    for output in fixture.engine.take_outputs() {
        if let EngineOutput::Event(event) = output
            && let Some(wire) = event.as_wire_event() {
                fixture.wire.push(wire.kind);
            }
    }
}

/// The engine-side events of the last drain, unconverted (the
/// ordering assertions need the raw kinds) — the drained events'
/// wire forms land in the trail too, so the episode stays
/// inspectable end-to-end even when this helper consumed the
/// outputs.
fn engine_events(fixture: &mut HotplugFixture) -> Vec<EngineEventKind> {
    let mut kinds = Vec::new();
    for output in fixture.engine.take_outputs() {
        if let EngineOutput::Event(event) = output {
            if let Some(wire) = event.as_wire_event() {
                fixture.wire.push(wire.kind);
            }
            kinds.push(event.kind);
        }
    }
    kinds
}

/// Open a playing audio session over a scripted media, with the
/// shared virtual clock the hotplug windows read.
fn open_playing(script_len: u64) -> HotplugFixture {
    let now = Arc::new(SharedNow(AtomicU64Placement::new(0)));
    let sink = Arc::new(Collector::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::clone(&now) as Arc<dyn MonotonicSource>,
            Timestamp::new(0, base()),
            Rate::ONE,
        )
        .must(),
    );
    let mut script: Vec<DemuxEvent> = (0..script_len)
        .map(|index| DemuxEvent::Packet(common::packet(media_types::TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let config = common::config_with(
        vec![common::track(1, MediaKind::Audio)],
        script,
        None,
        None,
        Arc::clone(&clock),
        Arc::clone(&sink),
    );
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    let mut fixture = HotplugFixture {
        engine,
        handle,
        now,
        clock,
        sink,
        wire: Vec::new(),
    };
    // Prime to Playing, then play 1.5 s of media so the loss lands
    // mid-playback with a nonzero position (the queue holds 12+
    // packets; the script has plenty).
    for _ in 0..64 {
        fixture.engine.poll();
        if fixture.engine.state() == SessionState::Playing {
            break;
        }
    }
    assert_eq!(
        fixture.engine.state(),
        SessionState::Playing,
        "the session must be playing before the device goes away"
    );
    drain_wire(&mut fixture);
    run_ms(&mut fixture, 1_500, 50);
    assert!(
        common::frames_of(&fixture.sink) > 0,
        "frames delivered before the loss (the pump ran)"
    );
    fixture
}

/// The media clock's position, nanoseconds (the same read the
/// engine's loss hook performs).
fn position_ns(fixture: &HotplugFixture) -> i64 {
    fixture
        .clock
        .now()
        .as_nanos()
        .map(|ns| i64::try_from(ns.0).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// The wire trail since the last mark, as display names.
fn wire_since(fixture: &HotplugFixture, mark: usize) -> Vec<&str> {
    fixture.wire[mark..]
        .iter()
        .map(|kind| kind_name(kind))
        .collect()
}

/// The stable display name of a wire event kind (assertion
/// vocabulary — the enum has no `as_str`).
fn kind_name(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::AudioDeviceLost { .. } => "audio.device-lost",
        EventKind::AudioDeviceChanged { .. } => "audio.device-changed",
        EventKind::VideoBackendChanged { .. } => "video.backend-changed",
        EventKind::SessionStateChanged { .. } => "session.state-changed",
        EventKind::PlaybackError { .. } => "playback.error",
        _ => "other",
    }
}

/// A rename so the import reads naturally in the fixture (the atomic
/// IS the shared now source; the alias documents the unit).
type AtomicU64Placement = SharedNs;

/// The typed device-loss error a reopen attempt fails with (the
/// L0 vocabulary's own audio-output row).
fn device_error() -> EngineError {
    EngineError::new(
        PipelineStage::AudioOutput,
        media_types::ErrorKind::Device {
            which: media_types::error::DeviceSlot::AudioOutput,
            cause: media_types::error::DeviceCause::Disconnected,
        },
        media_types::error::ErrorContext::new("audio.reopen.no-device"),
    )
}

// ---------------------------------------------------------------------------
// AC 1 — the flagship: vanish mid-play, renegotiate, resume
// ---------------------------------------------------------------------------

/// The device vanishes mid-playback; the engine suspends in-pipeline
/// (no restart: same session id, same epoch, same pipeline
/// generation), the reopen window offers the host an attempt, and a
/// **new** device resumes playback at the remembered position —
/// cause-then-effect on the wire, frames flowing again, no panic.
#[test]
fn device_vanishes_mid_playback_engine_renegotiates_and_resumes_on_new_device() {
    let mut fixture = open_playing(512);
    let session_before = fixture.engine.session_id().must();
    let generation_before = fixture.engine.generation().must();
    let position_at_loss = position_ns(&fixture);
    assert!(
        position_at_loss > 0,
        "the loss must land mid-playback (position {position_at_loss} ns)"
    );

    // --- The device goes away (the OS seam's typed fact). ---
    let loss_ns = fixture.now.0.load(Ordering::Relaxed);
    let events = {
        fixture.engine.observe_audio_device_lost(loss_ns, "sink-7");
        engine_events(&mut fixture)
    };
    // Cause precedes effect (doc 05 §3.3).
    assert_eq!(
        events.first(),
        Some(&EngineEventKind::AudioDeviceLost {
            device_id: "sink-7".to_string()
        }),
        "the loss fact leads the suspend reaction: {events:?}"
    );
    assert!(matches!(
        events.get(1),
        Some(EngineEventKind::TimeTick { .. })
    ));
    assert!(matches!(
        events.get(2),
        Some(EngineEventKind::StateChanged { .. })
    ));
    assert_eq!(
        fixture.engine.state(),
        SessionState::Paused,
        "the suspend is the Pause row: {}",
        fixture.engine.state()
    );
    // The no-restart proof: nothing was torn down or re-minted.
    assert_eq!(fixture.engine.session_id(), Some(session_before));
    assert_eq!(fixture.engine.generation(), Some(generation_before));
    assert!(fixture.engine.cancel_token().is_some());
    let stats = fixture.engine.hotplug_stats();
    assert_eq!(stats.losses, 1);
    assert!(stats.suspended);
    assert_eq!(stats.suspended_position_ns, Some(position_at_loss));

    // --- The window offers a reopen; the host tries a NEW device. ---
    let poll_ns = loss_ns + 1_000_000;
    let step = fixture.engine.poll_hotplug(poll_ns);
    assert_eq!(
        step,
        HotplugStep::ReopenDevice,
        "the first reopen attempt is due immediately"
    );
    let events = {
        fixture
            .engine
            .observe_audio_reopen(poll_ns, Ok("sink-9".to_string()));
        engine_events(&mut fixture)
    };
    assert_eq!(
        events.first(),
        Some(&EngineEventKind::AudioDeviceChanged {
            device_id: "sink-9".to_string()
        }),
        "the migration fact leads the resume reaction: {events:?}"
    );
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    // Still no restart.
    assert_eq!(fixture.engine.session_id(), Some(session_before));
    assert_eq!(fixture.engine.generation(), Some(generation_before));

    // --- The resume continues from the remembered position. ---
    let frames_at_resume = common::frames_of(&fixture.sink);
    run_ms(&mut fixture, 500, 50);
    assert!(
        common::frames_of(&fixture.sink) > frames_at_resume,
        "playback continues after the migration (no zombie session)"
    );
    let position_after = position_ns(&fixture);
    assert!(
        position_after >= position_at_loss,
        "the clock continued from its frozen value ({position_after} >= {position_at_loss})"
    );
    let stats = fixture.engine.hotplug_stats();
    assert_eq!(stats.reopens, 1);
    assert!(!stats.suspended);

    // --- The wire trail carries the whole episode, ordered. ---
    let names = wire_since(&fixture, 0);
    let lost_at = names
        .iter()
        .position(|name| *name == "audio.device-lost")
        .must();
    let paused_at = fixture
        .wire
        .iter()
        .position(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: PlaybackState::Paused,
                    ..
                }
            )
        })
        .must();
    assert!(
        lost_at < paused_at,
        "device-lost precedes state-changed → paused: {names:?}"
    );
    let changed_at = names
        .iter()
        .position(|name| *name == "audio.device-changed")
        .must();
    assert!(
        changed_at > paused_at,
        "device-changed follows the suspend: {names:?}"
    );

    // --- The session closes cleanly afterwards. ---
    fixture
        .handle
        .close(commands::envelope::RequestId(1))
        .must();
    fixture.engine.poll();
}

/// The exhausted window: every reopen attempt fails, the budget ends,
/// and the terminal is the **non-fatal** suspension notice — the
/// session stays paused at the remembered position (never `error`),
/// closeable like any paused session, and the notice names the device
/// and the attempt count.
#[test]
fn exhausted_reopen_window_suspends_non_fatal_at_remembered_position() {
    let mut fixture = open_playing(512);
    let session_before = fixture.engine.session_id().must();
    let generation_before = fixture.engine.generation().must();
    let position_at_loss = position_ns(&fixture);
    let loss_ns = fixture.now.0.load(Ordering::Relaxed);
    fixture.engine.observe_audio_device_lost(loss_ns, "sink-7");
    assert_eq!(fixture.engine.state(), SessionState::Paused);

    // Drain the whole reopen budget with typed failures, advancing
    // virtual time past each backoff (the default window: 6 attempts
    // inside 10 s, backoff 250 ms doubling to 4 s).
    let mut now_ns = loss_ns;
    let mut attempts = 0u32;
    let mut notices = 0u32;
    for _ in 0..64 {
        match fixture.engine.poll_hotplug(now_ns) {
            HotplugStep::ReopenDevice => {
                attempts += 1;
                fixture
                    .engine
                    .observe_audio_reopen(now_ns, Err(device_error()));
                drain_wire(&mut fixture);
            }
            HotplugStep::None => {}
        }
        now_ns += 300_000_000; // 300 ms per host tick
        fixture.now.0.store(now_ns, Ordering::Relaxed);
        fixture.engine.poll();
        drain_wire(&mut fixture);
        // Count notices as they land.
        notices = fixture
            .wire
            .iter()
            .filter(|kind| matches!(kind, EventKind::PlaybackError { fatal: false, .. }))
            .count() as u32;
        let stats = fixture.engine.hotplug_stats();
        if stats.notified {
            break;
        }
    }
    let stats = fixture.engine.hotplug_stats();
    assert!(
        stats.notified,
        "the window must exhaust (attempts {attempts})"
    );
    assert_eq!(notices, 1, "the notice is one-time: {notices} in the trail");
    assert!(
        attempts >= 1,
        "at least one reopen attempt was made ({attempts})"
    );
    // The notice's payload: non-fatal, audio-output stage, device
    // code, the lost device, the attempt count, and the failure.
    let notice = fixture
        .wire
        .iter()
        .find_map(|kind| match kind {
            EventKind::PlaybackError {
                stage,
                code,
                fatal: false,
                details,
                ..
            } => Some((*stage, *code, details.clone())),
            _ => None,
        })
        .must();
    assert_eq!(notice.0, commands::Stage::AudioOutput);
    assert_eq!(notice.1, commands::ErrorCode::Device);
    assert_eq!(
        notice.2.get("device").map(String::as_str),
        Some("sink-7"),
        "the notice names the lost device: {:?}",
        notice.2
    );
    assert_eq!(
        notice.2.get("reopen_attempts").map(String::as_str),
        Some(attempts.to_string().as_str()),
        "the notice carries the attempt count"
    );
    assert_eq!(
        notice.2.get("context").map(String::as_str),
        Some("audio.device.suspend")
    );

    // The session is ALIVE, paused, at the remembered position —
    // never error, never restarted.
    assert_eq!(fixture.engine.state(), SessionState::Paused);
    assert_eq!(fixture.engine.session_id(), Some(session_before));
    assert_eq!(fixture.engine.generation(), Some(generation_before));
    assert_eq!(
        position_ns(&fixture),
        position_at_loss,
        "the clock stays frozen at the remembered position"
    );

    // A late reopen still resumes (the honest answer does not decay):
    // the guard is notified but a success is a success.
    let step = fixture.engine.poll_hotplug(now_ns);
    assert_eq!(
        step,
        HotplugStep::None,
        "no further attempts after the latch"
    );
    fixture
        .engine
        .observe_audio_reopen(now_ns, Ok("sink-late".to_string()));
    assert_eq!(
        fixture.engine.state(),
        SessionState::Playing,
        "a late re-acquisition resumes the suspended session"
    );

    // Clean close.
    fixture
        .handle
        .close(commands::envelope::RequestId(2))
        .must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
}

/// Repeated loss is tolerated: each episode gets a fresh window and a
/// fresh budget; the position is preserved across all of them.
#[test]
fn repeated_device_loss_tolerated_with_fresh_windows() {
    let mut fixture = open_playing(1024);
    for episode in 0..3u32 {
        let position_at_loss = position_ns(&fixture);
        let loss_ns = fixture.now.0.load(Ordering::Relaxed);
        fixture.engine.observe_audio_device_lost(loss_ns, "sink-7");
        assert_eq!(
            fixture.engine.state(),
            SessionState::Paused,
            "episode {episode}: suspended"
        );
        let step = fixture.engine.poll_hotplug(loss_ns);
        assert_eq!(step, HotplugStep::ReopenDevice);
        fixture
            .engine
            .observe_audio_reopen(loss_ns, Ok("sink-9".to_string()));
        assert_eq!(
            fixture.engine.state(),
            SessionState::Playing,
            "episode {episode}: resumed"
        );
        let stats = fixture.engine.hotplug_stats();
        assert_eq!(stats.losses, u64::from(episode) + 1);
        assert_eq!(stats.reopens, u64::from(episode) + 1);
        assert!(
            position_ns(&fixture) >= position_at_loss,
            "episode {episode}: position preserved"
        );
        run_ms(&mut fixture, 200, 50);
    }
    // Three episodes, no restart ever.
    let stats = fixture.engine.hotplug_stats();
    assert_eq!(stats.losses, 3);
    assert_eq!(stats.reopens, 3);
}

// ---------------------------------------------------------------------------
// AC 2 — the renderer backend report relay
// ---------------------------------------------------------------------------

/// The renderer's capability report lands on the engine's one wire
/// event stream as `video.backend-changed` with the typed payload —
/// including the startup report, replayed onto the first session's
/// stream at open (the catalog row's "initial auto-negotiation
/// report").
#[test]
fn renderer_backend_report_relays_as_video_backend_changed() {
    // The startup negotiation happens BEFORE any session exists: the
    // report is cached (this build: no hardware backend compiled in —
    // the honest software-present verdict, see renderer-wgpu's
    // fallback chain).
    let mut fixture = {
        // A fresh engine, not the playing fixture: the replay-at-open
        // needs the observation to precede the open.
        let now = Arc::new(SharedNow(AtomicU64Placement::new(0)));
        let sink = Arc::new(Collector::default());
        let clock = Arc::new(
            media_clock::MediaClock::system_monotonic(
                Arc::clone(&now) as Arc<dyn MonotonicSource>,
                Timestamp::new(0, base()),
                Rate::ONE,
            )
            .must(),
        );
        let script: Vec<DemuxEvent> = (0..64)
            .map(|index| DemuxEvent::Packet(common::packet(media_types::TrackId::new(1), index)))
            .collect();
        let config = common::config_with(
            vec![common::track(1, MediaKind::Audio)],
            script,
            None,
            None,
            clock.clone(),
            sink.clone(),
        );
        let mut engine = Engine::new();
        engine.observe_renderer_backend(
            "software",
            "no hardware backend in this build (wgpu import Lead-gated); software present active",
        );
        let stats = engine.hotplug_stats();
        assert_eq!(
            stats.renderer_backend,
            Some((
                "software".to_string(),
                "no hardware backend in this build (wgpu import Lead-gated); software present active"
                    .to_string()
            )),
            "the startup report is cached before any session"
        );
        let handle = engine.open(config).must();
        HotplugFixture {
            engine,
            handle,
            now,
            clock,
            sink,
            wire: Vec::new(),
        }
    };
    drain_wire(&mut fixture);
    // The initial report replayed at open.
    assert!(
        fixture
            .wire
            .iter()
            .any(|kind| matches!(kind, EventKind::VideoBackendChanged { backend, .. } if backend == "software")),
        "the initial backend report is on the first session's stream: {:?}",
        fixture.wire
    );

    // A mid-session fallback (the simulated backend failure: hardware
    // compositor lost → software present) relays live.
    fixture.engine.observe_renderer_backend(
        "software",
        "hardware compositor failed (simulated adapter loss); fell back to software present",
    );
    drain_wire(&mut fixture);
    let report = fixture
        .wire
        .iter()
        .filter_map(|kind| match kind {
            EventKind::VideoBackendChanged { backend, reason } => {
                Some((backend.clone(), reason.clone()))
            }
            _ => None,
        })
        .next_back()
        .must();
    assert_eq!(report.0, "software");
    assert!(
        report.1.contains("fell back"),
        "the fallback cause is display-safe and specific: {report:?}"
    );
    // The engine's own state agrees.
    let stats = fixture.engine.hotplug_stats();
    assert_eq!(
        stats.renderer_backend,
        Some(report),
        "the cached report is the last observation"
    );
}

// ---------------------------------------------------------------------------
// AC 3 — the session-scoped drain rule (doc 05 §3.2 "Not
// pipeline-scoped"): a superseding mint must not swallow the
// device/backend facts
// ---------------------------------------------------------------------------

fn pair(epoch: u64, pipeline: u64) -> GenerationPair {
    GenerationPair {
        epoch: SessionEpoch(epoch),
        pipeline: PipelineGeneration(pipeline),
    }
}

fn stamped_event(epoch: u64, counter: u64, kind: EngineEventKind) -> EngineEvent {
    EngineEvent {
        seq: 0,
        session: SessionId(epoch),
        generation: Generation(counter),
        origin: None,
        kind,
    }
}

/// The audio device facts and the backend report survive an
/// intra-epoch supersession at drain (they speak for the session's
/// environment, not one pipeline generation), while a pipeline-scoped
/// sibling of the same stamp is suppressed — and cross-epoch nothing
/// survives (D-4).
#[test]
fn session_scoped_hotplug_events_survive_intra_epoch_supersession() {
    let kinds = [
        EngineEventKind::AudioDeviceLost {
            device_id: "sink-7".to_string(),
        },
        EngineEventKind::AudioDeviceChanged {
            device_id: "sink-9".to_string(),
        },
        EngineEventKind::RendererBackendReport {
            backend: "software".to_string(),
            reason: "test".to_string(),
        },
    ];
    for kind in kinds {
        assert!(
            !kind.is_pipeline_scoped(),
            "{kind:?} is session-scoped per doc 05 §3.2"
        );
        // Superseded within the epoch: survives.
        assert!(
            !stamped_event(1, 3, kind.clone()).is_stale_at_drain(&pair(1, 4)),
            "an intra-epoch superseded {kind:?} must survive the drain"
        );
        // Cross-epoch: stale by construction (D-4 — nothing crosses
        // a session boundary).
        assert!(
            stamped_event(1, 3, kind.clone()).is_stale_at_drain(&pair(2, 4)),
            "a cross-epoch {kind:?} must not cross the session boundary"
        );
    }
    // The control: a pipeline-scoped kind of the same stamp IS
    // suppressed intra-epoch (the rule still has teeth).
    assert!(
        stamped_event(1, 3, EngineEventKind::Buffering { progress: 0.0 })
            .is_stale_at_drain(&pair(1, 4)),
        "a superseded pipeline-scoped event is still suppressed"
    );
}

// ---------------------------------------------------------------------------
// Honesty guards
// ---------------------------------------------------------------------------

/// A loss with no live session is recorded (the observability truth)
/// but emits nothing — there is no stream to carry it; the engine
/// neither panics nor invents a session.
#[test]
fn loss_without_a_live_session_is_recorded_not_emitted() {
    let mut engine = Engine::new();
    let step = engine.observe_audio_device_lost(0, "sink-ghost");
    assert_eq!(step, HotplugStep::None);
    let stats = engine.hotplug_stats();
    assert_eq!(stats.losses, 1);
    assert_eq!(engine.take_outputs().len(), 0, "nothing to carry it");
    // The window still runs: a reopen can succeed and simply
    // re-arms the active device (no session to resume).
    let step = engine.poll_hotplug(1);
    assert_eq!(step, HotplugStep::ReopenDevice);
    engine.observe_audio_reopen(1, Ok("sink-9".to_string()));
    assert_eq!(engine.hotplug_stats().reopens, 1);
    assert_eq!(engine.take_outputs().len(), 0);
}

/// The wire payload shapes match the catalog contract (doc 05 §3.2
/// payload column): `deviceId` for the audio rows, `backend` +
/// `reason` for the backend report.
#[test]
fn hotplug_wire_payloads_match_the_catalog_contract() {
    let lost = EngineEventKind::AudioDeviceLost {
        device_id: "sink-7".to_string(),
    }
    .as_wire()
    .must();
    match lost {
        EventKind::AudioDeviceLost { device_id } => assert_eq!(device_id, "sink-7"),
        other => unreachable!("the wire form is the device-lost row: {other:?}"),
    }
    let changed = EngineEventKind::AudioDeviceChanged {
        device_id: "sink-9".to_string(),
    }
    .as_wire()
    .must();
    match changed {
        EventKind::AudioDeviceChanged { device_id } => assert_eq!(device_id, "sink-9"),
        other => unreachable!("the wire form is the device-changed row: {other:?}"),
    }
    let backend = EngineEventKind::RendererBackendReport {
        backend: "software".to_string(),
        reason: "negotiated".to_string(),
    }
    .as_wire()
    .must();
    match backend {
        EventKind::VideoBackendChanged { backend, reason } => {
            assert_eq!(backend, "software");
            assert_eq!(reason, "negotiated");
        }
        other => unreachable!("the wire form is the backend-changed row: {other:?}"),
    }
    let suspended = EngineEventKind::AudioDeviceSuspended {
        device_id: "sink-7".to_string(),
        reopen_attempts: 4,
        last_error: None,
    }
    .as_wire()
    .must();
    match suspended {
        EventKind::PlaybackError {
            stage,
            code,
            fatal,
            details,
            ..
        } => {
            assert_eq!(stage, commands::Stage::AudioOutput);
            assert_eq!(code, commands::ErrorCode::Device);
            assert!(!fatal, "the suspension notice is non-fatal");
            assert_eq!(details.get("device").map(String::as_str), Some("sink-7"));
            assert_eq!(
                details.get("reopen_attempts").map(String::as_str),
                Some("4")
            );
        }
        other => unreachable!("the wire form is the non-fatal error row: {other:?}"),
    }
}

/// The engine-side suspension notice and the recovery guard's own
/// direct-drive notice are the same payload — the one-bridge rule
/// (both paths derive from the guard's state; drift here would mean
/// two truths on one wire).
#[test]
fn engine_hook_and_guard_direct_drive_notices_agree() {
    use playback_engine::{AudioDeviceGuard, RetryWindow};

    let mut guard = AudioDeviceGuard::new(RetryWindow::DEFAULT);
    let now = 0u64;
    let _ = guard.observe_device_lost(now, "sink-7", 42_000_000_000);
    // Spend the budget with failures (backoff-doubling inside the
    // 10 s window: the default schedule exhausts by attempts).
    let mut t = now;
    let mut guard_notice: Option<EventKind> = None;
    for _ in 0..40 {
        let step = guard.poll(t);
        let events = match step.action {
            playback_engine::RecoveryAction::ReopenDevice => {
                // The by-budget terminal lands in the RESULT step's
                // events (the guard's own single emission); the
                // by-time terminal in poll's.
                guard.observe_reopen_result(t, Err(device_error())).events
            }
            _ => step.events,
        };
        if let Some(EventKind::PlaybackError { .. }) = events.first() {
            guard_notice = events.into_iter().next();
            break;
        }
        t += 300_000_000;
    }
    let guard_notice = guard_notice.must();
    let EventKind::PlaybackError {
        message: guard_message,
        details: guard_details,
        ..
    } = &guard_notice
    else {
        unreachable!("the guard's notice is the non-fatal error row");
    };

    // The engine-side translation of the same terminal state.
    let engine_notice = EngineEventKind::AudioDeviceSuspended {
        device_id: guard.suspended_device().unwrap_or_default().to_string(),
        reopen_attempts: guard.reopen_attempts_used(),
        last_error: guard.last_reopen_failure(),
    }
    .as_wire()
    .must();
    let EventKind::PlaybackError {
        message: engine_message,
        details: engine_details,
        ..
    } = &engine_notice
    else {
        unreachable!("the engine-side notice is the non-fatal error row");
    };
    assert_eq!(guard_message, engine_message, "one sentence, one truth");
    assert_eq!(guard_details, engine_details, "one hint table, one truth");
}

// Silence the unused warning for the audio-format import the harness
// documents but does not need directly (the fixture's frames carry
// it).
const _: Option<AudioFormat> = None;
