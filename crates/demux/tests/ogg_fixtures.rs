//! The Ogg fixture acceptance suite (card 5.B1 AC): the committed
//! ffmpeg-generated valid fixtures demux **bit-exactly** against the
//! recorded ffmpeg raw-dump truth, a real `cat`-chained two-link
//! fixture asserts the chain contract, and the mutated/truncated
//! derivative families stay inside the typed-error-or-bounded-recovery
//! vocabulary (the `+FB` fuzz-lite contract) with their exact
//! per-derivative outcomes pinned.
//!
//! # Fixture provenance (recorded for review)
//!
//! The four valid fixtures under `tests/fixtures/generated/valid/`
//! are **ffmpeg products** (container bytes from ffmpeg 7.1.5 /
//! Lavf61.7.103; signal content is lavfi-generated sine tones — no
//! third-party media, CC0-equivalent test data):
//!
//! * `ogg_vorbis_8k_st.ogg` — `ffmpeg -f lavfi -i
//!   sine=frequency=440:sample_rate=8000:duration=0.4 -ac 2 -c:a
//!   libvorbis -q:a 2` (serial 1073372718);
//! * `ogg_vorbis_16k_mono.ogg` — the same shape at 16 kHz mono,
//!   660 Hz (serial 1983438);
//! * `ogg_opus_48k_mono.opus` — `-c:a libopus -b:a 24k` mono 48 kHz
//!   (serial 516290875);
//! * `ogg_chain_two_links.ogg` — **`cat ogg_vorbis_8k_st.ogg
//!   ogg_vorbis_16k_mono.ogg`**, byte-for-byte: two ffmpeg-generated
//!   parents with distinct serial numbers, the canonical RFC 3533
//!   chained stream.
//!
//! They are committed read-only; the xtask generator does not own
//! them (extra files are invisible to `cargo xtask fixtures --check`
//! by design — the same arrangement as the 5.A2 WAV/AIFF fixtures).
//! Total committed ogg fixture bytes ≈ 23 KiB (< 200 KiB mandate).
//!
//! # The recorded truth (never transcribed by hand)
//!
//! Every pinned value below was **measured this session** from the
//! committed bytes with the installed ffmpeg/ffprobe:
//!
//! * stream parameters via `ffprobe -show_entries stream=…`
//!   (codec_name / sample_rate / duration_ts) and packet counts via
//!   `ffprobe -count_packets -show_entries stream=nb_read_packets`;
//! * the payload digest via `ffmpeg -i <fixture> -map 0:a -c copy
//!   -f md5 -` — the MD5 of ffmpeg's demuxed packet stream (the raw
//!   dump; MD5 per the card's wording, RFC 1321 self-built in
//!   [`common`]).
//!
//! The differential claim: our demuxer's concatenated packet payloads
//! hash identically to ffmpeg's raw dump for the same file — and the
//! per-stream packet counts match ffprobe's `nb_read_packets`
//! exactly (both sides strip codec header packets: ffmpeg parses them
//! into extradata, we stash them in `header_packets()`).
//!
//! # The chain differential (honest disagreement, measured)
//!
//! On the chained fixture ffmpeg's own ogg demuxer reports **31**
//! packets: it strips the *first* link's Vorbis headers into
//! extradata but **leaks the second link's 3 header packets as data
//! packets** (its per-stream extradata is built once). We stash both
//! links' headers and deliver 14 + 14 = 28 packets, whose payload is
//! exactly the concatenation of the two standalone parents' payloads
//! (each independently ffmpeg-verified above) — asserted byte-for-
//! byte in [`ogg_chain_two_links_is_two_links_with_a_fresh_track`].
//! The discrepancy is ffmpeg's chain quirk, surfaced here rather
//! than smoothed over; the per-file table in the worklog records both
//! numbers.
//!
//! # Derivative families
//!
//! The `__trunc0..4`, `__flip1/4/16`, `__garbage0` derivatives were
//! generated from the vorbis parent by deterministic byte surgery
//! (verified this session: truncations are exact parent prefixes at
//! the recorded cut offsets; flips are single-bit XORs `0x01` at the
//! recorded sites; garbage0 appends 4096 `0xEE` bytes past the
//! container extent). Their pinned outcomes are the **measured**
//! demuxer behavior, asserted exactly (see the table constants).

mod common;

use common::{md5_hex, open_via_input};
use demux::ogg::OggDemuxer;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// Drain a started demuxer to its terminal, collecting the payload,
/// recovery reasons (walk order) and the terminal error code (`""` =
/// clean EOS).
fn drain_ogg(
    ogg: &mut OggDemuxer,
) -> (Vec<u8>, Vec<&'static str>, String, u64, u32, u32, u64, u64) {
    let mut payload = Vec::new();
    let mut reasons = Vec::new();
    let mut terminal = String::new();
    let mut packets = 0u64;
    let mut discontinuities = 0u32;
    let mut tracks = Vec::new();
    loop {
        match ogg.next_event() {
            Ok(DemuxEvent::Packet(p)) => {
                packets += 1;
                tracks.push(p.track.handle());
                payload.extend_from_slice(&p.data);
            }
            Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
            Ok(DemuxEvent::Discontinuity) => discontinuities += 1,
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                terminal = err.context.code.to_owned();
                break;
            }
        }
    }
    let _ = &tracks;
    (
        payload,
        reasons,
        terminal,
        packets,
        discontinuities,
        ogg.links() as u32,
        ogg.granule_discontinuities(),
        ogg.damage_total(),
    )
}

/// One valid fixture's ffmpeg-measured truth.
struct Truth {
    /// Fixture path under `tests/fixtures/`.
    fixture: &'static str,
    /// ffprobe `codec_name` — the registry descriptor our track must
    /// carry verbatim.
    codec: &'static str,
    /// ffprobe `sample_rate` (the clock's rate; Opus is always 48 kHz
    /// by RFC 7845).
    rate: u32,
    /// `ffprobe -count_packets nb_read_packets` (headers stripped by
    /// ffmpeg, exactly as we stash them).
    packets: u64,
    /// ffprobe `duration_ts` — the EOS granule in track ticks.
    granule: i64,
    /// `ffmpeg -map 0:a -c copy -f md5` over the demuxed packet
    /// stream.
    md5: &'static str,
}

/// The three standalone valid fixtures (ffmpeg products; truth
/// measured this session — see the module docs for the commands).
const TRUTHS: [Truth; 3] = [
    Truth {
        fixture: "generated/valid/ogg_vorbis_8k_st.ogg",
        codec: "vorbis",
        rate: 8_000,
        packets: 14,
        granule: 3_200,
        md5: "335fd5b66d943a78ed5b2fc1678029c8",
    },
    Truth {
        fixture: "generated/valid/ogg_vorbis_16k_mono.ogg",
        codec: "vorbis",
        rate: 16_000,
        packets: 14,
        granule: 6_400,
        md5: "73b12e77881f630d0d6a4a8ad1d14be1",
    },
    Truth {
        fixture: "generated/valid/ogg_opus_48k_mono.opus",
        codec: "opus",
        rate: 48_000,
        packets: 21,
        granule: 19_512,
        md5: "3e6f08d3445bc0faa0e77cc686475569",
    },
];

#[test]
fn every_ogg_fixture_matches_the_ffmpeg_truth() {
    for truth in &TRUTHS {
        let path = testkit::fixture_path(truth.fixture).must();
        let stream = open_via_input(&path).must();
        let mut ogg = match OggDemuxer::new(stream) {
            Ok(ogg) => ogg,
            Err(err) => unreachable!("{} opens: {err}", truth.fixture),
        };
        let tracks = match ogg.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("{} starts: {err}", truth.fixture),
        };
        assert_eq!(tracks.len(), 1, "{}", truth.fixture);
        let track = &tracks[0];
        assert_eq!(track.codec.as_str(), truth.codec, "{}", truth.fixture);
        assert_eq!(
            track.kind,
            media_types::MediaKind::Audio,
            "{}",
            truth.fixture
        );
        assert_eq!(
            track.timing.base.as_parts(),
            (1, truth.rate),
            "{}: one tick = one granule-domain sample at {} Hz",
            truth.fixture,
            truth.rate
        );
        // The EOS granule is the span end in track ticks.
        match track.timing.span.end {
            Stamp::Present(ts) => assert_eq!(ts.ticks, truth.granule, "{}", truth.fixture),
            Stamp::Missing => unreachable!("{}: span end present", truth.fixture),
        }
        // The duration follows from the clock (Opus 19_512 @ 48 kHz,
        // Vorbis at its rate — the granule/second contract).
        let duration = ogg
            .track_duration(track.id)
            .unwrap_or_else(|| unreachable!("{}: clock known", truth.fixture));
        let nanos = u64::try_from(
            u128::from(truth.granule.unsigned_abs()) * 1_000_000_000u128 / u128::from(truth.rate),
        )
        .must();
        assert_eq!(
            duration,
            media_types::time::Duration::Known(media_types::time::Nanoseconds(nanos)),
            "{}: duration = granule / rate",
            truth.fixture
        );
        // Header packets are stashed, not events (the ID/comment/setup
        // or OpusHead/OpusTags packets): ffprobe strips them the same
        // way, so `nb_read_packets` counts data packets only.
        let headers = ogg.header_packets(ogg.serial_numbers()[0]);
        assert!(!headers.is_empty(), "{}: headers stashed", truth.fixture);
        match truth.codec {
            "vorbis" => assert!(
                headers[0]
                    .as_ref()
                    .starts_with(&[0x01, b'v', b'o', b'r', b'b']),
                "{}",
                truth.fixture
            ),
            "opus" => assert!(
                headers[0].as_ref().starts_with(b"OpusHead"),
                "{}",
                truth.fixture
            ),
            _ => unreachable!("codec table is exhaustive"),
        }
        // Bit-exact payload parity with ffmpeg's raw dump.
        let (payload, reasons, terminal, packets, _, links, gdisc, damage) = drain_ogg(&mut ogg);
        assert_eq!(packets, truth.packets, "{}: packet count", truth.fixture);
        assert_eq!(
            md5_hex(&payload),
            truth.md5,
            "{}: payload parity vs ffmpeg raw dump",
            truth.fixture
        );
        // A clean ffmpeg product walks clean.
        assert!(
            terminal.is_empty(),
            "{}: clean EOS ({terminal})",
            truth.fixture
        );
        assert!(
            reasons.is_empty(),
            "{}: no damage ({reasons:?})",
            truth.fixture
        );
        assert_eq!(links, 1, "{}", truth.fixture);
        assert_eq!(gdisc, 0, "{}", truth.fixture);
        assert_eq!(damage, 0, "{}", truth.fixture);
        eprintln!(
            "fixture(ogg): PASS {} codec={} rate={} packets={} granule={} md5={}",
            truth.fixture, truth.codec, truth.rate, packets, truth.granule, truth.md5
        );
    }
}

/// The chain AC: `cat a.ogg b.ogg` (two ffmpeg-generated parents,
/// distinct serial numbers) demuxes as **two logical links** — the
/// boundary is a `Discontinuity`, link 2's packets ride a fresh track
/// id, and the granule restart is surfaced honestly (never smoothed
/// into a cross-link timeline).
#[test]
fn ogg_chain_two_links_is_two_links_with_a_fresh_track() {
    let path = testkit::fixture_path("generated/valid/ogg_chain_two_links.ogg").must();
    let stream = open_via_input(&path).must();
    let mut ogg = match OggDemuxer::new(stream) {
        Ok(ogg) => ogg,
        Err(err) => unreachable!("chain opens: {err}"),
    };
    let tracks = match ogg.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("chain starts: {err}"),
    };
    assert_eq!(tracks.len(), 1, "start reports link 0 only");
    assert_eq!(tracks[0].codec.as_str(), "vorbis");

    // Walk with order-aware bookkeeping: where does the boundary sit?
    let mut link0 = Vec::new();
    let mut link1 = Vec::new();
    let mut boundary_seen = false;
    let mut payload = Vec::new();
    let mut terminal = String::new();
    let mut boundary_at_packet = 0u64;
    loop {
        match ogg.next_event() {
            Ok(DemuxEvent::Packet(p)) => {
                payload.extend_from_slice(&p.data);
                if boundary_seen {
                    link1.push(p.track.handle());
                } else {
                    link0.push(p.track.handle());
                }
            }
            Ok(DemuxEvent::Discontinuity) if !boundary_seen => {
                boundary_seen = true;
                boundary_at_packet = u64::try_from(link0.len()).must();
            }
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::Recovered { reason, .. }) => {
                unreachable!("a cat of two clean files recovers nothing ({reason})")
            }
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                terminal = err.context.code.to_owned();
                break;
            }
        }
    }
    assert!(terminal.is_empty(), "chain walks clean ({terminal})");

    // Two logical links, one serial each, both parents accounted for.
    assert_eq!(ogg.links(), 2, "two logical links");
    assert_eq!(ogg.serial_numbers().len(), 2, "one serial per link");
    let counts = ogg.packet_counts();
    assert_eq!(counts.len(), 2);
    for (_, count) in &counts {
        assert_eq!(*count, 14, "each parent's 14 data packets");
    }
    assert_eq!(
        boundary_at_packet, 14,
        "the boundary lands after link 1's packets"
    );

    // The boundary is a Discontinuity and link 2 rides a fresh track.
    assert!(boundary_seen, "the chain boundary is a Discontinuity event");
    assert!(link0.iter().all(|h| *h == link0[0]), "link 1: one track");
    assert!(link1.iter().all(|h| *h == link1[0]), "link 2: one track");
    assert_ne!(link0[0], link1[0], "link 2's packets ride a fresh track id");
    assert_eq!(link0.len(), 14);
    assert_eq!(link1.len(), 14);
    assert_eq!(
        ogg.tracks().len(),
        2,
        "the accessor surfaces link 2's track"
    );

    // Granule honesty: the per-serial clocks restart at link 2's own
    // zero (6400 @ 16 kHz is link-2-local, NOT 3200+6400 cross-link
    // — no fabricated timeline); the restart surfaces as the boundary
    // Discontinuity + fresh track, never as a granule regression
    // *within* a serial (granule_discontinuities stays 0).
    assert_eq!(
        ogg.granule_discontinuities(),
        0,
        "no within-serial clock lie"
    );
    let track2 = &ogg.tracks()[1];
    match track2.timing.span.end {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 6_400, "link 2's own clock"),
        Stamp::Missing => unreachable!("link 2 clock known"),
    }
    let _ = ogg.track_duration(ogg.tracks()[0].id).must();

    // Payload = exactly the concatenation of the two standalone
    // parents' payloads (each independently ffmpeg-verified above):
    // our chain differential. ffmpeg's own ogg demuxer reports 31
    // packets here (it leaks link 2's 3 header packets as data — see
    // the module docs); we stash them, so 28.
    let parent_a = demuxed_payload("generated/valid/ogg_vorbis_8k_st.ogg");
    let parent_b = demuxed_payload("generated/valid/ogg_vorbis_16k_mono.ogg");
    let mut expected = parent_a;
    expected.extend_from_slice(&parent_b);
    assert_eq!(payload, expected, "chain payload == parent A || parent B");
    assert_eq!(payload.len(), 401 + 336, "the measured parent extents");
    assert_eq!(
        md5_hex(&payload),
        "f484292cc422cd6f630bbf2fbfe73e22",
        "the measured chain payload digest"
    );
    assert_eq!(
        ogg.damage_total(),
        0,
        "a cat of two clean files is not damage"
    );
}

/// Drain one standalone fixture to its full packet payload (helper
/// for the chain byte-parity check).
fn demuxed_payload(fixture: &str) -> Vec<u8> {
    let path = testkit::fixture_path(fixture).must();
    let stream = open_via_input(&path).must();
    let mut ogg = match OggDemuxer::new(stream) {
        Ok(ogg) => ogg,
        Err(err) => unreachable!("{fixture} opens: {err}"),
    };
    if let Err(err) = ogg.start() {
        unreachable!("{fixture} starts: {err}");
    }
    let mut payload = Vec::new();
    loop {
        match ogg.next_event() {
            Ok(DemuxEvent::Packet(p)) => payload.extend_from_slice(&p.data),
            Ok(DemuxEvent::EndOfStream) => return payload,
            Ok(_) => {}
            Err(err) => unreachable!("{fixture} drains clean: {err}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Derivative families: the `+FB` fuzz-lite pinning
// ---------------------------------------------------------------------------

/// One derivative's measured outcome (the pinned vocabulary: typed
/// terminal error, or clean EOS with a measured recovery reason /
/// damage footprint).
struct Derivative {
    /// Suffix under `generated/malformed/ogg_vorbis_8k_st__`.
    name: &'static str,
    /// Data packet bytes delivered before the terminal.
    bytes: u64,
    /// Data packets delivered before the terminal.
    packets: u64,
    /// Recovery reasons, in walk order.
    reasons: &'static [&'static str],
    /// Terminal: `""` = clean EOS, else the typed error code.
    terminal: &'static str,
    /// Whether `start` itself fails (terminal fires at start).
    fails_at_start: bool,
    /// Measured damage footprint (bytes).
    damage: u64,
    /// Measured granule discontinuities.
    gdisc: u64,
}

/// The ogg family (parent `ogg_vorbis_8k_st.ogg`: BOS(id 30 B) +
/// headers page (12 segs, 2706 B) + EOS page (14 audio packets,
/// 401 B, granule 3200) = 3245 B; derivatives verified as exact
/// parent prefixes / single-bit flips / +4096 `0xEE` garbage).
const DERIVATIVES: [Derivative; 9] = [
    // 12 bytes: the capture is there, the 27-byte page header is not
    // — the sync scan finds no whole page, no BOS ever lands.
    Derivative {
        name: "trunc0",
        bytes: 0,
        packets: 0,
        reasons: &[],
        terminal: "demux.ogg.no-streams",
        fails_at_start: true,
        damage: 0,
        gdisc: 0,
    },
    // Cut mid first-page body (40 of 58 B): the BOS page's declared
    // extent runs past the stream — typed truncation at start.
    Derivative {
        name: "trunc1",
        bytes: 0,
        packets: 0,
        reasons: &[],
        terminal: "demux.ogg.truncated-page",
        fails_at_start: true,
        damage: 0,
        gdisc: 0,
    },
    // Cut mid headers-page body (1500 of 2803 B): the header hunt
    // reads into the short page — typed truncation at start.
    Derivative {
        name: "trunc2",
        bytes: 0,
        packets: 0,
        reasons: &[],
        terminal: "demux.ogg.truncated-page",
        fails_at_start: true,
        damage: 0,
        gdisc: 0,
    },
    // Cut at the exact page boundary after the headers page (2803 B):
    // both header pages are whole, the audio page never arrives — a
    // clean end with zero packets (the stream honestly ends empty).
    Derivative {
        name: "trunc3",
        bytes: 0,
        packets: 0,
        reasons: &[],
        terminal: "",
        fails_at_start: false,
        damage: 0,
        gdisc: 0,
    },
    // 12 bytes short of the end: mid-EOS-page cut — typed truncation
    // at start (the header hunt reads ahead to the first data page).
    Derivative {
        name: "trunc4",
        bytes: 0,
        packets: 0,
        reasons: &[],
        terminal: "demux.ogg.truncated-page",
        fails_at_start: true,
        damage: 0,
        gdisc: 0,
    },
    // One flip at data-payload offset 68: the headers page's CRC now
    // lies — the page is dropped with a sync recovery at the EOS
    // page; all 14 audio packets survive **byte-identically** (they
    // lived on the dropped page's successor).
    Derivative {
        name: "flip1",
        bytes: 401,
        packets: 14,
        reasons: &["ogg: page CRC mismatch"],
        terminal: "",
        fails_at_start: false,
        damage: 2_745,
        gdisc: 1,
    },
    // One flip at EOS-page body offset 2810: the audio page's CRC
    // lies — dropped, its 14 packets are lost, the walk ends clean.
    Derivative {
        name: "flip4",
        bytes: 0,
        packets: 0,
        reasons: &["ogg: page CRC mismatch"],
        terminal: "",
        fails_at_start: false,
        damage: 442,
        gdisc: 0,
    },
    // One flip at BOS body offset 30: the ID header's CRC lies — the
    // BOS page is dropped, the stream opens headless: opaque
    // passthrough (no codec, no clock), 16 packets (the comment +
    // setup headers are now data), default base, no duration.
    Derivative {
        name: "flip16",
        bytes: 3_107,
        packets: 16,
        reasons: &["ogg: page CRC mismatch"],
        terminal: "",
        fails_at_start: false,
        damage: 58,
        gdisc: 0,
    },
    // 4096 garbage bytes past the EOS page: the walk scans, finds no
    // capture, ends — the parent's packets all survive, damage noted.
    Derivative {
        name: "garbage0",
        bytes: 401,
        packets: 14,
        reasons: &["ogg: lost capture"],
        terminal: "",
        fails_at_start: false,
        damage: 4_096,
        gdisc: 0,
    },
];

#[test]
fn ogg_derivatives_stay_inside_the_typed_vocabulary() {
    for d in &DERIVATIVES {
        let fixture = format!("generated/malformed/ogg_vorbis_8k_st__{}", d.name);
        let path = testkit::fixture_path(&fixture).must();
        let stream = open_via_input(&path).must();
        let mut ogg = match OggDemuxer::new(stream) {
            Ok(ogg) => ogg,
            Err(err) => unreachable!("{} constructs: {err}", d.name),
        };
        if d.fails_at_start {
            match ogg.start() {
                Err(err) => assert_eq!(err.context.code, d.terminal, "{}", d.name),
                Ok(_) => unreachable!("{} fails at start", d.name),
            }
            // The typed error is sticky by shape.
            match ogg.next_event() {
                Err(replay) => assert_eq!(replay.context.code, d.terminal, "{}", d.name),
                Ok(_) => unreachable!("{}: terminal is sticky", d.name),
            }
            eprintln!("fixture(ogg): PINNED {} start-err {}", d.name, d.terminal);
            continue;
        }
        let tracks = match ogg.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("{} starts: {err}", d.name),
        };
        let (payload, reasons, terminal, packets, _, _, gdisc, damage) = drain_ogg(&mut ogg);
        assert_eq!(payload.len() as u64, d.bytes, "{}: payload bytes", d.name);
        assert_eq!(packets, d.packets, "{}: packets", d.name);
        assert_eq!(reasons, d.reasons, "{}: recovery reasons", d.name);
        assert_eq!(terminal, d.terminal, "{}: terminal", d.name);
        assert_eq!(damage, d.damage, "{}: damage footprint", d.name);
        assert_eq!(gdisc, d.gdisc, "{}: granule discontinuities", d.name);
        let _ = &tracks;
        eprintln!(
            "fixture(ogg): PINNED {} packets={} bytes={} reasons={:?} terminal={:?}",
            d.name,
            packets,
            payload.len(),
            reasons,
            terminal
        );
    }
}

/// The `+FB` fuzz-lite contract over every committed ogg fixture and
/// derivative: absence of panic/hang/unbounded allocation (the
/// harness swallows the typed errors — the suites above pin them).
#[test]
fn ogg_fixtures_and_derivatives_survive_the_fuzz_harness() {
    let mut fixtures: Vec<String> = TRUTHS.iter().map(|t| t.fixture.to_string()).collect();
    fixtures.push("generated/valid/ogg_chain_two_links.ogg".to_string());
    for d in &DERIVATIVES {
        fixtures.push(format!("generated/malformed/ogg_vorbis_8k_st__{}", d.name));
    }
    for fixture in &fixtures {
        let bytes = testkit::fixture_bytes(fixture).must();
        demux::ogg::fuzz::fuzz_demux_ogg(&bytes);
    }
    // A second pass over the deterministic mutation families the
    // corpus harness style uses, applied to the chain fixture too:
    // every truncation prefix at page-ish strides and every 1024th
    // byte flipped — all through the fuzz contract.
    let chain = testkit::fixture_bytes("generated/valid/ogg_chain_two_links.ogg").must();
    for cut in [0usize, 1, 12, 40, 58, 1500, 2803, 3245, 3303, 6664, 7000] {
        let end = cut.min(chain.len());
        demux::ogg::fuzz::fuzz_demux_ogg(&chain[..end]);
    }
    for off in (0..chain.len()).step_by(1024) {
        let mut bytes = chain.clone();
        bytes[off] ^= 0x80;
        demux::ogg::fuzz::fuzz_demux_ogg(&bytes);
    }
    eprintln!(
        "fixture(ogg): fuzz-lite contract green over {} fixtures + sweeps",
        fixtures.len()
    );
}
