//! ADPCM breadth differential suite (card 5.J2b AC): the committed
//! generated corpus vs the ffmpeg 7.1.5 reference — **bit-exact PCM
//! MD5** on every flavor/channel path (doc 04 §3.1 rows, the 2.B4
//! "reference digest" precedent carried to audio).
//!
//! # Corpus provenance (every byte regenerated and re-measured this
//! session, ffmpeg 7.1.5-0+deb13u1 on Debian 13)
//!
//! Reference MD5s are `ffmpeg -map 0:a -c:a pcm_s16le -f md5 -` of
//! the reference decode (the md5 muxer hashes the decoder's s16le
//! output frames; our decoder emits interleaved native-endian s16,
//! identical bytes on the little-endian build host — asserted by the
//! match itself).
//!
//! | case | fixture | packets | authored by | oracle path | reference MD5 |
//! |---|---|---|---|---|---|
//! | `adpcm_yamaha` mono 22 050 | `yamaha_mono_payload.bin` | 18 × 1024 | ffmpeg enc, NUT-carried (`-c:a adpcm_yamaha`) | NUT demux → dec → s16le | `aeddc262b3dd010955f9c73c6cfc86ad` |
//! | `adpcm_yamaha` stereo 22 050 | `yamaha_st_payload.bin` | 35 × 1024 | ffmpeg enc, NUT-carried | same | `fcf6aecad5c50e142a74235c43624f55` |
//! | `adpcm_swf` stereo 22 050 | `swf_payload.bin` | 9 × 4101 | ffmpeg enc, FLV-carried (`-c:a adpcm_swf`) | FLV demux → dec → s16le | `9aac930510b6ef482b7303db62a7b200` |
//! | `adpcm_ima_qt` stereo 22 050 | `qt_payload.bin` | 173 × 68 | ffmpeg enc, MOV-carried (`-c:a adpcm_ima_qt`) | MOV demux → dec → s16le | `a2cb46ce5b9ad2da111403ad6195bf3b` |
//! | `adpcm_ima_oki` mono 8 000 | `oki_mono.vox` | 1 × 512 | hand-crafted ramp (no ffmpeg encoder exists) | **forced decoder** `-c:a adpcm_ima_oki -f mulaw -ar 8000 -ac 1` | `60714fbfc3997f823b7b97dd90e958ed` |
//! | `adpcm_ima_oki` stereo 8 000 | `oki_st.vox` | 1 × 512 | hand-crafted ramp | forced decoder, `-ac 2` | `bad7f93b285386ba34d2e30ec6d6e5f3` |
//! | `adpcm_ima_ws` mono 22 050 | `ws_mono.bin` | 1 × 512 | hand-crafted ramp (encoder exists, no muxer carries it) | forced decoder `-c:a adpcm_ima_ws -f mulaw -ar 22050 -ac 1` | `cbbfd8928ee6a5b2c019330d3d5ff7fb` |
//! | `adpcm_ima_ws` stereo 22 050 | `ws_st.bin` | 1 × 512 | hand-crafted ramp | forced decoder, `-ac 2` | `ec91fc42a66bee03edf43a5e91b80932` |
//!
//! Payloads were extracted with `-map 0:a -c:a copy -f data`, packet
//! sizes with `ffprobe -show_entries packet=size` (the committed
//! `*_sizes.json`); the forced-decoder cases ride the `mulaw` raw
//! demuxer purely as a byte carrier (its 512-byte file is a single
//! packet — verified with ffprobe — and both flavors are continuous,
//! so packetization cannot change the decode).
//!
//! The lettered `adpcm_oki` 32k/24k variants have **no ffmpeg 7.1.5
//! encoder or decoder** (checked `-encoders`/`-decoders` this
//! session) — the typed `Unsupported` gate in the module is the
//! honest posture, covered by the unit tests there.

use codec_api::prelude::*;
use codecs::audio::adpcm::AdpcmDecoder;
use media_types::packet::SharedBytes;
use media_types::{EngineError, GenerationId, Stamp, TrackId};

/// One differential case: descriptor + fixture + geometry + the
/// reference MD5 measured this session (see the table above).
struct Case {
    descriptor: &'static str,
    payload: &'static str,
    sizes: Option<&'static str>,
    channels: u16,
    rate: u32,
    expected_md5: &'static str,
}

const CASES: &[Case] = &[
    Case {
        descriptor: "adpcm_yamaha",
        payload: "yamaha_mono_payload.bin",
        sizes: Some("yamaha_mono_sizes.json"),
        channels: 1,
        rate: 22_050,
        expected_md5: "aeddc262b3dd010955f9c73c6cfc86ad",
    },
    Case {
        descriptor: "adpcm_yamaha",
        payload: "yamaha_st_payload.bin",
        sizes: Some("yamaha_st_sizes.json"),
        channels: 2,
        rate: 22_050,
        expected_md5: "fcf6aecad5c50e142a74235c43624f55",
    },
    Case {
        descriptor: "adpcm_swf",
        payload: "swf_payload.bin",
        sizes: Some("swf_sizes.json"),
        channels: 2,
        rate: 22_050,
        expected_md5: "9aac930510b6ef482b7303db62a7b200",
    },
    Case {
        descriptor: "adpcm_ima_qt",
        payload: "qt_payload.bin",
        sizes: Some("qt_sizes.json"),
        channels: 2,
        rate: 22_050,
        expected_md5: "a2cb46ce5b9ad2da111403ad6195bf3b",
    },
    Case {
        descriptor: "adpcm_ima_oki",
        payload: "oki_mono.vox",
        sizes: None,
        channels: 1,
        rate: 8_000,
        expected_md5: "60714fbfc3997f823b7b97dd90e958ed",
    },
    Case {
        descriptor: "adpcm_ima_oki",
        payload: "oki_st.vox",
        sizes: None,
        channels: 2,
        rate: 8_000,
        expected_md5: "bad7f93b285386ba34d2e30ec6d6e5f3",
    },
    Case {
        descriptor: "adpcm_ima_ws",
        payload: "ws_mono.bin",
        sizes: None,
        channels: 1,
        rate: 22_050,
        expected_md5: "cbbfd8928ee6a5b2c019330d3d5ff7fb",
    },
    Case {
        descriptor: "adpcm_ima_ws",
        payload: "ws_st.bin",
        sizes: None,
        channels: 2,
        rate: 22_050,
        expected_md5: "ec91fc42a66bee03edf43a5e91b80932",
    },
];

/// The breadth fixture directory (crate-local: `tests/fixtures/`
/// under this crate's manifest — deliberately NOT the workspace
/// corpus root, which is governed by its MANIFEST/CORPUS walk).
fn fixture_bytes(name: &str) -> Result<Vec<u8>, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/breadth")
        .join(name);
    std::fs::read(&path).map_err(|err| format!("breadth fixture {name}: {err}"))
}

/// Parse the committed sizes JSON (a flat `[n, n, …]` array; the
/// ffprobe `packet=size` capture needs no real JSON machinery).
fn parse_sizes(json: &[u8]) -> Result<Vec<usize>, String> {
    let text = match std::str::from_utf8(json) {
        Ok(text) => text,
        Err(err) => return Err(format!("sizes json is not utf-8: {err}")),
    };
    let body = text.trim().trim_start_matches('[').trim_end_matches(']');
    body.split(',')
        .map(|entry| {
            entry
                .trim()
                .parse::<usize>()
                .map_err(|err| format!("size entry `{entry}`: {err}"))
        })
        .collect()
}

/// The 16-byte raw-facts skeleton (WAVEFORMATEX with `wFormatTag 0`,
/// the module's documented channel/rate carrier).
fn raw_facts(channels: u16, rate: u32) -> Vec<u8> {
    let mut facts = Vec::with_capacity(16);
    facts.extend_from_slice(&0u16.to_le_bytes());
    facts.extend_from_slice(&channels.to_le_bytes());
    facts.extend_from_slice(&rate.to_le_bytes());
    facts.extend_from_slice(&0u32.to_le_bytes());
    facts.extend_from_slice(&0u16.to_le_bytes());
    facts.extend_from_slice(&4u16.to_le_bytes());
    facts
}

/// A capture sink collecting audio frames (the module-test shape).
#[derive(Default)]
struct Capture {
    pcm: Vec<u8>,
    frames: usize,
}

impl FrameSink for Capture {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            unreachable!("breadth decode must emit audio frames only");
        };
        self.pcm.extend_from_slice(audio.samples.as_slice());
        self.frames += 1;
        Ok(())
    }
}

/// A decode packet with the given payload bytes.
fn packet(generation: GenerationId, data: &[u8]) -> Packet {
    Packet {
        track: TrackId::new(0),
        pts: Stamp::Missing,
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(data.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// Decode one case's payload through the contract path and return the
/// concatenated PCM (plus per-packet outcomes are asserted Emitted —
/// the corpus is clean by construction).
fn decode_case(case: &Case) -> Vec<u8> {
    let payload = match fixture_bytes(case.payload) {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("{err}"),
    };
    let sizes = match case.sizes {
        Some(name) => {
            let json = match fixture_bytes(name) {
                Ok(bytes) => bytes,
                Err(err) => unreachable!("{err}"),
            };
            match parse_sizes(&json) {
                Ok(sizes) => sizes,
                Err(err) => unreachable!("{err}"),
            }
        }
        // The forced-decoder ramps: one whole-file packet (ffprobe
        // verified the raw demuxer's packetization).
        None => vec![payload.len()],
    };
    assert_eq!(
        sizes.iter().sum::<usize>(),
        payload.len(),
        "{}: sizes must tile the payload",
        case.payload
    );
    let mut decoder = AdpcmDecoder::new();
    let generation = GenerationId::new(7);
    let config = DecoderConfig::new(
        match media_types::ids::CodecDescriptor::new(case.descriptor) {
            Ok(descriptor) => descriptor,
            Err(err) => unreachable!("descriptor: {err}"),
        },
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(raw_facts(case.channels, case.rate))),
    );
    match decoder.configure(&config) {
        Ok(()) => {}
        Err(err) => unreachable!("breadth configure: {err}"),
    }
    let mut capture = Capture::default();
    let mut offset = 0usize;
    for (index, &size) in sizes.iter().enumerate() {
        let outcome = match decoder.decode(
            packet(generation, &payload[offset..offset + size]),
            &mut capture,
        ) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("corpus decode: {err}"),
        };
        assert_eq!(
            outcome,
            DecodeOutcome::Emitted { frames: 1 },
            "{} packet {index}",
            case.payload
        );
        offset += size;
    }
    assert_eq!(
        match decoder.flush(&mut Capture::default()) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("flush: {err}"),
        },
        DecodeOutcome::Drained
    );
    assert_eq!(capture.frames, sizes.len(), "{}: frame count", case.payload);
    capture.pcm
}

#[test]
fn breadth_corpus_is_bit_exact_against_ffmpeg_reference() {
    for case in CASES {
        let pcm = decode_case(case);
        // Format sanity rides along: the whole corpus is interleaved
        // s16 at the facts geometry (asserted once per case here, the
        // module tests cover the rest).
        assert!(
            pcm.len().is_multiple_of(usize::from(case.channels) * 2),
            "{}: pcm must be whole sample instants",
            case.payload
        );
        let digest = md5_hex(&pcm);
        assert_eq!(
            digest, case.expected_md5,
            "{} ({}, {}ch): PCM MD5 vs ffmpeg 7.1.5 reference — \
             regen notes in this file's header table",
            case.payload, case.descriptor, case.channels
        );
    }
}

fn fixture(name: &str) -> Vec<u8> {
    match fixture_bytes(name) {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("{err}"),
    }
}

#[test]
fn corpus_geometry_matches_the_reference_shapes() {
    // The packet shapes the ffprobe capture recorded (also a guard
    // against a stale fixture/sizes pair slipping in).
    let shapes: &[(&str, usize, usize)] = &[
        ("yamaha_mono_payload.bin", 18, 1024),
        ("yamaha_st_payload.bin", 35, 1024),
        ("swf_payload.bin", 9, 4101),
        ("qt_payload.bin", 173, 68),
    ];
    for &(file, count, size) in shapes {
        let payload = fixture(file);
        assert_eq!(payload.len(), count * size, "{file}");
    }
    for file in ["oki_mono.vox", "oki_st.vox", "ws_mono.bin", "ws_st.bin"] {
        assert_eq!(fixture(file).len(), 512, "{file}");
    }
}

/// Fuzz-lite (doc 04 §8.5 posture, the PCM fuzz-survival shape):
/// deterministic pseudo-random payloads, truncations and bit-flips of
/// the real corpus through every flavor — the decoder must answer
/// `Emitted`/`Recovered`/typed errors, never panic, and its output
/// must stay inside the documented geometry bound.
#[test]
fn fuzz_lite_never_panics_and_stays_bounded() {
    let flavors: &[(&str, u16)] = &[
        ("adpcm_yamaha", 1),
        ("adpcm_yamaha", 2),
        ("adpcm_ima_oki", 1),
        ("adpcm_ima_oki", 2),
        ("adpcm_swf", 2),
        ("adpcm_ima_qt", 1),
        ("adpcm_ima_qt", 2),
        ("adpcm_ima_ws", 2),
    ];
    // A 64-bit LCG (deterministic across runs and platforms).
    let mut seed: u64 = 0x5EED_5A2B;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        seed
    };
    let mut corpus = Vec::new();
    for case in CASES {
        corpus.push(fixture(case.payload));
    }
    let mut runs = 0usize;
    for round in 0..240 {
        for &(descriptor, channels) in flavors {
            let payload: Vec<u8> = if round % 3 == 0 {
                // Pure random junk (bounded length, includes empty).
                let len = (next() % 300) as usize;
                (0..len).map(|_| (next() >> 32) as u8).collect()
            } else if round % 3 == 1 {
                // A truncation of a real corpus payload.
                let source = &corpus[runs % corpus.len()];
                let cut = (next() as usize) % (source.len() + 1);
                source[..cut].to_vec()
            } else {
                // A bit-flipped mutation of a real payload.
                let mut source = corpus[runs % corpus.len()].clone();
                for _ in 0..1 + (next() % 8) {
                    let pos = (next() as usize) % source.len();
                    source[pos] ^= (next() >> 40) as u8 | 1;
                }
                source
            };
            let mut decoder = AdpcmDecoder::new();
            let generation = GenerationId::new(3);
            let config = DecoderConfig::new(
                match media_types::ids::CodecDescriptor::new(descriptor) {
                    Ok(descriptor) => descriptor,
                    Err(err) => unreachable!("descriptor: {err}"),
                },
                generation,
                ThreadBudget::default(),
                None,
                Some(SharedBytes::from_vec(raw_facts(channels, 22_050))),
            );
            match decoder.configure(&config) {
                Ok(()) => {}
                Err(err) => unreachable!("fuzz configure: {err}"),
            }
            let mut capture = Capture::default();
            let outcome = decoder.decode(packet(generation, &payload), &mut capture);
            match outcome {
                Ok(DecodeOutcome::Emitted { .. } | DecodeOutcome::Recovered { .. }) => {}
                Ok(DecodeOutcome::NeedMoreInput) => assert!(payload.is_empty()),
                Ok(DecodeOutcome::Drained) => {}
                Err(err) => unreachable!("{descriptor} fuzz round {round}: {err}"),
            }
            let bound = payload.len().saturating_mul(8).saturating_add(64);
            assert!(
                capture.pcm.len() <= bound,
                "{descriptor} round {round}: {} > bound {bound}",
                capture.pcm.len()
            );
            assert!(
                capture.pcm.len() % (usize::from(channels) * 2) == 0,
                "{descriptor} round {round}: whole instants"
            );
            runs += 1;
        }
    }
    assert!(runs >= 240 * 8);
}

// ---------------------------------------------------------------------------
// A std-only MD5 (RFC 1321) — the codecs crate deliberately carries no
// dev-dependencies beyond the workspace testkit set, and the digest is
// only ever compared against the pinned reference table above.
// ---------------------------------------------------------------------------

/// MD5 of `data`, lowercase hex (matches `ffmpeg -f md5`'s `MD5=…`).
fn md5_hex(data: &[u8]) -> String {
    let digest = md5(data);
    let mut hex = String::with_capacity(32);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// The raw 16-byte MD5 digest of `data` (RFC 1321).
fn md5(data: &[u8]) -> [u8; 16] {
    let mut message = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_le_bytes());

    let mut a0: u32 = 0x6745_2301;
    let mut b0: u32 = 0xefcd_ab89;
    let mut c0: u32 = 0x98ba_dcfe;
    let mut d0: u32 = 0x1032_5476;

    for chunk in message.as_chunks::<64>().0 {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(MD5_K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(MD5_S[i]));
            a = temp;
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut digest = [0u8; 16];
    digest[0..4].copy_from_slice(&a0.to_le_bytes());
    digest[4..8].copy_from_slice(&b0.to_le_bytes());
    digest[8..12].copy_from_slice(&c0.to_le_bytes());
    digest[12..16].copy_from_slice(&d0.to_le_bytes());
    digest
}

/// The MD5 implementation is pinned against RFC 1321's own test
/// vectors before any differential assertion relies on it.
#[test]
fn md5_implementation_matches_rfc1321_vectors() {
    assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(md5_hex(b"a"), "0cc175b9c0f1b6a831c399e269772661");
    assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        md5_hex(b"message digest"),
        "f96b697d7cb7938d525a2f31aaf161d0"
    );
    assert_eq!(
        md5_hex(b"abcdefghijklmnopqrstuvwxyz"),
        "c3fcd3d76192e4007dfb496cca67e13b"
    );
    assert_eq!(
        md5_hex(&b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"[..]),
        "d174ab98d277d9f5a5611c2c9f419d9f"
    );
    assert_eq!(
        md5_hex(&b"1234567890".repeat(8)),
        "57edf4a22be3c955ac49da2e2107b67a"
    );
}
