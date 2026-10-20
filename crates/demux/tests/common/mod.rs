//! Shared helpers for the demux integration suites (card 2.B2).
//!
//! - [`sha256`]: the test-only SHA-256 (FIPS 180-4) used to verify the
//!   fixture digests. Zero external dependencies (C5): the algorithm is
//!   public domain, the shape follows the xtask fixture generator's
//!   in-tree implementation, and the first test that uses it pins it
//!   against the committed corpus's recorded whole-file hash.
//! - [`Expectation`]: a parser for the `expectation/1` TOML subset the
//!   AVI corpus records (doc 10 §5.4) — the tests verify the *recorded*
//!   truth, never a transcribed copy of it.
//! - [`open_via_input`]: the real byte source of the first vertical —
//!   the fixture opened through `input`'s `FileAccess` (card 2.A1),
//!   proving the 2.A1+2.B2 composition at the plugin-api seam.

use std::path::Path;

use media_types::EngineError;
use plugin_api::{Access, SourceStream};
use testkit::Must;

// ---------------------------------------------------------------------------
// SHA-256 (FIPS 180-4) — test-only
// ---------------------------------------------------------------------------

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

struct Sha256 {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_bytes: u64,
}

impl Sha256 {
    fn new() -> Sha256 {
        Sha256 {
            // Fractional parts of the square roots of the first 8
            // primes (FIPS 180-4 §5.3.3).
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: [0u8; 64],
            buf_len: 0,
            total_bytes: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_bytes = self.total_bytes.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let want = 64 - self.buf_len;
            let take = want.min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                let block = self.buf;
                self.compress(&block);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.compress(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    fn finish(mut self) -> [u8; 32] {
        let bit_len = self.total_bytes.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buf_len != 56 {
            self.update(&[0]);
        }
        // The length is appended without counting it in total_bytes.
        self.buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.buf;
        self.compress(&block);
        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
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
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finish()
}

/// SHA-256 of `data`, lowercase hex (the corpus's recording format).
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = sha256(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

// ---------------------------------------------------------------------------
// expectation/1 TOML subset
// ---------------------------------------------------------------------------

/// One parsed `expectation/1` file (the AVI corpus subset; fields this
/// card does not consume are ignored by design).
#[derive(Debug, Clone)]
pub struct Expectation {
    pub fixture: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub track_count: u64,
    pub video_codec: Option<String>,
    pub video_width: Option<u32>,
    pub video_height: Option<u32>,
    pub video_fps: Option<(u32, u32)>,
    pub audio_codec: Option<String>,
    pub audio_rate: Option<u32>,
    pub audio_channels: Option<u16>,
    pub duration_us: u64,
    pub seek_targets_us: Vec<u64>,
    pub seek_tolerance_us: u64,
    pub video_sample_count: u64,
    pub video_sample_sha256: Vec<String>,
    pub audio_stream_sha256: String,
    pub audio_stream_bytes: u64,
    pub cut_offset: Option<u64>,
}

/// Parse an `expectation/1` TOML (subset: the AVI corpus fields). Test
/// code only — a wrong shape is a broken *fixture*, so failures are
/// `unreachable!` (the testkit `must` vocabulary).
pub fn parse_expectation(relative: &str) -> Expectation {
    let path = testkit::fixture_path(relative).must();
    let text = std::fs::read_to_string(&path).must();
    let mut expect = Expectation {
        fixture: String::new(),
        sha256: String::new(),
        size_bytes: 0,
        track_count: 0,
        video_codec: None,
        video_width: None,
        video_height: None,
        video_fps: None,
        audio_codec: None,
        audio_rate: None,
        audio_channels: None,
        duration_us: 0,
        seek_targets_us: Vec::new(),
        seek_tolerance_us: 0,
        video_sample_count: 0,
        video_sample_sha256: Vec::new(),
        audio_stream_sha256: String::new(),
        audio_stream_bytes: 0,
        cut_offset: None,
    };

    let mut section = "";
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1];
            continue;
        }
        // The per-frame digest array spans many lines: accumulate
        // until its closing bracket, then harvest the quoted strings.
        if let Some(rest) = line.strip_prefix("video_sample_sha256 = [") {
            let mut acc = rest.to_string();
            while !acc.contains(']') {
                match lines.next() {
                    Some(next) => {
                        acc.push(' ');
                        acc.push_str(next.trim());
                    }
                    None => break,
                }
            }
            expect.video_sample_sha256 = harvest_strings(&acc).0;
            continue;
        }
        let (key, value) = split_key_value(line);
        match section {
            "tracks" => match key {
                "count" => expect.track_count = parse_u64(value),
                // The corpus records duration_us inside the [tracks]
                // block (it is the tracks' shared extent).
                "duration_us" => expect.duration_us = parse_u64(value),
                "video" => {
                    expect.video_codec = extract_string(value, "codec");
                    expect.video_width = extract_u32(value, "width");
                    expect.video_height = extract_u32(value, "height");
                    expect.video_fps = extract_fps(value);
                }
                "audio" => {
                    expect.audio_codec = extract_string(value, "codec");
                    expect.audio_rate = extract_u32(value, "rate");
                    expect.audio_channels =
                        extract_u32(value, "channels").and_then(|c| u16::try_from(c).ok());
                }
                _ => {}
            },
            "seek" => match key {
                "targets_us" => {
                    let inline = value.trim_start_matches('[').trim_end_matches(']');
                    expect.seek_targets_us = inline
                        .split(',')
                        .filter(|part| !part.trim().is_empty())
                        .map(parse_u64)
                        .collect();
                }
                "tolerance_us" => expect.seek_tolerance_us = parse_u64(value),
                _ => {}
            },
            "essence_v0" => match key {
                "video_sample_count" => expect.video_sample_count = parse_u64(value),
                "audio_stream_sha256" => expect.audio_stream_sha256 = parse_string(value),
                "audio_stream_bytes" => expect.audio_stream_bytes = parse_u64(value),
                _ => {}
            },
            "derivation" => {
                if key == "cut_offset" {
                    expect.cut_offset = Some(parse_u64(value));
                }
            }
            _ => match key {
                "fixture" => expect.fixture = parse_string(value),
                "sha256" => expect.sha256 = parse_string(value),
                "size_bytes" => expect.size_bytes = parse_u64(value),
                "duration_us" => expect.duration_us = parse_u64(value),
                _ => {}
            },
        }
    }
    expect
}

/// Harvest every complete quoted string in `line`; returns
/// `(strings, remainder_after_last_open_quote_or_close)` where the
/// remainder tracks whether the array is still open.
fn harvest_strings(line: &str) -> (Vec<String>, String) {
    let mut items = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        match after.find('"') {
            Some(end) => {
                items.push(after[..end].to_string());
                rest = &after[end + 1..];
            }
            None => return (items, after.to_string()),
        }
    }
    (items, rest.to_string())
}

fn split_key_value(line: &str) -> (&str, &str) {
    match line.split_once('=') {
        Some((key, value)) => (key.trim(), value.trim()),
        None => ("", ""),
    }
}

fn parse_string(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn parse_u64(value: &str) -> u64 {
    let cleaned: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    cleaned.parse().unwrap_or(0)
}

/// `video = [{ ... codec = "mjpeg" ... }]` — extract a quoted string
/// field by name from the inline table.
fn extract_string(value: &str, field: &str) -> Option<String> {
    let needle = format!("{field} = \"");
    let start = value.find(&needle)? + needle.len();
    let rest = &value[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract a numeric field from the inline table.
fn extract_u32(value: &str, field: &str) -> Option<u32> {
    let needle = format!("{field} = ");
    let start = value.find(&needle)? + needle.len();
    let rest = &value[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '_')
        .unwrap_or(rest.len());
    let cleaned: String = rest[..end].chars().filter(|c| c.is_ascii_digit()).collect();
    if cleaned.is_empty() {
        None
    } else {
        u32::try_from(cleaned.parse::<u64>().unwrap_or(u64::MAX)).ok()
    }
}

/// `fps = { num = 30, den = 1 }` inside the inline table.
fn extract_fps(value: &str) -> Option<(u32, u32)> {
    let num = extract_u32(value, "num")?;
    let den = extract_u32(value, "den")?;
    Some((num, den))
}

// ---------------------------------------------------------------------------
// The real byte source (card 2.A1 composition)
// ---------------------------------------------------------------------------

/// Open a fixture through `input`'s `FileAccess` — the first vertical's
/// access stack, not an in-memory stand-in.
pub fn open_via_input(path: &Path) -> Result<Box<dyn SourceStream>, EngineError> {
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
