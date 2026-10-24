//! Minimal AV1 OBU (Open Bitstream Unit) framing — card 5.E1's
//! "sequence/frame header OBU parse MINIMAL (enough to feed rav1d:
//! temporal unit framing)".
//!
//! # Honest scope
//!
//! This is a **framing validator, not a bitstream parser**: it walks
//! OBU headers, decodes the `leb128` size fields, and classifies the
//! OBU types of one temporal unit (TU) — the structure rav1d needs
//! delivered intact (one IVF packet = one TU, the demuxer's
//! documented contract). It does **not** decode sequence-header or
//! frame-header bits: that job belongs to the engine (rav1d's
//! `dav1d_parse_sequence_header` / internal OBU parser), and
//! duplicating it here would drift from the reference the moment a
//! profile lands. What this module adds over handing rav1d raw bytes
//! is (a) typed, byte-precise damage classification *before* the
//! engine sees the packet (the `av1.obu.*` error codes), (b) the TU
//! census ([`TemporalUnit::may_produce_picture`],
//! [`TemporalUnit::has_sequence_header`]) the decoder's
//! presentation-time association needs, and (c) a bounded-OBU-count
//! resource guard for hostile inputs.
//!
//! # Grammar (AV1 spec §5.3, framing subset)
//!
//! ```text
//! obu_header(): 1 byte — forbidden(1)=0, type(4), ext(1), has_size(1), reserved(1)=0
//! [ext] 1 byte — temporal_id(3), spatial_id(2), reserved(3)=0
//! [has_size] leb128 obu_size  (≤ 8 bytes, value ≤ u32::MAX)
//! payload: obu_size bytes (or, with has_size=0, to the end of the TU)
//! ```
//!
//! Every entry point is total: any byte string yields a parsed TU or
//! a typed [`EngineError`](media_types::EngineError) — no panics, no
//! unbounded work (the OBU-count cap below), the `fuzz_codec_av1`
//! surface (doc 04 §4.1).

use media_types::error::{CorruptDetail, ErrorContext};
use media_types::{EngineError, ErrorKind, PipelineStage};
use std::ops::Range;

/// OBU type: sequence header (`1`, spec §6.2).
pub const OBU_SEQUENCE_HEADER: u8 = 1;
/// OBU type: temporal delimiter (`2`, spec §6.3).
pub const OBU_TEMPORAL_DELIMITER: u8 = 2;
/// OBU type: frame header (`3`, spec §6.4).
pub const OBU_FRAME_HEADER: u8 = 3;
/// OBU type: tile group (`4`, spec §6.5).
pub const OBU_TILE_GROUP: u8 = 4;
/// OBU type: metadata (`5`, spec §6.7).
pub const OBU_METADATA: u8 = 5;
/// OBU type: frame (combined header+tiles, `6`, spec §6.8).
pub const OBU_FRAME: u8 = 6;
/// OBU type: redundant frame header (`7`, spec §6.6).
pub const OBU_REDUNDANT_FRAME_HEADER: u8 = 7;
/// OBU type: tile list (`8`, spec §6.9 — the scalable/encoding path).
pub const OBU_TILE_LIST: u8 = 8;
/// OBU type: padding (`15`, spec §6.10).
pub const OBU_PADDING: u8 = 15;

/// Resource guard: a TU declaring more OBUs than this is typed
/// damage, not parsed (a real 4K AV1 TU carries hundreds at most —
/// tile rows × columns; the cap exists so a hostile byte string
/// cannot make the census allocate unboundedly). Fuzz-lite inputs
/// hit this long before memory pressure.
pub const MAX_OBUS_PER_TU: usize = 4096;

/// One framed OBU (header facts + payload range within the TU).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObuInfo {
    /// The 4-bit `obu_type` (constants above; reserved values pass
    /// through — skipping unknown OBUs is spec behavior, §6 general).
    pub obu_type: u8,
    /// 3-bit temporal id (0 when the extension flag is clear).
    pub temporal_id: u8,
    /// 2-bit spatial id (0 when the extension flag is clear).
    pub spatial_id: u8,
    /// Byte range of the OBU payload (after header, extension and
    /// size field) within the TU buffer.
    pub payload: Range<usize>,
}

/// One validated temporal unit: the OBU census the decoder needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalUnit {
    /// Every framed OBU, in stream order.
    pub obus: Vec<ObuInfo>,
    /// A sequence-header OBU is present (the stream-definition OBU —
    /// rav1d also reports it as a new-sequence event; kept here so
    /// callers can react without peeking into `obus`).
    pub has_sequence_header: bool,
    /// The TU contains frame data that can produce a picture
    /// ([`OBU_FRAME`], or a frame header — plain or redundant —
    /// paired with a tile group). The decoder keys its
    /// presentation-time association on this.
    pub may_produce_picture: bool,
    /// Total payload bytes covered by the framed OBUs (≤ input len).
    pub payload_bytes: usize,
}

/// The typed structural-damage constructor (this module's only error
/// class — `ErrorKind::Corrupt`, the per-frame kind per doc 03
/// §4.5(d)).
fn obu_error(code: &'static str, detail: CorruptDetail) -> EngineError {
    EngineError::new(PipelineStage::Decode, ErrorKind::Corrupt { detail }, ErrorContext::new(code))
}

/// Read one `leb128` value starting at `cursor` in `bytes`.
///
/// Returns the value and the bytes consumed. AV1 caps `obu_size` at
/// `u32::MAX` (spec §5.3.4: at most 8 bytes, value < 2³²); a longer
/// or overlarge encoding is typed damage, never a wrap.
fn read_leb128(bytes: &[u8], cursor: usize) -> Result<(u64, usize), EngineError> {
    let mut value: u64 = 0;
    for i in 0..8 {
        let Some(&b) = bytes.get(cursor + i) else {
            return Err(obu_error(
                "av1.obu.leb128_truncated",
                CorruptDetail::Truncated {
                    expected: (i + 1) as u64,
                    found: (bytes.len() - cursor) as u64,
                },
            ));
        };
        value |= u64::from(b & 0x7f) << (i * 7);
        if b & 0x80 == 0 {
            return Ok((value, i + 1));
        }
    }
    // Ninth byte would be needed: outside the 8-byte spec window.
    Err(obu_error(
        "av1.obu.leb128_too_long",
        CorruptDetail::InvalidStructure { offset: Some(cursor as u64) },
    ))
}

/// Frame one temporal unit (`bytes` = one IVF packet payload, the
/// demuxer's one-TU-per-packet contract).
///
/// # Errors
/// Typed [`ErrorKind::Corrupt`] damage with `av1.obu.*` context
/// codes: empty TU, forbidden/reserved header bits, a `leb128` size
/// outside the spec window, or a declared payload past the end of
/// the unit (truncation). Damage is per-packet; the next TU parses
/// independently.
pub fn parse_temporal_unit(bytes: &[u8]) -> Result<TemporalUnit, EngineError> {
    if bytes.is_empty() {
        return Err(obu_error(
            "av1.obu.empty_tu",
            CorruptDetail::Truncated { expected: 1, found: 0 },
        ));
    }
    let mut obus = Vec::new();
    let mut has_sequence_header = false;
    let mut has_frame_header = false;
    let mut has_tile_group = false;
    let mut may_produce_picture = false;
    let mut payload_bytes = 0usize;
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if obus.len() == MAX_OBUS_PER_TU {
            return Err(obu_error(
                "av1.obu.count_limit",
                CorruptDetail::InvalidStructure { offset: Some(cursor as u64) },
            ));
        }
        let header_offset = cursor;
        let header = bytes[cursor];
        if header & 0x80 != 0 {
            return Err(obu_error(
                "av1.obu.forbidden_bit",
                CorruptDetail::InvalidStructure { offset: Some(header_offset as u64) },
            ));
        }
        if header & 0x01 != 0 {
            return Err(obu_error(
                "av1.obu.reserved_bit",
                CorruptDetail::InvalidStructure { offset: Some(header_offset as u64) },
            ));
        }
        let obu_type = (header >> 3) & 0x0f;
        let extension = header & 0x02 != 0;
        let has_size = header & 0x04 != 0;
        cursor += 1;

        let mut temporal_id = 0u8;
        let mut spatial_id = 0u8;
        if extension {
            let Some(&ext) = bytes.get(cursor) else {
                return Err(obu_error(
                    "av1.obu.extension_truncated",
                    CorruptDetail::Truncated {
                        expected: (cursor + 1) as u64,
                        found: bytes.len() as u64,
                    },
                ));
            };
            if ext & 0x07 != 0 {
                return Err(obu_error(
                    "av1.obu.extension_reserved",
                    CorruptDetail::InvalidStructure { offset: Some(cursor as u64) },
                ));
            }
            temporal_id = ext >> 5;
            spatial_id = (ext >> 3) & 0x03;
            cursor += 1;
        }

        let payload = if has_size {
            let (size, leb_len) = read_leb128(bytes, cursor)?;
            if size > u64::from(u32::MAX) {
                return Err(obu_error(
                    "av1.obu.size_overflow",
                    CorruptDetail::InvalidStructure { offset: Some(cursor as u64) },
                ));
            }
            cursor += leb_len;
            let start = cursor;
            let end = start.checked_add(size as usize).ok_or_else(|| {
                obu_error(
                    "av1.obu.size_overflow",
                    CorruptDetail::InvalidStructure { offset: Some(cursor as u64) },
                )
            })?;
            if end > bytes.len() {
                return Err(obu_error(
                    "av1.obu.payload_truncated",
                    CorruptDetail::Truncated { expected: end as u64, found: bytes.len() as u64 },
                ));
            }
            cursor = end;
            start..end
        } else {
            // has_size=0: the payload runs to the end of the unit
            // (the low-overhead bitstream form, spec §5.2).
            let start = cursor;
            cursor = bytes.len();
            start..cursor
        };

        match obu_type {
            OBU_SEQUENCE_HEADER => has_sequence_header = true,
            OBU_FRAME => may_produce_picture = true,
            OBU_FRAME_HEADER | OBU_REDUNDANT_FRAME_HEADER => has_frame_header = true,
            OBU_TILE_GROUP => has_tile_group = true,
            _ => {}
        }
        payload_bytes += payload.len();
        obus.push(ObuInfo { obu_type, temporal_id, spatial_id, payload });
    }

    if !may_produce_picture && has_frame_header && has_tile_group {
        may_produce_picture = true;
    }

    Ok(TemporalUnit {
        obus,
        has_sequence_header,
        may_produce_picture,
        payload_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frame helper: `(type, has_size, ext, payload)` → OBU bytes.
    fn obu(obu_type: u8, has_size: bool, ext: Option<(u8, u8)>, payload: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        let mut header = obu_type << 3;
        if ext.is_some() {
            header |= 0x02;
        }
        if has_size {
            header |= 0x04;
        }
        b.push(header);
        if let Some((t, s)) = ext {
            b.push((t << 5) | (s << 3));
        }
        if has_size {
            b.push(payload.len() as u8);
        }
        b.extend_from_slice(payload);
        b
    }

    #[test]
    fn empty_tu_is_typed_truncation() {
        let err = parse_temporal_unit(&[]).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.empty_tu");
        assert_eq!(
            err.kind,
            ErrorKind::Corrupt { detail: CorruptDetail::Truncated { expected: 1, found: 0 } }
        );
    }

    #[test]
    fn forbidden_and_reserved_bits_are_typed() {
        let err = parse_temporal_unit(&[0x80]).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.forbidden_bit");
        let err = parse_temporal_unit(&[0x01]).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.reserved_bit");
    }

    #[test]
    fn sizeless_obu_takes_rest_of_unit() {
        // TD OBU (type 2) with has_size=0, no payload.
        let tu = parse_temporal_unit(&[0x10]).unwrap();
        assert_eq!(tu.obus.len(), 1);
        assert_eq!(tu.obus[0].obu_type, OBU_TEMPORAL_DELIMITER);
        assert_eq!(tu.obus[0].payload, 0..0);
        assert!(!tu.may_produce_picture);
        // PADDING (15) with has_size=0 swallows trailing bytes.
        let tu = parse_temporal_unit(&[0xf0, 1, 2, 3]).unwrap();
        assert_eq!(tu.obus.len(), 1);
        assert_eq!(tu.obus[0].payload, 1..4);
        assert_eq!(tu.payload_bytes, 3);
    }

    #[test]
    fn sized_obus_chain_with_exact_consumption() {
        let mut bytes = obu(OBU_TEMPORAL_DELIMITER, true, None, &[]);
        bytes.extend(obu(OBU_SEQUENCE_HEADER, true, None, &[0xaa; 4]));
        bytes.extend(obu(OBU_FRAME, true, Some((1, 2)), &[0x55; 9]));
        let tu = parse_temporal_unit(&bytes).unwrap();
        assert_eq!(tu.obus.len(), 3);
        assert!(tu.has_sequence_header);
        assert!(tu.may_produce_picture);
        assert_eq!(tu.obus[2].temporal_id, 1);
        assert_eq!(tu.obus[2].spatial_id, 2);
        assert_eq!(tu.payload_bytes, 13);
    }

    #[test]
    fn frame_header_plus_tile_group_counts_as_picture() {
        let mut bytes = obu(OBU_FRAME_HEADER, true, None, &[0x01]);
        bytes.extend(obu(OBU_TILE_GROUP, true, None, &[0x02]));
        let tu = parse_temporal_unit(&bytes).unwrap();
        assert!(tu.may_produce_picture);
        assert!(!tu.has_sequence_header);
    }

    #[test]
    fn truncated_payload_is_typed_with_expected_end() {
        let bytes = obu(OBU_FRAME, true, None, &[0x00; 32]); // claims 32
        let cut = &bytes[..bytes.len() - 5];
        let err = parse_temporal_unit(cut).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.payload_truncated");
        assert_eq!(
            err.kind,
            ErrorKind::Corrupt { detail: CorruptDetail::Truncated { expected: 35, found: 30 } }
        );
    }

    #[test]
    fn leb128_boundaries() {
        // Multi-byte leb128 (300 = 0xAC 0x02).
        let mut bytes = vec![OBU_METADATA << 3 | 0x04, 0xac, 0x02];
        bytes.extend_from_slice(&[0u8; 300]);
        let tu = parse_temporal_unit(&bytes).unwrap();
        assert_eq!(tu.obus[0].payload.len(), 300);
        // 8 continuation bytes → too long.
        let mut nine = vec![OBU_METADATA << 3 | 0x04];
        nine.extend(std::iter::repeat_n(0x80u8, 9));
        let err = parse_temporal_unit(&nine).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.leb128_too_long");
        // Truncated mid-leb128.
        let err = parse_temporal_unit(&[OBU_METADATA << 3 | 0x04, 0x80]).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.leb128_truncated");
        // Value above u32::MAX (0x1f << 28 = 0xF_0000_0000).
        let err = parse_temporal_unit(&[
            OBU_METADATA << 3 | 0x04,
            0xff, 0xff, 0xff, 0xff, 0x1f,
        ])
        .unwrap_err();
        assert_eq!(err.context.code, "av1.obu.size_overflow");
    }

    #[test]
    fn obu_count_guard_fires_typed() {
        // MAX+1 zero-payload sized OBUs.
        let mut bytes = Vec::new();
        for _ in 0..=MAX_OBUS_PER_TU {
            bytes.extend_from_slice(&[OBU_PADDING << 3 | 0x04, 0x00]);
        }
        let err = parse_temporal_unit(&bytes).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.count_limit");
    }

    #[test]
    fn extension_byte_rules() {
        // Extension byte with reserved low bits set → typed.
        let err = parse_temporal_unit(&[0x12, 0x01]).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.extension_reserved");
        // Extension header with no extension byte → truncated.
        let err = parse_temporal_unit(&[0x12]).unwrap_err();
        assert_eq!(err.context.code, "av1.obu.extension_truncated");
    }
}
