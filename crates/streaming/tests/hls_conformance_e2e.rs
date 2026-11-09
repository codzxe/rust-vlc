//! The 6.B3 **conformance corpus** end-to-end: clean-room
//! synthesized vectors for the live-edge and discontinuity
//! behaviors the public specs (RFC 8216 + the Apple HLS authoring
//! spec / IETF bis draft tag grammar) document — every vector is a
//! fixture playlist over the REAL ffmpeg-generated TS segments of
//! `tests/fixtures/hls/ts_live/` (see `hls_live_e2e.rs` for the
//! generation evidence), served over a **real socket** by the mock
//! testkit, driven through the live session, and pinned to an
//! explicit oracle.
//!
//! # Clean-room provenance
//!
//! No proprietary vendor test suite is used or copied. Each vector
//! below is *synthesized from the documented behavior* (the spec
//! clause is named per vector) and its expected output derived by
//! hand from this repo's own fixtures — the same method as the
//! ffmpeg-oracle e2e lanes.
//!
//! # The corpus
//!
//! | Vector | Spec behavior | Test |
//! |---|---|---|
//! | L1 | Live-edge join when the window is shorter than `live_edge_back` | `live_edge_join_clamps_to_first_segment_of_a_short_window` |
//! | L2 | Window slid (`MEDIA-SEQUENCE` advances): nothing refetched, concatenation undisturbed | `window_slide_never_refetches_or_disturbs_the_stream` |
//! | L3 | Reload cadence at the exact target-duration boundary (§6.3.4) | `reload_cadence_at_the_exact_target_duration_boundary` |
//! | D1 | `EXT-X-DISCONTINUITY` seam with a CONTINUING timeline (single `X-TIMESTAMP-MAP`) | `discontinuity_seam_with_continuing_timestamps` |
//! | D2 | `EXT-X-DISCONTINUITY` seam where the encoder RESET the timeline (superseding `X-TIMESTAMP-MAP`) | `discontinuity_seam_with_encoder_timestamp_reset` |
//! | G1 | `#EXT-X-GAP` marker: unknown-tag tolerance (§4.1 ignore rule), no gap semantics yet | `gap_tag_is_tolerated_no_gap_semantics_yet` |

use std::sync::Arc;

use plugin_api::SourceStream;
use streaming::clock::FakeClock;
use streaming::fetch::HttpFetch;
use streaming::hls::live::{HlsLiveConfig, open_live};
use streaming::hls::timestamp_map::PTS_MODULUS;
use streaming::mockhttp::{MockClient, MockConfig, MockServer, ScriptedRoute};
use testkit::Must;

fn fixture(name: &str) -> Vec<u8> {
    testkit::fixture_bytes(&format!("hls/ts_live/{name}")).must()
}

/// The static segment routes (the real TS bodies from the 6.B1b
/// corpus: s-lane 44.1 kHz AAC, t-lane 22.05 kHz AAC).
fn segment_routes() -> Vec<(String, Vec<u8>)> {
    ["s00.ts", "s01.ts", "s02.ts", "t00.ts", "t01.ts"]
        .iter()
        .map(|name| (format!("/hls/ts_live/{name}"), fixture(name)))
        .collect()
}

/// A live source whose playlist route walks `bodies` (last repeats).
fn serve(bodies: Vec<Vec<u8>>) -> MockServer {
    match MockServer::spawn(MockConfig {
        routes: segment_routes(),
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

fn cat(names: &[&str]) -> Vec<u8> {
    let mut all = Vec::new();
    for name in names {
        all.extend_from_slice(&fixture(name));
    }
    all
}

/// **L1 — live-edge join, short window.** The join point is
/// `live_edge_back` segments behind the newest segment, clamped at
/// the first segment of the window (a window that has not grown yet
/// has nothing older to skip). Vector: 2 segments published,
/// `live_edge_back = 3` → playback starts at s00; the oracle is
/// `cat(s00, s01)`; the live-edge wallclock (X-TIMESTAMP-MAP) is
/// `LOCAL` + BOTH segments' EXTINF (the edge is the newest
/// segment's end, however far back the join clamped).
#[test]
fn live_edge_join_clamps_to_first_segment_of_a_short_window() {
    let body = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXT-X-TIMESTAMP-MAP:MPEGTS=900000,LOCAL=2020-01-01T00:00:00Z\n\
        #EXTINF:2.507756,\ns00.ts\n\
        #EXTINF:2.507756,\ns01.ts\n";
    let server = serve(vec![body.as_bytes().to_vec()]);
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    let config = HlsLiveConfig {
        live_edge_back: 3,
        max_stalled_reloads: 1,
        ..HlsLiveConfig::default()
    };
    let mut stream =
        match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), config) {
            Ok(s) => s,
            Err(e) => unreachable!("short-window live must open: {e}"),
        };
    // The join clamped at s00: both segments are in the plan.
    let oracle = cat(&["s00.ts", "s01.ts"]);
    let streamed = match read_exactly(&mut stream, oracle.len()) {
        Ok(b) => b,
        Err(e) => unreachable!("read the short window: {e}"),
    };
    assert_eq!(streamed, oracle, "join clamps at the first segment");
    // Live-edge wallclock: LOCAL + 2 × EXTINF (rounded ms).
    assert_eq!(
        stream.live_edge_wallclock_ms(),
        Some(1_577_836_800_000 + 5_016),
        "edge = end of newest segment = LOCAL + window EXTINF"
    );
}

/// **L2 — window slid.** A reload that advances `MEDIA-SEQUENCE`
/// is the same stream with old segments dropped: bytes already in
/// hand are NEVER refetched, the concatenation is undisturbed, and
/// the request count pins it (1 open + 1 reload + exactly the
/// played segments).
#[test]
fn window_slide_never_refetches_or_disturbs_the_stream() {
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
        #EXTINF:2.507767,\nt00.ts\n";
    let w3 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:1\n\
        #EXTINF:2.507756,\ns01.ts\n\
        #EXTINF:2.484533,\ns02.ts\n\
        #EXTINF:2.507767,\nt00.ts\n\
        #EXT-X-ENDLIST\n";
    let server = serve(
        [w1, w2, w3]
            .iter()
            .map(|w| w.as_bytes().to_vec())
            .collect(),
    );
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    let config = HlsLiveConfig {
        live_edge_back: 2,
        max_stalled_reloads: 2,
        ..HlsLiveConfig::default()
    };
    let mut stream = match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), config) {
        Ok(s) => s,
        Err(e) => unreachable!("live must open: {e}"),
    };
    let oracle = cat(&["s01.ts", "s02.ts", "t00.ts"]);
    let streamed = match read_exactly(&mut stream, oracle.len()) {
        Ok(b) => b,
        Err(e) => unreachable!("read across the slide: {e}"),
    };
    assert_eq!(streamed, oracle, "slide keeps the concatenation");
    // EOF after ENDLIST drain.
    assert!(stream.read(64).unwrap_or_default().is_empty(), "ended");
    // The pin: 1 opening fetch + 2 reloads + exactly 3 segment
    // fetches — s01/s02 were fetched once, before the slide.
    let stats = stream.stats();
    assert_eq!(stats.requests, 6, "open + 2 reloads + 3 segments only");
    assert_eq!(stats.segments_fetched, 3, "no refetch across the slide");
    assert_eq!(stats.segments_lost_window, 0, "nothing slid past us");
    assert_eq!(stats.playlist_reloads, 2);
}

/// **L3 — reload cadence at the exact target-duration boundary**
/// (RFC 8216 §6.3.4: a client reloads no sooner than one target
/// duration after a productive reload). Vector: the pacing clock is
/// advanced by EXACTLY `TARGETDURATION` before the dry read that
/// reloads — the boundary is "no sooner than", so an elapsed time
/// equal to the wait must NOT sleep; a second session reloaded
/// early DOES sleep. `FakeClock` makes both deterministic.
#[test]
fn reload_cadence_at_the_exact_target_duration_boundary() {
    let w1 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:2\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXTINF:1.0,\ns00.ts\n\
        #EXTINF:1.0,\ns01.ts\n";
    let w2 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:2\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXTINF:1.0,\ns00.ts\n\
        #EXTINF:1.0,\ns01.ts\n\
        #EXT-X-ENDLIST\n";
    let open_at_boundary = |advance: u64, probe: &str| {
        let server = serve(vec![
            w1.as_bytes().to_vec(),
            w2.as_bytes().to_vec(),
        ]);
        let clock = FakeClock::new();
        let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
        let config = HlsLiveConfig {
            live_edge_back: 1,
            max_stalled_reloads: 2,
            pacing_clock: Some(Arc::new(clock.clone())),
            ..HlsLiveConfig::default()
        };
        let mut stream =
            match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), config) {
                Ok(s) => s,
                Err(e) => unreachable!("{probe}: live must open: {e}"),
            };
        // Join back=1: fetch s01 (1 segment), then advance virtual
        // time to the boundary and demand the reload.
        let one = fixture("s01.ts").len();
        let played = match read_exactly(&mut stream, one) {
            Ok(b) => b,
            Err(e) => unreachable!("{probe}: drain the join: {e}"),
        };
        assert_eq!(played.len(), one);
        clock.advance_ms(advance);
        let oracle = cat(&["s01.ts", "s00.ts"]);
        let _ = oracle;
        let drained = match read_exactly(&mut stream, usize::MAX / 2) {
            Ok(b) => b,
            Err(e) => unreachable!("{probe}: read to the end: {e}"),
        };
        assert_eq!(drained, cat(&["s01.ts", "s00.ts"]), "{probe}: bytes");
        (stream.stats().paced_reloads, stream.stats().playlist_reloads)
    };
    // At the exact boundary (2 s elapsed, wait 2 s): no sleep.
    let (paced, reloads) = open_at_boundary(2_000, "boundary");
    assert_eq!(reloads, 1, "the ENDLIST window arrives via one reload");
    assert_eq!(paced, 0, "elapsed == wait: §6.3.4 is 'no sooner than'");
    // Early (only 1 s of 2 s elapsed): the reload sleeps the rest.
    let (paced_early, _) = open_at_boundary(1_000, "early");
    assert!(paced_early >= 1, "an early reload is paced out");
}

/// **D1 — discontinuity seam, CONTINUING timestamps.** An
/// `EXT-X-DISCONTINUITY` marker with ONE `X-TIMESTAMP-MAP` for the
/// whole playlist: the timeline is one continuous media clock
/// (format/encoder parameters change at the seam, timestamps do
/// not reset). The session must (a) record the seam on the t00
/// boundary, (b) keep the same PTS↔wallclock mapping across it —
/// the t-lane's media position under the map is the s-lane's
/// EXTINF total past the anchor.
#[test]
fn discontinuity_seam_with_continuing_timestamps() {
    let body = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXT-X-TIMESTAMP-MAP:MPEGTS=900000,LOCAL=2020-01-01T00:00:00Z\n\
        #EXTINF:2.507756,\ns00.ts\n\
        #EXTINF:2.507756,\ns01.ts\n\
        #EXT-X-DISCONTINUITY\n\
        #EXTINF:2.507767,\nt00.ts\n\
        #EXT-X-ENDLIST\n";
    let server = serve(vec![body.as_bytes().to_vec()]);
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    let config = HlsLiveConfig {
        live_edge_back: 3,
        max_stalled_reloads: 1,
        ..HlsLiveConfig::default()
    };
    let mut stream = match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), config) {
        Ok(s) => s,
        Err(e) => unreachable!("seam playlist must open: {e}"),
    };
    let oracle = cat(&["s00.ts", "s01.ts", "t00.ts"]);
    let streamed = match read_exactly(&mut stream, oracle.len()) {
        Ok(b) => b,
        Err(e) => unreachable!("read across the seam: {e}"),
    };
    assert_eq!(streamed, oracle, "byte-exact across the seam");
    // The seam is RECORDED on t00's boundary, not refused.
    assert_eq!(stream.stats().discontinuities, 1);
    let seam_offset = fixture("s00.ts").len() + fixture("s01.ts").len();
    let boundaries = stream.segment_boundaries();
    let seam = boundaries
        .iter()
        .find(|b| b.discontinuity)
        .unwrap_or_else(|| panic!("the t00 boundary carries the seam flag"));
    assert_eq!(seam.offset as usize, seam_offset, "seam sits at t00");
    // One map for the whole playlist: the mapping before/after the
    // seam is the SAME anchor — continuing timestamps.
    let map = stream.timestamp_map().expect("the playlist carries a map");
    let after_seam_pts = map.pts_for_wallclock_ms(map.wallclock_ms_for_pts(900_000 + 2_507_756 + 2_507_756));
    assert_eq!(after_seam_pts, 900_000 + 5_015_512);
    assert_eq!(
        stream.live_edge_wallclock_ms(),
        Some(1_577_836_800_000 + 7_523),
        "edge = anchor + all three EXTINF"
    );
}

/// **D2 — discontinuity seam, ENCODER TIMESTAMP RESET.** The
/// reload after the seam republishes `X-TIMESTAMP-MAP` with the
/// MPEG-2 counter reset (PTS back near zero) and `LOCAL` jumped
/// forward — the documented shape of an encoder restart mid-live.
/// The session must adopt the SUPERSEDING map for its live-edge
/// derivation while the byte concatenation stays undisturbed and
/// the seam is recorded.
#[test]
fn discontinuity_seam_with_encoder_timestamp_reset() {
    let w1 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXT-X-TIMESTAMP-MAP:MPEGTS=45000,LOCAL=2020-01-01T00:00:00Z\n\
        #EXTINF:2.507756,\ns00.ts\n\
        #EXTINF:2.507756,\ns01.ts\n";
    let w2 = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXT-X-TIMESTAMP-MAP:MPEGTS=45000,LOCAL=2020-01-01T00:00:00Z\n\
        #EXTINF:2.507756,\ns00.ts\n\
        #EXTINF:2.507756,\ns01.ts\n\
        #EXT-X-DISCONTINUITY\n\
        #EXT-X-TIMESTAMP-MAP:MPEGTS=8589844592,LOCAL=2020-01-01T01:00:00Z\n\
        #EXTINF:2.507767,\nt00.ts\n\
        #EXTINF:2.507767,\nt01.ts\n\
        #EXT-X-ENDLIST\n";
    let server = serve(
        [w1, w2].iter().map(|w| w.as_bytes().to_vec()).collect(),
    );
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    let config = HlsLiveConfig {
        live_edge_back: 2,
        max_stalled_reloads: 2,
        ..HlsLiveConfig::default()
    };
    let mut stream = match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), config) {
        Ok(s) => s,
        Err(e) => unreachable!("encoder-reset playlist must open: {e}"),
    };
    // Before the reload: the ORIGINAL anchor governs.
    assert_eq!(stream.timestamp_map().map(|m| m.local_ms), Some(1_577_836_800_000));
    let oracle = cat(&["s00.ts", "s01.ts", "t00.ts", "t01.ts"]);
    let streamed = match read_exactly(&mut stream, oracle.len()) {
        Ok(b) => b,
        Err(e) => unreachable!("read across the reset seam: {e}"),
    };
    assert_eq!(streamed, oracle, "bytes undisturbed by the reset");
    // After the reload: the SUPERSEDING map governs (PTS reset to
    // just before the 33-bit wrap, LOCAL jumped an hour) — the
    // last tag in the playlist wins, per the tag's
    // forward-application rule.
    let map = stream.timestamp_map().expect("the reloaded window carries a map");
    assert_eq!(map.mpegts, 8_589_844_592, "the reset PTS anchor (2^33 - 900000)");
    assert_eq!(map.local_ms, 1_577_840_400_000, "LOCAL jumped an hour");
    // The wrap-adjacent anchor still converts exactly (rollover
    // arithmetic): PTS 900000 (1 s past the wrap) is +10 s of media.
    assert_eq!(map.wallclock_ms_for_pts(900_000), 1_577_840_410_000);
    assert_eq!(map.wallclock_ms_for_pts(PTS_MODULUS - 900_000), 1_577_840_400_000);
    // The seam itself was recorded on t00's boundary.
    assert_eq!(stream.stats().discontinuities, 1);
    let seam_offset = fixture("s00.ts").len() + fixture("s01.ts").len();
    assert!(
        stream
            .segment_boundaries()
            .iter()
            .any(|b| b.discontinuity && b.offset as usize == seam_offset),
        "the seam is recorded at t00"
    );
}

/// **G1 — `#EXT-X-GAP`.** The shared grammar treats the tag as an
/// unknown tag (RFC 8216 §4.1: clients MUST ignore unrecognized
/// tags), so a gap-marked segment is fetched and played like any
/// other today — honest v0: NO gap-specific skip/fence exists yet
/// (recorded as pending in the HLS record). The vector pins the
/// CURRENT behavior so the record row cites a test that ran.
#[test]
fn gap_tag_is_tolerated_no_gap_semantics_yet() {
    let body = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:3\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXTINF:2.507756,\ns00.ts\n\
        #EXT-X-GAP\n\
        #EXTINF:2.507756,\ns01.ts\n\
        #EXT-X-ENDLIST\n";
    let server = serve(vec![body.as_bytes().to_vec()]);
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    let config = HlsLiveConfig {
        live_edge_back: 3,
        max_stalled_reloads: 1,
        ..HlsLiveConfig::default()
    };
    let mut stream = match open_live(fetch, &server.url("hls/ts_live/pl.m3u8"), config) {
        Ok(s) => s,
        Err(e) => unreachable!("gap-marked playlist must open: {e}"),
    };
    let oracle = cat(&["s00.ts", "s01.ts"]);
    let streamed = match read_exactly(&mut stream, oracle.len()) {
        Ok(b) => b,
        Err(e) => unreachable!("read the gap-marked window: {e}"),
    };
    assert_eq!(
        streamed, oracle,
        "v0: the gap-marked segment is played, not skipped"
    );
    assert_eq!(stream.stats().segments_fetched, 2);
}
