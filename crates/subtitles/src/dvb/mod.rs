//! DVB subtitles (ETSI EN 300 743) — page/region/object/CLUT state
//! machine, decode to straight-alpha RGBA regions, and the reduce to
//! the 5.L1 compositing contract (card 5.N2, ADR-012 self-build).
//!
//! # What this module owns
//!
//! * [`segment`] — the 6-byte segment header walk and the typed
//!   payloads (page composition §7.2.2, region composition §7.2.3,
//!   CLUT definition §7.2.4, object data §7.2.5, display definition
//!   §7.2.1, end-of-display-set, stuffing);
//! * [`rle`] — the 2/4/8-bit pixel RLE tables (§7.2.5) with the
//!   map-table translation for cross-depth objects and the
//!   non-modifying color rule;
//! * [`clut`] — CLUT banks + the BT.601-headroom YCbCr→RGBA
//!   conversion (integer-identical to the reference decoders) +
//!   the default banks (the `BadClutId` fallback);
//! * this file — the **page state machine**: page versions and the
//!   acquisition/mode-change resets, region pixel buffers with fill
//!   and versioned CLUTs, object→region placements, the display-set
//!   composition into [`DvbPageUpdate`]s, and the render path to
//!   [`CompositedCue`]s;
//! * [`fuzz`] — the `fuzz_sub_dvb` harness.
//!
//! # The contract (dirty-world, D-4 + MP §8)
//!
//! [`DvbDecoder::decode_pes`] takes one subtitle PES payload (the
//! bytes after the PES header) plus its 90 kHz PTS and returns a
//! [`DvbPageUpdate`]: the composed region images for THIS display
//! set, the page timeout, and a **capped typed damage ledger**. It
//! never panics; every malformation (truncated segment, bad CLUT id,
//! RLE overrun, object placed before its region, region too large)
//! is typed data and the decode continues to the next segment.
//!
//! # Page lifecycle (EN 300 743 §7.3)
//!
//! A page is a list of regions; a region is a pixel buffer at a CLUT
//! depth; objects are RLE pixel blocks placed inside regions. Each
//! pixel-data sub-block is **one line**: a data-type byte (0x10/0x11/
//! 0x12), the line's RLE ending in EOL, then `0xF0` steps two rows
//! down (interleaved fields — top field first, then the bottom field
//! buffer). `page_state` 1 (acquisition point) or 2 (mode change)
//! drops all regions/objects/CLUTs. A repeated page version is a
//! no-op. Regions compose at the end-of-display-set segment (or,
//! when a stream omits it, once page+region+object have all been
//! seen — the reference behavior). An empty page composes to zero
//! regions: the display clears.
//!
//! # Timing
//!
//! EN 300 743 gives each display set a PTS and the page a
//! `page_time_out`. [`DvbCueBuilder`] turns consecutive updates into
//! half-open `[t0, t1)` spans exactly like the reference decoders'
//! end-display-time computation: the previous page stays up until
//! one nanosecond before the next update's PTS, and `page_time_out`
//! bounds a trailing page.
//!
//! # Never
//!
//! I/O, unsafe, unbounded allocation (region pixel buffers are
//! capped at 4 MiB, D-2), panics.

pub mod clut;
pub mod fuzz;
pub mod rle;
pub mod segment;

use std::collections::HashMap;

use crate::compositing::bitmap::{CompositedCue, CueBitmap, CueError};
use crate::compositing::layout::{
    Anchor, CueAlignment, CueLayout, HorizontalAlign, Rational, SafeMargin, VerticalAlign,
};
use media_types::time::{Stamp, TimeSpan, Timestamp};

/// Damage ledger cap (the SRT 256-event precedent, sized for one
/// display set).
pub const MAX_DAMAGE_EVENTS: usize = 64;

/// Region pixel-buffer cap: width × height × 1 byte (CLUT indices),
/// 4 MiB — twice a 1080p canvas, generous against the 320×256-typical
/// broadcast regions, tight against hostile dimensions (D-2).
pub const MAX_REGION_PIXELS: u32 = 4 * 1024 * 1024;

/// Typed decode damage (never a panic; the decode continues).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DvbDamage {
    /// A segment header ran past the payload end.
    TruncatedSegment {
        /// The segment kind (`None` when even the sync byte is gone).
        kind: Option<&'static str>,
    },
    /// A segment payload was too short for its grammar.
    TruncatedPayload {
        kind: &'static str,
        needed: usize,
        got: usize,
    },
    /// `segment_length` overran the PES payload — the walk stopped.
    LengthOverrun,
    /// An unknown segment type byte (skipped).
    UnknownSegment {
        type_byte: u8,
    },
    /// A region referenced a CLUT id never defined — the default
    /// banks were used (the reference fallback).
    BadClutId {
        clut_id: u8,
        region_id: u8,
    },
    /// A pixel run or line crossed the region edge and was clipped.
    RleOverrun {
        region_id: u8,
        object_id: u16,
    },
    /// An object data segment arrived for an object no region
    /// composition ever placed (page-before-region damage).
    ObjectWithoutRegion {
        object_id: u16,
    },
    /// An object placement fell outside its region — dropped.
    ObjectOutsideRegion {
        object_id: u16,
        region_id: u8,
    },
    /// Region dimensions over the cap or zero — region disabled.
    RegionSizeInvalid {
        region_id: u8,
        width: u16,
        height: u16,
    },
    /// `region_depth` code outside 1..=3 — fell back to 4-bit.
    RegionDepthInvalid {
        region_id: u8,
        depth_code: u8,
    },
    /// Object coding method 1 (characters), 2 (progressive) or 3 —
    /// unsupported by the render path (typed, EN 300 743 allows).
    ObjectCodingUnsupported {
        object_id: u16,
        coding: u8,
    },
    /// A pixel-data sub-block with an unknown data-type byte.
    UnknownPixelBlock {
        byte: u8,
        object_id: u16,
    },
    /// The damage ledger overflowed (count of dropped events).
    DamageOverflow {
        dropped: usize,
    },
}

impl std::fmt::Display for DvbDamage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DvbDamage::TruncatedSegment { kind } => {
                write!(f, "truncated segment header ({kind:?})")
            }
            DvbDamage::TruncatedPayload { kind, needed, got } => {
                write!(f, "truncated {kind} payload: needed {needed}, got {got}")
            }
            DvbDamage::LengthOverrun => f.write_str("segment length overruns the PES payload"),
            DvbDamage::UnknownSegment { type_byte } => {
                write!(f, "unknown segment type 0x{type_byte:02X}")
            }
            DvbDamage::BadClutId { clut_id, region_id } => write!(
                f,
                "region {region_id} references undefined CLUT {clut_id} (default banks used)"
            ),
            DvbDamage::RleOverrun { region_id, object_id } => write!(
                f,
                "object {object_id} RLE crossed region {region_id} edge (clipped)"
            ),
            DvbDamage::ObjectWithoutRegion { object_id } => {
                write!(f, "object {object_id} has no region placement")
            }
            DvbDamage::ObjectOutsideRegion { object_id, region_id } => {
                write!(f, "object {object_id} placed outside region {region_id}")
            }
            DvbDamage::RegionSizeInvalid { region_id, width, height } => {
                write!(f, "region {region_id} size {width}x{height} invalid/over cap")
            }
            DvbDamage::RegionDepthInvalid { region_id, depth_code } => {
                write!(f, "region {region_id} depth code {depth_code} invalid (4-bit used)")
            }
            DvbDamage::ObjectCodingUnsupported { object_id, coding } => {
                write!(f, "object {object_id} coding method {coding} unsupported")
            }
            DvbDamage::UnknownPixelBlock { byte, object_id } => {
                write!(f, "unknown pixel-data sub-block 0x{byte:02X} in object {object_id}")
            }
            DvbDamage::DamageOverflow { dropped } => {
                write!(f, "damage ledger overflow: {dropped} events dropped")
            }
        }
    }
}

/// One composed region image: straight-alpha RGBA8 at the region's
/// page position (palette already applied).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DvbRegionImage {
    /// Page-composition horizontal address.
    pub x: u16,
    /// Page-composition vertical address.
    pub y: u16,
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` RGBA8 bytes.
    pub rgba: Vec<u8>,
}

/// The result of one display set (one PES payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DvbPageUpdate {
    /// The page id this decoder locked onto (first page segment's
    /// id; EN 300 743 multiplexes ancillary pages, demuxed upstream).
    pub page_id: u16,
    /// The PES PTS in nanoseconds, when present.
    pub pts_ns: Option<i64>,
    /// `page_time_out` seconds (bounds a trailing page).
    pub timeout_s: u8,
    /// The composed regions (empty = clear the display).
    pub regions: Vec<DvbRegionImage>,
    /// The display extent the page was composed for (DDS, or the
    /// 720×576 default).
    pub display_extent: (u32, u32),
    /// The capped damage ledger.
    pub damage: Vec<DvbDamage>,
}

impl DvbPageUpdate {
    /// Was this display set clean (no damage events)?
    pub fn is_clean(&self) -> bool {
        self.damage.is_empty()
    }
}

/// Internal region state (the pixel buffer holds CLUT-bank indices,
/// always 8 bits wide — 2/4-bit data is mapped on write).
#[derive(Debug, Clone)]
struct RegionState {
    width: u32,
    height: u32,
    depth: u8,
    clut_id: u8,
    pbuf: Vec<u8>,
    dirty: bool,
}

/// Object placement (region id + position) — the object data
/// segment needs this to know where to paint.
#[derive(Debug, Clone, Copy)]
struct Placement {
    region_id: u8,
    x: u16,
    y: u16,
}

/// Display-definition state.
#[derive(Debug, Clone, Copy)]
struct DisplayDef {
    width: u32,
    height: u32,
    version: u8,
}

/// The DVB subtitle page state machine.
#[derive(Debug, Clone)]
pub struct DvbDecoder {
    page_id: Option<u16>,
    page_version: Option<u8>,
    page_entries: Vec<segment::PageRegionRef>,
    page_timeout_s: u8,
    regions: HashMap<u8, RegionState>,
    cluts: HashMap<u8, clut::Clut>,
    objects: HashMap<u16, Vec<Placement>>,
    display_def: Option<DisplayDef>,
    damage: Vec<DvbDamage>,
    damage_dropped: usize,
    /// Per-PES flags (the compose-without-end-of-display fallback).
    saw_page: bool,
    saw_region: bool,
    saw_object: bool,
    composed: bool,
    composed_regions: Vec<DvbRegionImage>,
}

impl Default for DvbDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DvbDecoder {
    /// A fresh decoder (no page lock yet).
    pub fn new() -> DvbDecoder {
        DvbDecoder {
            page_id: None,
            page_version: None,
            page_entries: Vec::new(),
            page_timeout_s: 0,
            regions: HashMap::new(),
            cluts: HashMap::new(),
            objects: HashMap::new(),
            display_def: None,
            damage: Vec::new(),
            damage_dropped: 0,
            saw_page: false,
            saw_region: false,
            saw_object: false,
            composed: false,
            composed_regions: Vec::new(),
        }
    }

    /// Reset all page state (a stream discontinuity / seek).
    pub fn reset(&mut self) {
        *self = DvbDecoder::new();
    }

    fn damage(&mut self, event: DvbDamage) {
        if self.damage.len() < MAX_DAMAGE_EVENTS {
            self.damage.push(event);
        } else {
            self.damage_dropped += 1;
        }
    }

    /// Decode one subtitle PES payload (bytes after the PES header)
    /// with its 90 kHz PTS. Returns the composed display set.
    pub fn decode_pes(&mut self, payload: &[u8], pts_90khz: Option<i64>) -> DvbPageUpdate {
        self.saw_page = false;
        self.saw_region = false;
        self.saw_object = false;
        self.composed = false;

        let pts_ns = pts_90khz.and_then(|ticks| {
            Timestamp::from_parts(ticks, 1, 90_000)
                .ok()
                .and_then(|ts| ts.as_nanos().ok())
                .map(|ns| ns.0 as i64)
        });

        let (segments, used) = segment::walk_segments(payload);
        if payload.is_empty() || used < payload.len() {
            if segments.is_empty() {
                self.damage(DvbDamage::TruncatedSegment { kind: None });
            } else {
                self.damage(DvbDamage::LengthOverrun);
            }
        }

        for seg in &segments {
            match seg.kind {
                segment::SegmentKind::PageComposition => self.page_segment(seg),
                segment::SegmentKind::RegionComposition => self.region_segment(seg),
                segment::SegmentKind::ClutDefinition => self.clut_segment(seg),
                segment::SegmentKind::ObjectData => self.object_segment(seg),
                segment::SegmentKind::DisplayDefinition => self.display_def_segment(seg),
                segment::SegmentKind::EndOfDisplaySet => self.compose(),
                segment::SegmentKind::Stuffing => {}
                segment::SegmentKind::Unknown(b) => {
                    self.damage(DvbDamage::UnknownSegment { type_byte: b });
                }
            }
        }

        // The end-of-display-set fallback (see module docs).
        if !self.composed && self.saw_page && self.saw_region && self.saw_object {
            self.compose();
        }

        let page_id = self.page_id.unwrap_or(0);
        let display_extent = self.display_def.map_or((720, 576), |d| (d.width, d.height));
        let mut damage = std::mem::take(&mut self.damage);
        if self.damage_dropped > 0 {
            damage.push(DvbDamage::DamageOverflow {
                dropped: self.damage_dropped,
            });
            self.damage_dropped = 0;
        }
        DvbPageUpdate {
            page_id,
            pts_ns,
            timeout_s: self.page_timeout_s,
            regions: std::mem::take(&mut self.composed_regions),
            display_extent,
            damage,
        }
    }

    fn page_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.saw_page = true;
        if self.page_id.is_none() {
            self.page_id = Some(seg.page_id);
        }
        let Some(page) = segment::parse_page(seg.data) else {
            self.damage(DvbDamage::TruncatedPayload {
                kind: "page-composition",
                needed: 2,
                got: seg.data.len(),
            });
            return;
        };
        if self.page_version == Some(page.version) {
            // Unchanged page (§7.2.2.2): object data may still follow;
            // placements stay as they are.
            self.page_timeout_s = page.timeout_s;
            return;
        }
        self.page_version = Some(page.version);
        self.page_timeout_s = page.timeout_s;

        if page.state == 1 || page.state == 2 {
            // Acquisition point / mode change: full reset.
            self.regions.clear();
            self.objects.clear();
            self.cluts.clear();
        }
        if page.regions.len() % 6 != 0 {
            self.damage(DvbDamage::TruncatedPayload {
                kind: "page-composition region entry",
                needed: page.regions.len() / 6 * 6 + 6,
                got: page.regions.len(),
            });
        }
        let mut entries: Vec<segment::PageRegionRef> = page.region_refs().collect();
        // Duplicate region ids: keep the first (reference behavior).
        let mut seen = std::collections::HashSet::new();
        entries.retain(|e| seen.insert(e.region_id));
        self.page_entries = entries;
    }

    fn region_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.saw_region = true;
        let Some(region) = segment::parse_region(seg.data) else {
            self.damage(DvbDamage::TruncatedPayload {
                kind: "region-composition",
                needed: 10,
                got: seg.data.len(),
            });
            return;
        };
        let depth = region.depth();
        if depth != 1 << (region.depth_code & 7) {
            self.damage(DvbDamage::RegionDepthInvalid {
                region_id: region.region_id,
                depth_code: region.depth_code,
            });
        }
        let (w, h) = (u32::from(region.width), u32::from(region.height));
        let valid = w > 0 && h > 0 && w.saturating_mul(h) <= MAX_REGION_PIXELS;
        if !valid {
            self.damage(DvbDamage::RegionSizeInvalid {
                region_id: region.region_id,
                width: region.width,
                height: region.height,
            });
            self.regions.remove(&region.region_id);
            return;
        }

        match self.regions.get_mut(&region.region_id) {
            Some(existing) if existing.width == w && existing.height == h => {
                existing.depth = depth;
                existing.clut_id = region.clut_id;
                if region.fill {
                    existing.pbuf.fill(region.fill_color);
                    existing.dirty = false;
                }
            }
            _ => {
                let mut pbuf = vec![0u8; (w * h) as usize];
                if region.fill {
                    pbuf.fill(region.fill_color);
                }
                self.regions.insert(
                    region.region_id,
                    RegionState {
                        width: w,
                        height: h,
                        depth,
                        clut_id: region.clut_id,
                        pbuf,
                        dirty: false,
                    },
                );
            }
        }

        // A new region composition replaces the old placement list
        // for this region.
        for list in self.objects.values_mut() {
            list.retain(|p| p.region_id != region.region_id);
        }
        for o in region.object_refs() {
            let placement = Placement {
                region_id: region.region_id,
                x: o.x,
                y: o.y,
            };
            if let Some(r) = self.regions.get(&region.region_id) {
                if u32::from(placement.x) >= r.width || u32::from(placement.y) >= r.height {
                    self.damage(DvbDamage::ObjectOutsideRegion {
                        object_id: o.object_id,
                        region_id: region.region_id,
                    });
                    continue;
                }
            }
            self.objects.entry(o.object_id).or_default().push(placement);
        }
    }

    fn clut_segment(&mut self, seg: &segment::RawSegment<'_>) {
        let Some(clut_seg) = segment::parse_clut(seg.data) else {
            self.damage(DvbDamage::TruncatedPayload {
                kind: "clut-definition",
                needed: 2,
                got: seg.data.len(),
            });
            return;
        };
        // Versioned: skip an unchanged CLUT (§7.2.4.2).
        if let Some(existing) = self.cluts.get(&clut_seg.clut_id) {
            if existing.version == clut_seg.version {
                return;
            }
        }
        let entries: Vec<_> = clut_seg.entries().collect();
        self.cluts.insert(
            clut_seg.clut_id,
            clut::Clut::from_entries(clut_seg.version, entries.into_iter()),
        );
    }

    fn display_def_segment(&mut self, seg: &segment::RawSegment<'_>) {
        let Some(dds) = segment::parse_display_def(seg.data) else {
            self.damage(DvbDamage::TruncatedPayload {
                kind: "display-definition",
                needed: 5,
                got: seg.data.len(),
            });
            return;
        };
        if self.display_def.is_some_and(|d| d.version == dds.version) {
            return;
        }
        let (w, h) = (u32::from(dds.width), u32::from(dds.height));
        if w > 0 && h > 0 && w <= 8192 && h <= 8192 {
            self.display_def = Some(DisplayDef {
                width: w,
                height: h,
                version: dds.version,
            });
        }
    }

    fn object_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.saw_object = true;
        let Some(object) = segment::parse_object(seg.data) else {
            self.damage(DvbDamage::TruncatedPayload {
                kind: "object-data",
                needed: 3,
                got: seg.data.len(),
            });
            return;
        };
        if object.coding_method != 0 {
            self.damage(DvbDamage::ObjectCodingUnsupported {
                object_id: object.object_id,
                coding: object.coding_method,
            });
            return;
        }
        let placements: Vec<Placement> = self
            .objects
            .get(&object.object_id)
            .cloned()
            .unwrap_or_default();
        if placements.is_empty() {
            self.damage(DvbDamage::ObjectWithoutRegion {
                object_id: object.object_id,
            });
            return;
        }
        for placement in placements {
            if !self.paint_field(object.object_id, &placement, 0, object.top_field, object.non_modifying) {
                break;
            }
            // Empty bottom field: the reference reuses the top data
            // one row down (progressive-as-interlaced streams).
            let bottom = if object.bottom_field.is_empty() {
                object.top_field
            } else {
                object.bottom_field
            };
            self.paint_field(object.object_id, &placement, 1, bottom, object.non_modifying);
        }
    }

    /// Paint one field of one object. Returns `false` when the field
    /// aborted (hostile placement — skip the bottom field too).
    fn paint_field(
        &mut self,
        object_id: u16,
        placement: &Placement,
        field: usize,
        data: &[u8],
        non_mod: bool,
    ) -> bool {
        let Some(region) = self.regions.get(&placement.region_id) else {
            self.damage(DvbDamage::ObjectWithoutRegion { object_id });
            return false;
        };
        let (rw, rh) = (region.width as usize, region.height as usize);
        let mut x = placement.x as usize;
        let mut y = placement.y as usize + field;

        // Map tables for cross-depth pixel data (§7.2.5 defaults;
        // 0x20/0x21/0x22 sub-blocks override them).
        let mut map2to4 = [0x00u8, 0x07, 0x08, 0x0F];
        let mut map2to8 = [0x00u8, 0x77, 0x88, 0xFF];
        let mut map4to8 = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, //
            0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        ];
        let depth = region.depth;

        let mut i = 0usize;
        while i < data.len() {
            let byte = data[i];
            // The placement guard (reference "Invalid object
            // location"): after a full line the next byte must be
            // 0xF0, and y must stay inside the region.
            if (byte != 0xF0 && x >= rw) || y >= rh {
                self.damage(DvbDamage::RleOverrun {
                    region_id: placement.region_id,
                    object_id,
                });
                return false;
            }
            i += 1;
            match byte {
                0x10 => {
                    let map = match depth {
                        8 => Some(&map2to8),
                        4 => Some(&map2to4),
                        _ => None,
                    };
                    let (used, x_end) =
                        self.paint_line(placement.region_id, x, y, &data[i..], Rle::Two, map.map(|m| m as &[u8]), non_mod);
                    x = x_end;
                    i += used;
                }
                0x11 => {
                    if depth < 4 {
                        self.damage(DvbDamage::UnknownPixelBlock { byte: 0x11, object_id });
                        return true;
                    }
                    let map = (depth == 8).then_some(&map4to8);
                    let (used, x_end) =
                        self.paint_line(placement.region_id, x, y, &data[i..], Rle::Four, map.map(|m| m as &[u8]), non_mod);
                    x = x_end;
                    i += used;
                }
                0x12 => {
                    if depth < 8 {
                        self.damage(DvbDamage::UnknownPixelBlock { byte: 0x12, object_id });
                        return true;
                    }
                    let (used, x_end) =
                        self.paint_line(placement.region_id, x, y, &data[i..], Rle::Eight, None, non_mod);
                    x = x_end;
                    i += used;
                }
                0x20 => {
                    if data.len() - i < 2 {
                        return true;
                    }
                    map2to4 = [data[i] >> 4, data[i] & 0xF, data[i + 1] >> 4, data[i + 1] & 0xF];
                    i += 2;
                }
                0x21 => {
                    if data.len() - i < 4 {
                        return true;
                    }
                    map2to8.copy_from_slice(&data[i..i + 4]);
                    i += 4;
                }
                0x22 => {
                    if data.len() - i < 16 {
                        return true;
                    }
                    map4to8.copy_from_slice(&data[i..i + 16]);
                    i += 16;
                }
                0xF0 => {
                    x = placement.x as usize;
                    y += 2;
                }
                other => {
                    // Unknown sub-block: size unknown — the reference
                    // logs and continues past the single byte.
                    self.damage(DvbDamage::UnknownPixelBlock {
                        byte: other,
                        object_id,
                    });
                }
            }
        }
        true
    }

    /// Decode one pixel line into the region row at `y` from column
    /// `x`. Returns `(bytes consumed, x after the line)`.
    fn paint_line(
        &mut self,
        region_id: u8,
        x: usize,
        y: usize,
        data: &[u8],
        rle: Rle,
        map: Option<&[u8]>,
        non_mod: bool,
    ) -> (usize, usize) {
        let Some(region) = self.regions.get_mut(&region_id) else {
            return (data.len(), x);
        };
        region.dirty = true;
        if y >= region.height as usize {
            return (data.len(), x);
        }
        let row_start = y * region.width as usize;
        let row = &mut region.pbuf[row_start..row_start + region.width as usize];
        match rle {
            Rle::Two => {
                let m = map.and_then(|m| <&[u8; 4]>::try_from(m).ok());
                let (used, x_end, _stop) = rle::decode_line_2bit(data, row, x, m, non_mod);
                (used, x_end)
            }
            Rle::Four => {
                let m = map.and_then(|m| <&[u8; 16]>::try_from(m).ok());
                let (used, x_end, _stop) = rle::decode_line_4bit(data, row, x, m, non_mod);
                (used, x_end)
            }
            Rle::Eight => {
                let (used, x_end, _stop) = rle::decode_line_8bit(data, row, x, non_mod);
                (used, x_end)
            }
        }
    }

    /// Compose the current page state into region images (the
    /// end-of-display-set action).
    fn compose(&mut self) {
        self.composed = true;
        let mut out = Vec::new();
        let mut missing_reported = std::collections::HashSet::new();
        let entries: Vec<segment::PageRegionRef> = self.page_entries.clone();
        for entry in &entries {
            let Some(region) = self.regions.get(&entry.region_id) else {
                // A page entry whose region composition never arrived
                // (or was disabled): typed once, skipped.
                if missing_reported.insert(entry.region_id) {
                    self.damage(DvbDamage::ObjectWithoutRegion {
                        object_id: u16::from(entry.region_id) | 0x8000,
                    });
                }
                continue;
            };
            if !region.dirty {
                continue;
            }
            match self.cluts.get(&region.clut_id) {
                Some(clut) => out.push(Self::region_image(*entry, region, clut)),
                None => {
                    let bad = DvbDamage::BadClutId {
                        clut_id: region.clut_id,
                        region_id: entry.region_id,
                    };
                    let clut = clut::Clut::default_banks();
                    out.push(Self::region_image(*entry, region, &clut));
                    self.damage(bad);
                }
            }
        }
        self.composed_regions = out;
    }

    fn region_image(
        entry: segment::PageRegionRef,
        region: &RegionState,
        clut: &clut::Clut,
    ) -> DvbRegionImage {
        let bank = clut.bank(region.depth);
        let mut rgba = Vec::with_capacity(region.pbuf.len() * 4);
        for &index in &region.pbuf {
            let c = bank[index as usize];
            rgba.extend_from_slice(&[c.r, c.g, c.b, c.a]);
        }
        DvbRegionImage {
            x: entry.x,
            y: entry.y,
            width: region.width,
            height: region.height,
            rgba,
        }
    }
}

/// Which RLE table a pixel-data sub-block uses.
#[derive(Debug, Clone, Copy)]
enum Rle {
    Two,
    Four,
    Eight,
}

/// Render page updates into compositing cues (the 5.L1 contract):
/// each composed region becomes a [`CompositedCue`] with its exact
/// page position as a rational anchor over the display extent (the
/// [`CueLayout::reference`] for presentation-extent scaling), and
/// the half-open span closes one nanosecond before the next update's
/// PTS (the reference end-display-time rule), bounded by the page
/// timeout for a trailing page.
#[derive(Debug, Clone)]
pub struct DvbCueBuilder {
    extent: (u32, u32),
}

impl DvbCueBuilder {
    /// A cue builder for the given display extent.
    pub fn new(extent: (u32, u32)) -> DvbCueBuilder {
        DvbCueBuilder { extent }
    }

    /// Convert one update (with the NEXT update's PTS in
    /// nanoseconds, when known) into cues. `z_base` orders regions
    /// within the page (page entry order).
    ///
    /// # Errors
    /// [`CueError`] from the bitmap/span validation (zero-sized
    /// regions are skipped instead of erroring).
    pub fn cues(
        &self,
        update: &DvbPageUpdate,
        next_pts_ns: Option<i64>,
        z_base: u16,
    ) -> Result<Vec<CompositedCue>, CueError> {
        let mut cues = Vec::new();
        let start = update.pts_ns;
        let end = match (next_pts_ns, start) {
            (Some(next), Some(start)) if next > start => Some(next - 1),
            _ => start.map(|s| s.saturating_add(i64::from(update.timeout_s) * 1_000_000_000)),
        };
        for (i, region) in update.regions.iter().enumerate() {
            if region.width == 0 || region.height == 0 {
                continue;
            }
            let bitmap = CueBitmap::from_rgba(
                region.width,
                region.height,
                region.height,
                region.rgba.clone(),
            )?;
            let ns = |v: Option<i64>| {
                v.map(|ns| Stamp::Present(Timestamp::from_parts(ns, 1, 1_000_000_000).expect("ns base")))
                    .unwrap_or(Stamp::Missing)
            };
            let span = TimeSpan {
                start: ns(start),
                end: ns(end),
            };
            let layout = self.layout_for(region.x, region.y);
            cues.push(CompositedCue::new(
                span,
                bitmap,
                layout,
                z_base.saturating_add(i as u16),
            )?);
        }
        Ok(cues)
    }

    /// The exact-position layout: anchor at `(x/width, y/height)` of
    /// the display extent, top-left aligned, scaling with the
    /// presentation extent via [`CueLayout::reference`]. The 5 %
    /// title-safe clamp applies only to regions authored outside the
    /// safe box (a documented divergence — broadcast content is
    /// authored inside it).
    fn layout_for(&self, x: u16, y: u16) -> CueLayout {
        let (w, h) = (self.extent.0.max(1), self.extent.1.max(1));
        let u = Rational::new(u32::from(x), w).unwrap_or(Rational::ZERO);
        let v = Rational::new(u32::from(y), h).unwrap_or(Rational::ZERO);
        CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
            alignment: CueAlignment {
                horizontal: HorizontalAlign::Start,
                vertical: VerticalAlign::Top,
            },
            margin: SafeMargin::TitleSafe,
            reference: Some(self.extent),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Segment header + payload.
    fn seg(kind: u8, page: u16, data: &[u8]) -> Vec<u8> {
        let mut v = vec![0x0F, kind];
        v.extend_from_slice(&page.to_be_bytes());
        v.extend_from_slice(&(data.len() as u16).to_be_bytes());
        v.extend_from_slice(data);
        v
    }

    fn page_seg(timeout: u8, version: u8, state: u8, entries: &[(u8, u16, u16)]) -> Vec<u8> {
        let mut d = vec![timeout, (version << 4) | (state << 2)];
        for &(id, x, y) in entries {
            d.push(id);
            d.push(0);
            d.extend_from_slice(&x.to_be_bytes());
            d.extend_from_slice(&y.to_be_bytes());
        }
        seg(0x10, 1, &d)
    }

    fn region_seg(
        id: u8,
        w: u16,
        h: u16,
        depth_code: u8,
        clut: u8,
        fill: bool,
        fill_color: u8,
        objects: &[(u16, u16, u16)],
    ) -> Vec<u8> {
        let mut d = vec![
            id,
            (2 << 4) | (u8::from(fill) << 3),
            (w >> 8) as u8,
            w as u8,
            (h >> 8) as u8,
            h as u8,
            0b01_000_00 | (depth_code << 2),
            clut,
            0,
            fill_color,
        ];
        for &(oid, x, y) in objects {
            d.extend_from_slice(&oid.to_be_bytes());
            d.push((x >> 8) as u8);
            d.push(x as u8);
            d.push((y >> 8) as u8);
            d.push(y as u8);
        }
        seg(0x11, 1, &d)
    }

    fn clut_seg(id: u8, version: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
        let mut d = vec![id, version << 4];
        for &(eid, y, cb, cr, alpha) in entries {
            // depth flags: 2-bit bank (0x80) + 4-bit bank (0x40),
            // full-range (bit 0).
            d.extend_from_slice(&[eid, 0b1100_0001, y, cr, cb, alpha]);
        }
        seg(0x12, 1, &d)
    }

    fn object_seg(id: u16, top: &[u8], bottom: &[u8]) -> Vec<u8> {
        let mut d = id.to_be_bytes().to_vec();
        d.push(0);
        d.extend_from_slice(&(top.len() as u16).to_be_bytes());
        d.extend_from_slice(&(bottom.len() as u16).to_be_bytes());
        d.extend_from_slice(top);
        d.extend_from_slice(bottom);
        seg(0x13, 1, &d)
    }

    #[test]
    fn empty_payload_is_typed_not_panicking() {
        let mut dec = DvbDecoder::new();
        let up = dec.decode_pes(&[], Some(0));
        assert!(up.regions.is_empty());
        assert!(matches!(
            up.damage.first(),
            Some(DvbDamage::TruncatedSegment { kind: None })
        ));
    }

    #[test]
    fn truncated_object_is_typed() {
        let mut dec = DvbDecoder::new();
        let payload = seg(0x13, 1, &[0x00, 0x09]);
        let up = dec.decode_pes(&payload, None);
        assert!(matches!(
            up.damage.first(),
            Some(DvbDamage::TruncatedPayload { kind: "object-data", .. })
        ));
    }

    #[test]
    fn full_page_lifecycle_two_updates() {
        let mut dec = DvbDecoder::new();
        // Display set 1: acquisition, region 4x2 at (10,20), object 1
        // at (1,0). Top line: 2-bit codes 1,2,3 then EOL. Bottom:
        // 2-bit codes 3,3 then EOL (the region is 2-bit deep — 4-bit
        // sub-blocks would be typed-unknown there). Each line:
        // [type byte][data][0xF0].
        let top = [0x10u8, 0b01_10_11_00, 0xF0];
        let bottom = [0x10u8, 0b1111_0000, 0xF0];
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(5, 1, 1, &[(7, 10, 20)]));
        payload.extend_from_slice(&region_seg(7, 4, 2, 1, 9, true, 0, &[(1, 1, 0)]));
        // CLUT: entry 1 = white, 2 = red-ish, 3 = blue-ish (2-bit +
        // 4-bit banks).
        payload.extend_from_slice(&clut_seg(
            9,
            0,
            &[
                (1, 235, 128, 128, 0),
                (2, 82, 90, 240, 0),
                (3, 41, 240, 110, 0),
            ],
        ));
        payload.extend_from_slice(&object_seg(1, &top, &bottom));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, Some(0));
        assert!(up.is_clean(), "damage: {:?}", up.damage);
        assert_eq!(up.regions.len(), 1);
        let r = &up.regions[0];
        assert_eq!((r.x, r.y, r.width, r.height), (10, 20, 4, 2));
        // Top row: col0 = fill 0 (transparent), col1 = index 1
        // (white), col2 = index 2 (red), col3 = index 3 (blue).
        assert_eq!(&r.rgba[0..4], &[0, 0, 0, 0]);
        assert_eq!(&r.rgba[4..8], &[255, 255, 255, 255]);
        assert_eq!(&r.rgba[8..12], &[255, 1, 0, 255], "red-ish index 2");
        assert_eq!(&r.rgba[12..16], &[0, 0, 255, 255]);
        // Bottom row (2-bit codes 3,3 at cols 1..3): index 3 = blue
        // (two pixels: cols 1 and 2 of row 1).
        assert_eq!(&r.rgba[(4 + 1) * 4..(4 + 3) * 4], &[0, 0, 255, 255, 0, 0, 255, 255]);

        // Display set 2 (normal case): new object data repaints.
        let top2 = [0x10u8, 0b11_01_00_00, 0xF0];
        let mut payload2 = Vec::new();
        payload2.extend_from_slice(&page_seg(5, 2, 0, &[(7, 10, 20)]));
        payload2.extend_from_slice(&object_seg(1, &top2, &[]));
        payload2.extend_from_slice(&seg(0x80, 1, &[]));
        let up2 = dec.decode_pes(&payload2, Some(90_000));
        assert!(up2.is_clean(), "damage: {:?}", up2.damage);
        assert_eq!(up2.regions.len(), 1);
        // Repainted: top row col1 = index 3 (blue) now.
        assert_eq!(&up2.regions[0].rgba[4..8], &[0, 0, 255, 255]);

        // Display set 3: empty page (acquisition, no regions) clears.
        let mut payload3 = Vec::new();
        payload3.extend_from_slice(&page_seg(5, 3, 1, &[]));
        payload3.extend_from_slice(&seg(0x80, 1, &[]));
        let up3 = dec.decode_pes(&payload3, Some(180_000));
        assert!(up3.regions.is_empty());
        assert!(up3.is_clean());
    }

    #[test]
    fn mode_change_clears_state() {
        let mut dec = DvbDecoder::new();
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(5, 1, 1, &[(7, 0, 0)]));
        payload.extend_from_slice(&region_seg(7, 2, 2, 1, 9, true, 0, &[]));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, None);
        // Region not dirty (no object painted): no rects, clean.
        assert!(up.regions.is_empty());
        assert!(up.is_clean());

        // Mode change (state 2) drops the region; the page entry
        // becomes a typed no-op.
        let mut payload2 = Vec::new();
        payload2.extend_from_slice(&page_seg(5, 2, 2, &[(7, 0, 0)]));
        payload2.extend_from_slice(&seg(0x80, 1, &[]));
        let up2 = dec.decode_pes(&payload2, None);
        assert!(up2.regions.is_empty());
    }

    #[test]
    fn bad_clut_id_uses_default_banks() {
        let mut dec = DvbDecoder::new();
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(5, 1, 1, &[(7, 0, 0)]));
        // A 2-row region so the reused bottom field fits (regions
        // are interlaced-paired in real streams).
        payload.extend_from_slice(&region_seg(7, 2, 2, 1, 42, false, 0, &[(1, 0, 0)]));
        let top = [0x10u8, 0b01_01_00_00, 0xF0];
        payload.extend_from_slice(&object_seg(1, &top, &[]));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, None);
        assert!(matches!(
            up.damage.first(),
            Some(DvbDamage::BadClutId { clut_id: 42, region_id: 7 })
        ));
        // Default 2-bit bank index 1 = opaque white.
        assert_eq!(up.regions.len(), 1);
        assert_eq!(&up.regions[0].rgba[0..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn object_before_region_is_typed() {
        let mut dec = DvbDecoder::new();
        let payload = object_seg(5, &[0x10, 0x00], &[]);
        let up = dec.decode_pes(&payload, None);
        assert!(matches!(
            up.damage.first(),
            Some(DvbDamage::ObjectWithoutRegion { object_id: 5 })
        ));
    }

    #[test]
    fn rle_overrun_is_clipped_and_typed() {
        let mut dec = DvbDecoder::new();
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(5, 1, 1, &[(7, 0, 0)]));
        payload.extend_from_slice(&region_seg(7, 3, 1, 1, 9, false, 0, &[(1, 0, 0)]));
        // 2-bit long run 29+255 = 280 px of value 1 in a 3-wide row.
        let top = [0x10u8, 0b00001111, 0b1111_1101, 0b0100_0000, 0xF0];
        payload.extend_from_slice(&object_seg(1, &top, &[]));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, None);
        assert_eq!(up.regions.len(), 1);
        assert_eq!(up.regions[0].rgba.len(), 3 * 4, "row fully painted");
    }

    #[test]
    fn region_over_cap_is_disabled() {
        let mut dec = DvbDecoder::new();
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(5, 1, 1, &[(7, 0, 0)]));
        payload.extend_from_slice(&region_seg(7, 4000, 4000, 1, 9, false, 0, &[]));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, None);
        assert!(matches!(
            up.damage.first(),
            Some(DvbDamage::RegionSizeInvalid { region_id: 7, .. })
        ));
        assert!(up.regions.is_empty());
    }

    #[test]
    fn unknown_pixel_block_stops_object() {
        let mut dec = DvbDecoder::new();
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(5, 1, 1, &[(7, 0, 0)]));
        payload.extend_from_slice(&region_seg(7, 2, 1, 1, 9, false, 0, &[(1, 0, 0)]));
        let top = [0x99u8, 0x00];
        payload.extend_from_slice(&object_seg(1, &top, &[]));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, None);
        assert!(matches!(
            up.damage.first(),
            Some(DvbDamage::UnknownPixelBlock { byte: 0x99, object_id: 1 })
        ));
    }

    #[test]
    fn cue_builder_spans_and_positions() {
        let mut dec = DvbDecoder::new();
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(5, 1, 1, &[(7, 360, 288)]));
        payload.extend_from_slice(&region_seg(7, 2, 2, 1, 9, false, 0, &[(1, 0, 0)]));
        let top = [0x10u8, 0b01_01_00_00, 0xF0];
        payload.extend_from_slice(&object_seg(1, &top, &[]));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, Some(90_000));
        let builder = DvbCueBuilder::new(up.display_extent);
        // The next update's PTS in NANOSECONDS: 180_000 ticks = 2 s.
        let cues = builder.cues(&up, Some(2_000_000_000), 0).unwrap();
        assert_eq!(cues.len(), 1);
        let cue = &cues[0];
        assert_eq!(cue.start, Some(1_000_000_000));
        // End = next pts - 1ns (the reference end-display-time rule).
        assert_eq!(cue.end, Some(2_000_000_000 - 1));
        assert_eq!(cue.bitmap.width, 2);
        // Anchor: exact page position (360/720, 288/576) = (1/2, 1/2).
        assert_eq!(cue.layout.anchor.u, Rational::new(1, 2).unwrap());
        assert_eq!(cue.layout.anchor.v, Rational::new(1, 2).unwrap());
        assert_eq!(cue.layout.reference, Some((720, 576)));
        assert!(cue.active_at(1_000_000_000));
        assert!(!cue.active_at(2_000_000_000 - 1));
    }

    #[test]
    fn timeout_bounds_trailing_page() {
        let mut dec = DvbDecoder::new();
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_seg(2, 1, 1, &[(7, 0, 0)]));
        payload.extend_from_slice(&region_seg(7, 2, 1, 1, 9, false, 0, &[(1, 0, 0)]));
        let top = [0x10u8, 0b01_01_00_00, 0xF0];
        payload.extend_from_slice(&object_seg(1, &top, &[]));
        payload.extend_from_slice(&seg(0x80, 1, &[]));
        let up = dec.decode_pes(&payload, Some(90_000));
        let builder = DvbCueBuilder::new(up.display_extent);
        // No next update: the 2s page timeout bounds the span.
        let cues = builder.cues(&up, None, 0).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].end, Some(1_000_000_000 + 2_000_000_000));
    }
}
