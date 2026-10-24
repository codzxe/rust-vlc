//! The real-corpus WAV differential (card 5.A2, the user's standing
//! mandate): every WAV in the randomized real corpus (corpus-v4,
//! ffmpeg-authored bytes) demuxed through the real access stack and
//! compared against the **measured ffmpeg truth** — stream parameters
//! and sample-frame counts from ffprobe, payload digests from
//! ffmpeg's raw dump (`ffmpeg -i <file> -map 0:a -c copy -f md5 -`).
//!
//! Every pinned value below was measured this session from the
//! corpus-v4 instance at `/tmp/my-project/corpus-v4/media` (ffmpeg
//! 7.1 / Lavf61.7.103, the same build that authored the corpus). The
//! corpus regenerates with fresh seeds: when it is absent (and no
//! `LUMEN_CORPUS` override is set) this suite **skips loudly** — the
//! per-file PASS/FAIL table for this instance lives in the worklog
//! entry, exactly as measured.
//!
//! # The ADPCM `fact` discrepancy (measured, not smoothed over)
//!
//! For the two ADPCM files, ffprobe's `duration_ts` is the `fact`
//! chunk's declaration, and the corpus's own encoder wrote it *short*:
//!
//! * `0042` (IMA): fact says 167_853; ffmpeg's own decoder emits
//!   168_822 frames (= 166 full 1024-byte blocks × 1017);
//! * `0102` (MS): fact says 128_000; ffmpeg's decoder emits
//!   128_524 (= 127 full blocks × 1012).
//!
//! The byte extent is authority (the demuxer's documented contract):
//! our frame count equals ffmpeg's **decoded** output exactly, and
//! the fact disagreement is damage-noted (`fact-frames-short-of-data`)
//! — the table below pins both numbers, the discrepancy is visible,
//! never smoothed away.

use std::path::PathBuf;

mod common;

use common::{md5_hex, open_via_input};
use demux::wav::WavDemuxer;
use media_types::MediaKind;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One corpus file's ffmpeg-measured truth.
struct CorpusTruth {
    /// File name inside the corpus media dir.
    file: &'static str,
    /// ffprobe `codec_name` (the registry descriptor we must emit).
    codec: &'static str,
    /// ffprobe `sample_rate`.
    rate: u32,
    /// ffprobe `channels`.
    channels: u16,
    /// ffprobe `duration_ts` (the container's declared sample-frame
    /// count: byte-exact for PCM, the `fact` declaration for ADPCM).
    probe_frames: u64,
    /// The authoritative sample-frame count — ffmpeg's **decoded**
    /// output for ADPCM (where the corpus's fact chunks under-declare),
    /// `duration_ts` everywhere else. This is what our demuxer must
    /// report.
    truth_frames: u64,
    /// `ffmpeg -map 0:a -c copy -f md5 -` (the raw packet-stream dump).
    md5: &'static str,
}

/// The twelve real WAVs of corpus-v4 (all measured this session).
const TRUTHS: [CorpusTruth; 12] = [
    CorpusTruth {
        file: "0004-audio-only.wav",
        codec: "pcm_s24le",
        rate: 8_000,
        channels: 1,
        probe_frames: 32_000,
        truth_frames: 32_000,
        md5: "3851cb662678447e08cc277df2cef5a8",
    },
    CorpusTruth {
        file: "0006-audio-only.wav",
        codec: "pcm_s24le",
        rate: 176_400,
        channels: 2,
        probe_frames: 388_433,
        truth_frames: 388_433,
        md5: "b75effa1f0fa4b5aa117bde6fe8f21be",
    },
    CorpusTruth {
        file: "0013-sr-chaos.wav",
        codec: "pcm_s16le",
        rate: 11_025,
        channels: 1,
        probe_frames: 14_796,
        truth_frames: 14_796,
        md5: "257e76855078d21e34572002cd43f4dc",
    },
    CorpusTruth {
        file: "0016-audio-only.wav",
        codec: "flac",
        rate: 96_000,
        channels: 4,
        probe_frames: 384_000,
        // FLAC-in-WAV: the fact declaration is the only container-level
        // count; our frames() stays honestly None for this payload.
        truth_frames: 384_000,
        md5: "1eb11d2025d1d6dd23ff4c73281d44e0",
    },
    CorpusTruth {
        file: "0026-audio-only.wav",
        codec: "pcm_s16le",
        rate: 11_025,
        channels: 2,
        probe_frames: 44_100,
        truth_frames: 44_100,
        md5: "f470ec0c67569e18f1e2ea90e0854494",
    },
    CorpusTruth {
        file: "0041-audio-only.wav",
        codec: "pcm_s16le",
        rate: 8_000,
        channels: 1,
        probe_frames: 32_000,
        truth_frames: 32_000,
        md5: "4b76cbcebd7e3fd39e1c80958c610adc",
    },
    CorpusTruth {
        file: "0042-audio-only.wav",
        codec: "adpcm_ima_wav",
        rate: 44_056,
        channels: 2,
        // ffprobe duration_ts (the fact declaration) — the corpus's
        // own encoder wrote it 969 frames short of its own geometry.
        probe_frames: 167_853,
        truth_frames: 168_822,
        md5: "7adb44574acf26b75b04fd2c644f0df1",
    },
    CorpusTruth {
        file: "0095-audio-only.wav",
        codec: "pcm_s32le",
        rate: 44_100,
        channels: 1,
        probe_frames: 176_400,
        truth_frames: 176_400,
        md5: "8f7f929bd28c9b9cb3935d40ae2cf446",
    },
    CorpusTruth {
        file: "0102-audio-only.wav",
        codec: "adpcm_ms",
        rate: 32_000,
        channels: 2,
        probe_frames: 128_000,
        truth_frames: 128_524,
        md5: "2d716e7454d6b35d175f661abf3ef0d6",
    },
    CorpusTruth {
        file: "0129-sr-chaos.wav",
        codec: "pcm_s16le",
        rate: 22_050,
        channels: 4,
        probe_frames: 24_255,
        truth_frames: 24_255,
        md5: "85b80f16595882451d1c5379b0eed480",
    },
    CorpusTruth {
        file: "0135-audio-only.wav",
        codec: "pcm_u8",
        rate: 22_050,
        channels: 2,
        probe_frames: 61_233,
        truth_frames: 61_233,
        md5: "483fdc855fc7fffbc634cc63f07cf2df",
    },
    CorpusTruth {
        file: "0138-audio-only.wav",
        codec: "pcm_s16le",
        rate: 16_000,
        channels: 6,
        probe_frames: 64_000,
        truth_frames: 64_000,
        md5: "cdc77c5bc51f43519f421cc18569e742",
    },
];

/// The corpus media dir: the `LUMEN_CORPUS` override, else the
/// sandbox default. `None` when neither exists (the loud skip).
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}

#[test]
fn corpus_wavs_match_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(wav): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
    let mut table = Vec::new();
    for truth in &TRUTHS {
        let path = dir.join(truth.file);
        assert!(path.is_file(), "corpus file missing: {path:?}");
        let stream = open_via_input(&path).must();
        let mut wav = match WavDemuxer::new(stream) {
            Ok(wav) => wav,
            Err(err) => unreachable!("{} constructs: {err}", truth.file),
        };
        let tracks = match wav.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("{} starts: {err}", truth.file),
        };
        assert_eq!(tracks.len(), 1, "{}", truth.file);
        let track = &tracks[0];
        assert_eq!(track.kind, MediaKind::Audio, "{}", truth.file);
        assert_eq!(track.codec.as_str(), truth.codec, "{}", truth.file);
        assert_eq!(
            track.timing.base.as_parts(),
            (1, truth.rate),
            "{}: tick = one sample frame",
            truth.file
        );
        let fmt = wav
            .format()
            .unwrap_or_else(|| unreachable!("{}", truth.file));
        assert_eq!(
            u32::from(fmt.channels),
            u32::from(truth.channels),
            "{}",
            truth.file
        );
        assert_eq!(fmt.samples_per_sec, truth.rate, "{}", truth.file);

        // The span end is the container's sample-frame count.
        let span_frames = match track.timing.span.end {
            Stamp::Present(ts) => ts.ticks as u64,
            Stamp::Missing => unreachable!("{}: span end present", truth.file),
        };
        assert_eq!(
            span_frames, truth.truth_frames,
            "{}: span frames",
            truth.file
        );
        // The demuxer-side count agrees; FLAC-in-WAV stays honestly
        // None (no frame grammar at the demux layer).
        assert_eq!(
            wav.frames(),
            if truth.codec == "flac" {
                None
            } else {
                Some(truth.truth_frames)
            },
            "{}: frames()",
            truth.file
        );

        // Drain: payload parity with ffmpeg's raw dump + clean walk.
        let mut payload = Vec::new();
        let mut reasons = Vec::new();
        let mut terminal = String::new();
        let mut packets = 0u64;
        loop {
            match wav.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    assert!(
                        packet.keyframe,
                        "{}: PCM/ADPCM packets are keyframes",
                        truth.file
                    );
                    packets += 1;
                    payload.extend_from_slice(&packet.data);
                }
                Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
                Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => {
                    terminal = err.context.code.to_owned();
                    break;
                }
            }
        }
        let digest = md5_hex(&payload);
        assert_eq!(
            digest, truth.md5,
            "{}: payload parity vs ffmpeg raw dump",
            truth.file
        );
        assert!(
            terminal.is_empty(),
            "{}: clean EOS (got {terminal})",
            truth.file
        );
        // A clean corpus file walks clean — except the two ADPCM
        // files whose `fact` under-declares (ffmpeg's own duration_ts
        // shows the same lie; see the module docs): the disagreement
        // is damage-noted in frames×block bytes, never silent.
        if truth.probe_frames != truth.truth_frames {
            assert_eq!(
                reasons,
                ["fact-frames-short-of-data"],
                "{}: the fact under-declaration is damage-noted",
                truth.file
            );
            assert_eq!(
                wav.damage_total(),
                (truth.truth_frames - truth.probe_frames) * 1_024,
                "{}: the lying frames, valued in 1024-byte ADPCM blocks",
                truth.file
            );
        } else {
            assert!(
                reasons.is_empty(),
                "{}: no damage ({reasons:?})",
                truth.file
            );
            assert_eq!(
                wav.damage_total(),
                0,
                "{}: a clean corpus file walks clean",
                truth.file
            );
        }
        table.push(format!(
            "PASS {} codec={} rate={} ch={} frames={} (probe {}) md5={} packets={}",
            truth.file,
            truth.codec,
            truth.rate,
            truth.channels,
            truth.truth_frames,
            truth.probe_frames,
            digest,
            packets
        ));
    }
    for row in &table {
        eprintln!("corpus(wav): {row}");
    }
    eprintln!(
        "corpus(wav): {}/{} files PASS the ffmpeg differential",
        table.len(),
        TRUTHS.len()
    );
}

/// Every corpus WAV (and its mutated cousins under `dirty/`, when the
/// harness hands them over) survives the fuzz harness — the `+FB`
/// no-panic/no-hang/bounded-memory contract over real-world bytes.
#[test]
fn corpus_wavs_survive_the_fuzz_harness() {
    let Some(dir) = corpus_dir() else {
        eprintln!("corpus(wav): corpus absent — SKIP (fuzz sweep)");
        return;
    };
    for truth in &TRUTHS {
        let bytes = std::fs::read(dir.join(truth.file)).must();
        demux::wav::fuzz::fuzz_demux_wav(&bytes);
    }
    // The dirty/garbage siblings of the WAV lanes, when present: any
    // bytes are legal fuzz input (D-3 over the real corpus family).
    for subdir in ["dirty", "garbage"] {
        let dir = dir.join(subdir);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            let is_wavish = path.extension().is_some_and(|ext| {
                ext.to_str().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("wav") || ext.eq_ignore_ascii_case("wave")
                })
            });
            if is_wavish {
                let bytes = std::fs::read(&path).must();
                demux::wav::fuzz::fuzz_demux_wav(&bytes);
            }
        }
    }
}
