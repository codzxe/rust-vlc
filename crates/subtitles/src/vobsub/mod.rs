//! VobSub (IDX/SUB) — the DVD sub-picture subtitle pair (card 5.N2,
//! ADR-012 self-build; carriage already landed: the PS demuxer
//! routes `0xBD` substream `0x20..=0x3F` as `"dvd_subtitle"`).
//!
//! # What this module owns
//!
//! * [`idx`] — the `.idx` text parse (palette, size, per-stream
//!   timestamp/filepos entries, delays, tolerances);
//! * [`spu`] — the SPU grammar (display-control commands, the
//!   nibble-RLE interlaced bitmap, the MPEG-PS PES walker for the
//!   `.sub` half);
//! * this file — pairing the two: per-entry PES accumulation, SPU
//!   decode, palette→RGBA (with the reference's grayscale fallback
//!   when the idx lacks a palette), the transparent-border crop the
//!   reference applies, and the reduce to the 5.L1
//!   [`CompositedCue`] contract;
//! * [`fuzz`] — the `fuzz_sub_vobsub` harness.
//!
//! # The contract (dirty-world, D-4 + MP §8)
//!
//! [`parse_vobsub`] takes the idx bytes + the sub bytes and returns
//! every cue that decoded plus a capped typed damage ledger. It
//! never panics; every malformation (bad filepos, incomplete SPU,
//! RLE overrun, missing palette) is typed data. Multi-PES SPUs
//! accumulate exactly like the reference reconstruction buffer
//! (bounded by the entry's byte span — the distance to the next idx
//! filepos — and the 128 KiB cap).
//!
//! # Timing
//!
//! Each cue's span = idx timestamp + the SPU's start/end dates
//! (`(date << 10) / 90` ms, half-open `[t0, t1)` at the compositor).
//! An SPU without an end date is open-ended (the next entry on the
//! same stream implicitly replaces it at its timestamp — the
//! reference's subtitle-queue semantics).
//!
//! # Never
//!
//! I/O, unsafe, unbounded allocation (SPU accumulation capped at
//! 128 KiB), panics.

pub mod fuzz;
pub mod idx;
pub mod spu;

use crate::compositing::bitmap::{CompositedCue, CueBitmap, CueError};
use crate::compositing::layout::{
    Anchor, CueAlignment, CueLayout, HorizontalAlign, Rational, SafeMargin, VerticalAlign,
};
use crate::compositing::target::Rgba;
use media_types::time::{Stamp, TimeSpan, Timestamp};

pub use idx::{IdxDamage, IdxEntry, IdxStream, VobSubIdx, parse_idx};
pub use spu::{SpuDecoded, SpuError, SPU_MAX_SIZE, decode_spu};

/// The pair-decode damage ledger cap.
pub const MAX_DAMAGE_EVENTS: usize = 64;

/// Typed pair-decode damage (never a panic; the walk continues).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VobSubDamage {
    /// An idx `filepos` with no SPU PES in its byte span.
    BadFilepos {
        filepos: u64,
    },
    /// The SPU at a filepos failed to decode (truncated, bad
    /// offsets, RLE overrun, HD commands…).
    Spu {
        filepos: u64,
        error: SpuError,
    },
    /// No palette in the idx — the grayscale fallback was used.
    NoPalette,
    /// The damage ledger overflowed (count of dropped events).
    Overflow {
        dropped: usize,
    },
}

impl std::fmt::Display for VobSubDamage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VobSubDamage::BadFilepos { filepos } => {
                write!(f, "no SPU PES at idx filepos 0x{filepos:X}")
            }
            VobSubDamage::Spu { filepos, error } => {
                write!(f, "SPU at 0x{filepos:X} failed: {error}")
            }
            VobSubDamage::NoPalette => {
                f.write_str("idx has no palette — grayscale fallback used")
            }
            VobSubDamage::Overflow { dropped } => {
                write!(f, "damage ledger overflow: {dropped} events dropped")
            }
        }
    }
}

/// One decoded VobSub cue (pre-contract form).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VobSubCue {
    /// Which idx stream (substream index) this cue belongs to.
    pub stream_index: u8,
    /// The stream's language tag.
    pub lang: String,
    /// Cue start in nanoseconds (idx timestamp + start date).
    pub start_ns: i64,
    /// Cue end in nanoseconds (`None` = open-ended).
    pub end_ns: Option<i64>,
    /// Top-left x on the idx canvas (post transparent-border crop).
    pub x: u16,
    /// Top-left y.
    pub y: u16,
    pub width: u32,
    pub height: u32,
    /// Straight-alpha RGBA8, `width * height * 4`.
    pub rgba: Vec<u8>,
    /// The forced/menu flag (command `0x00`).
    pub forced: bool,
}

/// The parse result: the cues that survived + the damage ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VobSubDoc {
    /// The video canvas (idx `size:`), when declared.
    pub size: Option<(u32, u32)>,
    pub cues: Vec<VobSubCue>,
    pub damage: Vec<VobSubDamage>,
}

impl VobSubDoc {
    /// Did the whole pair decode clean?
    pub fn is_clean(&self) -> bool {
        self.damage.is_empty()
    }
}

/// Parse an IDX/SUB pair (both files' bytes).
pub fn parse_vobsub(idx_text: &[u8], sub: &[u8]) -> VobSubDoc {
    let idx = parse_idx(idx_text);
    let mut damage: Vec<VobSubDamage> = Vec::new();
    let mut dropped = 0usize;
    let mut record = |damage: &mut Vec<VobSubDamage>, d: VobSubDamage| {
        if damage.len() < MAX_DAMAGE_EVENTS {
            damage.push(d);
        } else {
            dropped += 1;
        }
    };
    if !idx.has_palette {
        record(&mut damage, VobSubDamage::NoPalette);
    }

    // Every SPU PES fragment with its sub-file offset, once.
    let fragments = spu::extract_spu_pes(sub);
    // The entry byte spans: [filepos, next filepos) across ALL
    // streams (the reference's psize logic).
    let mut bounds: Vec<u64> = idx
        .streams
        .iter()
        .flat_map(|s| s.entries.iter().map(|e| e.filepos))
        .collect();
    bounds.sort_unstable();
    bounds.dedup();

    let mut cues = Vec::new();
    for stream in &idx.streams {
        let want = 0x20u8 + stream.index;
        for entry in &stream.entries {
            let next_bound = bounds.iter().find(|&&fp| fp > entry.filepos).copied();
            let span_end = next_bound.unwrap_or(u64::MAX);
            let mut acc: Vec<u8> = Vec::new();
            for (ss, offset, _pts, payload) in &fragments {
                if *ss == want && (*offset as u64) >= entry.filepos && (*offset as u64) < span_end {
                    acc.extend_from_slice(payload);
                    if acc.len() >= SPU_MAX_SIZE {
                        break;
                    }
                }
            }
            if acc.is_empty() {
                record(&mut damage, VobSubDamage::BadFilepos { filepos: entry.filepos });
                continue;
            }
            match decode_spu(&acc) {
                Ok(spu) => {
                    if let Some(cue) = render_cue(&idx, entry, &spu) {
                        cues.push(VobSubCue {
                            stream_index: stream.index,
                            lang: stream.lang.clone(),
                            ..cue
                        });
                    }
                }
                Err(error) => record(
                    &mut damage,
                    VobSubDamage::Spu { filepos: entry.filepos, error },
                ),
            }
        }
    }
    if dropped > 0 {
        damage.push(VobSubDamage::Overflow { dropped });
    }
    VobSubDoc {
        size: idx.size,
        cues,
        damage,
    }
}

/// Apply the palette (or the grayscale fallback) and the
/// transparent-border crop; `None` = nothing visible.
fn render_cue(idx: &VobSubIdx, entry: &IdxEntry, spu: &SpuDecoded) -> Option<VobSubCue> {
    let rgba_palette: [Rgba; 4] = if idx.has_palette {
        let mut p = [Rgba::BLACK; 4];
        for (i, slot) in p.iter_mut().enumerate() {
            let rgb = idx.palette[spu.colormap[i] as usize] & 0xFF_FFFF;
            *slot = Rgba {
                r: ((rgb >> 16) & 0xFF) as u8,
                g: ((rgb >> 8) & 0xFF) as u8,
                b: (rgb & 0xFF) as u8,
                // The 4-bit contrast nibble scales to 8-bit by 17
                // (0x0 -> 0, 0xF -> 255) — the reference formula.
                a: spu.alpha[i].saturating_mul(17),
            };
        }
        p
    } else {
        guess_palette(spu)
    };

    let transparent = [
        rgba_palette[0].a == 0,
        rgba_palette[1].a == 0,
        rgba_palette[2].a == 0,
        rgba_palette[3].a == 0,
    ];
    let used: [bool; 4] = {
        let mut u = [false; 4];
        for &i in &spu.indices {
            u[i as usize] = true;
        }
        u
    };
    if (0..4).all(|i| !used[i] || transparent[i]) {
        // Everything visible is transparent: no cue (the reference's
        // "empty rectangle").
        return None;
    }

    let (x, y, w, h, indices) = if spu.forced {
        (
            spu.x,
            spu.y,
            spu.width,
            spu.height,
            spu.indices.clone(),
        )
    } else {
        crop_borders(spu, transparent)?
    };

    let mut rgba = Vec::with_capacity(w * h * 4);
    for &index in &indices {
        let c = rgba_palette[index as usize];
        rgba.extend_from_slice(&[c.r, c.g, c.b, c.a]);
    }
    let start_ms = entry.timestamp_ms + spu.start_ms;
    let end_ms = spu.end_ms.map(|e| entry.timestamp_ms + e);
    Some(VobSubCue {
        stream_index: 0,
        lang: String::new(),
        start_ns: start_ms * 1_000_000,
        end_ns: end_ms.map(|ms| ms * 1_000_000),
        x,
        y,
        width: w as u32,
        height: h as u32,
        rgba,
        forced: spu.forced,
    })
}

/// The reference's grayscale guess for palette-less idx files:
/// white-based level map over the distinct visible colors (ffmpeg
/// `guess_palette` with `subtitle_color = 0xffffff`).
fn guess_palette(spu: &SpuDecoded) -> [Rgba; 4] {
    const LEVEL_MAP: [&[u8]; 4] = [&[0xFF], &[0x00, 0xFF], &[0x00, 0x80, 0xFF], &[0x00, 0x55, 0xAA, 0xFF]];
    let mut palette = [Rgba { r: 0, g: 0, b: 0, a: 0 }; 4];
    let mut color_used = [false; 16];
    let mut nb_opaque = 0usize;
    for i in 0..4 {
        if spu.alpha[i] != 0 && !color_used[spu.colormap[i] as usize] {
            color_used[spu.colormap[i] as usize] = true;
            nb_opaque += 1;
        }
    }
    if nb_opaque == 0 {
        return palette;
    }
    let levels = LEVEL_MAP[nb_opaque - 1];
    let mut color_used = [0u8; 16];
    let mut j = 0usize;
    for i in 0..4 {
        if spu.alpha[i] != 0 {
            let cm = spu.colormap[i] as usize;
            if color_used[cm] == 0 {
                // The level map is already 8-bit gray (0x00..0xFF).
                let v = levels[j.min(levels.len() - 1)];
                palette[i] = Rgba { r: v, g: v, b: v, a: spu.alpha[i].saturating_mul(17) };
                color_used[cm] = (i + 1) as u8;
                j += 1;
            } else {
                let first = usize::from(color_used[cm] - 1);
                palette[i] = Rgba {
                    r: palette[first].r,
                    g: palette[first].g,
                    b: palette[first].b,
                    a: spu.alpha[i].saturating_mul(17),
                };
            }
        }
    }
    palette
}

/// Crop fully-transparent border rows/columns (the reference's
/// `find_smallest_bounding_rectangle` for non-menu subs). Returns
/// `None` when everything is transparent.
fn crop_borders(spu: &SpuDecoded, transparent: [bool; 4]) -> Option<(u16, u16, usize, usize, Vec<u8>)> {
    let (w, h) = (spu.width, spu.height);
    if w == 0 || h == 0 {
        return None;
    }
    let row_transparent = |y: usize| (0..w).all(|x| transparent[spu.indices[y * w + x] as usize]);
    let col_transparent = |x: usize| (0..h).all(|y| transparent[spu.indices[y * w + x] as usize]);
    let mut y1 = 0usize;
    while y1 < h && row_transparent(y1) {
        y1 += 1;
    }
    if y1 == h {
        return None;
    }
    let mut y2 = h - 1;
    while y2 > 0 && row_transparent(y2) {
        y2 -= 1;
    }
    let mut x1 = 0usize;
    while x1 < w - 1 && col_transparent(x1) {
        x1 += 1;
    }
    let mut x2 = w - 1;
    while x2 > 0 && col_transparent(x2) {
        x2 -= 1;
    }
    let (cw, ch) = (x2 - x1 + 1, y2 - y1 + 1);
    let mut out = Vec::with_capacity(cw * ch);
    for y in y1..=y2 {
        out.extend_from_slice(&spu.indices[y * w + x1..y * w + x1 + cw]);
    }
    Some((spu.x + x1 as u16, spu.y + y1 as u16, cw, ch, out))
}

/// Render the decoded cues into compositing cues (the 5.L1
/// contract): exact canvas position as a rational anchor (the DVB
/// precedent), open-ended spans when the stream sends no end date.
///
/// # Errors
/// [`CueError`] from the bitmap/span validation.
pub fn vobsub_cues(doc: &VobSubDoc, z: u16) -> Result<Vec<CompositedCue>, CueError> {
    let extent = doc.size.unwrap_or((720, 480));
    let mut cues = Vec::new();
    for (i, cue) in doc.cues.iter().enumerate() {
        if cue.width == 0 || cue.height == 0 {
            continue;
        }
        let bitmap = CueBitmap::from_rgba(cue.width, cue.height, cue.height, cue.rgba.clone())?;
        let (ew, eh) = (extent.0.max(1), extent.1.max(1));
        let u = Rational::new(u32::from(cue.x), ew).unwrap_or(Rational::ZERO);
        let v = Rational::new(u32::from(cue.y), eh).unwrap_or(Rational::ZERO);
        let layout = CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
            alignment: CueAlignment {
                horizontal: HorizontalAlign::Start,
                vertical: VerticalAlign::Top,
            },
            margin: SafeMargin::TitleSafe,
            reference: Some(extent),
        };
        let ns = |v: Option<i64>| {
            v.map(|ns| Stamp::Present(Timestamp::from_parts(ns, 1, 1_000_000_000).expect("ns base")))
                .unwrap_or(Stamp::Missing)
        };
        let span = TimeSpan {
            start: ns(Some(cue.start_ns)),
            end: ns(cue.end_ns),
        };
        cues.push(CompositedCue::new(span, bitmap, layout, z.saturating_add(i as u16))?);
    }
    Ok(cues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vobsub::spu::decode_spu;

    /// One SPU packet from field data + commands (mirror of the
    /// spu.rs test builder, kept local for the pair tests). The two
    /// dates land in chained DCSQT headers — `0x01`/`0x02` take no
    /// arguments (the reference grammar).
    fn build_spu(top: &[u8], bottom: &[u8], x1: u16, y1: u16, w: u16, h: u16, start_date: u16, end_date: u16) -> Vec<u8> {
        let x2 = x1 + w - 1;
        let y2 = y1 + h - 1;
        let mut spu = Vec::new();
        spu.extend_from_slice(&[0, 0]);
        spu.extend_from_slice(&[0, 0]);
        let off1 = spu.len();
        spu.extend_from_slice(top);
        let off2 = spu.len();
        spu.extend_from_slice(bottom);
        let cmds = spu.len();
        // DCSQT 1: date = start_date, chained to DCSQT 2.
        let cmds2 = cmds + 4 + 1 + 3 + 3 + 7 + 5 + 1;
        spu.extend_from_slice(&start_date.to_be_bytes());
        spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
        spu.push(0x01); // start display at the DCSQT date
        spu.push(0x03);
        spu.extend_from_slice(&[0x01, 0x23]); // colormap [0]=3,[1]=2,[2]=1,[3]=0
        spu.push(0x04);
        spu.extend_from_slice(&[0x0F, 0x00]); // alpha [0]=0,[1]=0,[2]=15,[3]=0
        spu.push(0x05);
        spu.extend_from_slice(&[
            (x1 >> 4) as u8,
            ((x1 & 0xF) << 4) as u8 | (x2 >> 8) as u8,
            (x2 & 0xFF) as u8,
            (y1 >> 4) as u8,
            ((y1 & 0xF) << 4) as u8 | (y2 >> 8) as u8,
            (y2 & 0xFF) as u8,
        ]);
        spu.push(0x06);
        spu.extend_from_slice(&(off1 as u16).to_be_bytes());
        spu.extend_from_slice(&(off2 as u16).to_be_bytes());
        spu.push(0xFF);
        debug_assert_eq!(spu.len(), cmds2, "cmds2 offset arithmetic");
        // DCSQT 2: date = end_date, self-chained (terminates).
        spu.extend_from_slice(&end_date.to_be_bytes());
        spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
        spu.push(0x02); // stop display at the DCSQT date
        spu.push(0xFF);
        let total = spu.len();
        spu[0..2].copy_from_slice(&(total as u16).to_be_bytes());
        spu[2..4].copy_from_slice(&(cmds as u16).to_be_bytes());
        spu
    }

    /// Wrap one SPU in a PES 0xBD with substream 0x20.
    fn wrap_pes(substream: u8, spu: &[u8]) -> Vec<u8> {
        let mut pes = Vec::new();
        pes.extend_from_slice(&[0x00, 0x00, 0x01, 0xBD]);
        let len = 3 + 1 + spu.len();
        pes.extend_from_slice(&(len as u16).to_be_bytes());
        pes.extend_from_slice(&[0x81, 0x00, 0x00]);
        pes.push(substream);
        pes.extend_from_slice(spu);
        pes
    }

    #[test]
    fn pair_decode_end_to_end() {
        // 4x2 SPU with a 2x2 visible center: line data paints c1
        // then fill c0 (transparent). Line = nibbles 6 (1px c2),
        // 0,0,0,0 (fill c0) -> [0x60, 0x00, 0x00].
        let top = [0x60u8, 0x00, 0x00];
        let bottom = [0x60u8, 0x00, 0x00];
        let spu = build_spu(&top, &bottom, 10, 20, 4, 2, 90, 450);
        let pes = wrap_pes(0x20, &spu);
        let idx = b"# VobSub index file, v7\nsize: 720x480\n\
            palette: 000000, ff0000, 00ff00, ffff00, ff0000, ff00ff, ffff00, ffffff,\n\
                      808080, 404040, 202020, 101010, 080808, 040404, 020202, 010101\n\
            id: en, index: 0\ntimestamp: 00:00:05:000, filepos: 000000000\n";
        let doc = parse_vobsub(idx, &pes);
        assert!(doc.is_clean(), "{:?}", doc.damage);
        assert_eq!(doc.cues.len(), 1);
        let cue = &doc.cues[0];
        // Crop: top line = c2 at col0 + 3 transparent cols; rows
        // keep col0 only -> 1x2 at (10,20).
        assert_eq!((cue.x, cue.y, cue.width, cue.height), (10, 20, 1, 2));
        // Palette: color 2 -> idx palette[colormap[2]=1] = ff0000 red,
        // alpha 15*17 = 255.
        assert_eq!(&cue.rgba[0..4], &[255, 0, 0, 255]);
        // Timing: idx 5s + start (90<<10)/90 = 1024ms; end
        // (450<<10)/90 = 5120ms.
        assert_eq!(cue.start_ns, (5_000 + 1_024) * 1_000_000);
        assert_eq!(cue.end_ns, Some((5_000 + 5_120) * 1_000_000));
        assert!(!cue.forced);
    }

    #[test]
    fn multi_pes_accumulation() {
        // SPU split across two PES fragments.
        let top = [0x60u8, 0x00, 0x00];
        let bottom = [0x60u8, 0x00, 0x00];
        let spu = build_spu(&top, &bottom, 0, 0, 4, 2, 0, 90);
        let (head, tail) = spu.split_at(spu.len() / 2);
        let pes1 = wrap_pes(0x20, head);
        let pes2 = wrap_pes(0x20, tail);
        let mut sub = pes1.clone();
        sub.extend_from_slice(&pes2);
        // One entry spanning both fragments.
        let idx = b"palette: 000000, 0000ff, 00ff00, ffff00, ff0000, ff00ff, ffff00, ffffff, 808080, 404040, 202020, 101010, 080808, 040404, 020202, 010101\nid: en, index: 0\ntimestamp: 00:00:01:000, filepos: 000000000\n";
        let doc = parse_vobsub(idx, &sub);
        assert!(doc.is_clean(), "{:?}", doc.damage);
        assert_eq!(doc.cues.len(), 1);
        // The same sub with a second idx entry pointing past the
        // fragments: both accumulate for entry 1 only.
        let idx2 = b"palette: 000000, 0000ff, 00ff00, ffff00, ff0000, ff00ff, ffff00, ffffff, 808080, 404040, 202020, 101010, 080808, 040404, 020202, 010101\nid: en, index: 0\ntimestamp: 00:00:01:000, filepos: 000000000\ntimestamp: 00:00:09:000, filepos: 000000100\n";
        let doc = parse_vobsub(idx2, &sub);
        assert_eq!(doc.cues.len(), 1);
        assert!(matches!(
            doc.damage.first(),
            Some(VobSubDamage::BadFilepos { filepos: 0x100 })
        ));
    }

    #[test]
    fn bad_filepos_and_missing_palette_are_typed() {
        let idx = b"id: en, index: 0\ntimestamp: 00:00:01:000, filepos: 000000200\n";
        let doc = parse_vobsub(idx, b"");
        assert!(matches!(
            doc.damage[0],
            VobSubDamage::NoPalette | VobSubDamage::BadFilepos { .. }
        ));
        assert!(doc.damage.contains(&VobSubDamage::BadFilepos { filepos: 0x200 }));
        assert!(doc.cues.is_empty());
    }

    #[test]
    fn grayscale_fallback_matches_reference_shape() {
        // No palette: one visible color -> level 0xFF -> white.
        let top = [0x60u8, 0x00, 0x00];
        let bottom = [0x60u8, 0x00, 0x00];
        let spu = build_spu(&top, &bottom, 0, 0, 2, 2, 0, 90);
        let decoded = decode_spu(&spu).unwrap();
        let p = guess_palette(&decoded);
        // colormap [0]=3,[1]=2,[2]=1,[3]=0; alpha [0]=0,[1]=0,[2]=15,[3]=0.
        // One opaque color (index 2, colormap 1): level_map[0][0]=0xFF.
        assert_eq!(p[2], Rgba { r: 255, g: 255, b: 255, a: 255 });
        assert_eq!(p[0].a, 0);
    }

    #[test]
    fn cue_contract_render() {
        let top = [0x60u8, 0x00, 0x00];
        let bottom = [0x60u8, 0x00, 0x00];
        let spu = build_spu(&top, &bottom, 360, 240, 4, 2, 0, 90);
        let pes = wrap_pes(0x20, &spu);
        let idx = b"size: 720x480\npalette: 000000, 0000ff, 00ff00, ffff00, ff0000, ff00ff, ffff00, ffffff, 808080, 404040, 202020, 101010, 080808, 040404, 020202, 010101\nid: en, index: 0\ntimestamp: 00:00:02:000, filepos: 000000000\n";
        let doc = parse_vobsub(idx, &pes);
        let cues = vobsub_cues(&doc, 7).unwrap();
        assert_eq!(cues.len(), 1);
        let cue = &cues[0];
        assert_eq!(cue.z, 7);
        assert_eq!(cue.layout.reference, Some((720, 480)));
        assert_eq!(cue.layout.anchor.u, Rational::new(360, 720).unwrap());
        assert_eq!(cue.layout.anchor.v, Rational::new(240, 480).unwrap());
        assert!(cue.active_at(cue.start.unwrap()));
        assert!(!cue.active_at(cue.end.unwrap()));
    }
}
