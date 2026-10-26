#![allow(dead_code)]
//! Shared Opus test surface (card 6.M-opus): the **decode-through-ogg**
//! driver (the full L1→L3 composition: `demux::ogg` hands the joined
//! `OpusHead ‖ OpusTags` chain + data packets to the
//! `codecs::audio::opus` contract decoder — no synthetic fixture
//! anywhere in the path), the live-ffmpeg oracle helpers, and the
//! tolerance-class comparator (the admission record's bar: two
//! independent float decoders are not bit-exact; s16-audible
//! equivalence is zero samples beyond one s16 LSB).
//!
//! Test-side verification tooling in the 2.B4 sense: its correctness
//! is pinned by the recorded expectations it is checked against
//! (ffprobe facts, ffmpeg PCM decodes), never trusted blind.

use std::path::PathBuf;

use codec_api::prelude::*;
use codecs::audio::opus::OpusDecoder;
use demux::ogg::OggDemuxer;
use media_types::frame::AudioFormat;
use media_types::{EngineError, Packet, TrackId};
use plugin_api::Demuxer;
use testkit::Must;

/// The corpus root (the flac/mp3/vorbis convention: `LUMEN_CORPUS`
/// override else the sandbox default; `None` → the caller SKIPs with
/// a message — the corpus is /tmp-resident).
pub fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        let path = PathBuf::from(dir);
        return path.is_dir().then_some(path);
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4");
    default.is_dir().then_some(default)
}

/// A frame sink that keeps every emitted audio frame.
#[derive(Default)]
pub struct CaptureSink {
    /// The decoded frames' formats, in emission order.
    pub formats: Vec<AudioFormat>,
    /// The raw sample bytes per frame (parallel to `formats`).
    pub buffers: Vec<Vec<u8>>,
    /// The pts ticks (base 1/48000, the valid timeline) per frame;
    /// `i64::MIN` = Missing.
    pub pts: Vec<i64>,
}

impl FrameSink for CaptureSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            unreachable!("the Opus decoder only emits audio frames: {frame:?}");
        };
        let ticks = match audio.pts {
            media_types::Stamp::Present(ts) => ts.ticks,
            media_types::Stamp::Missing => i64::MIN,
        };
        self.formats.push(audio.format);
        self.buffers.push(audio.samples.as_slice().to_vec());
        self.pts.push(ticks);
        Ok(())
    }
}

/// One decode-through-ogg run's results.
pub struct OggOpusDecode {
    /// The negotiated output format.
    pub format: AudioFormat,
    /// Every emitted frame.
    pub sink: CaptureSink,
    /// Data packets the demuxer handed the decoder.
    pub packets: usize,
    /// The pre-skip the OpusHead declared.
    pub pre_skip: u16,
    /// The raw per-channel sample count the decoder decoded
    /// (pre-skip INCLUDED — the end-trim differential observable).
    pub decoded_raw: u64,
    /// The last container granule (the valid-timeline length).
    pub last_granule: Option<i64>,
}

/// Decode a whole Ogg Opus file through the **real composition**:
/// `demux::ogg` over an in-memory stream → the header packets joined
/// verbatim as `extra_data` (the Ogg host's staging, OPW-ogg) → the
/// codec-api `OpusDecoder` fed every data packet.
pub fn decode_through_ogg(data: &[u8]) -> OggOpusDecode {
    let stream = match demux::ogg::test_support::mem_stream(data) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem stream: {err}"),
    };
    let mut demuxer = match OggDemuxer::new(stream) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("ogg open: {err}"),
    };
    let tracks = match demuxer.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("ogg start: {err}"),
    };
    let track = tracks
        .iter()
        .find(|t| t.codec.as_str() == "opus")
        .unwrap_or_else(|| unreachable!("no opus track: {tracks:?}"));
    let track_id: TrackId = track.id;
    let serial = *demuxer
        .serial_numbers()
        .iter()
        .find(|s| {
            demuxer
                .header_packets(**s)
                .first()
                .is_some_and(|h| h.as_slice().starts_with(b"OpusHead"))
        })
        .unwrap_or_else(|| unreachable!("no opus serial"));
    // L2→L3 seam: the header chain, verbatim, as extra_data (the
    // Ogg host's OpusHead ‖ OpusTags join — matched exactly).
    let extra = {
        let mut joined = Vec::new();
        for packet in demuxer.header_packets(serial) {
            joined.extend_from_slice(packet.as_slice());
        }
        joined
    };
    let mut decoder: Option<OpusDecoder> = None;
    let mut sink = CaptureSink::default();
    let mut packets = 0usize;
    let mut last_granule = None;
    loop {
        match demuxer.next_event().must() {
            plugin_api::DemuxEvent::Packet(packet) => {
                if packet.track != track_id {
                    continue;
                }
                if let media_types::Stamp::Present(ts) = packet.pts {
                    last_granule = Some(ts.ticks);
                }
                packets += 1;
                let packet: Packet = packet;
                if decoder.is_none() {
                    let mut fresh = OpusDecoder::new();
                    match fresh.configure(&DecoderConfig::new(
                        match media_types::ids::CodecDescriptor::new("opus") {
                            Ok(d) => d,
                            Err(e) => unreachable!("{e:?}"),
                        },
                        packet.generation,
                        ThreadBudget::default(),
                        None,
                        Some(media_types::packet::SharedBytes::from_vec(extra.clone())),
                    )) {
                        Ok(()) => {}
                        Err(err) => unreachable!("configure with the demuxer's headers: {err}"),
                    }
                    decoder = Some(fresh);
                }
                let decoder = decoder.as_mut().unwrap_or_else(|| unreachable!("just set"));
                match decoder.decode(packet, &mut sink) {
                    Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
                    Ok(DecodeOutcome::NeedMoreInput) => {}
                    Ok(other) => unreachable!("one packet = at most one frame: {other:?}"),
                    Err(err) => unreachable!("corpus packet must decode: {err}"),
                }
            }
            plugin_api::DemuxEvent::EndOfStream => break,
            _ => continue,
        }
    }
    let decoder = decoder.unwrap_or_else(|| unreachable!("at least one data packet"));
    let format = decoder
        .output_format()
        .unwrap_or_else(|| unreachable!("configured decoder negotiates a format"));
    OggOpusDecode {
        format,
        pre_skip: decoder.pre_skip(),
        decoded_raw: decoder.decoded_total(),
        sink,
        packets,
        last_granule,
    }
}

/// Concatenated decoded PCM of a capture (f32le interleaved).
pub fn concatenated(sink: &CaptureSink) -> Vec<u8> {
    sink.buffers.concat()
}

/// Total per-channel output samples of a run.
pub fn output_samples(run: &OggOpusDecode) -> usize {
    let channels = run.format.channels as usize;
    concatenated(&run.sink).len() / (channels * 4)
}

/// Max per-sample |delta| between two f32le byte streams of equal
/// length (the honest lossy tolerance; 0.0 when identical).
pub fn f32_max_delta(ours: &[u8], theirs: &[u8]) -> f32 {
    let a: Vec<f32> = ours
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_ne_bytes(*b))
        .collect();
    let b: Vec<f32> = theirs
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_ne_bytes(*b))
        .collect();
    a.iter()
        .zip(&b)
        .map(|(x, y)| (*x - *y).abs())
        .fold(0.0f32, f32::max)
}

/// The number of samples differing by more than the tolerance.
pub fn samples_beyond(ours: &[u8], theirs: &[u8], tolerance: f32) -> usize {
    let a: Vec<f32> = ours
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_ne_bytes(*b))
        .collect();
    let b: Vec<f32> = theirs
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_ne_bytes(*b))
        .collect();
    a.iter()
        .zip(&b)
        .filter(|(x, y)| (**x - **y).abs() > tolerance)
        .count()
}

/// Whether an `ffmpeg` binary is reachable (the live oracle).
pub fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// ffmpeg's own decode of `path` as interleaved f32le PCM at the
/// stream's native channel count (the 48 kHz opus rate, `-ar` NOT
/// forced — ffmpeg's opus path is already 48 kHz; the channel order
/// is ffmpeg's NATIVE layout, the house convention our order map
/// targets).
pub fn ffmpeg_decode_f32le(path: &std::path::Path) -> Vec<u8> {
    let out = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a", "-c:a", "pcm_f32le", "-f", "f32le", "-"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap_or_else(|e| unreachable!("ffmpeg spawn: {e}"));
    if !out.status.success() {
        unreachable!(
            "ffmpeg failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out.stdout
}
