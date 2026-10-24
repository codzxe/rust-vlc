//! ADPCM fuzz-lite survival (card 5.J2a +FB): deterministic mutation
//! fuzz over the committed fixtures and (when present) the real
//! corpus's first ADPCM blocks. The contract under hammer: **no
//! panic, no hang, no untyped failure** — every decode call returns
//! `Ok` with a typed outcome (`Emitted`/`Recovered`), and configure
//! failures are typed `EngineError`s. The mutations are a seeded LCG
//! (reproducible, no dev-dependency); the corpus is bounded to the
//! first few blocks per file so the committed inputs stay small (the
//! two committed fixtures are ~6 KiB each, well under budget).

mod common;

use std::path::PathBuf;

use codec_api::prelude::*;
use common::md5_hex;
use media_types::ids::CodecDescriptor;
use media_types::packet::SharedBytes;
use media_types::{GenerationId, TrackId};

use codecs::audio::adpcm::AdpcmDecoder;

/// A tiny deterministic LCG (the fuzz-lite pattern; reproducible
/// without a dev-dependency).
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            self.next_u32() as usize % n
        }
    }
}

/// A sink that concatenates decoded PCM (the decode path is the
/// target; the bytes feed the determinism check).
#[derive(Default)]
struct Sink {
    pcm: Vec<u8>,
}

impl FrameSink for Sink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        if let Frame::Audio(audio) = frame {
            self.pcm.extend_from_slice(audio.samples.as_slice());
        }
        Ok(())
    }
}

/// One (descriptor, fmt-payload) pair extracted from a WAV file's
/// own bytes — the real demuxer's facts, not synthesized.
fn fmt_of(data: &[u8]) -> Option<(String, Vec<u8>)> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
            as usize;
        let body = pos + 8;
        if body + size > data.len() {
            return None;
        }
        if id == b"fmt " && size >= 16 {
            let tag = u16::from_le_bytes([data[body], data[body + 1]]);
            let descriptor = match tag {
                0x0002 => "adpcm_ms",
                0x0011 => "adpcm_ima_wav",
                _ => return None,
            };
            return Some((descriptor.to_string(), data[body..body + size].to_vec()));
        }
        pos = body + size + (size & 1);
    }
    None
}

/// The `data` chunk payload of a WAV (fuzz substrate).
fn data_of(data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 12;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
            as usize;
        let body = pos + 8;
        if body + size > data.len() {
            return None;
        }
        if id == b"data" {
            return Some(data[body..body + size].to_vec());
        }
        pos = body + size + (size & 1);
    }
    None
}

/// The committed fixture inputs (always-on coverage).
fn fixture_inputs() -> Vec<(String, Vec<u8>)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    ["adpcm_ima_mono.wav", "adpcm_ms_mono.wav"]
        .iter()
        .filter_map(|name| {
            std::fs::read(root.join(name))
                .ok()
                .map(|bytes| (name.to_string(), bytes))
        })
        .collect()
}

/// The real corpus's ADPCM files, bounded to the first blocks (the
/// speed-and-budget-conscious slice).
fn corpus_inputs() -> Vec<(String, Vec<u8>)> {
    let dir = std::env::var("LUMEN_CORPUS").map_or_else(
        |_| PathBuf::from("/tmp/my-project/corpus-v4/media"),
        PathBuf::from,
    );
    if !dir.is_dir() {
        eprintln!("corpus(adpcm-fuzz): {dir:?} absent — fixtures only");
        return Vec::new();
    }
    ["0042-audio-only.wav", "0102-audio-only.wav"]
        .iter()
        .filter_map(|name| {
            let full = std::fs::read(dir.join(name)).ok()?;
            // Keep the headers whole + the first 3 blocks of the data
            // chunk, rewriting the chunk's size field so the walkers
            // above still parse the trimmed file (bounded substrate).
            let mut pos = 12;
            while pos + 8 <= full.len() {
                let id = &full[pos..pos + 4];
                let size = u32::from_le_bytes([
                    full[pos + 4],
                    full[pos + 5],
                    full[pos + 6],
                    full[pos + 7],
                ]) as usize;
                let body = pos + 8;
                if body + size > full.len() {
                    return None;
                }
                if id == b"data" {
                    let keep = (3 * 1024).min(size);
                    let mut trimmed = full;
                    trimmed[pos + 4..pos + 8].copy_from_slice(&(keep as u32).to_le_bytes());
                    let end = body + keep + (keep & 1);
                    trimmed.truncate(end);
                    return Some((name.to_string(), trimmed));
                }
                pos = body + size + (size & 1);
            }
            None
        })
        .collect()
}

fn decode_mutation(
    descriptor: &str,
    fmt: &[u8],
    payload: &[u8],
) -> Result<(DecodeOutcome, Vec<u8>), EngineError> {
    let mut decoder = AdpcmDecoder::new();
    decoder.configure(&DecoderConfig::new(
        CodecDescriptor::new(descriptor)?,
        GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(fmt.to_vec())),
    ))?;
    let packet = media_types::Packet {
        track: TrackId::new(1),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(payload.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    };
    let mut sink = Sink::default();
    let outcome = decoder.decode(packet, &mut sink)?;
    Ok((outcome, sink.pcm))
}

#[test]
fn mutated_blocks_survive_with_typed_outcomes() {
    let mut inputs = fixture_inputs();
    inputs.extend(corpus_inputs());
    assert!(!inputs.is_empty(), "at least the fixtures must load");
    let mut rng = Lcg(0x5A17_2026_AD0C);
    let mut decoded_total = 0usize;
    let mut recovered = 0u32;
    for (name, file) in &inputs {
        let Some((descriptor, fmt)) = fmt_of(file) else {
            unreachable!("{name}: fixture fmt parses");
        };
        let Some(data) = data_of(file) else {
            unreachable!("{name}: fixture data parses");
        };
        // The unmutated substrate must be clean (self-check).
        match decode_mutation(&descriptor, &fmt, &data) {
            Ok((DecodeOutcome::Emitted { .. }, pcm)) => {
                decoded_total += pcm.len();
            }
            Ok((outcome, _)) => unreachable!("{name}: clean input must emit, got {outcome:?}"),
            Err(err) => unreachable!("{name}: clean input must decode: {err}"),
        }
        // Mutations: byte flips, truncations, bursts, wild headers.
        for round in 0..512u32 {
            let mut mutated = data.clone();
            match rng.below(4) {
                0 => {
                    // Flip 1..4 bytes anywhere.
                    let flips = 1 + rng.below(4);
                    for _ in 0..flips {
                        let at = rng.below(mutated.len());
                        mutated[at] ^= (rng.next_u32() & 0xFF) as u8;
                    }
                }
                1 => {
                    // Truncate to a random prefix (header damage).
                    let keep = rng.below(mutated.len() + 1);
                    mutated.truncate(keep);
                }
                2 => {
                    // Wild header bytes: predictor/index/delta abuse.
                    let at = rng.below(mutated.len().min(16));
                    let wild = (rng.next_u32() & 0xFF) as u8;
                    mutated[at] = wild;
                    if at + 1 < mutated.len() {
                        mutated[at + 1] ^= 0xFF;
                    }
                }
                _ => {
                    // Burst: a run of one byte value.
                    let start = rng.below(mutated.len());
                    let len = 1 + rng.below(64.min(mutated.len() - start));
                    let fill = (rng.next_u32() & 0xFF) as u8;
                    mutated[start..start + len].fill(fill);
                }
            }
            // The survival contract: Ok with a typed outcome, never a
            // panic, never an Err out of decode (damage is concealed
            // or skipped, both typed).
            match decode_mutation(&descriptor, &fmt, &mutated) {
                Ok((outcome, pcm)) => {
                    decoded_total += pcm.len();
                    if !matches!(
                        outcome,
                        DecodeOutcome::Emitted { .. } | DecodeOutcome::Recovered { .. }
                    ) {
                        unreachable!("{name} round {round}: unexpected outcome {outcome:?}");
                    }
                    if matches!(outcome, DecodeOutcome::Recovered { .. }) {
                        recovered += 1;
                    }
                }
                Err(err) => {
                    unreachable!("{name} round {round}: decode must not Err: {err}");
                }
            }
        }
    }
    // Liveness: the harness actually exercised output and damage.
    assert!(decoded_total > 0, "some mutations must decode bytes");
    assert!(recovered > 0, "some mutations must trigger recovery");
}

#[test]
fn mutated_fmt_headers_fail_typed_not_panicking() {
    let inputs = fixture_inputs();
    assert!(!inputs.is_empty(), "fixtures must load");
    let mut rng = Lcg(0x00C0_FFEE_0001);
    let mut typed_unsupported = 0u32;
    for (name, file) in &inputs {
        let Some((descriptor, fmt)) = fmt_of(file) else {
            unreachable!("{name}: fixture fmt parses");
        };
        let Some(data) = data_of(file) else {
            unreachable!("{name}: fixture data parses");
        };
        for _ in 0..256 {
            let mut mangled = fmt.clone();
            match rng.below(4) {
                0 => mangled.truncate(rng.below(mangled.len() + 1)),
                1 => {
                    let at = rng.below(mangled.len());
                    mangled[at] = (rng.next_u32() & 0xFF) as u8;
                }
                2 => {
                    // Wild tag / channels / block align.
                    let at = rng.below(16);
                    mangled[at] = (rng.next_u32() & 0xFF) as u8;
                    if at < 16 && mangled.len() > at + 1 {
                        mangled[at + 1] ^= 0x5A;
                    }
                }
                _ => mangled.clear(),
            }
            match decode_mutation(&descriptor, &mangled, &data) {
                Ok(_) => continue, // a still-valid header is fine
                Err(err) => {
                    assert!(
                        matches!(err.kind, media_types::ErrorKind::Unsupported { .. }),
                        "{name}: configure damage must be Unsupported, got {:?}",
                        err.kind
                    );
                    typed_unsupported += 1;
                }
            }
        }
    }
    assert!(typed_unsupported > 0, "some fmt mutations must be refused");
}

#[test]
fn empty_and_tiny_payloads_are_honest() {
    let inputs = fixture_inputs();
    assert!(!inputs.is_empty(), "fixtures must load");
    for (name, file) in &inputs {
        let Some((descriptor, fmt)) = fmt_of(file) else {
            unreachable!("{name}: fixture fmt parses");
        };
        // Empty payload: NeedMoreInput, nothing decoded.
        match decode_mutation(&descriptor, &fmt, &[]) {
            Ok((DecodeOutcome::NeedMoreInput, ref pcm)) if pcm.is_empty() => {}
            Ok((outcome, pcm)) => unreachable!("{name}: empty → {outcome:?}, {} bytes", pcm.len()),
            Err(err) => unreachable!("{name}: empty must not Err: {err}"),
        }
        // One stray byte: skipped (shorter than any header), typed.
        match decode_mutation(&descriptor, &fmt, &[0x42]) {
            Ok((DecodeOutcome::Recovered { .. }, ref pcm)) if pcm.is_empty() => {}
            Ok((outcome, pcm)) => unreachable!("{name}: stray → {outcome:?}, {} bytes", pcm.len()),
            Err(err) => unreachable!("{name}: stray must not Err: {err}"),
        }
    }
}

#[test]
fn unmutated_substrate_decodes_deterministically() {
    // Guards the fuzz substrate itself: the same bytes decode to the
    // same PCM every run (the pooled-slot rotation must not leak
    // state between packets or runs).
    let mut inputs = fixture_inputs();
    inputs.extend(corpus_inputs());
    for (name, file) in &inputs {
        let Some((descriptor, fmt)) = fmt_of(file) else {
            continue;
        };
        let Some(data) = data_of(file) else {
            continue;
        };
        let mut first: Option<String> = None;
        for _ in 0..3 {
            let (outcome, pcm) = match decode_mutation(&descriptor, &fmt, &data) {
                Ok(pair) => pair,
                Err(err) => unreachable!("{name}: {err}"),
            };
            assert!(matches!(outcome, DecodeOutcome::Emitted { frames: 1 }));
            let digest = md5_hex(&pcm);
            match &first {
                None => first = Some(digest),
                Some(pinned) => assert_eq!(&digest, pinned, "{name}: decode is deterministic"),
            }
        }
    }
}
