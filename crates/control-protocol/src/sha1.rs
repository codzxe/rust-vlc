//! In-tree SHA-1 (RFC 3174) — card 7.J1, ADR-011 in-tree ethos.
//!
//! Exists for exactly one purpose: the RFC 6455 §4.2.2
//! `Sec-WebSocket-Accept` computation (`base64(SHA-1(key + GUID))`).
//! SHA-1 is **not** a security primitive here (RFC 6455's own design;
//! collision resistance is irrelevant to a nonce-hash) and MUST NOT be
//! used anywhere else in Lumen. Implemented in-crate (~100 lines,
//! fully specified) rather than pulling a `sha1` crate — no new
//! dependencies (C5).
//!
//! Pinned by external vectors (RFC 3174 examples, the NIST
//! million-`a` vector, and the RFC 6455 §1.3 end-to-end handshake
//! example) in the tests below — never only by self-generated output
//! (D-5).

/// One SHA-1 hash: 20 bytes.
pub type Digest = [u8; 20];

/// Streaming SHA-1 state (RFC 3174 §6.1).
#[derive(Debug, Clone)]
pub struct Sha1 {
    state: [u32; 5],
    buffer: [u8; 64],
    buffered: usize,
    /// Total message length processed so far, in BITS (u64 ⇒ the
    /// RFC's 2^64-bit message cap; callers never reach it — frames are
    /// capped at 1 MiB long before).
    length_bits: u64,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    /// Fresh state with the RFC 3174 §6.1.1 initialization vectors.
    pub fn new() -> Sha1 {
        Sha1 {
            state: [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0],
            buffer: [0u8; 64],
            buffered: 0,
            length_bits: 0,
        }
    }

    /// Absorb `data` (any length; internally buffered in 64-byte blocks).
    pub fn update(&mut self, data: &[u8]) {
        self.length_bits = self.length_bits.wrapping_add((data.len() as u64) * 8);
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

    /// Finish: pad per RFC 3174 §6.1.2 and emit the big-endian digest.
    pub fn finalize(mut self) -> Digest {
        let length_bits = self.length_bits;
        // Append 0x80, then zeros, then the 8-byte big-endian bit length
        // so the total is a multiple of 64 bytes.
        self.update(&[0x80]);
        // update() bumped length_bits; the true length was saved above.
        while self.buffered != 56 {
            self.update(&[0x00]);
        }
        let mut length_bytes = [0u8; 8];
        length_bytes.copy_from_slice(&length_bits.to_be_bytes());
        self.update(&length_bytes);
        let mut out = [0u8; 20];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    /// The RFC 3174 §6.1.2 block function over one 64-byte block.
    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = self.state;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}

/// One-shot SHA-1.
pub fn sha1(data: &[u8]) -> Digest {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize()
}

/// Lowercase hex rendering (test/log convenience).
pub fn hex(digest: &Digest) -> String {
    let mut out = String::with_capacity(40);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// External oracle vectors (RFC 3174 §7.3 examples + the classic
    /// NIST million-`a` vector) — not self-generated (D-5).
    #[test]
    fn external_vectors() {
        assert_eq!(hex(&sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hex(&sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(&sha1(b"The quick brown fox jumps over the lazy dog")), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
        let million_a = vec![b'a'; 1_000_000];
        assert_eq!(hex(&sha1(&million_a)), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
    }

    /// RFC 6455 §1.3: the doc's own worked handshake example.
    #[test]
    fn rfc6455_handshake_vector() {
        let mut hasher = Sha1::new();
        hasher.update(b"dGhlIHNhbXBsZSBub25jZQ==");
        hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let accept = crate::base64::encode(&hasher.finalize());
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    /// Streaming in odd chunk sizes must equal the one-shot hash
    /// (buffer-boundary coverage — D-3 dirty-world discipline).
    #[test]
    fn streaming_matches_oneshot_at_boundaries() {
        let data: Vec<u8> = (0u8..=255).cycle().take(1000).collect();
        let oneshot = sha1(&data);
        for chunk in [1usize, 3, 7, 63, 64, 65, 128, 999] {
            let mut hasher = Sha1::new();
            for piece in data.chunks(chunk) {
                hasher.update(piece);
            }
            assert_eq!(hasher.finalize(), oneshot, "chunk size {chunk}");
        }
    }
}
