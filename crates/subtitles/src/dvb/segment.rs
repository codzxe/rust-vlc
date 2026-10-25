//! DVB segment grammar (ETSI EN 300 743 §7.2): the 6-byte header
//! walk and the typed payloads. Borrowed views — the page state
//! machine in [`crate::dvb`] copies what it keeps.
//!
//! Bit layouts follow the reference decoders (ffmpeg `dvbsubdec`,
//! this wave's differential oracle) exactly: page region entries are
//! 6 bytes (`id`, reserved, x:16, y:16), region object entries are 6
//! bytes (+2 for character-coded objects) with the object type in
//! the top 2 bits of byte 2 and 12-bit x/y packed around it.

/// Segment type codes (EN 300 743 table 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SegmentKind {
    PageComposition,
    RegionComposition,
    ClutDefinition,
    ObjectData,
    DisplayDefinition,
    EndOfDisplaySet,
    Stuffing,
    /// Reserved/unknown type byte — skipped, typed damage.
    Unknown(u8),
}

impl SegmentKind {
    fn of(type_byte: u8) -> SegmentKind {
        match type_byte {
            0x10 => SegmentKind::PageComposition,
            0x11 => SegmentKind::RegionComposition,
            0x12 => SegmentKind::ClutDefinition,
            0x13 => SegmentKind::ObjectData,
            0x14 => SegmentKind::DisplayDefinition,
            0x80 => SegmentKind::EndOfDisplaySet,
            0xFF => SegmentKind::Stuffing,
            other => SegmentKind::Unknown(other),
        }
    }
}

/// One raw segment: kind + page id + payload slice.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawSegment<'a> {
    pub kind: SegmentKind,
    pub page_id: u16,
    pub data: &'a [u8],
}

/// Walk the segments of one subtitle PES payload. Returns every
/// well-formed segment and the consumed byte count; a truncated
/// header or length stops the walk (the caller records the damage).
pub(crate) fn walk_segments(payload: &[u8]) -> (Vec<RawSegment<'_>>, usize) {
    let mut out = Vec::new();
    let mut p = 0usize;
    while p + 6 <= payload.len() {
        if payload[p] != 0x0F {
            break;
        }
        let kind = SegmentKind::of(payload[p + 1]);
        let page_id = u16::from_be_bytes([payload[p + 2], payload[p + 3]]);
        let len = u16::from_be_bytes([payload[p + 4], payload[p + 5]]) as usize;
        let data_start = p + 6;
        if data_start + len > payload.len() {
            break;
        }
        out.push(RawSegment {
            kind,
            page_id,
            data: &payload[data_start..data_start + len],
        });
        p = data_start + len;
    }
    (out, p)
}

/// Page composition payload (§7.2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageSegment<'a> {
    /// `page_time_out` in seconds.
    pub timeout_s: u8,
    pub version: u8,
    /// 0 = normal case, 1 = acquisition point, 2 = mode change.
    pub state: u8,
    pub regions: &'a [u8],
}

pub(crate) fn parse_page(data: &[u8]) -> Option<PageSegment<'_>> {
    if data.len() < 2 {
        return None;
    }
    Some(PageSegment {
        timeout_s: data[0],
        version: data[1] >> 4,
        state: (data[1] >> 2) & 0x3,
        regions: &data[2..],
    })
}

/// One region placement from the page composition (6 bytes: id,
/// reserved, x, y as 16-bit BE reads — the 10-bit addresses sit
/// right-aligned in them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageRegionRef {
    pub region_id: u8,
    pub x: u16,
    pub y: u16,
}

impl PageSegment<'_> {
    /// The region placements. A trailing partial entry (< 6 bytes)
    /// is dropped here and length-accounted by the damage ledger.
    pub fn region_refs(&self) -> impl Iterator<Item = PageRegionRef> + '_ {
        self.regions.chunks_exact(6).map(|e| PageRegionRef {
            region_id: e[0],
            x: u16::from_be_bytes([e[2], e[3]]),
            y: u16::from_be_bytes([e[4], e[5]]),
        })
    }
}

/// Region composition payload (§7.2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegionSegment<'a> {
    pub region_id: u8,
    pub version: u8,
    pub fill: bool,
    pub width: u16,
    pub height: u16,
    /// 2, 4 or 8 — `1 << region_depth`; invalid codes fall back to 4
    /// (the caller records the damage).
    pub depth_code: u8,
    pub clut_id: u8,
    /// Fill color at the region depth (0 when absent for 8-bit).
    pub fill_color: u8,
    pub objects: &'a [u8],
}

impl RegionSegment<'_> {
    /// The usable depth in bits (2/4/8) after the validity rule.
    pub fn depth(&self) -> u8 {
        let depth = 1u16 << (self.depth_code & 7);
        if (2..=8).contains(&depth) {
            depth as u8
        } else {
            4
        }
    }
}

pub(crate) fn parse_region(data: &[u8]) -> Option<RegionSegment<'_>> {
    if data.len() < 10 {
        return None;
    }
    let depth_code = (data[6] >> 2) & 7;
    let depth = 1u16 << (depth_code & 7);
    let (fill_color, objects) = if depth == 8 {
        (data[8], &data[10..])
    } else if depth == 4 {
        (data[9] >> 4, &data[10..])
    } else {
        ((data[9] >> 2) & 3, &data[10..])
    };
    Some(RegionSegment {
        region_id: data[0],
        version: data[1] >> 4,
        fill: (data[1] >> 3) & 1 == 1,
        width: u16::from_be_bytes([data[2], data[3]]),
        height: u16::from_be_bytes([data[4], data[5]]),
        depth_code,
        clut_id: data[7],
        fill_color,
        objects,
    })
}

/// One object placement inside a region (6 bytes, +2 for
/// character-coded objects).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RegionObjectRef {
    pub object_id: u16,
    /// EN 300 743 object_type: 0 = basic (bitmap), 1/2 = character
    /// coded (fg/bg colors follow), 3 = reserved.
    pub object_type: u8,
    pub x: u16,
    pub y: u16,
    pub fg_color: Option<u8>,
    pub bg_color: Option<u8>,
}

impl RegionSegment<'_> {
    /// Object placements: byte 2 carries the type in its top 2 bits
    /// and the x high nibble in its low 4 (12-bit x/y overall).
    pub fn object_refs(&self) -> impl Iterator<Item = RegionObjectRef> + '_ {
        let mut objs = self.objects;
        let mut out = Vec::new();
        while objs.len() >= 6 {
            let object_id = u16::from_be_bytes([objs[0], objs[1]]);
            let object_type = objs[2] >> 6;
            let x = ((u16::from(objs[2]) & 0x0F) << 8) | u16::from(objs[3]);
            let y = ((u16::from(objs[4]) & 0x0F) << 8) | u16::from(objs[5]);
            let (fg, bg, size) = if object_type == 1 || object_type == 2 {
                if objs.len() < 8 {
                    break;
                }
                (Some(objs[6]), Some(objs[7]), 8)
            } else {
                (None, None, 6)
            };
            out.push(RegionObjectRef {
                object_id,
                object_type,
                x,
                y,
                fg_color: fg,
                bg_color: bg,
            });
            objs = &objs[size..];
        }
        out.into_iter()
    }
}

/// CLUT definition payload (§7.2.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClutSegment<'a> {
    pub clut_id: u8,
    pub version: u8,
    pub entries: &'a [u8],
}

pub(crate) fn parse_clut(data: &[u8]) -> Option<ClutSegment<'_>> {
    if data.len() < 2 {
        return None;
    }
    Some(ClutSegment {
        clut_id: data[0],
        version: data[1] >> 4,
        entries: &data[2..],
    })
}

impl ClutSegment<'_> {
    /// Typed entries. Entries with a zero depth-flags byte stop the
    /// walk (invalid per EN 300 743 — the reference logs and skips
    /// the rest); truncated entries stop it too.
    pub fn entries(&self) -> impl Iterator<Item = crate::dvb::clut::ClutEntry> + '_ {
        let mut rest = self.entries;
        let mut out = Vec::new();
        while rest.len() >= 2 {
            let entry_id = rest[0];
            let flags = rest[1];
            let depth_2 = flags & 0x80 != 0;
            let depth_4 = flags & 0x40 != 0;
            let depth_8 = flags & 0x20 != 0;
            let full_range = flags & 1 == 1;
            if !(depth_2 || depth_4 || depth_8) {
                break;
            }
            let size = if full_range { 6 } else { 5 };
            if rest.len() < size {
                break;
            }
            let e = if full_range {
                crate::dvb::clut::ClutEntry {
                    entry_id,
                    depth_2,
                    depth_4,
                    depth_8,
                    full_range,
                    y: rest[2],
                    cr: rest[3],
                    cb: rest[4],
                    alpha: rest[5],
                }
            } else {
                // Limited range: Y(6) Cr(6) Cb(6) T(2) packed MSB-first
                // over 3 bytes (high-aligned into 8-bit components,
                // 4 trailing padding bits).
                let b0 = rest[2];
                let b1 = rest[3];
                let b2 = rest[4];
                crate::dvb::clut::ClutEntry {
                    entry_id,
                    depth_2,
                    depth_4,
                    depth_8,
                    full_range,
                    y: b0 & 0xFC,
                    cr: (((b0 & 0x03) << 4) | (b1 >> 4)) << 2,
                    cb: (((b1 & 0x0F) << 2) | (b2 >> 6)) << 2,
                    alpha: ((b2 >> 4) & 0x03) << 6,
                }
            };
            out.push(e);
            rest = &rest[size..];
        }
        out.into_iter()
    }
}

/// Object data payload (§7.2.5): id, coding method, and the two
/// field-length-prefixed pixel-data blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ObjectSegment<'a> {
    pub object_id: u16,
    /// 0 = pixels (RLE), 1 = characters, 2 = progressive pixels,
    /// 3 = reserved.
    pub coding_method: u8,
    pub non_modifying: bool,
    pub top_field: &'a [u8],
    pub bottom_field: &'a [u8],
}

pub(crate) fn parse_object(data: &[u8]) -> Option<ObjectSegment<'_>> {
    if data.len() < 3 {
        return None;
    }
    let object_id = u16::from_be_bytes([data[0], data[1]]);
    let coding_method = (data[2] >> 2) & 3;
    let non_modifying = (data[2] >> 1) & 1 == 1;
    if coding_method != 0 {
        return Some(ObjectSegment {
            object_id,
            coding_method,
            non_modifying,
            top_field: &[],
            bottom_field: &[],
        });
    }
    if data.len() < 7 {
        return None;
    }
    let top_len = u16::from_be_bytes([data[3], data[4]]) as usize;
    let bottom_len = u16::from_be_bytes([data[5], data[6]]) as usize;
    let rest = &data[7..];
    if top_len + bottom_len > rest.len() {
        return None;
    }
    Some(ObjectSegment {
        object_id,
        coding_method,
        non_modifying,
        top_field: &rest[..top_len],
        bottom_field: &rest[top_len..top_len + bottom_len],
    })
}

/// Display definition payload (§7.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisplayDefSegment {
    pub version: u8,
    pub display_window: bool,
    /// The display width/height (max+1), or the window extent when
    /// `display_window` is set.
    pub width: u16,
    pub height: u16,
    pub x: u16,
    pub y: u16,
}

pub(crate) fn parse_display_def(data: &[u8]) -> Option<DisplayDefSegment> {
    if data.len() < 5 {
        return None;
    }
    let info = data[0];
    let version = info >> 4;
    let display_window = info & 0x08 != 0;
    let width = u16::from_be_bytes([data[1], data[2]]) + 1;
    let height = u16::from_be_bytes([data[3], data[4]]) + 1;
    let (x, y, w, h) = if display_window {
        if data.len() < 13 {
            return None;
        }
        let x = u16::from_be_bytes([data[5], data[6]]);
        let wx = u16::from_be_bytes([data[7], data[8]]);
        let y = u16::from_be_bytes([data[9], data[10]]);
        let wy = u16::from_be_bytes([data[11], data[12]]);
        (x, y, wx - x + 1, wy - y + 1)
    } else {
        (0, 0, width, height)
    };
    Some(DisplayDefSegment {
        version,
        display_window,
        width: w,
        height: h,
        x,
        y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_walk_and_truncation() {
        // Two segments: a 2-byte stuffing and a 1-byte unknown, then
        // a third whose length overruns — the walk stops, typed.
        let payload = [
            0x0F, 0xFF, 0x00, 0x01, 0x00, 0x02, 0xAA, 0xBB, //
            0x0F, 0x77, 0x00, 0x01, 0x00, 0x01, 0xCC, //
            0x0F, 0x80, 0x00, 0x01, 0x00, 0x05, 0x01,
        ];
        let (segs, used) = walk_segments(&payload);
        assert_eq!(used, 15, "truncated third segment not consumed");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].kind, SegmentKind::Stuffing);
        assert_eq!(segs[0].data, &[0xAA, 0xBB]);
        assert_eq!(segs[1].kind, SegmentKind::Unknown(0x77));
        assert_eq!(segs[1].page_id, 1);
    }

    #[test]
    fn bad_sync_stops_walk() {
        let (segs, used) = walk_segments(&[0x00, 0x0F, 0x10, 0, 0, 0, 0]);
        assert!(segs.is_empty());
        assert_eq!(used, 0);
    }

    #[test]
    fn page_region_refs_parse() {
        let mut data = vec![10, 0x54]; // timeout 10s, version 5, state 1 (acquisition)
        for (id, x, y) in [(1u8, 100u16, 200u16), (2, 720, 576)] {
            data.push(id);
            data.push(0); // reserved
            data.extend_from_slice(&x.to_be_bytes());
            data.extend_from_slice(&y.to_be_bytes());
        }
        data.extend_from_slice(&[0x99, 0x00, 0x00]); // trailing partial entry
        let page = parse_page(&data).unwrap();
        assert_eq!((page.timeout_s, page.version, page.state), (10, 5, 1));
        let refs: Vec<_> = page.region_refs().collect();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], PageRegionRef { region_id: 1, x: 100, y: 200 });
        assert_eq!(refs[1], PageRegionRef { region_id: 2, x: 720, y: 576 });
    }

    #[test]
    fn region_parse_4bit() {
        // id 1, version 2, fill 1, 16x8, depth code 1 (=> 2 bits),
        // clut 3, codes for 8/4/2-bit fills, then one 6-byte object.
        let data = [
            0x01, 0x2A, 0x00, 0x10, 0x00, 0x08, 0b01_001_00, 0x03, 0x00, 0xF4, //
            0x00, 0x07, 0x00, 0x02, 0x00, 0x03,
        ];
        let r = parse_region(&data).unwrap();
        assert_eq!(r.region_id, 1);
        assert_eq!(r.version, 2);
        assert!(r.fill);
        assert_eq!((r.width, r.height), (16, 8));
        assert_eq!(r.depth(), 2);
        assert_eq!(r.clut_id, 3);
        assert_eq!(r.fill_color, (0xF4 >> 2) & 3);
        let objs: Vec<_> = r.object_refs().collect();
        assert_eq!(objs.len(), 1);
        assert_eq!(objs[0].object_id, 7);
        assert_eq!((objs[0].x, objs[0].y), (2, 3));
        assert_eq!(objs[0].object_type, 0);
    }

    #[test]
    fn region_object_12bit_positions() {
        // Object id 0x1234, type 0, x=0xABC, y=0x123. The region
        // header is 10 bytes (id, ver/fill, w, h, depth, clut, 8-bit
        // fill, 4/2-bit fill) before the object entries.
        let data = [
            0x07, 0x00, 0x00, 0x08, 0x00, 0x0C, 0b01_010_00, 0x03, 0x00, 0x00, // hdr
            0x12, 0x34, 0x0A, 0xBC, 0x01, 0x23,
        ];
        let r = parse_region(&data).unwrap();
        let objs: Vec<_> = r.object_refs().collect();
        assert_eq!(objs.len(), 1);
        assert_eq!(objs[0].object_id, 0x1234);
        assert_eq!(objs[0].object_type, 0);
        assert_eq!(objs[0].x, 0xABC);
        assert_eq!(objs[0].y, 0x123);
        // Character-coded: 8 bytes, fg/bg follow.
        let data = [
            0x07, 0x00, 0x00, 0x08, 0x00, 0x0C, 0b01_010_00, 0x03, 0x00, 0x00, // hdr
            0x12, 0x34, 0x4A, 0xBC, 0x01, 0x23, 0x09, 0x0A,
        ];
        let r = parse_region(&data).unwrap();
        let objs: Vec<_> = r.object_refs().collect();
        assert_eq!(objs.len(), 1);
        assert_eq!(objs[0].object_type, 1);
        assert_eq!(objs[0].fg_color, Some(9));
        assert_eq!(objs[0].bg_color, Some(10));
    }

    #[test]
    fn region_invalid_depth_falls_back_to_4() {
        // depth code 0 => 1 << 0 = 1: invalid, depth() falls back.
        let data = [
            0x01, 0x00, 0x00, 0x10, 0x00, 0x08, 0b01_000_00, 0x03, 0x00, 0x00,
        ];
        let r = parse_region(&data).unwrap();
        assert_eq!(r.depth(), 4);
    }

    #[test]
    fn clut_entries_full_and_limited() {
        // id 7, version 1, one full-range 8-bit-bank entry (id 3).
        let full = [0x07, 0x10, 0x03, 0b0010_0001, 235, 128, 128, 0];
        let seg = parse_clut(&full).unwrap();
        let es: Vec<_> = seg.entries().collect();
        assert_eq!(es.len(), 1);
        assert!(es[0].depth_8 && es[0].full_range);
        assert_eq!(es[0].y, 235);
        // One limited-range entry: 3 bytes of Y6/Cr6/Cb6/T2.
        let lim = [0x07, 0x10, 0x03, 0b0010_0000, 0b1010_0110, 0b1101_0011, 0b0110_1001];
        let seg = parse_clut(&lim).unwrap();
        let es: Vec<_> = seg.entries().collect();
        assert_eq!(es.len(), 1);
        assert_eq!(es[0].y, 0b1010_0100);
        assert_eq!(es[0].cr, 0b1011_0100);
        assert_eq!(es[0].cb, 0b0011_0100);
        assert_eq!(es[0].alpha, 0b1000_0000);
        // Invalid depth-flags byte stops the walk, no panic.
        let bad = [0x07, 0x10, 0x03, 0x00, 1, 2, 3, 4];
        assert_eq!(parse_clut(&bad).unwrap().entries().count(), 0);
    }

    #[test]
    fn object_parse_fields() {
        let mut data = vec![0x00, 0x09, 0x00]; // id 9, coding 0, non-mod 0
        data.extend_from_slice(&2u16.to_be_bytes()); // top field len
        data.extend_from_slice(&3u16.to_be_bytes()); // bottom field len
        data.extend_from_slice(&[0x10, 0x00]); // top: 2-bit block, EOL
        data.extend_from_slice(&[0x11, 0x00, 0x00]); // bottom
        let o = parse_object(&data).unwrap();
        assert_eq!(o.object_id, 9);
        assert_eq!(o.coding_method, 0);
        assert!(!o.non_modifying);
        assert_eq!(o.top_field, &[0x10, 0x00]);
        assert_eq!(o.bottom_field, &[0x11, 0x00, 0x00]);
        // Coding method 1 (characters): fields empty, typed.
        let o = parse_object(&[0x00, 0x09, 0b0000_0100]).unwrap();
        assert_eq!(o.coding_method, 1);
        // Truncated field lengths -> None (typed damage upstream).
        assert!(parse_object(&[0x00, 0x09, 0x00, 0x00, 0x10]).is_none());
        assert!(parse_object(&[0x00]).is_none());
    }

    #[test]
    fn display_def_parse() {
        let data = [0x15, 0x02, 0xCF, 0x02, 0x3F]; // v1, no window, 720x576
        let d = parse_display_def(&data).unwrap();
        assert_eq!((d.width, d.height, d.x, d.y), (720, 576, 0, 0));
        assert!(!d.display_window);
        let mut data = data.to_vec();
        data.extend_from_slice(&[0, 0, 1, 0x67, 0, 0, 1, 0x50]); // window: wx=359, wy=336
        data[0] = 0x1D; // window flag set
        let d = parse_display_def(&data).unwrap();
        assert!(d.display_window);
        // The window max addresses are INCLUSIVE: wx=359 -> 360,
        // wy=336 -> 337 (both +1, like the display size fields).
        assert_eq!((d.x, d.y, d.width, d.height), (0, 0, 360, 337));
        // Truncated window -> None.
        data.truncate(10);
        assert!(parse_display_def(&data).is_none());
    }
}
