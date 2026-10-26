//! The 6.B1b end-to-end proofs: a REAL live-shaped HLS source —
//! ffmpeg-generated MPEG-TS segments behind a reload-mutating
//! playlist — served over a **real socket** by the mock testkit's
//! scripted route, opened through the live session, and played to
//! its `ENDLIST` end.
//!
//! # Fixtures (generated this session, ffmpeg 7.1.5)
//!
//! `tests/fixtures/hls/ts_live/` — TWO encoding lanes, so the
//! `#EXT-X-DISCONTINUITY` marker in the playlist sits on a **real**
//! encoding boundary (AAC-LC 44.1 kHz mono → AAC-LC 22.05 kHz mono),
//! not a cosmetic flag:
//!
//! ```text
//! ffmpeg -y -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=7.5" \
//!   -c:a aac -b:a 64k -f segment -segment_time 2.5 \
//!   -segment_list s.m3u8 -segment_list_type m3u8 s%02d.ts
//! ffmpeg -y -f lavfi -i "sine=frequency=880:sample_rate=22050:duration=5.0" \
//!   -c:a aac -b:a 64k -f segment -segment_time 2.5 \
//!   -segment_list t.m3u8 -segment_list_type m3u8 t%02d.ts
//! ```
//!
//! The committed `s.m3u8`/`t.m3u8` are ffmpeg's own output (the
//! generation evidence — a finite encode always ends with
//! `#EXT-X-ENDLIST`, which is why the live windows below are
//! hand-authored); their measured EXTINF durations are reused
//! verbatim. ffmpeg's ~0.02 s flush-tail segments (`s03.ts`,
//! `t02.ts`) are dropped: the corpus is the three/two full segments.
//!
//! # The live source this test scripts
//!
//! The scripted playlist route advances through four windows over the
//! same on-disk segments (a server publishing a sliding window):
//!
//! * `W1` `MEDIA-SEQUENCE:0` → s00, s01, s02
//! * `W2` `MEDIA-SEQUENCE:1` → s01, s02, `#EXT-X-DISCONTINUITY`, t00
//! * `W3` `MEDIA-SEQUENCE:2` → s02, `#EXT-X-DISCONTINUITY`, t00, t01
//! * `W4` `MEDIA-SEQUENCE:2` + `#EXT-X-ENDLIST` (the stream ends)
//!
//! Joining `live_edge_back = 2` on W1 starts at s01, so the honest
//! oracle is `cat(s01, s02, t00, t01)` = **100 204 bytes** (measured
//! this session), with one recorded discontinuity seam at t00's
//! boundary (offset 51 136) and three reloads along the way.
//!
//! # What is NOT claimed here
//!
//! No decode differential across the discontinuity seam: the v0
//! contract records the seam (boundaries + stats) while the stream
//! stays byte-concatenation — the decoded-content differential across
//! real encoder switches is B3's conformance corpus. Live sessions
//! are unknown-length by design, so container playback before the
//! `ENDLIST` is the later streaming-container lane; the demuxer below
//! starts only once the session has ended and drained (size known).

use std::sync::Arc;

use demux::ts::{PesConsumer, TsDemuxer};
use plugin_api::{DemuxEvent, Demuxer, SeekOrigin, SourceStream};
use streaming::fetch::HttpFetch;
use streaming::hls::live::{HlsLiveConfig, open_live};
use streaming::mockhttp::{MockClient, MockConfig, MockServer, ScriptedRoute};
use testkit::Must;

/// The on-disk oracle (measured this session): joining back=2 on W1
/// and playing to the ENDLIST end.
const ORACLE_BYTES: u64 = 100_204;
/// Where t00's bytes start in the concatenation (s01+s02 = 51 136).
const DISCONTINUITY_OFFSET: u64 = 51_136;

fn fixture(name: &str) -> Vec<u8> {
    testkit::fixture_bytes(&format!("hls/ts_live/{name}")).must()
}

/// The four windows (hand-authored over the ffmpeg-measured EXTINF
/// durations; `#EXT-X-DISCONTINUITY` sits on the real 44.1k→22.05k
/// encoding boundary).
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

/// The live source over a real socket: the playlist route scripted
/// through `bodies`, the segment resources static.
fn serve_live(bodies: Vec<Vec<u8>>) -> MockServer {
    let routes = ["s00.ts", "s01.ts", "s02.ts", "t00.ts", "t01.ts"]
        .iter()
        .map(|name| (format!("/hls/ts_live/{name}"), fixture(name)))
        .collect();
    match MockServer::spawn(MockConfig {
        routes,
        scripted_routes: vec![(
            "/hls/ts_live/pl.m3u8".to_owned(),
            ScriptedRoute::new(bodies),
        )],
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    }
}

fn live_config() -> HlsLiveConfig {
    HlsLiveConfig {
        live_edge_back: 2,
        max_stalled_reloads: 2,
        ..HlsLiveConfig::default()
    }
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

/// The main lane: live open at the edge → two window slides → a real
/// discontinuity seam → ENDLIST termination — byte-exact against the
/// on-disk oracle, the seam recorded, the wire shape pinned, and the
/// (now length-known) ended session demuxing through the REAL TS
/// machinery.
#[test]
fn live_slides_endlist_and_discontinuity_over_a_real_socket() {
    let server = serve_live(windows());
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    let mut stream = match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), live_config()) {
        Ok(s) => s,
        Err(e) => unreachable!("live session must open: {e}"),
    };
    assert_eq!(stream.size(), None, "live = unknown length by design");

    // (1) Byte-exact against cat(s01, s02, t00, t01).
    let streamed = match read_exactly(&mut stream, ORACLE_BYTES as usize) {
        Ok(b) => b,
        Err(e) => unreachable!("read across the slides: {e}"),
    };
    let mut on_disk = Vec::new();
    for name in ["s01.ts", "s02.ts", "t00.ts", "t01.ts"] {
        on_disk.extend_from_slice(&fixture(name));
    }
    assert_eq!(streamed.len(), ORACLE_BYTES as usize);
    assert_eq!(streamed, on_disk, "session stream == cat(played window)");

    // (2) ENDLIST termination: EOF shape + the exact total.
    let eof = match stream.read(64) {
        Ok(c) => c,
        Err(e) => unreachable!("post-ENDLIST read: {e}"),
    };
    assert!(eof.is_empty(), "ENDLIST drains to EndOfStream");
    assert_eq!(
        stream.size(),
        Some(ORACLE_BYTES),
        "exact total once ended+drained"
    );

    // (3) The discontinuity record: the seam on t00's boundary, the
    // window identity, and the reload accounting.
    let boundaries = stream.segment_boundaries();
    assert_eq!(boundaries.len(), 4, "one boundary per played segment");
    assert!(!boundaries[0].discontinuity && !boundaries[1].discontinuity);
    assert_eq!(boundaries[2].media_sequence, 3, "t00's absolute sequence");
    assert_eq!(boundaries[2].offset, DISCONTINUITY_OFFSET);
    assert!(
        boundaries[2].discontinuity,
        "the 44.1k→22.05k seam is recorded"
    );
    assert_eq!(stream.stats().discontinuities, 1);
    assert_eq!(stream.stats().segments_fetched, 4);
    assert_eq!(stream.stats().playlist_reloads, 3, "one per dry window");
    assert_eq!(stream.stats().stalled_reloads, 0, "every reload productive");
    assert_eq!(stream.stats().segments_lost_window, 0);
    assert_eq!(stream.media_sequence(), 2, "the final window base");

    // (4) The wire shape, pinned: playlist, s01, s02, RELOAD, t00,
    // RELOAD, t01, RELOAD(=ENDLIST). Live = NO size probes (the
    // contrast with the VOD session's probe contract: unknown length
    // is the honest live shape).
    let requests = server.requests();
    let paths: Vec<&str> = requests.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/hls/ts_live/pl.m3u8",
            "/hls/ts_live/s01.ts",
            "/hls/ts_live/s02.ts",
            "/hls/ts_live/pl.m3u8",
            "/hls/ts_live/t00.ts",
            "/hls/ts_live/pl.m3u8",
            "/hls/ts_live/t01.ts",
            "/hls/ts_live/pl.m3u8",
        ],
        "demand-driven: reload exactly when the window runs dry"
    );
    assert!(
        requests.iter().all(|r| r.header("range").is_none()),
        "live sessions never probe sizes (whole-body GETs only)"
    );

    // (5) The ended session is now length-known and fully retained:
    // rewind and demux through the REAL TS machinery (the PSI
    // head-scan exercises the live session's retained-prefix seek).
    // The PES payloads flow ACROSS the recorded seam — the decode
    // differential across real encoder switches is B3's corpus, not
    // claimed here.
    let pos = match stream.seek(SeekOrigin::Start(0)) {
        Ok(p) => p,
        Err(e) => unreachable!("rewind the ended session: {e}"),
    };
    assert_eq!(pos.0, 0);
    struct CountingPes {
        pushes: std::sync::Arc<std::sync::Mutex<u64>>,
    }
    impl PesConsumer for CountingPes {
        fn pes_payload(
            &mut self,
            _pid: u16,
            _pusi: bool,
            _payload: &[u8],
        ) -> Result<(), media_types::EngineError> {
            match self.pushes.lock() {
                Ok(mut guard) => {
                    *guard += 1;
                    Ok(())
                }
                Err(_) => Err(media_types::EngineError::new(
                    media_types::PipelineStage::Demux,
                    media_types::ErrorKind::Unsupported { format: None },
                    media_types::error::ErrorContext::new("streaming.hls.test-counter-poisoned"),
                )),
            }
        }
    }
    let mut demuxer = match TsDemuxer::new(Box::new(stream)) {
        Ok(d) => d,
        Err(e) => unreachable!("ts open over the ended live session: {e}"),
    };
    let pushes = Arc::new(std::sync::Mutex::new(0u64));
    demuxer.set_pes_consumer(Box::new(CountingPes {
        pushes: Arc::clone(&pushes),
    }));
    let tracks = match demuxer.start() {
        Ok(t) => t,
        Err(e) => unreachable!("ts start (the PSI scan rewinds): {e}"),
    };
    assert_eq!(tracks.len(), 1, "one audio track");
    assert_eq!(tracks[0].codec.as_str(), "aac");
    let mut events = 0u64;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(_)) => events += 1,
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("ended session demuxes cleanly: {err}"),
        }
    }
    let pes_pushes = match pushes.lock() {
        Ok(guard) => *guard,
        Err(_) => unreachable!("the counter lock"),
    };
    assert!(pes_pushes > 0, "PES flowed across the seam (measured)");
    eprintln!(
        "live lane: {ORACLE_BYTES} bytes byte-exact, seam at {DISCONTINUITY_OFFSET} recorded, \
         {pes_pushes} PES pushes / {events} packets through the ended session (measured)"
    );
}

/// A source that never publishes past the opening window: the
/// stalled-reload budget fails typed over the real socket, with the
/// request count measured.
#[test]
fn stalled_live_source_fails_typed_live_timeout() {
    // One window, repeated forever: the server never slides.
    let bodies = vec![windows()[0].clone()];
    let server = serve_live(bodies);
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    let mut stream = match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), live_config()) {
        Ok(s) => s,
        Err(e) => unreachable!("live session must open: {e}"),
    };
    // back=2: s01+s02 play (51 136 bytes, measured).
    let streamed = match read_exactly(&mut stream, 51_136) {
        Ok(b) => b,
        Err(e) => unreachable!("read the window: {e}"),
    };
    assert_eq!(streamed.len(), 51_136);
    assert_eq!(stream.stats().segments_fetched, 2);
    // At the live edge with a never-advancing window: budget 2 → the
    // 3rd consecutive unproductive reload refuses typed.
    let err = match stream.read(1) {
        Ok(_) => unreachable!("a stalled live source must refuse, not EOF"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "streaming.hls.live-timeout");
    assert_eq!(stream.stats().stalled_reloads, 3, "measured stalls");
    assert_eq!(stream.stats().playlist_reloads, 3);
    let logged = server.requests();
    let paths: Vec<&str> = logged.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/hls/ts_live/pl.m3u8",
            "/hls/ts_live/s01.ts",
            "/hls/ts_live/s02.ts",
            "/hls/ts_live/pl.m3u8",
            "/hls/ts_live/pl.m3u8",
            "/hls/ts_live/pl.m3u8",
        ],
        "open, the two segments, then three stalled reloads"
    );
}
