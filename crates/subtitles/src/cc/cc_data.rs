//! The ATSC `cc_data()` carriage layer (card 5.N3 step 1).
//!
//! # What lands here
//!
//! The bit grammar that carries CEA-608/CEA-708 caption bytes inside
//! container/codec user data — nothing about the caption semantics
//! themselves (those are [`super::eia608`] and [`super::cea708`]):
//!
//! * **GA94 SEI form** ([`parse_ga94_sei`]) — the H.264 SEI type-4
//!   `user_data_registered_itu_t_t35` payload as ATSC A/53 Part 4 /
//!   CTA-708 define it: country code `0xB5`, provider code `0x0031`,
//!   the `GA94` user identifier, `user_data_type_code 0x03`, then the
//!   `cc_data()` body. The codecs crate's SEI parser captures type-4
//!   messages intact and skipped (`SeiMessage::Unparsed`, the
//!   read-only seam named by ADR-012 "extraction surface (a)") —
//!   this is the consuming half.
//! * **MPEG-2 picture `user_data` GA94 form** ([`parse_mpeg2_ga94`]) —
//!   the same `GA94` + `cc_data()` body after the `0x000001B2` start
//!   code (the ADR's "MPEG-2 user_data carriage" gap: this parser is
//!   the decode half; the mpeg12 dispatch is the Lead's wiring).
//! * **SCTE-20 form** ([`parse_scte20`]) — the bit-packed carriage of
//!   SCTE 20: after the `0x03` user_data_type_code the pairs ride
//!   WITHOUT parity — each byte is 7 data bits padded with one
//!   reserved bit — closed by `1111` marker nibbles. Odd parity is
//!   re-synthesized here so the 608 layer applies one uniform parity
//!   rule.
//! * the shared **[`CcDataTriplet`]** — `cc_valid`, `cc_type`
//!   (0/1 = 608 field 1/2, 2 = 708 packet start, 3 = continuation)
//!   and the two payload bytes.
//!
//! # Grammar (ATSC `cc_data()`, byte-aligned form)
//!
//! ```text
//! cc_data() {
//!   reserved               1  // '0' — tolerated if set
//!   process_cc_data_flag   1  // '0' => the payload is not captions
//!   zero_bit               1  // shall be '0' — typed refusal if set
//!   cc_count               5
//!   for (i = 0; i < cc_count; i++) {
//!     cc_valid             1
//!     cc_type              2
//!     cc_data_1            8  // MSB = odd parity (608 carriage)
//!     cc_data_2            8
//!   }
//!   marker bits            8  // 0xFF
//! }
//! ```
//!
//! Trailing bytes after the marker are tolerated and ignored (dirty
//! world: encoders append private data). Everything else is a typed
//! [`CcDataError`] — no panics (MP §8).

use super::eia608;

/// A single `cc_data()` triplet: the 2-byte caption payload plus its
/// transport tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CcDataTriplet {
    /// `cc_valid` — false triplets are padding and are skipped by the
    /// decode layer without damage.
    pub cc_valid: bool,
    /// `cc_type`: 0 = 608 field 1, 1 = 608 field 2 / XDS, 2 = 708
    /// DTVCC packet start, 3 = DTVCC continuation.
    pub cc_type: u8,
    /// The two payload bytes (`cc_data_1`, `cc_data_2`).
    pub data: [u8; 2],
}

impl CcDataTriplet {
    /// A valid field-1/field-2 608 pair with the given bytes.
    #[must_use]
    pub fn cc(field: u8, data: [u8; 2]) -> CcDataTriplet {
        CcDataTriplet {
            cc_valid: true,
            cc_type: field,
            data,
        }
    }
}

/// Typed `cc_data()` grammar failures (MP §8: data, never a panic).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CcDataError {
    /// The payload ended before the grammar was satisfied.
    Truncated {
        /// What the parser was reading.
        what: &'static str,
        /// Bytes the grammar needed.
        needed: usize,
        /// Bytes actually present.
        got: usize,
    },
    /// A non-US T.35 country code (only `0xB5` is routed to captions).
    BadCountryCode {
        /// The offending code.
        code: u8,
    },
    /// A non-ATSC T.35 provider code (only `0x0031` is).
    BadProviderCode {
        /// The offending code.
        code: u16,
    },
    /// The user identifier after the provider code was not `GA94`.
    BadUserIdentifier {
        /// The 4 offending bytes.
        got: [u8; 4],
    },
    /// The `user_data_type_code` was not closed captioning (`0x03`).
    BadTypeCode {
        /// The offending code.
        code: u8,
    },
    /// The `zero_bit` after the flags was set (shall be 0).
    BadZeroBit,
    /// The final marker byte was not `0xFF`.
    BadMarker {
        /// The offending byte.
        got: u8,
    },
}

impl std::fmt::Display for CcDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CcDataError::Truncated { what, needed, got } => {
                write!(
                    f,
                    "cc_data payload truncated at {what}: needed {needed} bytes, got {got}"
                )
            }
            CcDataError::BadCountryCode { code } => {
                write!(f, "T.35 country code {code:#04x} is not the ATSC 0xB5")
            }
            CcDataError::BadProviderCode { code } => {
                write!(f, "T.35 provider code {code:#06x} is not the ATSC 0x0031")
            }
            CcDataError::BadUserIdentifier { got } => write!(
                f,
                "T.35 user identifier {:?} is not GA94",
                got.map(|b| b as char)
            ),
            CcDataError::BadTypeCode { code } => {
                write!(
                    f,
                    "user_data_type_code {code:#04x} is not closed captioning (0x03)"
                )
            }
            CcDataError::BadZeroBit => f.write_str("cc_data zero_bit set (shall be 0)"),
            CcDataError::BadMarker { got } => {
                write!(f, "cc_data marker byte {got:#04x} is not 0xFF")
            }
        }
    }
}

impl std::error::Error for CcDataError {}

/// The `GA94` user identifier bytes.
pub const GA94: [u8; 4] = *b"GA94";

/// The ATSC T.35 country code (United States).
pub const ATSC_COUNTRY_CODE: u8 = 0xB5;

/// The ATSC T.35 provider code.
pub const ATSC_PROVIDER_CODE: u16 = 0x0031;

/// The user_data_type_code for closed captioning.
pub const CC_TYPE_CODE: u8 = 0x03;

/// Parse the flag/count byte shared by the byte-aligned forms.
fn flags_and_count(body: &[u8]) -> Result<(bool, u8), CcDataError> {
    let byte = *body.first().ok_or(CcDataError::Truncated {
        what: "cc_data flags",
        needed: 1,
        got: 0,
    })?;
    if byte & 0x20 != 0 {
        return Err(CcDataError::BadZeroBit);
    }
    // process_cc_data_flag == 0: the payload is not caption data —
    // an empty result, not an error (the A/53 allowance).
    Ok((byte & 0x40 != 0, byte & 0x1F))
}

/// Parse the `cc_data()` body (after the `0x03` type code): the
/// flag/count byte, the triplets, the `0xFF` marker.
fn parse_cc_data_body(body: &[u8]) -> Result<Vec<CcDataTriplet>, CcDataError> {
    let (process, count) = flags_and_count(body)?;
    if !process {
        return Ok(Vec::new());
    }
    let n = usize::from(count);
    let needed = 1 + n * 3 + 1;
    if body.len() < needed {
        return Err(CcDataError::Truncated {
            what: "cc_data triplets + marker",
            needed,
            got: body.len(),
        });
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = &body[1 + i * 3..1 + i * 3 + 3];
        out.push(CcDataTriplet {
            cc_valid: t[0] & 0x04 != 0, // [ 5 reserved ][1 valid][2 type]
            cc_type: t[0] & 0x03,
            data: [t[1], t[2]],
        });
    }
    if body[needed - 1] != 0xFF {
        return Err(CcDataError::BadMarker {
            got: body[needed - 1],
        });
    }
    Ok(out)
}

/// Parse the ATSC GA94 closed-caption payload of an H.264 SEI type-4
/// `user_data_registered_itu_t_t35` message (country + provider +
/// identifier + type + `cc_data()`).
///
/// # Errors
/// Typed [`CcDataError`]s — see the variants.
pub fn parse_ga94_sei(payload: &[u8]) -> Result<Vec<CcDataTriplet>, CcDataError> {
    if payload.len() < 4 + 4 {
        return Err(CcDataError::Truncated {
            what: "T.35 header (country/provider/GA94/type)",
            needed: 8,
            got: payload.len(),
        });
    }
    if payload[0] != ATSC_COUNTRY_CODE {
        return Err(CcDataError::BadCountryCode { code: payload[0] });
    }
    let provider = u16::from_be_bytes([payload[1], payload[2]]);
    if provider != ATSC_PROVIDER_CODE {
        return Err(CcDataError::BadProviderCode { code: provider });
    }
    let mut ident = [0u8; 4];
    ident.copy_from_slice(&payload[3..7]);
    if ident != GA94 {
        return Err(CcDataError::BadUserIdentifier { got: ident });
    }
    if payload[7] != CC_TYPE_CODE {
        return Err(CcDataError::BadTypeCode { code: payload[7] });
    }
    parse_cc_data_body(&payload[8..])
}

/// Parse the GA94 closed-caption payload of an MPEG-2 picture
/// `user_data` field — the bytes after the `0x000001B2` start code,
/// starting at the `GA94` identifier. The SEI form
/// ([`parse_ga94_sei`]) is the T.35 superset; this entry exists
/// because the MPEG-2 carriage has no T.35 country/provider header.
///
/// # Errors
/// Typed [`CcDataError`]s — see the variants.
pub fn parse_mpeg2_ga94(user_data: &[u8]) -> Result<Vec<CcDataTriplet>, CcDataError> {
    if user_data.len() < 4 + 1 {
        return Err(CcDataError::Truncated {
            what: "MPEG-2 user_data GA94 + type",
            needed: 5,
            got: user_data.len(),
        });
    }
    let mut ident = [0u8; 4];
    ident.copy_from_slice(&user_data[0..4]);
    if ident != GA94 {
        return Err(CcDataError::BadUserIdentifier { got: ident });
    }
    if user_data[4] != CC_TYPE_CODE {
        return Err(CcDataError::BadTypeCode { code: user_data[4] });
    }
    parse_cc_data_body(&user_data[5..])
}

/// A minimal MSB-first bit reader (SCTE-20 is a bit-field grammar,
/// not byte-aligned past the type code).
struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl BitReader<'_> {
    fn take(&mut self, n: u32) -> Option<u32> {
        let mut v = 0u32;
        for _ in 0..n {
            let byte = self.data.get(self.bit / 8)?;
            v = (v << 1) | u32::from(byte & (0x80 >> (self.bit % 8)) != 0);
            self.bit += 1;
        }
        Some(v)
    }
}

/// Parse an SCTE-20 `caption_user_data` payload: the bytes after the
/// `0x000001B2` start code, starting at the `0x03`
/// `user_data_type_code`. The header is
/// `[process_cc_data_flag 1][zero_bit 1][cc_count 5]` and each
/// triplet `[cc_valid 1][cc_type 2][cc_data_1 7 + reserved 1][cc_data_2 7 + reserved 1]`
/// — the bit order of the reference decoders' SCTE-20 walk (no
/// leading reserved bit, unlike the byte-aligned `cc_data()` form).
/// The stream closes with `1111` marker nibbles (tolerated when
/// absent).
///
/// # Errors
/// [`CcDataError::Truncated`] for a short bit stream,
/// [`CcDataError::BadTypeCode`] for a non-`0x03` type code,
/// [`CcDataError::BadZeroBit`] when the zero bit is set.
pub fn parse_scte20(payload: &[u8]) -> Result<Vec<CcDataTriplet>, CcDataError> {
    let first = *payload.first().ok_or(CcDataError::Truncated {
        what: "SCTE-20 user_data_type_code",
        needed: 1,
        got: 0,
    })?;
    if first != CC_TYPE_CODE {
        return Err(CcDataError::BadTypeCode { code: first });
    }
    let mut r = BitReader {
        data: &payload[1..],
        bit: 0,
    };
    // [process_cc_data_flag 1][zero_bit 1] — the SCTE-20 header as
    // ffmpeg's mpeg12 SCTE-20 walk reads it (skip 2, then cc_count).
    let flags = r.take(2).ok_or(CcDataError::Truncated {
        what: "SCTE-20 flags",
        needed: 2,
        got: 0,
    })?;
    if flags & 0x1 != 0 {
        return Err(CcDataError::BadZeroBit);
    }
    if flags & 0x2 == 0 {
        return Ok(Vec::new()); // process_cc_data_flag == 0
    }
    let count = r.take(5).ok_or(CcDataError::Truncated {
        what: "SCTE-20 cc_count",
        needed: 5,
        got: 0,
    })?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let head = r.take(3).ok_or(CcDataError::Truncated {
            what: "SCTE-20 triplet head",
            needed: 3,
            got: 0,
        })? as u8;
        let mut data = [0u8; 2];
        for slot in &mut data {
            let bits = r.take(7).ok_or(CcDataError::Truncated {
                what: "SCTE-20 data bits",
                needed: 7,
                got: 0,
            })? as u8;
            let _ = r.take(1); // reserved bit — value not defined
            // Re-synthesize odd parity in the MSB so the 608 layer's
            // uniform parity rule applies.
            *slot = eia608::with_odd_parity(bits);
        }
        out.push(CcDataTriplet {
            cc_valid: head & 0x04 != 0,
            cc_type: head & 0x03,
            data,
        });
    }
    // marker nibbles: 4 bits '1111' per caption byte pair — padding,
    // consumed when present, never required.
    let _ = r.take(count * 2 * 4);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The byte-aligned flags byte: reserved 0, process 1, zero 0,
    /// count in the low 5 bits.
    fn flags(count: u8) -> u8 {
        0x40 | (count & 0x1F)
    }

    fn triplet_bytes(field: u8, d1: u8, d2: u8) -> [u8; 3] {
        [0x04 | (field & 0x03), d1, d2]
    }

    fn ga94_body(triplets: &[[u8; 3]]) -> Vec<u8> {
        let mut v = vec![ATSC_COUNTRY_CODE, 0x00, 0x31];
        v.extend_from_slice(&GA94);
        v.push(CC_TYPE_CODE);
        v.push(flags(triplets.len() as u8));
        for t in triplets {
            v.extend_from_slice(t);
        }
        v.push(0xFF);
        v
    }

    /// An MSB-first bit writer (test vocabulary for SCTE-20 vectors).
    struct BitWriter {
        out: Vec<u8>,
        bit: u32,
    }

    impl BitWriter {
        fn new() -> BitWriter {
            BitWriter {
                out: Vec::new(),
                bit: 0,
            }
        }
        fn put(&mut self, n: u32, v: u32) {
            for i in (0..n).rev() {
                if self.bit.is_multiple_of(8) {
                    self.out.push(0);
                }
                if v & (1 << i) != 0 {
                    let idx = self.out.len() - 1;
                    self.out[idx] |= 0x80 >> (self.bit % 8);
                }
                self.bit += 1;
            }
        }
    }

    fn scte20_body(triplets: &[(bool, u8, u8, u8)]) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.put(1, 1); // process_cc_data_flag
        w.put(1, 0); // zero_bit
        w.put(5, triplets.len() as u32);
        for &(valid, ty, d1, d2) in triplets {
            w.put(1, u32::from(valid));
            w.put(2, u32::from(ty));
            w.put(7, u32::from(d1));
            w.put(1, 0); // reserved
            w.put(7, u32::from(d2));
            w.put(1, 0); // reserved
        }
        let mut v = vec![CC_TYPE_CODE];
        v.extend_from_slice(&w.out);
        v.extend_from_slice(&[0xFF]); // marker nibbles (approximate)
        v
    }

    #[test]
    fn ga94_roundtrip() {
        let payload = ga94_body(&[triplet_bytes(0, 0x94, 0x14), triplet_bytes(2, 0x41, 0x00)]);
        let out = parse_ga94_sei(&payload).must();
        assert_eq!(out.len(), 2);
        assert_eq!(
            out[0],
            CcDataTriplet {
                cc_valid: true,
                cc_type: 0,
                data: [0x94, 0x14]
            }
        );
        assert_eq!(out[1].cc_type, 2);
    }

    #[test]
    fn ga94_rejects_wrong_country_provider_identifier_type() {
        let mut p = ga94_body(&[]);
        p[0] = 0x01;
        assert!(matches!(
            parse_ga94_sei(&p),
            Err(CcDataError::BadCountryCode { code: 0x01 })
        ));
        let mut p = ga94_body(&[]);
        p[1] = 0xFF;
        assert!(matches!(
            parse_ga94_sei(&p),
            Err(CcDataError::BadProviderCode { .. })
        ));
        let mut p = ga94_body(&[]);
        p[3] = b'X';
        assert!(matches!(
            parse_ga94_sei(&p),
            Err(CcDataError::BadUserIdentifier { .. })
        ));
        let mut p = ga94_body(&[]);
        p[7] = 0x09;
        assert!(matches!(
            parse_ga94_sei(&p),
            Err(CcDataError::BadTypeCode { code: 0x09 })
        ));
    }

    #[test]
    fn ga94_rejects_truncation_marker_and_zero_bit() {
        let full = ga94_body(&[triplet_bytes(0, 0, 0)]);
        assert!(matches!(
            parse_ga94_sei(&full[..full.len() - 1]),
            Err(CcDataError::Truncated { .. })
        ));
        let mut p = ga94_body(&[triplet_bytes(0, 0, 0)]);
        let n = p.len();
        p[n - 1] = 0x7E;
        assert!(matches!(
            parse_ga94_sei(&p),
            Err(CcDataError::BadMarker { got: 0x7E })
        ));
        let mut p = ga94_body(&[triplet_bytes(0, 0, 0)]);
        p[8] |= 0x20; // zero_bit set
        assert!(matches!(parse_ga94_sei(&p), Err(CcDataError::BadZeroBit)));
    }

    #[test]
    fn process_flag_zero_is_empty_not_error() {
        let mut p = ga94_body(&[triplet_bytes(0, 0, 0)]);
        p[8] = 0x01; // process=0, count=1 (payload still sized)
        assert!(parse_ga94_sei(&p).must().is_empty());
    }

    #[test]
    fn trailing_bytes_after_marker_are_tolerated() {
        let mut p = ga94_body(&[triplet_bytes(0, 0, 0)]);
        p.extend_from_slice(&[0xDE, 0xAD]);
        assert_eq!(parse_ga94_sei(&p).must().len(), 1);
    }

    #[test]
    fn mpeg2_ga94_entry_matches_sei_body() {
        let mut p = Vec::new();
        p.extend_from_slice(&GA94);
        p.push(CC_TYPE_CODE);
        p.push(flags(1));
        p.extend_from_slice(&triplet_bytes(0, 0x94, 0x2F));
        p.push(0xFF);
        let out = parse_mpeg2_ga94(&p).must();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].data, [0x94, 0x2F]);
    }

    #[test]
    fn scte20_roundtrip_with_resynthesized_parity() {
        let v = scte20_body(&[
            (true, 0, 0x14, 0x25),
            (true, 1, 0x31, 0x32),
            (false, 0, 0, 0),
        ]);
        let out = parse_scte20(&v).must();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].cc_type, 0);
        assert!(out[0].cc_valid);
        assert_eq!(out[0].data[0] & 0x7F, 0x14);
        assert_eq!(out[0].data[1] & 0x7F, 0x25);
        assert_eq!(out[1].cc_type, 1);
        assert_eq!(out[1].data[0] & 0x7F, 0x31);
        assert!(!out[2].cc_valid);
        // every byte re-synthesized with odd parity (the pinned
        // helper — parity_ok — is the same predicate)
        for t in &out {
            for b in t.data {
                assert!(eia608::parity_ok(b), "byte {b:#04x}");
            }
        }
    }

    #[test]
    fn scte20_truncated_is_typed() {
        assert!(matches!(
            parse_scte20(&[]),
            Err(CcDataError::Truncated { .. })
        ));
        assert!(matches!(
            parse_scte20(&[0x04]),
            Err(CcDataError::BadTypeCode { code: 0x04 })
        ));
        // flags byte present but the count bits are gone (process=1,
        // zero=0, then the stream ends mid-count).
        assert!(matches!(
            parse_scte20(&[0x03, 0xA0]),
            Err(CcDataError::Truncated { .. })
        ));
    }
}
