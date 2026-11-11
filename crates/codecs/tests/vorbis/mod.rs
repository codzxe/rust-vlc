#![allow(dead_code)]
//! Shared Vorbis test surface (card 5.B2): the crate-local fixture
//! loader, the reference-data parser, and the **decode-through-ogg**
//! driver (the full L1→L3 composition: `demux::ogg` hands header
//! packets + data packets to the `codecs::audio::vorbis` contract
//! decoder — no synthetic fixture anywhere in the path).
//!
//! This module is a *module* (`tests/vorbis/`), deliberately not a
//! top-level `tests/*.rs` file (cargo would auto-discover it as its
//! own empty test binary — the flac/mp3 precedent). Test-side
//! verification tooling in the 2.B4 sense: its correctness is pinned
//! by the recorded expectations it is checked against (ffprobe facts,
//! ffmpeg PCM digests), never trusted blind.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use codec_api::prelude::*;
use codecs::audio::vorbis::{VorbisDecoder, VorbisHeaders};
use demux::ogg::OggDemuxer;
use media_types::frame::AudioFormat;
use media_types::{EngineError, Packet, TrackId};
use plugin_api::Demuxer;
use testkit::Must;

/// The crate-local fixture directory.
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Read a committed fixture (panics: fixtures are committed bytes —
/// absence is a repo-state error, not a test verdict).
pub fn fixture_bytes(name: &str) -> Vec<u8> {
    match std::fs::read(fixture_path(name)) {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("committed fixture {name}: {err}"),
    }
}

/// The reference-data file path.
fn reference_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/vorbis_reference.txt")
}

/// One `[section]` of the reference data: `key = value` pairs.
type Section = HashMap<String, String>;

/// Parsed reference data (lazily, once per test binary).
pub fn reference() -> &'static HashMap<String, Section> {
    static REF: OnceLock<HashMap<String, Section>> = OnceLock::new();
    REF.get_or_init(
        || match parse_reference(&std::fs::read(reference_path()).must()) {
            Ok(map) => map,
            Err(err) => unreachable!("reference data must parse: {err}"),
        },
    )
}

/// The `fixture.<name>` section (panics: recorded expectations are
/// part of the committed fixture set).
pub fn fixture_reference(name: &str) -> &'static Section {
    reference_section(&format!("fixture.{name}"))
}

/// Any `[section]` of the reference data by its full name (e.g.
/// `corpus.0085-audio-only`).
pub fn reference_section(name: &str) -> &'static Section {
    match reference().get(name) {
        Some(section) => section,
        None => unreachable!("no [{name}] section in the reference data"),
    }
}

/// Parse the ini-like reference format (`[section]` + `key = value`,
/// `#` comments; values kept verbatim).
fn parse_reference(data: &[u8]) -> Result<HashMap<String, Section>, String> {
    let text = std::str::from_utf8(data).map_err(|e| e.to_string())?;
    let mut map: HashMap<String, Section> = HashMap::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[') {
            let name = name.strip_suffix(']').ok_or("unterminated [section]")?;
            current = name.to_string();
            map.entry(current.clone()).or_default();
            continue;
        }
        let (key, value) = line.split_once('=').ok_or("expected key = value")?;
        let section = map
            .get_mut(&current)
            .ok_or_else(|| format!("key outside a section: {key}"))?;
        section.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(map)
}

/// The corpus root (the flac/mp3 convention: `LUMEN_CORPUS` override
/// else the sandbox default; `None` → the caller SKIPs with a
/// message — the corpus is /tmp-resident).
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
    /// The pts ticks (base 1/rate) per frame; `i64::MIN` = Missing.
    pub pts: Vec<i64>,
}

impl FrameSink for CaptureSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            unreachable!("the Vorbis decoder only emits audio frames: {frame:?}");
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
pub struct OggDecode {
    /// The parsed header chain (from the demuxer's header packets).
    pub headers: VorbisHeaders,
    /// Every emitted frame.
    pub sink: CaptureSink,
    /// Data packets the demuxer handed the decoder.
    pub packets: usize,
    /// The last container stamp seen (the final granule), when any.
    pub last_granule: Option<i64>,
}

/// Decode a whole Ogg Vorbis file through the **real composition**:
/// `demux::ogg` over an in-memory stream → header packets joined as
/// `extra_data` → the codec-api `VorbisDecoder` fed every data packet.
pub fn decode_through_ogg(data: &[u8]) -> OggDecode {
    // L1: the demuxer over the bytes (the mem-stream seam the ogg
    // module's own tests use).
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
        .find(|t| t.codec.as_str() == "vorbis")
        .unwrap_or_else(|| unreachable!("no vorbis track: {tracks:?}"));
    let track_id: TrackId = track.id;
    let serial = *demuxer
        .serial_numbers()
        .first()
        .unwrap_or_else(|| unreachable!("no serials"));
    // L2→L3 seam: the header packets, verbatim, as extra_data.
    let headers = match VorbisHeaders::from_packets(
        demuxer.header_packets(serial).iter().map(|p| p.as_slice()),
    ) {
        Ok(headers) => headers,
        Err(err) => unreachable!("header chain: {err}"),
    };
    // The decoder is configured with the header chain AND the
    // generation the demuxer actually stamps its packets with (the
    // first packet's own generation — the composition must not guess
    // a fresh one and mark the whole stream stale).
    let mut decoder: Option<VorbisDecoder> = None;
    let extra = {
        let mut joined = Vec::new();
        for packet in demuxer.header_packets(serial) {
            joined.extend_from_slice(packet.as_slice());
        }
        joined
    };
    // The decode loop: every data packet through the contract path.
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
                    let mut fresh = VorbisDecoder::new();
                    match fresh.configure(&DecoderConfig::new(
                        match media_types::ids::CodecDescriptor::new("vorbis") {
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
                    // The head-hold (6.FIX-vorbis-headtrim) makes the
                    // per-packet frame count stream-shaped: 0 while the
                    // head is withheld, `released + 1` on the release
                    // packet, 1 in steady state — any count is the
                    // contract's `Emitted`.
                    Ok(DecodeOutcome::Emitted { .. }) => {}
                    // A zero-length packet is legal Ogg (and the
                    // silence fixture carries them): consumed,
                    // nothing decodable.
                    Ok(DecodeOutcome::NeedMoreInput) => {}
                    Ok(other) => unreachable!("packet decode: {other:?}"),
                    Err(err) => unreachable!("fixture/corpus packet must decode: {err}"),
                }
            }
            plugin_api::DemuxEvent::EndOfStream => break,
            _ => continue,
        }
    }
    if let Some(decoder) = decoder.as_mut() {
        match decoder.flush(&mut sink) {
            Ok(DecodeOutcome::Drained) => {}
            Ok(other) => unreachable!("flush: {other:?}"),
            Err(err) => unreachable!("flush: {err}"),
        }
    }
    OggDecode {
        headers,
        sink,
        packets,
        last_granule,
    }
}

/// Concatenated decoded PCM of a capture.
pub fn concatenated(sink: &CaptureSink) -> Vec<u8> {
    sink.buffers.concat()
}

/// The valid-timeline window of a decode run: the head trim is the
/// decoder-measured timeline shift (the first stamped page's raw−granule
/// distance — recoverable from the run as `total − last_granule −
/// tail_excess`… but the honest direct observable is the pts series:
/// the first frame whose pts is present defines the shift against its
/// raw position). Here we derive it from the FIRST present pts: the
/// raw emission is `frames[0..]`; the shift = `0 − pts[0_present]`
/// adjusted by that frame's raw position.
pub fn window_at(sink: &CaptureSink, channels: usize, head_trim: usize, samples: usize) -> Vec<u8> {
    // 6.FIX-vorbis-headtrim: the decoder now trims the stream head
    // itself (the withheld head is released minus the measured encoder
    // delay, first Present pts at 0), so the harness-side trim is
    // DERIVED from the emission: the first Present pts frames names
    // the raw→valid shift of whatever is actually in the buffers.
    // The `head_trim` argument remains the FALLBACK for the honestly
    // un-trimmed streams (single-data-page encodes released by flush
    // with an all-Missing pts series — the pinned pre-fix shape).
    let mut raw_pos = 0usize;
    let mut derived: Option<i64> = None;
    for (index, ticks) in sink.pts.iter().enumerate() {
        if *ticks != i64::MIN {
            derived = Some(raw_pos as i64 - ticks);
            break;
        }
        raw_pos += sink.buffers[index].len() / (channels * 4);
    }
    let trim = match derived {
        Some(shift) => shift.max(0) as usize,
        None => head_trim,
    };
    let pcm = concatenated(sink);
    let start = trim * channels * 4;
    let end = (start + samples * channels * 4).min(pcm.len());
    pcm.get(start..end).unwrap_or(&[]).to_vec()
}

pub fn window(sink: &CaptureSink, channels: usize, ffmpeg_samples: usize) -> Vec<u8> {
    // Raw timeline: cumulative samples before each frame.
    let mut raw_pos = 0usize;
    let mut shift = 0i64;
    for (index, ticks) in sink.pts.iter().enumerate() {
        if *ticks != i64::MIN {
            // This frame's raw start is raw_pos; the container says
            // its valid start is ticks.
            shift = raw_pos as i64 - ticks;
            let _ = index;
            break;
        }
        raw_pos += sink.buffers[index].len() / (channels * 4);
    }
    let pcm = concatenated(sink);
    let start = shift.max(0) as usize * channels * 4;
    let end = (start + ffmpeg_samples * channels * 4).min(pcm.len());
    pcm.get(start..end).unwrap_or(&[]).to_vec()
}

/// MD5 over f32le bytes with `-0.0` canonicalized to `+0.0` (the MDCT
/// emits `-0.0` on digital-silence lanes where ffmpeg emits `+0.0`;
/// same value, different bits — the pin digests are over the
/// canonicalized form, the reference data explains why).
pub fn canonical_f32_md5(bytes: &[u8]) -> String {
    let mut canon = bytes.to_vec();
    for chunk in canon.as_chunks_mut::<4>().0 {
        if *chunk == [0, 0, 0, 0x80] {
            chunk.fill(0);
        }
    }
    crate::common::md5_hex(&canon)
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

/// Whether an `ffmpeg` binary is reachable (the live oracle).
pub fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// ffmpeg's own decode of `path` as interleaved f32le PCM.
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
