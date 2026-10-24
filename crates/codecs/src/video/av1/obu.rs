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
    /// The TU contains OBUs that may produce a picture
    /// ([`OBU_FRAME`]; a frame header — plain or redundant —
    /// including `show_existing_frame` overlay TUs that carry ONLY a
    /// header; or a tile group). The decoder keys its
    /// presentation-time association on this; candidates that never
    /// emit age out at the decoder's pending-map cap.
    pub may_produce_picture: bool,
    /// The TU carries a self-contained frame — an [`OBU_FRAME`] /
    /// [`OBU_FRAME_HEADER`] / [`OBU_REDUNDANT_FRAME_HEADER`] whose
    /// first payload byte peeks as `!show_existing_frame` with
    /// `frame_type ∈ {KEY, INTRA_ONLY}` (spec §6.8: 1 + 2 bits). The
    /// decoder's reference-state gate feeds the engine one of these
    /// before any dependent frame — the containment for a confirmed
    /// upstream abort (rav1d 1.1.0 `decode.rs` `on_error` unwraps a
    /// `None` frame header when a dependent frame meets broken
    /// reference state; a mid-stream drop aborts the process). A
    /// 3-bit peek is the minimum semantics that can never *create*
    /// a wrong decode — it only gates feeding, and misclassification
    /// degrades to a typed refusal, never a mis-decode.
    /// `[VERIFY-P1: reduced-still-picture headers shift the bit
    /// layout — such streams refuse until the upstream fix; record
    /// re-trigger on the next rav1d bump.]`
    pub has_independent_frame: bool,
    /// Total payload bytes covered by the framed OBUs (≤ input len).
    pub payload_bytes: usize,
}

/// The typed structural-damage constructor (this module's only error
/// class — `ErrorKind::Corrupt`, the per-frame kind per doc 03
/// §4.5(d)).
fn obu_error(code: &'static str, detail: CorruptDetail) -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Corrupt { detail },
        ErrorContext::new(code),
    )
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
        CorruptDetail::InvalidStructure {
            offset: Some(cursor as u64),
        },
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
            CorruptDetail::Truncated {
                expected: 1,
                found: 0,
            },
        ));
    }
    let mut obus = Vec::new();
    let mut has_sequence_header = false;
    let mut has_frame_header = false;
    let mut has_tile_group = false;
    let mut may_produce_picture = false;
    let mut has_independent_frame = false;
    let mut payload_bytes = 0usize;
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if obus.len() == MAX_OBUS_PER_TU {
            return Err(obu_error(
                "av1.obu.count_limit",
                CorruptDetail::InvalidStructure {
                    offset: Some(cursor as u64),
                },
            ));
        }
        let header_offset = cursor;
        let header = bytes[cursor];
        if header & 0x80 != 0 {
            return Err(obu_error(
                "av1.obu.forbidden_bit",
                CorruptDetail::InvalidStructure {
                    offset: Some(header_offset as u64),
                },
            ));
        }
        if header & 0x01 != 0 {
            return Err(obu_error(
                "av1.obu.reserved_bit",
                CorruptDetail::InvalidStructure {
                    offset: Some(header_offset as u64),
                },
            ));
        }
        let obu_type = (header >> 3) & 0x0f;
        // Spec §5.3.1 bit order after the 4-bit type:
        // extension_flag (bit 2) THEN has_size_field (bit 1); the
        // reserved bit (bit 0) is checked above. Swapping these two
        // misreads every real stream (a size field becomes a phantom
        // extension byte) — pinned by the fixture-backed differential
        // suite, not just the unit table below.
        let extension = header & 0x04 != 0;
        let has_size = header & 0x02 != 0;
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
                    CorruptDetail::InvalidStructure {
                        offset: Some(cursor as u64),
                    },
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
                    CorruptDetail::InvalidStructure {
                        offset: Some(cursor as u64),
                    },
                ));
            }
            cursor += leb_len;
            let start = cursor;
            let end = start.checked_add(size as usize).ok_or_else(|| {
                obu_error(
                    "av1.obu.size_overflow",
                    CorruptDetail::InvalidStructure {
                        offset: Some(cursor as u64),
                    },
                )
            })?;
            if end > bytes.len() {
                return Err(obu_error(
                    "av1.obu.payload_truncated",
                    CorruptDetail::Truncated {
                        expected: end as u64,
                        found: bytes.len() as u64,
                    },
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
            // FRAME, FRAME_HEADER (a `show_existing_frame` overlay
            // TU carries ONLY a frame header — libaom's altref
            // pattern, pinned by the differential fixtures) and
            // TILE_GROUP all MAY produce a picture; the structural
            // census cannot tell an overlay from a header-only
            // fragment, so all three gate the pts-association map
            // and non-emitting candidates age out at the cap.
            OBU_FRAME => may_produce_picture = true,
            OBU_FRAME_HEADER | OBU_REDUNDANT_FRAME_HEADER => {
                has_frame_header = true;
                may_produce_picture = true;
            }
            OBU_TILE_GROUP => {
                has_tile_group = true;
                may_produce_picture = true;
            }
            _ => {}
        }
        if matches!(
            obu_type,
            OBU_FRAME | OBU_FRAME_HEADER | OBU_REDUNDANT_FRAME_HEADER
        ) && !payload.is_empty()
        {
            // The 3-bit frame-header peek (field docs on
            // [`TemporalUnit::has_independent_frame`]).
            let b0 = bytes[payload.start];
            let show_existing = b0 & 0x80 != 0;
            let frame_type = (b0 >> 5) & 0b11;
            if !show_existing && matches!(frame_type, 0 | 2) {
                has_independent_frame = true;
            }
        }
        payload_bytes += payload.len();
        obus.push(ObuInfo {
            obu_type,
            temporal_id,
            spatial_id,
            payload,
        });
    }

    if !may_produce_picture && has_frame_header && has_tile_group {
        may_produce_picture = true;
    }
    Ok(TemporalUnit {
        obus,
        has_sequence_header,
        may_produce_picture,
        has_independent_frame,
        payload_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The `unwrap_err` counterpart to [`testkit::Must`] for the
    /// negative fixtures (the workspace denies raw `unwrap*`).
    trait MustErr<E> {
        fn must_err(self) -> E;
    }

    impl<T, E> MustErr<E> for Result<T, E> {
        fn must_err(self) -> E {
            match self {
                Ok(_) => unreachable!("must_err(): expected an error, got Ok"),
                Err(err) => err,
            }
        }
    }

    /// Frame helper: `(type, has_size, ext, payload)` → OBU bytes
    /// (spec bit order: extension bit 2, size bit 1 — mirrors the
    /// parser).
    fn obu(obu_type: u8, has_size: bool, ext: Option<(u8, u8)>, payload: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        let mut header = obu_type << 3;
        if ext.is_some() {
            header |= 0x04;
        }
        if has_size {
            header |= 0x02;
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
        let err = parse_temporal_unit(&[]).must_err();
        assert_eq!(err.context.code, "av1.obu.empty_tu");
        assert_eq!(
            err.kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: 1,
                    found: 0
                }
            }
        );
    }

    #[test]
    fn forbidden_and_reserved_bits_are_typed() {
        let err = parse_temporal_unit(&[0x80]).must_err();
        assert_eq!(err.context.code, "av1.obu.forbidden_bit");
        let err = parse_temporal_unit(&[0x01]).must_err();
        assert_eq!(err.context.code, "av1.obu.reserved_bit");
    }

    #[test]
    fn sizeless_obu_takes_rest_of_unit() {
        // TD OBU (type 2) with has_size=0, no payload.
        // Payload ranges are absolute TU byte offsets (header byte
        // included) — the same convention the PADDING half below and
        // the leb128/extension offsets use.
        let tu = parse_temporal_unit(&[0x10]).must();
        assert_eq!(tu.obus.len(), 1);
        assert_eq!(tu.obus[0].obu_type, OBU_TEMPORAL_DELIMITER);
        assert_eq!(tu.obus[0].payload, 1..1);
        assert!(!tu.may_produce_picture);
        // PADDING (15) with has_size=0 swallows trailing bytes:
        // header = 15<<3 = 0x78 (the MSB must stay clear — 0xf0
        // would set the forbidden bit).
        let tu = parse_temporal_unit(&[0x78, 1, 2, 3]).must();
        assert_eq!(tu.obus.len(), 1);
        assert_eq!(tu.obus[0].payload, 1..4);
        assert_eq!(tu.payload_bytes, 3);
    }

    #[test]
    fn sized_obus_chain_with_exact_consumption() {
        let mut bytes = obu(OBU_TEMPORAL_DELIMITER, true, None, &[]);
        bytes.extend(obu(OBU_SEQUENCE_HEADER, true, None, &[0xaa; 4]));
        bytes.extend(obu(OBU_FRAME, true, Some((1, 2)), &[0x55; 9]));
        let tu = parse_temporal_unit(&bytes).must();
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
        let tu = parse_temporal_unit(&bytes).must();
        assert!(tu.may_produce_picture);
        assert!(!tu.has_sequence_header);
    }

    #[test]
    fn show_existing_overlay_tu_counts_as_picture() {
        // The libaom altref pattern the differential fixtures pin:
        // TD + FRAME_HEADER only (a `show_existing_frame` overlay —
        // a 5-byte packet that still emits a picture, so it must
        // gate the pts-association map).
        let mut bytes = obu(OBU_TEMPORAL_DELIMITER, true, None, &[]);
        bytes.extend(obu(OBU_FRAME_HEADER, true, None, &[0x10]));
        let tu = parse_temporal_unit(&bytes).must();
        assert_eq!(tu.obus.len(), 2);
        assert!(tu.may_produce_picture);
        assert!(!tu.has_sequence_header);
    }

    #[test]
    fn independent_frame_peek_matches_the_frame_header_bits() {
        // KEY (frame_type 0): show_existing=0, type=00 → 0b000_xxxx.
        let bytes = [
            obu(OBU_TEMPORAL_DELIMITER, true, None, &[]),
            obu(OBU_FRAME, true, None, &[0x00, 0xaa]),
        ]
        .concat();
        let tu = parse_temporal_unit(&bytes).must();
        assert!(tu.has_independent_frame, "KEY peeks independent");
        // INTRA_ONLY (2) is also self-contained.
        let bytes = [obu(OBU_FRAME, true, None, &[0x40, 0xaa])].concat();
        let tu = parse_temporal_unit(&bytes).must();
        assert!(tu.has_independent_frame, "INTRA_ONLY peeks independent");
        // INTER (1) and SWITCH (3) depend on reference state.
        for ft in [1u8, 3] {
            let bytes = [obu(OBU_FRAME, true, None, &[ft << 5, 0xaa])].concat();
            let tu = parse_temporal_unit(&bytes).must();
            assert!(!tu.has_independent_frame, "frame_type {ft} is dependent");
        }
        // show_existing_frame overlays are dependent by definition.
        let bytes = [obu(OBU_FRAME_HEADER, true, None, &[0x80, 0x00])].concat();
        let tu = parse_temporal_unit(&bytes).must();
        assert!(!tu.has_independent_frame, "show_existing is dependent");
        // Non-frame OBUs never peek.
        let bytes = [obu(OBU_METADATA, true, None, &[0x00])].concat();
        let tu = parse_temporal_unit(&bytes).must();
        assert!(!tu.has_independent_frame);
    }

    #[test]
    fn truncated_payload_is_typed_with_expected_end() {
        let bytes = obu(OBU_FRAME, true, None, &[0x00; 32]); // claims 32
        let cut = &bytes[..bytes.len() - 5];
        let err = parse_temporal_unit(cut).must_err();
        assert_eq!(err.context.code, "av1.obu.payload_truncated");
        // The unit needs header+leb+32 payload = 34 bytes; 5 were cut.
        assert_eq!(
            err.kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: 34,
                    found: 29
                }
            }
        );
    }

    #[test]
    fn leb128_boundaries() {
        // Multi-byte leb128 (300 = 0xAC 0x02); 0x02 = has_size bit.
        let mut bytes = vec![OBU_METADATA << 3 | 0x02, 0xac, 0x02];
        bytes.extend_from_slice(&[0u8; 300]);
        let tu = parse_temporal_unit(&bytes).must();
        assert_eq!(tu.obus[0].payload.len(), 300);
        // 8 continuation bytes → too long.
        let mut nine = vec![OBU_METADATA << 3 | 0x02];
        nine.extend(std::iter::repeat_n(0x80u8, 9));
        let err = parse_temporal_unit(&nine).must_err();
        assert_eq!(err.context.code, "av1.obu.leb128_too_long");
        // Truncated mid-leb128.
        let err = parse_temporal_unit(&[OBU_METADATA << 3 | 0x02, 0x80]).must_err();
        assert_eq!(err.context.code, "av1.obu.leb128_truncated");
        // Value above u32::MAX (0x1f << 28 = 0xF_0000_0000).
        let err = parse_temporal_unit(&[OBU_METADATA << 3 | 0x02, 0xff, 0xff, 0xff, 0xff, 0x1f])
            .must_err();
        assert_eq!(err.context.code, "av1.obu.size_overflow");
    }

    #[test]
    fn obu_count_guard_fires_typed() {
        // MAX+1 zero-payload sized OBUs (0x02 = has_size bit).
        let mut bytes = Vec::new();
        for _ in 0..=MAX_OBUS_PER_TU {
            bytes.extend_from_slice(&[OBU_PADDING << 3 | 0x02, 0x00]);
        }
        let err = parse_temporal_unit(&bytes).must_err();
        assert_eq!(err.context.code, "av1.obu.count_limit");
    }

    #[test]
    fn extension_byte_rules() {
        // Extension byte with reserved low bits set → typed
        // (0x14 = TD | extension bit 0x04).
        let err = parse_temporal_unit(&[0x14, 0x01]).must_err();
        assert_eq!(err.context.code, "av1.obu.extension_reserved");
        // Extension header with no extension byte → truncated.
        let err = parse_temporal_unit(&[0x14]).must_err();
        assert_eq!(err.context.code, "av1.obu.extension_truncated");
    }
}
