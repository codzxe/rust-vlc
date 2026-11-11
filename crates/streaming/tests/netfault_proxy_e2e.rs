//! Milestone-1 proofs for the synthetic fault proxy (issue #14,
//! 6.F1/6.F2 test support): every [`Fault`] mode fires **exactly as
//! scripted**, over the real-socket mockhttp transport, with the
//! injectable clock pacing delays.
//!
//! What each test pins:
//!
//! * `truncate_at_offset_*` — the byte-offset cut answers the typed
//!   [`NetworkError::BodyTruncated`] `{expected, received}` (the
//!   advertised-length-then-close shape), once, then heals;
//! * `reset_after_offset_*` — the mid-body reset answers a typed I/O
//!   error, connection-reset flavor, once, then heals;
//! * `drop_after_offset_*` — the black hole answers
//!   [`NetworkError::ReadTimeout`] (and `is_timeout()` agrees), once;
//! * `offset_past_body_end_*` — a fault scripted past the body's end
//!   never fires (the network healed before the cut) — the rule that
//!   makes the proxy exactly-scripted, not best-effort;
//! * `delay_paced_by_injectable_clock` — a `Delay` costs zero real
//!   time: the fake clock advanced by exactly the scripted ms and
//!   recorded the sleep;
//! * `fault_script_sequence_*` — multi-fault scripts fire in order,
//!   then pass through (the recover-after-a-bad-moment shape).

use std::sync::Arc;

use streaming::clock::FakeClock;
use streaming::error::NetworkError;
use streaming::fetch::{HttpFetch, HttpRequest};
use streaming::mockhttp::{MockClient, MockConfig, MockServer};

#[path = "netfault/mod.rs"]
mod netfault;

use netfault::{Fault, FaultProxy, FaultScript};
use testkit::Must;

/// A body with a recognizable byte pattern (offset drift is visible).
fn body(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// One mock server serving `/body` with `len` patterned bytes.
fn serve(len: usize) -> MockServer {
    match MockServer::spawn(MockConfig {
        body: body(len),
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    }
}

/// The proxy stack under test: real socket -> MockClient -> proxy.
fn proxied(script: FaultScript, clock: Arc<FakeClock>) -> FaultProxy {
    FaultProxy::new(
        Arc::new(MockClient::new()),
        script,
        clock as Arc<dyn streaming::clock::WallClock>,
    )
}

fn get(server: &MockServer) -> HttpRequest {
    HttpRequest::get(server.url("/body"))
}

#[test]
fn truncate_at_offset_fires_exactly_once() {
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::once(Fault::TruncateAt { at: 300 });
    let proxy = proxied(script.clone(), clock);

    // The cut: advertised 1000, received 300 — typed, positional.
    match proxy.fetch(get(&server)) {
        Err(NetworkError::BodyTruncated { expected, received }) => {
            assert_eq!((expected, received), (1000, 300));
        }
        other => unreachable!("truncate: wrong outcome {other:?}"),
    }
    // The network healed: script exhausted, full body again.
    let second = proxy.fetch(get(&server)).must();
    assert_eq!(second.body, body(1000));
    // Exactly as scripted: one fault bit, two fetches seen.
    assert_eq!(script.fired(), 1);
    assert_eq!(script.fetched(), 2);
}

#[test]
fn reset_after_offset_fires_exactly_once() {
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::once(Fault::ResetAfter { at: 300 });
    let proxy = proxied(script.clone(), clock);

    match proxy.fetch(get(&server)) {
        Err(NetworkError::Io { source }) => {
            assert_eq!(
                source.kind(),
                std::io::ErrorKind::ConnectionReset,
                "reset fault must be connection-reset flavored"
            );
        }
        other => unreachable!("reset: wrong outcome {other:?}"),
    }
    let second = proxy.fetch(get(&server)).must();
    assert_eq!(second.body.len(), 1000);
    assert_eq!(script.fired(), 1);
    assert_eq!(script.fetched(), 2);
}

#[test]
fn drop_after_offset_fires_exactly_once() {
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::once(Fault::DropAfter { at: 300 });
    let proxy = proxied(script.clone(), clock);

    match proxy.fetch(get(&server)) {
        Err(err @ NetworkError::ReadTimeout) => {
            assert!(err.is_timeout(), "ReadTimeout must self-report as timeout");
        }
        other => unreachable!("drop: wrong outcome {other:?}"),
    }
    assert!(proxy.fetch(get(&server)).is_ok());
    assert_eq!(script.fired(), 1);
    assert_eq!(script.fetched(), 2);
}

#[test]
fn offset_past_body_end_never_fires() {
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::once(Fault::TruncateAt { at: 5000 });
    let proxy = proxied(script.clone(), clock);

    // The cut was scripted past the body's end: the fetch completed
    // before the fault could bite — full body, zero faults fired.
    let resp = proxy.fetch(get(&server)).must();
    assert_eq!(resp.body, body(1000));
    assert_eq!(script.fired(), 0);
    assert_eq!(script.fetched(), 1);
}

#[test]
fn delay_paced_by_injectable_clock() {
    let server = serve(100);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::once(Fault::Delay { ms: 900 });
    let proxy = proxied(script.clone(), clock.clone());
    // The shared script handle sees the proxy's counters (the clone
    // FaultProxy::new stored is the same script — asserted below).
    assert_eq!(script.fetched(), 0, "nothing fetched yet");

    let resp = proxy.fetch(get(&server)).must();
    assert_eq!(resp.body.len(), 100);
    // The delay was paid in VIRTUAL time, exactly once, exactly as
    // scripted — zero real wall-clock cost, full observability.
    assert_eq!(clock.sleeps(), vec![900]);
    assert_eq!(script.fired(), 1);
    // And the healed fetch after it needs no sleep at all.
    proxy.fetch(get(&server)).must();
    assert_eq!(clock.sleeps(), vec![900], "no extra sleeps after heal");
}

#[test]
fn fault_script_sequence_fires_in_order_then_heals() {
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::new(vec![
        Fault::Delay { ms: 5 },
        Fault::TruncateAt { at: 10 },
        Fault::DropAfter { at: 10 },
    ]);
    let proxy = proxied(script.clone(), clock.clone());

    // 1: delayed success.
    assert!(proxy.fetch(get(&server)).is_ok());
    // 2: the truncation.
    assert!(matches!(
        proxy.fetch(get(&server)),
        Err(NetworkError::BodyTruncated {
            expected: 1000,
            received: 10
        })
    ));
    // 3: the drop.
    assert!(matches!(
        proxy.fetch(get(&server)),
        Err(NetworkError::ReadTimeout)
    ));
    // 4: script exhausted — the healthy network is back.
    let healed = proxy.fetch(get(&server)).must();
    assert_eq!(healed.body, body(1000));

    assert_eq!(script.fired(), 3);
    assert_eq!(script.fetched(), 4);
    assert_eq!(clock.sleeps(), vec![5], "only the scripted delay slept");
}

#[test]
fn persistent_fault_script_never_heals_within_its_budget() {
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::forever(Fault::ResetAfter { at: 10 });
    let proxy = proxied(script.clone(), clock);

    // The outage persists: every fetch bites the same reset — the
    // arm the bounded-backoff proofs (6.F2) exhaust their budget
    // against. Three probes, three faults, no healing.
    for _ in 0..3 {
        assert!(matches!(
            proxy.fetch(get(&server)),
            Err(NetworkError::Io { .. })
        ));
    }
    assert_eq!(script.fired(), 3);
    assert_eq!(script.fetched(), 3);
    // ...but the script is finite (64 entries), so a runaway caller
    // eventually sees the healed pass-through, never a spin.
    for _ in 0..64 {
        let _ = proxy.fetch(get(&server));
    }
    assert!(proxy.fetch(get(&server)).is_ok(), "finite script heals");
    assert_eq!(script.fired(), 64);
}
