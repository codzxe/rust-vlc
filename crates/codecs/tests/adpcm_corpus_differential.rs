//! ADPCM differential suite (card 5.J2a AC): the real-corpus ADPCM
//! files (0042 IMA-WAV, 0102 MS) plus one ffmpeg-generated fixture
//! per flavor decode **bit-exactly** against the ffmpeg 7.1.5
//! reference — the concatenated PCM MD5 of our decoder's output
//! equals `ffmpeg -i <file> -map 0:a -f md5 -`.
//!
//! # The comparison window (the fact-chunk damage, documented)
//!
//! The two corpus files carry `fact` chunks their own encoder wrote
//! short of the data extent (0042: fact 167_853 vs data-derived
//! 168_822; 0102: fact 128_000 vs 128_524 — the demuxer suite's
//! documented damage). **ffmpeg decodes the data-derived window**
//! (measured this session: 675_288 output bytes = 168_822 frames × 4
//! for 0042; 514_096 = 128_524 × 4 for 0102), and so does this
//! decoder — the demuxer hands whole `nBlockAlign` blocks only, the
//! byte extent is the authority, and the MD5 compares the identical
//! window on both sides. No truncation, no smoothing.
//!
//! # Pinned truth (measured this session, ffmpeg 7.1.5)
//!
//! | file | descriptor | ch | rate | blocks | decoded frames | ffmpeg MD5 |
//! |---|---|---|---|---|---|---|
//! | corpus 0042 | `adpcm_ima_wav` | 2 | 44_056 | 166 × 1024 | 168_822 | `72528c03a21636ae7e69c5ada81077e6` |
//! | corpus 0102 | `adpcm_ms` | 2 | 32_000 | 127 × 1024 | 128_524 | `a1de073a928ed1209422852ff5b99a28` |
//! | fixture `adpcm_ima_mono.wav` | `adpcm_ima_wav` | 1 | 8_000 | 6 × 1024 | 12_246 | `8a8146087f7573fa71963c3ae2c65f3b` |
//! | fixture `adpcm_ms_mono.wav` | `adpcm_ms` | 1 | 8_000 | 6 × 1024 | 12_216 | `a95d2cbb66f65a4adcb36b57bc68e48e` |
//!
//! The fixtures are the mono complement to the stereo corpus (the
//! mono nibble paths, 2 samples per byte). Corpus rows skip loudly
//! when the corpus dir is absent (`LUMEN_CORPUS` override honored);
//! the table for the 2026-10-24 instance is in the worklog.

mod common;

use std::path::PathBuf;
use std::time::Instant;

use codec_api::prelude::*;
use common::md5_hex;
use demux::wav::WavDemuxer;
use media_types::ids::CodecDescriptor;
use media_types::packet::SharedBytes;
use plugin_api::{Access, DemuxEvent, Demuxer};
use testkit::Must;

/// One file's ffmpeg-measured truth.
struct Truth {
    /// Display name (corpus) or fixture stem.
    name: &'static str,
    /// Registry descriptor the track must carry.
    descriptor: &'static str,
    /// Channels.
    channels: u32,
    /// Sample rate.
    rate: u32,
    /// Decoded sample frames (the data-derived window).
    frames: u64,
    /// MD5 of the whole decoded PCM (s16le packed, the ffmpeg dump).
    md5: &'static str,
}

const TRUTHS: [Truth; 2] = [
    Truth {
        name: "adpcm_ima_mono.wav",
        descriptor: "adpcm_ima_wav",
        channels: 1,
        rate: 8_000,
        frames: 12_246,
        md5: "8a8146087f7573fa71963c3ae2c65f3b",
    },
    Truth {
        name: "adpcm_ms_mono.wav",
        descriptor: "adpcm_ms",
        channels: 1,
        rate: 8_000,
        frames: 12_216,
        md5: "a95d2cbb66f65a4adcb36b57bc68e48e",
    },
];

const CORPUS_TRUTHS: [Truth; 2] = [
    Truth {
        name: "0042-audio-only.wav",
        descriptor: "adpcm_ima_wav",
        channels: 2,
        rate: 44_056,
        frames: 168_822,
        md5: "72528c03a21636ae7e69c5ada81077e6",
    },
    Truth {
        name: "0102-audio-only.wav",
        descriptor: "adpcm_ms",
        channels: 2,
        rate: 32_000,
        frames: 128_524,
        md5: "a1de073a928ed1209422852ff5b99a28",
    },
];

/// The corpus media dir: `LUMEN_CORPUS` override, else the sandbox
/// default. `None` when neither exists (the loud skip).
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}

/// A capture sink concatenating audio frames.
#[derive(Default)]
struct Capture {
    pcm: Vec<u8>,
    frames: u64,
    first_pts: Option<media_types::time::Stamp>,
}

impl FrameSink for Capture {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            return Err(EngineError::new(
                media_types::PipelineStage::Decode,
                media_types::ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                media_types::error::ErrorContext::new("adpcm.differential.sink"),
            ));
        };
        if self.first_pts.is_none() {
            self.first_pts = Some(audio.pts);
        }
        self.pcm.extend_from_slice(audio.samples.as_slice());
        let per_instant = u64::from(audio.format.channels) * 2;
        self.frames += (audio.samples.len() as u64) / per_instant;
        Ok(())
    }
}

/// Open a path through `input`'s FileAccess (the first vertical's
/// access stack).
fn open_via_input(
    path: &std::path::Path,
) -> Result<Box<dyn plugin_api::SourceStream>, EngineError> {
    let uri = match media_types::ids::SourceUri::parse(&format!(
        "file://{}",
        path.to_str().unwrap_or("")
    )) {
        Ok(uri) => uri,
        Err(err) => unreachable!("fixture path is a valid file URI ({path:?}): {err}"),
    };
    let access = input::FileAccess::new();
    access.open(&uri)
}

/// The full pipeline for one file: open → demux → configure → decode
/// every packet → the concatenated PCM. Returns `(pcm, frames,
/// first_pts, elapsed)`.
fn decode_file(
    path: &std::path::Path,
    expected: &Truth,
) -> (
    Vec<u8>,
    u64,
    Option<media_types::time::Stamp>,
    std::time::Duration,
) {
    let started = Instant::now();
    let stream = open_via_input(path).must();
    let mut wav = match WavDemuxer::new(stream) {
        Ok(wav) => wav,
        Err(err) => unreachable!("{} constructs: {err}", path.display()),
    };
    let tracks = match wav.start() {
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
    assert_eq!(
        descriptor, expected.descriptor,
        "{}: registry descriptor",
        expected.name
    );
    let fmt_bytes = wav
        .fmt_chunk_bytes()
        .unwrap_or_else(|| unreachable!("{}: fmt bytes present", path.display()))
        .to_vec();
    let mut decoder = codecs::audio::adpcm::AdpcmDecoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new(&descriptor).must(),
            media_types::GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(fmt_bytes)),
        ))
        .must();
    let wav_format = wav
        .format()
        .unwrap_or_else(|| unreachable!("{}: format present", path.display()));
    assert_eq!(
        u32::from(wav_format.channels),
        expected.channels,
        "{}: channels",
        expected.name
    );
    assert_eq!(
        wav_format.samples_per_sec, expected.rate,
        "{}: rate",
        expected.name
    );
    let mut capture = Capture::default();
    loop {
        match wav.next_event().must() {
            DemuxEvent::Packet(packet) => {
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
    let elapsed = started.elapsed();
    (capture.pcm, capture.frames, capture.first_pts, elapsed)
}

/// Assert one truth row end-to-end.
fn check(truth: &Truth, path: &std::path::Path) {
    let (pcm, frames, first_pts, elapsed) = decode_file(path, truth);
    assert_eq!(
        frames, truth.frames,
        "{}: decoded frame count (data-derived window)",
        truth.name
    );
    let digest = md5_hex(&pcm);
    assert_eq!(
        digest, truth.md5,
        "{}: concatenated PCM MD5 vs the ffmpeg dump",
        truth.name
    );
    // The PTS ladder starts at zero (tick = sample frame).
    assert!(
        matches!(first_pts, Some(media_types::time::Stamp::Present(ts)) if ts.ticks == 0),
        "{}: first pts is present and zero",
        truth.name
    );
    // Throughput observability (the 5.X1 note: raw numbers, no gate).
    let input_bytes = std::fs::metadata(path).map_or(0u64, |m| m.len());
    let decoded_mb = pcm.len() as f64 / 1e6;
    eprintln!(
        "adpcm-differential[{}]: {} frames in {:?} — {:.2} MB PCM decoded \
         ({:.0}× file bytes), throughput {:.1} MB/s of PCM",
        truth.name,
        frames,
        elapsed,
        decoded_mb,
        if input_bytes > 0 {
            decoded_mb / (input_bytes as f64 / 1e6)
        } else {
            0.0
        },
        if elapsed.as_secs_f64() > 0.0 {
            decoded_mb / elapsed.as_secs_f64()
        } else {
            0.0
        },
    );
}

#[test]
fn committed_fixtures_decode_bit_exact() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for truth in &TRUTHS {
        check(truth, &root.join(truth.name));
    }
}

#[test]
fn real_corpus_decodes_bit_exact() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(adpcm): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
    for truth in &CORPUS_TRUTHS {
        check(truth, &dir.join(truth.name));
    }
}
