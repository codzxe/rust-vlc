//! The RFC 7845 → house channel-order mapping (card 6.M-opus; the
//! admission record's binding condition 2, the output-seam half).
//!
//! ruopus emits mapping-family-1 output in the **Vorbis channel
//! order** (RFC 7845 §5.1.1). The house PCM convention — the order
//! every other decoder here emits, and the order ffmpeg's native
//! decoder reorders to — is the **WAV/WAVEFORMATEXTENSIBLE order**
//! (the PCM decoder is a byte-order-preserving passthrough of it; the
//! AC-3 record measured its 5.1 lane matching ffmpeg's FL FR FC LFE
//! BL BR directly). The 5.1 shape: Vorbis order is FL FC FR BL BR
//! LFE, house order is FL FR FC LFE BL BR — the same samples, a
//! different interleave, permuting at the decoder's output seam so
//! every downstream consumer (differential, mixers, writers) sees the
//! one house order.
//!
//! Only mapping **family 1** (the Vorbis-ordered surround family) is
//! remapped. Family 0 (mono/stereo) is order-identical on both sides;
//! family 255 is **discrete** — channel identity is undefined by the
//! format, so the bytes pass through untouched (typed honestly, never
//! a guessed permutation).
//!
//! The 6-channel (5.1) row is corpus-verified (0077's differential
//! asserts the mapped order matches ffmpeg's native output, sample
//! for sample); the 3/4/5/7/8 rows are the documented Vorbis↔WAV
//! table entries (ffmpeg's own `vorbis` layout conversions), untested
//! against a lane this corpus does not carry.

/// The output-channel permutation for a mapping-family-1 stream:
/// `out[i] = decoded[PERMUTATION[ch][i]]`, `None` when the count has
/// no defined Vorbis→house row (pass through, honest).
#[must_use]
pub(crate) fn vorbis_to_house(channels: u8) -> Option<&'static [u8]> {
    match channels {
        // 1–2: identity on both sides (no row needed, but the table
        // carries it for the call-site's uniformity).
        1 => Some(&[0]),
        2 => Some(&[0, 1]),
        // 3: Vorbis FL FC FR → house FL FR FC.
        3 => Some(&[0, 2, 1]),
        // 4: Vorbis FL FC FR BC → house FL FR FC BC.
        4 => Some(&[0, 2, 1, 3]),
        // 5: Vorbis FL FC FR BL BR → house FL FR FC BL BR.
        5 => Some(&[0, 2, 1, 3, 4]),
        // 6 (5.1): Vorbis FL FC FR BL BR LFE → house FL FR FC LFE
        // BL BR — the 0077 lane's measured permutation.
        6 => Some(&[0, 2, 1, 5, 3, 4]),
        // 7 (6.1): Vorbis FL FC FR SL SR BC LFE → house FL FR FC
        // LFE BC SL SR.
        7 => Some(&[0, 2, 1, 6, 5, 3, 4]),
        // 8 (7.1): Vorbis FL FC FR SL SR BL BR LFE → house FL FR
        // FC LFE BL BR SL SR.
        8 => Some(&[0, 2, 1, 7, 5, 6, 3, 4]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permutations_are_total_bijections() {
        for channels in 1u8..=8 {
            let perm = vorbis_to_house(channels)
                .unwrap_or_else(|| unreachable!("1..=8 must have a row"));
            assert_eq!(perm.len(), usize::from(channels));
            let mut seen = vec![false; usize::from(channels)];
            for &slot in perm {
                assert!(usize::from(slot) < usize::from(channels));
                seen[usize::from(slot)] = true;
            }
            assert!(seen.iter().all(|&s| s), "every slot used exactly once");
        }
    }

    #[test]
    fn the_51_row_is_the_measured_vorbis_to_wav_permutation() {
        // Vorbis: [FL FC FR BL BR LFE], house: [FL FR FC LFE BL BR].
        let perm = vorbis_to_house(6).unwrap_or_else(|| unreachable!("5.1 row"));
        // out FL = in FL(0), out FR = in FR(2), out FC = in FC(1),
        // out LFE = in LFE(5), out BL = in BL(3), out BR = in BR(4).
        assert_eq!(perm, &[0, 2, 1, 5, 3, 4]);
    }

    #[test]
    fn unknown_counts_have_no_row() {
        assert!(vorbis_to_house(0).is_none());
        assert!(vorbis_to_house(9).is_none());
        assert!(vorbis_to_house(255).is_none());
    }
}
