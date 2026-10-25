//! PGS (HDMV, Blu-ray) subtitles — segment grammar, epoch state
//! machine, RLE + palette decode to straight-alpha RGBA objects, and
//! the reduce to the 5.L1 compositing contract (card 5.N1, ADR-012
//! self-build, zero new dependencies).
//!
//! # What this module owns
//!
//! * [`segment`] — the segment walk (both framings: `.sup`-style
//!   PTS-prefixed and Matroska-stripped) and the typed payloads
//!   (PCS/WDS/PDS/ODS/END);
//! * [`palette`] — PDS 16-bit YCbCrA → straight-alpha RGBA8 (BT.601
//!   vs BT.709 by video height, the reference decoder's rule);
//! * [`rle`] — the PGS run-length codes (literal pixels, `0x00`
//!   escapes: EOL, 1–63 runs, 16-bit long runs) with typed
//!   clip/snap damage;
//! * this file — the **epoch state machine** (acquisition/epoch
//!   start flush, object and palette banks, multi-fragment ODS
//!   assembly, the END-of-display-set composition into
//!   [`PgsObjectImage`]s) and the render path to
//!   [`compositing::CompositedCue`]s;
//! * [`fuzz`] — the `fuzz_sub_pgs` harness.
//!
//! # Epoch lifecycle (the public mpv/mkvtoolnix/ffmpeg model)
//!
//! A PGS stream is a sequence of **display sets** — a PCS, optional
//! WDS/PDS/ODS segments, closed by END. The PCS composition state
//! drives the epoch machine: `EpochStart` (0x80) begins a new epoch
//! and releases every object and palette; `AcquisitionPoint` (0x40)
//! and `EpochContinue` (0xC0) release them too (the reference
//! flushes on every non-zero state); `Normal` (0x00) continues the
//! epoch — objects defined earlier stay valid, so a PCS + END pair
//! alone re-presents (or, with zero objects, clears) the screen.
//! At END the current composition renders: each PCS object ref
//! places its decoded bitmap at `(x, y)` on the PCS video extent.
//! A missing object skips (typed); a missing palette drops the set
//! (typed — the reference errors the whole frame).
//!
//! # Timing
//!
//! PGS has no explicit end times: a subtitle stays up until the
//! next display set replaces or clears it. [`PgsCueBuilder`] turns
//! consecutive display sets into half-open `[t0, t1)` spans — the
//! previous set stays up until one nanosecond before the next set's
//! PTS (the DVB builder's rule), open-ended for a trailing set.
//!
//! # The contract (dirty-world, D-4 + MP §8)
//!
//! [`PgsDecoder::decode_display_set`] takes one display-set payload
//! plus its 90 kHz PTS and returns a [`PgsDisplaySet`]: the composed
//! object images, the video extent, and a **capped typed damage
//! ledger**. It never panics; every malformation (truncated segment,
//! unknown segment id, palette-missing object, RLE overrun) is typed
//! data and the decode continues to the next segment.
//!
//! # Never
//!
//! I/O, unsafe, unbounded allocation (object canvases capped at
//! [`rle::MAX_OBJECT_PIXELS`], 8 palettes / 64 objects per epoch —
//! the reference caps), panics.

pub mod fuzz;
pub mod palette;
pub mod rle;
pub mod segment;

use std::collections::HashMap;

use crate::compositing::bitmap::{CompositedCue, CueBitmap, CueError};
use crate::compositing::layout::{
    Anchor, CueAlignment, CueLayout, HorizontalAlign, Rational, SafeMargin, VerticalAlign,
};
use crate::compositing::target::Rgba;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};

/// Damage ledger cap (the SRT 256-event precedent, sized for one
/// display set; the DVB precedent is 64).
pub const MAX_DAMAGE_EVENTS: usize = 64;

/// Max palettes live in one epoch (the reference's cap).
pub const MAX_EPOCH_PALETTES: usize = 8;
/// Max objects live in one epoch (the reference's cap).
pub const MAX_EPOCH_OBJECTS: usize = 64;

/// Typed decode damage (never a panic; the decode continues).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PgsDamage {
    /// A segment header ran past the payload end.
    TruncatedSegment {
        /// The segment kind (`None` when even the type byte is gone).
        kind: Option<&'static str>,
    },
    /// A segment payload was too short for its grammar.
    TruncatedPayload {
        kind: &'static str,
        needed: usize,
        got: usize,
    },
    /// A segment's declared size overran the payload — the walk
    /// stopped early.
    LengthOverrun,
    /// An unknown segment type byte (skipped).
    UnknownSegment { type_byte: u8 },
    /// The PCS referenced an object never defined in this epoch —
    /// the object was skipped.
    ObjectMissing { object_id: u16 },
    /// The PCS referenced a palette never defined in this epoch —
    /// the whole display set was dropped (the reference's
    /// invalid-palette error path).
    PaletteMissing { palette_id: u8 },
    /// The epoch's object/palette banks were full — the newcomer
    /// was dropped.
    EpochBankFull { bank: &'static str },
    /// An object's RLE crossed its canvas edge and was clipped.
    RleOverrun { object_id: u16 },
    /// A continuation ODS fragment overflowed the object's declared
    /// RLE length — the extra bytes were dropped.
    FragmentOverrun { object_id: u16 },
    /// The ODS declared a zero or over-cap bitmap — object disabled.
    ObjectSizeInvalid {
        object_id: u16,
        width: u16,
        height: u16,
    },
    /// A mid-stream ODS arrived with no first fragment in flight for
    /// its id — dropped.
    FragmentWithoutHeader { object_id: u16 },
    /// The damage ledger overflowed (count of dropped events).
    DamageOverflow { dropped: usize },
}

impl std::fmt::Display for PgsDamage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PgsDamage::TruncatedSegment { kind } => {
                write!(f, "truncated segment header ({kind:?})")
            }
            PgsDamage::TruncatedPayload { kind, needed, got } => {
                write!(f, "truncated {kind} payload: needed {needed}, got {got}")
            }
            PgsDamage::LengthOverrun => f.write_str("segment size overruns the payload"),
            PgsDamage::UnknownSegment { type_byte } => {
                write!(f, "unknown segment type 0x{type_byte:02X}")
            }
            PgsDamage::ObjectMissing { object_id } => {
                write!(f, "composition object {object_id} never defined (skipped)")
            }
            PgsDamage::PaletteMissing { palette_id } => write!(
                f,
                "composition palette {palette_id} never defined (display set dropped)"
            ),
            PgsDamage::EpochBankFull { bank } => write!(f, "epoch {bank} bank full (dropped)"),
            PgsDamage::RleOverrun { object_id } => {
                write!(f, "object {object_id} RLE crossed its canvas (clipped)")
            }
            PgsDamage::FragmentOverrun { object_id } => write!(
                f,
                "object {object_id} continuation fragment overflowed its RLE length"
            ),
            PgsDamage::ObjectSizeInvalid {
                object_id,
                width,
                height,
            } => write!(
                f,
                "object {object_id} size {width}x{height} invalid/over cap"
            ),
            PgsDamage::FragmentWithoutHeader { object_id } => write!(
                f,
                "object {object_id} continuation fragment with no first fragment"
            ),
            PgsDamage::DamageOverflow { dropped } => {
                write!(f, "damage ledger overflow: {dropped} events dropped")
            }
        }
    }
}

/// One composed object image: straight-alpha RGBA8 at its PCS
/// position (palette already applied).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgsObjectImage {
    pub object_id: u16,
    /// The PCS placement.
    pub x: u16,
    pub y: u16,
    pub width: u32,
    pub height: u32,
    /// The forced-on flag from the PCS (surfaced for the track's
    /// forced-subtitle handling).
    pub forced: bool,
    /// `width * height * 4` RGBA8 bytes.
    pub rgba: Vec<u8>,
}

/// The result of one display set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgsDisplaySet {
    /// The PTS in nanoseconds (the container PTS when provided, else
    /// the PCS segment's own `.sup` PTS).
    pub pts_ns: Option<i64>,
    /// The PCS video extent.
    pub video: (u32, u32),
    /// The HDMV frame-rate nibble (rendered via
    /// [`segment::frame_rate_id_table`]).
    pub frame_rate_id: u8,
    /// The composition number (monotonic per stream).
    pub composition_number: u16,
    /// The composed objects (empty = clear the display).
    pub objects: Vec<PgsObjectImage>,
    /// The capped damage ledger.
    pub damage: Vec<PgsDamage>,
}

impl PgsDisplaySet {
    /// Was this display set clean (no damage events)?
    pub fn is_clean(&self) -> bool {
        self.damage.is_empty()
    }
}

/// An object under assembly (the epoch's object bank).
#[derive(Debug, Clone)]
struct ObjectState {
    width: u32,
    height: u32,
    /// The reassembled RLE (fragments appended as they arrive).
    rle: Vec<u8>,
    /// The declared total RLE length minus what has arrived.
    remaining: usize,
}

/// The PGS epoch state machine.
#[derive(Debug, Clone)]
pub struct PgsDecoder {
    /// The framing the payload uses (sticky once detected).
    framing: Option<segment::PgsFraming>,
    /// The live PCS (video extent, palette ref, composition objects).
    pcs: Option<segment::Pcs>,
    /// The epoch's windows (tracked for the state machine; the
    /// reference places objects by their PCS x/y directly).
    windows: Vec<segment::Window>,
    /// The epoch's palettes.
    palettes: HashMap<u8, palette::Palette>,
    /// The epoch's objects.
    objects: HashMap<u16, ObjectState>,
    damage: Vec<PgsDamage>,
    damage_dropped: usize,
    /// Per-display-set flags.
    composed: bool,
    composed_objects: Vec<PgsObjectImage>,
    composed_video: (u32, u32),
    composed_frame_rate: u8,
    composed_number: u16,
    set_pts_ns: Option<i64>,
}

impl Default for PgsDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl PgsDecoder {
    /// A fresh decoder (no epoch yet).
    pub fn new() -> PgsDecoder {
        PgsDecoder {
            framing: None,
            pcs: None,
            windows: Vec::new(),
            palettes: HashMap::new(),
            objects: HashMap::new(),
            damage: Vec::new(),
            damage_dropped: 0,
            composed: false,
            composed_objects: Vec::new(),
            composed_video: (0, 0),
            composed_frame_rate: 0,
            composed_number: 0,
            set_pts_ns: None,
        }
    }

    /// Reset all state (a stream discontinuity / seek: the next
    /// epoch-start PCS rebuilds everything anyway, but a seek to a
    /// `Normal` PCS mid-epoch would otherwise present stale objects).
    pub fn reset(&mut self) {
        *self = PgsDecoder::new();
    }

    fn damage(&mut self, event: PgsDamage) {
        if self.damage.len() < MAX_DAMAGE_EVENTS {
            self.damage.push(event);
        } else {
            self.damage_dropped += 1;
        }
    }

    /// Decode one display-set payload (one PES payload / Matroska
    /// block's worth of segments) with its 90 kHz PTS. The framing is
    /// detected on the first call and reused after (streams do not
    /// switch mid-play; an explicit override is [`Self::set_framing`]).
    pub fn decode_display_set(&mut self, payload: &[u8], pts_90khz: Option<i64>) -> PgsDisplaySet {
        let framing = self
            .framing
            .unwrap_or_else(|| segment::PgsFraming::detect(payload));
        self.framing = Some(framing);
        self.decode_framed(payload, framing, pts_90khz)
    }

    /// Force the framing (carriage knowledge beats detection).
    pub fn set_framing(&mut self, framing: segment::PgsFraming) {
        self.framing = Some(framing);
    }

    /// The framing in use, once detected.
    pub fn framing(&self) -> Option<segment::PgsFraming> {
        self.framing
    }

    fn decode_framed(
        &mut self,
        payload: &[u8],
        framing: segment::PgsFraming,
        pts_90khz: Option<i64>,
    ) -> PgsDisplaySet {
        self.composed = false;
        self.set_pts_ns = pts_90khz.and_then(|ticks| {
            Timestamp::from_parts(ticks, 1, 90_000)
                .ok()
                .and_then(|ts| ts.as_nanos().ok())
                .map(|ns| ns.0 as i64)
        });

        let (segments, used) = segment::walk_segments(payload, framing);
        if payload.is_empty() || used < payload.len() {
            // The walk stopped early (or never started): either a
            // bare header remnant or a size overrun past the last
            // whole segment.
            if segments.is_empty() {
                self.damage(PgsDamage::TruncatedSegment { kind: None });
            } else {
                self.damage(PgsDamage::LengthOverrun);
            }
        }

        for seg in &segments {
            match seg.kind {
                segment::SegmentKind::Pcs => self.pcs_segment(seg),
                segment::SegmentKind::Wds => self.wds_segment(seg),
                segment::SegmentKind::Pds => self.pds_segment(seg),
                segment::SegmentKind::Ods => self.ods_segment(seg),
                segment::SegmentKind::End => self.compose(),
                segment::SegmentKind::Unknown(b) => {
                    self.damage(PgsDamage::UnknownSegment { type_byte: b });
                }
            }
        }

        // Streams that omit the END segment: compose once a PCS has
        // been seen (the DVB missing-end-of-display-set fallback).
        if !self.composed && self.pcs.is_some() {
            self.compose();
        }

        let mut damage = std::mem::take(&mut self.damage);
        if self.damage_dropped > 0 {
            damage.push(PgsDamage::DamageOverflow {
                dropped: self.damage_dropped,
            });
            self.damage_dropped = 0;
        }
        PgsDisplaySet {
            pts_ns: self.set_pts_ns,
            video: self.composed_video,
            frame_rate_id: self.composed_frame_rate,
            composition_number: self.composed_number,
            objects: std::mem::take(&mut self.composed_objects),
            damage,
        }
    }

    fn pcs_segment(&mut self, seg: &segment::RawSegment<'_>) {
        // The PCS segment's own PTS (.sup framing) fills in a
        // missing container PTS.
        if self.set_pts_ns.is_none()
            && let Some(pts) = seg.pts_90khz
        {
            self.set_pts_ns = Timestamp::from_parts(i64::from(pts), 1, 90_000)
                .ok()
                .and_then(|ts| ts.as_nanos().ok())
                .map(|ns| ns.0 as i64);
        }
        let Some(pcs) = segment::parse_pcs(seg.data) else {
            self.damage(PgsDamage::TruncatedPayload {
                kind: "presentation-composition",
                needed: 11,
                got: seg.data.len(),
            });
            return;
        };
        if pcs.state.flushes_epoch() {
            // Epoch start / acquisition / epoch continue: the whole
            // bank is released (the reference's flush_cache).
            self.palettes.clear();
            self.objects.clear();
            self.windows.clear();
        }
        // A palette-update PCS re-uses the referenced palette's
        // entries with only the deltas applied — the reference does
        // not implement it (logged); we honor the palette ref.
        self.pcs = Some(pcs);
    }

    fn wds_segment(&mut self, seg: &segment::RawSegment<'_>) {
        self.windows = segment::parse_wds(seg.data);
    }

    fn pds_segment(&mut self, seg: &segment::RawSegment<'_>) {
        let Some(pds) = segment::parse_pds(seg.data) else {
            self.damage(PgsDamage::TruncatedPayload {
                kind: "palette-definition",
                needed: 2,
                got: seg.data.len(),
            });
            return;
        };
        if let Some(existing) = self.palettes.get(&pds.id)
            && existing.version == pds.version
        {
            // Unchanged palette — a repeated PDS is a no-op.
            return;
        }
        if self.palettes.len() >= MAX_EPOCH_PALETTES && !self.palettes.contains_key(&pds.id) {
            self.damage(PgsDamage::EpochBankFull { bank: "palette" });
            return;
        }
        // The matrix rule keys off the CURRENT video height (the
        // PCS preceding this PDS in the same display set).
        let matrix = palette::ColorMatrix::for_video_height(
            self.pcs.as_ref().map_or(0, |p| u32::from(p.height)),
        );
        self.palettes.insert(
            pds.id,
            palette::Palette::from_entries(pds.id, pds.version, pds.entries.into_iter(), matrix),
        );
    }

    fn ods_segment(&mut self, seg: &segment::RawSegment<'_>) {
        let Some(ods) = segment::parse_ods(seg.data) else {
            self.damage(PgsDamage::TruncatedPayload {
                kind: "object-definition",
                needed: 4,
                got: seg.data.len(),
            });
            return;
        };
        if ods.first {
            let Some((total, width, height)) = ods.header else {
                // parse_ods already guarantees a header on first
                // fragments — unreachable, typed defensively.
                self.damage(PgsDamage::TruncatedPayload {
                    kind: "object-definition header",
                    needed: 7,
                    got: seg.data.len().saturating_sub(4),
                });
                return;
            };
            let (w, h) = (u32::from(width), u32::from(height));
            let valid = w > 0 && h > 0 && w.saturating_mul(h) <= rle::MAX_OBJECT_PIXELS;
            if !valid {
                self.damage(PgsDamage::ObjectSizeInvalid {
                    object_id: ods.id,
                    width,
                    height,
                });
                self.objects.remove(&ods.id);
                return;
            }
            if self.objects.len() >= MAX_EPOCH_OBJECTS && !self.objects.contains_key(&ods.id) {
                self.damage(PgsDamage::EpochBankFull { bank: "object" });
                return;
            }
            // The declared total covers width+height too (the
            // reference subtracts 2*2).
            let declared = (total as usize).saturating_sub(4);
            let frag = ods.fragment.len().min(declared);
            if frag < ods.fragment.len() {
                self.damage(PgsDamage::FragmentOverrun { object_id: ods.id });
            }
            self.objects.insert(
                ods.id,
                ObjectState {
                    width: w,
                    height: h,
                    rle: ods.fragment[..frag].to_vec(),
                    remaining: declared - frag,
                },
            );
        } else {
            // Continuation fragment: append to the in-flight object.
            let take = match self.objects.get_mut(&ods.id) {
                Some(object) => {
                    let take = ods.fragment.len().min(object.remaining);
                    object.rle.extend_from_slice(&ods.fragment[..take]);
                    object.remaining -= take;
                    take
                }
                None => {
                    self.damage(PgsDamage::FragmentWithoutHeader { object_id: ods.id });
                    return;
                }
            };
            if take < ods.fragment.len() {
                self.damage(PgsDamage::FragmentOverrun { object_id: ods.id });
            }
        }
    }

    /// Compose the current epoch state into object images (the
    /// END-of-display-set action).
    fn compose(&mut self) {
        self.composed = true;
        let Some(pcs) = self.pcs.clone() else {
            return;
        };
        self.composed_video = (u32::from(pcs.width), u32::from(pcs.height));
        self.composed_frame_rate = pcs.frame_rate_id;
        self.composed_number = pcs.composition_number;

        let mut out = Vec::new();
        if pcs.objects.is_empty() || pcs.width == 0 || pcs.height == 0 {
            // Zero composition objects (the reference's blank-display
            // path) or a degenerate extent: the display clears.
            self.composed_objects = out;
            return;
        }
        // The palette ref is per-composition (not per-object).
        let Some(palette) = self.palettes.get(&pcs.palette_id).cloned() else {
            self.damage(PgsDamage::PaletteMissing {
                palette_id: pcs.palette_id,
            });
            self.composed_objects = out;
            return;
        };
        for comp in &pcs.objects {
            let Some(object) = self.objects.get(&comp.id).cloned() else {
                self.damage(PgsDamage::ObjectMissing { object_id: comp.id });
                continue;
            };
            let (indices, rle_damage) =
                rle::decode_object(&object.rle, object.width, object.height);
            if rle_damage.any() {
                self.damage(PgsDamage::RleOverrun { object_id: comp.id });
            }
            let mut rgba = Vec::with_capacity(indices.len() * 4);
            for &index in &indices {
                let c = palette.rgba[index as usize];
                rgba.extend_from_slice(&[c.r, c.g, c.b, c.a]);
            }
            out.push(PgsObjectImage {
                object_id: comp.id,
                x: comp.x,
                y: comp.y,
                width: object.width,
                height: object.height,
                forced: comp.forced,
                rgba,
            });
        }
        self.composed_objects = out;
    }
}

/// Render display sets into compositing cues (the 5.L1 contract):
/// each composed object becomes a [`CompositedCue`] with its exact
/// PCS position as a rational anchor over the video extent (the
/// [`CueLayout::reference`] for presentation-extent scaling), and
/// the half-open span closes one nanosecond before the next set's
/// PTS (the DVB builder's rule; a trailing set is open-ended).
#[derive(Debug, Clone)]
pub struct PgsCueBuilder;

impl Default for PgsCueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PgsCueBuilder {
    /// A cue builder.
    pub fn new() -> PgsCueBuilder {
        PgsCueBuilder
    }

    /// Convert one display set (with the NEXT set's PTS in
    /// nanoseconds, when known) into cues. `z_base` orders objects
    /// within the composition (PCS object order).
    ///
    /// # Errors
    /// [`CueError`] from the bitmap/span validation.
    pub fn cues(
        &self,
        set: &PgsDisplaySet,
        next_pts_ns: Option<i64>,
        z_base: u16,
    ) -> Result<Vec<CompositedCue>, CueError> {
        let mut cues = Vec::new();
        let start = set.pts_ns;
        let end = match (next_pts_ns, start) {
            (Some(next), Some(start)) if next > start => Some(next - 1),
            _ => None,
        };
        // The 1/1e9 nanosecond base: `num`/`den` are coprime and
        // non-zero, so this literal is exactly `TimeBase::new(1,
        // 1_000_000_000)`'s Ok value (its gcd reduction is a no-op)
        // — the DVB builder's construction, reused verbatim.
        let ns_base = TimeBase {
            num: 1,
            den: 1_000_000_000,
        };
        let ns = |v: Option<i64>| -> Stamp {
            match v {
                Some(ns) => Stamp::Present(Timestamp::new(ns, ns_base)),
                None => Stamp::Missing,
            }
        };
        for (i, object) in set.objects.iter().enumerate() {
            if object.width == 0 || object.height == 0 {
                continue;
            }
            let bitmap = CueBitmap::from_rgba(
                object.width,
                object.height,
                object.height,
                object.rgba.clone(),
            )?;
            let span = TimeSpan {
                start: ns(start),
                end: ns(end),
            };
            cues.push(CompositedCue::new(
                span,
                bitmap,
                Self::layout_for(set.video, object.x, object.y),
                z_base.saturating_add(i as u16),
            )?);
        }
        Ok(cues)
    }

    /// The exact-position layout: anchor at `(x/video_w, y/video_h)`
    /// of the video extent, top-left aligned, scaling with the
    /// presentation extent via [`CueLayout::reference`] (the DVB
    /// builder's placement vocabulary, one source of truth).
    fn layout_for(video: (u32, u32), x: u16, y: u16) -> CueLayout {
        let (w, h) = (video.0.max(1), video.1.max(1));
        let u = Rational::new(u32::from(x), w).unwrap_or(Rational::ZERO);
        let v = Rational::new(u32::from(y), h).unwrap_or(Rational::ZERO);
        CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
            alignment: CueAlignment {
                horizontal: HorizontalAlign::Start,
                vertical: VerticalAlign::Top,
            },
            margin: SafeMargin::TitleSafe,
            reference: Some(video),
        }
    }
}

/// The transparent-black RGBA constant the palette banks default
/// to (PDS entries left unmentioned stay transparent — the
/// reference's zero-filled CLUT).
pub(crate) const TRANSPARENT_BLACK: Rgba = Rgba::BLACK.with_alpha(0);

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    // ---------- hand-crafted PGS builders ----------
    //
    // HONEST LABEL: these vectors are hand-crafted for this card
    // against the ffmpeg pgssubdec.c semantics fetched 2026-10-25 —
    // no reference encoder exists in this sandbox (ffmpeg refuses
    // text->PGS transcoding), so there is no differential oracle;
    // the grammar is cross-checked line-by-line instead.

    fn seg(kind: u8, data: &[u8]) -> Vec<u8> {
        let mut v = vec![kind];
        v.extend_from_slice(&(data.len() as u16).to_be_bytes());
        v.extend_from_slice(data);
        v
    }

    fn pcs(state: u8, palette_id: u8, objects: &[(u16, u16, u16)], video: (u16, u16)) -> Vec<u8> {
        let mut d = vec![
            video.0.to_be_bytes()[0],
            video.0.to_be_bytes()[1],
            video.1.to_be_bytes()[0],
            video.1.to_be_bytes()[1],
            0x20, // frame-rate nibble 2 = 24 fps
            0x00,
            0x01, // composition number 1
            state,
            0x00, // no palette update
            palette_id,
            objects.len() as u8,
        ];
        for &(id, x, y) in objects {
            d.extend_from_slice(&id.to_be_bytes());
            d.push(0); // window id
            d.push(0); // flags: not forced, not cropped
            d.extend_from_slice(&x.to_be_bytes());
            d.extend_from_slice(&y.to_be_bytes());
        }
        seg(0x16, &d)
    }

    fn pds(id: u8, version: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
        let mut d = vec![id, version];
        for &(eid, y, cr, cb, a) in entries {
            d.extend_from_slice(&[eid, y, cr, cb, a]);
        }
        seg(0x14, &d)
    }

    fn ods_first(id: u16, width: u16, height: u16, rle: &[u8]) -> Vec<u8> {
        let total = 4 + rle.len();
        ods_first_declared(id, width, height, total, rle)
    }

    /// A first ODS fragment whose declared total covers MORE than
    /// this fragment (multi-fragment objects).
    fn ods_first_declared(id: u16, width: u16, height: u16, total: usize, rle: &[u8]) -> Vec<u8> {
        let mut d = id.to_be_bytes().to_vec();
        d.push(0); // version
        d.push(0x80); // first fragment
        d.push((total >> 16) as u8);
        d.push((total >> 8) as u8);
        d.push(total as u8);
        d.extend_from_slice(&width.to_be_bytes());
        d.extend_from_slice(&height.to_be_bytes());
        d.extend_from_slice(rle);
        seg(0x15, &d)
    }

    fn ods_cont(id: u16, rle: &[u8]) -> Vec<u8> {
        let mut d = id.to_be_bytes().to_vec();
        d.push(0); // version
        d.push(0x40); // continuation (last)
        d.extend_from_slice(rle);
        seg(0x15, &d)
    }

    const END: &[u8] = &[0x80, 0x00, 0x00];

    /// Palette 0: entry 1 = opaque studio white, entry 2 = opaque
    /// studio black (BT.709 — 1080-line video).
    const WHITE_ENTRY: (u8, u8, u8, u8, u8) = (1, 235, 128, 128, 255);
    const BLACK_ENTRY: (u8, u8, u8, u8, u8) = (2, 16, 128, 128, 255);

    #[test]
    fn epoch_lifecycle_full_display_set() {
        // Epoch start: 2x2 object of entry-1 pixels at (10, 20) on
        // a 1920x1080 canvas.
        let rle = [0x00, 0x82, 0x01, 0x00, 0x00, 0x00, 0x82, 0x01];
        let buf = [
            pcs(0x80, 0, &[(1, 10, 20)], (1920, 1080)),
            pds(0, 0, &[WHITE_ENTRY, BLACK_ENTRY]),
            ods_first(1, 2, 2, &rle),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(0));
        assert!(set.is_clean(), "damage: {:?}", set.damage);
        assert_eq!(set.video, (1920, 1080));
        assert_eq!(set.frame_rate_id, 2);
        assert_eq!(set.composition_number, 1);
        assert_eq!(set.objects.len(), 1);
        let img = &set.objects[0];
        assert_eq!((img.x, img.y, img.width, img.height), (10, 20, 2, 2));
        assert_eq!(img.rgba.len(), 16);
        // Every pixel is opaque white (235,128,128 -> 255s).
        assert!(
            img.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|c| c == &[255, 255, 255, 255])
        );
        assert_eq!(dec.framing(), Some(segment::PgsFraming::Stripped));
    }

    #[test]
    fn epoch_normal_reuses_objects_palette_update() {
        let rle = [0x00, 0x82, 0x01, 0x00, 0x00, 0x00, 0x82, 0x02];
        let mut dec = PgsDecoder::new();
        // Set 1: define object 1 + palette 0.
        let buf = [
            pcs(0x80, 0, &[(1, 0, 0)], (1920, 1080)),
            pds(0, 0, &[WHITE_ENTRY]),
            ods_first(1, 2, 2, &rle),
            END.to_vec(),
        ]
        .concat();
        let set1 = dec.decode_display_set(&buf, Some(0));
        assert!(set1.is_clean());
        // Set 2: NORMAL state, only a new palette version (entry 1
        // now black) + PCS + END — object 1 persists from the epoch.
        let buf2 = [
            pds(0, 1, &[BLACK_ENTRY]),
            pcs(0x00, 0, &[(1, 100, 200)], (1920, 1080)),
            END.to_vec(),
        ]
        .concat();
        let set2 = dec.decode_display_set(&buf2, Some(90_000));
        assert!(set2.is_clean(), "damage: {:?}", set2.damage);
        assert_eq!(set2.objects.len(), 1);
        let img = &set2.objects[0];
        assert_eq!((img.x, img.y), (100, 200));
        // The composed image renders from the LIVE palette: the
        // version-1 PDS REPLACED it (the reference's zero-filled
        // CLUT) — entry 1 reverted to transparent, entry 2 is the
        // new opaque studio black.
        assert_eq!(&img.rgba[0..4], &[0, 0, 0, 0], "entry 1 transparent");
        assert_eq!(&img.rgba[8..12], &[0, 0, 0, 255], "entry 2 black");
        assert_eq!(set2.pts_ns, Some(1_000_000_000));
    }

    #[test]
    fn empty_composition_clears_and_epoch_start_flushes() {
        let rle = [0x00, 0x82, 0x01, 0x00, 0x00, 0x00, 0x82, 0x01];
        let mut dec = PgsDecoder::new();
        let buf = [
            pcs(0x80, 0, &[(1, 0, 0)], (1920, 1080)),
            pds(0, 0, &[WHITE_ENTRY]),
            ods_first(1, 2, 2, &rle),
            END.to_vec(),
        ]
        .concat();
        let _ = dec.decode_display_set(&buf, Some(0));
        // Empty PCS at Normal: clear (0 objects, no damage).
        let buf = [pcs(0x00, 0, &[], (1920, 1080)), END.to_vec()].concat();
        let set = dec.decode_display_set(&buf, Some(90_000));
        assert!(set.is_clean());
        assert!(set.objects.is_empty());
        // Epoch start flushes: the old object/palette are gone, so a
        // Normal PCS referencing them is typed damage.
        let buf = [pcs(0x80, 0, &[], (1920, 1080)), END.to_vec()].concat();
        let _ = dec.decode_display_set(&buf, Some(180_000));
        let buf = [pcs(0x00, 0, &[(1, 0, 0)], (1920, 1080)), END.to_vec()].concat();
        let set = dec.decode_display_set(&buf, Some(270_000));
        assert_eq!(set.objects.len(), 0);
        assert!(matches!(
            set.damage.first(),
            Some(PgsDamage::PaletteMissing { palette_id: 0 })
        ));
    }

    #[test]
    fn multi_fragment_ods_assembles() {
        // 4x1 object: three fragments, the first declaring the full
        // 8-byte RLE total (4 header + 8 = 12).
        let buf = [
            pcs(0x80, 0, &[(7, 0, 0)], (1280, 720)),
            pds(0, 0, &[WHITE_ENTRY]),
            ods_first_declared(7, 4, 1, 4 + 8, &[0x01, 0x01]),
            ods_cont(7, &[0x01, 0x01]),
            ods_cont(7, &[0x01, 0x01, 0x01, 0x01]),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(0));
        assert!(set.is_clean(), "damage: {:?}", set.damage);
        assert_eq!(set.objects.len(), 1);
        assert_eq!(set.objects[0].rgba.len(), 16);
    }

    #[test]
    fn palette_missing_drops_set_object_missing_skips() {
        // Object defined but palette never: set dropped, typed.
        let rle = [0x81, 0x01, 0x81, 0x01];
        let buf = [
            pcs(0x80, 5, &[(1, 0, 0)], (1920, 1080)),
            ods_first(1, 2, 2, &rle),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(0));
        assert!(set.objects.is_empty());
        assert!(matches!(
            set.damage.first(),
            Some(PgsDamage::PaletteMissing { palette_id: 5 })
        ));
        // Palette fine but object 2 never defined: skipped, typed.
        let buf = [
            pcs(0x80, 0, &[(1, 0, 0), (2, 4, 4)], (1920, 1080)),
            pds(0, 0, &[WHITE_ENTRY]),
            ods_first(1, 2, 2, &rle),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(0));
        assert_eq!(set.objects.len(), 1);
        assert!(
            set.damage
                .iter()
                .any(|d| matches!(d, PgsDamage::ObjectMissing { object_id: 2 }))
        );
    }

    #[test]
    fn truncated_and_unknown_segments_are_typed() {
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&[], Some(0));
        assert!(matches!(
            set.damage.first(),
            Some(PgsDamage::TruncatedSegment { kind: None })
        ));
        // Unknown type byte + a PCS truncated below its grammar.
        let mut buf = seg(0x77, &[1, 2, 3]);
        buf.extend_from_slice(&[0x16, 0x00, 0x05, 1, 2, 3, 4, 5]);
        let set = dec.decode_display_set(&buf, Some(0));
        assert!(
            set.damage
                .iter()
                .any(|d| matches!(d, PgsDamage::UnknownSegment { type_byte: 0x77 }))
        );
        assert!(set.damage.iter().any(|d| matches!(
            d,
            PgsDamage::TruncatedPayload {
                kind: "presentation-composition",
                ..
            }
        )));
        // Declared size overruns past whole segments: LengthOverrun.
        let buf: &[u8] = &[
            0x14, 0x00, 0x02, 0xAA, 0xBB, // a whole 2-byte PDS
            0x14, 0x00, 0x40, 1, 2, // then a 64-byte declaration
        ];
        let set = dec.decode_display_set(buf, Some(0));
        assert!(
            set.damage
                .iter()
                .any(|d| matches!(d, PgsDamage::LengthOverrun))
        );
    }

    #[test]
    fn rle_overrun_and_short_fragment_are_typed() {
        // A 256-run into a 2x2 canvas.
        let buf = [
            pcs(0x80, 0, &[(1, 0, 0)], (1920, 1080)),
            pds(0, 0, &[WHITE_ENTRY]),
            ods_first(1, 2, 2, &[0x00, 0x42, 0x00]),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(0));
        assert_eq!(set.objects.len(), 1, "clipped run still renders");
        assert!(
            set.damage
                .iter()
                .any(|d| matches!(d, PgsDamage::RleOverrun { object_id: 1 }))
        );
        // Continuation without a first fragment.
        let buf = [
            pcs(0x80, 0, &[], (1920, 1080)),
            ods_cont(9, &[1, 2, 3]),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(0));
        assert!(
            set.damage
                .iter()
                .any(|d| matches!(d, PgsDamage::FragmentWithoutHeader { object_id: 9 }))
        );
    }

    #[test]
    fn invalid_object_size_is_typed() {
        let buf = [
            pcs(0x80, 0, &[], (1920, 1080)),
            ods_first(1, 0, 0, &[0x81, 0x01]),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(0));
        assert!(set.damage.iter().any(|d| matches!(
            d,
            PgsDamage::ObjectSizeInvalid {
                object_id: 1,
                width: 0,
                height: 0
            }
        )));
    }

    #[test]
    fn sup_framing_uses_segment_pts_when_container_silent() {
        let rle = [0x00, 0x82, 0x01, 0x00, 0x00, 0x00, 0x82, 0x01];
        let set = |pts: u32| -> Vec<u8> {
            let s = |kind: u8, data: &[u8]| -> Vec<u8> {
                let mut v = pts.to_be_bytes().to_vec();
                v.push(kind);
                v.extend_from_slice(&(data.len() as u16).to_be_bytes());
                v.extend_from_slice(data);
                v
            };
            let mut ods_data = vec![
                0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0x0C, 0x00, 0x02, 0x00, 0x02,
            ];
            ods_data.extend_from_slice(&rle);
            let pcs_data = [
                0x07, 0x80, 0x04, 0x38, 0x20, 0x00, 0x01, 0x80, 0x00, 0x00, 0x01, //
                0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ];
            [
                s(0x16, &pcs_data),
                s(0x14, &[0x00, 0x00, 0x01, 235, 128, 128, 255]),
                s(0x15, &ods_data),
                s(0x80, &[]),
            ]
            .concat()
        };
        let mut dec = PgsDecoder::new();
        let s = dec.decode_display_set(&set(90_000), None);
        assert!(s.is_clean(), "damage: {:?}", s.damage);
        assert_eq!(s.pts_ns, Some(1_000_000_000), "segment PTS fills in");
        assert_eq!(s.objects.len(), 1);
    }

    #[test]
    fn cue_builder_spans_and_layout() {
        let rle = [0x00, 0x82, 0x01, 0x00, 0x00, 0x00, 0x82, 0x01];
        let buf = [
            pcs(0x80, 0, &[(1, 960, 540)], (1920, 1080)),
            pds(0, 0, &[WHITE_ENTRY]),
            ods_first(1, 2, 2, &rle),
            END.to_vec(),
        ]
        .concat();
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(90_000));
        let cues = PgsCueBuilder::new()
            .cues(&set, Some(2_000_000_000), 0)
            .must();
        assert_eq!(cues.len(), 1);
        let cue = &cues[0];
        // [1s, 2s): half-open at both ends, one ns before the next.
        assert_eq!(cue.start, Some(1_000_000_000));
        assert_eq!(cue.end, Some(2_000_000_000 - 1));
        assert!(cue.active_at(1_000_000_000));
        assert!(cue.active_at(1_999_999_998));
        assert!(!cue.active_at(1_999_999_999));
        assert_eq!(cue.bitmap.width, 2);
        // The anchor is the exact PCS position over the video extent.
        assert_eq!(cue.layout.anchor.u, Rational::new(960, 1920).must());
        assert_eq!(cue.layout.anchor.v, Rational::new(540, 1080).must());
        assert_eq!(cue.layout.reference, Some((1920, 1080)));
        // No next PTS: open-ended.
        let open = PgsCueBuilder::new().cues(&set, None, 3).must();
        assert_eq!(open[0].end, None);
        assert_eq!(open[0].z, 3);
    }

    #[test]
    fn damage_ledger_is_capped() {
        let mut dec = PgsDecoder::new();
        // 80 unknown segments in one payload: the cap is 64 + the
        // overflow marker.
        let mut buf = Vec::new();
        for _ in 0..80 {
            buf.extend_from_slice(&seg(0x77, &[1]));
        }
        let set = dec.decode_display_set(&buf, Some(0));
        assert!(set.damage.len() <= MAX_DAMAGE_EVENTS + 1);
        assert!(matches!(
            set.damage.last(),
            Some(PgsDamage::DamageOverflow { dropped: 16 })
        ));
    }
}
