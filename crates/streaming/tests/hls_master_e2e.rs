//! The 6.B2a/6.B2b end-to-end proofs: a multi-variant master served
//! over a **real socket** by the mock HTTP testkit, opened through the
//! adaptive session, with the bandwidth trace **scripted** through the
//! host-observation seam (`note_throughput` — the same seam the
//! playback engine uses in production to feed 6.D1's buffer model).
//!
//! # What each test proves
//!
//! 1. **Selection** — the master's ladder selects the highest
//!    bandwidth under the configured cap, and the wire shows it: the
//!    first requests are the master, the selected variant's playlist,
//!    then that variant's segments.
//! 2. **Scripted drop/rise** — a bandwidth crash steps the ladder
//!    down at segment boundaries (one rung per boundary), a recovery
//!    steps it back up; the server's request log is the oracle for
//!    **which variant actually fed each segment**, and the timeline
//!    stays continuous (absolute media sequence +1 across every seam,
//!    seams recorded as discontinuities).
//! 3. **No unbounded switching** — a link oscillating *inside* the
//!    hysteresis band produces zero further switches (the band + the
//!    dwell are the anti-thrash contract; the EWMA smooths, the
//!    margins hold).
//!
//! VOD variant playlists are static resources, so the testkit's
//! per-path `routes` carry them; the scripted surface for this card
//! is the bandwidth trace, not the playlist bodies.

use std::sync::Arc;

use plugin_api::SourceStream;
use streaming::fetch::HttpFetch;
use streaming::hls::master::{AbrConfig, HlsMasterConfig, open_master};
use streaming::mockhttp::{MockConfig, MockServer};
use testkit::Must;

/// The ladder under test (bits per second, descending rows).
const HI_BW: u64 = 2_000_000;
const MID_BW: u64 = 800_000;
const LO_BW: u64 = 200_000;
/// Media-sequence base shared by every variant (alignment identity).
const SEQ_BASE: u64 = 100;
/// Segments per variant playlist (long enough that the crash phase
/// of the anti-thrash test cannot drain the stream before its
/// oscillation phase runs).
const SEGMENTS: usize = 24;

fn master_text() -> Vec<u8> {
    format!(
        "#EXTM3U\n\
         #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"en\",DEFAULT=YES,URI=\"aud.m3u8\"\n\
         #EXT-X-STREAM-INF:BANDWIDTH={HI_BW},RESOLUTION=1280x720\n/hi.m3u8\n\
         #EXT-X-STREAM-INF:BANDWIDTH={MID_BW},RESOLUTION=640x360\n/mid.m3u8\n\
         #EXT-X-STREAM-INF:BANDWIDTH={LO_BW}\n/lo.m3u8\n"
    )
    .into_bytes()
}

/// A VOD variant playlist: `SEGMENTS` segments, sequence base 100.
fn variant_playlist(prefix: &str) -> Vec<u8> {
    let mut s = String::from("#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:2\n\
        #EXT-X-MEDIA-SEQUENCE:100\n#EXT-X-PLAYLIST-TYPE:VOD\n");
    for i in 0..SEGMENTS {
        s.push_str("#EXTINF:2.0,\n");
        s.push_str(&format!("/{prefix}/seg{i}.ts\n"));
    }
    s.push_str("#EXT-X-ENDLIST\n");
    s.into_bytes()
}

/// Serve the whole ladder: master + three variant playlists + segment
/// bodies (distinct sizes per variant so the wire oracle is legible).
fn serve_ladder() -> MockServer {
    let mut routes = vec![
        ("/master.m3u8".to_owned(), master_text()),
        ("/hi.m3u8".to_owned(), variant_playlist("hi")),
        ("/mid.m3u8".to_owned(), variant_playlist("mid")),
        ("/lo.m3u8".to_owned(), variant_playlist("lo")),
    ];
    for i in 0..SEGMENTS {
        routes.push((format!("/hi/seg{i}.ts"), vec![b'H'; 500]));
        routes.push((format!("/mid/seg{i}.ts"), vec![b'M'; 200]));
        routes.push((format!("/lo/seg{i}.ts"), vec![b'L'; 50]));
    }
    match MockServer::spawn(MockConfig {
        routes,
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    }
}

/// The variant prefix of each segment fetch, from the server's own
/// request log (the wire oracle).
fn segment_fetches(server: &MockServer) -> Vec<String> {
    server
        .requests()
        .iter()
        .filter(|r| r.path.contains("/seg"))
        .map(|r| {
            r.path
                .split('/')
                .nth(1)
                .unwrap_or("?")
                .to_owned()
        })
        .collect()
}

/// The default config plus the ladder-sized cap (so the test owns
/// the numbers, not the [PROPOSAL] default).
fn config() -> HlsMasterConfig {
    HlsMasterConfig {
        abr: AbrConfig {
            max_bandwidth_bps: HI_BW,
            ..AbrConfig::default()
        },
        ..HlsMasterConfig::default()
    }
}

/// 6.B2a over the wire: selection picks hi under the cap; the first
/// requests are master, hi playlist, then hi segments in order.
#[test]
fn master_selects_highest_variant_under_cap() {
    let server = serve_ladder();
    let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
    let mut stream =
        open_master(fetch, &server.url("/master.m3u8"), config()).must();
    assert_eq!(stream.abr_snapshot().current, 0, "hi selected");
    assert_eq!(stream.stats().current_bandwidth_bps, Some(HI_BW));

    let mut got = 0;
    while got < 1000 {
        let chunk = stream.read(500).must();
        got += chunk.len();
    }
    let logged = server.requests();
    let paths: Vec<&str> = logged.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths[0], "/master.m3u8", "master first");
    assert_eq!(paths[1], "/hi.m3u8", "selected variant's playlist");
    assert_eq!(paths[2], "/hi/seg0.ts", "hi segments in order");
    assert_eq!(paths[3], "/hi/seg1.ts");
    assert!(paths.iter().all(|p| !p.starts_with("/lo")), "no lo yet");
}

/// 6.B2b over the wire: a scripted crash steps hi → mid → lo at
/// segment boundaries (one rung per boundary), a recovery steps back
/// up, the request log names the variant that fed every segment, and
/// the timeline stays continuous across every seam.
#[test]
fn scripted_drop_and_rise_fetch_the_right_variants() {
    let server = serve_ladder();
    let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
    let mut stream =
        open_master(fetch, &server.url("/master.m3u8"), config()).must();

    // Healthy hi (2 Mbps, in-band), then the crash (0.1 Mbps).
    let mut all = Vec::new();
    all.extend_from_slice(&stream.read(500).must()); // dwell
    all.extend_from_slice(&stream.read(500).must());
    stream.note_throughput(500, 2_000_000);
    stream.note_throughput(5_000, 400_000_000);
    all.extend_from_slice(&stream.read(500).must());
    assert!(segment_fetches(&server).contains(&"mid".to_owned()), "down to mid");

    // Starve to the bottom rung.
    let mut guard = 0;
    while stream.abr_snapshot().current != 2 && guard < 6 {
        stream.note_throughput(5_000, 400_000_000);
        all.extend_from_slice(&stream.read(200).must());
        stream.note_throughput(5_000, 400_000_000);
        guard += 1;
    }
    assert_eq!(stream.abr_snapshot().current, 2, "starved to lo");
    let switches_at_bottom = stream.stats().variant_switches;

    // Recovery: 5 Mbps with the dwell served steps back up.
    for _ in 0..3 {
        stream.note_throughput(625_000, 1_000_000_000);
        all.extend_from_slice(&stream.read(60).must());
        stream.note_throughput(625_000, 1_000_000_000);
    }
    assert!(
        stream.abr_snapshot().current >= 1,
        "recovered off lo"
    );

    // Drain and audit the whole timeline.
    loop {
        let chunk = stream.read(128).must();
        if chunk.is_empty() {
            break;
        }
        all.extend_from_slice(&chunk);
    }
    let variants = segment_fetches(&server);
    assert_eq!(variants.len(), SEGMENTS, "every segment fetched once");
    assert_eq!(variants[0], "hi");
    assert!(variants.contains(&"lo".to_owned()), "bottom rung served");
    // The wire oracle: hi…, mid…, lo…, then a recovery rung — in
    // segment order, never interleaved mid-fetch.
    let mut runs: Vec<(String, usize)> = Vec::new();
    for v in &variants {
        match runs.last_mut() {
            Some((last, n)) if last == v => *n += 1,
            _ => runs.push((v.clone(), 1)),
        }
    }
    assert!(runs.len() >= 3, "at least hi/mid/lo runs: {runs:?}");
    assert!(runs[0].0 == "hi", "starts on the selected rung");
    // Timeline: absolute sequences +1 across every seam; seams flagged.
    let bounds = stream.segment_boundaries();
    assert_eq!(bounds.len(), SEGMENTS);
    for w in bounds.windows(2) {
        assert_eq!(w[1].media_sequence, w[0].media_sequence + 1);
    }
    assert_eq!(bounds[0].media_sequence, SEQ_BASE);
    assert!(
        bounds.iter().any(|b| b.discontinuity),
        "switch seams recorded"
    );
    assert_eq!(stream.size(), Some(all.len() as u64));
    let _ = switches_at_bottom;
}

/// The anti-thrash contract over the wire: a link oscillating inside
/// the hysteresis band (above the down margin, below the up margin)
/// produces **zero** further switches — the request log shows the
/// variant never changes again.
#[test]
fn in_band_oscillation_never_switches_again() {
    let server = serve_ladder();
    let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
    let mut stream =
        open_master(fetch, &server.url("/master.m3u8"), config()).must();

    // Crash to lo (as above: two starve samples per boundary so the
    // EWMA — whose memory is part of the anti-flap design — crosses
    // the 80% line within the guard), then oscillate inside lo's
    // band: the up margin to mid needs 1 Mbps; feed ~0.6-0.9 Mbps
    // alternations (above lo's down margin of 160 kbps, below mid's
    // up margin).
    let mut all = Vec::new();
    all.extend_from_slice(&stream.read(500).must());
    all.extend_from_slice(&stream.read(500).must());
    stream.note_throughput(500, 2_000_000);
    for _ in 0..6 {
        stream.note_throughput(5_000, 400_000_000); // 0.1 Mbps
        stream.note_throughput(5_000, 400_000_000);
        all.extend_from_slice(&stream.read(200).must());
    }
    assert_eq!(stream.abr_snapshot().current, 2, "on lo");
    let before = stream.stats().variant_switches;
    let fetches_before = segment_fetches(&server).len();

    // The oscillation: 12 alternations of 0.6 / 0.4 Mbps — both
    // inside the band (0.16 .. 1.0 Mbps for the lo→mid boundary).
    // Each read drains any retained remainder first, then pulls
    // fresh lo segments (50 B each).
    for k in 0..12 {
        let bps_bytes = if k % 2 == 0 { 75_000 } else { 50_000 };
        stream.note_throughput(bps_bytes, 1_000_000_000);
        all.extend_from_slice(&stream.read(200).must());
    }
    assert_eq!(
        stream.stats().variant_switches, before,
        "no further switches inside the band"
    );
    let after = segment_fetches(&server);
    assert!(
        after[fetches_before..].iter().all(|v| v == "lo"),
        "lo served throughout the oscillation: {:?}",
        &after[fetches_before..]
    );
    assert!(after.len() > fetches_before);
    let _ = all;
}
