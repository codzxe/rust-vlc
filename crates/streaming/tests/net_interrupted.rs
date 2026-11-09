//! The interrupted-network scenarios (issue #14, M3 — cards 6.F2/
//! 6.F3): the M1 [`FaultProxy`] + [`FakeClock`] test support driving
//! the real streaming consumers, over the real-socket mockhttp
//! transport, end to end.
//!
//! Three scenarios, one per failure class the wave was asked to
//! prove:
//!
//! * `mid_segment_drop_resumes_byte_exact` — the black hole bites
//!   mid-segment; [`RetryFetch`] re-issues the identical ranged
//!   request and the resumed body is **byte-exact**;
//! * `manifest_reset_fails_typed_with_bounded_backoff` — a
//!   persistent reset outage at the manifest; the failure surfaces
//!   as the typed `streaming.hls.fetch-failed` refusal after a
//!   **bounded**, fully virtual backoff ladder (every sleep under
//!   the policy's per-attempt ceiling, zero real wall-clock cost);
//! * `live_window_edge_survives_wire_delay` — a delayed wire at
//!   live-window join time; the session still starts at the live
//!   edge, loses no window, and streams **byte-exact** against the
//!   on-disk oracle, with the delays paid in exactly the scripted
//!   virtual milliseconds.
//!
//! Stack under test (the composition the reconnect module docs
//! describe): real socket → `MockClient` → `FaultProxy` (scripted)
//! → `RetryFetch` (where bounded retry is the scenario) → the HLS
//! session.

use std::sync::Arc;
use std::time::{Duration, Instant};

use plugin_api::access::SourceStream;
use streaming::clock::{FakeClock, WallClock};
use streaming::fetch::{ByteRange, HttpFetch, HttpRequest};
use streaming::hls::live::{HlsLiveConfig, open_live};
use streaming::hls::media::{HlsVodConfig, open_vod};
use streaming::mockhttp::{MockClient, MockConfig, MockServer, ScriptedRoute};
use streaming::reconnect::RetryFetch;

#[path = "netfault/mod.rs"]
mod netfault;

use netfault::{Fault, FaultProxy, FaultScript};

/// A body with a recognizable byte pattern (offset drift is visible).
fn body(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// One mock server serving `len` patterned bytes at `/body`, Range
/// honored (the ranged-resource resume shape).
fn serve(len: usize) -> MockServer {
    match MockServer::spawn(MockConfig {
        body: body(len),
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    }
}

/// The proxied transport stack: socket → MockClient → proxy.
fn proxied(script: FaultScript, clock: Arc<FakeClock>) -> FaultProxy {
    FaultProxy::new(
        Arc::new(MockClient::new()),
        script,
        clock as Arc<dyn WallClock>,
    )
}

// ---------------------------------------------------------------------------
// (a) Mid-segment drop → RetryFetch resume, byte-exact
// ---------------------------------------------------------------------------

/// The black hole bites 300 bytes into a ranged segment fetch; the
/// retry wrapper re-issues the **identical** ranged request (its
/// documented resume shape) and the healed network answers the same
/// slice — byte-exact, with exactly one fault fired and one bounded
/// backoff sleep observed on the virtual clock.
#[test]
fn mid_segment_drop_resumes_byte_exact() {
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::once(Fault::DropAfter { at: 300 });
    let proxy = proxied(script.clone(), clock.clone());
    let retry = RetryFetch::new(Arc::new(proxy), clock.clone()).expect("default policy is valid");

    // The ranged segment request (the ranged resource's resume unit):
    // bytes 250.. to the end.
    let request = HttpRequest::get(server.url("/body")).with_range(ByteRange::From { start: 250 });

    let response = retry.fetch(request).expect("resumed ranged fetch");
    // Byte-exact resume: the healed re-issue answered the SAME slice.
    assert_eq!(
        response.body,
        body(1000)[250..],
        "resumed body must equal the requested range, byte-exact"
    );

    // Exactly as scripted: the drop bit once; the retry re-issued the
    // identical request (the proxy saw exactly two fetches).
    assert_eq!(script.fired(), 1);
    assert_eq!(script.fetched(), 2);

    // The bounded backoff, observed: at most one sleep (a full-jitter
    // draw of zero advances no virtual time and records nothing),
    // and any sleep that did happen sits under the policy's first
    // ceiling (500 ms by default).
    let policy = retry.policy();
    let sleeps = clock.sleeps();
    assert!(
        sleeps.len() <= 1,
        "one retry means at most one backoff sleep (slept {sleeps:?})"
    );
    for sleep in &sleeps {
        assert!(
            *sleep <= policy.ceiling_ms(0),
            "first backoff sleep {sleep} over the ceiling {}",
            policy.ceiling_ms(0)
        );
    }

    // And the wrapper's own ledger agrees: one fetch, one retry,
    // recovered, nothing exhausted.
    let stats = retry.stats();
    assert_eq!(stats.fetches, 1);
    assert_eq!(stats.retries, 1);
    assert_eq!(stats.recovered, 1);
    assert_eq!(stats.exhausted, 0);

    // The sibling mid-segment fault — the advertised-length cut —
    // resumes the same way on a plain (unranged) segment GET: the
    // typed truncation, then the healed byte-exact full body.
    let server = serve(1000);
    let clock = Arc::new(FakeClock::new());
    let script = FaultScript::once(Fault::TruncateAt { at: 300 });
    let proxy = proxied(script.clone(), clock.clone());
    let retry = RetryFetch::new(Arc::new(proxy), clock.clone()).expect("default policy is valid");
    let response = retry
        .fetch(HttpRequest::get(server.url("/body")))
        .expect("resumed plain fetch");
    assert_eq!(response.body, body(1000), "the whole body, byte-exact");
    assert_eq!(script.fired(), 1);
    assert_eq!(script.fetched(), 2);
    let stats = retry.stats();
    assert_eq!((stats.retries, stats.recovered, stats.exhausted), (1, 1, 0));
    assert!(
        clock.sleeps().len() <= 1,
        "one bounded backoff sleep at most"
    );
}

// ---------------------------------------------------------------------------
// (b) Manifest reset → typed failure + bounded backoff
// ---------------------------------------------------------------------------

/// A persistent reset outage (the network never heals within any
/// sane budget): every fetch — starting with the manifest — dies in
/// a connection reset. [`open_vod`] must refuse with the typed
/// `streaming.hls.fetch-failed` error after the bounded retry budget
/// is spent, with every backoff sleep on the virtual clock and each
/// under its per-attempt exponential ceiling.
#[test]
fn manifest_reset_fails_typed_with_bounded_backoff() {
    // A minimal honest VOD manifest (ENDLIST, plain segments).
    let manifest = b"#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:2\n\
        #EXTINF:2.0,\nseg0.ts\n\
        #EXTINF:2.0,\nseg1.ts\n\
        #EXT-X-ENDLIST\n"
        .to_vec();
    let server = match MockServer::spawn(MockConfig {
        body: manifest,
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    };

    let clock = Arc::new(FakeClock::new());
    // 64 scripted resets: more than any retry budget, still finite.
    let script = FaultScript::forever(Fault::ResetAfter { at: 10 });
    let proxy = proxied(script.clone(), clock.clone());
    let retry = Arc::new(RetryFetch::new(Arc::new(proxy), clock.clone()).expect("policy"));

    let started = Instant::now();
    let transport: Arc<dyn HttpFetch> = retry.clone();
    let error = match open_vod(transport, &server.url("/pl.m3u8"), HlsVodConfig::default()) {
        Ok(_) => unreachable!("a persistent reset outage must not open a session"),
        Err(e) => e,
    };

    // The typed refusal: the fetch-failed mapping of the reset class.
    assert_eq!(
        error.context.code, "streaming.hls.fetch-failed",
        "the refusal must be the typed fetch-failure, got {error:?}"
    );
    assert!(
        matches!(error.kind, media_types::ErrorKind::Network { .. }),
        "the mapped kind must carry the network cause"
    );

    // Bounded retry, observed: the budget (default 4 retries) was
    // spent — 5 wire fetches, every one bitten by the scripted reset.
    let stats = retry.stats();
    assert_eq!(stats.retries, 4);
    assert_eq!(stats.exhausted, 1);
    assert_eq!(stats.recovered, 0);
    assert_eq!(script.fetched(), 5);
    assert_eq!(script.fired(), 5);

    // Bounded backoff, observed: at most one sleep per retry (a
    // full-jitter zero draw records nothing), each under its own
    // exponential ceiling; the total is under the ceiling sum.
    let policy = retry.policy();
    let sleeps = clock.sleeps();
    assert!(
        sleeps.len() as u32 <= policy.max_retries,
        "one sleep per retry at most (slept {sleeps:?})"
    );
    let ceiling_sum: u64 = (0..policy.max_retries).map(|a| policy.ceiling_ms(a)).sum();
    for (attempt, sleep) in sleeps.iter().enumerate() {
        assert!(
            *sleep <= policy.ceiling_ms(attempt as u32),
            "backoff sleep {sleep} over attempt-{attempt} ceiling"
        );
    }
    assert!(
        sleeps.iter().sum::<u64>() <= ceiling_sum,
        "total backoff {sleeps:?} over the ladder sum {ceiling_sum}"
    );

    // And the whole bounded ladder cost zero real wall-clock time.
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the backoff must be virtual, not waited (took {:?})",
        started.elapsed()
    );
}

// ---------------------------------------------------------------------------
// (c) Live-window edge under wire delay
// ---------------------------------------------------------------------------

/// The on-disk oracle (measured, the same fixtures the live e2e
/// uses): joining back=2 on W1 and playing to the ENDLIST end.
const ORACLE_BYTES: u64 = 100_204;

fn fixture(name: &str) -> Vec<u8> {
    testkit::fixture_bytes(&format!("hls/ts_live/{name}")).must()
}

// The `Must` trait import the `.must()` fixture reads use above.
use testkit::Must;

/// The four live windows (the same advance shape the live e2e pins:
/// a sliding window over the real ffmpeg-encoded segments, one real
/// discontinuity seam, then ENDLIST).
fn windows() -> Vec<Vec<u8>> {
    let w1 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXTINF:2.507756,\ns00.ts\n\
        #EXTINF:2.507756,\ns01.ts\n\
        #EXTINF:2.484533,\ns02.ts\n";
    let w2 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:1\n\
        #EXTINF:2.507756,\ns01.ts\n\
        #EXTINF:2.484533,\ns02.ts\n\
        #EXT-X-DISCONTINUITY\n\
        #EXTINF:2.507767,\nt00.ts\n";
    let w3 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:2\n\
        #EXTINF:2.484533,\ns02.ts\n\
        #EXT-X-DISCONTINUITY\n\
        #EXTINF:2.507767,\nt00.ts\n\
        #EXTINF:2.507767,\nt01.ts\n";
    let w4 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:2\n\
        #EXTINF:2.484533,\ns02.ts\n\
        #EXT-X-DISCONTINUITY\n\
        #EXTINF:2.507767,\nt00.ts\n\
        #EXTINF:2.507767,\nt01.ts\n\
        #EXT-X-ENDLIST\n";
    [w1, w2, w3, w4]
        .iter()
        .map(|w| w.as_bytes().to_vec())
        .collect()
}

/// Read exactly `n` bytes (the live consumer's loop).
fn read_exactly(
    stream: &mut streaming::hls::live::HlsLiveStream,
    n: usize,
) -> Result<Vec<u8>, media_types::EngineError> {
    let mut all = Vec::new();
    while all.len() < n {
        let chunk = stream.read(n - all.len())?;
        if chunk.is_empty() {
            break;
        }
        all.extend_from_slice(&chunk);
    }
    Ok(all)
}

/// A slow wire at live-join time: the open and the first edge
/// segment arrive 900 ms and 400 ms late (virtually — the fault
/// proxy paces its delays through the fake clock). The session must
/// still join at the live edge (`live_edge_back = 2`), lose no
/// segment to the window sliding under the delay, and stream
/// byte-exact against the on-disk oracle.
#[test]
fn live_window_edge_survives_wire_delay() {
    let routes = ["s00.ts", "s01.ts", "s02.ts", "t00.ts", "t01.ts"]
        .iter()
        .map(|name| (format!("/hls/ts_live/{name}"), fixture(name)))
        .collect();
    let server = match MockServer::spawn(MockConfig {
        routes,
        scripted_routes: vec![(
            "/hls/ts_live/pl.m3u8".to_owned(),
            ScriptedRoute::new(windows()),
        )],
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    };

    let clock = Arc::new(FakeClock::new());
    // The wire is slow for the first two fetches (the manifest, then
    // the first edge segment), then heals.
    let script = FaultScript::new(vec![Fault::Delay { ms: 900 }, Fault::Delay { ms: 400 }]);
    let proxy = proxied(script.clone(), clock.clone());

    let config = HlsLiveConfig {
        live_edge_back: 2,
        max_stalled_reloads: 2,
        ..HlsLiveConfig::default()
    };
    let started = Instant::now();
    let mut stream = match open_live(Arc::new(proxy), &server.url("hls/ts_live/pl.m3u8"), config) {
        Ok(s) => s,
        Err(e) => unreachable!("live session must open through the delay: {e}"),
    };

    // Byte-exact against cat(s01, s02, t00, t01) — the edge join
    // (back=2 on W1) held despite the delayed wire.
    let streamed = match read_exactly(&mut stream, ORACLE_BYTES as usize) {
        Ok(b) => b,
        Err(e) => unreachable!("read across the slides under delay: {e}"),
    };
    let mut on_disk = Vec::new();
    for name in ["s01.ts", "s02.ts", "t00.ts", "t01.ts"] {
        on_disk.extend_from_slice(&fixture(name));
    }
    assert_eq!(streamed.len(), ORACLE_BYTES as usize);
    assert_eq!(
        streamed, on_disk,
        "delayed-wire session == cat(played window)"
    );

    // The edge held: nothing slid out of the server window unfetched.
    let stats = stream.stats();
    assert_eq!(stats.segments_lost_window, 0);

    // The delays were paid in VIRTUAL time, exactly as scripted (the
    // two scripted fetches, then a healed wire — no other sleeps: the
    // config runs unpaced demand-driven reloads).
    assert_eq!(clock.sleeps(), vec![900, 400]);
    assert_eq!(script.fired(), 2);

    // Zero real wall-clock cost for the 1.3 s of scripted latency.
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the wire delay must be virtual, not waited (took {:?})",
        started.elapsed()
    );
}
