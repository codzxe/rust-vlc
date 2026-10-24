//! VBR seek headers (card 5.A3): the Xing/Info tag, the LAME
//! gapless extension, and the rare Fraunhofer VBRI header.
//!
//! # Xing/Info (the common case)
//!
//! The FIRST audio frame of a VBR (or ffmpeg-authored CBR) MP3
//! carries, in its ancillary data after the side information, a
//! `Xing` (VBR) or `Info` (CBR) header:
//!
//! ```text
//! "Xing"/"Info" | flags u32 | [frames u32] [bytes u32] [toc u8×100]
//!                                 [quality u32] [LAME tag…]
//! ```
//!
//! Fields ride only when their flag bit is set (bit0 frames, bit1
//! bytes, bit2 toc, bit3 quality). The **TOC** is 100 bytes: entry
//! `i` holds the percentage (`i` → `toc[i]`) of the file's byte
//! offset where roughly `i%` of the frames have passed — the VBR
//! seek index ([`seek`](super::seek)).
//!
//! # LAME tag (gapless facts)
//!
//! After the Xing fields, LAME (and ffmpeg's libmp3lame wrapper)
//! writes its encoder tag. The two fields we need — **encoder
//! delay** and **padding**, 12 bits each — sit 21 bytes into the
//! LAME tag:
//!
//! ```text
//! delay   = (tag[21] << 4) | (tag[22] >> 4)
//! padding = ((tag[22] & 0x0F) << 8) | tag[23]
//! ```
//!
//! The honest audio content of the stream is
//! `frames × samples_per_frame − delay − padding` samples; trimming
//! that window is the gapless job of [`GaplessTrim`]/the stream
//! decoder.
//!
//! # VBRI (the rare case)
//!
//! Fraunhofer encoders put a `VBRI` header exactly 32 bytes after
//! the end of the 4-byte frame header (+CRC), carrying version,
//! **delay**, quality, bytes, frames and a table-of-contents with
//! explicit entry count/scale/size. No known offline encoder in the
//! sandbox produces it (ffmpeg cannot; LAME writes Xing), so the VBRI
//! path is unit-tested against hand-crafted headers — stated
//! honestly in the worklog.

use super::header::{FrameHeader, side_info_size};
use super::error::{corrupt, unsupported};
use media_types::EngineError;
use media_types::error::CorruptDetail;

/// The Xing/Info header + LAME gapless facts of one stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XingInfo {
    /// The 4-byte magic: `Xing` (VBR) or `Info` (CBR carrier).
    pub magic: [u8; 4],
    /// True when the magic is `Xing` (a VBR stream claim).
    pub is_vbr: bool,
    /// Total frame count, when the flag bit was set.
    pub frames: Option<u32>,
    /// Total stream byte length (audio frames only), when set.
    pub bytes: Option<u32>,
    /// The 100-entry seek TOC, when set.
    pub toc: Option<[u8; 100]>,
    /// The quality field, when set.
    pub quality: Option<u32>,
    /// The LAME tag's gapless facts, when a LAME tag was found.
    pub lame: Option<LameTag>,
    /// Byte offset of the carrying frame's start within the file.
    pub frame_offset: u64,
    /// Byte offset just past the carrying frame.
    pub frame_end: u64,
}

/// LAME-tag gapless facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LameTag {
    /// Encoder delay in samples (silence the encoder prepended).
    pub delay: u32,
    /// Encoder padding in samples (silence appended to fill the last
    /// frame).
    pub padding: u32,
}

impl LameTag {
    /// The gapless trim window for a stream with `total` decoded
    /// samples: `[delay, total - padding)`.
    #[must_use]
    pub fn trim_window(&self, total: u64) -> (u64, u64) {
        let start = u64::from(self.delay);
        let end = total.saturating_sub(u64::from(self.padding)).max(start);
        (start, end)
    }
}

/// A parsed VBRI header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbriInfo {
    /// VBRI version.
    pub version: u16,
    /// Encoder delay in samples (VBRI carries it directly).
    pub delay: u16,
    /// Quality indicator.
    pub quality: u16,
    /// Total stream byte length.
    pub bytes: u32,
    /// Total frame count.
    pub frames: u32,
    /// TOC entries (flat bytes; entry size/scale per header fields).
    pub toc: Vec<u8>,
    /// Bytes per TOC entry.
    pub toc_entry_size: u16,
    /// Frames per TOC entry.
    pub toc_frames_per_entry: u16,
    /// Byte offset of the carrying frame's start.
    pub frame_offset: u64,
}

/// The offset just past the frame header + CRC + side information —
/// where the Xing/Info payload starts (MPEG-1 stereo: 36; mono: 21;
/// MPEG-2 stereo: 21; mono: 13).
fn ancillary_offset(header: &FrameHeader) -> usize {
    let channels = super::header::channel_count(header.channel_mode);
    4 + usize::from(header.protected_by_crc) * 2 + side_info_size(header.version, channels)
}

/// Parse the Xing/Info + LAME payload out of the FIRST frame of a
/// stream (`frame` = the whole frame's bytes at `frame_offset`).
///
/// Returns `Ok(None)` when the frame carries no Xing/Info header (a
/// plain CBR stream — legal, not damage). Returns a typed error when
/// a header IS present but its claims are impossible (frames=0, a
/// TOC contradicting monotonic file arithmetic is tolerated but
/// frame-count-vs-data inconsistency across the whole stream is
/// checked by the stream decoder, which knows the real walked count).
///
/// The scan accepts the magic at the spec offset or (tolerantly)
/// within the following 64 bytes — some encoders insert filler.
pub fn parse_xing(frame: &[u8], frame_offset: u64) -> Result<Option<XingInfo>, EngineError> {
    let Some(header) = parse_head(frame) else {
        return Err(corrupt(
            "mp3.xing.not_a_frame",
            CorruptDetail::InvalidStructure { offset: Some(0) },
            "the carrying frame's header does not parse",
        )
        .into_engine_error());
    };
    let base = ancillary_offset(&header);
    let Some(pos) = find_magic(frame, base, b"Xing").or_else(|| find_magic(frame, base, b"Info"))
    else {
        return Ok(None);
    };
    let rest = match frame.get(pos + 4..) {
        Some(r) => r,
        None => {
            return Err(corrupt(
                "mp3.xing.truncated",
                CorruptDetail::Truncated {
                    expected: u64::try_from(pos + 8).unwrap_or(u64::MAX),
                    found: frame.len() as u64,
                },
                "Xing magic without a flags word",
            )
            .into_engine_error());
        }
    };
    if rest.len() < 4 {
        return Err(corrupt(
            "mp3.xing.flags_truncated",
            CorruptDetail::Truncated {
                expected: 4,
                found: rest.len() as u64,
            },
            "Xing flags word truncated",
        )
        .into_engine_error());
    }
    let flags = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]);
    let mut cur = 4usize;
    let take = |cur: &mut usize, n: usize| -> Option<&[u8]> {
        let s = rest.get(*cur..*cur + n)?;
        *cur += n;
        Some(s)
    };
    let frames = if flags & 0x1 != 0 {
        match take(&mut cur, 4) {
            Some(b) => Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]])),
            None => {
                return Err(corrupt(
                    "mp3.xing.frames_truncated",
                    CorruptDetail::Truncated {
                        expected: u64::try_from(cur + 4).unwrap_or(u64::MAX),
                        found: rest.len() as u64,
                    },
                    "Xing frames field truncated",
                )
                .into_engine_error());
            }
        }
    } else {
        None
    };
    let bytes = if flags & 0x2 != 0 {
        match take(&mut cur, 4) {
            Some(b) => Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]])),
            None => {
                return Err(corrupt(
                    "mp3.xing.bytes_truncated",
                    CorruptDetail::Truncated {
                        expected: u64::try_from(cur + 4).unwrap_or(u64::MAX),
                        found: rest.len() as u64,
                    },
                    "Xing bytes field truncated",
                )
                .into_engine_error());
            }
        }
    } else {
        None
    };
    let toc = if flags & 0x4 != 0 {
        match take(&mut cur, 100) {
            Some(b) => {
                let mut t = [0u8; 100];
                t.copy_from_slice(b);
                Some(t)
            }
            None => {
                return Err(corrupt(
                    "mp3.xing.toc_truncated",
                    CorruptDetail::Truncated {
                        expected: u64::try_from(cur + 100).unwrap_or(u64::MAX),
                        found: rest.len() as u64,
                    },
                    "Xing TOC truncated",
                )
                .into_engine_error());
            }
        }
    } else {
        None
    };
    let quality = if flags & 0x8 != 0 {
        match take(&mut cur, 4) {
            Some(b) => Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]])),
            None => {
                return Err(corrupt(
                    "mp3.xing.quality_truncated",
                    CorruptDetail::Truncated {
                        expected: u64::try_from(cur + 4).unwrap_or(u64::MAX),
                        found: rest.len() as u64,
                    },
                    "Xing quality field truncated",
                )
                .into_engine_error());
            }
        }
    } else {
        None
    };
    if let Some(0) = frames {
        // A header that claims zero frames is inconsistent with the
        // frame that carries it.
        return Err(corrupt(
            "mp3.xing.zero_frames",
            CorruptDetail::InvalidStructure {
                offset: Some(frame_offset + pos as u64 + 8),
            },
            "Xing header claims 0 frames but is itself inside frame 1",
        )
        .into_engine_error());
    }
    // LAME tag: begins right after the Xing fields.
    let lame = parse_lame(rest.get(cur..).unwrap_or(&[]));
    Ok(Some(XingInfo {
        magic: [frame[pos], frame[pos + 1], frame[pos + 2], frame[pos + 3]],
        is_vbr: frame.get(pos..pos + 4) == Some(b"Xing"),
        frames,
        bytes,
        toc,
        quality,
        lame,
        frame_offset,
        frame_end: frame_offset + frame.len() as u64,
    }))
}

fn parse_head(frame: &[u8]) -> Option<FrameHeader> {
    super::header::parse_frame_header(frame).ok()
}

fn find_magic(frame: &[u8], base: usize, magic: &[u8; 4]) -> Option<usize> {
    let window = frame.get(base..(base + 64 + 4).min(frame.len()))?;
    window
        .windows(4)
        .position(|w| w == magic)
        .map(|p| base + p)
}

/// Parse the LAME tag (delay/padding) from the bytes following the
/// Xing fields. The tag must begin with `LAME` (or `Lavc` — ffmpeg's
/// libmp3lame wrapper writes the same structure under its own
/// identifier, verified empirically on ffmpeg 7.1-authored fixtures:
/// delay/padding at offsets 21..24, 12+12 bits, and
/// `frames × spf − delay − padding` equals ffmpeg's decoded sample
/// count exactly); the delay and padding sit at offsets 21..24.
fn parse_lame(tail: &[u8]) -> Option<LameTag> {
    let magic = tail.get(..4)?;
    if magic != b"LAME" && magic != b"Lavc" {
        return None;
    }
    let b = tail.get(21..24)?;
    Some(LameTag {
        delay: (u32::from(b[0]) << 4) | (u32::from(b[1]) >> 4),
        padding: ((u32::from(b[1]) & 0x0F) << 8) | u32::from(b[2]),
    })
}

/// Parse the VBRI header out of the FIRST frame of a stream
/// (`frame` = the whole frame at `frame_offset`).
///
/// VBRI sits at a FIXED offset: 32 bytes after the 4-byte header
/// (+CRC). `Ok(None)` when the magic is absent (the common case —
/// every non-Fraunhofer encoder writes Xing instead).
pub fn parse_vbri(frame: &[u8], frame_offset: u64) -> Result<Option<VbriInfo>, EngineError> {
    let Some(header) = parse_head(frame) else {
        return Err(corrupt(
            "mp3.vbri.not_a_frame",
            CorruptDetail::InvalidStructure { offset: Some(0) },
            "the carrying frame's header does not parse",
        )
        .into_engine_error());
    };
    let base = ancillary_offset(&header);
    // VBRI's fixed slot is 32 bytes after the frame header end; our
    // ancillary offset (side info) is the honest anchor — accept the
    // magic there.
    let Some(pos) = find_magic_vbri(frame, base) else {
        return Ok(None);
    };
    let rest = frame.get(pos + 4..).unwrap_or(&[]);
    if rest.len() < 14 {
        return Err(corrupt(
            "mp3.vbri.truncated",
            CorruptDetail::Truncated {
                expected: u64::try_from(pos + 18).unwrap_or(u64::MAX),
                found: frame.len() as u64,
            },
            "VBRI header shorter than its fixed fields",
        )
        .into_engine_error());
    }
    let version = u16::from_be_bytes([rest[0], rest[1]]);
    let delay = u16::from_be_bytes([rest[2], rest[3]]);
    let quality = u16::from_be_bytes([rest[4], rest[5]]);
    let bytes = u32::from_be_bytes([rest[6], rest[7], rest[8], rest[9]]);
    let frames = u32::from_be_bytes([rest[10], rest[11], rest[12], rest[13]]);
    if frames == 0 || bytes == 0 {
        return Err(corrupt(
            "mp3.vbri.zero_totals",
            CorruptDetail::InvalidStructure { offset: Some(0) },
            "VBRI header claims zero frames/bytes but is itself inside frame 1",
        )
        .into_engine_error());
    }
    let toc_entries = u16::from_be_bytes([rest[14], rest[15]]);
    let toc_scale = u16::from_be_bytes([rest[16], rest[17]]);
    let toc_entry_size = u16::from_be_bytes([rest[18], rest[19]]);
    let toc_frames_per_entry = u16::from_be_bytes([rest[20], rest[21]]);
    let toc_len = usize::from(toc_entries) * usize::from(toc_entry_size.max(1));
    let toc = match frame.get(pos + 4 + 22..pos + 4 + 22 + toc_len) {
        Some(t) => t.to_vec(),
        None => {
            return Err(corrupt(
                "mp3.vbri.toc_truncated",
                CorruptDetail::Truncated {
                    expected: u64::try_from(pos + 4 + 22 + toc_len).unwrap_or(u64::MAX),
                    found: frame.len() as u64,
                },
                "VBRI TOC runs past the frame",
            )
            .into_engine_error());
        }
    };
    if toc_scale != 1 {
        return Err(unsupported(
            "mp3.vbri.toc_scale",
            format!("VBRI TOC scale {toc_scale} != 1 is not supported"),
        )
        .into_engine_error());
    }
    Ok(Some(VbriInfo {
        version,
        delay,
        quality,
        bytes,
        frames,
        toc,
        toc_entry_size,
        toc_frames_per_entry,
        frame_offset,
    }))
}

fn find_magic_vbri(frame: &[u8], base: usize) -> Option<usize> {
    let window = frame.get(base..(base + 40).min(frame.len()))?;
    window.windows(4).position(|w| w == b"VBRI").map(|p| base + p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a first frame carrying a Xing header: a canonical
    /// MPEG-1 L3 128 kbps 44.1 kHz stereo header, side info sized
    /// 32, then the Xing payload.
    fn xing_frame(magic: &[u8; 4], flags: u32, extra: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        // 32 bytes side info (stereo MPEG-1) + padding to frame size
        let mut body = Vec::new();
        body.extend_from_slice(&[0u8; 32]);
        body.extend_from_slice(magic);
        body.extend_from_slice(&flags.to_be_bytes());
        body.extend_from_slice(extra);
        let header = match super::super::header::parse_frame_header(&f) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        f.extend_from_slice(&body);
        let need = header.frame_size as usize;
        f.resize(need, 0);
        f
    }

    fn lame_tag(delay: u32, padding: u32) -> Vec<u8> {
        let mut t = Vec::new();
        t.extend_from_slice(b"LAME3.100"); // magic + 5 version chars
        t.extend_from_slice(&[0; 12]); // rev/lowpass/replaygain/flags/bitrate
        let d = delay.min(0x0FFF);
        let p = padding.min(0x0FFF);
        // 12-bit delay | 12-bit padding packed into 3 bytes at 21..24.
        let packed: u32 = (d << 12) | p;
        t.extend_from_slice(&packed.to_be_bytes()[1..4]);
        t.extend_from_slice(&[0; 8]); // tail fields
        assert_eq!(t.len(), 32);
        t
    }

    #[test]
    fn parses_frames_bytes_toc_quality_and_lame() {
        let mut extra = Vec::new();
        extra.extend_from_slice(&100u32.to_be_bytes()); // frames
        extra.extend_from_slice(&41_700u32.to_be_bytes()); // bytes
        let mut toc = [0u8; 100];
        for (i, t) in toc.iter_mut().enumerate() {
            *t = ((i as u32 * 255) / 99) as u8;
        }
        extra.extend_from_slice(&toc);
        extra.extend_from_slice(&57u32.to_be_bytes()); // quality
        extra.extend_from_slice(&lame_tag(1105, 340));
        let frame = xing_frame(b"Xing", 0x000F, &extra);
        let x = match parse_xing(&frame, 0) {
            Ok(Some(x)) => x,
            other => unreachable!("{other:?}"),
        };
        assert!(x.is_vbr);
        assert_eq!(x.frames, Some(100));
        assert_eq!(x.bytes, Some(41_700));
        assert_eq!(x.toc.as_ref().map(|t| t[50]), Some(128));
        assert_eq!(x.quality, Some(57));
        let lame = match x.lame {
            Some(l) => l,
            None => unreachable!("LAME tag must parse"),
        };
        assert_eq!((lame.delay, lame.padding), (1105, 340));
        assert_eq!(lame.trim_window(115_200), (1105, 114_860));
        // padding larger than total clamps to the window start.
        assert_eq!(lame.trim_window(500), (1105, 1105));
    }

    #[test]
    fn info_magic_is_cbr_flagged() {
        let frame = xing_frame(b"Info", 0x0001, &50u32.to_be_bytes());
        let x = match parse_xing(&frame, 4096) {
            Ok(Some(x)) => x,
            other => unreachable!("{other:?}"),
        };
        assert!(!x.is_vbr);
        assert_eq!(x.frames, Some(50));
        assert_eq!(x.toc, None);
        assert_eq!(x.frame_offset, 4096);
    }

    #[test]
    fn no_xing_is_none_and_zero_frames_is_typed() {
        let frame = xing_frame(b"\0\0\0\0", 0, &[]);
        assert!(matches!(parse_xing(&frame, 0), Ok(None)));
        let frame = xing_frame(b"Xing", 0x0001, &0u32.to_be_bytes());
        let e = match parse_xing(&frame, 0) {
            Err(e) => e,
            Ok(_) => unreachable!("zero frames must fail"),
        };
        assert_eq!(e.context.code, "mp3.xing.zero_frames");
        // Truncated flags word: the frame ends 2 bytes into it.
        let mut f = xing_frame(b"Xing", 0, &[]);
        f.truncate(4 + 32 + 4 + 2);
        let e = match parse_xing(&f, 0) {
            Err(e) => e,
            Ok(_) => unreachable!("truncated fields must fail"),
        };
        assert_eq!(e.context.code, "mp3.xing.flags_truncated");
    }

    #[test]
    fn truncated_field_claims_are_typed() {
        // flags claim a TOC but the frame ends 50 bytes into it.
        let mut frame = xing_frame(b"Xing", 0x0004, &[]);
        frame.truncate(4 + 32 + 4 + 50);
        let e = match parse_xing(&frame, 0) {
            Err(e) => e,
            Ok(_) => unreachable!("missing TOC must fail"),
        };
        assert_eq!(e.context.code, "mp3.xing.toc_truncated");
    }

    #[test]
    fn hand_crafted_vbri_header_parses() {
        let mut f = Vec::new();
        f.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        f.extend_from_slice(&[0u8; 32]); // side info
        f.extend_from_slice(b"VBRI");
        f.extend_from_slice(&1u16.to_be_bytes()); // version
        f.extend_from_slice(&500u16.to_be_bytes()); // delay
        f.extend_from_slice(&100u16.to_be_bytes()); // quality
        f.extend_from_slice(&123_456u32.to_be_bytes()); // bytes
        f.extend_from_slice(&200u32.to_be_bytes()); // frames
        f.extend_from_slice(&2u16.to_be_bytes()); // toc entries
        f.extend_from_slice(&1u16.to_be_bytes()); // toc scale
        f.extend_from_slice(&2u16.to_be_bytes()); // entry size
        f.extend_from_slice(&100u16.to_be_bytes()); // frames/entry
        f.extend_from_slice(&0x1000u16.to_be_bytes()); // entry 0
        f.extend_from_slice(&0x8000u16.to_be_bytes()); // entry 1
        let v = match parse_vbri(&f, 0) {
            Ok(Some(v)) => v,
            other => unreachable!("{other:?}"),
        };
        assert_eq!((v.version, v.delay, v.quality), (1, 500, 100));
        assert_eq!((v.bytes, v.frames), (123_456, 200));
        assert_eq!(v.toc, [0x10, 0x00, 0x80, 0x00]);
        assert_eq!(v.toc_entry_size, 2);
        assert_eq!(v.toc_frames_per_entry, 100);
        // No magic → None.
        let plain = xing_frame(b"\0\0\0\0", 0, &[]);
        assert!(matches!(parse_vbri(&plain, 0), Ok(None)));
        // Zero totals → typed.
        let mut zero = f.clone();
        // bytes field at side+4+4+2+2+2 → zero frames at +18..+22.
        let off = 4 + 32 + 4 + 2 + 2 + 2 + 4;
        for b in zero.get_mut(off..off + 4).unwrap_or(&mut []) {
            *b = 0;
        }
        let e = match parse_vbri(&zero, 0) {
            Err(e) => e,
            Ok(_) => unreachable!("zero totals must fail"),
        };
        assert_eq!(e.context.code, "mp3.vbri.zero_totals");
        // Truncated TOC → typed.
        let mut cut = f.clone();
        cut.truncate(cut.len() - 2);
        let e = match parse_vbri(&cut, 0) {
            Err(e) => e,
            Ok(_) => unreachable!("cut TOC must fail"),
        };
        assert_eq!(e.context.code, "mp3.vbri.toc_truncated");
    }
}
