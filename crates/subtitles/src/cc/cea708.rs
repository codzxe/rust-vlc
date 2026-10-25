//! CEA-708 (CTA-708) decode — the STAGED half of card 5.N3.
//!
//! # What is implemented (honest scope)
//!
//! * **Fragment reassembly** ([`DtvccReassembler`]): the `cc_data()`
//!   DTVCC packet layer. A `cc_type == 2` triplet starts a packet —
//!   its first byte carries `sequence_number` (2 bits, ignored: the
//!   reference decoders tolerate loss by re-locking on any start)
//!   and `packet_size_code` (6 bits); the expected packet length is
//!   `2 × packet_size_code + 1` bytes (the start triplet contributes
//!   one byte, each `cc_type == 3` continuation two). `cc_type == 3`
//!   without a start is typed damage, not a panic.
//! * **Service-block layer** ([`parse_dtvcc_packet`]): the packet
//!   walk — `[service_number(4)][block_size(4)]` headers, the
//!   two-byte escape form for services 8–63, `block_size == 0`
//!   meaning 32 bytes, truncated blocks typed.
//! * **Service 1 → EIA-608 route**: the compatibility service. Its
//!   payload is the 608 byte stream (parity re-synthesized the same
//!   way as SCTE-20 — the F1 carriage does not guarantee the parity
//!   bit) and feeds [`super::eia608`] on field 1.
//! * **Native services 2–63: recorded, not interpreted**
//!   ([`Cc708ServiceRecord`]): the service block is WALKED with the
//!   CTA-708 §8.4.9 command-length table (so a record proves the
//!   stream parses) and reduced to an op histogram — the window
//!   model (DEFINE_WINDOW geometry, pen/color state, per-window
//!   addressing, borders, word wrap, the G0–G3 glyph planes) is
//!   explicitly OUT. See the honesty table in [`super`].
//!
//! # Never
//!
//! Panics, unsafe, unbounded allocation (the reassembly buffer is
//! capped at [`MAX_DTVCC_PACKET`]).

use super::CcDamage;

/// The DTVCC packet cap, bytes (CTA-708: packets never exceed 128;
/// the size code tops out at 127).
pub const MAX_DTVCC_PACKET: usize = 128;

/// One native-service block record: walked, counted, not rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cc708ServiceRecord {
    /// The service number (2–63 for native blocks).
    pub service: u8,
    /// Total payload bytes across this packet's blocks for the
    /// service.
    pub bytes: usize,
    /// The op codes seen (each once, stream order, capped at
    /// [`MAX_RECORDED_OPS`]).
    pub ops: Vec<Cc708Op>,
}

/// Cap on the distinct-op record (D-2: bounded bookkeeping).
pub const MAX_RECORDED_OPS: usize = 32;

/// The CTA-708 §8.4.9 command families, as walked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Cc708Op {
    /// `0x00`–`0x0F` — NUL/ETX/BS/FF/CR/HCR + reserved slots.
    Miscellaneous,
    /// `0x10` — the EXT1 escape (two-byte G2/G3 codes).
    Ext1,
    /// `0x11`–`0x17` — attribute codes.
    Attribute,
    /// `0x18`–`0x1F` — preamble address codes.
    Preamble,
    /// `0x20`–`0x7F` — G0 plane text.
    TextG0,
    /// `0x80`–`0x87` — SET_CURRENT_WINDOW.
    SetCurrentWindow,
    /// `0x88` CLEAR_WINDOWS.
    ClearWindows,
    /// `0x89` DISPLAY_WINDOWS.
    DisplayWindows,
    /// `0x8A` HIDE_WINDOWS.
    HideWindows,
    /// `0x8B` TOGGLE_WINDOWS.
    ToggleWindows,
    /// `0x8C` DELETE_WINDOWS.
    DeleteWindows,
    /// `0x8D` DELAY.
    Delay,
    /// `0x8E` DELAY_CANCEL.
    DelayCancel,
    /// `0x8F` RESET.
    Reset,
    /// `0x90` SET_PEN_ATTRIBUTES.
    SetPenAttributes,
    /// `0x91` SET_PEN_COLOR.
    SetPenColor,
    /// `0x92` SET_PEN_FONT.
    SetPenFont,
    /// `0x97` SET_WINDOW_ATTRIBUTES.
    SetWindowAttributes,
    /// `0x98`–`0x9F` DEFINE_WINDOW.
    DefineWindow,
    /// `0xA0`–`0xFF` — G1 plane text.
    TextG1,
    /// Anything not covered above (reserved).
    Reserved,
}

impl Cc708Op {
    fn classify(code: u8) -> Cc708Op {
        match code {
            0x00..=0x0F => Cc708Op::Miscellaneous,
            0x10 => Cc708Op::Ext1,
            0x11..=0x17 => Cc708Op::Attribute,
            0x18..=0x1F => Cc708Op::Preamble,
            0x20..=0x7F => Cc708Op::TextG0,
            0x80..=0x87 => Cc708Op::SetCurrentWindow,
            0x88 => Cc708Op::ClearWindows,
            0x89 => Cc708Op::DisplayWindows,
            0x8A => Cc708Op::HideWindows,
            0x8B => Cc708Op::ToggleWindows,
            0x8C => Cc708Op::DeleteWindows,
            0x8D => Cc708Op::Delay,
            0x8E => Cc708Op::DelayCancel,
            0x8F => Cc708Op::Reset,
            0x90 => Cc708Op::SetPenAttributes,
            0x91 => Cc708Op::SetPenColor,
            0x92 => Cc708Op::SetPenFont,
            0x93..=0x96 => Cc708Op::Reserved,
            0x97 => Cc708Op::SetWindowAttributes,
            0x98..=0x9F => Cc708Op::DefineWindow,
            0xA0..=0xFF => Cc708Op::TextG1,
        }
    }

    /// The command's total length in bytes (opcode + parameters),
    /// per the CTA-708 §8.4.9 table as walked by the reference
    /// decoders.
    #[must_use]
    pub const fn total_len(self) -> usize {
        match self {
            // EXT1 + the escaped G2/G3 code byte.
            Cc708Op::Ext1 => 2,
            // Preambles carry a parameter byte.
            Cc708Op::Preamble => 2,
            // The window-bitmap and delay commands carry one byte.
            Cc708Op::ClearWindows
            | Cc708Op::DisplayWindows
            | Cc708Op::HideWindows
            | Cc708Op::ToggleWindows
            | Cc708Op::DeleteWindows
            | Cc708Op::Delay => 2,
            Cc708Op::SetPenAttributes => 3,
            Cc708Op::SetPenColor => 4,
            Cc708Op::SetPenFont => 4,
            Cc708Op::SetWindowAttributes => 5,
            Cc708Op::DefineWindow => 7,
            // Everything else is a single byte.
            _ => 1,
        }
    }
}

/// The DTVCC packet reassembler (the `cc_type` 2/3 half of
/// `cc_data()`).
#[derive(Debug, Clone, Default)]
pub(crate) struct DtvccReassembler {
    buf: Vec<u8>,
    expected: usize,
    active: bool,
}

impl DtvccReassembler {
    /// Feed one triplet's payload. Returns the completed packet when
    /// the expected length is reached (or exceeded).
    pub(crate) fn push(
        &mut self,
        cc_type: u8,
        data: [u8; 2],
        damage: &mut Vec<CcDamage>,
    ) -> Option<Vec<u8>> {
        match cc_type {
            2 => {
                // Start: re-lock unconditionally (a lost start is
                // recovered by the next one).
                let code = (data[0] & 0x3F) as usize;
                self.expected = code * 2 + 1;
                if self.expected > MAX_DTVCC_PACKET {
                    damage.push(CcDamage::DtvccOverrun {
                        size: self.expected,
                    });
                    self.active = false;
                    self.buf.clear();
                    return None;
                }
                self.buf.clear();
                self.buf.push(data[1]);
                self.active = true;
                None
            }
            _ => {
                if !self.active {
                    damage.push(CcDamage::DtvContinuationWithoutStart);
                    return None;
                }
                self.buf.extend_from_slice(&data);
                if self.buf.len() >= self.expected {
                    self.active = false;
                    Some(std::mem::take(&mut self.buf))
                } else {
                    None
                }
            }
        }
    }

    /// Drop any partial state (a seek/reset).
    pub(crate) fn reset(&mut self) {
        self.buf.clear();
        self.active = false;
        self.expected = 0;
    }
}

/// One parsed service block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cc708ServiceBlock {
    /// The service number (0 = the 608 compatibility service).
    pub service: u8,
    /// The block payload (trailing bytes when the packet ended
    /// early — [`CcDamage::DtvccTruncatedBlock`] was pushed).
    pub payload: Vec<u8>,
}

/// Walk one reassembled DTVCC packet into service blocks.
///
/// # Damage
/// [`CcDamage::DtvccBadEscape`] / [`CcDamage::UnknownService708`] /
/// [`CcDamage::DtvccTruncatedBlock`] — the walk stops on the first
/// malformed header, keeping what parsed.
pub(crate) fn parse_dtvcc_packet(
    packet: &[u8],
    damage: &mut Vec<CcDamage>,
) -> Vec<Cc708ServiceBlock> {
    let mut blocks = Vec::new();
    let mut pos = 0usize;
    while pos < packet.len() {
        let header = packet[pos];
        pos += 1;
        let mut service = header >> 4;
        let size = header & 0x0F;
        if service == 7 {
            let Some(&escape) = packet.get(pos) else {
                damage.push(CcDamage::DtvccTruncatedBlock {
                    service: 7,
                    needed: 1,
                    got: 0,
                });
                break;
            };
            pos += 1;
            if escape & 0xC0 != 0x40 {
                damage.push(CcDamage::DtvccBadEscape { got: escape });
                break;
            }
            service = 8 + (escape & 0x3F);
            if service > 63 {
                damage.push(CcDamage::UnknownService708 { service });
                break;
            }
        }
        if service == 0 {
            damage.push(CcDamage::UnknownService708 { service: 0 });
            break;
        }
        // block_size 0 means 32 bytes (CTA-708 §? service header).
        let len = if size == 0 { 32 } else { size as usize };
        let end = pos.saturating_add(len).min(packet.len());
        if end - pos < len {
            damage.push(CcDamage::DtvccTruncatedBlock {
                service,
                needed: len,
                got: end - pos,
            });
        }
        blocks.push(Cc708ServiceBlock {
            service,
            payload: packet[pos..end].to_vec(),
        });
        pos = end;
    }
    blocks
}

/// Walk a native (2–63) service block's payload with the
/// command-length table, producing the honest record.
pub(crate) fn walk_native_block(payload: &[u8], record: &mut Cc708ServiceRecord) {
    record.bytes += payload.len();
    let mut pos = 0usize;
    while pos < payload.len() {
        let op = Cc708Op::classify(payload[pos]);
        let len = op.total_len().min(payload.len() - pos);
        if record.ops.len() < MAX_RECORDED_OPS && !record.ops.contains(&op) {
            record.ops.push(op);
        }
        pos += len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn reassembly_locks_on_start_and_completes() {
        let mut r = DtvccReassembler::default();
        let mut d = Vec::new();
        // packet_size_code 2 -> expected 5 bytes: the start's second
        // byte plus two continuations.
        assert!(r.push(2, [0x42, 0x11], &mut d).is_none());
        assert!(r.push(3, [0x22, 0x33], &mut d).is_none());
        let done = r.push(3, [0x44, 0x55], &mut d).must();
        assert_eq!(done, vec![0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(d.is_empty());
    }

    #[test]
    fn continuation_without_start_is_typed() {
        let mut r = DtvccReassembler::default();
        let mut d = Vec::new();
        assert!(r.push(3, [1, 2], &mut d).is_none());
        assert!(matches!(
            d.first(),
            Some(CcDamage::DtvContinuationWithoutStart)
        ));
    }

    #[test]
    fn size_code_caps_below_the_packet_guard() {
        // The 6-bit size code tops out at 63 -> 127 bytes, one under
        // the 128-byte cap: the DtvccOverrun guard is defensive only
        // (reachable never by a conforming size field).
        let mut r = DtvccReassembler::default();
        let mut d = Vec::new();
        r.push(2, [0x7F, 0], &mut d); // low 6 bits = 63
        assert!(d.is_empty());
        assert_eq!(r.expected, 127);
        // A start re-locks mid-packet without damage.
        r.push(2, [0x41, 0x11], &mut d);
        assert!(d.is_empty());
        assert_eq!(r.expected, 3);
    }

    #[test]
    fn service_blocks_parse_with_escape_and_zero_size() {
        let mut d = Vec::new();
        // svc1 size2 + 2 bytes; svc2 size1 + 1 byte; escape svc9 size1.
        let packet = [0x12, 0xAA, 0xBB, 0x21, 0xCC, 0x71, 0x41, 0xDD];
        let blocks = parse_dtvcc_packet(&packet, &mut d);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].service, 1);
        assert_eq!(blocks[0].payload, vec![0xAA, 0xBB]);
        assert_eq!(blocks[1].service, 2);
        assert_eq!(blocks[2].service, 8 + 1);
        assert!(d.is_empty());
        // block_size 0 means 32 bytes.
        let mut d = Vec::new();
        let blocks = parse_dtvcc_packet(&[0x20, 0xAA], &mut d);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].payload.len(), 1);
        assert!(matches!(
            d.first(),
            Some(CcDamage::DtvccTruncatedBlock {
                needed: 32,
                got: 1,
                ..
            })
        ));
    }

    #[test]
    fn truncated_and_bad_headers_are_typed() {
        let mut d = Vec::new();
        let blocks = parse_dtvcc_packet(&[0x12, 0xAA], &mut d);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(
            d.first(),
            Some(CcDamage::DtvccTruncatedBlock {
                needed: 2,
                got: 1,
                ..
            })
        ));
        let mut d = Vec::new();
        let blocks = parse_dtvcc_packet(&[0x70, 0x00], &mut d); // escape with bad prefix
        assert!(blocks.is_empty());
        assert!(matches!(
            d.first(),
            Some(CcDamage::DtvccBadEscape { got: 0x00 })
        ));
        let mut d = Vec::new();
        let blocks = parse_dtvcc_packet(&[0x01, 0xAA], &mut d); // service 0
        assert!(blocks.is_empty());
        assert!(matches!(
            d.first(),
            Some(CcDamage::UnknownService708 { service: 0 })
        ));
    }

    #[test]
    fn native_walk_counts_ops() {
        let mut rec = Cc708ServiceRecord {
            service: 2,
            bytes: 0,
            ops: Vec::new(),
        };
        // DEFINE_WINDOW (7) + SET_PEN_COLOR (4) + G0 text.
        let payload = [0x98, 1, 2, 3, 4, 5, 6, 0x91, 1, 2, 3, b'h', b'i'];
        walk_native_block(&payload, &mut rec);
        assert_eq!(rec.bytes, payload.len());
        assert!(rec.ops.contains(&Cc708Op::DefineWindow));
        assert!(rec.ops.contains(&Cc708Op::SetPenColor));
        assert!(rec.ops.contains(&Cc708Op::TextG0));
        assert_eq!(rec.ops.len(), 3);
    }
}
