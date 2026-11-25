//! FX2-A10 (Devil-Fix wave FX2 — the FX1-A06 risk-item follow-up):
//! regression tests for the retained-window usize-underflow twins of
//! the master session's fixed panic (FIX-PLAN F0.4h class, one sibling
//! file per test):
//!
//! * `hls/media.rs` (the VOD session, 6.B1a): one segment body larger
//!   than `max_retained_bytes` — a shape `HlsVodConfig::validate`
//!   accepts (only zero limits and the 1 GiB ceiling are checked) —
//!   evicts the retained window past the read cursor, and `serve`'s
//!   window-offset subtraction `pos - retained_from` underflowed (the
//!   wrapped index then failed the retained-slice bounds check) —
//!   W10-A09 F-001 [S2], the same executed shape as the master
//!   session's W10-A10 F-001.
//! * `hls/live.rs` (the live session, 6.B1b): the same `absorb`
//!   budget rule and the same `serve` retained-window shape — the
//!   twin the wave-10 summary flagged "likely replicated" and
//!   FX1-A06's risk note left to this wave (W10-A06's own pass on
//!   that file did not carry the finding).
//!
//! Both tests mirror `fx1_a06_hls_master_oversized_segment_refuses_typed`:
//! a 3-segment ladder over the mock HTTP testkit whose middle segment
//! body (64 B) exceeds the accepted retention budget (32 B); the read
//! at the evicted cursor must refuse typed
//! `streaming.hls.read-unretained` (the master fix's code) instead of
//! panicking, the cursor must stay unmoved, and a forward seek past
//! the evicted body must resume the stream — the guard refuses only
//! the evicted range, never wedges the session. Each test also pins
//! its file's own prefix contract (the VOD probe-fixed total; the
//! live pending queue serving the post-eviction segment without a
//! reload), so the guard demonstrably refuses only the evicted range.

use std::sync::Arc;

use plugin_api::{SeekOrigin, SourcePosition, SourceStream};
use streaming::fetch::HttpFetch;
use streaming::hls::live::{open_live, HlsLiveConfig};
use streaming::hls::media::{open_vod, HlsVodConfig};
use streaming::mockhttp::{MockConfig, MockServer};
use testkit::Must;

/// Retention budget the configs accept but one crafted body exceeds.
const RETAIN: usize = 32;
/// A body that fits the window and serves byte-exact (the control).
const SMALL: usize = 24;
/// The crafted oversized body — over the whole budget, the audit's
/// trigger shape.
const OVERSIZED: usize = 64;

/// W10-A09 F-001 (the media.rs twin of the master session's W10-A10
/// F-001): one segment body larger than `max_retained_bytes` — a
/// config `validate` accepts (only the zero limits and the 1 GiB
/// ceiling are checked) — evicts the retained window past the read
/// cursor, and `serve` underflowed `pos - retained_from` (debug:
/// "attempt to subtract with overflow"; release: the wrapped index
/// then failed the retained-slice bounds check). The guard refuses
/// the read typed; the healthy prefix before the oversized body still
/// serves, the probe-fixed total stays honest, and a forward seek
/// past the evicted body resumes the stream.
#[test]
fn fx2_a10_hls_vod_oversized_segment_refuses_typed() {
    let playlist = "#EXTM3U\n\
        #EXT-X-TARGETDURATION:2\n\
        #EXT-X-MEDIA-SEQUENCE:100\n\
        #EXT-X-PLAYLIST-TYPE:VOD\n\
        #EXTINF:2.0,\n/seg0.ts\n\
        #EXTINF:2.0,\n/seg1.ts\n\
        #EXTINF:2.0,\n/seg2.ts\n\
        #EXT-X-ENDLIST\n";
    let routes = vec![
        ("/media.m3u8".to_owned(), playlist.as_bytes().to_vec()),
        ("/seg0.ts".to_owned(), vec![b'H'; SMALL]),
        ("/seg1.ts".to_owned(), vec![b'X'; OVERSIZED]),
        ("/seg2.ts".to_owned(), vec![b'Y'; SMALL]),
    ];
    let server = match MockServer::spawn(MockConfig {
        routes,
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    };
    let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
    let mut config = HlsVodConfig::default();
    // The audit's config: retention smaller than one fetched body
    // (the response budget still admits the body — 8 MiB default).
    config.max_retained_bytes = RETAIN;
    let mut stream = open_vod(fetch, &server.url("/media.m3u8"), config).must();

    // The open-time probes fix the total (24 + 64 + 24) before the
    // first read — the known-length contract the guard must not
    // disturb.
    assert_eq!(stream.size(), Some((SMALL + OVERSIZED + SMALL) as u64));

    // The healthy prefix: seg0 (24 B) fits the window and serves.
    assert_eq!(stream.read(SMALL).must(), vec![b'H'; SMALL]);

    // seg1 (64 B) over the budget: the window restarts at the body's
    // end while the cursor still sits at 24 — the read must refuse
    // typed instead of wrapping the window offset (the pre-fix
    // panic site).
    let err = testkit::assert_err(stream.read(8), "oversized segment body refuses typed");
    assert_eq!(err.context.code, "streaming.hls.read-unretained");
    assert_eq!(stream.position(), SourcePosition(SMALL as u64));

    // Not wedged: a forward seek past the evicted body resumes on
    // seg2 — the guard refuses only the evicted range.
    stream
        .seek(SeekOrigin::Start((SMALL + OVERSIZED) as u64))
        .must();
    assert_eq!(stream.read(8).must(), vec![b'Y'; 8]);
}

/// The live.rs twin: the same `absorb` budget rule and `serve`
/// retained-window shape — W10-A06's own pass on that file left the
/// read-side underflow unnamed (the wave-10 summary flagged it
/// "likely replicated"; FX1-A06's master-fix risk note assigned it to
/// this wave). A live window (no `ENDLIST`) whose second segment body
/// is over the accepted retention budget evicts the window past the
/// read cursor; the read refuses typed, the cursor is unmoved, and a
/// forward seek past the evicted body resumes on the third segment —
/// the pending queue serves it without spending a reload.
#[test]
fn fx2_a10_hls_live_oversized_segment_refuses_typed() {
    let playlist = "#EXTM3U\n\
        #EXT-X-TARGETDURATION:2\n\
        #EXT-X-MEDIA-SEQUENCE:100\n\
        #EXTINF:2.0,\n/seg0.ts\n\
        #EXTINF:2.0,\n/seg1.ts\n\
        #EXTINF:2.0,\n/seg2.ts\n";
    let routes = vec![
        ("/pl.m3u8".to_owned(), playlist.as_bytes().to_vec()),
        ("/seg0.ts".to_owned(), vec![b'H'; SMALL]),
        ("/seg1.ts".to_owned(), vec![b'X'; OVERSIZED]),
        ("/seg2.ts".to_owned(), vec![b'Y'; SMALL]),
    ];
    let server = match MockServer::spawn(MockConfig {
        routes,
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    };
    let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
    let mut config = HlsLiveConfig::default();
    // The audit's config: retention smaller than one fetched body.
    // `live_edge_back` 3 over a 3-segment window joins at the oldest
    // segment, so the whole ladder is the fetched path.
    config.max_retained_bytes = RETAIN;
    config.live_edge_back = 3;
    let mut stream = open_live(fetch, &server.url("/pl.m3u8"), config).must();

    // The healthy prefix: seg0 (24 B) fits the window and serves.
    assert_eq!(stream.read(SMALL).must(), vec![b'H'; SMALL]);

    // seg1 (64 B) over the budget: the same eviction state as the VOD
    // twin — the read must refuse typed instead of wrapping the
    // window offset (the pre-fix panic site).
    let err = testkit::assert_err(stream.read(8), "oversized segment body refuses typed");
    assert_eq!(err.context.code, "streaming.hls.read-unretained");
    assert_eq!(stream.position(), SourcePosition(SMALL as u64));

    // Not wedged: a forward seek within fetched bytes past the
    // evicted body resumes on seg2 — the guard refuses only the
    // evicted range, and the pending queue serves it with no reload
    // spent.
    stream
        .seek(SeekOrigin::Start((SMALL + OVERSIZED) as u64))
        .must();
    assert_eq!(stream.read(8).must(), vec![b'Y'; 8]);
}
