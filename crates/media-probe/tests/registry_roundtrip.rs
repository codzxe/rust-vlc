//! The R3 registration round-trip suite (card 5.R3-REG): every
//! landed container format, probed from the committed fixtures its
//! demuxer card authored, through the REAL registration surface —
//! `ProbeRegistry::with_builtins()` — asserting the format id the
//! open path will route on (doc 01 §2 step 3, doc 03 §4.2).
//!
//! The fixtures are the demuxer/codec cards' own differential
//! evidence (ffmpeg-authored, sha/MD5-pinned by their suites); this
//! suite re-uses them read-only as probe windows. Two fixture homes
//! exist: the workspace `tests/fixtures/` root (via `testkit`) and
//! the per-crate fixture dirs of `demux`/`codecs` (cross-crate
//! read-only, the `CARGO_MANIFEST_DIR` relative path — the same
//! pattern the demux suites use for their own fixture homes).
//!
//! Also pins the extensions registration (each signature's claimed
//! extension list must contain its fixture's extension — the
//! mirror of the demuxers' `EXTENSIONS` tables) and the honest
//! negatives: garbage and near-miss magics stay `probe.no-demuxer`.

use media_probe::signature::{
    AdtsSignature, AiffSignature, AnnexbSignature, AviSignature, FlacSignature, IvfSignature,
    MkvSignature, Mp2Signature, Mp3Signature, Mp4Signature, OggSignature, ProbePlugin, PsSignature,
    TsSignature, WavSignature,
};
use media_probe::{ProbeConfig, ProbeRegistry, SourceMeta};
use media_types::PipelineStage;
use testkit::{Must, fixture_path};

/// One registration row: fixture → expected format id.
struct Row {
    /// Fixture path relative to the workspace `tests/fixtures/`
    /// root, or `None` for a crate-relative path.
    fixture: Option<&'static str>,
    /// Crate-relative fixture path (manifest-dir based) when the
    /// fixture lives in a sibling crate's fixture home.
    crate_fixture: Option<&'static str>,
    expected: &'static str,
    extension: &'static str,
}

/// The bounded probe window: the leading 4096 bytes of a fixture.
fn window(path: &std::path::Path) -> Vec<u8> {
    let bytes = std::fs::read(path).must();
    let take = bytes.len().min(4096);
    bytes[..take].to_vec()
}

/// The round-trip: fixture bytes → the builtin registry → format id.
#[test]
fn every_landed_container_probes_to_its_format_id() {
    let rows = [
        // First vertical + its sibling (2.A2's signatures, re-proven
        // through the consolidated registry).
        Row {
            fixture: Some("generated/valid/avi_mjpeg_pcm.avi"),
            crate_fixture: None,
            expected: "avi",
            extension: "avi",
        },
        Row {
            fixture: Some("generated/valid/mp4_h264_aac.mp4"),
            crate_fixture: None,
            expected: "mp4",
            extension: "mp4",
        },
        Row {
            fixture: Some("generated/valid/mp4_qt.mov"),
            crate_fixture: None,
            expected: "mp4",
            extension: "mov",
        },
        // 5.A2: WAV + AIFF.
        Row {
            fixture: Some("generated/valid/wav_pcm_s16le_44k1_st.wav"),
            crate_fixture: None,
            expected: "wav",
            extension: "wav",
        },
        Row {
            fixture: Some("generated/valid/wav_rf64_44k1_st.wav"),
            crate_fixture: None,
            expected: "wav",
            extension: "wav",
        },
        Row {
            fixture: Some("generated/valid/aiff_pcm_s16be_44k1_st.aiff"),
            crate_fixture: None,
            expected: "aiff",
            extension: "aiff",
        },
        Row {
            fixture: Some("generated/valid/aifc_sowt_s16le_44k1_st.aifc"),
            crate_fixture: None,
            expected: "aiff",
            extension: "aifc",
        },
        // 5.B1: Ogg (vorbis + opus carriages).
        Row {
            fixture: Some("generated/valid/ogg_vorbis_8k_st.ogg"),
            crate_fixture: None,
            expected: "ogg",
            extension: "ogg",
        },
        Row {
            fixture: Some("generated/valid/ogg_opus_48k_mono.opus"),
            crate_fixture: None,
            expected: "ogg",
            extension: "opus",
        },
        // 5.D1: Matroska + WebM.
        Row {
            fixture: Some("generated/valid/mkv_h264_aac.mkv"),
            crate_fixture: None,
            expected: "matroska",
            extension: "mkv",
        },
        Row {
            fixture: Some("generated/valid/webm_vp9_opus.webm"),
            crate_fixture: None,
            expected: "matroska",
            extension: "webm",
        },
        // 5.C2b: ADTS (the codecs card's AAC fixtures).
        Row {
            fixture: None,
            crate_fixture: Some("../codecs/tests/fixtures/sine48_lc_mono.aac"),
            expected: "adts",
            extension: "aac",
        },
        // 5.I1: MPEG-TS (the mpeg12 card's in-TS fixture).
        Row {
            fixture: None,
            crate_fixture: Some("../codecs/tests/fixtures/mpeg12/mpeg2-in-ts.ts"),
            expected: "mpegts",
            extension: "ts",
        },
        // 5.I2: MPEG-PS / DVD-VOB (the demuxer card's committed
        // differential fixtures).
        Row {
            fixture: None,
            crate_fixture: Some("../demux/fixtures/ps/ps-vob-ac3.vob"),
            expected: "mpegps",
            extension: "vob",
        },
        Row {
            fixture: None,
            crate_fixture: Some("../demux/fixtures/ps/ps-mp2-video-audio.mpg"),
            expected: "mpegps",
            extension: "mpg",
        },
        // 5.I3b: IVF (AV1 + VP9 deliveries).
        Row {
            fixture: Some("generated/ivf/av1_64x48_30fps.ivf"),
            crate_fixture: None,
            expected: "ivf",
            extension: "ivf",
        },
        Row {
            fixture: Some("generated/ivf/vp9_64x48_25fps.ivf"),
            crate_fixture: None,
            expected: "ivf",
            extension: "ivf",
        },
        // 6.OPW-es: the raw elementary-stream family. The MPEG-audio
        // rows ride the codecs card's committed fixtures (the
        // ID3-prefixed `mpeg2_24k.mp3` proves the tag-skip anchor;
        // `mp2_48k.mp2` the layer-10 discrimination); the Annex-B
        // rows are this card's own committed ffmpeg-authored
        // fixtures (SPS-led H.264, VPS-led HEVC — the decisive
        // parameter-set shapes); the FLAC row is the codecs card's
        // committed native-FLAC fixture.
        Row {
            fixture: None,
            crate_fixture: Some("../codecs/tests/fixtures/mpeg2_24k.mp3"),
            expected: "mp3",
            extension: "mp3",
        },
        Row {
            fixture: None,
            crate_fixture: Some("../codecs/tests/fixtures/mono48.mp3"),
            expected: "mp3",
            extension: "mp3",
        },
        Row {
            fixture: None,
            crate_fixture: Some("../codecs/tests/fixtures/mp2_48k.mp2"),
            expected: "mp2",
            extension: "mp2",
        },
        Row {
            fixture: None,
            crate_fixture: Some("fixtures/annexb_probe_h264.264"),
            expected: "h264",
            extension: "264",
        },
        Row {
            fixture: None,
            crate_fixture: Some("fixtures/annexb_probe_hevc.265"),
            expected: "hevc",
            extension: "265",
        },
        Row {
            fixture: None,
            crate_fixture: Some("../codecs/tests/fixtures/flac_mono_s16_44100.flac"),
            expected: "flac",
            extension: "flac",
        },
    ];
    let registry = ProbeRegistry::with_builtins();
    let config = ProbeConfig::new(4096, 60).must();
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for row in rows {
        let path = match (row.fixture, row.crate_fixture) {
            (Some(relative), None) => fixture_path(relative).must(),
            (None, Some(relative)) => manifest_dir.join(relative),
            _ => unreachable!("a row must name exactly one fixture home"),
        };
        let bytes = window(&path);
        assert!(!bytes.is_empty(), "{}: empty probe window", path.display());
        let meta = SourceMeta::new(None, Some(row.extension));
        let report = registry.probe(&bytes, &meta, &config).must();
        assert_eq!(
            report.format.as_str(),
            row.expected,
            "{}: wrong format id",
            path.display()
        );
        assert!(
            report.confidence.as_u8() >= 60,
            "{}: confidence {} below threshold",
            path.display(),
            report.confidence.as_u8()
        );
    }
}

/// The extensions registration: every builtin signature's claimed
/// list contains its fixtures' extensions (the mirror of the
/// demuxers' `EXTENSIONS` tables — drift here is a registration
/// bug, not a hint).
#[test]
fn extension_lists_cover_their_fixtures() {
    let rows: [(Box<dyn ProbePlugin>, &[&str]); 14] = [
        (Box::new(AviSignature), &["avi"]),
        (Box::new(Mp4Signature), &["mp4", "mov"]),
        (Box::new(WavSignature), &["wav"]),
        (Box::new(AiffSignature), &["aiff", "aifc"]),
        (Box::new(OggSignature), &["ogg", "opus"]),
        (Box::new(MkvSignature), &["mkv", "webm"]),
        (Box::new(AdtsSignature), &["aac"]),
        (Box::new(TsSignature), &["ts"]),
        (Box::new(PsSignature), &["vob", "mpg"]),
        (Box::new(IvfSignature), &["ivf"]),
        (Box::new(Mp3Signature), &["mp3"]),
        (Box::new(Mp2Signature), &["mp2"]),
        (Box::new(AnnexbSignature), &["264", "265"]),
        (Box::new(FlacSignature), &["flac"]),
    ];
    for (signature, claimed_by_fixtures) in rows {
        let extensions = signature.extensions();
        for extension in claimed_by_fixtures {
            assert!(
                extensions.contains(extension),
                "extension `{extension}` missing from the registration list {extensions:?}"
            );
        }
    }
}

/// Honest negatives: near-miss magics must NOT be stolen by the
/// ADTS signature (the 12-bit sync + zero layer refinement), and
/// garbage stays a typed no-hit. The MPEG-1 Layer III frame header
/// that used to sit in the negative list (an ADTS near-miss) now
/// probes **`mp3`** — 6.OPW-es landed that signature — so the
/// non-collision is pinned from the positive side below.
#[test]
fn near_misses_and_garbage_stay_unprobed() {
    let registry = ProbeRegistry::with_builtins();
    let config = ProbeConfig::new(4096, 60).must();
    let meta = SourceMeta::new(Some(4096), Some("bin"));
    for case in [
        &b"OggT\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00"[..],
        &b"FORM\x00\x00\x00\x10AIFX"[..],
        // TS: sync byte present but no second sync at the 188-byte
        // stride — the demuxer's own probe gate refuses it too.
        &[[0x47_u8].as_slice(), [0u8; 200].as_slice()].concat()[..],
        &b"DKIY\x00\x20\x00\x00"[..],
        &b"\x00\x00\x01\xBB\x00\x0d\x00\x00"[..],
    ] {
        match registry.probe(case, &meta, &config) {
            Ok(report) => unreachable!("must not probe: {case:?} -> {report:?}"),
            Err(err) => {
                assert_eq!(err.context.code, "probe.no-demuxer");
                assert_eq!(err.stage, PipelineStage::Probe);
            }
        }
    }
    // The ADTS/MPEG-audio boundary, pinned positively from both
    // sides: the Layer III header probes `mp3` (NOT `adts`), and a
    // layer-00 frame probes `adts` (NOT `mp3`) — the layer-bit
    // disjointness is the two signatures' shared contract.
    let mp3_frame = [0xFF_u8, 0xFB, 0x90, 0x00];
    let report = registry.probe(&mp3_frame, &meta, &config).must();
    assert_eq!(report.format.as_str(), "mp3");
    let adts_frame = [0xFF_u8, 0xF1, 0x50, 0x80, 0x00, 0x00];
    let report = registry.probe(&adts_frame, &meta, &config).must();
    assert_eq!(report.format.as_str(), "adts");
}
