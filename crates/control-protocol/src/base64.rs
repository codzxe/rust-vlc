//! In-tree base64 (RFC 4648 §4, standard alphabet) — card 7.J1.
//!
//! Two call sites, both cold-path: the `Sec-WebSocket-Accept` encoding
//! and the session-token rendering. Strict decode: exactly the standard
//! alphabet, mandatory canonical padding, no whitespace — a dirty-world
//! input is a typed [`Base64Error`], never a panic (D-3).
//!
//! Implemented in-crate rather than pulling a `base64` crate (C5 /
//! ADR-011 ethos). Pinned by the RFC 4648 test vectors.

/// Decode failures (typed; no raw input echoed beyond lengths).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base64Error {
    /// A character outside the standard alphabet (offset recorded).
    InvalidSymbol { offset: usize },
    /// Padding in a wrong position or amount.
    BadPadding,
    /// A length that cannot be a multiple of 4 with canonical padding.
    BadLength { len: usize },
}

impl std::fmt::Display for Base64Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Base64Error::InvalidSymbol { offset } => {
                write!(f, "invalid base64 symbol at offset {offset}")
            }
            Base64Error::BadPadding => write!(f, "non-canonical base64 padding"),
            Base64Error::BadLength { len } => {
                write!(f, "base64 length {len} is not a multiple of 4")
            }
        }
    }
}

impl std::error::Error for Base64Error {}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode to standard base64 with `=` padding (RFC 4648 §4).
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        // Pack up to 3 bytes into a 24-bit big-endian window; the four
        // 6-bit groups (top first) are the output quartet, with the
        // 1-byte tail yielding "XX==" and the 2-byte tail "XXX=".
        let bytes = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let window = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);
        let quartet = [
            ALPHABET[(window >> 18) as usize & 0x3f] as char,
            ALPHABET[(window >> 12) as usize & 0x3f] as char,
            ALPHABET[(window >> 6) as usize & 0x3f] as char,
            ALPHABET[window as usize & 0x3f] as char,
        ];
        match chunk.len() {
            3 => {
                out.push(quartet[0]);
                out.push(quartet[1]);
                out.push(quartet[2]);
                out.push(quartet[3]);
            }
            2 => {
                for ch in quartet.iter().take(3) {
                    out.push(*ch);
                }
                out.push('=');
            }
            _ => {
                for ch in quartet.iter().take(2) {
                    out.push(*ch);
                }
                out.push_str("==");
            }
        }
    }
    out
}

/// Decode strict standard base64 (RFC 4648 §4): canonical padding only.
///
/// # Errors
/// Typed [`Base64Error`] on any non-alphabet byte, misplaced padding,
/// non-canonical trailing bits or a bad length — never a panic.
pub fn decode(text: &str) -> Result<Vec<u8>, Base64Error> {
    let bytes = text.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err(Base64Error::BadLength { len: bytes.len() });
    }
    let pad = bytes.iter().rev().take_while(|b| **b == b'=').count();
    if pad > 2 {
        return Err(Base64Error::BadPadding);
    }
    // '=' may appear only as the trailing run, and the symbol count in
    // front of it must match the padding (mod 4 = 4-pad).
    let symbols = &bytes[..bytes.len() - pad];
    if symbols.iter().any(|b| *b == b'=') {
        return Err(Base64Error::BadPadding);
    }
    if pad != 0 && symbols.len() % 4 != 4 - pad {
        return Err(Base64Error::BadPadding);
    }
    let mut out = Vec::with_capacity(symbols.len() / 4 * 3 + 2);
    let full_groups = symbols.len() / 4;
    for (index, group) in symbols[..full_groups * 4].chunks_exact(4).enumerate() {
        let window = group_window(group, index * 4)?;
        out.push((window >> 16) as u8);
        out.push((window >> 8) as u8);
        out.push(window as u8);
    }
    // The padded tail: 3 symbols ⇒ 2 bytes, 2 symbols ⇒ 1 byte, and the
    // unused low bits must be zero (canonical form).
    let tail = &symbols[full_groups * 4..];
    match (tail.len(), pad) {
        (0, _) => {}
        (3, 1) => {
            let window = group_window(tail, full_groups * 4)?;
            if window & 0xff != 0 {
                return Err(Base64Error::BadPadding);
            }
            out.push((window >> 16) as u8);
            out.push((window >> 8) as u8);
        }
        (2, 2) => {
            let window = group_window(tail, full_groups * 4)?;
            if window & 0xffff != 0 {
                return Err(Base64Error::BadPadding);
            }
            out.push((window >> 16) as u8);
        }
        _ => return Err(Base64Error::BadPadding),
    }
    Ok(out)
}

/// Fold one group of 2–4 alphabet symbols into its bit window.
fn group_window(group: &[u8], base_offset: usize) -> Result<u32, Base64Error> {
    let mut window: u32 = 0;
    for (j, symbol) in group.iter().enumerate() {
        let value = symbol_value(*symbol)
            .ok_or(Base64Error::InvalidSymbol { offset: base_offset + j })?;
        window = (window << 6) | value as u32;
    }
    // A short group sits at the TOP of its bit window (the symbols are
    // the most-significant 6-bit groups of the final 24-bit block).
    window <<= 6 * (4 - group.len()) as u32;
    Ok(window)
}

/// Alphabet position of one symbol, `None` when not in the alphabet.
fn symbol_value(symbol: u8) -> Option<u8> {
    match symbol {
        b'A'..=b'Z' => Some(symbol - b'A'),
        b'a'..=b'z' => Some(symbol - b'a' + 26),
        b'0'..=b'9' => Some(symbol - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4648 §10 test vectors (the external oracle — D-5).
    #[test]
    fn rfc4648_vectors() {
        let cases: &[(&str, &str)] = &[
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, encoded) in cases {
            assert_eq!(encode(plain.as_bytes()), *encoded, "encoding {plain:?}");
            match decode(encoded) {
                Ok(bytes) => assert_eq!(bytes, plain.as_bytes(), "decoding {encoded:?}"),
                Err(err) => unreachable!("vector {encoded} failed: {err}"),
            }
        }
    }

    #[test]
    fn roundtrip_all_byte_values() {
        let data: Vec<u8> = (0u8..=255).collect();
        for len in 0..data.len() {
            let encoded = encode(&data[..len]);
            match decode(&encoded) {
                Ok(back) => assert_eq!(back, &data[..len], "len {len}"),
                Err(err) => unreachable!("len {len} failed: {err}"),
            }
        }
    }

    #[test]
    fn dirty_inputs_are_typed_errors() {
        assert_eq!(decode("Zm9!"), Err(Base64Error::InvalidSymbol { offset: 3 }));
        assert_eq!(decode("Zm9v!"), Err(Base64Error::BadLength { len: 5 }));
        assert_eq!(decode("Z"), Err(Base64Error::BadLength { len: 1 }));
        assert_eq!(decode("Zg="), Err(Base64Error::BadLength { len: 3 }));
        assert_eq!(decode("=g=="), Err(Base64Error::BadPadding)); // '=' outside the trailing run
        assert_eq!(decode("Z==="), Err(Base64Error::BadPadding));
        assert_eq!(decode("Zh=="), Err(Base64Error::BadPadding)); // non-canonical tail bits
        assert_eq!(decode("Zm 9"), Err(Base64Error::InvalidSymbol { offset: 2 })); // whitespace
    }
}
