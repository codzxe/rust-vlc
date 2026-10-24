//! Byte-to-text decoding for SRT (card 5.M1): BOM detection over the
//! three real-world encodings (UTF-8, UTF-16LE, UTF-16BE), UTF-8
//! fallback, and line-ending normalization — all std-only (C5), all
//! damage-tolerant, never panicking.
//!
//! ## Detection policy
//!
//! * `EF BB BF` → UTF-8 with BOM (BOM stripped);
//! * `FF FE` → UTF-16LE (BOM stripped);
//! * `FE FF` → UTF-16BE (BOM stripped);
//! * no BOM → UTF-8 (the overwhelming modern default). Invalid
//!   sequences degrade to U+FFFD via the lossy conversion with one
//!   [`SrtDamageKind::InvalidUtf8`] event — the parse continues.
//!
//! BOM-less UTF-16 is deliberately NOT sniffed heuristically: the
//! NUL-interleaved garbage it produces in a UTF-8 read degrades into
//! garbage-block damage events instead of a mis-detected document
//! (the honest failure), and every real UTF-16 SRT in the wild —
//! every Windows-era tool that emitted UTF-16 — carries the BOM.

use crate::srt::damage::{DamageSink, SrtDamageKind};

/// The encoding a source was decoded as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceEncoding {
    /// UTF-8, no BOM (the fallback when no BOM is present).
    Utf8,
    /// UTF-8 with a BOM (BOM detected and stripped).
    Utf8Bom,
    /// UTF-16 little-endian (BOM detected and stripped).
    Utf16Le,
    /// UTF-16 big-endian (BOM detected and stripped).
    Utf16Be,
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

/// Decode `bytes` per the BOM policy above, recording damage.
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
        // No BOM (a `Some` for the other arms is exhaustive above; a
        // `None` and a hypothetical future variant both fall back).
        _ => DecodedSource {
            text: decode_utf8(bytes, damage),
            encoding: SourceEncoding::Utf8,
        },
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
    fn invalid_utf8_falls_back_lossily() {
        let (d, dmg) = decode_parts(b"1\n\xff\xfe bad\n");
        assert_eq!(d.encoding, SourceEncoding::Utf8);
        assert_eq!(d.text, "1\n\u{FFFD}\u{FFFD} bad\n");
        assert_eq!(dmg, vec![SrtDamageKind::InvalidUtf8]);
    }

    #[test]
    fn line_endings_normalize() {
        assert_eq!(normalize_line_endings("a\r\nb\rc\nd"), "a\nb\nc\nd");
        assert_eq!(normalize_line_endings("\r\r\n"), "\n\n");
        assert_eq!(normalize_line_endings(""), "");
    }
}
