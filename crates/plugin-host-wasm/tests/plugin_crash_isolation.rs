//! §9.5 crash isolation & lifecycle (card 7.F4, issue #46): the doc
//! 06 §7.5 crash-injection suite driven end to end through the REAL
//! host and the REAL supervisor — and the **playback-unaffected
//! proof** (§9.5 `plugin_crash_isolation`), which runs the real
//! playback engine (the scenario-harness pattern from
//! `crates/playback-engine/tests/common`) in the same process as a
//! trap-on-every-call extension storm.
//!
//! The division of labor under test (doc 06 §5.6):
//!
//! - **wasmi contains** — every fixture trap destroys exactly one
//!   instance `Store`; the host process, other instances and the
//!   playback session are untouched; the error that crosses the
//!   boundary is a typed [`HostError`], never a panic (asserted
//!   mechanically with `catch_unwind` in the playback proof).
//! - **the supervisor decides** — each contained fault is recorded
//!   (`record_crash`) and answered with the doc 06 §5.6 sequence:
//!   immediate restart, 5 s, 30 s, fourth crash → `Disabled`, with
//!   `plugins.crashed`-shaped events (doc 05 §3) drained by the
//!   embedder. Policy values come from the settings schema, never
//!   re-stated here.

mod common;

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use media_clock::MonotonicSource;
use media_types::frame::{AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet,
    PipelineStage, SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId,
    TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::{
    DecoderProvider, Engine, EventPolicy, FrameConsumer, QueuePlan, SessionConfig, SessionState,
    TickBudget, TimeTickPolicy,
};
use plugin_api::{DemuxEvent, Demuxer, PluginManifest};
use testkit::Must;

use common::{
    crash_escalate_wasm, crash_fuel_burner_wasm, crash_oom_wasm, crash_survivor_wasm,
    crash_trap_mid_call_wasm, crash_trap_on_entry_wasm, manifest, manifest_toml, ok, spawn,
};
use plugin_host_wasm::HostError;
use plugin_host_wasm::instance::InstanceStatus;
use plugin_host_wasm::state::HostServices;
use plugin_host_wasm::supervisor::{ActionTaken, BackoffPolicy, ExtensionSupervisor, RunState};

/// The supervised extension under test: the `ext.acme.hello` fixture
/// identity `manifest_toml` stamps (version 0.1.0).
const EXT_ID: &str = "ext.acme.hello";
const EXT_VERSION: &str = "0.1.0";

/// One supervised dispatch round — the embedder-side loop of doc 06
/// §5.8: consult the run-state, spawn (a restart), invoke, and feed
/// any contained fault to the supervisor. Returns the decision the
/// embedder acts on (`None` = dispatch happened, fault or not).
///
/// This function IS the boundary: a trap crossing it would unwind the
/// test process; instead every fixture returns through it with a
/// typed error.
fn supervised_dispatch(
    supervisor: &mut ExtensionSupervisor,
    wasm: &[u8],
    manifest: &PluginManifest,
    entry: &str,
    now_ms: u64,
) -> Option<Result<Vec<i64>, HostError>> {
    match supervisor.state(EXT_ID, EXT_VERSION) {
        RunState::Disabled { .. } => None, // policy refuses the respawn
        RunState::Enabled { .. } => {
            let mut instance = ok(spawn(wasm, manifest.clone(), HostServices::empty()));
            let outcome = instance.invoke(entry, &[]);
            if let Err(host_err) = &outcome {
                supervisor.record_crash(EXT_ID, EXT_VERSION, &host_err.to_string(), now_ms);
            }
            Some(outcome)
        }
    }
}

// ---------------------------------------------------------------------------
// §7.5 crash injection — every fixture contained, host + survivor alive
// ---------------------------------------------------------------------------

#[test]
fn crash_injection_trap_on_entry_is_contained() {
    let wasm = crash_trap_on_entry_wasm();
    let ext_manifest = manifest(&manifest_toml(&wasm, ""));
    let mut supervisor = ExtensionSupervisor::new(ok(BackoffPolicy::from_schema_defaults()));

    // The maximally hostile extension: every export is `unreachable`.
    // Dispatch 1: trap contained, typed error, immediate restart
    // granted (doc 06 §5.6 "immediate retry once").
    let first = supervised_dispatch(&mut supervisor, &wasm, &ext_manifest, "init", 0)
        .unwrap_or_else(|| unreachable!("the first crash is inside the restart budget"));
    let err = first
        .err()
        .unwrap_or_else(|| unreachable!("trap-on-entry must fault"));
    assert!(
        matches!(err, HostError::Wasm(ref reason) if reason.contains("trap")),
        "typed wasm trap error, got: {err}"
    );
    assert!(matches!(
        supervisor.state(EXT_ID, EXT_VERSION),
        RunState::Enabled { crashes: 1, .. }
    ));

    // The `work` entry traps too — same containment, second decision.
    let second = supervised_dispatch(&mut supervisor, &wasm, &ext_manifest, "work", 1)
        .unwrap_or_else(|| unreachable!("still inside the budget"));
    assert!(second.is_err());
    assert!(matches!(
        supervisor.state(EXT_ID, EXT_VERSION),
        RunState::Enabled { crashes: 2, .. }
    ));

    // THE HOST SURVIVES: a healthy instance spawns and completes work
    // in the same process, same engine, same ledger discipline.
    let survivor_wasm = crash_survivor_wasm();
    let mut survivor = ok(spawn(
        &survivor_wasm,
        manifest(&manifest_toml(&survivor_wasm, "")),
        HostServices::empty(),
    ));
    assert_eq!(ok(survivor.invoke("hello", &[])), vec![0]);
    let logs = survivor.log_records();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].1, "survivor-still-alive");
}

#[test]
fn crash_injection_trap_mid_call_is_contained() {
    let wasm = crash_trap_mid_call_wasm();
    let ext_manifest = manifest(&manifest_toml(&wasm, ""));
    let mut supervisor = ExtensionSupervisor::new(ok(BackoffPolicy::from_schema_defaults()));

    // The trap fires AFTER an honest import call: the host state the
    // import touched (the log record) is intact and observable —
    // containment is clean, not lost.
    let mut instance = ok(spawn(&wasm, ext_manifest.clone(), HostServices::empty()));
    let err = instance
        .invoke("work", &[])
        .err()
        .unwrap_or_else(|| unreachable!("the mid-call trap must fault"));
    assert!(matches!(err, HostError::Wasm(_)), "got: {err}");
    assert!(matches!(instance.status(), InstanceStatus::Faulted { .. }));
    let logs = instance.log_records();
    assert_eq!(
        logs,
        vec![(2u8, "about to trap".to_string())],
        "the import's effect survived the contained trap"
    );
    // Further dispatch is refused honestly.
    assert!(matches!(
        instance.invoke("work", &[]),
        Err(HostError::InstanceFaulted { .. })
    ));
    // The supervisor sees the contained fault and schedules restart 1.
    let action = supervised_dispatch(&mut supervisor, &wasm, &ext_manifest, "work", 0);
    assert!(action.is_some_and(|outcome| outcome.is_err()));
    assert!(matches!(
        supervisor.state(EXT_ID, EXT_VERSION),
        RunState::Enabled { crashes: 1, .. }
    ));
    let events = supervisor.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, EXT_ID);
    assert_eq!(
        events[0].action,
        ActionTaken::Isolated {
            restart: 1,
            delay_ms: 0
        }
    );
}

#[test]
fn crash_injection_host_fault_from_import_is_contained() {
    let wasm = crash_escalate_wasm();
    let ext_manifest = manifest(&manifest_toml(
        &wasm,
        "http-fetch = true\nnetwork-host-allowlist = [\"good.test\"]",
    ));
    let mut supervisor = ExtensionSupervisor::new(ok(BackoffPolicy::from_schema_defaults()));

    // Doc 06 §7.5 case 3: the escalation threshold trips from INSIDE
    // an import callback — the host stops the guest mid-call with a
    // host fault, the instance faults with the typed disable error,
    // and the supervisor records it like any contained crash.
    let mut instance = ok(spawn(&wasm, ext_manifest, HostServices::empty()));
    let err = instance
        .invoke("work", &[])
        .err()
        .unwrap_or_else(|| unreachable!("the third allowlist violation must fault"));
    assert!(
        matches!(err, HostError::ExtensionDisabled { .. }),
        "typed escalation disable, got: {err}"
    );
    assert!(matches!(instance.status(), InstanceStatus::Faulted { .. }));
    assert_eq!(
        instance.ledger().escalation_count(),
        3,
        "three security events counted on the shared ledger"
    );
    // The embedder records the contained fault; restart 1 is granted.
    supervisor.record_crash(EXT_ID, EXT_VERSION, &err.to_string(), 0);
    assert!(matches!(
        supervisor.drain_events().as_slice(),
        [event] if event.action == ActionTaken::Isolated {
            restart: 1,
            delay_ms: 0
        }
    ));
}

#[test]
fn crash_injection_resource_exhaustion_is_contained() {
    // §9.4/§7.4 escalation: the fuel burner (CPU) and the memory hog
    // (OOM) both fault the instance with typed limit errors; the
    // supervisor runs them through the same crash policy (§5.4: every
    // limit breach "converts into the crash/isolation path").
    let mut supervisor = ExtensionSupervisor::new(ok(BackoffPolicy::from_schema_defaults()));

    let fuel_wasm = crash_fuel_burner_wasm();
    let fuel_manifest = manifest(&manifest_toml(&fuel_wasm, ""));
    let mut burner = ok(spawn(&fuel_wasm, fuel_manifest, HostServices::empty()));
    let start = std::time::Instant::now();
    let err = burner
        .invoke("work", &[])
        .err()
        .unwrap_or_else(|| unreachable!("the tight loop must be fuel-killed"));
    assert!(matches!(err, HostError::CpuLimit), "got: {err}");
    assert!(
        start.elapsed() < std::time::Duration::from_secs(5),
        "no hang: killed in {:?}",
        start.elapsed()
    );
    assert!(matches!(burner.status(), InstanceStatus::Faulted { .. }));
    supervisor.record_crash(EXT_ID, EXT_VERSION, &err.to_string(), 0);

    let oom_wasm = crash_oom_wasm();
    let oom_manifest = manifest(&manifest_toml(&oom_wasm, ""));
    let mut hog = ok(spawn(&oom_wasm, oom_manifest, HostServices::empty()));
    let err = hog
        .invoke("work", &[])
        .err()
        .unwrap_or_else(|| unreachable!("the grow loop must hit the store cap"));
    assert!(
        matches!(err, HostError::MemoryLimit { cap_bytes } if cap_bytes == 1024 * 1024),
        "MemoryOom at the host limiter, got: {err}"
    );
    assert!(matches!(hog.status(), InstanceStatus::Faulted { .. }));
    supervisor.record_crash(EXT_ID, EXT_VERSION, &err.to_string(), 1);

    // Both contained faults are ordinary crash-policy input: two
    // crashes so far, restart 2 after the 5 s backoff.
    let events = supervisor.drain_events();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[1].action,
        ActionTaken::Isolated {
            restart: 2,
            delay_ms: 5_000
        }
    );

    // The host heap is unaffected (bounded memory everywhere, D-2): a
    // fresh healthy instance spawns and completes work.
    let survivor_wasm = crash_survivor_wasm();
    let mut survivor = ok(spawn(
        &survivor_wasm,
        manifest(&manifest_toml(&survivor_wasm, "")),
        HostServices::empty(),
    ));
    assert_eq!(ok(survivor.invoke("hello", &[])), vec![0]);
}

// ---------------------------------------------------------------------------
// §9.5 plugin_restart_backoff — the full supervised sequence
// ---------------------------------------------------------------------------

#[test]
fn plugin_restart_backoff() {
    let wasm = crash_trap_on_entry_wasm();
    let ext_manifest = manifest(&manifest_toml(&wasm, ""));
    let mut supervisor = ExtensionSupervisor::new(ok(BackoffPolicy::from_schema_defaults()));

    // The embedder's virtual clock (doc 06 §5.8: the embedder owns
    // the clock; the supervisor is exactly testable).
    let mut now_ms = 0u64;
    let mut actions = Vec::new();
    for round in 0..4 {
        // Respect the scheduled backoff before respawning: the
        // decision from round k-1 said how long to wait.
        if let Some(ActionTaken::Isolated { delay_ms, .. }) = actions.last() {
            now_ms += delay_ms;
        }
        let outcome = supervised_dispatch(&mut supervisor, &wasm, &ext_manifest, "work", now_ms);
        let action = match outcome {
            // The spawn was policy-refused: Disabled.
            None => ActionTaken::Disabled,
            Some(result) => {
                assert!(
                    result.is_err(),
                    "round {round}: every dispatch of this fixture traps"
                );
                supervisor
                    .drain_events()
                    .pop()
                    .map(|event| event.action)
                    .unwrap_or_else(|| unreachable!("every crash emits an event"))
            }
        };
        actions.push(action);
    }

    // Doc 06 §5.6's default sequence, from the committed settings
    // schema: immediate, 5 s, 30 s, fourth crash → Disabled.
    assert_eq!(
        actions,
        vec![
            ActionTaken::Isolated {
                restart: 1,
                delay_ms: 0
            },
            ActionTaken::Isolated {
                restart: 2,
                delay_ms: 5_000
            },
            ActionTaken::Isolated {
                restart: 3,
                delay_ms: 30_000
            },
            ActionTaken::Disabled,
        ]
    );

    // The run-state is the truthful UI surface (doc 05 §1): Disabled
    // with the policy-timer deadline the settings layer configured.
    let state = supervisor.state(EXT_ID, EXT_VERSION);
    match state {
        RunState::Disabled {
            crashes,
            reenable_at_ms,
            ..
        } => {
            assert_eq!(crashes, 4);
            assert_eq!(
                reenable_at_ms,
                Some(now_ms + 900_000),
                "the 15 min default re-enable timer from the schema"
            );
        }
        other => unreachable!("must be Disabled, got {other:?}"),
    }

    // Further dispatch is policy-refused before any spawn happens —
    // the embedder consults the run-state first.
    assert!(supervised_dispatch(&mut supervisor, &wasm, &ext_manifest, "work", now_ms).is_none());

    // A user re-enable (doc 06 §5.6: resets the backoff) — the next
    // crash is an immediate restart again.
    supervisor.reenable(EXT_ID, EXT_VERSION);
    now_ms += 1_000;
    let outcome = supervised_dispatch(&mut supervisor, &wasm, &ext_manifest, "work", now_ms);
    assert!(outcome.is_some_and(|outcome| outcome.is_err()));
    assert!(matches!(
        supervisor.drain_events().as_slice(),
        [event] if event.action == ActionTaken::Isolated {
            restart: 1,
            delay_ms: 0
        }
    ));
}

// ---------------------------------------------------------------------------
// §9.5 plugin_crash_isolation — THE playback-unaffected proof
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps — the
/// scenario-harness pattern).
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The collecting sink: every delivered frame, in delivery order.
#[derive(Default)]
struct Collector {
    frames: Mutex<usize>,
}

impl FrameConsumer for Collector {
    fn accept(&self, _frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        *self.frames.lock().unwrap_or_else(|p| p.into_inner()) += 1;
        Ok(())
    }
}

/// The scripted demux stage: a fixed track list over a mutable event
/// script (the doc 03 §4.6(f) shape — the real engine driven by
/// scripted stages, never a mocked engine).
struct FakeDemuxer {
    tracks: Vec<Track>,
    script: Mutex<VecDeque<DemuxEvent>>,
}

impl Demuxer for FakeDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        Ok(self
            .script
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .pop_front()
            .unwrap_or(DemuxEvent::EndOfStream))
    }
    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.not-implemented"),
        ))
    }
}

/// One frame out per packet (the codec-api stale-packet guard kept).
struct FakeDecoder {
    generation: GenerationId,
}

impl Decoder for FakeDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        Ok(())
    }
    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        if packet.is_stale(self.generation) {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        let frame = Frame::Audio(media_types::frame::AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: 48_000,
            },
            samples: SharedBytes::from_static(&[0u8; 4]),
            pts: packet.pts,
            duration: None,
            generation: self.generation,
        });
        out.accept(frame)?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }
    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: a working decoder per track.
struct FakeProvider;

impl DecoderProvider for FakeProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(Box::new(FakeDecoder {
            generation: config.generation,
        }))
    }
}

/// Everything the proof needs to drive one real session.
struct PlaybackFixture {
    engine: Engine,
    collector: Arc<Collector>,
}

/// Open a real audio session with `packets` scripted packets then
/// end-of-stream (the `open_audio` scenario shape).
fn open_audio(packets: u64) -> PlaybackFixture {
    let track = Track {
        id: TrackId::new(1),
        stream: media_types::StreamId::new(1),
        kind: MediaKind::Audio,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: TimeBase::new(1, 1_000_000).must(),
            gaps: Vec::new(),
        },
    };
    let mut script: VecDeque<DemuxEvent> = (0..packets)
        .map(|index| {
            DemuxEvent::Packet(Packet {
                track: TrackId::new(1),
                pts: Stamp::Present(Timestamp::new(
                    (index * 1_000) as i64,
                    TimeBase::new(1, 1_000_000).must(),
                )),
                dts: Stamp::Missing,
                duration: Some(Duration::Known(Nanoseconds(1_000_000))),
                data: SharedBytes::from_static(&[7u8; 64]),
                keyframe: true,
                discontinuity: false,
                generation: GenerationId::FIRST,
            })
        })
        .collect();
    script.push_back(DemuxEvent::EndOfStream);

    let collector = Arc::new(Collector::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(SharedNow(AtomicU64::new(0))),
            Timestamp::new(0, TimeBase::new(1, 1_000_000).must()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-fake.media").must(),
        Box::new(FakeDemuxer {
            tracks: vec![track],
            script: Mutex::new(script),
        }),
        Arc::new(FakeProvider),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        clock,
    );
    config.queues = QueuePlan::new(limits, limits);
    config.budget = TickBudget {
        demux_events: 64,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    };
    config.throttle = EventPolicy {
        tick: TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    };
    let mut engine = Engine::new();
    let _handle = engine.open(config).must();
    PlaybackFixture { engine, collector }
}

/// One extension-pool storm dispatch (doc 06 §5.8, serialized):
/// spawn a fresh instance of the hostile extension and invoke it.
///
/// `catch_unwind` is the mechanical half of "no panic crosses the
/// boundary": the dispatch must RETURN (with a typed error), never
/// unwind — a trap that escaped wasmi would abort right here. Returns
/// the contained fault's typed text for the supervisor.
fn storm_dispatch(wasm: &[u8], ext_manifest: &PluginManifest) -> String {
    let mut instance = ok(spawn(wasm, ext_manifest.clone(), HostServices::empty()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        instance.invoke("work", &[])
    }))
    .unwrap_or_else(|payload| unreachable!("a trap crossed the sandbox boundary: {payload:?}"));
    let err = outcome.err().unwrap_or_else(|| unreachable!("must trap"));
    assert!(
        matches!(err, HostError::Wasm(_)),
        "contained typed error, got: {err}"
    );
    err.to_string()
}

#[test]
fn plugin_crash_isolation() {
    // THE proof (doc 06 §9.5, MP §6 "a plugin crash cannot take down
    // playback"): a real playback session runs to completion while a
    // trap-on-every-call extension is dispatched against it again and
    // again — playback delivers every frame, the extension ends the
    // session `Disabled` under the backoff policy, the host emits the
    // crash events, and no panic ever crosses the sandbox boundary.
    const PACKETS: u64 = 96;
    let mut playback = open_audio(PACKETS);

    let wasm = crash_trap_on_entry_wasm();
    let ext_manifest = manifest(&manifest_toml(&wasm, ""));
    let mut supervisor = ExtensionSupervisor::new(ok(BackoffPolicy::from_schema_defaults()));

    let mut now_ms = 0u64;
    let mut dispatches = 0usize;
    let mut last_frames = 0usize;

    // Phase 1 — playback under storm: drive the session one poll at a
    // time; every 8th step the "extension pool" dispatches the hostile
    // extension once while the policy still allows it.
    for step in 0..2_000u64 {
        playback.engine.poll();

        if step % 8 == 0
            && matches!(
                supervisor.state(EXT_ID, EXT_VERSION),
                RunState::Enabled { .. }
            )
        {
            now_ms += 10;
            let reason = storm_dispatch(&wasm, &ext_manifest);
            supervisor.record_crash(EXT_ID, EXT_VERSION, &reason, now_ms);
            dispatches += 1;
        }

        // Playback keeps making progress through every storm round:
        // frames never regress and reach the full script.
        let frames = *playback
            .collector
            .frames
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        assert!(frames >= last_frames, "frames never regress");
        last_frames = frames;
        if matches!(playback.engine.state(), SessionState::Ended) {
            break;
        }
    }

    // Playback ran to completion, every scripted frame delivered —
    // the crash storm never touched the pipeline.
    assert_eq!(
        playback.engine.state(),
        SessionState::Ended,
        "the playback session survived the extension storm"
    );
    assert_eq!(last_frames, PACKETS as usize, "all frames delivered");
    assert!(
        dispatches >= 1,
        "the storm was actually interleaved with playback"
    );

    // Phase 2 — the storm continues (dispatch-only) until the policy
    // disables the extension; the ended session is never disturbed.
    let mut watchdog = 0;
    while matches!(
        supervisor.state(EXT_ID, EXT_VERSION),
        RunState::Enabled { .. }
    ) {
        now_ms += 10;
        let reason = storm_dispatch(&wasm, &ext_manifest);
        supervisor.record_crash(EXT_ID, EXT_VERSION, &reason, now_ms);
        dispatches += 1;
        watchdog += 1;
        assert!(watchdog < 32, "the crash window is bounded by policy");
        assert_eq!(
            playback.engine.state(),
            SessionState::Ended,
            "the ended session is never disturbed"
        );
        assert_eq!(frames_of(&playback), PACKETS as usize);
    }

    // The extension crashed on every dispatch and was Disabled by the
    // fourth crash (doc 06 §5.6) — then stayed disabled.
    assert!(
        dispatches >= 4,
        "at least the full crash window ran (got {dispatches})"
    );
    assert!(matches!(
        supervisor.state(EXT_ID, EXT_VERSION),
        RunState::Disabled { crashes: 4, .. }
    ));
    // A further storm round spawns nothing: the embedder consults the
    // run-state before every dispatch (the Disabled truth of doc 05
    // §1's plugin UI). The four events are the doc 05 §3
    // `plugins.crashed` shape — one per crash, the final action
    // Disabled.
    let drained = supervisor.drain_events();
    assert_eq!(drained.len(), 4, "exactly one event per crash");
    assert_eq!(
        drained.last().map(|e| e.action.clone()),
        Some(ActionTaken::Disabled),
        "the fourth crash moved the extension to Disabled"
    );
    assert!(
        drained
            .iter()
            .all(|e| e.id == EXT_ID && e.version == EXT_VERSION)
    );
    assert!(
        drained.iter().all(|e| !e.reason.is_empty()),
        "every event carries the contained fault's reason"
    );
    // A user re-enable (doc 06 §5.6) resets the window: the next
    // crash is an immediate supervised restart again, event-shaped.
    supervisor.reenable(EXT_ID, EXT_VERSION);
    let reason = storm_dispatch(&wasm, &ext_manifest);
    supervisor.record_crash(EXT_ID, EXT_VERSION, &reason, now_ms);
    let events = supervisor.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, EXT_ID);
    assert_eq!(events[0].version, EXT_VERSION);
    assert_eq!(
        events[0].action,
        ActionTaken::Isolated {
            restart: 1,
            delay_ms: 0
        }
    );
    assert!(!events[0].reason.is_empty(), "the event carries the error");
}

/// Frames delivered so far (the collector's count).
fn frames_of(playback: &PlaybackFixture) -> usize {
    *playback
        .collector
        .frames
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}
