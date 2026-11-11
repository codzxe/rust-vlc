//! std-only CRC-32 (ISO-HDLC / IEEE 802.3, the ZIP check value).
//!
//! Public-domain reflected polynomial math (0xEDB88320), used to
//! integrity-check every `.lumenext` ZIP entry we extract. No
//! dependency; verified against the canonical vectors in tests.

/// CRC-32 of `data`, initial value 0xFFFF_FFFF, final xor 0xFFFF_FFFF
/// (the ZIP central-directory convention).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc = (crc >> 8) ^ TABLE[((crc ^ b as u32) & 0xFF) as usize];
    }
    crc ^ 0xFFFF_FFFF
}

static TABLE: [u32; 256] = build_table();

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_vectors() {
        // Check values from the ISO-HDLC / PNG / ZIP literature,
        // cross-checked against zlib (crc32 of the same inputs).
        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
        // A high-bit byte string (zlib: 0xffff_ffff — the all-ones
        // fixed point of the reflected polynomial).
        assert_eq!(crc32(&[0xFF; 4]), 0xFFFF_FFFF);
    }
}
