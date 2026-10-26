//! ADPCM decode throughput (card 6.ADPCM-bench, Phase-6 wave-1
//! closeout): the 5.A1 `flac_bench.rs` convention — an ignored
//! measurement test whose number goes into the card's worklog entry
//! and the 5.R1 snapshot row it closes, never the normal gate run.
//!
//! The measured shape is the **same contract path the corpus
//! differential drives** (open via `input`'s FileAccess → WavDemuxer →
//! configure from the fmt chunk → one packet per block → sink) over
//! the real corpus ADPCM lanes: 0042 (`adpcm_ima_wav`, 44.056 kHz
//! stereo, 166 blocks / 168 822 sample frames) and 0102
//! (`adpcm_ms`, 32 kHz stereo, 127 blocks / 128 524 sample frames) —
//! the lane pair whose decode bit-exactness
//! `adpcm_corpus_differential` pins. Decode-only throughput is
//! strictly higher than the reported number (the wav demux + file
//! open ride along, `alac_bench.rs` style). The corpus is
//! /tmp-resident; when absent the test SKIPs with a message (the
//! same honesty rule as the differential) and the 5.R1 row it closes
//! stays DNF.

use std::path::{Path, PathBuf};
use std::time::Instant;

use codec_api::prelude::*;
use demux::wav::WavDemuxer;
use media_types::ids::{CodecDescriptor, SourceUri};
use media_types::packet::SharedBytes;
use plugin_api::{Access, DemuxEvent, Demuxer};

/// One corpus lane's measured truth (the differential's pinned table,
/// data-derived window).
struct Lane {
    /// Corpus file name.
    name: &'static str,
    /// Registry descriptor the track must carry (configure input).
    descriptor: &'static str,
    /// Channels.
    channels: u32,
    /// Sample rate (Hz).
    rate: u32,
    /// Decoded sample frames in the data-derived window (the
    /// differential's MD5 pins this exact extent).
    frames: u64,
}

const LANES: [Lane; 2] = [
    Lane {
        name: "0042-audio-only.wav",
        descriptor: "adpcm_ima_wav",
        channels: 2,
        rate: 44_056,
        frames: 168_822,
    },
    Lane {
        name: "0102-audio-only.wav",
        descriptor: "adpcm_ms",
        channels: 2,
        rate: 32_000,
        frames: 128_524,
    },
];

/// Repetitions per measurement (median reported, as in pcm_bench).
const REPS: usize = 7;

/// The corpus media dir: `LUMEN_CORPUS` override (honored as either
/// the corpus root or the media dir itself — the two conventions the
/// sibling benches use), else the sandbox default. `None` when
/// nothing exists (the loud skip).
fn corpus_media_dir() -> Option<PathBuf> {
    let candidate = |p: PathBuf| p.is_dir().then_some(p);
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        let dir = PathBuf::from(dir);
        return candidate(dir.join("media")).or_else(|| candidate(dir));
    }
    candidate(PathBuf::from("/tmp/my-project/corpus-v4/media"))
}

/// Open a path through `input`'s FileAccess (the first vertical's
/// access stack — the differential's exact open path).
fn open_via_input(
    path: &Path,
) -> Result<Box<dyn plugin_api::SourceStream>, media_types::EngineError> {
    let uri = match SourceUri::parse(&format!("file://{}", path.to_str().unwrap_or(""))) {
        Ok(uri) => uri,
        Err(err) => unreachable!("corpus path is a valid file URI ({path:?}): {err}"),
    };
    input::FileAccess::new().open(&uri)
}

/// A counting sink: inter-channel sample count only, no PCM copy
/// (throughput shape — the differential owns the byte-level truth).
#[derive(Default)]
struct CountingSink {
    inter_channel: u64,
}

impl FrameSink for CountingSink {
    fn accept(&mut self, frame: Frame) -> Result<(), media_types::EngineError> {
        let Frame::Audio(audio) = frame else {
            unreachable!("the wav lane carries audio only")
        };
        let bytes_per_sample = u64::from(audio.format.sample.bytes_per_sample().max(1));
        self.inter_channel += audio.samples.len() as u64 / bytes_per_sample;
        Ok(())
    }
}

/// One full contract-path pass over a lane (the differential's exact
/// shape): open → demux → configure → decode every packet → sink.
/// Returns the sink's inter-channel sample count.
fn decode_pass(lane: &Lane, path: &Path) -> u64 {
    let stream = open_via_input(path).unwrap_or_else(|e| unreachable!("{}: open: {e}", lane.name));
    let mut wav =
        WavDemuxer::new(stream).unwrap_or_else(|e| unreachable!("{}: new: {e}", lane.name));
    let tracks = wav
        .start()
        .unwrap_or_else(|e| unreachable!("{}: start: {e}", lane.name));
    assert_eq!(tracks.len(), 1, "{}: one audio track", lane.name);
    assert_eq!(
        tracks[0].codec.as_str(),
        lane.descriptor,
        "{}: registry descriptor",
        lane.name
    );
    let fmt_bytes = wav
        .fmt_chunk_bytes()
        .unwrap_or_else(|| unreachable!("{}: fmt chunk present", lane.name))
        .to_vec();
    let mut decoder = codecs::audio::adpcm::AdpcmDecoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new(lane.descriptor)
                .unwrap_or_else(|e| unreachable!("{}: descriptor: {e}", lane.name)),
            media_types::GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(fmt_bytes)),
        ))
        .unwrap_or_else(|e| unreachable!("{}: configure: {e}", lane.name));
    let wav_format = wav
        .format()
        .unwrap_or_else(|| unreachable!("{}: format present", lane.name));
    assert_eq!(
        u32::from(wav_format.channels),
        lane.channels,
        "{}: channels",
        lane.name
    );
    assert_eq!(wav_format.samples_per_sec, lane.rate, "{}: rate", lane.name);
    let mut sink = CountingSink::default();
    loop {
        match wav
            .next_event()
            .unwrap_or_else(|e| unreachable!("{}: event: {e}", lane.name))
        {
            DemuxEvent::Packet(packet) => {
                let outcome = decoder
                    .decode(packet, &mut sink)
                    .unwrap_or_else(|e| unreachable!("{}: decode: {e}", lane.name));
                assert!(
                    matches!(outcome, DecodeOutcome::Emitted { frames: 1 }),
                    "{}: clean file, every block emits (got {outcome:?})",
                    lane.name
                );
            }
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => continue,
            DemuxEvent::EndOfStream => break,
        }
    }
    sink.inter_channel
}

#[test]
#[ignore = "6.ADPCM-bench throughput measurement: cargo test -p codecs --release --test adpcm_bench -- --ignored (number in the 6.ADPCM-bench worklog entry + the 5.R1 row it closes)"]
fn adpcm_decode_throughput_corpus_lanes() {
    let Some(media) = corpus_media_dir() else {
        eprintln!(
            "corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP \
             (the 5.R1 ADPCM row stays DNF)"
        );
        return;
    };
    for lane in LANES {
        let path = media.join(lane.name);
        // Warm-up + correctness fact: the exact inter-channel extent
        // the differential's MD5 pins (frames × channels).
        let expected = lane.frames * u64::from(lane.channels);
        let got = decode_pass(&lane, &path);
        assert_eq!(
            got, expected,
            "{}: decoded inter-channel samples (the differential's pinned window)",
            lane.name
        );

        let mut per_sec: Vec<f64> = Vec::with_capacity(REPS);
        for _ in 0..REPS {
            let start = Instant::now();
            let got = decode_pass(&lane, &path);
            let elapsed = start.elapsed().as_secs_f64();
            assert_eq!(
                got, expected,
                "{}: every rep decodes the same window",
                lane.name
            );
            per_sec.push(expected as f64 / elapsed);
        }
        per_sec.sort_by(|a, b| a.total_cmp(b));
        let median = per_sec[REPS / 2];
        let lane_rate = f64::from(lane.channels) * f64::from(lane.rate);
        let realtime = median / lane_rate;
        let seconds = expected as f64 / lane_rate;
        eprintln!(
            "corpus {} ({}, {:.3} kHz {} ch, wav demux incl.): {} inter-channel samples, \
             median {:.0} samples/s = {:.0}x realtime ({:.3}s of audio)",
            lane.name,
            lane.descriptor,
            f64::from(lane.rate) / 1000.0,
            lane.channels,
            expected,
            median,
            realtime,
            seconds,
        );
        // Honest floor (flac_bench style): ADPCM is nibble
        // table-lookup arithmetic; release must sustain at least
        // 1 M/s (debug: sanity only).
        let floor = if cfg!(debug_assertions) {
            50_000.0
        } else {
            1_000_000.0
        };
        assert!(
            median >= floor,
            "{}: {median:.0} samples/s < {floor:.0} samples/s floor (N={REPS})",
            lane.name
        );
    }
}
