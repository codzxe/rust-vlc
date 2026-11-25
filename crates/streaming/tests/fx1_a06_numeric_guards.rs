//! FX1-A06 (Devil-Fix wave FX1, FIX-PLAN §2 F0.4g+h): regression
//! tests for the streaming crate's numeric-overflow guards — one
//! crafted input per audited panic site (wave 10):
//!
//! * DASH `SegmentTemplate@duration="0"` in a period with a known
//!   end — `fixed_segment_count`'s ceil-division divided by zero
//!   (W10-A08 F-001, executed at resolve.rs:735);
//! * DASH `SegmentTimeline` `S@duration="0"` with a negative `@r` —
//!   `expand_timeline`'s span ceil-division divided by zero
//!   (W10-A08 F-002, executed at resolve.rs:410);
//! * HLS adaptive master: one segment body larger than
//!   `max_retained_bytes` (a shape `HlsMasterConfig::validate`
//!   accepts) — `serve`'s retained-window offset subtraction
//!   wrapped and the retained slice then panicked (W10-A10 F-001);
//! * HLS `#EXT-X-TIMESTAMP-MAP` `LOCAL` with a year past the i64
//!   epoch-millisecond horizon — `parse_iso8601_utc`'s days→ms
//!   multiplication chain overflowed (W10-A06 F-001, executed in
//!   both profiles; netfuzz-reachable).
//!
//! Inputs are the audit's own crafted shapes — tiny synthetic MPD
//! text through the real playlists parser, and a 3-segment VOD
//! ladder over the mock HTTP testkit — so every guard is exercised
//! exactly the way the audit executed the defect it pins. Each test
//! also carries the non-overflowing control (positive durations, an
//! in-range year, the healthy prefix, the post-seek resume), so
//! the guards refuse only the arithmetic that actually overflows.

use std::sync::Arc;

use playlists::parse_mpd_bytes;
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};
use streaming::dash::{DashResolveError, RepresentationPlan, plan_representation};
use streaming::fetch::HttpFetch;
use streaming::hls::master::{HlsMasterConfig, open_master};
use streaming::hls::timestamp_map::TimestampMap;
use streaming::mockhttp::{MockConfig, MockServer};
use testkit::Must;

/// The crafted manifests resolve against a reserved example host
/// (test-double URL, never a shipped endpoint).
const MANIFEST_URL: &str = "http://ex.test/m.mpd"; // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint

/// Plan the first representation of the first adaptation set of the
/// first period of a crafted MPD text — the model shape the
/// resolve.rs unit tests drive with hand-built structs, here reached
/// through the real playlists parser (a crafted manifest is the
/// attacker's actual delivery shape).
fn plan_crafted_mpd(text: &str) -> Result<RepresentationPlan, DashResolveError> {
    let parsed = match parse_mpd_bytes(text.as_bytes()) {
        Ok(p) => p,
        Err(e) => unreachable!("crafted MPD must parse: {e}"),
    };
    let mpd = &parsed.mpd;
    let period = &mpd.periods[0];
    let set = &period.adaptation_sets[0];
    plan_representation(mpd, period, set, &set.representations[0], MANIFEST_URL)
}

/// A template-shaped manifest: one period of `PT10S` (a known end —
/// the precondition of the fixed-duration count), one
/// representation, one `SegmentTemplate` with `@duration` chosen by
/// the caller.
fn template_mpd(duration: &str) -> String {
    format!(
        "<MPD type=\"static\" mediaPresentationDuration=\"PT10S\">\
        <Period duration=\"PT10S\">\
        <AdaptationSet mimeType=\"video/mp4\">\
        <Representation id=\"v\" bandwidth=\"1000\">\
        <SegmentTemplate timescale=\"1\" duration=\"{duration}\" media=\"$Number$.m4s\"/>\
        </Representation></AdaptationSet></Period></MPD>"
    )
}

/// A timeline-shaped manifest: two `S` entries chosen by the caller.
fn timeline_mpd(s0: &str, s1: &str) -> String {
    format!(
        "<MPD type=\"static\">\
        <Period duration=\"PT10S\">\
        <AdaptationSet mimeType=\"video/mp4\">\
        <Representation id=\"v\" bandwidth=\"1000\">\
        <SegmentTemplate timescale=\"1\" media=\"$Time$.m4s\">\
        <SegmentTimeline>{s0}{s1}</SegmentTimeline>\
        </SegmentTemplate>\
        </Representation></AdaptationSet></Period></MPD>"
    )
}

/// W10-A08 F-001: `@duration="0"` inside a period with a known end —
/// `fixed_segment_count` divided `end / duration` by zero (the
/// parser accepts the zero; the resolver panicked). The guard
/// refuses typed — a zero @duration tiles no period finitely, the
/// segment count is unknowable — and the same manifest with a
/// positive @duration still plans, so only zero refuses.
#[test]
fn fx1_a06_dash_template_duration_zero_refuses_typed() {
    assert_eq!(
        plan_crafted_mpd(&template_mpd("0")),
        Err(DashResolveError::TimelineExtentUnknown)
    );
    // Control: positive @duration plans ceil(10 ticks / 3) = 4
    // segments of duration 3 — the normal division is unchanged.
    let plan = match plan_crafted_mpd(&template_mpd("3")) {
        Ok(p) => p,
        Err(e) => unreachable!("positive @duration must plan: {e}"),
    };
    assert_eq!(plan.timescale, 1);
    assert_eq!(plan.segments.len(), 4);
    assert!(plan.segments.iter().all(|s| s.duration == 3));
}

/// W10-A08 F-002: the audit's exact timeline vector —
/// `S(t=0,d=0,r=-1)` bounded by `S(t=50,d=10)`: the negative `@r`
/// resolves the span against the next `@t`, then divided
/// `span / entry.d` by zero. The guard refuses typed; the same
/// timeline with a positive `@d` plans the audit's pinned
/// arithmetic (span 35 @ d=10 → 4 repeats plus the bounding entry's
/// own segment).
#[test]
fn fx1_a06_dash_timeline_d_zero_negative_r_refuses_typed() {
    assert_eq!(
        plan_crafted_mpd(&timeline_mpd(
            "<S t=\"0\" d=\"0\" r=\"-1\"/>",
            "<S t=\"50\" d=\"10\"/>"
        )),
        Err(DashResolveError::TimelineExtentUnknown)
    );
    // Control: the resolve.rs unit test's vector, end to end through
    // the parser — 4 repeats of d=10 plus the t=35 entry = 5 slots.
    let plan = match plan_crafted_mpd(&timeline_mpd(
        "<S t=\"0\" d=\"10\" r=\"-1\"/>",
        "<S t=\"35\" d=\"10\"/>",
    )) {
        Ok(p) => p,
        Err(e) => unreachable!("positive @d must plan: {e}"),
    };
    assert_eq!(plan.segments.len(), 5);
    assert_eq!(plan.segments[3].time, 30);
    assert_eq!(plan.segments[4].time, 35);
}

/// W10-A10 F-001: one segment body larger than
/// `max_retained_bytes` — a config `validate` accepts (only the
/// zero limits and the 1 GiB ceiling are checked) — evicts the
/// retained window past the read cursor, and `serve` then
/// underflowed `pos - retained_from` (debug: "attempt to subtract
/// with overflow"; release: the wrapped index sliced an empty
/// window). The guard refuses the read typed; the healthy prefix
/// before the oversized body still serves, and a forward seek past
/// the evicted body resumes the stream (the guard refuses only the
/// evicted range, never wedges the session).
#[test]
fn fx1_a06_hls_master_oversized_segment_refuses_typed() {
    const RETAIN: usize = 32;
    const SMALL: usize = 24;
    const OVERSIZED: usize = 64;
    let master = "#EXTM3U\n\
        #EXT-X-STREAM-INF:BANDWIDTH=200000\n/media.m3u8\n";
    let media = "#EXTM3U\n\
        #EXT-X-TARGETDURATION:2\n\
        #EXT-X-MEDIA-SEQUENCE:100\n\
        #EXT-X-PLAYLIST-TYPE:VOD\n\
        #EXTINF:2.0,\n/seg0.ts\n\
        #EXTINF:2.0,\n/seg1.ts\n\
        #EXTINF:2.0,\n/seg2.ts\n\
        #EXT-X-ENDLIST\n";
    let routes = vec![
        ("/master.m3u8".to_owned(), master.as_bytes().to_vec()),
        ("/media.m3u8".to_owned(), media.as_bytes().to_vec()),
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
    let mut config = HlsMasterConfig::default();
    // The audit's config: retention smaller than one fetched body
    // (the response budget still admits the body — 8 MiB default).
    config.max_retained_bytes = RETAIN;
    let mut stream = open_master(fetch, &server.url("/master.m3u8"), config).must();

    // The healthy prefix: seg0 (24 B) fits the window and serves.
    assert_eq!(stream.read(SMALL).must(), vec![b'H'; SMALL]);

    // seg1 (64 B) over the budget: the window restarts at the body's
    // end while the cursor still sits at 24 — the read must refuse
    // typed instead of wrapping the window offset.
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

/// W10-A06 F-001: `LOCAL` with a 9-digit year — the days→
/// milliseconds multiplication chain overflowed i64 (executed:
/// "attempt to multiply with overflow" under overflow-checks, the
/// silently wrapped anchor -5336598346174303232 in release; the
/// in-tree netfuzz target feeds `from_playlist_text` fuzzed text).
/// The checked chain refuses the out-of-range value as `None` —
/// the parser's documented contract — while an in-range year (the
/// audit probe's own pick, well inside the chain's true horizon)
/// still parses with its exact millisecond image, so only the true
/// overflow refuses.
#[test]
fn fx1_a06_hls_timestamp_map_year_overflow_refuses_none() {
    // The audit's executed input.
    assert_eq!(
        TimestampMap::parse_tag_value("MPEGTS=0,LOCAL=999999999-01-01T00:00:00Z"),
        None
    );
    // The fuzz-reachable surface: the same year inside playlist
    // text.
    let playlist = "#EXTM3U\n\
        #EXT-X-TIMESTAMP-MAP:MPEGTS=0,LOCAL=999999999-01-01T00:00:00Z\n\
        #EXTINF:2.0,\nseg0.ts\n";
    assert_eq!(TimestampMap::from_playlist_text(playlist), None);
    // An in-range year — the audit probe's own pick, well inside the
    // chain's true horizon (year ≈ 292 278 994) — still parses, with
    // the probe's exact millisecond image: the guard is exact
    // arithmetic, never a blanket year cap.
    let edge = TimestampMap::parse_tag_value("MPEGTS=0,LOCAL=292277025-01-01T00:00:00Z").must();
    assert_eq!(edge.local_ms, 9_223_309_881_446_400_000);
    // A sane anchor is unchanged.
    let sane = TimestampMap::parse_tag_value("MPEGTS=900000,LOCAL=2000-02-29T12:34:56.789Z").must();
    assert_eq!(sane.local_ms, 951_827_696_000 + 789);
}
