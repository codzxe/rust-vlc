#![allow(dead_code)]
//! Shared FLAC test surface (card 5.A1): the crate-local fixture
//! loader, the reference-data parser, the frame walker, and the
//! contract-path decode-all driver used by the `flac_*.rs` suites.
//!
//! This module is a *module* (`tests/flac/`), deliberately not a
//! top-level `tests/*.rs` file: cargo would auto-discover such a file
//! as its own (empty) test binary. The repo's `tests/common/mod.rs`
//! precedent is followed for path resolution (including its
//! module-level `allow(dead_code)`: each test binary compiles this
//! module and not every helper is used by every binary); that shared
//! file stays untouched (outside this card's ownership).
//!
//! Everything here is **test-side verification tooling** in the 2.B4
//! sense (`extract_avi_mjpeg_frames`): its own correctness is pinned
//! by the recorded expectations it is checked against (ffprobe frame
//! counts, packet pts, ffmpeg PCM digests), never trusted blind.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use codec_api::prelude::*;
use codecs::audio::flac::{FlacDecoder, StreamInfo, StreamLayout, frame_header_at, parse_stream};
use media_types::frame::{AudioFormat, Frame};
use media_types::{EngineError, GenerationId, Packet};
use testkit::Must;

/// The crate-local fixture directory (committed, budget-checked:
/// 130 316 bytes total — the card's < 200 KB mandate).
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/flac_bitexact_reference.txt")
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
    match reference().get(&format!("fixture.{name}")) {
        Some(section) => section,
        None => unreachable!("no [fixture.{name}] section in the reference data"),
    }
}

/// Parse the ini-like reference format (`[section]` + `key = value`,
/// `#` comments). Values are kept verbatim (strings); typed accessors
/// below do the converting.
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

/// A `key = a,b,c` integer list (empty when the key is absent).
pub fn reference_list(section: &Section, key: &str) -> Vec<u64> {
    section
        .get(key)
        .map(|value| {
            value
                .split(',')
                .filter(|part| !part.is_empty())
                .map(|part| part.trim().parse::<u64>().must())
                .collect()
        })
        .unwrap_or_default()
}

/// The corpus root: the `LUMEN_CORPUS` env override when set, else the
/// sandbox's known default. `None` (→ the caller SKIPs with a message)
/// when neither exists — the corpus is /tmp-resident and may be
/// re-imaged away.
pub fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        let path = PathBuf::from(dir);
        return path.is_dir().then_some(path);
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4");
    default.is_dir().then_some(default)
}

/// The verbatim 34-byte STREAMINFO body of a native FLAC stream (the
/// `extra_data` the contract path configures with — the container's
/// own bytes, not a re-serialization).
pub fn streaminfo_body(data: &[u8]) -> Vec<u8> {
    let mut pos = 4usize; // past "fLaC"
    loop {
        let header = data.get(pos..pos + 4).must();
        let block_type = header[0] & 0x7f;
        let length =
            usize::try_from(u32::from_be_bytes([0, header[1], header[2], header[3]])).must();
        let body = data.get(pos + 4..pos + 4 + length).must();
        if block_type == 0 {
            return body.to_vec();
        }
        pos += 4 + length;
        if header[0] & 0x80 != 0 {
            unreachable!("STREAMINFO is always the first metadata block");
        }
    }
}

/// One walked frame: its containing byte range and navigation facts.
pub struct WalkedFrame {
    /// Absolute byte offset of the frame header.
    pub offset: u64,
    /// Bytes from `offset` to the next frame header (or EOF).
    pub packet: Vec<u8>,
    /// First inter-channel sample (the spec fixed-blocksize conversion).
    pub first_sample: u64,
    /// Block size in inter-channel samples.
    pub block_size: u32,
}

/// Walk the frame stream: scan for CRC-8-valid headers from
/// `frames_start`, each frame's packet running to the next valid
/// header (the last to EOF). The walker is the module's own parser
/// (spec §6 field order + CRC-8); its split is pinned externally by
/// the recorded ffprobe frame counts and the byte-exact PCM comparison
/// against ffmpeg's decode — a mis-split fails both, loudly.
pub fn walk_frames(data: &[u8], facts: &StreamInfo, frames_start: u64) -> Vec<WalkedFrame> {
    let mut starts: Vec<(u64, u64, u32)> = Vec::new(); // (offset, first_sample, block_size)
    let mut offset = frames_start;
    let end = data.len() as u64;
    while offset < end {
        if let Some(header) = frame_header_at(data, offset) {
            starts.push((offset, header.first_sample(facts), header.block_size));
        }
        offset += 1;
    }
    starts
        .iter()
        .enumerate()
        .map(|(index, (offset, first_sample, block_size))| {
            let next = starts.get(index + 1).map_or(end, |(next, _, _)| *next);
            WalkedFrame {
                offset: *offset,
                packet: data[*offset as usize..next as usize].to_vec(),
                first_sample: *first_sample,
                block_size: *block_size,
            }
        })
        .collect()
}

/// A frame sink that keeps every emitted audio frame.
#[derive(Default)]
pub struct CaptureSink {
    /// The decoded frames' formats, in emission order.
    pub formats: Vec<AudioFormat>,
    /// The raw sample bytes per frame (parallel to `formats`).
    pub buffers: Vec<Vec<u8>>,
    /// The first-sample (`pts` ticks, base 1/rate) values per frame.
    pub first_samples: Vec<u64>,
}

impl FrameSink for CaptureSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            unreachable!("the FLAC decoder only emits audio frames: {frame:?}");
        };
        let first_sample = match audio.pts {
            media_types::Stamp::Present(ts) => u64::try_from(ts.ticks).unwrap_or(u64::MAX),
            media_types::Stamp::Missing => u64::MAX,
        };
        self.formats.push(audio.format);
        self.buffers.push(audio.samples.as_slice().to_vec());
        self.first_samples.push(first_sample);
        Ok(())
    }
}

/// Decode a whole native FLAC stream through the **contract path**
/// (configure with the verbatim STREAMINFO, one packet per walked
/// frame, sink capture): the product surface end to end.
pub fn decode_all(data: &[u8]) -> (StreamLayout, CaptureSink) {
    decode_all_from_scan(data, None)
}

/// The seek-shape of [`decode_all`]: decode only the frames found at
/// or after `scan_start` (a seek plan's `byte_offset`) — the decoded
/// bytes must equal the full decode's suffix from that frame's first
/// sample, which is exactly what the seek suite asserts.
pub fn decode_all_from(data: &[u8], scan_start: u64) -> (StreamLayout, CaptureSink) {
    decode_all_from_scan(data, Some(scan_start))
}

/// Shared driver for the two decode-all shapes (`scan_start` `None`
/// = the metadata walk's own `frames_start`).
fn decode_all_from_scan(data: &[u8], scan_start: Option<u64>) -> (StreamLayout, CaptureSink) {
    let layout = match parse_stream(data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("fixture/corpus walk: {err}"),
    };
    let scan = scan_start.unwrap_or(layout.frames_start);
    let walked = walk_frames(data, &layout.stream_info, scan);
    let descriptor = match media_types::ids::CodecDescriptor::new("flac") {
        Ok(d) => d,
        Err(err) => unreachable!("{err}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(err) => unreachable!("{err}"),
    };
    let mut decoder = FlacDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(media_types::packet::SharedBytes::from_vec(streaminfo_body(
            data,
        ))),
    )) {
        Ok(()) => {}
        Err(err) => unreachable!("configure with valid streaminfo: {err}"),
    }
    let mut sink = CaptureSink::default();
    for frame in &walked {
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: media_types::packet::SharedBytes::from_vec(frame.packet.clone()),
            keyframe: true,
            discontinuity: false,
            generation,
        };
        match decoder.decode(packet, &mut sink) {
            Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
            Ok(other) => unreachable!("one packet = one frame: {other:?}"),
            Err(err) => unreachable!("fixture/corpus frame must decode: {err}"),
        }
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        Ok(other) => unreachable!("flush: {other:?}"),
        Err(err) => unreachable!("flush: {err}"),
    }
    (layout, sink)
}

/// Concatenated decoded PCM of a capture.
pub fn concatenated(sink: &CaptureSink) -> Vec<u8> {
    sink.buffers.concat()
}

/// Lift packed 3-byte samples (the decoder's S24 output, native
/// endian) into left-justified 4-byte little-endian words — ffmpeg's
/// `s32le` decode of 24-bit FLAC (the comparison base).
pub fn lift_s24_to_s32le(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() / 3 * 4);
    for triple in bytes.as_chunks::<3>().0 {
        // Native-endian 3-byte value → i32, then << 8, then LE bytes.
        let value = if cfg!(target_endian = "big") {
            (i32::from(triple[0]) << 16) | (i32::from(triple[1]) << 8) | i32::from(triple[2])
        } else {
            (i32::from(triple[2]) << 16) | (i32::from(triple[1]) << 8) | i32::from(triple[0])
        };
        out.extend_from_slice(&(value << 8).to_le_bytes());
    }
    out
}

/// The decoder's output bytes recoded to **little-endian at the
/// stream's native sample width** — the FLAC STREAMINFO-MD5 form
/// (16-bit and 24-bit outputs are already LE on an LE host; a
/// big-endian host reorders, hence the cfg). `width` is 2, 3 or 4.
pub fn to_native_le(bytes: &[u8], width: usize) -> Vec<u8> {
    if cfg!(target_endian = "little") || width < 2 {
        return bytes.to_vec();
    }
    let swap = |group: &[u8]| -> Vec<u8> { group.iter().rev().copied().collect() };
    match width {
        2 => bytes
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|g| swap(g))
            .collect(),
        3 => bytes
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|g| swap(g))
            .collect(),
        4 => bytes
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|g| swap(g))
            .collect(),
        _ => bytes.to_vec(),
    }
}
