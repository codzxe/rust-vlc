//! Shared fixtures for the renderer-wgpu integration suites (card
//! 2.D1 AC). Integration tests cannot see the crate's `#[cfg(test)]`
//! modules, so this module carries the test-side machinery:
//!
//! - a std-only **SHA-256** (the demux/codecs test-suite pattern;
//!   vector-pinned, additionally cross-checked against the system
//!   `sha256sum` at recording time — worklog 2.D1);
//! - a **structural PNG decoder** — chunk walk with CRC verification,
//!   stored-deflate inflation, Adler-32 check, filter-0 defiltering —
//!   so a committed PNG hash is proven to pin a *verified image*, not
//!   just "whatever the encoder emitted" (the 2.C3 value-truth
//!   precedent);
//! - the **virtual clock** of the 2.C4 scheduling suites (a
//!   [`MediaClock`] over a settable monotonic source — exact
//!   nanoseconds, zero sleeps, doc 03 §8);
//! - synthetic [`PresentRequest`] builders over
//!   [`renderer_wgpu::synthetic`].
//!
//! Test code only: a wrong shape here is a broken *fixture*, so the
//! `unreachable!`s are the testkit `must` vocabulary (AGENTS §3.5).

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use media_clock::{MediaClock, MonotonicSource, NANO_BASE, PlaybackClock};
use media_types::GenerationId;
use media_types::time::{Rate, Timestamp};
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::synthetic;
use testkit::Must;
use video_engine::{PresentRequest, Presenter};

// ---------------------------------------------------------------------------
// SHA-256 (FIPS 180-4) — test-side, vector-pinned
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

/// SHA-256 of `data`, lowercase hex (the recording format of the AC).
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = sha256(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Pin the hasher against FIPS 180-4 test vectors (plus one-block,
/// two-block and 56-byte padding-boundary inputs) — called by every
/// suite's first test so a broken hasher can never green the AC hash
/// by coincidence.
pub fn assert_sha256_vectors(context: &str) {
    // FIPS 180-4 / NIST examples.
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "{context}"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        "{context}"
    );
    // Empty and exactly-one-block inputs.
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "{context}"
    );
    assert_eq!(
        sha256_hex(&[0x61; 64]),
        "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
        "{context}"
    );
    // Two full blocks (system-tool-derived vector: sha256sum/hashlib).
    assert_eq!(
        sha256_hex(&[0x61; 128]),
        "6836cf13bac400e9105071cd6af47084dfacad4e5e302c94bfed24e013afb73e",
        "{context}"
    );
    // 56 bytes: padding crosses into a second block.
    assert_eq!(
        sha256_hex(&[0x61; 56]),
        "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
        "{context}"
    );
}

// ---------------------------------------------------------------------------
// CRC-32 (PNG/ISO 3309) + Adler-32 — the PNG decoder's integrity layer
// ---------------------------------------------------------------------------

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + u32::from(byte)) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

// ---------------------------------------------------------------------------
// Structural PNG decoder (the AC's value-truth device)
// ---------------------------------------------------------------------------

/// One walked PNG chunk: type, byte length, CRC as recorded.
#[derive(Debug, Clone)]
pub struct WalkedChunk {
    /// The four-byte chunk type (ASCII).
    pub kind: [u8; 4],
    /// The chunk's data length in bytes.
    pub len: u32,
    /// The chunk's recorded CRC-32 (verified against the recomputed
    /// one during the walk).
    pub crc: u32,
}

/// A fully verified PNG: dimensions, pixels, chunk census.
#[derive(Debug, Clone)]
pub struct DecodedPng {
    /// Image width (IHDR).
    pub width: u32,
    /// Image height (IHDR).
    pub height: u32,
    /// RGBA8 pixels, row-major, `width * height * 4` (defiltered).
    pub rgba: Vec<u8>,
    /// The `sRGB` chunk's rendering intent, when present.
    pub srgb_intent: Option<u8>,
    /// Every chunk in stream order (CRCs already verified).
    pub chunks: Vec<WalkedChunk>,
}

/// Decode and **verify** a PNG the honest way: signature, chunk CRCs,
/// IHDR fields, zlib header, stored-deflate blocks, Adler-32, filter
/// bytes. Returns the defiltered RGBA8 image.
///
/// Test-side verification of the crate's own encoder output: any
/// structural lie is a broken test fixture, hence the assert messages
/// carry the position (the testkit vocabulary).
pub fn decode_png(png: &[u8]) -> DecodedPng {
    let signature = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    assert!(png.starts_with(&signature), "PNG signature");

    // 1. Chunk walk: length + type + data + CRC, verified.
    let mut chunks = Vec::new();
    let mut idat = Vec::new();
    let mut offset = 8usize;
    let mut ihdr: Option<[u8; 13]> = None;
    let mut srgb_intent = None;
    while offset < png.len() {
        assert!(offset + 12 <= png.len(), "chunk header fits at {offset}");
        let len = u32::from_be_bytes([
            png[offset],
            png[offset + 1],
            png[offset + 2],
            png[offset + 3],
        ]) as usize;
        let kind = [
            png[offset + 4],
            png[offset + 5],
            png[offset + 6],
            png[offset + 7],
        ];
        let data_start = offset + 8;
        let data_end = data_start + len;
        assert!(data_end + 4 <= png.len(), "chunk body fits at {offset}");
        let data = &png[data_start..data_end];
        let recorded = u32::from_be_bytes([
            png[data_end],
            png[data_end + 1],
            png[data_end + 2],
            png[data_end + 3],
        ]);
        let mut crc_input = Vec::with_capacity(4 + len);
        crc_input.extend_from_slice(&kind);
        crc_input.extend_from_slice(data);
        let recomputed = crc32(&crc_input);
        assert_eq!(
            recorded,
            recomputed,
            "chunk {} CRC",
            String::from_utf8_lossy(&kind)
        );
        match &kind {
            b"IHDR" => {
                assert_eq!(len, 13, "IHDR length");
                let mut header = [0u8; 13];
                header.copy_from_slice(data);
                ihdr = Some(header);
            }
            b"sRGB" => {
                assert_eq!(len, 1, "sRGB length");
                srgb_intent = Some(data[0]);
            }
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => assert_eq!(len, 0, "IEND length"),
            other => unreachable!("unexpected chunk {}", String::from_utf8_lossy(other)),
        }
        chunks.push(WalkedChunk {
            kind,
            len: len as u32,
            crc: recorded,
        });
        offset = data_end + 4;
    }
    assert_eq!(offset, png.len(), "no trailing bytes");
    let ihdr = ihdr.must();

    // 2. IHDR: 8-bit RGBA, deflate, no filter, no interlace.
    let width = u32::from_be_bytes([ihdr[0], ihdr[1], ihdr[2], ihdr[3]]);
    let height = u32::from_be_bytes([ihdr[4], ihdr[5], ihdr[6], ihdr[7]]);
    assert_eq!(&ihdr[8..13], &[8, 6, 0, 0, 0], "IHDR fields");

    // 3. zlib stream: header, stored blocks, Adler-32.
    assert_eq!(&idat[..2], &[0x78, 0x01], "zlib header");
    let mut raw = Vec::new();
    let mut at = 2usize;
    loop {
        assert!(at + 5 <= idat.len(), "stored block header fits at {at}");
        let header = idat[at];
        assert_eq!(header & 0x06, 0, "stored (BTYPE=00) block at {at}");
        let len = u16::from_le_bytes([idat[at + 1], idat[at + 2]]) as usize;
        let nlen = u16::from_le_bytes([idat[at + 3], idat[at + 4]]);
        assert_eq!(!(len as u16), nlen, "NLEN complement at {at}");
        raw.extend_from_slice(&idat[at + 5..at + 5 + len]);
        at += 5 + len;
        if header & 0x01 == 1 {
            break;
        }
    }
    assert_eq!(at + 4, idat.len(), "Adler-32 trails the last block");
    let recorded_adler = u32::from_be_bytes([idat[at], idat[at + 1], idat[at + 2], idat[at + 3]]);
    assert_eq!(recorded_adler, adler32(&raw), "Adler-32");

    // 4. Scanlines: filter byte 0 (None), then width*4 pixel bytes.
    let stride = width as usize * 4;
    assert_eq!(
        raw.len(),
        (stride + 1) * height as usize,
        "raw scanline size"
    );
    let mut rgba = Vec::with_capacity(stride * height as usize);
    for row in 0..height as usize {
        let base = row * (stride + 1);
        assert_eq!(raw[base], 0, "filter type None (row {row})");
        rgba.extend_from_slice(&raw[base + 1..base + 1 + stride]);
    }

    DecodedPng {
        width,
        height,
        rgba,
        srgb_intent,
        chunks,
    }
}

// ---------------------------------------------------------------------------
// Virtual clock (the 2.C4 integration-suite pattern)
// ---------------------------------------------------------------------------

/// A deterministic monotonic source: plain nanoseconds, test-driven.
pub struct FakeMonotonic(AtomicU64);

impl MonotonicSource for FakeMonotonic {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// A [`MediaClock`] in system-monotonic master over a settable
/// nanosecond source — exact nanosecond positions at rate 1, zero
/// sleeps (doc 03 §8; the 2.C4 `VirtualClock` shape).
pub struct VirtualClock {
    source: Arc<FakeMonotonic>,
    clock: Arc<MediaClock>,
}

impl VirtualClock {
    /// A clock at position zero, running, generation `FIRST`.
    pub fn new() -> VirtualClock {
        let source = Arc::new(FakeMonotonic(AtomicU64::new(0)));
        let clock =
            MediaClock::system_monotonic(source.clone(), Timestamp::new(0, NANO_BASE), Rate::ONE)
                .must();
        VirtualClock {
            source,
            clock: Arc::new(clock),
        }
    }

    /// The scheduler-facing handle.
    pub fn clock(&self) -> Arc<dyn PlaybackClock> {
        self.clock.clone()
    }

    /// Move the virtual clock to an absolute position (monotonic-safe).
    pub fn set_ns(&self, at: u64) {
        let _ = self.source.0.fetch_max(at, Ordering::Relaxed);
    }

    /// Advance the virtual clock by a delta.
    pub fn advance_ns(&self, by: u64) {
        let _ = self.source.0.fetch_add(by, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Synthetic presentation requests
// ---------------------------------------------------------------------------

/// A presentation request wrapping the crate's deterministic synthetic
/// frame at `extent`, scheduled for the `when_ns` slot under
/// `generation` (the 2.C4 `PresentRequest` shape, fed by the 2.D1
/// generator).
pub fn synthetic_request(
    extent: Extent2D,
    when_ns: i64,
    generation: GenerationId,
) -> PresentRequest {
    let mut frame = synthetic::synthetic_frame(extent);
    frame.generation = generation;
    PresentRequest {
        frame,
        when: Timestamp::new(when_ns, NANO_BASE),
        generation,
    }
}

/// The synthetic frame's expected RGBA8 bytes (the pure generator's
/// output — the value-truth reference the renderer must reproduce at
/// 1:1 placement).
pub fn expected_pattern(extent: Extent2D) -> Vec<u8> {
    synthetic::synthetic_pattern(extent)
}

/// Present one request through any [`Presenter`] (the seam under test).
pub fn present<P: Presenter + ?Sized>(sink: &mut P, request: &PresentRequest) {
    sink.present(request).must();
}
