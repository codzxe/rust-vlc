//! MD5 message digest (RFC 1321), implemented from the public
//! specification — clean-room-legal (AGENTS.md §3.1: public
//! specifications only) and dependency-free (AGENTS.md §3.2: any
//! external crate needs Lead intake; MD5 over ~130 lines of std-only
//! Rust is not worth one).
//!
//! **Scope**: test-tool only. MD5 is not a security primitive here —
//! it is doc 10 §6.6's chosen *decoded-PCM comparison digest* (the
//! same digest VLC-side tooling produces), and its collision
//! resistance is irrelevant to "did two decoders emit the same
//! bytes". It never enters engine crates.
//!
//! The implementation is the textbook one: 512-bit blocks, a 128-bit
//! chaining state, 64 rounds of the RFC's F/G/H/I mixing with the
//! sine-derived K table, Merkle–Damgård padding (0x80, zeros, 64-bit
//! little-endian bit length).

/// The per-round left-rotation amounts (RFC 1321 §3.4, the `S` table:
/// four rounds of 16 operations, each round cycling four shifts).
const SHIFTS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// The per-round additive constants (RFC 1321 §3.4, the `K` table:
/// `floor(abs(sin(i + 1)) · 2³²)` for `i = 0..63`, tabulated).
const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// Streaming MD5 state.
#[derive(Clone)]
pub struct Md5 {
    /// The chaining variables (RFC's A, B, C, D).
    state: [u32; 4],
    /// Total message bytes seen so far (padding needs the bit length).
    len_bytes: u64,
    /// Partial block (0..64 bytes buffered).
    buffer: [u8; 64],
    buffered: usize,
}

impl Default for Md5 {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5 {
    /// A fresh digest over the empty message.
    pub fn new() -> Md5 {
        Md5 {
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            len_bytes: 0,
            buffer: [0u8; 64],
            buffered: 0,
        }
    }

    /// Absorb more bytes (any length; blocks are processed as they
    /// fill).
    pub fn update(&mut self, data: &[u8]) {
        self.len_bytes = self.len_bytes.wrapping_add(data.len() as u64);
        let mut rest = data;
        if self.buffered > 0 {
            let want = 64 - self.buffered;
            let take = want.min(rest.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&rest[..take]);
            self.buffered += take;
            rest = &rest[take..];
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while rest.len() >= 64 {
            let (block, tail) = rest.split_at(64);
            let mut owned = [0u8; 64];
            owned.copy_from_slice(block);
            self.compress(&owned);
            rest = tail;
        }
        if !rest.is_empty() {
            self.buffer[..rest.len()].copy_from_slice(rest);
            self.buffered = rest.len();
        }
    }

    /// Pad, compress the final blocks, and answer the 16-byte digest.
    pub fn finalize(mut self) -> [u8; 16] {
        let bit_len = self.len_bytes.wrapping_mul(8);
        // Padding: 0x80, zeros, then the 64-bit little-endian bit
        // length — with room guaranteed for the length field.
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0x00]);
        }
        // Bypass `update`'s length accounting for the appended size
        // (it is padding metadata, not message bytes).
        self.buffer[56..64].copy_from_slice(&bit_len.to_le_bytes());
        let block = self.buffer;
        self.compress(&block);
        let mut out = [0u8; 16];
        for (word, chunk) in self.state.iter().zip(out.as_chunks_mut::<4>().0) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    /// The digest as lowercase hex (the report/JSON spelling).
    pub fn finalize_hex(self) -> String {
        to_hex(&self.finalize())
    }

    /// One 64-byte block through the 64 rounds (RFC 1321 §3.4).
    fn compress(&mut self, block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for (slot, chunk) in m.iter_mut().zip(block.as_chunks::<4>().0) {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(chunk);
            *slot = u32::from_le_bytes(bytes);
        }
        let [mut a, mut b, mut c, mut d] = self.state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let tmp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(SHIFTS[i]));
            a = tmp;
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

/// Lowercase hex spelling of 16 bytes.
fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// One-shot MD5 over `data`, lowercase hex.
pub fn md5_hex(data: &[u8]) -> String {
    let mut digest = Md5::new();
    digest.update(data);
    digest.finalize_hex()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The RFC 1321 §A.5 test suite (the seven canonical vectors) —
    /// the spec's own truth, cross-checked against Python's
    /// `hashlib.md5` during development (independent implementation).
    #[test]
    fn rfc1321_vectors() {
        let cases: &[(&str, &str)] = &[
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "c3fcd3d76192e4007dfb496cca67e13b",
            ),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ];
        for (input, want) in cases {
            assert_eq!(&md5_hex(input.as_bytes()), want, "input {input:?}");
        }
    }

    /// A multi-block message (the 80-char vector already forces two
    /// blocks) fed one byte at a time must agree with the one-shot
    /// digest — the streaming buffer logic is the risky half.
    #[test]
    fn streaming_matches_one_shot() {
        let data: Vec<u8> = (0..255u16)
            .map(|v| (v * 7) as u8)
            .cycle()
            .take(1000)
            .collect();
        let one_shot = md5_hex(&data);
        let mut digest = Md5::new();
        for byte in &data {
            digest.update(std::slice::from_ref(byte));
        }
        assert_eq!(digest.finalize_hex(), one_shot);
    }

    /// Chunked feeding at awkward boundaries (not 64-aligned) also
    /// agrees — the partial-block path.
    #[test]
    fn chunked_boundaries_agree() {
        let data: Vec<u8> = (0..500u32).map(|v| v as u8).collect();
        let one_shot = md5_hex(&data);
        for chunk in [1usize, 3, 7, 63, 64, 65, 113] {
            let mut digest = Md5::new();
            for piece in data.chunks(chunk) {
                digest.update(piece);
            }
            assert_eq!(digest.finalize_hex(), one_shot, "chunk {chunk}");
        }
    }
}
