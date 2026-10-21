#![allow(dead_code)]
//! Shared support for the 2.C3 acceptance suites (std-only; the same
//! audit-one-screen discipline as the codecs crate's test common).
//!
//! Contents:
//! - fixture access via `CARGO_MANIFEST_DIR`;
//! - a std-only SHA-256 (`sha256_hex`, FIPS 180-4 public math — same
//!   implementation family as the codecs test common; pinned here by
//!   the corpus's own recorded digest, not by faith);
//! - the **spec'd sine**: the committed corpus's NCO tone plan,
//!   re-implemented from the fixture writer's documented public math
//!   (`tools/xtask/src/cmd/fixtures.rs` — quarter-wave constant,
//!   phase-accumulator oscillator, tone frequencies/amplitudes). The
//!   reconstruction is pinned against the expectation TOML's recorded
//!   `audio_stream_sha256`: if the writer's tone plan ever changes,
//!   this suite breaks first (writer and consumer pin each other, the
//!   same closed loop the 2.B5 `[audio_probe]` digest uses).

use std::path::PathBuf;

/// Repository fixtures root, relative to this crate's manifest.
fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

/// Absolute path of a committed fixture (e.g.
/// `"generated/valid/avi_mjpeg_pcm.avi"`).
pub fn fixture_path(relative: &str) -> PathBuf {
    fixtures_root().join(relative)
}

/// Read a committed fixture's bytes.
///
/// # Errors
/// Propagates filesystem errors verbatim (the tests render them).
pub fn fixture_bytes(relative: &str) -> Result<Vec<u8>, String> {
    std::fs::read(fixture_path(relative))
        .map_err(|e| format!("{}: {e}", fixture_path(relative).display()))
}

/// Extract one `key = "value"` string from an expectation TOML (the
/// files are machine-generated with one key per line — a targeted
/// reader, not a TOML parser; the codecs test common carries the same
/// shape).
///
/// # Errors
/// A human-readable message when the key is missing.
pub fn toml_string(toml: &str, key: &str) -> Result<String, String> {
    let prefix = format!("{key} = \"");
    for line in toml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix)
            && let Some(value) = rest.strip_suffix('"')
        {
            return Ok(value.to_string());
        }
    }
    Err(format!("key `{key}` not found in expectation TOML"))
}

// ---- SHA-256 (FIPS 180-4, public specification) -------------------------

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 of `data`, lowercase hex (pinned against the corpus's
/// committed `audio_stream_sha256` by the first test that uses it).
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for block in message.chunks(64) {
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

// ---- The spec'd sine: the corpus tone plan (public math) ----------------
//
// Everything in this section is transcribed from the fixture writer's
// documented constants and formulas
// (tools/xtask/src/cmd/fixtures.rs, card 2.A3): the quarter-wave
// table, the interpolated phase-accumulator oscillator, and the tone
// plan (440 Hz left / 660 Hz right at 12 000 / 9 000 amplitude, 44
// 100 Hz stereo s16le, 1470 samples per 30 fps frame tick × 60
// frames). Pure integer math — byte-identical on every host. The
// transcription is pinned by `audio_stream_sha256` in the sine-hash
// suite: if the writer's tone plan ever changes, this suite breaks
// first (writer and consumer pin each other, the closed loop the
// 2.B5 `[audio_probe]` digest established).

/// `round(sin(i * pi / 128) * 32767)` for i in 0..=64 (quarter wave,
/// endpoint 32767 included) — fixed at authoring time, public math
/// (fixture writer `SIN_QUARTER`).
const SIN_QUARTER: [u16; 65] = [
    0, 804, 1608, 2410, 3212, 4011, 4808, 5602, //
    6393, 7179, 7962, 8739, 9512, 10278, 11039, 11793, //
    12539, 13279, 14010, 14732, 15446, 16151, 16846, 17530, //
    18204, 18868, 19519, 20159, 20787, 21403, 22005, 22594, //
    23170, 23731, 24279, 24811, 25329, 25832, 26319, 26790, //
    27245, 27683, 28105, 28510, 28898, 29268, 29621, 29956, //
    30273, 30571, 30852, 31113, 31356, 31580, 31785, 31971, //
    32137, 32285, 32412, 32521, 32609, 32678, 32728, 32757, //
    32767,
];

/// Full-wave table: `TABLE[j]` = sine at phase `j/256`, j in 0..=256
/// (wrapping point duplicated so interpolation never indexes out).
fn sine_full_table() -> [i32; 257] {
    let mut table = [0i32; 257];
    for (j, slot) in table.iter_mut().enumerate() {
        let t = j & 0xff;
        let magnitude = if t <= 64 {
            SIN_QUARTER[t] as i32
        } else if t <= 128 {
            SIN_QUARTER[128 - t] as i32
        } else if t <= 192 {
            -(SIN_QUARTER[t - 128] as i32)
        } else {
            -(SIN_QUARTER[256 - t] as i32)
        };
        *slot = magnitude;
    }
    table
}

/// Integer sine oscillator (fixture writer `SineOsc`).
struct SineOsc {
    phase: u32,
    step: u32,
}

impl SineOsc {
    /// `step = round(freq * 2^32 / rate)` — u64 intermediate, no float.
    fn new(freq: u32, rate: u32) -> SineOsc {
        let numerator = (freq as u64) << 32;
        let step = (numerator + (rate as u64) / 2) / rate as u64;
        SineOsc {
            phase: 0,
            step: u32::try_from(step).unwrap_or(u32::MAX),
        }
    }

    /// Next sample in [-32767, 32767] (15-bit scale).
    fn sample(&mut self, table: &[i32; 257]) -> i32 {
        let index = (self.phase >> 24) as usize;
        let frac = ((self.phase >> 8) & 0xffff) as i32;
        let a = table[index];
        let b = table[index + 1];
        self.phase = self.phase.wrapping_add(self.step);
        a + (((b - a) * frac) >> 16)
    }
}

/// The corpus tone plan (fixture writer constants).
pub const TONE_RATE: u32 = 44_100;
/// Samples per 30 fps frame tick (`AUDIO_RATE / FPS`).
pub const TONE_SAMPLES_PER_TICK: usize = 1470;
/// Total sample-instants per channel (`FRAME_COUNT * SAMPLES_PER_TICK`).
pub const TONE_TOTAL_SAMPLES: usize = 88_200;
const TONE_LEFT_HZ: u32 = 440;
const TONE_RIGHT_HZ: u32 = 660;
const TONE_LEFT_AMP: i32 = 12_000;
const TONE_RIGHT_AMP: i32 = 9_000;

/// The committed corpus's interleaved s16le stereo PCM, regenerated
/// from the spec'd tone plan (fixture writer `generate_pcm`).
pub fn corpus_pcm() -> Vec<u8> {
    let table = sine_full_table();
    let mut left = SineOsc::new(TONE_LEFT_HZ, TONE_RATE);
    let mut right = SineOsc::new(TONE_RIGHT_HZ, TONE_RATE);
    let mut pcm = Vec::with_capacity(TONE_TOTAL_SAMPLES * 4);
    for _ in 0..TONE_TOTAL_SAMPLES {
        let l = ((left.sample(&table) * TONE_LEFT_AMP) >> 15) as i16;
        let r = ((right.sample(&table) * TONE_RIGHT_AMP) >> 15) as i16;
        pcm.extend_from_slice(&l.to_le_bytes());
        pcm.extend_from_slice(&r.to_le_bytes());
    }
    pcm
}

/// The pinned f32 conversion of a s16le stream (the same conversion
/// the corpus's `[audio_probe]` provenance pins and audio-engine's
/// stage A implements exactly): each scalar `v` → `v as f32 / 32768.0`
/// as IEEE-754 little-endian bytes. Every i16 is exact in f32 and the
/// divisor is a power of two, so this is bit-deterministic everywhere.
pub fn s16_stream_to_f32_bytes(s16le: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s16le.len() * 2);
    for pair in s16le.as_chunks::<2>().0 {
        let sample = i16::from_le_bytes(*pair);
        out.extend_from_slice(&(f32::from(sample) / 32_768.0).to_le_bytes());
    }
    out
}
