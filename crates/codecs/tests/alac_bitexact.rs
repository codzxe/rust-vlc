//! ALAC differential suite (card 5.J3a AC): the ffmpeg-generated
//! fixtures (stereo/mono × 16/24-bit, 0.7 s each — 8 packets per file
//! incl. the final **partial frame**, so the multi-frame and the
//! `has_size` paths are both exercised) decode **bit-exactly** against
//! the ffmpeg 7.1.5 reference, through the full L1→L3 composition:
//! `input` FileAccess → `demux::mp4` (5.C1) → this decoder. No
//! synthetic fixture anywhere in the path.
//!
//! # The oracle (and why it is an interleaved dump, not `-f md5`)
//!
//! ffmpeg's ALAC decoder outputs **planar** samples (`s16p`/`s32p`),
//! so `ffmpeg -map 0:a -f md5` hashes planar-layout PCM while Lumen's
//! contract output is interleaved — the digests would differ even for
//! identical samples. The honest oracle is the interleaved dump:
//! `ffmpeg -i <file> -f s16le -` (16-bit) / `-f s24le -` (24-bit),
//! which is byte-comparable to our negotiated `S16`/`S24`
//! native-endian interleaved output on this (little-endian) host.
//!
//! # Pinned truth (measured this session, ffmpeg 7.1.5)
//!
//! | fixture | ch | rate | width | ffmpeg MD5 (interleaved dump) |
//! |---|---|---|---|---|
//! | `alac_stereo16.m4a` | 2 | 44_100 | s16le | `e33ff912590e04d68117017a1a2ccd23` |
//! | `alac_mono16.m4a` | 1 | 44_100 | s16le | `a2a11b40df939a1d604fd71233cf83e8` |
//! | `alac_stereo24.m4a` | 2 | 48_000 | s24le | `04ca6310cf048ddca4f12374b1c3849e` |
//! | `alac_mono24.m4a` | 1 | 48_000 | s24le | `109eea10ed4c881a5516143c817f1595` |
//!
//! The 24-bit rows exercise the extra-bits (shift > 0) splice and the
//! wider rice-escape width. The prior wave-5 session's suspected
//! engine bug (`alac.frame.truncated` on multi-frame stereo) is
//! **not reproduced** here — see the decoder module docs for the
//! bit-level audit; these rows are the tripwire: any engine or
//! wrapper drift fails loudly.

mod common;

use std::path::{Path, PathBuf};

use codec_api::prelude::*;
use common::md5_hex;
use demux::mp4::Mp4Demuxer;
use media_types::ids::{CodecDescriptor, SourceUri};
use media_types::packet::SharedBytes;
use plugin_api::{Access, DemuxEvent, Demuxer};
use testkit::Must;

/// One fixture's ffmpeg-measured truth.
struct Truth {
    fixture: &'static str,
    channels: u32,
    rate: u32,
    /// `s16le` / `s24le` — the oracle dump format (= our width).
    dump: &'static str,
    md5: &'static str,
    /// Expected packets (7 × 4096 + 1 partial at 0.7 s).
    packets: u64,
}

const TRUTHS: [Truth; 4] = [
    Truth {
        fixture: "alac_stereo16.m4a",
        channels: 2,
        rate: 44_100,
        dump: "s16le",
        md5: "e33ff912590e04d68117017a1a2ccd23",
        packets: 8,
    },
    Truth {
        fixture: "alac_mono16.m4a",
        channels: 1,
        rate: 44_100,
        dump: "s16le",
        md5: "a2a11b40df939a1d604fd71233cf83e8",
        packets: 8,
    },
    Truth {
        fixture: "alac_stereo24.m4a",
        channels: 2,
        rate: 48_000,
        dump: "s24le",
        md5: "04ca6310cf048ddca4f12374b1c3849e",
        packets: 9,
    },
    Truth {
        fixture: "alac_mono24.m4a",
        channels: 1,
        rate: 48_000,
        dump: "s24le",
        md5: "109eea10ed4c881a5516143c817f1595",
        packets: 9,
    },
];

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/alac").join(name)
}

/// The real byte source (the 5.J2a composition: input FileAccess).
fn open_via_input(path: &Path) -> Result<Box<dyn plugin_api::SourceStream>, media_types::EngineError> {
    let uri = match SourceUri::parse(&format!("file://{}", path.to_str().unwrap_or(""))) {
        Ok(uri) => uri,
        Err(err) => unreachable!("fixture path is a valid file URI ({path:?}): {err}"),
    };
    let access = input::FileAccess::new();
    access.open(&uri)
}

/// Capture sink: concatenates the PCM and tracks pts monotonicity.
#[derive(Default)]
struct Capture {
    pcm: Vec<u8>,
    frames: u64,
    last_pts: Option<u64>,
    pts_monotonic: bool,
}

impl FrameSink for Capture {
    fn accept(&mut self, frame: Frame) -> Result<(), media_types::EngineError> {
        match frame {
            Frame::Audio(audio) => {
                if let media_types::Stamp::Present(ts) = audio.pts {
                    let nanos = match ts.as_nanos() {
                        Ok(nanos) => nanos.0,
                        Err(err) => unreachable!("clean pts converts: {err:?}"),
                    };
                    self.pts_monotonic = match self.last_pts {
                        Some(last) => self.pts_monotonic && nanos >= last,
                        None => true,
                    };
                    self.last_pts = Some(nanos);
                }
                self.pcm.extend_from_slice(&audio.samples);
                self.frames += 1;
            }
            Frame::Video(_) | Frame::Subtitle(_) => {
                unreachable!("ALAC emits audio only")
            }
        }
        Ok(())
    }
}

/// The full pipeline for one file: open → demux (real MP4) → configure
/// → decode every packet → the concatenated PCM + facts.
fn decode_file(path: &Path) -> (Vec<u8>, u64, bool, media_types::frame::AudioFormat) {
    let stream = open_via_input(path).must();
    let mut mp4 = match Mp4Demuxer::new(stream) {
        Ok(mp4) => mp4,
        Err(err) => unreachable!("{} constructs: {err}", path.display()),
    };
    let tracks = match mp4.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("{} starts: {err}", path.display()),
    };
    assert_eq!(tracks.len(), 1, "{}", path.display());
    assert_eq!(
        tracks[0].kind,
        media_types::MediaKind::Audio,
        "{}",
        path.display()
    );
    let descriptor = tracks[0].codec.as_str().to_string();
    assert_eq!(descriptor, "alac", "{}: registry descriptor", path.display());
    let cookie = mp4
        .sample_entry_bytes(tracks[0].id)
        .unwrap_or_else(|| unreachable!("{}: sample entry present", path.display()))
        .to_vec();
    let mut decoder = codecs::audio::alac::AlacDecoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new(&descriptor).must(),
            media_types::GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(cookie)),
        ))
        .must();
    let mut capture = Capture::default();
    let mut packets = 0u64;
    loop {
        match mp4.next_event().must() {
            DemuxEvent::Packet(packet) => {
                packets += 1;
                let outcome = decoder.decode(packet, &mut capture).must();
                assert!(
                    matches!(outcome, DecodeOutcome::Emitted { frames: 1 }),
                    "{}: clean file, every packet emits (got {outcome:?})",
                    path.display()
                );
            }
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => continue,
            DemuxEvent::EndOfStream => break,
        }
    }
    let sample_entry = mp4
        .sample_entry_bytes(tracks[0].id)
        .unwrap_or_else(|| unreachable!("{}: sample entry present", path.display()));
    let parsed = match codecs::audio::alac::parse_alac_config(sample_entry) {
        Ok(parsed) => parsed,
        Err(e) => unreachable!("cookie parses: {}", e.context.code),
    };
    let format = codecs::audio::alac::output_format(&parsed);
    (capture.pcm, packets, capture.pts_monotonic, format)
}

#[test]
fn fixtures_bitexact_vs_ffmpeg() {
    for truth in TRUTHS {
        let path = fixture_path(truth.fixture);
        let (pcm, packets, pts_monotonic, format) = decode_file(&path);
        // The free surface's shape preview agrees with the contract's.
        assert_eq!(format.layout, media_types::frame::AudioLayout::Interleaved);
        assert_eq!(packets, truth.packets, "{}: packet count", truth.fixture);
        assert_eq!(format.channels, truth.channels, "{}: channels", truth.fixture);
        assert_eq!(format.rate, truth.rate, "{}: rate", truth.fixture);
        assert!(pts_monotonic, "{}: pts monotone", truth.fixture);
        let ours = md5_hex(&pcm);
        assert_eq!(
            ours, truth.md5,
            "{}: whole-output PCM MD5 vs ffmpeg {} dump",
            truth.fixture, truth.dump
        );
        // Byte-level window sanity: the dump length is the full
        // decoded extent on both sides (0.7 s × rate × channels ×
        // width — no truncation, no padding).
        let width = if truth.dump == "s16le" { 2 } else { 3 };
        let expected_len =
            (f64::from(truth.rate) * 0.7).round() as usize * truth.channels as usize * width;
        assert_eq!(
            pcm.len(),
            expected_len,
            "{}: decoded extent (expected ~{expected_len} bytes)",
            truth.fixture
        );
    }
}
