
// ---------------------------------------------------------------------------
// Epoch-aware output hub (W1-10 wiring, audit M11)
// ---------------------------------------------------------------------------

/// Open a session on an EXISTING engine (the M11 scenario needs one
/// engine-scoped output hub across two sessions).
fn open_on(engine: &mut Engine, script_len: u64) -> SessionHandle {
    let script: Vec<DemuxEvent> = (0..script_len)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-fake.media").must(),
        Box::new(FakeDemuxer {
            tracks: vec![track(1, MediaKind::Audio)],
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start: None,
        }),
        Arc::new(FakeProvider { fail_select: None }),
        Arc::new(Collector::default()),
        virtual_clock(),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    engine.open(config).must()
}

fn prime_playing(engine: &mut Engine, context: &str) {
    for _ in 0..40 {
        if engine.state() == SessionState::Playing {
            return;
        }
        engine.poll();
    }
    assert_eq!(engine.state(), SessionState::Playing, "{context}");
}

#[test]
fn superseded_epoch_events_never_leak_into_the_next_session() {
    // M11: one engine, one engine-scoped output hub, two sessions.
    // Session 1 (epoch 1) runs open→play→stop→close with NO drain;
    // session 2 (epoch 2) opens at pipeline 0 — every epoch-1 event
    // is now stale BY CONSTRUCTION. The bare `generation < live`
    // comparison waved all ten of them through (verified: seq 1..8
    // at gen 0, seq 9 at gen 1, seq 10 at gen 2 — all leaked).
    let mut engine = Engine::new();

    let handle1 = open_on(&mut engine, 200);
    prime_playing(&mut engine, "priming 1");
    handle1.stop(RequestId(80)).must();
    engine.poll();
    handle1.close(RequestId(81)).must();
    engine.poll();
    assert_eq!(engine.state(), SessionState::Idle);

    let handle2 = open_on(&mut engine, 200);
    prime_playing(&mut engine, "priming 2");
    drop(handle2);

    // NOW drain: current pair is (epoch 2, pipeline 0).
    let outputs = engine.take_outputs();
    let stale_leaked: Vec<(u64, SessionId, Generation)> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => Some((event.seq, event.session, event.generation)),
            _ => None,
        })
        .filter(|(_, session, _)| *session != SessionId(2))
        .collect();
    assert!(
        stale_leaked.is_empty(),
        "superseded-epoch events leaked across the session boundary: {stale_leaked:?}"
    );
    assert!(
        engine.stats().suppressed_stale_events > 0,
        "the epoch boundary must count its suppressions: {:?}",
        engine.stats()
    );
    // Receipts of the dead session's commands survive (doc 05 §4.3:
    // responses are never suppressed) — the fix must not overreach.
    let receipts: Vec<RequestId> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt.request),
            _ => None,
        })
        .collect();
    assert!(receipts.contains(&RequestId(80)));
    assert!(receipts.contains(&RequestId(81)));
}
