//! Byte-to-text decoding for SRT (card 5.M1): BOM detection over the
//! three real-world encodings (UTF-8, UTF-16LE, UTF-16BE), UTF-8
//! validity with a Windows-1252 8-bit fallback, and line-ending
//! normalization — all std-only (C5), all damage-tolerant, never
//! panicking.
//!
//! ## Detection policy
//!
//! * `EF BB BF` → UTF-8 with BOM (BOM stripped);
//! * `FF FE` → UTF-16LE (BOM stripped);
//! * `FE FF` → UTF-16BE (BOM stripped);
//! * no BOM, VALID UTF-8 → UTF-8 (lossless — the overwhelming modern
//!   default, and the only no-BOM path that needs no heuristic);
//! * no BOM, INVALID UTF-8 → **Windows-1252** (the common 8-bit
//!   fallback for legacy SRTs — ffmpeg's `sub_charenc` behaviour).
//!   One [`SrtDamageKind::Windows1252Fallback`] event records the
//!   heuristic decision; the five undefined bytes (0x81/0x8D/0x8F/
//!   0x90/0x9D) degrade to U+FFFD with
//!   [`SrtDamageKind::UndefinedWindows1252Byte`] events.
//!   `[VERIFY]` against a chardet-grade corpus once one is licensed
//!   in — the heuristic is single-shot, no confidence scoring.
//!
//! BOM-less UTF-16 is deliberately NOT sniffed heuristically: the
//! NUL-interleaved garbage it produces in a UTF-8 read degrades into
//! garbage-block damage events instead of a mis-detected document
//! (the honest failure), and every real UTF-16 SRT in the wild —
//! every Windows-era tool that emitted UTF-16 — carries the BOM.
//! (Windows-1252 never produces NUL bytes, so the two policies never
//! collide.)

use crate::srt::damage::{DamageSink, SrtDamageKind};

/// The encoding a source was decoded as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceEncoding {
    /// UTF-8, no BOM (the fallback when the bytes are valid UTF-8).
    Utf8,
    /// UTF-8 with a BOM (BOM detected and stripped).
    Utf8Bom,
    /// UTF-16 little-endian (BOM detected and stripped).
    Utf16Le,
    /// UTF-16 big-endian (BOM detected and stripped).
    Utf16Be,
    /// Windows-1252 — the 8-bit fallback when no BOM is present and
    /// the bytes are not valid UTF-8 (legacy SRTs).
    Windows1252,
}

impl SourceEncoding {
    /// Stable name for diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf8Bom => "utf-8-bom",
            Self::Utf16Le => "utf-16le",
            Self::Utf16Be => "utf-16be",
            Self::Windows1252 => "windows-1252",
        }
    }
}

/// A decoded source: the text (line endings NOT yet normalized) plus
/// the encoding it was decoded as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSource {
    /// The decoded text.
    pub text: String,
    /// The encoding that was detected/used.
    pub encoding: SourceEncoding,
}

/// Detect a leading BOM, if any.
#[must_use]
pub fn detect_bom(bytes: &[u8]) -> Option<SourceEncoding> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some(SourceEncoding::Utf8Bom)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Some(SourceEncoding::Utf16Le)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some(SourceEncoding::Utf16Be)
    } else {
        None
    }
}

/// Decode `bytes` per the detection policy above, recording damage.
#[must_use]
pub fn decode(bytes: &[u8], damage: &mut DamageSink) -> DecodedSource {
    match detect_bom(bytes) {
        Some(SourceEncoding::Utf8Bom) => DecodedSource {
            text: decode_utf8(&bytes[3..], damage),
            encoding: SourceEncoding::Utf8Bom,
        },
        Some(SourceEncoding::Utf16Le) => DecodedSource {
            text: decode_utf16(&bytes[2..], true, damage),
            encoding: SourceEncoding::Utf16Le,
        },
        Some(SourceEncoding::Utf16Be) => DecodedSource {
            text: decode_utf16(&bytes[2..], false, damage),
            encoding: SourceEncoding::Utf16Be,
        },
        // No BOM: valid UTF-8 wins losslessly; only an invalid byte
        // stream falls to the Windows-1252 heuristic.
        _ if std::str::from_utf8(bytes).is_ok() => DecodedSource {
            // Reuse the lossy path purely for its stray-BOM strip
            // (the validation above guarantees no replacement).
            text: decode_utf8(bytes, damage),
            encoding: SourceEncoding::Utf8,
        },
        _ => {
            damage.record(SrtDamageKind::Windows1252Fallback, 0);
            DecodedSource {
                text: decode_windows1252(bytes, damage),
                encoding: SourceEncoding::Windows1252,
            }
        }
    }
}

/// UTF-8 with lossy degradation + one damage event when invalid.
fn decode_utf8(bytes: &[u8], damage: &mut DamageSink) -> String {
    if std::str::from_utf8(bytes).is_err() {
        damage.record(SrtDamageKind::InvalidUtf8, 0);
    }
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    // A stray leading U+FEFF (double BOM, dirty world) pollutes the
    // first index line — strip one.
    if text.starts_with('\u{FEFF}') {
        text.replace_range(0..'\u{FEFF}'.len_utf8(), "");
    }
    text
}

/// The Windows-1252 high-half table, `0x80..=0x9F` (32 entries;
/// `None` = the five undefined bytes). `0x00..=0x7F` is ASCII and
/// `0xA0..=0xFF` is Latin-1 — both handled inline, table-free.
const CP1252_HIGH: [Option<char>; 32] = [
    Some('\u{20AC}'),
    None,
    Some('\u{201A}'),
    Some('\u{0192}'),
    Some('\u{201E}'),
    Some('\u{2026}'),
    Some('\u{2020}'),
    Some('\u{2021}'),
    Some('\u{02C6}'),
    Some('\u{2030}'),
    Some('\u{0160}'),
    Some('\u{2039}'),
    Some('\u{0152}'),
    None,
    Some('\u{017D}'),
    None,
    None,
    Some('\u{2018}'),
    Some('\u{2019}'),
    Some('\u{201C}'),
    Some('\u{201D}'),
    Some('\u{2022}'),
    Some('\u{2013}'),
    Some('\u{2014}'),
    Some('\u{02DC}'),
    Some('\u{2122}'),
    Some('\u{0161}'),
    Some('\u{203A}'),
    Some('\u{0153}'),
    None,
    Some('\u{017E}'),
    Some('\u{0178}'),
];

/// Windows-1252 decode: ASCII and Latin-1 ranges pass through, the
/// high half maps through [`CP1252_HIGH`], undefined bytes degrade
/// to U+FFFD with damage. One pass, one output allocation.
fn decode_windows1252(bytes: &[u8], damage: &mut DamageSink) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            0x80..=0x9F => match CP1252_HIGH[usize::from(b - 0x80)] {
                Some(c) => out.push(c),
                None => {
                    damage.record(SrtDamageKind::UndefinedWindows1252Byte, 0);
                    out.push('\u{FFFD}');
                }
            },
            // Every other byte maps to the same char as Latin-1
            // (ASCII included — no table needed).
            _ => out.push(char::from(b)),
        }
    }
    if out.starts_with('\u{FEFF}') {
        out.replace_range(0..'\u{FEFF}'.len_utf8(), "");
    }
    out
}

/// UTF-16 decode: surrogate pairs joined, unpaired surrogates and the
/// odd trailing byte degraded with damage. One pass, one output
/// allocation sized to the code-unit count.
fn decode_utf16(bytes: &[u8], little_endian: bool, damage: &mut DamageSink) -> String {
    let unit = |pair: [u8; 2]| -> u16 {
        if little_endian {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        }
    };
    let mut out = String::with_capacity(bytes.len() / 2 + 1);
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let u = unit([bytes[i], bytes[i + 1]]);
        i += 2;
        if (0xD800..=0xDBFF).contains(&u) {
            // High surrogate: needs a following low surrogate.
            if i + 2 <= bytes.len() {
                let v = unit([bytes[i], bytes[i + 1]]);
                if (0xDC00..=0xDFFF).contains(&v) {
                    i += 2;
                    let c = 0x1_0000u32 + ((u as u32 - 0xD800) << 10) + (v as u32 - 0xDC00);
                    // In range by construction; the fallback is unreachable.
                    if let Some(c) = char::from_u32(c) {
                        out.push(c);
                        continue;
                    }
                }
            }
            damage.record(SrtDamageKind::UnpairedUtf16Surrogate, 0);
            out.push('\u{FFFD}');
        } else if (0xDC00..=0xDFFF).contains(&u) {
            damage.record(SrtDamageKind::UnpairedUtf16Surrogate, 0);
            out.push('\u{FFFD}');
        } else {
            // Every non-surrogate u16 maps to a char.
            out.push(char::from_u32(u as u32).unwrap_or('\u{FFFD}'));
        }
    }
    if i < bytes.len() {
        damage.record(SrtDamageKind::OddTrailingUtf16Byte, 0);
    }
    out
}

/// Normalize line endings to `\n`: CRLF → LF, lone CR → LF (classic
/// Mac and Windows files both occur; UTF-16 SRTs are near-always
/// CRLF). One pass over the chars, output ≤ input length.
#[must_use]
pub fn normalize_line_endings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            // Consume the LF of a CRLF pair in the same iteration.
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{DecodedSource, SourceEncoding, decode, detect_bom, normalize_line_endings};
    use crate::srt::damage::{DamageSink, SrtDamageKind};

    fn decode_parts(bytes: &[u8]) -> (DecodedSource, Vec<SrtDamageKind>) {
        let mut sink = DamageSink::new();
        let decoded = decode(bytes, &mut sink);
        let (events, _) = sink.into_parts();
        (decoded, events.into_iter().map(|e| e.kind).collect())
    }

    #[test]
    fn bom_detection_matrix() {
        assert_eq!(
            detect_bom(&[0xEF, 0xBB, 0xBF, b'1']),
            Some(SourceEncoding::Utf8Bom)
        );
        assert_eq!(
            detect_bom(&[0xFF, 0xFE, b'1']),
            Some(SourceEncoding::Utf16Le)
        );
        assert_eq!(
            detect_bom(&[0xFE, 0xFF, 0, b'1']),
            Some(SourceEncoding::Utf16Be)
        );
        assert_eq!(detect_bom(b"1\n"), None);
        // A UTF-16BE BOM must not be confused with the UTF-8 BOM's
        // prefix or vice versa.
        assert_eq!(detect_bom(&[0xEF, 0xBB]), None);
    }

    #[test]
    fn utf8_bom_is_stripped() {
        let (d, dmg) = decode_parts("\u{FEFF}1\n00:00:01,000 --> 00:00:02,000\nhi\n".as_bytes());
        assert_eq!(d.encoding, SourceEncoding::Utf8Bom);
        assert!(d.text.starts_with('1'));
        assert!(dmg.is_empty());
    }

    #[test]
    fn utf16_le_and_be_decode_identically() {
        let sample = "1\n00:00:01,000 --> 00:00:02,000\r\nhéllo → ünïcode\n";
        let mut le = vec![0xFF, 0xFE];
        let mut be = vec![0xFE, 0xFF];
        for unit in sample.encode_utf16() {
            le.extend_from_slice(&unit.to_le_bytes());
            be.extend_from_slice(&unit.to_be_bytes());
        }
        let (d_le, dmg_le) = decode_parts(&le);
        let (d_be, dmg_be) = decode_parts(&be);
        assert_eq!(d_le.encoding, SourceEncoding::Utf16Le);
        assert_eq!(d_be.encoding, SourceEncoding::Utf16Be);
        assert_eq!(d_le.text, sample);
        assert_eq!(d_be.text, sample);
        assert!(dmg_le.is_empty() && dmg_be.is_empty());
    }

    #[test]
    fn unpaired_surrogates_and_odd_tail_degrade() {
        // D800 with no following DC00-range unit, then an odd byte
        // (the final 0x42 is half a code unit — dropped with damage).
        let bytes = [0xFF, 0xFE, 0x00, 0xD8, b'A', 0x00, 0x42];
        let (d, dmg) = decode_parts(&bytes);
        assert_eq!(d.text, "\u{FFFD}A");
        assert_eq!(
            dmg,
            vec![
                SrtDamageKind::UnpairedUtf16Surrogate,
                SrtDamageKind::OddTrailingUtf16Byte
            ]
        );
    }

    #[test]
    fn invalid_utf8_falls_back_to_windows1252() {
        // No BOM, invalid UTF-8, but clean CP1252: 0xE9 = é, 0x93 = “.
        let (d, dmg) = decode_parts(b"1\ncaf\xe9 \x93quoted\x94\n");
        assert_eq!(d.encoding, SourceEncoding::Windows1252);
        assert_eq!(d.text, "1\ncaf\u{e9} \u{201C}quoted\u{201D}\n");
        assert_eq!(dmg, vec![SrtDamageKind::Windows1252Fallback]);
    }

    #[test]
    fn windows1252_undefined_bytes_degrade() {
        // 0x81 is one of the five holes.
        let (d, dmg) = decode_parts(b"a\x81b\x8dc");
        assert_eq!(d.encoding, SourceEncoding::Windows1252);
        assert_eq!(d.text, "a\u{FFFD}b\u{FFFD}c");
        assert_eq!(
            dmg,
            vec![
                SrtDamageKind::Windows1252Fallback,
                SrtDamageKind::UndefinedWindows1252Byte,
                SrtDamageKind::UndefinedWindows1252Byte
            ]
        );
    }

    #[test]
    fn valid_multibyte_utf8_stays_utf8() {
        // Valid UTF-8 multibyte must NOT be misread as CP1252.
        let (d, dmg) = decode_parts("1\ncaf\u{e9} \u{201C}q\u{201D}\n".as_bytes());
        assert_eq!(d.encoding, SourceEncoding::Utf8);
        assert_eq!(d.text, "1\ncaf\u{e9} \u{201C}q\u{201D}\n");
        assert!(dmg.is_empty());
    }

    #[test]
    fn bomd_utf8_invalid_payload_stays_lossy_utf8() {
        // A UTF-8 BOM commits to UTF-8: an invalid payload degrades
        // lossily, it does not switch to CP1252.
        let (d, dmg) = decode_parts(&[0xEF, 0xBB, 0xBF, b'a', 0xE9, b'b']);
        assert_eq!(d.encoding, SourceEncoding::Utf8Bom);
        assert_eq!(d.text, "a\u{FFFD}b");
        assert_eq!(dmg, vec![SrtDamageKind::InvalidUtf8]);
    }

    #[test]
    fn line_endings_normalize() {
        assert_eq!(normalize_line_endings("a\r\nb\rc\nd"), "a\nb\nc\nd");
        assert_eq!(normalize_line_endings("\r\r\n"), "\n\n");
        assert_eq!(normalize_line_endings(""), "");
    }
}
