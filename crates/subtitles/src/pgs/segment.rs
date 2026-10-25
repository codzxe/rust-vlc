//! PGS (HDMV, Blu-ray) segment grammar — the segment walk and the
//! typed payloads (card 5.N1, ADR-012 self-build).
//!
//! Grammar verified against the public reference decoder behavior
//! (ffmpeg `libavcodec/pgssubdec.c`, fetched 2026-10-25 for this
//! card; segment types 0x14/0x15/0x16/0x17/0x80):
//!
//! ```text
//! segment := PTS:u32be | type:u8 | size:u16be | payload[size]   (.sup)
//! segment :=              type:u8 | size:u16be | payload[size]   (Matroska-stripped)
//! ```
//!
//! Matroska `S_HDMV/PGS` blocks carry the segments WITHOUT the
//! 4-byte PTS prefix (mkvtoolnix strips it); the standalone `.sup`
//! and the MPEG-TS PES carriage keep it. [`PgsFraming::detect`]
//! picks deterministically (whole-buffer consumption scores).
//!
//! Payload grammars (all big-endian):
//!
//! * **PCS (0x16):** video width/height `u16×2`, frame-rate byte
//!   (upper nibble = HDMV rate id), composition number `u16`,
//!   composition state byte (`>> 6`: 0 normal, 1 acquisition point,
//!   2 epoch start, 3 epoch continue), palette-update flag byte
//!   (`0x80`), palette id, object count, then `count ×` object refs:
//!   id `u16`, window id `u8`, composition flags `u8` (`0x80` crop,
//!   `0x40` forced), x `u16`, y `u16`, plus 2×`u16×4` crop box when
//!   the crop flag is set.
//! * **WDS (0x17):** window count `u8`, then `count ×` (id `u8`,
//!   x `u16`, y `u16`, width `u16`, height `u16`).
//! * **PDS (0x14):** palette id `u8`, palette version `u8`, then
//!   5-byte entries (entry id, Y, Cr, Cb, alpha) to the payload end.
//! * **ODS (0x15):** object id `u16`, version `u8`, sequence
//!   descriptor `u8` (`0x80` first fragment, `0x40` last); a first
//!   fragment carries the object header: total data length `u24`
//!   (width+height included), width `u16`, height `u16`, then the
//!   RLE fragment. Later fragments append raw RLE bytes.
//! * **END (0x80):** empty — closes the display set.

/// How segments are framed in the payload handed to the decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgsFraming {
    /// Standalone `.sup` / TS-PES: each segment carries a 4-byte
    /// big-endian 90 kHz PTS before the type byte.
    Sup,
    /// Matroska `S_HDMV/PGS`: the PTS prefix is stripped.
    Stripped,
}

impl PgsFraming {
    /// Deterministically pick a framing for `data`: the framing whose
    /// segment walk consumes the whole buffer exactly, preferring
    /// `Stripped` on ties (Matroska is the dominant in-tree carriage).
    /// An empty/unwalkable buffer reports `Stripped` (zero segments —
    /// the typed truncation path fires either way).
    #[must_use = "the walker needs a framing to walk with"]
    pub fn detect(data: &[u8]) -> PgsFraming {
        let (_, used_sup) = walk_segments(data, PgsFraming::Sup);
        let (_, used_stripped) = walk_segments(data, PgsFraming::Stripped);
        match (used_sup == data.len(), used_stripped == data.len()) {
            (true, false) => PgsFraming::Sup,
            _ => PgsFraming::Stripped,
        }
    }

    /// The header size for this framing (7 with PTS, 3 without).
    pub fn header_len(self) -> usize {
        match self {
            PgsFraming::Sup => 7,
            PgsFraming::Stripped => 3,
        }
    }
}

/// Segment type byte vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    /// 0x14 — palette definition.
    Pds,
    /// 0x15 — object definition (RLE fragment).
    Ods,
    /// 0x16 — presentation composition.
    Pcs,
    /// 0x17 — window definition.
    Wds,
    /// 0x80 — end of display set.
    End,
    /// Any other byte (skipped, typed upstream).
    Unknown(u8),
}

impl SegmentKind {
    fn from_type_byte(b: u8) -> SegmentKind {
        match b {
            0x14 => SegmentKind::Pds,
            0x15 => SegmentKind::Ods,
            0x16 => SegmentKind::Pcs,
            0x17 => SegmentKind::Wds,
            0x80 => SegmentKind::End,
            other => SegmentKind::Unknown(other),
        }
    }
}

/// One raw segment: type + payload slice + its own PTS when the
/// framing carried one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawSegment<'a> {
    pub kind: SegmentKind,
    /// The segment's own PTS in 90 kHz ticks (`.sup` framing only).
    pub pts_90khz: Option<u32>,
    pub data: &'a [u8],
}

/// Walk the segments of a payload. Returns `(segments, bytes_used)`;
/// the walk stops at the first segment whose declared size overruns
/// the remaining bytes (the reference behavior — except `END`, whose
/// declared size is allowed to overrun, per the reference decoder).
pub fn walk_segments<'a>(data: &'a [u8], framing: PgsFraming) -> (Vec<RawSegment<'a>>, usize) {
    let mut segments = Vec::new();
    let mut pos = 0usize;
    let hdr = framing.header_len();
    while data.len() - pos >= hdr {
        let (pts, type_off) = match framing {
            PgsFraming::Sup => (
                Some(u32::from_be_bytes([
                    data[pos],
                    data[pos + 1],
                    data[pos + 2],
                    data[pos + 3],
                ])),
                pos + 4,
            ),
            PgsFraming::Stripped => (None, pos),
        };
        let kind = SegmentKind::from_type_byte(data[type_off]);
        let size = u16::from_be_bytes([data[type_off + 1], data[type_off + 2]]) as usize;
        let body = type_off + 3;
        if body + size > data.len() {
            // END may overrun its declared size (observed in the
            // wild; the reference allows it). Every other kind stops
            // the walk — the caller reports the truncation.
            if kind == SegmentKind::End {
                segments.push(RawSegment {
                    kind,
                    pts_90khz: pts,
                    data: &data[body..],
                });
                pos = data.len();
            }
            break;
        }
        segments.push(RawSegment {
            kind,
            pts_90khz: pts,
            data: &data[body..body + size],
        });
        pos = body + size;
    }
    (segments, pos)
}

/// HDMV frame-rate ids (PCS byte >> 4) — the STC/rate table shared
/// with the stream descriptors (0 = unspecified).
#[must_use = "callers render the id into records"]
pub fn frame_rate_id_table(id: u8) -> Option<(u32, u32)> {
    match id {
        1 => Some((24_000, 1001)),
        2 => Some((24, 1)),
        3 => Some((25, 1)),
        4 => Some((30_000, 1001)),
        5 => Some((30, 1)),
        6 => Some((50, 1)),
        7 => Some((60_000, 1001)),
        8 => Some((60, 1)),
        _ => None,
    }
}

/// One PCS composition-object reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositionObject {
    pub id: u16,
    pub window_id: u8,
    /// `0x40` — forced-on flag (burned-in/forced subtitles).
    pub forced: bool,
    pub x: u16,
    pub y: u16,
    /// The crop box when the PCS crop flag (`0x80`) was set.
    pub crop: Option<(u16, u16, u16, u16)>,
}

/// Composition state (PCS state byte `>> 6`) — the epoch machine's
/// input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionState {
    /// 0 — continue within the epoch; objects and palettes stay.
    Normal,
    /// 1 — acquisition point (a safe re-join of the stream).
    AcquisitionPoint,
    /// 2 — epoch start (a new composition from scratch).
    EpochStart,
    /// 3 — epoch continue (rare; the reference flushes too).
    EpochContinue,
}

impl CompositionState {
    fn from_byte(b: u8) -> CompositionState {
        match b >> 6 {
            0 => CompositionState::Normal,
            1 => CompositionState::AcquisitionPoint,
            2 => CompositionState::EpochStart,
            _ => CompositionState::EpochContinue,
        }
    }

    /// Does this state release the epoch's objects and palettes?
    /// (Every non-zero state does, per the reference decoder.)
    pub fn flushes_epoch(self) -> bool {
        !matches!(self, CompositionState::Normal)
    }
}

/// A parsed PCS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcs {
    pub width: u16,
    pub height: u16,
    /// The frame-rate nibble (see [`frame_rate_id_table`]).
    pub frame_rate_id: u8,
    pub composition_number: u16,
    pub state: CompositionState,
    pub palette_update: bool,
    pub palette_id: u8,
    pub objects: Vec<CompositionObject>,
}

/// Parse a PCS payload. `None` when truncated below the 11-byte
/// fixed part or an object ref is short (typed upstream).
pub fn parse_pcs(data: &[u8]) -> Option<Pcs> {
    if data.len() < 11 {
        return None;
    }
    let width = u16::from_be_bytes([data[0], data[1]]);
    let height = u16::from_be_bytes([data[2], data[3]]);
    let frame_rate_id = data[4] >> 4;
    let composition_number = u16::from_be_bytes([data[5], data[6]]);
    let state = CompositionState::from_byte(data[7]);
    let palette_update = data[8] & 0x80 != 0;
    let palette_id = data[9];
    let count = data[10] as usize;
    let mut objects = Vec::with_capacity(count.min(64));
    let mut p = 11usize;
    for _ in 0..count {
        if data.len() - p < 8 {
            return None;
        }
        let id = u16::from_be_bytes([data[p], data[p + 1]]);
        let window_id = data[p + 2];
        let flags = data[p + 3];
        let x = u16::from_be_bytes([data[p + 4], data[p + 5]]);
        let y = u16::from_be_bytes([data[p + 6], data[p + 7]]);
        p += 8;
        let crop = if flags & 0x80 != 0 {
            if data.len() - p < 8 {
                return None;
            }
            let c = (
                u16::from_be_bytes([data[p], data[p + 1]]),
                u16::from_be_bytes([data[p + 2], data[p + 3]]),
                u16::from_be_bytes([data[p + 4], data[p + 5]]),
                u16::from_be_bytes([data[p + 6], data[p + 7]]),
            );
            p += 8;
            Some(c)
        } else {
            None
        };
        objects.push(CompositionObject {
            id,
            window_id,
            forced: flags & 0x40 != 0,
            x,
            y,
            crop,
        });
    }
    Some(Pcs {
        width,
        height,
        frame_rate_id,
        composition_number,
        state,
        palette_update,
        palette_id,
        objects,
    })
}

/// One window (WDS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Parse a WDS payload into its windows (short tails are dropped by
/// the caller's truncation accounting — the fixed part is 1 byte).
pub fn parse_wds(data: &[u8]) -> Vec<Window> {
    let Some(&count) = data.first() else {
        return Vec::new();
    };
    let count = count as usize;
    let mut windows = Vec::new();
    let mut p = 1usize;
    for _ in 0..count {
        if data.len() - p < 9 {
            break;
        }
        let id = data[p];
        let x = u16::from_be_bytes([data[p + 1], data[p + 2]]);
        let y = u16::from_be_bytes([data[p + 3], data[p + 4]]);
        let width = u16::from_be_bytes([data[p + 5], data[p + 6]]);
        let height = u16::from_be_bytes([data[p + 7], data[p + 8]]);
        p += 9;
        windows.push(Window {
            id,
            x,
            y,
            width,
            height,
        });
    }
    windows
}

/// One PDS palette entry (pre-conversion; byte order Y, Cr, Cb).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteEntry {
    pub id: u8,
    pub y: u8,
    pub cr: u8,
    pub cb: u8,
    pub alpha: u8,
}

/// A parsed PDS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pds {
    pub id: u8,
    pub version: u8,
    /// 5-byte entries — a short trailing entry is dropped (typed
    /// upstream via the entry accounting).
    pub entries: Vec<PaletteEntry>,
}

/// Parse a PDS payload.
pub fn parse_pds(data: &[u8]) -> Option<Pds> {
    if data.len() < 2 {
        return None;
    }
    let id = data[0];
    let version = data[1];
    let mut entries = Vec::new();
    let mut p = 2usize;
    while data.len() - p >= 5 {
        entries.push(PaletteEntry {
            id: data[p],
            y: data[p + 1],
            cr: data[p + 2],
            cb: data[p + 3],
            alpha: data[p + 4],
        });
        p += 5;
    }
    Some(Pds {
        id,
        version,
        entries,
    })
}

/// A parsed ODS fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ods {
    pub id: u16,
    pub version: u8,
    /// The sequence descriptor: first (`0x80`) / last (`0x40`).
    pub first: bool,
    pub last: bool,
    /// The object header, first fragments only: (total RLE length
    /// INCLUDING the 4 header bytes, width, height).
    pub header: Option<(u32, u16, u16)>,
    /// The raw RLE fragment bytes.
    pub fragment: Vec<u8>,
}

/// Parse an ODS payload. `None` when truncated below the 4-byte
/// fixed part or a first-fragment header is short.
pub fn parse_ods(data: &[u8]) -> Option<Ods> {
    if data.len() < 4 {
        return None;
    }
    let id = u16::from_be_bytes([data[0], data[1]]);
    let version = data[2];
    let desc = data[3];
    let first = desc & 0x80 != 0;
    let last = desc & 0x40 != 0;
    let mut p = 4usize;
    let mut header = None;
    if first {
        if data.len() - p < 7 {
            return None;
        }
        let total = u32::from(data[p]) << 16 | u32::from(data[p + 1]) << 8 | u32::from(data[p + 2]);
        let width = u16::from_be_bytes([data[p + 3], data[p + 4]]);
        let height = u16::from_be_bytes([data[p + 5], data[p + 6]]);
        p += 7;
        header = Some((total, width, height));
    }
    Some(Ods {
        id,
        version,
        first,
        last,
        header,
        fragment: data[p..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    fn sup_seg(pts: u32, kind: u8, data: &[u8]) -> Vec<u8> {
        let mut v = pts.to_be_bytes().to_vec();
        v.push(kind);
        v.extend_from_slice(&(data.len() as u16).to_be_bytes());
        v.extend_from_slice(data);
        v
    }

    #[test]
    fn walk_sup_framing_including_pts() {
        let buf = [sup_seg(90_000, 0x16, &[0; 11]), sup_seg(90_001, 0x80, &[])].concat();
        let (segs, used) = walk_segments(&buf, PgsFraming::Sup);
        assert_eq!(used, buf.len());
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].kind, SegmentKind::Pcs);
        assert_eq!(segs[0].pts_90khz, Some(90_000));
        assert_eq!(segs[1].kind, SegmentKind::End);
        assert_eq!(segs[1].data.len(), 0);
    }

    #[test]
    fn walk_strips_and_detects_both_framings() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0x16, 0x00, 0x0B]);
        buf.extend_from_slice(&[0; 11]);
        buf.extend_from_slice(&[0x14, 0x00, 0x02, 0x00, 0x00]);
        let (segs, used) = walk_segments(&buf, PgsFraming::Stripped);
        assert_eq!(used, buf.len());
        assert_eq!(segs.len(), 2);
        assert!(segs.iter().all(|s| s.pts_90khz.is_none()));
        assert_eq!(PgsFraming::detect(&buf), PgsFraming::Stripped);
        // .sup form: the same bytes with PTS prefixes.
        let sup = [sup_seg(0, 0x16, &[0; 11]), sup_seg(0, 0x14, &[0x00, 0x00])].concat();
        assert_eq!(PgsFraming::detect(&sup), PgsFraming::Sup);
    }

    #[test]
    fn walk_stops_on_overrun_except_end() {
        // Declares 100 bytes of PCS but only 5 follow.
        let buf = [0x16, 0x00, 0x64, 1, 2, 3, 4, 5];
        let (segs, used) = walk_segments(&buf, PgsFraming::Stripped);
        assert!(segs.is_empty());
        assert_eq!(used, 0);
        // END overrunning its declared size still fires.
        let buf = [0x80, 0x00, 0x64, 1, 2, 3];
        let (segs, used) = walk_segments(&buf, PgsFraming::Stripped);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].kind, SegmentKind::End);
        assert_eq!(used, buf.len());
    }

    #[test]
    fn pcs_parses_full_grammar() {
        let mut d = vec![
            0x07, 0x80, 0x04, 0x38, 0x20, 0x00, 0x2A, 0x80, 0x00, 0x03, 0x01,
        ];
        // One object: id 0x0001, window 0, flags 0xC0 (crop+forced),
        // x 0x0100, y 0x0200, crop (2,3,16,8).
        d.extend_from_slice(&[0x00, 0x01, 0x00, 0xC0, 0x01, 0x00, 0x02, 0x00]);
        d.extend_from_slice(&[0x00, 0x02, 0x00, 0x03, 0x00, 0x10, 0x00, 0x08]);
        let pcs = parse_pcs(&d).must();
        assert_eq!((pcs.width, pcs.height), (1920, 1080));
        assert_eq!(pcs.frame_rate_id, 2);
        assert_eq!(pcs.composition_number, 42);
        assert_eq!(pcs.state, CompositionState::EpochStart);
        assert!(!pcs.palette_update);
        assert_eq!(pcs.palette_id, 3);
        assert_eq!(pcs.objects.len(), 1);
        let o = pcs.objects[0];
        assert_eq!(o.id, 1);
        assert!(o.forced);
        assert_eq!((o.x, o.y), (256, 512));
        assert_eq!(o.crop, Some((2, 3, 16, 8)));
        assert!(pcs.state.flushes_epoch());
        assert!(!CompositionState::Normal.flushes_epoch());
    }

    #[test]
    fn pcs_short_object_ref_is_none() {
        let mut d = vec![0; 11];
        d[10] = 1; // one object, but no bytes follow.
        assert!(parse_pcs(&d).is_none());
        assert!(parse_pcs(&d[..10]).is_none());
    }

    #[test]
    fn wds_parses_and_drops_short_tail() {
        let mut d = vec![2];
        d.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x01]); // win 0: (256,0,512,1)
        d.extend_from_slice(&[1, 0, 0, 0x00, 0x64, 0x00, 0x20, 0x00]); // short by 1
        let w = parse_wds(&d);
        assert_eq!(w.len(), 1);
        assert_eq!((w[0].x, w[0].y, w[0].width, w[0].height), (256, 0, 512, 1));
        assert!(parse_wds(&[]).is_empty());
    }

    #[test]
    fn pds_entry_order_is_y_cr_cb() {
        let d = [0x02, 0x00, 0x05, 0x41, 0x10, 0x20, 0x30, 0xFF, 0x99];
        let pds = parse_pds(&d).must();
        assert_eq!(pds.id, 2);
        assert_eq!(pds.entries.len(), 1);
        let e = pds.entries[0];
        assert_eq!(
            (e.id, e.y, e.cr, e.cb, e.alpha),
            (0x05, 0x41, 0x10, 0x20, 0x30)
        );
        // The short trailing byte (0x99) is dropped, not panicked on.
        assert!(parse_pds(&[0x02]).is_none());
    }

    #[test]
    fn ods_first_and_continuation_parse() {
        // First fragment: id 1, version 2, desc 0x80, total 16,
        // 4x3 bitmap, 5 bytes of RLE.
        let mut d = vec![
            0x00, 0x01, 0x02, 0x80, 0x00, 0x00, 0x10, 0x00, 0x04, 0x00, 0x03,
        ];
        d.extend_from_slice(&[0xC0, 0x07, 0x41, 0x00, 0x8F]);
        let ods = parse_ods(&d).must();
        assert_eq!(ods.id, 1);
        assert_eq!(ods.version, 2);
        assert!(ods.first);
        assert!(!ods.last);
        assert_eq!(ods.header, Some((16, 4, 3)));
        assert_eq!(ods.fragment, [0xC0, 0x07, 0x41, 0x00, 0x8F]);
        // Continuation fragment: desc 0x40 (last), 2 bytes.
        let d = [0x00, 0x01, 0x02, 0x40, 0x12, 0x34];
        let ods = parse_ods(&d).must();
        assert!(!ods.first);
        assert!(ods.last);
        assert_eq!(ods.header, None);
        assert_eq!(ods.fragment, [0x12, 0x34]);
        assert!(parse_ods(&[0x00, 0x01, 0x02, 0x80]).is_none());
    }

    #[test]
    fn frame_rate_table_covers_the_hdmv_ids() {
        assert_eq!(frame_rate_id_table(1), Some((24_000, 1001)));
        assert_eq!(frame_rate_id_table(3), Some((25, 1)));
        assert_eq!(frame_rate_id_table(8), Some((60, 1)));
        assert_eq!(frame_rate_id_table(0), None);
        assert_eq!(frame_rate_id_table(9), None);
    }
}
