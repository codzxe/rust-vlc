//! H.264 SEI parser (card 5.F1b; ITU-T H.264 (03/2023) §7.3.2.3
//! `sei()` syntax and the Annex D payload subset this card owns).
//!
//! Clean-room, parse-level: each SEI NAL RBSP is a sequence of
//! `(payloadType, payloadSize, payload)` messages (both header
//! numbers accumulated over `0xFF` continuation bytes), closed by
//! `rbsp_trailing_bits()`. The card scope's payloads:
//!
//! - **buffering_period (0)** — minimal: `seq_parameter_set_id`
//!   only. The CPB delay fields need the SPS VUI HRD parameters
//!   (`cpb_cnt_minus1`, delay bit widths) — card 5.F1a's SPS/VUI
//!   parse supplies them at Lead merge (the unification point
//!   recorded in [`SeiMessage::BufferingPeriod`]).
//! - **pic_timing (1)** — presence + payload length recorded;
//!   field decode needs the same VUI HRD/pic-struct context
//!   (`pic_struct_present_flag`), same unification point.
//! - **user_data_unregistered (5)** — the full 16-byte
//!   ISO/IEC 11578 UUID + data bytes, with the x264 build-string
//!   case (the corpus's encoder) and the ffmpeg `Lavc`-prefix
//!   variant classified by [`UserDataUnregistered`]'s helpers.
//! - **recovery_point (6)** — all four syntax fields.
//! - **filler_payload (12)** — size + the all-`0xFF` check.
//!
//! Any other payload type is captured as
//! [`SeiMessage::Unparsed`] (type + size) and skipped intact —
//! parsed presence, no silent drop.
//!
//! # RBSP trailing-bit tolerance
//!
//! After the last message the RBSP should hold exactly
//! `rbsp_stop_one_bit` + alignment zeros. This parser tolerates the
//! two real-world deviations (both observed in encoder wilds): a
//! final byte of pure `0x80` handled by the spec's
//! `more_rbsp_data()` rule, and all-zero tail padding (counted in
//! [`ParsedSei::zero_tail_bytes`], never treated as a message).
//!
//! # Damage model
//!
//! Typed [`SeiError`]s: truncated payloads, short user-data UUIDs,
//! Exp-Golomb range violations, and a message-count budget (a
//! malformed `payloadType=0xFF…` chain of empty payloads is
//! input-bounded anyway — every message consumes ≥ 2 bytes — the
//! budget is the allocation bound). No panics, no hangs.

use core::fmt;

use super::slice::{BitReadError, RbspBitReader};

/// The x264 encoder's unregistered-user-data UUID
/// (`dc45e9bd-e6d9-48b7-962c-d820d923eeef`), verified against the
/// corpus's SEI NALs (both 0019-raw and the 0014 mkv lane carry the
/// `"x264 - core 164 …"` build string under it).
pub const X264_UUID: [u8; 16] = [
    0xdc, 0x45, 0xe9, 0xbd, 0xe6, 0xd9, 0x48, 0xb7, 0x96, 0x2c, 0xd8, 0x20, 0xd9, 0x23, 0xee, 0xef,
];

/// The prefix ffmpeg's own encoder-tag SEIs use in the UUID slot
/// (`"Lavc"` — libavcodec's h264 SEI writers open user data with
/// the library name; matched best-effort, the x264 UUID above is
/// the corpus-verified case).
pub const FFMPEG_LAVC_PREFIX: [u8; 4] = *b"Lavc";

/// Upper bound on messages per SEI NAL (spec streams carry < 8;
/// empty-payload mutation chains are the reason for a bound).
const MAX_MESSAGES: usize = 64;

/// Typed SEI parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeiError {
    /// Bit-level truncation or Exp-Golomb range violation.
    Bit(BitReadError),
    /// The declared `payloadSize` exceeds the RBSP remainder.
    TruncatedPayload {
        /// The payload type being read.
        payload_type: u64,
        /// The declared size.
        size: u64,
        /// Bytes actually left.
        available: usize,
    },
    /// user_data_unregistered payload shorter than the fixed
    /// 16-byte UUID.
    UserDataTooShort {
        /// Payload bytes present.
        size: u64,
    },
    /// More messages than the malformed-stream budget.
    MessageBudgetExceeded,
}

impl fmt::Display for SeiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SeiError::Bit(e) => write!(f, "sei bit read: {e}"),
            SeiError::TruncatedPayload {
                payload_type,
                size,
                available,
            } => write!(
                f,
                "sei payload type {payload_type} declares {size} bytes, {available} left"
            ),
            SeiError::UserDataTooShort { size } => write!(
                f,
                "user_data_unregistered payload {size} bytes < 16-byte uuid"
            ),
            SeiError::MessageBudgetExceeded => {
                write!(f, "more than {MAX_MESSAGES} sei messages in one nal")
            }
        }
    }
}

impl std::error::Error for SeiError {}

impl From<BitReadError> for SeiError {
    fn from(e: BitReadError) -> Self {
        SeiError::Bit(e)
    }
}

/// One parsed SEI message (see module docs for the per-type scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeiMessage {
    /// `buffering_period` (type 0), minimal: the
    /// `seq_parameter_set_id` only — CPB removal/initial delay
    /// arrays need the SPS VUI HRD parameters (card 5.F1a's parse;
    /// Lead-merge unification point).
    BufferingPeriod {
        /// `seq_parameter_set_id` (`ue(v)`).
        seq_parameter_set_id: u32,
    },
    /// `pic_timing` (type 1): presence + length. Field decode needs
    /// the VUI HRD delay widths and `pic_struct_present_flag` —
    /// same unification point as [`SeiMessage::BufferingPeriod`].
    PicTiming {
        /// Payload bytes carried by the message.
        payload_len: u64,
    },
    /// `user_data_unregistered` (type 5): 16-byte UUID + data.
    UserDataUnregistered(UserDataUnregistered),
    /// `recovery_point` (type 6).
    RecoveryPoint(RecoveryPoint),
    /// `filler_payload` (type 12).
    Filler(FillerPayload),
    /// Any other payload type: type + size recorded, payload
    /// skipped intact.
    Unparsed {
        /// The `payloadType`.
        payload_type: u64,
        /// The `payloadSize`.
        size: u64,
    },
}

/// `user_data_unregistered` payload (§D.9 / the type-5 grammar in
/// the 2009+ spec: `uuid_iso_iec_11578` is 16 raw bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDataUnregistered {
    /// The 16-byte `uuid_iso_iec_11578`.
    pub uuid: [u8; 16],
    /// The user-data bytes after the UUID.
    pub data: Vec<u8>,
}

impl UserDataUnregistered {
    /// True when the UUID is x264's (the corpus's encoder).
    pub fn is_x264(&self) -> bool {
        self.uuid == X264_UUID
    }

    /// True when the UUID opens with ffmpeg's `Lavc` prefix.
    pub fn is_ffmpeg_lavc(&self) -> bool {
        self.uuid.starts_with(&FFMPEG_LAVC_PREFIX)
    }

    /// The user data as a lossy UTF-8 string (build strings are
    /// ASCII; lossy keeps mutated streams panic-free).
    pub fn as_str_lossy(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

/// `recovery_point` payload (§D.2.5 / Annex D recovery point).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryPoint {
    /// `recovery_frame_cnt` (`ue(v)`).
    pub recovery_frame_cnt: u32,
    /// `exact_match_flag` (`u(1)`).
    pub exact_match_flag: bool,
    /// `broken_link_flag` (`u(1)`).
    pub broken_link_flag: bool,
    /// `changing_slice_group_idc` (`u(2)`).
    pub changing_slice_group_idc: u8,
}

/// `filler_payload` (§7.3.2.3.2 / D.x: `ff_byte` run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillerPayload {
    /// Declared payload size.
    pub size: u64,
    /// True when every filler byte is `0xFF` (spec-conformant).
    pub all_ff: bool,
}

/// A parsed SEI NAL (all messages + trailing-bit accounting).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedSei {
    /// The messages, in stream order.
    pub messages: Vec<SeiMessage>,
    /// All-zero tail bytes tolerated after the last message (see
    /// module docs — recorded, never parsed as a message).
    pub zero_tail_bytes: usize,
}

/// Parse an SEI NAL RBSP (the NAL header byte already removed,
/// emulation prevention already stripped — card 5.F1a's layer).
///
/// # Errors
/// Typed [`SeiError`]s — never a panic, never a hang.
pub fn parse_sei(rbsp: &[u8]) -> Result<ParsedSei, SeiError> {
    let mut messages = Vec::new();
    let mut pos: usize = 0;
    while more_rbsp_data(&rbsp[pos..]) {
        if messages.len() >= MAX_MESSAGES {
            return Err(SeiError::MessageBudgetExceeded);
        }
        let payload_type = read_accumulated(rbsp, &mut pos);
        let size = read_accumulated(rbsp, &mut pos);
        let size_us = usize::try_from(size).map_err(|_| SeiError::TruncatedPayload {
            payload_type,
            size,
            available: rbsp.len() - pos,
        })?;
        let end = pos.checked_add(size_us).ok_or(SeiError::TruncatedPayload {
            payload_type,
            size,
            available: rbsp.len() - pos,
        })?;
        if end > rbsp.len() {
            return Err(SeiError::TruncatedPayload {
                payload_type,
                size,
                available: rbsp.len() - pos,
            });
        }
        let payload = &rbsp[pos..end];
        pos = end;
        messages.push(match payload_type {
            0 => {
                let mut r = RbspBitReader::new(payload);
                SeiMessage::BufferingPeriod {
                    seq_parameter_set_id: r.read_ue()?,
                }
            }
            1 => SeiMessage::PicTiming {
                payload_len: u64::try_from(payload.len()).unwrap_or(u64::MAX),
            },
            5 => {
                if payload.len() < 16 {
                    return Err(SeiError::UserDataTooShort {
                        size: u64::try_from(payload.len()).unwrap_or(u64::MAX),
                    });
                }
                let mut uuid = [0u8; 16];
                uuid.copy_from_slice(&payload[..16]);
                SeiMessage::UserDataUnregistered(UserDataUnregistered {
                    uuid,
                    data: payload[16..].to_vec(),
                })
            }
            6 => {
                let mut r = RbspBitReader::new(payload);
                SeiMessage::RecoveryPoint(RecoveryPoint {
                    recovery_frame_cnt: r.read_ue()?,
                    exact_match_flag: r.read_bit()?,
                    broken_link_flag: r.read_bit()?,
                    changing_slice_group_idc: r.read_bits(2)? as u8,
                })
            }
            12 => SeiMessage::Filler(FillerPayload {
                size,
                all_ff: payload.iter().all(|&b| b == 0xFF),
            }),
            other => SeiMessage::Unparsed {
                payload_type: other,
                size,
            },
        });
    }
    let zero_tail_bytes = rbsp[pos..].iter().filter(|&&b| b == 0).count();
    Ok(ParsedSei {
        messages,
        zero_tail_bytes,
    })
}

/// §9 (`more_rbsp_data()`): true when the remaining bytes hold
/// anything beyond the single trailing `rbsp_stop_one_bit` — i.e.
/// at least two set bits remain (a payload byte plus the stop bit).
/// An all-zero remainder carries no data (tolerated, counted).
fn more_rbsp_data(rest: &[u8]) -> bool {
    let mut set_bits = 0usize;
    for &b in rest {
        set_bits += b.count_ones() as usize;
        if set_bits > 1 {
            return true;
        }
    }
    false
}

/// `payloadType` / `payloadSize`: accumulate `0xFF` continuation
/// bytes then the final `< 0xFF` byte (§7.3.2.3). `pos` advances
/// past the read; on an exhausted remainder this returns the last
/// accumulation (the caller's payload-bounds check turns it into a
/// typed truncation).
fn read_accumulated(rbsp: &[u8], pos: &mut usize) -> u64 {
    let mut value: u64 = 0;
    while *pos < rbsp.len() {
        let b = rbsp[*pos];
        *pos += 1;
        value += u64::from(b);
        if b != 0xFF {
            break;
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::super::slice::tests::{annexb_nals, corpus_path};
    use super::*;

    /// SEI NALs (header byte stripped, EP removed) of a corpus file.
    fn corpus_seis(name: &str) -> Vec<Vec<u8>> {
        let Some(path) = corpus_path(name) else {
            return Vec::new();
        };
        let data = std::fs::read(path).unwrap_or_default();
        annexb_nals(&data)
            .into_iter()
            .filter(|(h, _)| (h & 0x1F) == 6)
            .map(|(_, rbsp)| rbsp)
            .collect()
    }

    #[test]
    fn corpus_0019_carries_the_x264_build_string_and_a_recovery_point() {
        let seis = corpus_seis("0019-raw-video.h264");
        if seis.is_empty() {
            eprintln!("corpus: 0019-raw-video.h264 not present — skipping");
            return;
        }
        assert_eq!(seis.len(), 2);
        let first = parse_sei(&seis[0]).unwrap_or_else(|e| unreachable!("0019 sei[0]: {e}"));
        assert_eq!(first.zero_tail_bytes, 0);
        assert_eq!(first.messages.len(), 1);
        match &first.messages[0] {
            SeiMessage::UserDataUnregistered(ud) => {
                assert!(ud.is_x264());
                assert!(!ud.is_ffmpeg_lavc());
                let s = ud.as_str_lossy();
                // The exact build string observed in the corpus NAL
                // (hex-dumped pre-parse, worklog 5.F1b).
                assert!(
                    s.starts_with("x264 - core 164 r3108 31e19f9 - H.264/MPEG-4 AVC codec"),
                    "unexpected user data: {s}"
                );
                assert!(s.contains("Copyleft 2003-2023"));
                assert!(s.contains("options: cabac=0"));
            }
            other => unreachable!("0019 sei[0] should be user data, got {other:?}"),
        }
        // Hand-decoded from the corpus bytes `06 06 01 c4 80`:
        // recovery_frame_cnt 0, exact_match 1, broken_link 0, idc 0.
        let second = parse_sei(&seis[1]).unwrap_or_else(|e| unreachable!("0019 sei[1]: {e}"));
        assert_eq!(
            second.messages,
            vec![SeiMessage::RecoveryPoint(RecoveryPoint {
                recovery_frame_cnt: 0,
                exact_match_flag: true,
                broken_link_flag: false,
                changing_slice_group_idc: 0,
            })]
        );
    }

    #[test]
    fn corpus_0014_mkv_lane_carries_the_same_x264_uuid() {
        let Some(bytes) = super::super::slice::tests::corpus_0014_annexb() else {
            eprintln!("corpus: 0014-muxed-av.mkv or ffmpeg not present — skipping");
            return;
        };
        let seis: Vec<Vec<u8>> = annexb_nals(&bytes)
            .into_iter()
            .filter(|(h, _)| (h & 0x1F) == 6)
            .map(|(_, rbsp)| rbsp)
            .collect();
        assert_eq!(seis.len(), 1);
        let parsed = parse_sei(&seis[0]).unwrap_or_else(|e| unreachable!("0014 sei: {e}"));
        assert_eq!(parsed.messages.len(), 1);
        match &parsed.messages[0] {
            SeiMessage::UserDataUnregistered(ud) => {
                assert!(ud.is_x264());
                assert!(ud.as_str_lossy().starts_with("x264 - core 164"));
            }
            other => unreachable!("0014 sei should be user data, got {other:?}"),
        }
    }

    #[test]
    fn synthesized_payloads_parse_and_round_trip_their_fields() {
        // buffering_period: type 0, size 1, payload ue(0)=0x80.
        let bp = parse_sei(&[0x00, 0x01, 0x80, 0x80])
            .unwrap_or_else(|e| unreachable!("buffering_period: {e}"));
        assert_eq!(
            bp.messages,
            vec![SeiMessage::BufferingPeriod {
                seq_parameter_set_id: 0
            }]
        );

        // recovery_point: type 6, size 1, payload 0x50 =
        // ue(1)="010", exact=1, broken=0, idc=00.
        let rp = parse_sei(&[0x06, 0x01, 0x50, 0x80])
            .unwrap_or_else(|e| unreachable!("recovery_point: {e}"));
        assert_eq!(
            rp.messages,
            vec![SeiMessage::RecoveryPoint(RecoveryPoint {
                recovery_frame_cnt: 1,
                exact_match_flag: true,
                broken_link_flag: false,
                changing_slice_group_idc: 0,
            })]
        );

        // filler: type 12, size 3, payload ff ff ff.
        let fill = parse_sei(&[0x0C, 0x03, 0xFF, 0xFF, 0xFF, 0x80])
            .unwrap_or_else(|e| unreachable!("filler: {e}"));
        assert_eq!(
            fill.messages,
            vec![SeiMessage::Filler(FillerPayload {
                size: 3,
                all_ff: true
            })]
        );
        // Non-0xFF filler bytes are recorded, not rejected.
        let bad_fill = parse_sei(&[0x0C, 0x02, 0xFF, 0x00, 0x80])
            .unwrap_or_else(|e| unreachable!("bad filler: {e}"));
        assert_eq!(
            bad_fill.messages,
            vec![SeiMessage::Filler(FillerPayload {
                size: 2,
                all_ff: false
            })]
        );

        // user_data_unregistered with the Lavc prefix (ffmpeg form):
        // 16-byte uuid + 8 data bytes = 0x18 payload.
        let mut ud_nal = vec![0x05, 0x18];
        ud_nal.extend_from_slice(&FFMPEG_LAVC_PREFIX);
        ud_nal.extend_from_slice(&[0u8; 12]);
        ud_nal.extend_from_slice(b"58.9.100");
        ud_nal.push(0x80);
        let ud = parse_sei(&ud_nal).unwrap_or_else(|e| unreachable!("lavc user data: {e}"));
        match &ud.messages[0] {
            SeiMessage::UserDataUnregistered(u) => {
                assert!(u.is_ffmpeg_lavc());
                assert!(!u.is_x264());
                assert_eq!(u.as_str_lossy(), "58.9.100");
            }
            other => unreachable!("expected user data, got {other:?}"),
        }

        // Unparsed type 3 (reserve): recorded, skipped intact.
        let unp = parse_sei(&[0x03, 0x02, 0xAA, 0xBB, 0x80])
            .unwrap_or_else(|e| unreachable!("unparsed: {e}"));
        assert_eq!(
            unp.messages,
            vec![SeiMessage::Unparsed {
                payload_type: 3,
                size: 2
            }]
        );

        // Multi-message NAL + 0xFF continuation size accumulation
        // (255 + 1 = 256-byte payload).
        let mut multi = vec![0x0C, 0xFF, 0x01];
        multi.extend(std::iter::repeat_n(0xFF, 256));
        multi.push(0x80);
        let m = parse_sei(&multi).unwrap_or_else(|e| unreachable!("multi: {e}"));
        assert_eq!(
            m.messages,
            vec![SeiMessage::Filler(FillerPayload {
                size: 256,
                all_ff: true
            })]
        );
    }

    #[test]
    fn trailing_bit_forms_are_tolerated_and_counted() {
        // Clean stop bit only.
        let clean = parse_sei(&[0x06, 0x01, 0xC4, 0x80])
            .unwrap_or_else(|e| unreachable!("clean stop: {e}"));
        assert_eq!(clean.messages.len(), 1);
        assert_eq!(clean.zero_tail_bytes, 0);
        // All-zero tail padding after the message (encoder quirk):
        // tolerated, counted, not a message.
        let padded = parse_sei(&[0x06, 0x01, 0xC4, 0x80, 0x00, 0x00])
            .unwrap_or_else(|e| unreachable!("padded: {e}"));
        assert_eq!(padded.messages.len(), 1);
        assert_eq!(padded.zero_tail_bytes, 2);
        // Empty RBSP: no messages, no panic.
        let empty = parse_sei(&[]).unwrap_or_else(|e| unreachable!("empty: {e}"));
        assert!(empty.messages.is_empty());
        // Bare trailing byte: no message.
        let bare = parse_sei(&[0x80]).unwrap_or_else(|e| unreachable!("bare: {e}"));
        assert!(bare.messages.is_empty());
    }

    #[test]
    fn error_paths_are_typed() {
        // Declared size larger than the remainder.
        assert_eq!(
            parse_sei(&[0x05, 0x40, 0xAB]),
            Err(SeiError::TruncatedPayload {
                payload_type: 5,
                size: 0x40,
                available: 1,
            })
        );
        // user_data_unregistered shorter than the 16-byte UUID.
        assert_eq!(
            parse_sei(&[0x05, 0x04, 0xDE, 0xAD, 0xBE, 0xEF, 0x80]),
            Err(SeiError::UserDataTooShort { size: 4 })
        );
        // buffering_period payload of pure zeros → 31+ leading
        // zeros with no marker bit → golomb overflow.
        assert!(matches!(
            parse_sei(&[0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x80]),
            Err(SeiError::Bit(BitReadError::GolombOverflow { .. }))
        ));
        // 0xFF continuation chains whose size overruns the input →
        // the payload-bounds check trips typed (no hang, no overread).
        assert!(matches!(
            parse_sei(&[0xFF, 0xFF, 0x05, 0xFF, 0x10]),
            Err(SeiError::TruncatedPayload { .. })
        ));
    }

    /// +FB (card 5.F1b): deterministic mutation property — mutants
    /// parse Ok or typed-Err, never panic, never hang.
    #[test]
    fn mutated_sei_streams_are_typed_or_ok_never_panics() {
        let seis = corpus_seis("0019-raw-video.h264");
        if seis.is_empty() {
            eprintln!("corpus: 0019 not present — skipping sei mutation property");
            return;
        }
        let mut seed: u64 = 0x5E1B_0DBA_1100;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut oks = 0;
        let mut errs = 0;
        for round in 0..200u32 {
            let mut mutant = seis[next() as usize % seis.len()].clone();
            let flips = 1 + (next() as usize % 3);
            for _ in 0..flips {
                if mutant.is_empty() {
                    break;
                }
                let byte = next() as usize % mutant.len();
                mutant[byte] ^= 1 << (next() as u32 % 8);
            }
            if round % 4 == 0 {
                let cut = next() as usize % (mutant.len() + 1);
                mutant.truncate(cut);
            }
            match parse_sei(&mutant) {
                Ok(p) => {
                    oks += 1;
                    assert!(p.messages.len() <= 64);
                }
                Err(_) => errs += 1,
            }
        }
        assert_eq!(oks + errs, 200);
        assert!(errs > 20, "mutations must bite (errs={errs})");
    }
}
