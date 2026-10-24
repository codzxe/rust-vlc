//! PSI/SI section parsing (card 5.I1a — the PSI half of MPEG-TS):
//! ISO/IEC 13818-1 §2.4.4 sections (PAT/PMT/CAT) plus the DVB SI
//! tables the Lumen surface consumes (SDT, EN 300 468 §5.2.3).
//!
//! # What this is
//!
//! Pure byte work over **complete** sections (the packet layer in
//! [`super`] reassembles them; a section spans TS packets on its PID).
//! One section is `table_id`, a 16-bit header whose low 12 bits are
//! `section_length`, the table body, and — when
//! `section_syntax_indicator == 1` — a trailing CRC-32/MPEG-2 that
//! must make the whole section hash to zero.
//!
//! # Section arithmetic (the bounded-memory discipline)
//!
//! `section_length` is a **12-bit** count of the bytes after the
//! length field itself, so a section never exceeds
//! [`MAX_SECTION_BYTES`] = 3 + 4095. The parser trusts nothing: every
//! loop walks a cursor and every declared sub-length
//! (`program_info_length`, `ES_info_length`, descriptor lengths, the
//! SDT `descriptors_loop_length`) is clamped to its parent before it
//! is consumed — a lying length is a typed truncation, never an
//! over-read or a loop.
//!
//! # The SDT service-loop header is 5 bytes
//!
//! EN 300 468 §5.2.3: `service_id` (16) + reserved/EIT flags (8) +
//! `descriptors_loop_length` (12 bits in 16). A prior session read 6;
//! the corpus SDT bytes pin 5 (a 6-byte read would eat the service
//! descriptor's tag byte and every name walks off the rails — the
//! differential caught it).
//!
//! # DVB text (EN 300 468 annex A, the honest subset)
//!
//! The corpus (ffmpeg-authored) writes plain ASCII with **no**
//! charset prefix. The decoder accepts: no prefix / a leading byte
//! ≥ 0x20 → ISO 6937-shaped text (ASCII pass-through; the 2-byte
//! precomposed accents of leads 0xC1–0xCC on a Latin-1 base letter —
//! the common Western set, table in [`compose_iso6937`]; anything
//! else falls to the Latin-1 reading); prefix `0x05` → the same
//! after stripping; `0x15` → UTF-8; `0x00`–`0x0B` → the annex's
//! ISO 8859 parts approximated as Latin-1; `0x10` → a 2-byte code
//! follows (skipped, then the annex's ISO 10646 approximated as
//! UTF-16-BE). Non-representable compositions fall back to the raw
//! byte — recorded as a documented gap, never an error: a garbled
//! service name is damage to surface, not a stream to refuse.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// A whole section (header + body + CRC) is at most this
/// (3 + the 12-bit `section_length` ceiling).
pub const MAX_SECTION_BYTES: usize = 3 + 4095;

/// `table_id` — program association (ISO 13818-1 §2.4.4.3).
pub const TID_PAT: u8 = 0x00;
/// `table_id` — conditional access (skipped with a note).
pub const TID_CAT: u8 = 0x01;
/// `table_id` — program map (§2.4.4.8).
pub const TID_PMT: u8 = 0x02;
/// `table_id` — service description (EN 300 468 §5.2.3).
pub const TID_SDT: u8 = 0x42;

/// The CRC-32/MPEG-2 table (poly `0x04C11DB7`, init `0xFFFF_FFFF`,
/// no reflection, no final xor) — generated natively at compile time.
static CRC_TABLE: [u32; 256] = build_crc_table();

const fn build_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u32) << 24;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04C1_1DB7
            } else {
                crc << 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

/// CRC-32/MPEG-2 of `bytes` (the check value of the appended-CRC form
/// is zero — [`section_crc_ok`] uses exactly that).
#[must_use]
pub fn crc32_mpeg(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc = (crc << 8) ^ CRC_TABLE[(((crc >> 24) ^ u32::from(*byte)) & 0xFF) as usize];
    }
    crc
}

/// Whether a long-form section's trailing CRC-32 verifies
/// (whole-section hash == 0).
#[must_use]
pub fn section_crc_ok(section: &[u8]) -> bool {
    crc32_mpeg(section) == 0
}

// -- typed errors -----------------------------------------------------------

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(PipelineStage::Demux, kind, ErrorContext::new(code))
}

/// Not an MPEG-TS PSI section at all (broken table shape).
pub fn not_psi(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("mpegts-psi".to_string()),
                sniffed: sniffed[..sniffed.len().min(16)].to_vec(),
            }),
        },
        "demux.ts.psi.not-psi",
    )
}

/// The section's declared structure runs past the bytes actually
/// present (`expected` = the byte count the section needed, `found` =
/// what the caller supplied).
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.ts.psi.truncated",
    )
}

/// The section's arithmetic is impossible (`section_length` smaller
/// than its mandatory header, a loop length that breaks the stride,
/// a length field overrunning its parent).
pub fn bad_structure(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ts.psi.bad-structure",
    )
}

/// The trailing CRC-32 does not verify — the section is damaged.
/// `offset` rides the source chain (the taxonomy's no-payload rule:
/// the section's bytes never enter the error).
pub fn crc_mismatch(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::ChecksumMismatch,
        },
        "demux.ts.psi.crc-mismatch",
    )
    .with_source(SectionOffset(offset))
}

/// Source-chain leaf carrying a section's byte offset (structured
/// data only, per the L0 no-secrets rule).
#[derive(Debug)]
struct SectionOffset(u64);

impl std::fmt::Display for SectionOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PSI section at byte {}", self.0)
    }
}
impl std::error::Error for SectionOffset {}

// -- the parsed tables ------------------------------------------------------

/// One PAT program entry: `program_number` → the PID its PMT rides
/// (`0x1FFF` = the NIT pointer, exposed as-is).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatEntry {
    /// The program's number (1 is ffmpeg's default).
    pub program_number: u16,
    /// The PID carrying this program's PMT (or the NIT PID for
    /// `program_number == 0`).
    pub pid: u16,
}

/// A parsed Program Association Table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pat {
    /// `transport_stream_id`.
    pub transport_stream_id: u16,
    /// The program loop (empty = a TS with no programs).
    pub entries: Vec<PatEntry>,
}

/// One PMT elementary stream entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EsEntry {
    /// The raw `stream_type` byte (the mapping to a codec descriptor
    /// is the demuxer's registry row, not the section's).
    pub stream_type: u8,
    /// The PID carrying this elementary stream.
    pub pid: u16,
    /// The verbatim `ES_info` descriptor bytes (registration, ISO 639
    /// language, ... — kept raw for the decode seam).
    pub es_info: Vec<u8>,
}

/// A parsed Program Map Table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pmt {
    /// `program_number` (links back to the PAT entry).
    pub program_number: u16,
    /// The PID carrying the program's PCR.
    pub pcr_pid: u16,
    /// The verbatim program-level descriptor bytes.
    pub program_info: Vec<u8>,
    /// The elementary-stream loop.
    pub streams: Vec<EsEntry>,
}

/// One SDT service (EN 300 468 §5.2.3 + the 0x48 descriptor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtService {
    /// `service_id` (== the PAT `program_number` of the same service).
    pub service_id: u16,
    /// The 0x48 descriptor's `service_type` byte (`0x01` = digital
    /// television, the ffmpeg default).
    pub service_type: u8,
    /// The provider name (the 0x48 descriptor's first string).
    pub provider: Option<String>,
    /// The service name (the 0x48 descriptor's second string).
    pub name: Option<String>,
}

/// A parsed Service Description Table (actual-stream section only,
/// `table_id 0x42`; the other-transport variant 0x46 is skipped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sdt {
    /// `transport_stream_id`.
    pub transport_stream_id: u16,
    /// `original_network_id`.
    pub original_network_id: u16,
    /// The service loop.
    pub services: Vec<SdtService>,
}

/// One complete parsed section — the packet layer's per-PID outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PsiSection {
    /// A Program Association Table.
    Pat(Pat),
    /// A Program Map Table.
    Pmt(Pmt),
    /// A Service Description Table.
    Sdt(Sdt),
    /// A recognized-but-unconsumed section (CAT, NIT, BAT, EIT, TDT,
    /// ... and `current_next_indicator == 0` "future" versions):
    /// consumed positionally with a note, never a failure.
    Skipped {
        /// The section's `table_id`.
        table_id: u8,
        /// The section's whole byte count.
        bytes: usize,
    },
}

/// Parse one **complete** section (`buf` is exactly the section: its
/// `section_length` must account for every byte present).
///
/// # Errors
/// Typed (stage `Demux`): [`not_psi`] for a shape that is no section,
/// [`bad_structure`] for impossible arithmetic, [`truncated`] when the
/// declared structure needs more bytes than `buf` carries,
/// [`crc_mismatch`] when a long-form section's CRC-32 fails.
pub fn parse_section(buf: &[u8]) -> Result<PsiSection, EngineError> {
    if buf.len() < 3 {
        return Err(not_psi(buf));
    }
    let table_id = buf[0];
    let syntax = buf[1] & 0x80 != 0;
    let section_length = (usize::from(buf[1] & 0x0F) << 8) | usize::from(buf[2]);
    if section_length < 5 {
        // The long-form remainder (5 fixed bytes + 4 CRC) cannot fit.
        return Err(bad_structure(0));
    }
    let whole = 3 + section_length;
    if whole != buf.len() {
        // The reassembler hands over exactly this many bytes; a
        // mismatch means the two arithmetics disagree.
        return Err(truncated(whole as u64, buf.len() as u64));
    }
    // The tables we consume are all long-form; short-form tables are
    // skipped as consumed bytes.
    if !syntax {
        return Ok(PsiSection::Skipped {
            table_id,
            bytes: whole,
        });
    }
    if whole < 12 {
        // 8 fixed header bytes + a non-empty body + 4 CRC.
        return Err(bad_structure(0));
    }
    if !section_crc_ok(buf) {
        return Err(crc_mismatch(0));
    }
    let body = &buf[..whole - 4]; // CRC stripped
    // Long-form tail: version/current_next + section numbers.
    let current_next = body[5] & 0x01 != 0;
    if !current_next {
        // A "next" (not yet applicable) version: consumed, not used.
        return Ok(PsiSection::Skipped {
            table_id,
            bytes: whole,
        });
    }
    match table_id {
        TID_PAT => parse_pat(body).map(PsiSection::Pat),
        TID_PMT => parse_pmt(body).map(PsiSection::Pmt),
        TID_SDT => parse_sdt(body).map(PsiSection::Sdt),
        _ => Ok(PsiSection::Skipped {
            table_id,
            bytes: whole,
        }),
    }
}

fn parse_pat(body: &[u8]) -> Result<Pat, EngineError> {
    // body: 8 fixed bytes (tid, len, tsid, ver+cni, sec_num, last)
    // + 4-byte entries to the end.
    let transport_stream_id = u16::from_be_bytes([body[3], body[4]]);
    let mut entries = Vec::new();
    let mut at = 8;
    if body.len() < 8 || !body.len().is_multiple_of(4) {
        return Err(bad_structure(8));
    }
    while at + 4 <= body.len() {
        let program_number = u16::from_be_bytes([body[at], body[at + 1]]);
        let pid = u16::from_be_bytes([body[at + 2], body[at + 3]]) & 0x1FFF;
        entries.push(PatEntry {
            program_number,
            pid,
        });
        at += 4;
    }
    Ok(Pat {
        transport_stream_id,
        entries,
    })
}

fn parse_pmt(body: &[u8]) -> Result<Pmt, EngineError> {
    // body: program_number(2) + version(1) + sec(1) + last(1) +
    // PCR_PID(2) + program_info_length(2) + program descriptors +
    // the ES loop (the CRC was stripped upstream).
    if body.len() < 12 {
        return Err(bad_structure(0));
    }
    let program_number = u16::from_be_bytes([body[3], body[4]]);
    let pcr_pid = u16::from_be_bytes([body[8], body[9]]) & 0x1FFF;
    let pil = (usize::from(body[10] & 0x0F) << 8) | usize::from(body[11]);
    if 12 + pil > body.len() {
        return Err(truncated((12 + pil) as u64, body.len() as u64));
    }
    let program_info = body[12..12 + pil].to_vec();
    let mut at = 12 + pil;
    let mut streams = Vec::new();
    // The ES stride is 5 + ES_info_length; every step is bounded by
    // the body end before any field is trusted.
    while at < body.len() {
        if at + 5 > body.len() {
            return Err(bad_structure(at as u64));
        }
        let stream_type = body[at];
        let pid = u16::from_be_bytes([body[at + 1], body[at + 2]]) & 0x1FFF;
        let eil = (usize::from(body[at + 3] & 0x0F) << 8) | usize::from(body[at + 4]);
        if at + 5 + eil > body.len() {
            return Err(truncated((at + 5 + eil) as u64, body.len() as u64));
        }
        streams.push(EsEntry {
            stream_type,
            pid,
            es_info: body[at + 5..at + 5 + eil].to_vec(),
        });
        at += 5 + eil;
    }
    Ok(Pmt {
        program_number,
        pcr_pid,
        program_info,
        streams,
    })
}

fn parse_sdt(body: &[u8]) -> Result<Sdt, EngineError> {
    // body: 8 fixed bytes as PAT + original_network_id(2) +
    // reserved(1), then the service loop. THE service-loop header is
    // 5 bytes: service_id(2) + flags(1) + descriptors_loop_length(2)
    // (EN 300 468 §5.2.3 — a prior session read 6; the corpus pins
    // 5, see the module docs).
    if body.len() < 11 {
        return Err(bad_structure(0));
    }
    let transport_stream_id = u16::from_be_bytes([body[3], body[4]]);
    let original_network_id = u16::from_be_bytes([body[8], body[9]]);
    let mut at = 11;
    let mut services = Vec::new();
    while at < body.len() {
        if at + 5 > body.len() {
            return Err(bad_structure(at as u64));
        }
        let service_id = u16::from_be_bytes([body[at], body[at + 1]]);
        let dl = (usize::from(body[at + 3] & 0x0F) << 8) | usize::from(body[at + 4]);
        if at + 5 + dl > body.len() {
            return Err(truncated((at + 5 + dl) as u64, body.len() as u64));
        }
        let descriptors = &body[at + 5..at + 5 + dl];
        if let Some(svc) = parse_service_descriptor(descriptors, service_id) {
            services.push(svc);
        }
        at += 5 + dl;
    }
    Ok(Sdt {
        transport_stream_id,
        original_network_id,
        services,
    })
}

/// Walk a service's descriptor loop for the 0x48 service descriptor
/// (other descriptors are consumed positionally).
fn parse_service_descriptor(descriptors: &[u8], service_id: u16) -> Option<SdtService> {
    let mut at = 0;
    while at + 2 <= descriptors.len() {
        let tag = descriptors[at];
        let len = usize::from(descriptors[at + 1]);
        if at + 2 + len > descriptors.len() {
            // A lying descriptor length: the service degrades to
            // unknown names (the loop above already validated the
            // parent length, so this is defensiveness, not policy).
            return None;
        }
        if tag == 0x48 {
            let d = &descriptors[at + 2..at + 2 + len];
            let mut svc = SdtService {
                service_id,
                service_type: 0,
                provider: None,
                name: None,
            };
            if !d.is_empty() {
                svc.service_type = d[0];
            }
            // service_type(1) + provider_len(1) + provider +
            // name_len(1) + name; each string only when declared.
            if d.len() >= 2 {
                let plen = usize::from(d[1]);
                if 2 + plen <= d.len() {
                    svc.provider = Some(dvb_text(&d[2..2 + plen]));
                    if 2 + plen < d.len() {
                        let nlen = usize::from(d[2 + plen]);
                        if 2 + plen + 1 + nlen <= d.len() {
                            svc.name = Some(dvb_text(&d[2 + plen + 1..2 + plen + 1 + nlen]));
                        }
                    }
                }
            }
            return Some(svc);
        }
        at += 2 + len;
    }
    None
}

/// Decode a DVB text string (EN 300 468 annex A — the documented
/// subset; see the module docs). Empty input is the empty string.
#[must_use]
pub fn dvb_text(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let mut text = bytes;
    // A leading byte below 0x20 is a charset selector (annex A.1).
    if text[0] < 0x20 {
        let selector = text[0];
        text = &text[1..];
        match selector {
            0x05 => {} // ISO 6937 — the default shape below.
            0x15 => return String::from_utf8_lossy(text).into_owned(),
            0x10 => {
                // Two selector bytes follow, then ISO 10646 text —
                // approximated as UTF-16-BE (annex A.3's default).
                let skip = text.len().min(2);
                text = &text[skip..];
                return decode_utf16be_lossy(text);
            }
            // 0x00–0x0B: the ISO 8859 parts — approximated as
            // Latin-1 (the documented gap; ASCII is identical).
            _ => {}
        }
    }
    let mut out = String::new();
    let mut at = 0;
    while at < text.len() {
        let b = text[at];
        if (0xC1..=0xCC).contains(&b)
            && at + 1 < text.len()
            && (0x21..=0x7E).contains(&text[at + 1])
            && let Some(composed) = compose_iso6937(b, text[at + 1])
        {
            out.push(composed);
            at += 2;
            continue;
        }
        out.push(char::from(b)); // Latin-1 reading (identity on ASCII)
        at += 1;
    }
    out
}

fn decode_utf16be_lossy(bytes: &[u8]) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    let mut at = 0;
    while at + 2 <= bytes.len() {
        units.push(u16::from_be_bytes([bytes[at], bytes[at + 1]]));
        at += 2;
    }
    String::from_utf16_lossy(&units)
}

/// The ISO 6937 two-byte precomposed accents (the common Western
/// set — lead 0xC1–0xCC selects the accent, the second byte the base
/// letter). `None` = the pair is not in the carried subset (the
/// caller falls back to the Latin-1 reading of the lead byte).
fn compose_iso6937(lead: u8, second: u8) -> Option<char> {
    let base = second.to_ascii_lowercase();
    let composed = match lead {
        0xC1 => match base {
            b'a' => 'Á',
            b'e' => 'É',
            b'i' => 'Í',
            b'o' => 'Ó',
            b'u' => 'Ú',
            b'y' => 'Ý',
            b'c' => 'Ć',
            b'n' => 'Ń',
            b'z' => 'Ź',
            _ => return None,
        },
        0xC2 => match base {
            b'a' => 'À',
            b'e' => 'È',
            b'i' => 'Ì',
            b'o' => 'Ò',
            b'u' => 'Ù',
            _ => return None,
        },
        0xC3 => match base {
            b'a' => 'Â',
            b'e' => 'Ê',
            b'i' => 'Î',
            b'o' => 'Ô',
            b'u' => 'Û',
            _ => return None,
        },
        0xC4 => match base {
            b'a' => 'Ã',
            b'o' => 'Õ',
            b'n' => 'Ñ',
            _ => return None,
        },
        0xC8 => match base {
            b'a' => 'Ä',
            b'e' => 'Ë',
            b'i' => 'Ï',
            b'o' => 'Ö',
            b'u' => 'Ü',
            b'y' => 'Ÿ',
            _ => return None,
        },
        0xC9 => match base {
            b'u' => 'Ů',
            b'a' => 'Å',
            _ => return None,
        },
        0xCB => match base {
            b'c' => 'Ç',
            _ => return None,
        },
        0xCC => match base {
            b'c' => 'Č',
            b's' => 'Š',
            b'z' => 'Ž',
            b'r' => 'Ř',
            b'd' => 'Ď',
            b't' => 'Ť',
            b'n' => 'Ň',
            _ => return None,
        },
        _ => return None,
    };
    Some(composed)
}
