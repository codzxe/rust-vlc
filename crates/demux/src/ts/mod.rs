//! The MPEG-TS demuxer's packet layer + PSI plumbing (card 5.I1a —
//! this half; the PES half is a parallel card's `pes.rs`, plugged in
//! at the [`PesConsumer`] seam below).
//!
//! # What this is
//!
//! [`TsDemuxer`] implements the landed
//! [`Demuxer`](plugin_api::Demuxer) contract over any seekable
//! [`SourceStream`](plugin_api::SourceStream) for the ISO/IEC 13818-1
//! transport stream: 188-byte packet walk with bounded resync, the
//! PSI/SI tables ([`psi`]: PAT/PMT/SDT, CAT skipped-with-note), the
//! multi-program track surface, and a per-PES-PID delivery seam for
//! the packetized-elementary-stream half.
//!
//! # Packet layer (ISO/IEC 13818-1 §2.4.3)
//!
//! Sync `0x47`, TEI, PUSI, transport priority, 13-bit PID, transport
//! scrambling control, adaptation-field control, 4-bit continuity
//! counter. The adaptation field is surfaced with its flags and the
//! **raw** PCR (33-bit base + 9-bit extension, no interpolation —
//! the pacing math is the engine's, not the demuxer's).
//!
//! **Continuity policy: tolerant + damage-noted.** A CC that repeats
//! is a tolerated duplicate (counted); a CC that jumps resets the
//! expectation, is counted, and raises one
//! [`Discontinuity`](DemuxEvent::Discontinuity) — broadcast streams
//! splice and PID-tunnel; the demuxer reports, it does not refuse.
//! The adaptation-field discontinuity indicator behaves the same.
//!
//! # PSI collection (`start`)
//!
//! `start` walks a bounded head window ([`MAX_PSI_SCAN_BYTES`]) until
//! the PAT, every PAT-declared PMT and the SDT have been seen (the
//! broadcast repetition interval is sub-second; the corpus carries
//! all three in the first three packets). Missing SDT is legal (names
//! stay `None`); a missing PMT leaves its program trackless with a
//! damage note; **no PAT at all** is the typed
//! `demux.ts.no-pat` refusal. The walk then rewinds to the head so
//! the PES half sees the whole stream.
//!
//! # The PES seam (the parallel half's plug point)
//!
//! `next_event` walks every packet; PES payloads on
//! program-declared elementary PIDs are handed to the registered
//! [`PesConsumer`] (PUSI + the packet's payload slice, TS header and
//! adaptation field already stripped) — until that half lands, the
//! default consumer counts packets and bytes. **No fake packets are
//! ever emitted.**
//!
//! # Honesty notes (v0 scope)
//!
//! - Seeking: TS has no container index, and keyframe-truth needs the
//!   PES half's timestamps — [`SeekSupport::None`] and a typed
//!   refusal, never a fake landing.
//! - Scrambled packets are delivered as-is (descrambling is a CA
//!   module's job, out of engine scope) and counted.
//! - PIDs outside the PSI/ES sets are counted, not guessed at.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::ids::{CodecDescriptor, LanguageTag, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::time::{TimeBase, TimeSpan, TrackTiming};
use media_types::{EngineError, ErrorKind, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::header::DamageLog;
use crate::avi::riff::SourceReader;
use crate::ts::psi::{EsEntry, MAX_SECTION_BYTES, PsiSection, TID_CAT, parse_section};

pub mod psi;
pub mod test_support;

#[cfg(test)]
mod tests;

/// One TS packet: 188 bytes, sync byte first (ISO/IEC 13818-1 §2.4.3).
pub const PACKET_BYTES: usize = 188;

/// The head window `start` walks hunting the PSI tables.
pub const MAX_PSI_SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// The bounded per-event resync scan (a sync loss is recovered within
/// a packet or two in practice; this is the ceiling, never a loop).
pub const MAX_RESYNC_SCAN_BYTES: usize = 2048;

/// The head window the open probe scans for the first sync byte.
pub const MAX_OPEN_SCAN_BYTES: u64 = 256 * 1024;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["ts", "m2t", "mts"];

/// The well-known PID assignments (§2.4.4.3 Table 2-3).
pub const PID_PAT: u16 = 0x0000;
pub const PID_CAT: u16 = 0x0001;
pub const PID_NIT: u16 = 0x0010;
pub const PID_SDT_BAT: u16 = 0x0011;
/// The null packet PID (stuffing — counted, never delivered).
pub const PID_NULL: u16 = 0x1FFF;

// ---------------------------------------------------------------------------
// Typed errors (stage `Demux`, the crate's stage)
// ---------------------------------------------------------------------------

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Demux,
        kind,
        ErrorContext::new(code),
    )
}

/// Not an MPEG-TS stream (no 188-stride `0x47` sync anywhere in the
/// open-scan window).
pub fn not_ts(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("mpegts".to_string()),
                sniffed: sniffed[..sniffed.len().min(16)].to_vec(),
            }),
        },
        "demux.ts.not-ts",
    )
}

/// The bounded PSI scan found no Program Association Table — a TS
/// without a PAT has no program surface at all.
pub fn no_pat() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.ts.no-pat",
    )
}

/// A packet header is structurally impossible (adaptation field
/// overruns the packet).
pub fn bad_packet(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ts.bad-packet",
    )
}

/// The bounded resync scan failed (no `0x47` stride within
/// [`MAX_RESYNC_SCAN_BYTES`]).
pub fn resync_failed(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ts.resync-failed",
    )
}

/// `next_event` before `start` (operational shape, not container
/// data — the WAV/Ogg precedent).
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.ts.not-started",
    )
}

// ---------------------------------------------------------------------------
// The packet layer
// ---------------------------------------------------------------------------

/// The raw program clock reference (33-bit base + 9-bit extension,
/// §2.4.4.9) — exposed exactly as the stream carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcr {
    /// `program_clock_reference_base` (90 kHz units).
    pub base: u64,
    /// `program_clock_reference_extension` (27 MHz units).
    pub extension: u16,
}

/// The parsed adaptation field (§2.4.3.5) — the flags the demux layer
/// consumes; unhandled fields are skipped positionally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptationField {
    /// The declared field length (bytes after the length byte).
    pub length: u8,
    /// `discontinuity_indicator`.
    pub discontinuity: bool,
    /// `random_access_indicator` (a stream join point).
    pub random_access: bool,
    /// `elementary_stream_priority_indicator`.
    pub priority: bool,
    /// `OPCR_flag` (the flag as carried; the value is not consumed).
    pub opcr_flag: bool,
    /// `splice_countdown_flag` (the flag as carried).
    pub splice_countdown_flag: bool,
    /// `transport_private_data_flag` (the flag as carried).
    pub private_data_flag: bool,
    /// The raw PCR when `PCR_flag` was set and the six bytes parse.
    pub pcr: Option<Pcr>,
}

/// One parsed transport-stream packet (§2.4.3). `payload` is the
/// packet's bytes after the 4-byte header and any adaptation field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TsPacket<'a> {
    /// `transport_error_indicator` — the packet is declared corrupt.
    pub transport_error: bool,
    /// `payload_unit_start_indicator`.
    pub payload_unit_start: bool,
    /// `transport_priority`.
    pub transport_priority: bool,
    /// The 13-bit PID.
    pub pid: u16,
    /// `transport_scrambling_control` (2 bits, raw).
    pub scrambling: u8,
    /// `adaptation_field_control` (2 bits, raw: 1 = payload,
    /// 2 = adaptation only, 3 = both).
    pub adaptation_field_control: u8,
    /// `continuity_counter` (4 bits, raw).
    pub continuity_counter: u8,
    /// The parsed adaptation field when present.
    pub adaptation: Option<AdaptationField>,
    /// The payload bytes (empty for adaptation-only packets).
    pub payload: &'a [u8],
}

impl TsPacket<'_> {
    /// Parse one 188-byte packet. The sync byte must already be
    /// `0x47` (the walk's invariant; the demuxer's resync owns sync
    /// loss).
    ///
    /// # Errors
    /// [`bad_packet`] when the adaptation field's declared length
    /// overruns the packet.
    pub fn parse(bytes: &[u8]) -> Result<TsPacket<'_>, EngineError> {
        let h = bytes;
        let pid = (u16::from(h[1] & 0x1F) << 8) | u16::from(h[2]);
        let afc = h[3] >> 4 & 0x03;
        let mut at = 4;
        let mut adaptation = None;
        if afc & 0b10 != 0 {
            let length = h[4];
            let end = 5 + usize::from(length);
            if end > PACKET_BYTES {
                return Err(bad_packet(0));
            }
            let f = &h[5..end];
            let discontinuity = !f.is_empty() && f[0] & 0x80 != 0;
            let random_access = !f.is_empty() && f[0] & 0x40 != 0;
            let priority = !f.is_empty() && f[0] & 0x20 != 0;
            let pcr_flag = !f.is_empty() && f[0] & 0x10 != 0;
            let opcr_flag = !f.is_empty() && f[0] & 0x08 != 0;
            let splice_countdown_flag = !f.is_empty() && f[0] & 0x04 != 0;
            let private_data_flag = !f.is_empty() && f[0] & 0x02 != 0;
            let mut pcr = None;
            if pcr_flag && f.len() >= 7 {
                let base = u64::from(f[1]) << 25
                    | u64::from(f[2]) << 17
                    | u64::from(f[3]) << 9
                    | u64::from(f[4]) << 1
                    | u64::from(f[5] >> 7);
                let extension = (u16::from(f[5] & 0x01) << 8) | u16::from(f[6]);
                pcr = Some(Pcr { base, extension });
            }
            adaptation = Some(AdaptationField {
                length,
                discontinuity,
                random_access,
                priority,
                opcr_flag,
                splice_countdown_flag,
                private_data_flag,
                pcr,
            });
            at = end;
        }
        let payload = if afc & 0b01 != 0 {
            &h[at..]
        } else {
            &h[at..at]
        };
        Ok(TsPacket {
            transport_error: h[1] & 0x80 != 0,
            payload_unit_start: h[1] & 0x40 != 0,
            transport_priority: h[1] & 0x20 != 0,
            pid,
            scrambling: h[3] >> 6,
            adaptation_field_control: afc,
            continuity_counter: h[3] & 0x0F,
            adaptation,
            payload,
        })
    }

    /// Whether the packet carries payload (AFC bit 0).
    #[must_use]
    pub fn has_payload(&self) -> bool {
        self.adaptation_field_control & 0b01 != 0
    }
}

/// The probe: do the leading bytes look like a 188-stride `0x47`
/// transport stream?
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    if header.is_empty() || header[0] != 0x47 {
        return false;
    }
    // A lone sync byte proves nothing; a second at the stride does
    // (a whole 188-byte file is also accepted — the walk will tell).
    header.len() < PACKET_BYTES || header[PACKET_BYTES] == 0x47
}

// ---------------------------------------------------------------------------
// The multi-program surface
// ---------------------------------------------------------------------------

/// One elementary stream of a program (the PMT's ES entry + the
/// demuxer's registry mapping).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsStream {
    /// The PID carrying the stream.
    pub pid: u16,
    /// The raw PMT `stream_type` byte.
    pub stream_type: u8,
    /// The ISO 639 language when the PMT's 0x0A descriptor declared
    /// one (parsed, validated; `None` = undeclared).
    pub language: Option<LanguageTag>,
    /// The verbatim ES descriptor bytes (registration, ... — kept for
    /// the decode seam).
    pub es_info: Vec<u8>,
}

impl TsStream {
    /// The registry codec descriptor for the stream type (the
    /// ffprobe-compatible vocabulary; unknown types pass through as
    /// `ts-stream-0xNN` — honest unknowns, never guesses).
    #[must_use]
    pub fn descriptor(&self) -> CodecDescriptor {
        let name = match self.stream_type {
            0x01 => "mpeg1video",
            0x02 => "mpeg2video",
            0x03 | 0x04 => "mp3",
            0x05 => "ts-private-sections",
            0x06 => "ts-private-pes",
            0x0F | 0x11 => "aac",
            0x10 => "mpeg4",
            0x1B => "h264",
            0x24 => "hevc",
            0x42 => "cavs",
            0x81 => "ac3",
            0x82 => "dts",
            0x83 => "truehd",
            0x84 | 0x87 => "eac3",
            0x90 => "pgssub",
            other => {
                // A dynamic-but-honest pass-through name.
                return CodecDescriptor::new(&format!("ts-stream-{other:02x}"))
                    .unwrap_or_else(|_| unreachable!("formatted names are valid"));
            }
        };
        CodecDescriptor::new(name).unwrap_or_else(|_| unreachable!("static names are valid"))
    }

    /// The media kind for the stream type (`Data` for the private and
    /// unknown ranges — the honest bucket, not a guess).
    #[must_use]
    pub fn kind(&self) -> MediaKind {
        match self.stream_type {
            0x01 | 0x02 | 0x10 | 0x1B | 0x24 | 0x42 => MediaKind::Video,
            0x03 | 0x04 | 0x0F | 0x11 | 0x81 | 0x82 | 0x83 | 0x84 | 0x87 => MediaKind::Audio,
            0x90 => MediaKind::Subtitle,
            _ => MediaKind::Data,
        }
    }
}

/// One program of the transport stream (the multi-program surface:
/// PAT entry + its PMT + its SDT service names).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsProgram {
    /// The PAT's `program_number`.
    pub number: u16,
    /// The PID this program's PMT rides.
    pub pmt_pid: u16,
    /// The PCR PID from the PMT (`None` until the PMT was seen).
    pub pcr_pid: Option<u16>,
    /// The SDT service name (`None` = no SDT or no 0x48 descriptor).
    pub service_name: Option<String>,
    /// The SDT service provider name.
    pub service_provider: Option<String>,
    /// The PMT's elementary streams (empty until the PMT was seen).
    pub streams: Vec<TsStream>,
}

/// Demuxer-side counters (the differential's truth and the damage
/// report's numbers).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TsStats {
    /// Synced 188-byte packets walked.
    pub packets: u64,
    /// Packets with the sync byte lost (recovered by the bounded
    /// resync scan).
    pub sync_errors: u64,
    /// Bytes skipped by resync (the `Recovered` events' sum).
    pub resync_bytes: u64,
    /// Packets carrying the transport error indicator (skipped).
    pub tei_packets: u64,
    /// Tolerated duplicate continuity counters.
    pub cc_duplicates: u64,
    /// Continuity-counter jumps (each raised one `Discontinuity`).
    pub cc_jumps: u64,
    /// Complete PSI sections parsed.
    pub psi_sections: u64,
    /// PSI sections consumed-but-skipped (CAT/EIT/TDT/...).
    pub psi_skipped_sections: u64,
    /// PSI sections dropped for CRC failure.
    pub psi_crc_failures: u64,
    /// Partial PSI sections dropped (a PUSI cut them off).
    pub psi_partial_drops: u64,
    /// CAT sections skipped-with-note.
    pub cat_skips: u64,
    /// PES-carrying packets delivered to the PES seam.
    pub pes_packets: u64,
    /// PES payload bytes delivered to the PES seam.
    pub pes_bytes: u64,
    /// Scrambled packets (delivered as-is, counted).
    pub scrambled_packets: u64,
    /// Null packets (stuffing).
    pub null_packets: u64,
    /// Packets on PIDs the PSI never declared.
    pub unknown_pid_packets: u64,
}

/// The PES half's plug point (card 5.I1's other half): every
/// payload-carrying packet on a program-declared elementary PID, its
/// TS header and adaptation field stripped, PUSI passed through. The
/// consumer reassembles PES packets and mints
/// [`DemuxEvent::Packet`]s; until it is registered the demuxer counts
/// the bytes and emits nothing (no fake packets, ever).
pub trait PesConsumer: Send {
    /// One TS packet's PES payload slice.
    ///
    /// # Errors
    /// The consumer's own typed error, propagated through
    /// `next_event` (the demuxer remains usable).
    fn pes_payload(&mut self, pid: u16, pusi: bool, payload: &[u8]) -> Result<(), EngineError>;
}

/// A replayable error shape (the WAV/Ogg/ADTS `StickyShape`
/// precedent).
#[derive(Debug, Clone, PartialEq, Eq)]
struct StickyShape {
    stage: media_types::PipelineStage,
    kind: media_types::ErrorKind,
    context: ErrorContext,
}

impl StickyShape {
    fn of(err: &EngineError) -> StickyShape {
        StickyShape {
            stage: err.stage,
            kind: err.kind.clone(),
            context: err.context.clone(),
        }
    }

    fn mint(&self) -> EngineError {
        EngineError::new(self.stage, self.kind.clone(), self.context.clone())
    }
}

/// Per-PID PSI section reassembly (a section spans packets on its
/// PID; PUSI's pointer field starts a new one).
#[derive(Default)]
struct SectionAssembler {
    buf: Vec<u8>,
    /// The whole-section byte count once the header declared it.
    need: Option<usize>,
}

impl SectionAssembler {
    /// Feed one packet's payload; `Some(section)` when it completed
    /// (owned — the buffer resets for the next section). Only the
    /// section's **declared** bytes are taken from a packet: a PUSI
    /// packet's tail may carry the next section's start, which this
    /// v0 drops (damage-counted upstream by the had-partial rule
    /// only when a PUSI cuts a live partial; back-to-back sections in
    /// one packet are the documented gap — the corpus and broadcast
    /// PSI repeat one section per PUSI packet).
    fn feed(&mut self, pusi: bool, payload: &[u8]) -> Option<Vec<u8>> {
        let mut source: &[u8] = payload;
        if pusi {
            self.buf.clear();
            self.need = None;
            if payload.is_empty() {
                return None;
            }
            let ptr = usize::from(payload[0]);
            if 1 + ptr > payload.len() {
                // A pointer field past the payload: unusable start.
                return None;
            }
            source = &payload[1 + ptr..];
        } else if self.buf.is_empty() {
            // Continuation without a start: nothing to append to.
            return None;
        }
        // Grow the header if the section just started.
        if self.buf.len() < 3 {
            let take = (3 - self.buf.len()).min(source.len());
            self.buf.extend_from_slice(&source[..take]);
            source = &source[take..];
            if self.buf.len() < 3 {
                return None;
            }
            let declared = 3 + ((usize::from(self.buf[1] & 0x0F) << 8) | usize::from(self.buf[2]));
            if declared > MAX_SECTION_BYTES {
                // Over the 12-bit ceiling: a corrupt length, drop it.
                self.buf.clear();
                return None;
            }
            self.need = Some(declared);
        }
        let declared = self.need?;
        if self.buf.len() < declared {
            let take = (declared - self.buf.len()).min(source.len());
            self.buf.extend_from_slice(&source[..take]);
        }
        if self.buf.len() == declared {
            let out = std::mem::take(&mut self.buf);
            self.need = None;
            return Some(out);
        }
        None
    }
}

/// The continuity-counter expectation for one PID.
#[derive(Default)]
struct CcState {
    last: Option<u8>,
}

// ---------------------------------------------------------------------------
// The demuxer
// ---------------------------------------------------------------------------

/// The MPEG-TS demuxer (packet layer + PSI; PES at the seam).
pub struct TsDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    pending: VecDeque<DemuxEvent>,
    /// The byte position of the next packet to walk.
    walk: u64,
    /// The stream head the PSI scan starts from (post open-resync).
    head: u64,
    programs: Vec<TsProgram>,
    pmt_pids: HashSet<u16>,
    es_pids: HashSet<u16>,
    pid_tracks: HashMap<u16, TrackId>,
    psi_buffers: HashMap<u16, SectionAssembler>,
    cc: HashMap<u16, CcState>,
    /// SDT services that arrived before their PAT program (the
    /// tables' arrival order is arbitrary; names must survive it).
    pending_services: Vec<crate::ts::psi::SdtService>,
    damage: DamageLog,
    stats: TsStats,
    last_pcr: Option<Pcr>,
    transport_stream_id: Option<u16>,
    original_network_id: Option<u16>,
    pes: Option<Box<dyn PesConsumer>>,
    tracks_out: Vec<Track>,
}

impl TsDemuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unknown-length,
    /// unseekable or empty sources (the crate-wide precedent).
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<TsDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(TsDemuxer {
            reader,
            started: false,
            eos: false,
            sticky: None,
            pending: VecDeque::new(),
            walk: 0,
            head: 0,
            programs: Vec::new(),
            pmt_pids: HashSet::new(),
            es_pids: HashSet::new(),
            pid_tracks: HashMap::new(),
            psi_buffers: HashMap::new(),
            cc: HashMap::new(),
            pending_services: Vec::new(),
            damage: DamageLog::default(),
            stats: TsStats::default(),
            last_pcr: None,
            transport_stream_id: None,
            original_network_id: None,
            pes: None,
            tracks_out: Vec::new(),
        })
    }

    /// Register the PES half's consumer (the seam; see [`PesConsumer`]).
    pub fn set_pes_consumer(&mut self, consumer: Box<dyn PesConsumer>) {
        self.pes = Some(consumer);
    }

    /// The parsed programs (the multi-program surface; empty before
    /// `start`).
    #[must_use]
    pub fn programs(&self) -> &[TsProgram] {
        &self.programs
    }

    /// The PAT's `transport_stream_id` (`None` before the PAT).
    #[must_use]
    pub fn transport_stream_id(&self) -> Option<u16> {
        self.transport_stream_id
    }

    /// The SDT's `original_network_id` (`None` before the SDT).
    #[must_use]
    pub fn original_network_id(&self) -> Option<u16> {
        self.original_network_id
    }

    /// The most recent raw PCR seen on any PID.
    #[must_use]
    pub fn last_pcr(&self) -> Option<Pcr> {
        self.last_pcr
    }

    /// The counters (the differential's truth).
    #[must_use]
    pub fn stats(&self) -> TsStats {
        self.stats
    }

    /// Total bytes/decisions skipped by bounded recovery (the
    /// damage log's sum; `0`-byte entries are decisions).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// The damage log itself (observability).
    #[must_use]
    pub fn damage_events(&self) -> &[(u64, &'static str)] {
        &self.damage.events
    }

    /// The engine track serving a PID (`None` = not an elementary
    /// stream of any program) — the PES half's routing table.
    #[must_use]
    pub fn track_of_pid(&self, pid: u16) -> Option<TrackId> {
        self.pid_tracks.get(&pid).copied()
    }

    // -- internals ---------------------------------------------------------

    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    fn note(&mut self, skipped: u64, reason: &'static str) {
        self.damage.note(skipped, reason);
    }

    /// Whether a PID's sections are consumed (well-known SI PIDs and
    /// discovered PMT PIDs).
    fn is_psi_pid(&self, pid: u16) -> bool {
        matches!(pid, PID_PAT | PID_CAT | PID_NIT | PID_SDT_BAT) || self.pmt_pids.contains(&pid)
    }

    /// The bounded open scan for the first sync byte; `Ok(None)` when
    /// the window carries no 188-stride `0x47`.
    fn open_sync(&mut self) -> Result<Option<u64>, EngineError> {
        let len = self.reader.length();
        let window = MAX_OPEN_SCAN_BYTES.min(len);
        let head = self.reader.read_prefix_at(window as usize, 0)?;
        if head.is_empty() {
            return Ok(None);
        }
        let mut at = 0;
        while at < head.len() {
            if head[at] == 0x47 {
                let next = at + PACKET_BYTES;
                if next >= head.len() {
                    if at == 0 {
                        return Ok(Some(0)); // single-packet stream
                    }
                    break;
                }
                if head[next] == 0x47 {
                    return Ok(Some(u64::try_from(at).unwrap_or(u64::MAX)));
                }
            }
            at += 1;
        }
        Ok(None)
    }

    /// Apply a complete parsed section to the program surface.
    fn apply_section(&mut self, section: &PsiSection) {
        match section {
            PsiSection::Pat(pat) => {
                self.transport_stream_id = Some(pat.transport_stream_id);
                for entry in &pat.entries {
                    if entry.program_number == 0 {
                        // The NIT pointer — not a program.
                        self.note(0, "ts: PAT NIT pointer noted");
                        continue;
                    }
                    self.pmt_pids.insert(entry.pid);
                    if let Some(prog) = self
                        .programs
                        .iter_mut()
                        .find(|p| p.number == entry.program_number)
                    {
                        prog.pmt_pid = entry.pid;
                    } else {
                        let mut program = TsProgram {
                            number: entry.program_number,
                            pmt_pid: entry.pid,
                            pcr_pid: None,
                            service_name: None,
                            service_provider: None,
                            streams: Vec::new(),
                        };
                        // A stashed SDT service may name this program.
                        if let Some(at) = self
                            .pending_services
                            .iter()
                            .position(|s| s.service_id == entry.program_number)
                        {
                            let svc = self.pending_services.remove(at);
                            program.service_name = svc.name;
                            program.service_provider = svc.provider;
                        }
                        self.programs.push(program);
                    }
                }
            }
            PsiSection::Pmt(pmt) => {
                let streams: Vec<TsStream> = pmt
                    .streams
                    .iter()
                    .map(|es| TsStream {
                        pid: es.pid,
                        stream_type: es.stream_type,
                        language: language_of(&es.es_info),
                        es_info: es.es_info.clone(),
                    })
                    .collect();
                for pid in pmt.streams.iter().map(|EsEntry { pid, .. }| *pid) {
                    self.es_pids.insert(pid);
                }
                if let Some(prog) = self
                    .programs
                    .iter_mut()
                    .find(|p| p.number == pmt.program_number)
                {
                    prog.pcr_pid = Some(pmt.pcr_pid);
                    prog.streams = streams;
                } else {
                    // A PMT with no PAT entry (PAT lost or late).
                    self.programs.push(TsProgram {
                        number: pmt.program_number,
                        pmt_pid: 0,
                        pcr_pid: Some(pmt.pcr_pid),
                        service_name: None,
                        service_provider: None,
                        streams,
                    });
                    self.note(0, "ts: PMT without PAT entry");
                }
            }
            PsiSection::Sdt(sdt) => {
                self.original_network_id = Some(sdt.original_network_id);
                for svc in &sdt.services {
                    if let Some(prog) = self
                        .programs
                        .iter_mut()
                        .find(|p| p.number == svc.service_id)
                    {
                        prog.service_name = svc.name.clone();
                        prog.service_provider = svc.provider.clone();
                    } else {
                        // No program yet — the PAT may still be coming.
                        self.pending_services.push(svc.clone());
                    }
                }
            }
            PsiSection::Skipped {
                table_id,
                bytes: consumed,
            } => {
                self.stats.psi_skipped_sections += 1;
                self.stats.cat_skips += u64::from(*table_id == TID_CAT);
                if *table_id == TID_CAT {
                    self.note(
                        u64::try_from(*consumed).unwrap_or(u64::MAX),
                        "ts: CAT section skipped (conditional access)",
                    );
                } else if !is_eit(*table_id) {
                    // EITs are periodic broadcast furniture on real
                    // streams — counted, not logged.
                    self.note(0, "ts: SI section skipped");
                }
            }
        }
    }

    /// Walk one 188-byte packet at `self.walk` (already synced).
    fn process_packet(&mut self, packet: &[u8]) {
        self.stats.packets += 1;
        let parsed = match TsPacket::parse(packet) {
            Ok(p) => p,
            Err(_) => {
                // Adaptation field overruns the packet: damage-noted,
                // the walk continues at the next packet boundary.
                self.note(PACKET_BYTES as u64, "ts: bad packet header skipped");
                return;
            }
        };
        if parsed.transport_error {
            self.stats.tei_packets += 1;
            self.note(0, "ts: transport error indicator");
            return;
        }
        if parsed.pid == PID_NULL {
            self.stats.null_packets += 1;
            return;
        }
        // Continuity policy (payload-bearing packets only, §2.4.3.2).
        if parsed.has_payload() {
            let expected = self.cc.get(&parsed.pid).and_then(|s| s.last);
            if let Some(last) = expected {
                if parsed.continuity_counter == last {
                    self.stats.cc_duplicates += 1;
                } else if parsed.continuity_counter != (last + 1) & 0x0F {
                    self.stats.cc_jumps += 1;
                    self.note(0, "ts: continuity counter jump");
                    self.pending.push_back(DemuxEvent::Discontinuity);
                    // A PSI PID's partial section cannot be trusted
                    // across the jump — drop it.
                    if self.is_psi_pid(parsed.pid)
                        && let Some(asm) = self.psi_buffers.get_mut(&parsed.pid)
                        && !asm.buf.is_empty()
                    {
                        asm.buf.clear();
                        asm.need = None;
                        self.stats.psi_partial_drops += 1;
                    }
                }
            }
            self.cc.entry(parsed.pid).or_default().last = Some(parsed.continuity_counter);
        }
        if let Some(af) = parsed.adaptation {
            if af.pcr.is_some() {
                self.last_pcr = af.pcr;
            }
            if af.discontinuity {
                self.pending.push_back(DemuxEvent::Discontinuity);
                self.note(0, "ts: discontinuity indicator");
            }
        }
        if parsed.scrambling != 0 {
            self.stats.scrambled_packets += 1;
        }
        if parsed.payload.is_empty() {
            return;
        }
        if self.is_psi_pid(parsed.pid) {
            let had_partial = self
                .psi_buffers
                .get(&parsed.pid)
                .is_some_and(|a| !a.buf.is_empty());
            let completed = self
                .psi_buffers
                .entry(parsed.pid)
                .or_default()
                .feed(parsed.payload_unit_start, parsed.payload);
            // A PUSI cut a live partial, or a non-PUSI feed dropped
            // it (length over the ceiling): the partial is gone.
            if had_partial
                && (parsed.payload_unit_start
                    || (completed.is_none()
                        && self
                            .psi_buffers
                            .get(&parsed.pid)
                            .is_some_and(|a| a.buf.is_empty())))
            {
                self.stats.psi_partial_drops += 1;
            }
            if let Some(section) = completed {
                self.stats.psi_sections += 1;
                match parse_section(&section) {
                    Ok(parsed) => self.apply_section(&parsed),
                    Err(err) if err.context.code == "demux.ts.psi.crc-mismatch" => {
                        self.stats.psi_crc_failures += 1;
                        self.note(0, "ts: PSI section CRC failure");
                    }
                    Err(_) => {
                        // Structural damage in one section: dropped,
                        // the walk and the other PIDs continue.
                        self.note(0, "ts: PSI section unparseable");
                    }
                }
            }
        } else if self.es_pids.contains(&parsed.pid) {
            self.stats.pes_packets += 1;
            self.stats.pes_bytes += parsed.payload.len() as u64;
            if let Some(consumer) = self.pes.as_mut()
                && let Err(err) =
                    consumer.pes_payload(parsed.pid, parsed.payload_unit_start, parsed.payload)
            {
                let _ = self.fail(err);
            }
        } else {
            self.stats.unknown_pid_packets += 1;
        }
    }

    /// The PSI-scan completeness test: PAT + every declared PMT + SDT.
    fn psi_complete(&self) -> bool {
        self.transport_stream_id.is_some()
            && self.original_network_id.is_some()
            && !self.programs.is_empty()
            && self
                .programs
                .iter()
                .all(|p| p.pcr_pid.is_some() && !p.streams.is_empty())
    }

    /// Build the engine tracks from the collected programs (the
    /// multi-program surface's plumbing half).
    fn build_tracks(&mut self) {
        let base = TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}"));
        let mut tracks = Vec::new();
        let mut routing = HashMap::new();
        let mut next = 0u32;
        for prog in &self.programs {
            for stream in &prog.streams {
                let id = TrackId::new(next);
                let stream_id = StreamId::new(next);
                routing.insert(stream.pid, id);
                tracks.push(Track {
                    id,
                    stream: stream_id,
                    kind: stream.kind(),
                    language: stream.language.clone(),
                    codec: stream.descriptor(),
                    title: prog.service_name.clone(),
                    flags: TrackFlags::NONE,
                    timing: TrackTiming {
                        span: TimeSpan::UNKNOWN,
                        base,
                        gaps: Vec::new(),
                    },
                });
                next += 1;
            }
        }
        self.pid_tracks = routing;
        self.tracks_out = tracks;
    }

    /// Read the packet at `walk`; `Ok(None)` = the stream ended.
    fn read_packet(&mut self) -> Result<Option<Vec<u8>>, EngineError> {
        let bytes = self.reader.read_prefix_at(PACKET_BYTES, self.walk)?;
        if bytes.len() < PACKET_BYTES {
            return Ok(None);
        }
        Ok(Some(bytes))
    }

    /// The bounded resync scan from `self.walk` (sync lost). On
    /// success `self.walk` is the new synced position.
    fn resync(&mut self) -> Result<u64, EngineError> {
        let len = self.reader.length();
        let window_end =
            (self.walk + u64::try_from(MAX_RESYNC_SCAN_BYTES).unwrap_or(u64::MAX)).min(len);
        let want = usize::try_from(window_end - self.walk).unwrap_or(0);
        let window = self.reader.read_prefix_at(want, self.walk)?;
        let mut at = 1;
        while at + PACKET_BYTES < window.len() {
            if window[at] == 0x47 && window[at + PACKET_BYTES] == 0x47 {
                let skipped = u64::try_from(at).unwrap_or(u64::MAX);
                self.stats.sync_errors += 1;
                self.stats.resync_bytes += skipped;
                self.walk += skipped;
                self.note(skipped, "ts: resync after sync loss");
                self.pending.push_back(DemuxEvent::Recovered {
                    skipped,
                    reason: "ts: resync after sync loss",
                });
                return Ok(skipped);
            }
            at += 1;
        }
        Err(resync_failed(self.walk))
    }
}

impl Demuxer for TsDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if let Some(shape) = self.sticky.as_ref() {
            return Err(shape.mint());
        }
        if self.started {
            return Ok(self.tracks_out.clone());
        }
        // The open probe: first synced packet (a head offset is a
        // resync-class damage note, not an error).
        let head = match self.open_sync()? {
            Some(at) => at,
            None => {
                let sniffed = self.reader.read_prefix_at(16, 0)?;
                return Err(self.fail(not_ts(&sniffed)));
            }
        };
        if head > 0 {
            self.stats.sync_errors += 1;
            self.stats.resync_bytes += head;
            self.note(head, "ts: stream head resync");
        }
        self.head = head;
        self.walk = head;
        // The bounded PSI scan.
        let len = self.reader.length();
        let scan_end = (self.head + MAX_PSI_SCAN_BYTES).min(len);
        while self.walk + PACKET_BYTES as u64 <= scan_end && !self.psi_complete() {
            let packet = match self.read_packet()? {
                Some(p) => p,
                None => break,
            };
            if packet[0] != 0x47 {
                if self.resync().is_err() {
                    break;
                }
                continue;
            }
            self.process_packet(&packet);
            self.walk += PACKET_BYTES as u64;
        }
        if self.transport_stream_id.is_none() {
            return Err(self.fail(no_pat()));
        }
        let trackless: Vec<bool> = self
            .programs
            .iter()
            .map(|p| p.pcr_pid.is_none() || p.streams.is_empty())
            .collect();
        for trackless in trackless {
            if trackless {
                self.note(0, "ts: program without a usable PMT");
            }
        }
        self.build_tracks();
        // The scan's bookkeeping is collection-time observation, not
        // the delivered walk's truth — reset it. The replay from the
        // head re-encounters (and re-reports) everything once; the
        // PES seam then sees the whole stream, and the PSI tables
        // re-parse harmlessly (already applied).
        self.pending.clear();
        self.damage = DamageLog::default();
        self.stats = TsStats::default();
        if head > 0 {
            self.stats.sync_errors = 1;
            self.stats.resync_bytes = head;
            self.note(head, "ts: stream head resync");
        }
        self.walk = self.head;
        self.psi_buffers.clear();
        self.cc.clear();
        self.started = true;
        Ok(self.tracks_out.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(err) = self.sticky.as_ref() {
            return Err(err.clone().mint());
        }
        if !self.started {
            return Err(self.fail(not_started()));
        }
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(event);
            }
            if self.eos {
                return Ok(DemuxEvent::EndOfStream);
            }
            let packet = match self.read_packet()? {
                Some(p) => p,
                None => {
                    // The stream ended; a partial tail packet is a
                    // damage note, not a truncation error (the walk
                    // delivered everything it could).
                    let left = self.reader.length() - self.walk;
                    if left > 0 {
                        self.note(left, "ts: partial tail packet skipped");
                        self.pending.push_back(DemuxEvent::Recovered {
                            skipped: left,
                            reason: "ts: partial tail packet skipped",
                        });
                    }
                    // A PSI section left incomplete at the end is
                    // counted, not silently dropped.
                    for asm in self.psi_buffers.values() {
                        if !asm.buf.is_empty() {
                            self.stats.psi_partial_drops += 1;
                        }
                    }
                    self.eos = true;
                    continue;
                }
            };
            if packet[0] != 0x47 {
                self.resync()?;
                continue;
            }
            self.process_packet(&packet);
            self.walk += PACKET_BYTES as u64;
        }
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(self.fail(mint(
            ErrorKind::Unsupported {
                format: Some(FormatInfo {
                    name: Some("mpegts".to_string()),
                    sniffed: Vec::new(),
                }),
            },
            "demux.ts.seek.unsupported",
        )))
    }

    fn seekability(&self) -> SeekSupport {
        SeekSupport::None
    }
}

/// The ISO 639 language descriptor (0x0A) of an ES descriptor loop —
/// the first well-formed one wins.
fn language_of(es_info: &[u8]) -> Option<LanguageTag> {
    let mut at = 0;
    while at + 2 <= es_info.len() {
        let tag = es_info[at];
        let len = usize::from(es_info[at + 1]);
        if at + 2 + len > es_info.len() {
            return None;
        }
        if tag == 0x0A && len >= 3 {
            if let Ok(code) = std::str::from_utf8(&es_info[at + 2..at + 5])
                && let Ok(lang) = LanguageTag::parse(code)
            {
                return Some(lang);
            }
            return None;
        }
        at += 2 + len;
    }
    None
}

/// Whether a table id is in the EIT range (0x4E–0x6F, EN 300 468
/// §5.2.4 — the broadcast-now/next furniture).
fn is_eit(table_id: u8) -> bool {
    (0x4E..=0x6F).contains(&table_id)
}
//! The MPEG transport-stream demux layers (card 5.I1).
//!
//! **I1b seam state (this tree):** [`pes`] — the PES layer, the
//! 90 kHz timing model, and the minimal PID→PES assembly seam —
//! landed here first. The parallel PSI half (I1a: PAT/PMT/SDT and
//! the transport-packet walk) and the full `Demuxer` implementation
//! build on top of it. Merge rule for the Lead's I1 unification:
//! append `pub mod` lines, never reorder the existing ones.

pub mod pes;
