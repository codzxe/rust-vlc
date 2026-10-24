//! The style-token subset \[PROPOSAL\] (card 5.L2): bold, italic,
//! underline, color — deliberately the SRT-trivial run set (doc 04 §5
//! row 1) plus WebVTT's `color`. Karaoke, fonts, size and the rest of
//! the tag zoo are NOT modeled yet; 5.L1's parsers degrade them
//! honestly per doc 06 §7.8 instead of silently dropping them here.

use crate::model::error::ModelError;

/// An RGBA color, 8 bits per channel, straight (non-premultiplied)
/// alpha. Copy — 4 bytes, stack-friendly in span tables (D-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red, `0..=255`.
    pub r: u8,
    /// Green, `0..=255`.
    pub g: u8,
    /// Blue, `0..=255`.
    pub b: u8,
    /// Alpha, `0..=255` (255 = opaque).
    pub a: u8,
}

impl Color {
    /// Construct from channels.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    /// Append the lowercase hex form `#rrggbbaa` to `out` — no
    /// intermediate allocation (D-2: the serializer hot path writes
    /// straight into the output buffer).
    pub fn write_hex(self, out: &mut String) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        out.push('#');
        for byte in [self.r, self.g, self.b, self.a] {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }

    /// Parse `#rrggbbaa` (lower- or uppercase hex, exactly 9 bytes).
    ///
    /// # Errors
    /// [`ModelError::BadColor`] on any malformed literal.
    pub fn from_hex(s: &str) -> Result<Self, ModelError> {
        let bad = || ModelError::BadColor;
        let bytes = s.as_bytes();
        if bytes.len() != 9 || bytes[0] != b'#' {
            return Err(bad());
        }
        let mut channels = [0u8; 4];
        for (i, slot) in channels.iter_mut().enumerate() {
            let hi = hex_val(bytes[1 + i * 2]).ok_or_else(bad)?;
            let lo = hex_val(bytes[2 + i * 2]).ok_or_else(bad)?;
            *slot = (hi << 4) | lo;
        }
        Ok(Self {
            r: channels[0],
            g: channels[1],
            b: channels[2],
            a: channels[3],
        })
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// The style-token subset \[PROPOSAL\]: bold / italic / underline /
/// optional RGBA color. Copy + `Eq` — a span table is an array of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextStyle {
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underline.
    pub underline: bool,
    /// Text color; `None` = inherit the track style.
    pub color: Option<Color>,
}

impl TextStyle {
    /// Construct from flags.
    pub const fn new(bold: bool, italic: bool, underline: bool, color: Option<Color>) -> Self {
        Self {
            bold,
            italic,
            underline,
            color,
        }
    }

    /// All flags off, no color override.
    pub const PLAIN: Self = Self {
        bold: false,
        italic: false,
        underline: false,
        color: None,
    };
}

#[cfg(test)]
mod tests {
    use super::{Color, TextStyle};

    #[test]
    fn hex_roundtrip() {
        let mut s = String::new();
        Color::new(0x0a, 0xbc, 0xde, 0xff).write_hex(&mut s);
        assert_eq!(s, "#0abcdeff");
        assert_eq!(Color::from_hex(&s), Ok(Color::new(0x0a, 0xbc, 0xde, 0xff)));
        // Uppercase accepted.
        assert_eq!(
            Color::from_hex("#0ABCDEFF"),
            Ok(Color::new(0x0a, 0xbc, 0xde, 0xff))
        );
    }

    #[test]
    fn hex_rejects_malformed() {
        for bad in [
            "",
            "#fff",
            "#fffffff",
            "fffffff",
            "#gggggggg",
            "#0abcdeff extra",
        ] {
            assert!(Color::from_hex(bad).is_err(), "{bad} should be invalid");
        }
    }

    #[test]
    fn plain_default_matches() {
        assert_eq!(TextStyle::default(), TextStyle::PLAIN);
        let styled = TextStyle::new(true, false, true, None);
        assert!(styled.bold);
        assert!(!styled.italic);
        assert!(styled.underline);
    }
}
