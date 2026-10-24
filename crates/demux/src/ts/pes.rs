//! The MPEG-TS **PES layer** and its timing model (card 5.I1, the
//! I1b half: PES + timing; the parallel I1a half owns PSI, the Lead
//! unifies the transport walk).
//!
//! # What this is
//!
//! [`PesHeader`] parses an elementary-stream packet header per
//! ISO/IEC 13818-1 §2.4.3.7: the `packet_start_code_prefix`, the
//! [`stream_id`] classification (Table 2-18), `PES_packet_length`
//! (**0 = unbounded** — the video convention: the packet runs until
//! the next start code on that PID), the optional-header flags with
//! `data_alignment_indicator`, and the 33-bit **PTS/DTS** stamps
//! spread across marker-guarded byte fields at the 90 kHz MPEG
//! clock ([`MPEG_CLOCK_HZ`]). [`PesAssembler`] is the byte-assembly
//! seam: it turns a per-PID run of transport-packet payloads into
//! complete [`PesPacket`]s (bounded PES complete at
//! `6 + PES_packet_length`; unbounded PES complete at the next
//! `payload_unit_start`, or at the honest end-of-stream flush).
//!
//! # Timing (the honest contract)
//!
//! A PTS/DTS value **is** ticks of the MPEG clock: the demux
//! plumbing convention (the Matroska precedent — ticks in the
//! track's own base) gives every TS track
//! `TimeBase::new(1, 90_000)` and `Timestamp::new(pts, base)`
//! ([`mpeg_time_base`]/[`pts_stamp`]). Nothing is ever smoothed: a
//! regressed DTS, a >[`MAX_SANE_STEP_TICKS`] DTS jump, a PTS
//! regression on a stream emitting no DTS, a set
//! `discontinuity_indicator`, or a regressed PCR is a
//! [`TimingEvent`] on the drain queue (mapped by the demuxer to
//! `DemuxEvent::Discontinuity` — never silent, MP §8). B-frame PTS
//! *reordering with a monotone DTS is not damage* and emits nothing
//! (verified against the corpus's B-frame-storm lane).
//!
//! # Damage policy (bounded, honest)
//!
//! Mid-stream damage is recovered-and-counted (the `Recovered`
//! event shape): stray continuation payload before any start, a
//! start code that never comes, a bounded packet overrunning its
//! declared length, a mid-stream PES whose header cannot be parsed
//! (short header, bad PTS marker bits) — the bytes are dropped,
//! counted in [`PesAssembler::damage_bytes`], and surfaced via
//! [`PesAssembler::take_recovered`]. The end-of-stream flush
//! ([`PesAssembler::finish`]) is stricter: a bounded PES still short
//! of its declared length is the typed sticky truncation
//! (`demux.ts.pes.truncated`), and an unbounded one is emitted as
//! the final packet (ffmpeg's own last-packet behavior, verified by
//! the corpus differential). Accumulation is ceiling-checked
//! ([`MAX_PES_BYTES`], MP §12): exceeding it is the typed
//! `ResourceLimit` refusal, never an unbounded read.
//!
//! # The transport seam (for the Lead's I1 unification)
//!
//! [`PesAssembler::push_packet`] receives the payload of one
//! transport packet as-is — the `pointer_field` is a **PSI-only**
//! construct (§2.4.4.1); a PES packet commences directly at the
//! payload when `payload_unit_start_indicator` is set (§2.4.4.2,
//! verified on the corpus bytes: the first payload bytes after the
//! adaptation field are `00 00 01 E0`). The transport layer (I1a)
//! parses the adaptation field and hands this layer the
//! `discontinuity_indicator` bit and PCR observations
//! ([`PesAssembler::observe_pcr`]). The deliberately small
//! parameter list (not a `struct TsPacket`) is the minimal seam:
//! I1a's PSI walker and this layer share byte plumbing without
//! sharing types yet.
//!
//! # Example
//!
//! ```
//! use demux::ts::pes::{PesAssembler, PesStreamClass};
//!
//! # fn main() -> Result<(), media_types::EngineError> {
//! let mut asm = PesAssembler::new();
//! // One bounded audio PES (stream_id 0xC0, length 3): header,
//! // PTS field for tick 128_487, then 3 payload bytes.
//! let pts: u64 = 128_487;
//! let mut pes = vec![0x00, 0x00, 0x01, 0xC0, 0x00, 0x0B, 0x80, 0x80, 0x05];
//! pes.extend_from_slice(&[
//!     (0b0010_u8 << 4) | ((((pts >> 30) & 0x07) << 1) as u8) | 1,
//!     (pts >> 22) as u8,
//!     ((((pts >> 15) & 0x7F) << 1) as u8) | 1,
//!     (pts >> 7) as u8,
//!     (((pts & 0x7F) << 1) as u8) | 1,
//! ]);
//! pes.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
//! asm.push_packet(0x101, true, false, &pes)?;
//! asm.finish()?;
//! let pkt = asm.next_pes().expect("one packet");
//! assert_eq!(pkt.class, PesStreamClass::Audio);
//! assert_eq!(pkt.pts, Some(pts));
//! assert_eq!(pkt.payload, &[0xAA, 0xBB, 0xCC]);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::collections::VecDeque;

use media_types::error::{CorruptDetail, ErrorContext, LimitKind};
use media_types::time::{Stamp, TimeBase, Timestamp};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The MPEG system clock every PES timestamp counts (13818-1 §2.4.2:
/// 90 kHz — the `1/90000` time base ffprobe reports for every TS
/// stream, verified on the corpus).
pub const MPEG_CLOCK_HZ: u32 = 90_000;

/// Maximum bytes one PES packet may accumulate before the typed
/// memory-ceiling refusal (MP §12; the bounded-PES arithmetic max is
/// 6 + 65 535, unbounded video is ceiling-checked here).
pub const MAX_PES_BYTES: usize = 8 * 1024 * 1024;

/// A forward DTS jump larger than this (10 s of clock) is surfaced
/// as a timing event — splices exist, silence does not (MP §8).
pub const MAX_SANE_STEP_TICKS: u64 = 10 * (MPEG_CLOCK_HZ as u64);

const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// A start-code prefix that is not `00 00 01` (the transport seam
/// fed a payload that is not a PES start, or the stream is damaged).
pub fn bad_start_code() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.ts.pes.bad-start-code",
    )
}

/// The PES optional header is structurally short (fewer bytes than
/// its own `header_data_length` / PTS fields need), or the forbidden
/// `PTS_DTS_flags == 01` was set.
pub fn short_header() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: 0,
                found: 0,
            },
        },
        "demux.ts.pes.short-header",
    )
}

/// A PTS/DTS field with marker bits not set per §2.4.3.7 — the
/// 33-bit value cannot be trusted.
pub fn bad_pts() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.ts.pes.bad-pts",
    )
}

/// A bounded PES still short of its declared `PES_packet_length` at
/// the end-of-stream flush (`expected` = the declared total,
/// `found` = the bytes actually assembled).
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.ts.pes.truncated",
    )
}

/// One PES accumulation exceeded [`MAX_PES_BYTES`].
pub fn payload_over_ceiling(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.ts.pes.payload-over-ceiling",
    )
}

/// The MPEG time base could not be built (unreachable for the
/// constant 1/90 000 — the honest refused-panic shape).
pub fn time_base_unavailable() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Timing,
        },
        "demux.ts.pes.time-base",
    )
}

/// The stream-class half of stream_id Table 2-18 (§2.4.4.7): which
/// elementary stream a PES packet carries, and whether the optional
/// PES header (flags + PTS/DTS) applies to it at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PesStreamClass {
    /// `0xE0..=0xEF` — video (H.262/H.264/H.265/…).
    Video,
    /// `0xC0..=0xDF` — audio (MPEG/AAC/…).
    Audio,
    /// `0xBD` — private stream 1 (AC-3/DTS/… ride here); the
    /// optional header applies.
    Private1,
    /// `0xBF` — private stream 2 (no optional header).
    Private2,
    /// `0xBE` — padding: the payload is stuffing bytes, not a packet.
    Padding,
    /// `0xBC` program_stream_map / `0xFF` program_stream_directory —
    /// stream-map structures, no optional header.
    StreamMap,
    /// `0xF0`/`0xF1` — entitlement messages, no optional header.
    Entitlement,
    /// `0xF2`/`0xF3` — DSM-CC / ISO 13522 (MHEG), no optional header.
    Interactive,
    /// `0xF4..=0xFE` — H.222.1 types A–E and reserved.
    Reserved,
}

impl PesStreamClass {
    /// Does this class carry the optional PES header (flags,
    /// `header_data_length`, PTS/DTS)? §2.4.3.7 Table 2-16 applies
    /// to elementary streams and private 1 only.
    fn has_optional_header(self) -> bool {
        matches!(self, Self::Video | Self::Audio | Self::Private1)
    }
}

/// Classify a `stream_id` per Table 2-18. (Values below `0xBC` are
/// not assigned to any PES stream in 13818-1 — they arrive only in
/// damaged streams and classify as [`PesStreamClass::Reserved`].)
pub fn classify_stream_id(id: u8) -> PesStreamClass {
    match id {
        0xBC | 0xFF => PesStreamClass::StreamMap,
        0xBD => PesStreamClass::Private1,
        0xBE => PesStreamClass::Padding,
        0xBF => PesStreamClass::Private2,
        0xC0..=0xDF => PesStreamClass::Audio,
        0xE0..=0xEF => PesStreamClass::Video,
        0xF0 | 0xF1 => PesStreamClass::Entitlement,
        0xF2 | 0xF3 => PesStreamClass::Interactive,
        0xF4..=0xFE => PesStreamClass::Reserved,
        0x00..=0xBB => PesStreamClass::Reserved,
    }
}

/// Decode one 5-byte timestamp field (§2.4.3.7): `prefix` is the
/// 4-bit lead (`0b0010` for a lone PTS, `0b0011` for the PTS of a
/// PTS+DTS pair, `0b0001` for DTS); the three marker bits must be
/// set. The 33-bit value is reassembled across the guarded fields.
fn decode_ts5(bytes: &[u8], prefix: u8) -> Result<u64, EngineError> {
    if bytes.len() < 5 {
        return Err(short_header());
    }
    let (b0, b1, b2, b3, b4) = (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]);
    if b0 >> 4 != prefix || b0 & 0x01 == 0 || b2 & 0x01 == 0 || b4 & 0x01 == 0 {
        return Err(bad_pts());
    }
    Ok((u64::from(b0 & 0x0E) << 29)
        | (u64::from(b1) << 22)
        | (u64::from(b2 & 0xFE) << 14)
        | (u64::from(b3) << 7)
        | (u64::from(b4 & 0xFE) >> 1))
}

/// Encode a 33-bit timestamp into the 5 guarded bytes (the test
/// vector factory; the inverse of [`decode_ts5`]).
#[cfg(test)]
fn encode_ts5(value: u64, prefix: u8) -> [u8; 5] {
    let v = value & 0x1_FFFF_FFFF;
    [
        (prefix << 4) | (((v >> 30) & 0x07) as u8) << 1 | 0x01,
        (v >> 22) as u8,
        ((((v >> 15) & 0x7F) << 1) as u8) | 0x01,
        (v >> 7) as u8,
        (((v & 0x7F) << 1) as u8) | 0x01,
    ]
}

/// The parsed PES header (§2.4.3.7) — everything before the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PesHeader {
    /// The elementary-stream id (Table 2-18).
    pub stream_id: u8,
    /// Its class.
    pub class: PesStreamClass,
    /// `PES_packet_length` as declared (16-bit; **0 = unbounded**).
    pub pes_packet_length: u16,
    /// `data_alignment_indicator` — the payload starts at an
    /// access-unit boundary (a decoder-side hint, surfaced honestly).
    pub data_alignment: bool,
    /// Presentation timestamp, 90 kHz ticks (33-bit reassembled).
    pub pts: Option<u64>,
    /// Decoding timestamp (present only with PTS, flags `11`).
    pub dts: Option<u64>,
    /// Total header bytes (start code + length + optional header):
    /// the payload begins at this offset inside the PES packet.
    pub header_len: usize,
    /// Total on-wire length this header implies (`6 + length`), or
    /// `None` for unbounded (video: until the next start).
    pub total_len: Option<usize>,
}

impl PesHeader {
    /// Parse the PES header from the start of `bytes`. The optional
    /// header must be complete in `bytes` (the assembler buffers
    /// until enough bytes exist — errors with code
    /// `demux.ts.pes.short-header` mean "hold" or "damage" per the
    /// caller's position in the stream).
    pub fn parse(bytes: &[u8]) -> Result<PesHeader, EngineError> {
        if bytes.len() < 6 || bytes[0..3] != [0x00, 0x00, 0x01] {
            return Err(bad_start_code());
        }
        let stream_id = bytes[3];
        let class = classify_stream_id(stream_id);
        let pes_packet_length = u16::from_be_bytes([bytes[4], bytes[5]]);
        let mut header = PesHeader {
            stream_id,
            class,
            pes_packet_length,
            data_alignment: false,
            pts: None,
            dts: None,
            header_len: 6,
            total_len: if pes_packet_length == 0 {
                None
            } else {
                Some(6 + usize::from(pes_packet_length))
            },
        };
        if !class.has_optional_header() {
            return Ok(header);
        }
        if bytes.len() < 9 {
            return Err(short_header());
        }
        let flags2 = bytes[7];
        let data_len = usize::from(bytes[8]);
        header.header_len = 9 + data_len;
        header.data_alignment = flags2 & 0x04 != 0;
        if bytes.len() < header.header_len {
            return Err(short_header());
        }
        match flags2 >> 6 {
            0b10 => {
                header.pts = Some(decode_ts5(&bytes[9..], 0b0010)?);
            }
            0b11 => {
                header.pts = Some(decode_ts5(&bytes[9..], 0b0011)?);
                header.dts = Some(decode_ts5(&bytes[14..], 0b0001)?);
            }
            // 0b01 is forbidden (Table 2-16 note): the field shape
            // cannot be trusted either way.
            0b01 => return Err(short_header()),
            _ => {}
        }
        // The optional header must not swallow the whole declared
        // packet: a header-only "elementary stream" PES is
        // structurally broken (stuffing belongs on padding_stream).
        if header
            .total_len
            .is_some_and(|total| header.header_len >= total)
        {
            return Err(short_header());
        }
        Ok(header)
    }
}

/// The MPEG time base every TS track carries (the demux plumbing
/// clock convention: ticks in the track's own base — the Matroska
/// precedent). Constant 1/90 000; the error arm is the honest
/// refused-panic shape, never `unwrap`.
pub fn mpeg_time_base() -> Result<TimeBase, EngineError> {
    TimeBase::new(1, MPEG_CLOCK_HZ).map_err(|_| time_base_unavailable())
}

/// A 90 kHz PTS/DTS in the demux plumbing's present-stamp form.
pub fn pts_stamp(ticks: u64) -> Result<Stamp, EngineError> {
    let base = mpeg_time_base()?;
    let ticks = i64::try_from(ticks).map_err(|_| bad_pts())?;
    Ok(Stamp::Present(Timestamp::new(ticks, base)))
}

/// One complete assembled PES packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PesPacket {
    /// The transport PID it was assembled from.
    pub pid: u16,
    /// The elementary-stream id.
    pub stream_id: u8,
    /// Its class.
    pub class: PesStreamClass,
    /// Presentation timestamp (90 kHz ticks).
    pub pts: Option<u64>,
    /// Decoding timestamp.
    pub dts: Option<u64>,
    /// `data_alignment_indicator`.
    pub data_alignment: bool,
    /// The transport `discontinuity_indicator` was seen at/before
    /// this PES start (a clock-reset boundary: timestamps before it
    /// and after it must not be arithmetically mixed).
    pub discontinuity: bool,
    /// On-wire length (header + payload) as assembled.
    pub pes_len: usize,
    /// The elementary-stream payload (after the PES header).
    pub payload: Vec<u8>,
}

impl PesPacket {
    /// The packet's PTS in the plumbing's stamp form (`Missing`
    /// when the PES carried none).
    pub fn presentation(&self) -> Result<Stamp, EngineError> {
        match self.pts {
            Some(t) => pts_stamp(t),
            None => Ok(Stamp::Missing),
        }
    }
}

/// Assembly damage, recovered-and-counted (the demuxer maps this to
/// `DemuxEvent::Recovered`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    /// Bytes dropped by the recovery.
    pub skipped: u64,
    /// Why (observability).
    pub reason: &'static str,
}

/// An honest timing event (never silent — the demuxer maps this to
/// `DemuxEvent::Discontinuity` / marks the packet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimingEvent {
    /// The transport `discontinuity_indicator` was set on `pid`.
    DiscontinuityIndicator { pid: u16 },
    /// DTS went backwards on `pid` (clock damage — B-frame reorder
    /// never touches DTS, so this can never be benign reorder).
    DtsRegressed { pid: u16, from: u64, to: u64 },
    /// DTS jumped forward past [`MAX_SANE_STEP_TICKS`] on `pid`.
    DtsJump { pid: u16, from: u64, to: u64 },
    /// PTS went backwards on a stream that has **never** carried a
    /// DTS (audio-style monotone streams; a PID that has emitted any
    /// DTS has declared reorder capability — its PTS-only packets
    /// are the muxer's "DTS derivable" omission, not damage, the
    /// measured 0091 pattern).
    PtsRegressedNoDts { pid: u16, from: u64, to: u64 },
    /// The program clock (PCR) went backwards without a
    /// discontinuity having been flagged since the last observation.
    PcrRegressed { from: u64, to: u64 },
}

/// Per-PID assembly state.
struct Assembly {
    buf: Vec<u8>,
    /// Discontinuity seen since the last PES start on this PID.
    disc: bool,
    /// Last emitted DTS (the regression/jump reference).
    last_dts: Option<u64>,
    /// Last emitted PTS (the no-DTS regression reference).
    last_pts: Option<u64>,
    /// True once any packet on this PID carried a DTS — the PID has
    /// declared reorder capability, so PTS-only packets that regress
    /// are benign muxer omission, not timing damage.
    seen_dts: bool,
    /// True once this PID's payload failed a start-code check — it
    /// is probably not a PES PID (PSI/DATA); stop accumulating until
    /// the next `payload_unit_start`.
    dormant: bool,
}

impl Assembly {
    fn new() -> Self {
        Assembly {
            buf: Vec::new(),
            disc: false,
            last_dts: None,
            last_pts: None,
            seen_dts: false,
            dormant: false,
        }
    }
}

/// The PID→PES assembly seam — the minimal transport contract the
/// Lead's I1 unification drives (I1a's PSI walker is file-disjoint).
pub struct PesAssembler {
    states: HashMap<u16, Assembly>,
    complete: VecDeque<PesPacket>,
    recovered: VecDeque<Recovered>,
    timing: VecDeque<TimingEvent>,
    /// The program-clock reference for PCR regression checks.
    last_pcr: Option<u64>,
    damage_bytes: u64,
}

impl PesAssembler {
    /// A fresh assembler (no PIDs known — all are learned from
    /// `push_packet`).
    pub fn new() -> Self {
        PesAssembler {
            states: HashMap::new(),
            complete: VecDeque::new(),
            recovered: VecDeque::new(),
            timing: VecDeque::new(),
            last_pcr: None,
            damage_bytes: 0,
        }
    }

    /// Feed one transport packet's PES payload for `pid`, verbatim
    /// (§2.4.4.2: PES uses no `pointer_field` — that is a PSI-only
    /// construct; the payload commences with the PES start code
    /// when `start` is set). `start` is
    /// `payload_unit_start_indicator`; `discontinuity` is the
    /// adaptation field's `discontinuity_indicator` (a clock-reset
    /// boundary).
    ///
    /// # Errors
    /// The typed memory-ceiling refusal ([`MAX_PES_BYTES`]) only;
    /// every recoverable damage is a [`Recovered`] record instead.
    pub fn push_packet(
        &mut self,
        pid: u16,
        start: bool,
        discontinuity: bool,
        payload: &[u8],
    ) -> Result<(), EngineError> {
        if discontinuity {
            self.timing
                .push_back(TimingEvent::DiscontinuityIndicator { pid });
        }
        let had_pending = {
            let state = self.states.entry(pid).or_insert_with(Assembly::new);
            if start {
                state.dormant = false;
                state.disc = discontinuity;
                !state.buf.is_empty()
            } else {
                false
            }
        };
        let dormant = self.states.get(&pid).map(|s| s.dormant).unwrap_or(false);
        if had_pending {
            self.complete_one(pid);
        }
        if dormant {
            // Not a PES PID (or the current start is dead): the
            // payload is someone else's — counted, not accumulated.
            self.damage_bytes += payload.len() as u64;
            return Ok(());
        }
        let Some(state) = self.states.get_mut(&pid) else {
            return Ok(());
        };
        if state.buf.is_empty() && !start {
            // Continuation before any start on this PID.
            self.damage_bytes += payload.len() as u64;
            self.recovered.push_back(Recovered {
                skipped: payload.len() as u64,
                reason: "pes.stray-continuation",
            });
            return Ok(());
        }
        if let Some(attempted) = state
            .buf
            .len()
            .checked_add(payload.len())
            .filter(|&total| total > MAX_PES_BYTES)
            .map(|total| total as u64)
        {
            state.buf.clear();
            self.damage_bytes += attempted;
            return Err(payload_over_ceiling(MAX_PES_BYTES as u64, attempted));
        }
        state.buf.extend_from_slice(payload);
        if discontinuity {
            state.disc = true;
        }
        self.try_complete(pid);
        Ok(())
    }

    /// Observe one PCR (adaptation-field values, transport-layer
    /// parse): 33-bit `base` (90 kHz) + `extension` (27 MHz, 9
    /// bits). A PCR regression without a flagged discontinuity is a
    /// [`TimingEvent::PcrRegressed`]; a flagged discontinuity resets
    /// the reference (the indicator's own event already fired on the
    /// packet that carried it — no duplicate here).
    pub fn observe_pcr(&mut self, base: u64, extension: u16, discontinuity: bool) {
        let Some(pcr) = base
            .checked_mul(300)
            .and_then(|b| b.checked_add(u64::from(extension & 0x1FF)))
        else {
            return;
        };
        if discontinuity {
            // A flagged discontinuity invalidates the clock
            // reference: the next PCR establishes a fresh one
            // (comparing across the reset would fabricate a
            // regression that is really a new timeline).
            self.last_pcr = None;
            return;
        }
        if let Some(prev) = self.last_pcr.replace(pcr).filter(|prev| pcr < *prev) {
            self.timing.push_back(TimingEvent::PcrRegressed {
                from: prev,
                to: pcr,
            });
        }
    }

    /// Complete the buffered PES on `pid` now (an unbounded packet
    /// ended by the next start, a bounded one reached its declared
    /// length, or the EOS flush).
    fn complete_one(&mut self, pid: u16) {
        let Some(state) = self.states.get_mut(&pid) else {
            return;
        };
        let buf = std::mem::take(&mut state.buf);
        let disc = std::mem::replace(&mut state.disc, false);
        match PesHeader::parse(&buf) {
            Ok(header) => {
                if header.class == PesStreamClass::Padding {
                    // Stuffing bytes are not a packet.
                    self.damage_bytes += buf.len() as u64;
                    return;
                }
                if let (Some(pts), None) = (header.pts, header.dts) {
                    if let Some(event) = state
                        .last_pts
                        .filter(|&prev| pts < prev && !state.seen_dts)
                        .map(|prev| TimingEvent::PtsRegressedNoDts {
                            pid,
                            from: prev,
                            to: pts,
                        })
                    {
                        self.timing.push_back(event);
                    }
                    state.last_pts = Some(pts);
                }
                if let Some(dts) = header.dts {
                    state.seen_dts = true;
                    if let Some(prev) = state.last_dts.filter(|&prev| dts < prev) {
                        self.timing.push_back(TimingEvent::DtsRegressed {
                            pid,
                            from: prev,
                            to: dts,
                        });
                    } else if let Some(from) = state
                        .last_dts
                        .filter(|&prev| dts.saturating_sub(prev) > MAX_SANE_STEP_TICKS)
                    {
                        self.timing
                            .push_back(TimingEvent::DtsJump { pid, from, to: dts });
                    }
                    state.last_dts = Some(dts);
                }
                let payload = buf
                    .get(header.header_len..)
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default();
                self.complete.push_back(PesPacket {
                    pid,
                    stream_id: header.stream_id,
                    class: header.class,
                    pts: header.pts,
                    dts: header.dts,
                    data_alignment: header.data_alignment,
                    discontinuity: disc,
                    pes_len: buf.len(),
                    payload,
                });
            }
            Err(err) => {
                // Mid-stream header damage: dropped and counted (the
                // sticky truncation is reserved for the EOS flush).
                self.damage_bytes += buf.len() as u64;
                self.recovered.push_back(Recovered {
                    skipped: buf.len() as u64,
                    reason: if err.context.code == "demux.ts.pes.bad-start-code" {
                        "pes.bad-start-code"
                    } else {
                        "pes.header-damage"
                    },
                });
            }
        }
    }

    /// Check the buffered PES on `pid` for bounded completion.
    fn try_complete(&mut self, pid: u16) {
        let total = {
            let Some(state) = self.states.get(&pid) else {
                return;
            };
            if state.buf.len() < 6 {
                return;
            }
            match PesHeader::parse(&state.buf) {
                Ok(header) => match header.total_len {
                    Some(total) => total,
                    // Unbounded (length 0, video): waits for the
                    // next start or the flush.
                    None => return,
                },
                Err(err) => {
                    // A start-code failure on a fresh start means
                    // this is not a PES PID (PSI/DATA tables start
                    // with their table_id, never 00 00 01): dormancy
                    // keeps the walk cheap. Other header errors may
                    // still be a too-short buffer — hold it.
                    if let Some(state) = self
                        .states
                        .get_mut(&pid)
                        .filter(|_| err.context.code == "demux.ts.pes.bad-start-code")
                    {
                        self.damage_bytes += state.buf.len() as u64;
                        state.buf.clear();
                        state.dormant = true;
                    }
                    return;
                }
            }
        };
        let have = self
            .states
            .get(&pid)
            .map(|s| s.buf.len())
            .unwrap_or_default();
        if have < total {
            return;
        }
        if have > total {
            // A bounded packet overran its declared length: the
            // extra bytes are damage (no start code announced them).
            let skipped = (have - total) as u64;
            self.damage_bytes += skipped;
            self.recovered.push_back(Recovered {
                skipped,
                reason: "pes.length-overrun",
            });
            if let Some(state) = self.states.get_mut(&pid) {
                state.buf.truncate(total);
            }
        }
        self.complete_one(pid);
    }

    /// The end-of-stream flush: emit every trailing PES honestly —
    /// unbounded (video) buffers become final packets (ffmpeg's own
    /// last-packet behavior); a bounded buffer short of its declared
    /// length is the typed sticky truncation.
    ///
    /// # Errors
    /// [`truncated`] for a short bounded PES; nothing else.
    pub fn finish(&mut self) -> Result<(), EngineError> {
        let pids: Vec<u16> = self.states.keys().copied().collect();
        for pid in pids {
            let short = {
                let Some(state) = self.states.get_mut(&pid) else {
                    continue;
                };
                if state.buf.is_empty() {
                    continue;
                }
                match PesHeader::parse(&state.buf) {
                    Ok(header) => match header.total_len {
                        Some(total) if state.buf.len() < total => {
                            Some((total as u64, state.buf.len() as u64))
                        }
                        Some(total) if state.buf.len() > total => {
                            state.buf.truncate(total);
                            None
                        }
                        _ => None,
                    },
                    // Header damage: the counted drop (complete_one).
                    Err(_) => None,
                }
            };
            if let Some((expected, found)) = short {
                return Err(truncated(expected, found));
            }
            self.complete_one(pid);
        }
        Ok(())
    }

    /// Take the next completed PES packet, if any.
    pub fn next_pes(&mut self) -> Option<PesPacket> {
        self.complete.pop_front()
    }

    /// Drain the recovered-damage records (the `Recovered` event
    /// shape).
    pub fn take_recovered(&mut self) -> Vec<Recovered> {
        self.recovered.drain(..).collect()
    }

    /// Drain the honest timing events (the `Discontinuity`
    /// mapping — never silent).
    pub fn take_timing_events(&mut self) -> Vec<TimingEvent> {
        self.timing.drain(..).collect()
    }

    /// Total bytes dropped by recovery so far.
    pub fn damage_bytes(&self) -> u64 {
        self.damage_bytes
    }
}

impl Default for PesAssembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The typed-error code of a result ("" when `Ok`) — the
    /// lint-legal `unwrap_err` replacement for the unit suite.
    fn err_code<T>(result: Result<T, EngineError>) -> String {
        match result {
            Err(err) => err.context.code.to_string(),
            Ok(_) => String::new(),
        }
    }

    /// A video PES (stream_id 0xE0, unbounded) with optional stamps.
    fn video_pes(pts: Option<u64>, dts: Option<u64>, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0x00, 0x00, 0x01, 0xE0, 0x00, 0x00];
        let mut flags2 = 0x04u8; // data_alignment_indicator set
        let mut opt = Vec::new();
        match (pts, dts) {
            (Some(p), Some(d)) => {
                flags2 |= 0xC0;
                opt.extend_from_slice(&encode_ts5(p, 0b0011));
                opt.extend_from_slice(&encode_ts5(d, 0b0001));
            }
            (Some(p), None) => {
                flags2 |= 0x80;
                opt.extend_from_slice(&encode_ts5(p, 0b0010));
            }
            _ => {}
        }
        out.push(0x80); // flags1: '10' + no ESCR/ES_rate
        out.push(flags2);
        out.push(opt.len() as u8);
        out.extend_from_slice(&opt);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn stream_id_classification_table() {
        let cases = [
            (0xE0u8, PesStreamClass::Video),
            (0xEF, PesStreamClass::Video),
            (0xC0, PesStreamClass::Audio),
            (0xDF, PesStreamClass::Audio),
            (0xBD, PesStreamClass::Private1),
            (0xBE, PesStreamClass::Padding),
            (0xBF, PesStreamClass::Private2),
            (0xBC, PesStreamClass::StreamMap),
            (0xFF, PesStreamClass::StreamMap),
            (0xF0, PesStreamClass::Entitlement),
            (0xF1, PesStreamClass::Entitlement),
            (0xF2, PesStreamClass::Interactive),
            (0xFE, PesStreamClass::Reserved),
            (0xF4, PesStreamClass::Reserved),
        ];
        for (id, class) in cases {
            assert_eq!(classify_stream_id(id), class, "stream_id {id:#04X}");
        }
    }

    #[test]
    fn ts5_roundtrip_and_marker_guard() {
        for value in [0u64, 1, 126_000, 1 << 32, (1 << 33) - 1] {
            let enc = encode_ts5(value, 0b0010);
            assert_eq!(decode_ts5(&enc, 0b0010).must(), value);
        }
        let mut bad = encode_ts5(126_000, 0b0010);
        bad[2] &= 0xFE; // clear the middle marker
        assert_eq!(err_code(decode_ts5(&bad, 0b0010)), "demux.ts.pes.bad-pts");
        // Wrong prefix (a PTS-shape field where a DTS prefix is law).
        assert_eq!(
            err_code(decode_ts5(&encode_ts5(5, 0b0010), 0b0001)),
            "demux.ts.pes.bad-pts"
        );
        // Too few bytes for the guarded field at all.
        assert_eq!(
            err_code(decode_ts5(&[0x21, 0x00, 0x01], 0b0010)),
            "demux.ts.pes.short-header"
        );
    }

    #[test]
    fn unbounded_video_assembles_across_packets() {
        let full = video_pes(Some(129_000), Some(126_000), &[0x11; 700]);
        let mut asm = PesAssembler::new();
        // Split mid-payload across three transport payloads (the
        // second split point is inside the payload, after the
        // complete header — the hold-and-wait path).
        asm.push_packet(0x100, true, false, &full[..300]).must();
        assert!(asm.next_pes().is_none(), "unbounded: not complete yet");
        asm.push_packet(0x100, false, false, &full[300..600]).must();
        assert!(asm.next_pes().is_none());
        asm.push_packet(0x100, false, false, &full[600..]).must();
        // Unbounded: completes only at the next start.
        let next = video_pes(Some(132_000), Some(127_500), &[0x22; 4]);
        asm.push_packet(0x100, true, false, &next).must();
        let pkt = asm.next_pes().must();
        assert_eq!(pkt.stream_id, 0xE0);
        assert_eq!(pkt.class, PesStreamClass::Video);
        assert_eq!(pkt.pts, Some(129_000));
        assert_eq!(pkt.dts, Some(126_000));
        assert!(pkt.data_alignment);
        assert!(!pkt.discontinuity);
        assert_eq!(pkt.pes_len, full.len());
        assert_eq!(pkt.payload, vec![0x11; 700]);
        // The flush emits the final unbounded packet.
        asm.finish().must();
        let last = asm.next_pes().must();
        assert_eq!(last.pts, Some(132_000));
        assert_eq!(last.payload, vec![0x22; 4]);
        assert!(asm.next_pes().is_none());
        assert!(asm.take_recovered().is_empty());
        assert!(asm.take_timing_events().is_empty());
    }

    #[test]
    fn bounded_audio_completes_at_declared_length() {
        // PES_packet_length counts everything after the first 6
        // bytes: optional header (3) + payload (3) = 6.
        let mut pes = vec![0x00, 0x00, 0x01, 0xC1, 0x00, 0x06, 0x80, 0x00, 0x00];
        pes.extend_from_slice(&[1, 2, 3]);
        let mut asm = PesAssembler::new();
        asm.push_packet(0x101, true, false, &pes).must();
        let pkt = asm.next_pes().must();
        assert_eq!(pkt.class, PesStreamClass::Audio);
        assert_eq!(pkt.payload, vec![1, 2, 3]);
        assert_eq!(pkt.pes_len, 9 + 3);
        assert_eq!(pkt.pts, None);
        assert_eq!(pkt.dts, None);
        assert!(asm.take_timing_events().is_empty());
    }

    #[test]
    fn tiny_bounded_pes_without_optional_header_completes() {
        // Private stream 2 (0xBF): no optional header, length 1 —
        // total 7 bytes, below the 9-byte optional-header minimum.
        let pes = [0x00, 0x00, 0x01, 0xBF, 0x00, 0x01, 0x77];
        let mut asm = PesAssembler::new();
        asm.push_packet(0x104, true, false, &pes[..4]).must();
        assert!(asm.next_pes().is_none());
        asm.push_packet(0x104, false, false, &pes[4..]).must();
        let pkt = asm.next_pes().must();
        assert_eq!(pkt.class, PesStreamClass::Private2);
        assert_eq!(pkt.payload, vec![0x77]);
    }

    #[test]
    fn length_overrun_is_recovered_damage() {
        // Declared 3+3 bytes, but the same transport payload carries
        // 4 extra bytes no start code announced.
        let mut pes = vec![0x00, 0x00, 0x01, 0xC1, 0x00, 0x06, 0x80, 0x00, 0x00];
        pes.extend_from_slice(&[1, 2, 3]);
        pes.extend_from_slice(&[9, 9, 9, 9]);
        let mut asm = PesAssembler::new();
        asm.push_packet(0x101, true, false, &pes).must();
        let pkt = asm.next_pes().must();
        assert_eq!(pkt.payload, vec![1, 2, 3]);
        let rec = asm.take_recovered();
        assert_eq!(rec.len(), 1);
        assert_eq!(rec[0].reason, "pes.length-overrun");
        assert_eq!(rec[0].skipped, 4);
    }

    #[test]
    fn stray_continuation_before_any_start_is_counted() {
        let mut asm = PesAssembler::new();
        asm.push_packet(0x100, false, false, &[0xAB; 10]).must();
        assert_eq!(asm.damage_bytes(), 10);
        assert_eq!(asm.take_recovered()[0].reason, "pes.stray-continuation");
    }

    #[test]
    fn non_pes_pid_goes_dormant_instead_of_accumulating() {
        let mut asm = PesAssembler::new();
        // PAT-shaped: starts with table_id 0x00, never 00 00 01.
        asm.push_packet(0x0000, true, false, &[0x00, 0xB0, 0x0D])
            .must();
        asm.push_packet(0x0000, false, false, &[0x11; 180]).must();
        assert_eq!(asm.damage_bytes(), 3 + 180);
        assert!(asm.next_pes().is_none());
        assert!(asm.take_recovered().is_empty(), "dormancy, not recovery");
    }

    #[test]
    fn truncated_bounded_pes_at_eos_is_typed() {
        let mut pes = vec![0x00, 0x00, 0x01, 0xC1, 0x00, 0x20, 0x00, 0x00, 0x00];
        pes.extend_from_slice(&[1, 2, 3]); // 3 of 32 payload bytes
        let mut asm = PesAssembler::new();
        asm.push_packet(0x101, true, false, &pes).must();
        let err = match asm.finish() {
            Err(err) => err,
            Ok(()) => unreachable!("the short bounded PES must refuse"),
        };
        assert_eq!(err.context.code, "demux.ts.pes.truncated");
        assert!(matches!(
            err.kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: 38,
                    found: 12
                }
            }
        ));
    }

    #[test]
    fn bad_pts_marker_is_typed_damage_midstream() {
        let mut pes = vec![0x00, 0x00, 0x01, 0xE0, 0x00, 0x00, 0x80, 0x80, 0x05];
        pes.extend_from_slice(&encode_ts5(126_000, 0b0010));
        pes[11] &= 0xFE; // clear the middle marker byte of the PTS field
        pes.extend_from_slice(&[0x55; 4]);
        let mut asm = PesAssembler::new();
        asm.push_packet(0x100, true, false, &pes).must();
        let next = video_pes(Some(131_000), None, &[0x00; 2]);
        asm.push_packet(0x100, true, false, &next).must();
        asm.finish().must(); // the good PES completes at the flush
        // Only the good PES survives, and the damage is recorded.
        let pkt = asm.next_pes().must();
        assert_eq!(pkt.pts, Some(131_000));
        assert_eq!(asm.take_recovered()[0].reason, "pes.header-damage");
        assert!(asm.damage_bytes() > 0);
    }

    #[test]
    fn forbidden_pts_dts_flags_refuse_typed() {
        let mut pes = vec![0x00, 0x00, 0x01, 0xE0, 0x00, 0x00, 0x80, 0x40, 0x00];
        pes.extend_from_slice(&[0x55; 4]);
        assert_eq!(
            err_code(PesHeader::parse(&pes)),
            "demux.ts.pes.short-header"
        );
    }

    #[test]
    fn header_swallowing_the_declared_packet_is_refused() {
        // Optional header of 10 bytes inside a declared 12-byte PES
        // (6 + 6): nothing payload-shaped remains.
        let mut pes = vec![0x00, 0x00, 0x01, 0xE0, 0x00, 0x06, 0x80, 0x00, 0x0A];
        pes.extend_from_slice(&[0x00; 12]);
        assert_eq!(
            err_code(PesHeader::parse(&pes)),
            "demux.ts.pes.short-header"
        );
    }

    #[test]
    fn discontinuity_surfaces_as_event_and_packet_flag() {
        let first = video_pes(Some(1_000), None, &[0x01; 8]);
        let second = video_pes(Some(1_500), None, &[0x02; 8]);
        let mut asm = PesAssembler::new();
        asm.push_packet(0x100, true, false, &first).must();
        asm.push_packet(0x100, true, true, &second).must();
        assert_eq!(
            asm.take_timing_events(),
            vec![TimingEvent::DiscontinuityIndicator { pid: 0x100 }]
        );
        let _ = asm.next_pes().must(); // the pre-boundary packet
        asm.finish().must();
        let flagged = asm.next_pes().must();
        assert!(flagged.discontinuity);
    }

    #[test]
    fn dts_regression_and_jump_are_events() {
        let a = video_pes(Some(5_000), Some(10_000), &[0x01; 4]);
        let b = video_pes(Some(6_000), Some(9_000), &[0x02; 4]); // DTS back
        let c = video_pes(Some(7_000), Some(9_000 + 11 * 90_000), &[0x03; 4]); // > 10 s
        let mut asm = PesAssembler::new();
        asm.push_packet(0x100, true, false, &a).must();
        asm.push_packet(0x100, true, false, &b).must();
        asm.push_packet(0x100, true, false, &c).must();
        asm.finish().must(); // the last PES completes at the flush
        assert_eq!(
            asm.take_timing_events(),
            vec![
                TimingEvent::DtsRegressed {
                    pid: 0x100,
                    from: 10_000,
                    to: 9_000
                },
                TimingEvent::DtsJump {
                    pid: 0x100,
                    from: 9_000,
                    to: 9_000 + 11 * 90_000
                },
            ]
        );
    }

    #[test]
    fn pts_regression_without_dts_is_an_event_bframe_pts_with_dts_is_not() {
        let mut asm = PesAssembler::new();
        // Video with DTS: PTS 5000 then 3000 (reorder) — no event.
        let r1 = video_pes(Some(5_000), Some(1_000), &[0x01; 4]);
        let r2 = video_pes(Some(3_000), Some(2_000), &[0x02; 4]);
        asm.push_packet(0x100, true, false, &r1).must();
        asm.push_packet(0x100, true, false, &r2).must();
        assert!(asm.take_timing_events().is_empty());
        // The same PID then omits DTS (the muxer's "derivable"
        // omission) and regresses PTS — still no event: the PID has
        // declared reorder capability (the measured 0091 pattern).
        let r3 = video_pes(Some(2_500), None, &[0x03; 4]);
        asm.push_packet(0x100, true, false, &r3).must();
        asm.finish().must();
        assert!(asm.take_timing_events().is_empty());
        // Audio without DTS ever: PTS regression — event.
        let audio = |pts: u64| {
            let mut out = vec![0x00, 0x00, 0x01, 0xC0, 0x00, 0x0D, 0x80, 0x80, 0x05];
            out.extend_from_slice(&encode_ts5(pts, 0b0010));
            out.extend_from_slice(&[0x09; 5]);
            out
        };
        asm.push_packet(0x101, true, false, &audio(2_000)).must();
        asm.push_packet(0x101, true, false, &audio(1_000)).must();
        assert_eq!(
            asm.take_timing_events(),
            vec![TimingEvent::PtsRegressedNoDts {
                pid: 0x101,
                from: 2_000,
                to: 1_000
            }]
        );
    }

    #[test]
    fn pcr_regression_is_an_event_and_discontinuity_resets() {
        let mut asm = PesAssembler::new();
        asm.observe_pcr(100_000, 0, false);
        asm.observe_pcr(99_000, 0, false);
        assert_eq!(
            asm.take_timing_events(),
            vec![TimingEvent::PcrRegressed {
                from: 100_000 * 300,
                to: 99_000 * 300
            }]
        );
        // A flagged discontinuity then a lower PCR: reference reset,
        // no spurious regression (the indicator's own event fired on
        // the packet that carried the flag).
        asm.observe_pcr(99_000, 0, true);
        asm.observe_pcr(5_000, 0, false);
        assert!(asm.take_timing_events().is_empty());
    }

    #[test]
    fn payload_ceiling_is_the_typed_refusal() {
        let pes = [0x00, 0x00, 0x01, 0xE0, 0x00, 0x00, 0x80, 0x00, 0x00];
        let mut asm = PesAssembler::new();
        asm.push_packet(0x100, true, false, &pes).must();
        let mut err = None;
        // Feed one-megabyte continuations until the refusal fires.
        for _ in 0..(MAX_PES_BYTES / (1024 * 1024) + 2) {
            if let Err(e) = asm.push_packet(0x100, false, false, &[0xAA; 1024 * 1024]) {
                err = Some(e);
                break;
            }
        }
        let err = err.must();
        assert_eq!(err.context.code, "demux.ts.pes.payload-over-ceiling");
        assert!(matches!(
            err.kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::Memory,
                ..
            }
        ));
    }

    #[test]
    fn pts_stamp_maps_to_the_ninety_kilohertz_base() {
        let ts = match pts_stamp(126_000).must() {
            Stamp::Present(ts) => ts,
            Stamp::Missing => unreachable!("126_000 ticks is a present stamp"),
        };
        assert_eq!(ts.base.as_parts(), (1, MPEG_CLOCK_HZ));
        assert_eq!(ts.ticks, 126_000);
        assert_eq!(ts.as_nanos().must().0, 126_000 * 1_000_000_000 / 90_000);
        // The packet-side helper agrees for both stamp shapes.
        let bare = PesPacket {
            pid: 0,
            stream_id: 0xE0,
            class: PesStreamClass::Video,
            pts: None,
            dts: None,
            data_alignment: false,
            discontinuity: false,
            pes_len: 0,
            payload: Vec::new(),
        };
        assert_eq!(bare.presentation().must(), Stamp::Missing);
    }

    #[test]
    fn error_kinds_are_stable_and_staged() {
        let cases = [
            (bad_start_code(), "demux.ts.pes.bad-start-code"),
            (short_header(), "demux.ts.pes.short-header"),
            (bad_pts(), "demux.ts.pes.bad-pts"),
            (truncated(10, 4), "demux.ts.pes.truncated"),
            (
                payload_over_ceiling(10, 20),
                "demux.ts.pes.payload-over-ceiling",
            ),
            (time_base_unavailable(), "demux.ts.pes.time-base"),
        ];
        for (err, code) in cases {
            assert_eq!(err.stage, STAGE, "{code}");
            assert_eq!(err.context.code, code);
            assert!(err.to_string().starts_with("[demux] "), "{code}");
        }
    }
}
