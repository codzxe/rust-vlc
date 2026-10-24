//! The WAV/AIFF fixture acceptance suite (card 5.A2 AC): the committed
//! ffmpeg-generated valid fixtures demux **bit-exactly** against the
//! recorded ffmpeg raw-dump truth, and the mutated/truncated
//! derivative families stay inside the typed-error-or-bounded-recovery
//! vocabulary (the `+FB` fuzz-lite contract) with their exact
//! per-derivative outcomes pinned.
//!
//! # Fixture provenance (recorded for review)
//!
//! The ten valid fixtures under `tests/fixtures/generated/valid/` are
//! **ffmpeg products** (the container bytes ffmpeg 7.1 / Lavf61.7.103
//! writes; signal content is lavfi-generated tones — no third-party
//! media, CC0-equivalent test data). They were produced with the
//! ffmpeg CLI and are committed read-only; the xtask generator does
//! not own them (`cargo xtask fixtures --check` only drift-gates its
//! own AVI/MP4 outputs — extra files are invisible to it by design).
//!
//! # The recorded truth (never transcribed by hand)
//!
//! Every pinned value below was **measured this session** from the
//! committed bytes with the installed ffmpeg/ffprobe:
//!
//! * stream parameters via `ffprobe -show_entries stream=…`
//!   (codec_name / sample_rate / channels / duration_ts);
//! * the payload digest via `ffmpeg -i <fixture> -map 0:a -c copy
//!   -f md5 -` — the MD5 of ffmpeg's demuxed packet stream (the raw
//!   dump; MD5 per the card's wording, RFC 1321 self-built in
//!   [`common`], pinned against RFC 1321's own A.5 vectors below).
//!
//! The differential claim: our demuxer's concatenated packet payloads
//! hash identically to ffmpeg's raw dump for the same file — the
//! byte-for-byte payload parity the user's differential mandate asks
//! for, on real container bytes.
//!
//! # Derivative families
//!
//! The `__trunc0..4`, `__flip1/4/16`, `__garbage0` derivatives were
//! generated from the two PCM parents by deterministic byte surgery
//! (verified this session: truncations are exact parent prefixes at
//! the recorded cut offsets; flips are seeded single-bit XORs at the
//! recorded sites; garbage0 appends 4096 bytes past the container
//! extent). Their pinned outcomes are the measured demuxer behavior,
//! asserted exactly (see the table constants).

mod common;

use std::sync::Arc;

use common::{md5_hex, open_via_input};
use demux::aiff::AiffDemuxer;
use demux::wav::WavDemuxer;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One valid fixture's ffmpeg-measured truth.
struct Truth {
    /// Fixture path under `tests/fixtures/`.
    fixture: &'static str,
    /// ffprobe `codec_name` — the registry descriptor our track must
    /// carry verbatim.
    codec: &'static str,
    /// ffprobe `sample_rate`.
    rate: u32,
    /// ffprobe `channels`.
    channels: u16,
    /// ffprobe `duration_ts` (sample frames for PCM streams; the
    /// `fact`/COMM declaration for FLAC-in-WAV).
    frames: u64,
    /// `ffmpeg -c copy -f md5` over the demuxed packet stream.
    md5: &'static str,
}

/// The ten valid fixtures (ffmpeg products; truth measured this
/// session — see the module docs for the exact commands).
const TRUTHS: [Truth; 10] = [
    Truth {
        fixture: "generated/valid/wav_pcm_u8_44k1_mono.wav",
        codec: "pcm_u8",
        rate: 44_100,
        channels: 1,
        frames: 8_820,
        md5: "d5c54db7b4ade5332cfd96a9566a4ef4",
    },
    Truth {
        fixture: "generated/valid/wav_pcm_s16le_44k1_st.wav",
        codec: "pcm_s16le",
        rate: 44_100,
        channels: 2,
        frames: 8_820,
        md5: "dcaccb18f7fc858b87684ad3b25bc7c5",
    },
    Truth {
        fixture: "generated/valid/wav_pcm_s24le_48k_st.wav",
        codec: "pcm_s24le",
        rate: 48_000,
        channels: 2,
        frames: 9_600,
        md5: "0b8151a634c335f4b80e00d91a23b1b5",
    },
    Truth {
        fixture: "generated/valid/wav_pcm_f32le_48k_st.wav",
        codec: "pcm_f32le",
        rate: 48_000,
        channels: 2,
        frames: 9_600,
        md5: "898265e816609c24e606e535a121f646",
    },
    Truth {
        fixture: "generated/valid/wav_extensible_6ch_48k.wav",
        codec: "pcm_s16le",
        rate: 48_000,
        channels: 6,
        frames: 9_600,
        md5: "8caacacc6f48a620a28bdd0e4e65d5b4",
    },
    Truth {
        fixture: "generated/valid/wav_rf64_44k1_st.wav",
        codec: "pcm_s16le",
        rate: 44_100,
        channels: 2,
        frames: 8_820,
        md5: "5d105ceb212ad5d36e4f5608def3d8b7",
    },
    Truth {
        fixture: "generated/valid/aiff_pcm_s16be_44k1_st.aiff",
        codec: "pcm_s16be",
        rate: 44_100,
        channels: 2,
        frames: 8_820,
        md5: "a86269962ef80c06a1371573815cdb23",
    },
    Truth {
        fixture: "generated/valid/aiff_pcm_s24be_48k_st.aiff",
        codec: "pcm_s24be",
        rate: 48_000,
        channels: 2,
        frames: 9_600,
        md5: "9690c7b28041aaffa3feae60d1b1f88e",
    },
    Truth {
        fixture: "generated/valid/aifc_sowt_s16le_44k1_st.aifc",
        codec: "pcm_s16le",
        rate: 44_100,
        channels: 2,
        frames: 8_820,
        md5: "5d105ceb212ad5d36e4f5608def3d8b7",
    },
    Truth {
        fixture: "generated/valid/aifc_raw_u8_44k1_mono.aifc",
        codec: "pcm_u8",
        rate: 44_100,
        channels: 1,
        frames: 8_820,
        md5: "d5c54db7b4ade5332cfd96a9566a4ef4",
    },
];

/// RFC 1321's own A.5 test vectors — the pin before any differential
/// assertion relies on the test-side MD5.
#[test]
fn md5_impl_matches_rfc1321_vectors() {
    assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        md5_hex(b"abcdefghijklmnopqrstuvwxyz"),
        "c3fcd3d76192e4007dfb496cca67e13b"
    );
}

/// Drain a started WAV, returning (payload, damage-reason list,
/// terminal code) — `""` for a clean EOS.
fn drain_wav(wav: &mut WavDemuxer) -> (Vec<u8>, Vec<&'static str>, String) {
    let mut payload = Vec::new();
    let mut reasons = Vec::new();
    let mut terminal = String::new();
    loop {
        match wav.next_event() {
            Ok(DemuxEvent::Packet(packet)) => payload.extend_from_slice(&packet.data),
            Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                terminal = err.context.code.to_owned();
                break;
            }
        }
    }
    (payload, reasons, terminal)
}

/// The AIFF twin of [`drain_wav`].
fn drain_aiff(aiff: &mut AiffDemuxer) -> (Vec<u8>, Vec<&'static str>, String) {
    let mut payload = Vec::new();
    let mut reasons = Vec::new();
    let mut terminal = String::new();
    loop {
        match aiff.next_event() {
            Ok(DemuxEvent::Packet(packet)) => payload.extend_from_slice(&packet.data),
            Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                terminal = err.context.code.to_owned();
                break;
            }
        }
    }
    (payload, reasons, terminal)
}

// ---------------------------------------------------------------------------
// Valid fixtures: parameters + bit-exact payload parity vs ffmpeg
// ---------------------------------------------------------------------------

#[test]
fn every_wav_fixture_matches_the_ffmpeg_truth() {
    for truth in TRUTHS.iter().filter(|t| t.fixture.contains("wav_")) {
        let path = testkit::fixture_path(truth.fixture).must();
        let stream = open_via_input(&path).must();
        let mut wav = match WavDemuxer::new(stream) {
            Ok(wav) => wav,
            Err(err) => unreachable!("{} opens: {err}", truth.fixture),
        };
        let tracks = match wav.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("{} starts: {err}", truth.fixture),
        };
        assert_eq!(tracks.len(), 1, "{}", truth.fixture);
        let track = &tracks[0];
        assert_eq!(track.codec.as_str(), truth.codec, "{}", truth.fixture);
        assert_eq!(
            track.timing.base.as_parts(),
            (1, truth.rate),
            "{}: one tick = one sample frame at {} Hz",
            truth.fixture,
            truth.rate
        );
        // The sample-frame count: the span end in track ticks.
        match track.timing.span.end {
            Stamp::Present(ts) => assert_eq!(ts.ticks as u64, truth.frames, "{}", truth.fixture),
            Stamp::Missing => unreachable!("{}: span end present", truth.fixture),
        }
        // The demuxer-side count agrees (None only for the
        // frame-unknown FLAC payload — not among these fixtures).
        assert_eq!(wav.frames(), Some(truth.frames), "{}", truth.fixture);
        // Bit-exact payload parity with ffmpeg's raw dump.
        let (payload, reasons, terminal) = drain_wav(&mut wav);
        assert_eq!(
            md5_hex(&payload),
            truth.md5,
            "{}: payload parity",
            truth.fixture
        );
        assert_eq!(
            payload.len() as u64,
            truth.frames
                * u64::from(truth.channels)
                * (match truth.codec {
                    "pcm_u8" => 1u64,
                    "pcm_s16le" => 2,
                    "pcm_s24le" => 3,
                    "pcm_f32le" => 4,
                    other => unreachable!("{}: width of {other}", truth.fixture),
                }),
            "{}: byte total",
            truth.fixture
        );
        assert!(
            reasons.is_empty(),
            "{}: no damage on a valid file ({reasons:?})",
            truth.fixture
        );
        assert!(
            terminal.is_empty(),
            "{}: clean EOS ({terminal})",
            truth.fixture
        );
        assert_eq!(wav.damage_total(), 0, "{}", truth.fixture);
    }
}

#[test]
fn every_aiff_fixture_matches_the_ffmpeg_truth() {
    for truth in TRUTHS.iter().filter(|t| !t.fixture.contains("wav_")) {
        let path = testkit::fixture_path(truth.fixture).must();
        let stream = open_via_input(&path).must();
        let mut aiff = match AiffDemuxer::new(stream) {
            Ok(aiff) => aiff,
            Err(err) => unreachable!("{} opens: {err}", truth.fixture),
        };
        let tracks = match aiff.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("{} starts: {err}", truth.fixture),
        };
        assert_eq!(tracks.len(), 1, "{}", truth.fixture);
        let track = &tracks[0];
        assert_eq!(track.codec.as_str(), truth.codec, "{}", truth.fixture);
        assert_eq!(
            track.timing.base.as_parts(),
            (1, truth.rate),
            "{}: one tick = one sample frame at {} Hz",
            truth.fixture,
            truth.rate
        );
        match track.timing.span.end {
            Stamp::Present(ts) => assert_eq!(ts.ticks as u64, truth.frames, "{}", truth.fixture),
            Stamp::Missing => unreachable!("{}: span end present", truth.fixture),
        }
        assert_eq!(aiff.frames(), Some(truth.frames), "{}", truth.fixture);
        let (payload, reasons, terminal) = drain_aiff(&mut aiff);
        assert_eq!(
            md5_hex(&payload),
            truth.md5,
            "{}: payload parity",
            truth.fixture
        );
        assert!(
            reasons.is_empty(),
            "{}: no damage ({reasons:?})",
            truth.fixture
        );
        assert!(
            terminal.is_empty(),
            "{}: clean EOS ({terminal})",
            truth.fixture
        );
        assert_eq!(aiff.damage_total(), 0, "{}", truth.fixture);
    }
}

/// The RF64 fixture really is RF64 (the magic + ds64 path, not a plain
/// RIFF read of the same bytes).
#[test]
fn rf64_fixture_carries_the_ds64_truth() {
    let path = testkit::fixture_path("generated/valid/wav_rf64_44k1_st.wav").must();
    let stream = open_via_input(&path).must();
    let mut wav = match WavDemuxer::new(stream) {
        Ok(wav) => wav,
        Err(err) => unreachable!("opens: {err}"),
    };
    match wav.start() {
        Ok(_) => assert!(wav.is_rf64_like(), "the RF64 magic must be observed"),
        Err(err) => unreachable!("starts: {err}"),
    }
}

// ---------------------------------------------------------------------------
// Mutated families: exact pinned outcomes (the +FB contract)
// ---------------------------------------------------------------------------

/// A WAV derivative's pinned outcome (measured this session).
struct WavDerivative {
    name: &'static str,
    /// Packet payload bytes delivered before the terminal.
    bytes: u64,
    /// Recovery reasons, in walk order.
    reasons: &'static [&'static str],
    /// Terminal: `""` = clean EOS, else the typed error code.
    terminal: &'static str,
    /// Whether `start` itself fails (terminal fires at start).
    fails_at_start: bool,
}

/// The WAV family (parent `wav_pcm_s16le_44k1_st.wav`: fmt 16 + LIST
/// INFO + data 35280 B = 8820 frames; derivatives verified as exact
/// parent prefixes / seeded flips / +4096 garbage).
const WAV_DERIVATIVES: [WavDerivative; 9] = [
    // 12-byte head only: the RIFF declared 35358; 12 arrived.
    WavDerivative {
        name: "trunc0",
        bytes: 0,
        reasons: &[],
        terminal: "demux.wav.truncated",
        fails_at_start: true,
    },
    // Cut mid-`fmt ` (24 of the 36 bytes the chunk needs).
    WavDerivative {
        name: "trunc1",
        bytes: 0,
        reasons: &[],
        terminal: "demux.wav.truncated",
        fails_at_start: true,
    },
    // Cut inside the LIST payload: fmt parsed, data never reached —
    // the physically-short flavor wins over no-data.
    WavDerivative {
        name: "trunc2",
        bytes: 0,
        reasons: &[],
        terminal: "demux.wav.truncated",
        fails_at_start: true,
    },
    // Mid-data cut at 17679: 17600 whole frames deliver, the 1-byte
    // sliver is damage-counted, then the typed truncation.
    WavDerivative {
        name: "trunc3",
        bytes: 17_600,
        reasons: &[
            "riff-extends-past-stream",
            "data-extends-past-stream",
            "data-not-frame-aligned",
        ],
        terminal: "demux.wav.truncated",
        fails_at_start: false,
    },
    // 12 bytes short of the end: 35268 whole frames deliver.
    WavDerivative {
        name: "trunc4",
        bytes: 35_268,
        reasons: &["riff-extends-past-stream", "data-extends-past-stream"],
        terminal: "demux.wav.truncated",
        fails_at_start: false,
    },
    // 4096 garbage bytes past the RIFF extent: invisible.
    WavDerivative {
        name: "garbage0",
        bytes: 35_280,
        reasons: &[],
        terminal: "",
        fails_at_start: false,
    },
    // One flip, at data payload offset 68: payload content changes,
    // the container is whole — full demux, zero damage.
    WavDerivative {
        name: "flip1",
        bytes: 35_280,
        reasons: &[],
        terminal: "",
        fails_at_start: false,
    },
    // Flips hit the fmt tag (byte 20: tag 1 → 0x41): typed refusal.
    WavDerivative {
        name: "flip4",
        bytes: 0,
        reasons: &[],
        terminal: "demux.wav.unsupported-format",
        fails_at_start: true,
    },
    // Sixteen flips, fmt tag among them: typed refusal.
    WavDerivative {
        name: "flip16",
        bytes: 0,
        reasons: &[],
        terminal: "demux.wav.unsupported-format",
        fails_at_start: true,
    },
];

#[test]
fn wav_mutated_family_outcomes_are_pinned_exactly() {
    for deriv in WAV_DERIVATIVES {
        let rel = format!("generated/malformed/wav_pcm_s16le_44k1_st__{}", deriv.name);
        let path = testkit::fixture_path(&rel).must();
        let stream = open_via_input(&path).must();
        let mut wav = match WavDemuxer::new(stream) {
            Ok(wav) => wav,
            Err(err) => unreachable!("{} opens through the access stack: {err}", deriv.name),
        };
        match wav.start() {
            Ok(tracks) => {
                assert!(
                    !deriv.fails_at_start,
                    "{}: pinned as a start failure",
                    deriv.name
                );
                assert_eq!(tracks.len(), 1, "{}", deriv.name);
            }
            Err(err) => {
                assert!(
                    deriv.fails_at_start,
                    "{}: expected a started walk, got {err}",
                    deriv.name
                );
                assert_eq!(err.context.code, deriv.terminal, "{}", deriv.name);
                // Sticky replay: the same shape, not a new failure.
                match wav.start() {
                    Ok(_) => unreachable!("{}: sticky", deriv.name),
                    Err(replay) => {
                        assert_eq!(replay.context.code, deriv.terminal, "{}", deriv.name)
                    }
                }
                continue;
            }
        }
        let (payload, reasons, terminal) = drain_wav(&mut wav);
        assert_eq!(
            payload.len() as u64,
            deriv.bytes,
            "{}: delivered bytes",
            deriv.name
        );
        assert_eq!(reasons, deriv.reasons, "{}: recovery reasons", deriv.name);
        assert_eq!(terminal, deriv.terminal, "{}: terminal", deriv.name);
    }
}

/// The AIFF family (parent `aiff_pcm_s16be_44k1_st.aiff`: COMM 18 +
/// SSND 8 + 35280 B sound = 8820 frames).
struct AiffDerivative {
    name: &'static str,
    bytes: u64,
    reasons: &'static [&'static str],
    terminal: &'static str,
    fails_at_start: bool,
}

const AIFF_DERIVATIVES: [AiffDerivative; 9] = [
    AiffDerivative {
        name: "trunc0",
        bytes: 0,
        reasons: &[],
        terminal: "demux.aiff.truncated",
        fails_at_start: true,
    },
    // Cut mid-COMM (24 of the 38 bytes the chunk needs).
    AiffDerivative {
        name: "trunc1",
        bytes: 0,
        reasons: &[],
        terminal: "demux.aiff.truncated",
        fails_at_start: true,
    },
    // 10 bytes of sound: 8 deliver (4 frames), the 2-byte sliver is
    // damage-counted, then the typed truncation.
    AiffDerivative {
        name: "trunc2",
        bytes: 8,
        reasons: &[
            "form-extends-past-stream",
            "ssnd-extends-past-stream",
            "comm-frames-past-ssnd",
            "ssnd-not-frame-aligned",
        ],
        terminal: "demux.aiff.truncated",
        fails_at_start: false,
    },
    // Mid-sound cut at 17667: 17612 whole frames deliver.
    AiffDerivative {
        name: "trunc3",
        bytes: 17_612,
        reasons: &[
            "form-extends-past-stream",
            "ssnd-extends-past-stream",
            "comm-frames-past-ssnd",
            "ssnd-not-frame-aligned",
        ],
        terminal: "demux.aiff.truncated",
        fails_at_start: false,
    },
    // 12 bytes short: 35268 whole frames deliver, no sliver.
    AiffDerivative {
        name: "trunc4",
        bytes: 35_268,
        reasons: &[
            "form-extends-past-stream",
            "ssnd-extends-past-stream",
            "comm-frames-past-ssnd",
        ],
        terminal: "demux.aiff.truncated",
        fails_at_start: false,
    },
    AiffDerivative {
        name: "garbage0",
        bytes: 35_280,
        reasons: &[],
        terminal: "",
        fails_at_start: false,
    },
    // One flip at byte 18 (COMM size 0x12 → 0x112): the walk jumps
    // past the real SSND — no-ssnd.
    AiffDerivative {
        name: "flip1",
        bytes: 0,
        reasons: &[],
        terminal: "demux.aiff.no-ssnd",
        fails_at_start: true,
    },
    // Flips hit COMM channels (byte 20: 2 → 0x4002): bad-comm.
    AiffDerivative {
        name: "flip4",
        bytes: 0,
        reasons: &[],
        terminal: "demux.aiff.bad-comm",
        fails_at_start: true,
    },
    AiffDerivative {
        name: "flip16",
        bytes: 0,
        reasons: &[],
        terminal: "demux.aiff.bad-comm",
        fails_at_start: true,
    },
];

#[test]
fn aiff_mutated_family_outcomes_are_pinned_exactly() {
    for deriv in AIFF_DERIVATIVES {
        let rel = format!("generated/malformed/aiff_pcm_s16be_44k1_st__{}", deriv.name);
        let path = testkit::fixture_path(&rel).must();
        let stream = open_via_input(&path).must();
        let mut aiff = match AiffDemuxer::new(stream) {
            Ok(aiff) => aiff,
            Err(err) => unreachable!("{} opens through the access stack: {err}", deriv.name),
        };
        match aiff.start() {
            Ok(tracks) => {
                assert!(
                    !deriv.fails_at_start,
                    "{}: pinned as a start failure",
                    deriv.name
                );
                assert_eq!(tracks.len(), 1, "{}", deriv.name);
            }
            Err(err) => {
                assert!(
                    deriv.fails_at_start,
                    "{}: expected a started walk, got {err}",
                    deriv.name
                );
                assert_eq!(err.context.code, deriv.terminal, "{}", deriv.name);
                match aiff.start() {
                    Ok(_) => unreachable!("{}: sticky", deriv.name),
                    Err(replay) => {
                        assert_eq!(replay.context.code, deriv.terminal, "{}", deriv.name)
                    }
                }
                continue;
            }
        }
        let (payload, reasons, terminal) = drain_aiff(&mut aiff);
        assert_eq!(
            payload.len() as u64,
            deriv.bytes,
            "{}: delivered bytes",
            deriv.name
        );
        assert_eq!(reasons, deriv.reasons, "{}: recovery reasons", deriv.name);
        assert_eq!(terminal, deriv.terminal, "{}: terminal", deriv.name);
    }
}

/// The untouched-prefix derivatives deliver exactly the parent's
/// payload prefix (garbage0 byte-identical; trunc3/trunc4 the
/// whole-frame floor of the cut) — no invented bytes anywhere.
#[test]
fn derivative_payloads_are_parent_prefixes() {
    let parent_rel = "generated/valid/wav_pcm_s16le_44k1_st.wav";
    let parent = testkit::fixture_bytes(parent_rel).must();
    // The parent's data payload starts at 78 (fmt 12..36, LIST
    // 36..70, data header 70..78) — verified by reading the bytes.
    let parent_payload = &parent[78..];

    let garbage =
        testkit::fixture_bytes("generated/malformed/wav_pcm_s16le_44k1_st__garbage0").must();
    assert_eq!(
        &garbage[78..78 + 35_280],
        parent_payload,
        "garbage0: identical payload"
    );

    let trunc3 = testkit::fixture_bytes("generated/malformed/wav_pcm_s16le_44k1_st__trunc3").must();
    assert_eq!(
        &trunc3[78..78 + 17_600],
        &parent_payload[..17_600],
        "trunc3 prefix"
    );
    let trunc4 = testkit::fixture_bytes("generated/malformed/wav_pcm_s16le_44k1_st__trunc4").must();
    assert_eq!(
        &trunc4[78..78 + 35_268],
        &parent_payload[..35_268],
        "trunc4 prefix"
    );

    let aparent_rel = "generated/valid/aiff_pcm_s16be_44k1_st.aiff";
    let aparent = testkit::fixture_bytes(aparent_rel).must();
    // Sound starts at 54 (FORM 12, COMM 12..38, SSND header 38..46,
    // offset/blockSize pair 46..54).
    let asound = &aparent[54..];
    let agarbage =
        testkit::fixture_bytes("generated/malformed/aiff_pcm_s16be_44k1_st__garbage0").must();
    assert_eq!(
        &agarbage[54..54 + 35_280],
        asound,
        "aiff garbage0: identical sound"
    );
    let atrunc4 =
        testkit::fixture_bytes("generated/malformed/aiff_pcm_s16be_44k1_st__trunc4").must();
    assert_eq!(
        &atrunc4[54..54 + 35_268],
        &asound[..35_268],
        "aiff trunc4 prefix"
    );
}

// ---------------------------------------------------------------------------
// The whole committed corpus through the fuzz harnesses (+FB: no
// panic, no hang, bounded allocation — every file, both formats)
// ---------------------------------------------------------------------------

#[test]
fn every_wav_and_aiff_fixture_survives_the_fuzz_harnesses() {
    let mut relatives: Vec<String> = TRUTHS.iter().map(|t| t.fixture.to_string()).collect();
    for name in [
        "trunc0", "trunc1", "trunc2", "trunc3", "trunc4", "garbage0", "flip1", "flip4", "flip16",
    ] {
        relatives.push(format!("generated/malformed/wav_pcm_s16le_44k1_st__{name}"));
        relatives.push(format!(
            "generated/malformed/aiff_pcm_s16be_44k1_st__{name}"
        ));
    }
    for relative in relatives {
        let bytes: Arc<[u8]> = testkit::fixture_bytes(&relative).must().into();
        demux::wav::fuzz::fuzz_demux_wav(&bytes);
        demux::aiff::fuzz::fuzz_demux_aiff(&bytes);
        // Both harnesses must be total over any bytes: a WAV-shaped
        // input never bothers the AIFF harness and vice versa (the
        // construct paths reject, the drain never starts).
    }
}
