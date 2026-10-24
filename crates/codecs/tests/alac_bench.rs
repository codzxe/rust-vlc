//! ALAC decode throughput (card 5.J3a): the 5.A1 `flac_bench.rs`
//! convention — an ignored measurement test whose number goes into
//! the card's worklog entry, never the normal gate run.
//!
//! The measured shape is the **product contract path** (configure +
//! one packet per frame + sink) over the committed stereo-24 fixture
//! (the heaviest committed input: 9 packets, 2 channels × 24-bit) and
//! the stereo-16 fixture — through the full open → MP4-demux → decode
//! pipeline, so decode-only throughput is strictly higher than the
//! reported number.

mod common;

use std::path::{Path, PathBuf};
use std::time::Instant;

use codec_api::prelude::*;
use demux::mp4::Mp4Demuxer;
use media_types::ids::SourceUri;
use media_types::packet::SharedBytes;
use plugin_api::{Access, DemuxEvent, Demuxer};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/alac")
        .join(name)
}

fn open_via_input(
    path: &Path,
) -> Result<Box<dyn plugin_api::SourceStream>, media_types::EngineError> {
    let uri = match SourceUri::parse(&format!("file://{}", path.to_str().unwrap_or(""))) {
        Ok(uri) => uri,
        Err(err) => unreachable!("fixture path is a valid file URI ({path:?}): {err}"),
    };
    input::FileAccess::new().open(&uri)
}

#[derive(Default)]
struct Sink {
    bytes: usize,
    samples: u64,
}

impl FrameSink for Sink {
    fn accept(&mut self, frame: Frame) -> Result<(), media_types::EngineError> {
        match frame {
            Frame::Audio(audio) => {
                let per_interchannel = audio.format.sample.bytes_per_sample() as usize
                    * audio.format.channels as usize;
                self.samples += (audio.samples.len() / per_interchannel) as u64;
                self.bytes += audio.samples.len();
            }
            Frame::Video(_) | Frame::Subtitle(_) => unreachable!("audio only"),
        }
        Ok(())
    }
}

/// Repetitions per measurement (median reported, as in pcm_bench).
const REPS: usize = 7;

#[test]
#[ignore = "5.J3a throughput measurement: cargo test -p codecs --release --ignored (number in the 5.J3a worklog entry)"]
fn alac_decode_throughput_fixtures() {
    for (name, channels) in [("alac_stereo24.m4a", 2usize), ("alac_stereo16.m4a", 2)] {
        let path = fixture_path(name);
        let mut timings = Vec::with_capacity(REPS);
        let mut total_bytes = 0usize;
        let mut total_samples = 0u64;
        for rep in 0..=REPS {
            let started = Instant::now();
            let stream = open_via_input(&path).unwrap_or_else(|e| unreachable!("open: {e}"));
            let mut mp4 = Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("new: {e}"));
            let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
            let cookie = mp4
                .sample_entry_bytes(tracks[0].id)
                .unwrap_or_else(|| unreachable!("sample entry"))
                .to_vec();
            let mut decoder = codecs::audio::alac::AlacDecoder::new();
            decoder
                .configure(&DecoderConfig::new(
                    media_types::ids::CodecDescriptor::new("alac")
                        .unwrap_or_else(|e| unreachable!("{e}")),
                    media_types::GenerationId::FIRST,
                    ThreadBudget::default(),
                    None,
                    Some(SharedBytes::from_vec(cookie)),
                ))
                .unwrap_or_else(|e| unreachable!("configure: {e}"));
            let mut sink = Sink::default();
            loop {
                match mp4
                    .next_event()
                    .unwrap_or_else(|e| unreachable!("event: {e}"))
                {
                    DemuxEvent::Packet(packet) => {
                        let _ = decoder
                            .decode(packet, &mut sink)
                            .unwrap_or_else(|e| unreachable!("{e}"));
                    }
                    DemuxEvent::EndOfStream => break,
                    DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => continue,
                }
            }
            if rep > 0 {
                timings.push(started.elapsed());
            }
            total_bytes = sink.bytes;
            total_samples = sink.samples;
        }
        timings.sort();
        let median = timings[timings.len() / 2];
        let per_frame_samples = total_samples;
        eprintln!(
            "{name}: {total_bytes} PCM bytes ({per_frame_samples} frame-samples × {channels} ch) \
             in median {median:?} — {} MB/s of decoded PCM (full pipeline incl. demux)",
            total_bytes as f64 / median.as_secs_f64() / 1e6
        );
    }
}
