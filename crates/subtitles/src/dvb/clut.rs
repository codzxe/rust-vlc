//! DVB CLUTs (EN 300 743 §7.2.4): parse the definition segment
//! entries (2/4/8-bit depths, full- and limited-range YCbCrA) and
//! convert to straight-alpha RGBA8.
//!
//! The conversion is the ITU-R BT.601 headroom matrix with the SAME
//! integer arithmetic the reference decoders use (ffmpeg
//! `YUV_TO_RGB*_CCIR`, SCALEBITS=10), so the differential oracle is
//! bit-exact:
//!
//! ```text
//! r_add = FIX(1.40200·255/224)·(Cr-128) + 512
//! g_add = -FIX(0.34414·255/224)·(Cb-128) - FIX(0.71414·255/224)·(Cr-128) + 512
//! b_add = FIX(1.77200·255/224)·(Cb-128) + 512
//! r     = clamp(((Y-16)·FIX(255/219) + r_add) >> 10)
//! ```
//!
//! with `FIX(x) = round(x·2^10)`. DVB alpha is inverted relative to
//! straight alpha (`0` = opaque, `255` = transparent) and a luma of
//! `0` means fully transparent regardless of the alpha byte — both
//! rules applied here, once.

use crate::compositing::target::Rgba;

/// One CLUT: three banks (2-bit: 4 entries, 4-bit: 16, 8-bit: 256),
/// straight-alpha RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Clut {
    /// Version nibble last seen for this id (EDTV §7.2.4.2: a
    /// repeated version means "unchanged").
    pub version: u8,
    pub bank4: [Rgba; 4],
    pub bank16: [Rgba; 16],
    pub bank256: [Rgba; 256],
}

impl Clut {
    /// A CLUT from a definition segment (all default banks first, so
    /// partial redefinitions keep the unmentioned entries).
    pub fn from_entries(version: u8, entries: impl Iterator<Item = ClutEntry>) -> Clut {
        let mut clut = Clut::with_default_banks(version);
        for e in entries {
            let rgba = ycbcr_to_rgba(e.y, e.cb, e.cr, e.alpha);
            // The 2/4/8-bit banks alias the same entry id per depth
            // flag (EN 300 743 §7.2.4.2: entry flag bits 0x80 =
            // 2-bit bank, 0x40 = 4-bit, 0x20 = 8-bit) — the entry
            // lands in EVERY flagged bank.
            if e.depth_2 && e.entry_id < 4 {
                clut.bank4[e.entry_id as usize] = rgba;
            }
            if e.depth_4 && e.entry_id < 16 {
                clut.bank16[e.entry_id as usize] = rgba;
            }
            if e.depth_8 {
                clut.bank256[e.entry_id as usize] = rgba;
            }
        }
        clut
    }

    /// ffmpeg's `default_clut` (used when a region references an
    /// unknown CLUT id — the typed `BadClutId` damage path).
    pub fn default_banks() -> Clut {
        Clut::with_default_banks(0)
    }

    fn with_default_banks(version: u8) -> Clut {
        let mut bank4 = [Rgba::BLACK; 4];
        bank4[0] = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        bank4[1] = Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        bank4[2] = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        bank4[3] = Rgba {
            r: 127,
            g: 127,
            b: 127,
            a: 255,
        };

        let mut bank16 = [Rgba::BLACK; 16];
        bank16[0] = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        for (i, slot) in bank16.iter_mut().enumerate().skip(1) {
            let level = if i < 8 { 255 } else { 127 };
            *slot = Rgba {
                r: if i & 1 != 0 { level } else { 0 },
                g: if i & 2 != 0 { level } else { 0 },
                b: if i & 4 != 0 { level } else { 0 },
                a: 255,
            };
        }

        let mut bank256 = [Rgba::BLACK; 256];
        bank256[0] = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        for i in 1..256 {
            let i = i as u8;
            bank256[i as usize] = if i < 8 {
                Rgba {
                    r: if i & 1 != 0 { 255 } else { 0 },
                    g: if i & 2 != 0 { 255 } else { 0 },
                    b: if i & 4 != 0 { 255 } else { 0 },
                    a: 63,
                }
            } else {
                match i & 0x88 {
                    0x00 => Rgba {
                        r: if i & 1 != 0 { 85 } else { 0 } + if i & 0x10 != 0 { 170 } else { 0 },
                        g: if i & 2 != 0 { 85 } else { 0 } + if i & 0x20 != 0 { 170 } else { 0 },
                        b: if i & 4 != 0 { 85 } else { 0 } + if i & 0x40 != 0 { 170 } else { 0 },
                        a: 255,
                    },
                    0x08 => Rgba {
                        r: if i & 1 != 0 { 85 } else { 0 } + if i & 0x10 != 0 { 170 } else { 0 },
                        g: if i & 2 != 0 { 85 } else { 0 } + if i & 0x20 != 0 { 170 } else { 0 },
                        b: if i & 4 != 0 { 85 } else { 0 } + if i & 0x40 != 0 { 170 } else { 0 },
                        a: 127,
                    },
                    0x80 => Rgba {
                        r: 127
                            + if i & 1 != 0 { 43 } else { 0 }
                            + if i & 0x10 != 0 { 85 } else { 0 },
                        g: 127
                            + if i & 2 != 0 { 43 } else { 0 }
                            + if i & 0x20 != 0 { 85 } else { 0 },
                        b: 127
                            + if i & 4 != 0 { 43 } else { 0 }
                            + if i & 0x40 != 0 { 85 } else { 0 },
                        a: 255,
                    },
                    _ => Rgba {
                        r: if i & 1 != 0 { 43 } else { 0 } + if i & 0x10 != 0 { 85 } else { 0 },
                        g: if i & 2 != 0 { 43 } else { 0 } + if i & 0x20 != 0 { 85 } else { 0 },
                        b: if i & 4 != 0 { 43 } else { 0 } + if i & 0x40 != 0 { 85 } else { 0 },
                        a: 255,
                    },
                }
            };
        }

        Clut {
            version,
            bank4,
            bank16,
            bank256,
        }
    }

    /// The bank for a region depth (2 => 4 entries, 4 => 16, 8 => 256).
    pub fn bank(&self, depth: u8) -> &[Rgba] {
        match depth {
            2 => &self.bank4,
            4 => &self.bank16,
            _ => &self.bank256,
        }
    }
}

/// One CLUT definition entry, pre-extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClutEntry {
    pub entry_id: u8,
    /// Depth bits from the flag byte (`0x80` = 2-bit bank, `0x40` =
    /// 4-bit, `0x20` = 8-bit) — EN 300 743 names them "2/4/8 bit".
    pub depth_2: bool,
    pub depth_4: bool,
    pub depth_8: bool,
    pub full_range: bool,
    pub y: u8,
    pub cb: u8,
    pub cr: u8,
    /// DVB T-value (0 = opaque, 255 = transparent).
    pub alpha: u8,
}

/// FIX() constants, SCALEBITS = 10 (see module docs).
const FIX_R_CR: i32 = 1634; // round(1.40200 * 255/224 * 1024)
const FIX_G_CB: i32 = 401; // round(0.34414 * 255/224 * 1024)
const FIX_G_CR: i32 = 832; // round(0.71414 * 255/224 * 1024)
const FIX_B_CB: i32 = 2066; // round(1.77200 * 255/224 * 1024)
const FIX_Y: i32 = 1192; // round(255/219 * 1024)
const ONE_HALF: i32 = 512;

#[inline]
fn clamp255(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// YCbCr(601 headroom) + DVB T-value → straight-alpha RGBA8.
pub(crate) fn ycbcr_to_rgba(y: u8, cb: u8, cr: u8, alpha: u8) -> Rgba {
    // Y == 0 forces full transparency (EN 300 743 §7.2.4.2 — the
    // reference decoders apply it before the alpha).
    let t = if y == 0 { 0xFF } else { alpha };
    let cb = i32::from(cb) - 128;
    let cr = i32::from(cr) - 128;
    let r_add = FIX_R_CR * cr + ONE_HALF;
    let g_add = -FIX_G_CB * cb - FIX_G_CR * cr + ONE_HALF;
    let b_add = FIX_B_CB * cb + ONE_HALF;
    let luma = (i32::from(y) - 16) * FIX_Y;
    Rgba {
        r: clamp255((luma + r_add) >> 10),
        g: clamp255((luma + g_add) >> 10),
        b: clamp255((luma + b_add) >> 10),
        // DVB T: 0 = opaque.
        a: 255 - t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_is_opaque_white_is_white() {
        // Full-range white: Y=235 Cb=Cr=128 T=0 -> opaque (255,255,255,255).
        let c = ycbcr_to_rgba(235, 128, 128, 0);
        assert_eq!((c.r, c.g, c.b, c.a), (255, 255, 255, 255));
        // Full-range black: Y=16 -> opaque (0,0,0,255).
        let c = ycbcr_to_rgba(16, 128, 128, 0);
        assert_eq!((c.r, c.g, c.b, c.a), (0, 0, 0, 255));
    }

    #[test]
    fn y_zero_is_transparent_even_when_t_says_opaque() {
        let c = ycbcr_to_rgba(0, 128, 128, 0);
        assert_eq!(c.a, 0);
    }

    #[test]
    fn alpha_is_inverted_and_clamped() {
        let c = ycbcr_to_rgba(235, 128, 128, 255);
        assert_eq!(c.a, 0, "T=255 means fully transparent");
        let c = ycbcr_to_rgba(235, 128, 128, 128);
        assert_eq!(c.a, 127);
    }

    #[test]
    fn red_and_blue_primaries() {
        // Studio-range red: Y=81 Cb=90 Cr=240 -> bright red, ~0 g/b.
        let c = ycbcr_to_rgba(81, 90, 240, 0);
        assert!(c.r > 200 && c.g < 60 && c.b < 60, "red-ish, got {c:?}");
        // Blue: Cb high, Cr low.
        let c = ycbcr_to_rgba(81, 240, 90, 0);
        assert!(c.b > 200 && c.r < 60, "blue-ish, got {c:?}");
    }

    #[test]
    fn entry_depth_flags_route_to_their_banks() {
        // Blue (Y=41 Cb=240 Cr=110): no default bank entry is blue,
        // so a hit is unambiguous. 0x80 = 2-bit bank only.
        let e2 = ClutEntry {
            entry_id: 1,
            depth_2: true,
            depth_4: false,
            depth_8: false,
            full_range: true,
            y: 41,
            cb: 240,
            cr: 110,
            alpha: 0,
        };
        let clut = Clut::from_entries(0, std::iter::once(e2));
        let blue = Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        };
        assert_eq!(clut.bank4[1], blue, "2-bit flagged entry hits bank4");
        assert_ne!(clut.bank16[1], blue, "4-bit bank untouched");
        assert_ne!(clut.bank256[1], blue, "8-bit bank untouched");
        assert_ne!(clut.bank4[3], blue, "unflagged id untouched");
        // 0x20 = 8-bit bank only.
        let e8 = ClutEntry {
            entry_id: 1,
            depth_2: false,
            depth_4: false,
            depth_8: true,
            full_range: true,
            y: 41,
            cb: 240,
            cr: 110,
            alpha: 0,
        };
        let clut = Clut::from_entries(0, std::iter::once(e8));
        assert_eq!(clut.bank256[1], blue, "8-bit flagged entry hits bank256");
        assert_ne!(clut.bank4[1], blue, "2-bit bank untouched");
        // Both flags alias both banks (common: 0xC0).
        let e24 = ClutEntry {
            entry_id: 2,
            depth_2: true,
            depth_4: true,
            depth_8: false,
            full_range: true,
            y: 41,
            cb: 240,
            cr: 110,
            alpha: 0,
        };
        let clut = Clut::from_entries(0, std::iter::once(e24));
        assert_eq!(clut.bank4[2], blue);
        assert_eq!(clut.bank16[2], blue);
        assert_ne!(clut.bank256[2], blue);
    }

    #[test]
    fn default_banks_match_reference_shape() {
        let d = Clut::default_banks();
        assert_eq!(
            d.bank4[1],
            Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
        assert_eq!(
            d.bank16[7],
            Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
        assert_eq!(
            d.bank16[8],
            Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
        );
        assert_eq!(
            d.bank256[1],
            Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 63
            }
        );
        assert_eq!(d.bank256[0].a, 0);
    }

    #[test]
    fn limited_range_entries_unpack() {
        // The 3-byte limited-range packing (EN 300 743 §7.2.4):
        // Y(6) Cr(6) Cb(6) T(2) MSB-first, high-aligned into 8-bit
        // components, 4 trailing padding bits.
        let b0 = 0b1010_0110u8;
        let b1 = 0b1101_0011u8;
        let b2 = 0b0110_1001u8;
        let y = b0 & 0xFC;
        let cr = (((b0 & 0x03) << 4) | (b1 >> 4)) << 2;
        let cb = (((b1 & 0x0F) << 2) | (b2 >> 6)) << 2;
        let alpha = ((b2 >> 4) & 0x03) << 6;
        assert_eq!(y, 0b1010_0100);
        assert_eq!(cr, 0b1011_0100);
        assert_eq!(cb, 0b0011_0100);
        assert_eq!(alpha, 0b1000_0000);
    }
}
