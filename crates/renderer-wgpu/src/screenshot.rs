//! PNG encoding for screenshots and the headless acceptance proof
//! (card 2.D1 AC; doc 03 §4.11 f "screenshot tests of the compositing
//! result").
//!
//! A minimal, std-only PNG writer: 8-bit RGBA, no filtering, **stored**
//! (uncompressed) deflate blocks — byte-stable by construction (no
//! compressor heuristics, no entropy coding), which is exactly what a
//! committed-hash acceptance test needs. The stream declares itself
//! sRGB (the `sRGB` chunk, perceptual intent) matching the surface
//! semantics the bytes were presented under.
//!
//! Chunks: `IHDR` (8-bit RGBA, filter 0, interlace 0) → `sRGB` →
//! `IDAT` (zlib: stored blocks + Adler-32) → `IEND`. ~140 lines, no
//! dependencies — the same design brief as xtask's std-only JPEG
//! encoder for fixtures (2.A3), applied to the renderer's readback
//! path `[PROPOSAL: also the 2.D3 webview prototype's debug-capture
//! format]`.

use crate::error::{RendererError, RendererResult};
use crate::params::Extent2D;

/// Encode an RGBA8 buffer (row-major, no padding) as a PNG.
///
/// # Errors
/// [`RendererError::ZeroExtent`] for a zero dimension;
/// [`RendererError::EncodeSizeMismatch`] when the buffer is shorter
/// than `width * height * 4`.
pub fn encode_rgba(extent: Extent2D, rgba: &[u8]) -> RendererResult<Vec<u8>> {
    if extent.width == 0 || extent.height == 0 {
        return Err(RendererError::ZeroExtent {
            extent,
            from_frame: false,
        });
    }
    let expected = u64::from(extent.width)
        .checked_mul(u64::from(extent.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .unwrap_or(u64::MAX);
    if (rgba.len() as u64) < expected {
        return Err(RendererError::EncodeSizeMismatch {
            expected,
            found: rgba.len() as u64,
        });
    }

    let mut png = Vec::with_capacity(rgba.len() + rgba.len() / 32 + 128);
    png.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR: 8-bit RGBA, deflate, no filter, no interlace.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&extent.width.to_be_bytes());
    ihdr.extend_from_slice(&extent.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    push_chunk(&mut png, b"IHDR", &ihdr);

    // sRGB rendering intent 0 (perceptual) — the surface semantics.
    push_chunk(&mut png, b"sRGB", &[0]);

    // IDAT: zlib stream of filter-0 scanlines in stored deflate
    // blocks (deterministic — see the module docs).
    let stride = extent.width as usize * 4;
    let raw_len = (stride + 1) * extent.height as usize;
    let mut raw = Vec::with_capacity(raw_len);
    for row in 0..extent.height as usize {
        raw.push(0);
        let start = row * stride;
        raw.extend_from_slice(&rgba[start..start + stride]);
    }
    let mut idat = Vec::with_capacity(raw.len() + raw.len() / 65535 * 5 + 11);
    idat.extend_from_slice(&[0x78, 0x01]);
    let mut offset = 0usize;
    while offset < raw.len() {
        let block_len = (raw.len() - offset).min(65_535);
        let last = offset + block_len == raw.len();
        idat.push(u8::from(last));
        idat.extend_from_slice(&(block_len as u16).to_le_bytes());
        idat.extend_from_slice(&(!(block_len as u16)).to_le_bytes());
        idat.extend_from_slice(&raw[offset..offset + block_len]);
        offset += block_len;
    }
    if raw.is_empty() {
        // Degenerate but legal: one empty final stored block.
        idat.push(1);
        idat.extend_from_slice(&0u16.to_le_bytes());
        idat.extend_from_slice(&(!0u16).to_le_bytes());
    }
    idat.extend_from_slice(&adler32(&raw).to_be_bytes());
    push_chunk(&mut png, b"IDAT", &idat);

    push_chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

/// Append one PNG chunk (length + type + data + CRC-32 of type+data).
fn push_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// CRC-32 (PNG/ISO 3309, reflected polynomial 0xEDB88320).
fn crc32(data: &[u8]) -> u32 {
    // The 256-entry table is computed on the fly: encode_rgba is not
    // on a hot path, and a const table would just repeat what the
    // loop proves.
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

/// Adler-32 (RFC 1950) over the uncompressed data.
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

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    #[test]
    fn known_crc32_and_adler32_vectors() {
        // CRC-32 of "123456789" (classic check value).
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        // Adler-32 of "Wikipedia" (the RFC-1950-style example).
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        // Empty inputs.
        assert_eq!(crc32(b""), 0);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn encoded_stream_has_the_png_shape() {
        let png = encode_rgba(
            Extent2D {
                width: 2,
                height: 1,
            },
            &[1, 2, 3, 4, 5, 6, 7, 8],
        )
        .must();
        // Signature.
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        // IHDR first: length 13, dims, 8-bit RGBA.
        assert_eq!(&png[8..12], &13u32.to_be_bytes());
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &2u32.to_be_bytes());
        assert_eq!(&png[20..24], &1u32.to_be_bytes());
        assert_eq!(&png[24..29], &[8, 6, 0, 0, 0]);
        // sRGB chunk follows.
        assert_eq!(&png[33..37], &1u32.to_be_bytes());
        assert_eq!(&png[37..41], b"sRGB");
        // IEND is the last chunk: zero length, type IEND, crc.
        assert_eq!(&png[png.len() - 12..png.len() - 8], &0u32.to_be_bytes());
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn encoding_is_byte_stable_across_calls() {
        let pixels: Vec<u8> = (0..64u32).map(|i| (i * 7 % 256) as u8).collect();
        let a = encode_rgba(
            Extent2D {
                width: 4,
                height: 4,
            },
            &pixels,
        )
        .must();
        let b = encode_rgba(
            Extent2D {
                width: 4,
                height: 4,
            },
            &pixels,
        )
        .must();
        assert_eq!(a, b, "stored-deflate PNG must be deterministic");
    }

    #[test]
    fn size_mismatches_are_typed_errors() {
        let err = assert_err(
            encode_rgba(
                Extent2D {
                    width: 4,
                    height: 4,
                },
                &[0u8; 8],
            ),
            "expected a typed error",
        );
        assert_eq!(
            err,
            RendererError::EncodeSizeMismatch {
                expected: 64,
                found: 8
            }
        );
        let err = assert_err(
            encode_rgba(
                Extent2D {
                    width: 0,
                    height: 4,
                },
                &[0u8; 8],
            ),
            "expected a typed error",
        );
        assert!(matches!(err, RendererError::ZeroExtent { .. }));
    }

    #[test]
    fn large_streams_split_into_stored_blocks() {
        // 200_000 raw bytes need four 65_535-byte blocks.
        let extent = Extent2D {
            width: 1000,
            height: 50,
        }; // 200_000 + 50 filter bytes
        let pixels = vec![7u8; 1000 * 50 * 4];
        let png = encode_rgba(extent, &pixels).must();
        // Find the IDAT length: signature(8) + IHDR(25) + sRGB(13) = 46.
        let idat_len = u32::from_be_bytes([png[46], png[47], png[48], png[49]]) as usize;
        // zlib: 2 header + 4 blocks*(5+65535) + 4 adler... raw is
        // 50 rows of (1 + 4000) = 200_050 → blocks: 65535, 65535,
        // 65535, 3445 → 4 blocks.
        assert_eq!(idat_len, 2 + 4 * 5 + 200_050 + 4);
    }
}
