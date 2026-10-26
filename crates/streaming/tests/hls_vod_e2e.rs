//! The 6.B1a end-to-end proofs: real ffmpeg-generated HLS fixtures
//! served over a **real socket** by the mock HTTP testkit, opened
//! through the VOD session, and played through the existing demux +
//! decode machinery — the honesty oracle is ffmpeg itself, run on the
//! same content this session the fixtures were generated.
//!
//! # Fixtures (generated this session, ffmpeg 7.1.5)
//!
//! `tests/fixtures/hls/{ts_vod,adts_vod}/` — one 8 s 44.1 kHz mono
//! AAC-LC (64 kb/s) sine, segmented every 2.5 s by
//! `ffmpeg -f segment -segment_time 2.5 -segment_list pl.m3u8
//! -segment_list_type m3u8`, once into MPEG-TS segments (the classic
//! HLS shape) and once into ADTS segments (the raw-AAC shape).
//!
//! # The measured ffmpeg oracle (this session)
//!
//! * decoded PCM of the concatenated segments, both lanes:
//!   `ffmpeg -i cat -map 0:a:0 -f s16le -` → **708 608 bytes =
//!   354 304 samples/channel** (346 frames × 1024, mono);
//! * per-segment packet counts (ffprobe), both lanes:
//!   **108 + 108 + 107 + 23 = 346** AAC packets.
//!
//! # What each lane proves
//!
//! 1. **TS lane** — `open_vod` → `TsDemuxer` (the PSI head-scan +
//!    rewind exercises the session's retained-prefix seek) → AAC
//!    packets → `codecs::audio::aac::decode_stream` on the demuxed
//!    payload: the decode counts equal the ffmpeg oracle exactly.
//! 2. **ADTS lane** — the session's raw byte stream is byte-exact
//!    against the on-disk segment concatenation (the A2a parity
//!    shape), then demuxes through `AdtsDemuxer` and decodes through
//!    the same codec entry point to the same oracle numbers.
//! 3. Both request logs prove the wire shape: playlist first, then
//!    segments in playlist order, whole-body GETs (no `Range:`).

use std::sync::Arc;

use codecs::audio::aac::decode_stream;
use demux::adts::AdtsDemuxer;
use demux::ts::TsDemuxer;
use plugin_api::{DemuxEvent, Demuxer, SourceStream};
use streaming::fetch::HttpFetch;
use streaming::hls::media::{HlsVodConfig, open_vod};
use streaming::mockhttp::{MockClient, MockConfig, MockServer};
use testkit::Must;

/// The ffmpeg-measured oracle (this session; see the module docs).
const ORACLE_SAMPLES_PER_CHANNEL: usize = 354_304;
const ORACLE_AAC_PACKETS: usize = 346;

/// Segment names per lane (the playlist's own order).
const SEGMENTS: [&str; 4] = ["seg000", "seg001", "seg002", "seg003"];

fn lane_bytes(lane: &str, name: &str) -> Vec<u8> {
    testkit::fixture_bytes(&format!("hls/{lane}/{name}")).must()
}

/// Serve one lane over a real socket, returning the server.
fn serve_lane(lane: &str, ext: &str) -> MockServer {
    let mut routes = vec![(
        format!("/hls/{lane}/pl.m3u8"),
        lane_bytes(lane, "pl.m3u8"),
    )];
    for seg in SEGMENTS {
        routes.push((format!("/hls/{lane}/{seg}.{ext}"), lane_bytes(lane, &format!("{seg}.{ext}"))));
    }
    match MockServer::spawn(MockConfig {
        routes,
        ..MockConfig::default()
    }) {
        Ok(server) => server,
        Err(e) => unreachable!("spawn: {e}"),
    }
}

/// Drain a started demuxer, returning (audio packets, concatenated
/// packet payload).
fn drain_audio(demuxer: &mut dyn Demuxer) -> (usize, Vec<u8>) {
    let mut packets = 0usize;
    let mut payload = Vec::new();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                packets += 1;
                payload.extend_from_slice(&packet.data);
            }
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("fixture demuxes cleanly: {err}"),
        }
    }
    (packets, payload)
}

/// Walk ADTS frames in a byte stream (test-side truth: syncword
/// 0xFFF, 13-bit length at header bits 30..43; a wrong walk is a
/// broken test, not a broken player).
fn count_adts_frames(data: &[u8]) -> usize {
    let mut frames = 0usize;
    let mut at = 0usize;
    while at + 7 <= data.len() {
        let b = &data[at..at + 7];
        if b[0] == 0xFF && b[1] & 0xF0 == 0xF0 {
            let len = ((u32::from(b[3]) & 0x03) << 11)
                | (u32::from(b[4]) << 3)
                | (u32::from(b[5]) >> 5);
            if len as usize >= 7 && at + len as usize <= data.len() {
                frames += 1;
                at += len as usize;
                continue;
            }
        }
        at += 1; // resync (should not happen on these fixtures)
    }
    frames
}

/// Open a session on `url` through the mock client.
fn open_session(url: &str) -> streaming::hls::media::HlsVodStream {
    let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
    match open_vod(fetch, url, HlsVodConfig::default()) {
        Ok(stream) => stream,
        Err(e) => unreachable!("session must open: {e}"),
    }
}

/// The TS lane: playlist + MPEG-TS segments over a real socket →
/// TsDemuxer (seek-exercising PSI scan) → AAC decode — every count
/// equals the ffmpeg oracle.
#[test]
fn ts_lane_demuxes_and_decodes_to_the_ffmpeg_oracle() {
    let server = serve_lane("ts_vod", "ts");
    let session = open_session(&server.url("hls/ts_vod/pl.m3u8"));

    let mut demuxer = match TsDemuxer::new(Box::new(session)) {
        Ok(d) => d,
        Err(e) => unreachable!("ts open: {e}"),
    };
    let tracks = match demuxer.start() {
        Ok(t) => t,
        Err(e) => unreachable!("ts start (exercises the retained-prefix seek): {e}"),
    };
    assert_eq!(tracks.len(), 1, "one audio track");
    assert_eq!(tracks[0].codec.as_str(), "aac");
    let (packets, payload) = drain_audio(&mut demuxer);

    // Per-segment-boundary differential: our AAC packet count equals
    // ffmpeg's per-segment packet counts summed (108+108+107+23).
    assert_eq!(
        packets, ORACLE_AAC_PACKETS,
        "demuxed AAC packets == ffmpeg's per-segment sum"
    );
    // The demuxed payload is a well-formed ADTS stream of exactly the
    // oracle's frame count.
    assert_eq!(count_adts_frames(&payload), ORACLE_AAC_PACKETS);

    // Decode through the real codec stack: sample counts equal the
    // ffmpeg decode of the same content.
    let decoded = match decode_stream(&payload) {
        Ok(d) => d,
        Err(e) => unreachable!("aac decode: {e}"),
    };
    assert_eq!(decoded.frames, ORACLE_AAC_PACKETS);
    let channels = decoded.format.channels as usize;
    assert_eq!(channels, 1, "mono fixture");
    assert_eq!(
        decoded.samples.len() / channels,
        ORACLE_SAMPLES_PER_CHANNEL,
        "decoded samples/channel == ffmpeg's 708608 PCM bytes / 2"
    );

    // The wire shape: playlist first, then segments in order, all
    // whole-body GETs.
    let requests = server.requests();
    let paths: Vec<&str> = requests.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "/hls/ts_vod/pl.m3u8",
            "/hls/ts_vod/seg000.ts",
            "/hls/ts_vod/seg001.ts",
            "/hls/ts_vod/seg002.ts",
            "/hls/ts_vod/seg003.ts",
        ],
        "playlist first, segments in playlist order"
    );
    for req in &requests {
        assert!(
            req.header("range").is_none(),
            "whole-body segment GETs, no Range"
        );
    }
}

/// The ADTS lane: the session's raw stream is byte-exact against the
/// on-disk concatenation, then demuxes and decodes to the same
/// oracle.
#[test]
fn adts_lane_is_byte_exact_and_decodes_to_the_ffmpeg_oracle() {
    let server = serve_lane("adts_vod", "aac");
    let mut session = open_session(&server.url("hls/adts_vod/pl.m3u8"));

    // (1) Raw byte parity with the on-disk segment concatenation.
    let mut streamed = Vec::new();
    loop {
        let chunk = match session.read(8192) {
            Ok(c) => c,
            Err(e) => unreachable!("read: {e}"),
        };
        if chunk.is_empty() {
            break;
        }
        streamed.extend_from_slice(&chunk);
    }
    let mut on_disk = Vec::new();
    for seg in SEGMENTS {
        on_disk.extend_from_slice(&lane_bytes("adts_vod", &format!("{seg}.aac")));
    }
    assert_eq!(streamed, on_disk, "session stream == cat(segments)");
    assert_eq!(
        session.size(),
        Some(on_disk.len() as u64),
        "exact total once every segment is fetched"
    );

    // (2) Demux through the existing ADTS container host.
    let second = open_session(&server.url("hls/adts_vod/pl.m3u8"));
    let mut demuxer = match AdtsDemuxer::new(Box::new(second)) {
        Ok(d) => d,
        Err(e) => unreachable!("adts open: {e}"),
    };
    let tracks = match demuxer.start() {
        Ok(t) => t,
        Err(e) => unreachable!("adts start: {e}"),
    };
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].codec.as_str(), "aac");
    let (packets, payload) = drain_audio(&mut demuxer);
    assert_eq!(packets, ORACLE_AAC_PACKETS, "adts frames == oracle");
    assert_eq!(count_adts_frames(&payload), ORACLE_AAC_PACKETS);

    // (3) Decode the same bytes through the real codec stack.
    let decoded = match decode_stream(&payload) {
        Ok(d) => d,
        Err(e) => unreachable!("aac decode: {e}"),
    };
    let channels = decoded.format.channels as usize;
    assert_eq!(
        decoded.samples.len() / channels,
        ORACLE_SAMPLES_PER_CHANNEL,
        "decoded samples/channel == ffmpeg's oracle"
    );

    // The wire shape again: playlist + 4 ordered whole-body GETs.
    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[0].path, "/hls/adts_vod/pl.m3u8");
    for req in &requests[1..] {
        assert!(req.header("range").is_none());
    }
}
