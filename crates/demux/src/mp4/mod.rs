//! The MP4/MOV/3GP demuxer (card 5.C1): ISO-BMFF box walk —
//! progressive (`moov`/`stbl` sample tables) and fragmented
//! (`moof`/`traf`/`trun` with `mvex`/`trex` defaults), `edts`/`elst`
//! timeline application, chapter references (`tref`/`chap` +
//! QuickTime text samples, `chpl` fallback), QuickTime and 3GP brand
//! quirks, keyframe-indexed seek.
//!
//! # Engine choice (recorded for review)
//!
//! **Self-built box walker — Option B.** `mp4parse` 0.17.0 is MPL-2.0
//! (doc 11 §3.2 allowlists it; license was *not* the blocker) but
//! doc `feasibility/sections/subsystems.md` §1.2 records that its
//! public `read_mp4()` **skips `moof`/`mdat`/`sidx` at top level and
//! never surfaces fragments** — the card's fMP4 ACs (frag corpora
//! demux + seek) are unimplementable on it, and the demux crate is
//! C5 std-only-zero-dep. The clean-room reference is the test-side
//! walker in `crates/codecs/tests/common/mod.rs` (card 2.B4/2.B5),
//! generalized here.
//!
//! # Design (recorded for review)
//!
//! - **Declared sizes are hints; the stream is authority** (the AVI/
//!   WAV precedent): every box extent is clamped to parent ∩ stream;
//!   a structural box (`moov`, `moof`) cut by the end of the file is
//!   the typed `truncated` refusal, never a silent short parse.
//!   `size == 0` means "to end of parent/stream" (streaming-writer
//!   convention); `size == 1` reads the 64-bit `largesize` (which
//!   must be ≥ 16 or it is a structural lie).
//! - **Structural boxes are read whole, ceiling-checked** — `moov` ≤
//!   [`MAX_MOOV_BYTES`], each `moof` ≤ [`MAX_MOOF_BYTES`] — then
//!   walked in memory with a depth guard ([`MAX_BOX_DEPTH`]) and a
//!   child>parent check on every level. Sample payloads are *never*
//!   read whole: `next_event` pulls exactly one sample's bytes
//!   through the pooled [`PayloadSlots`] arena (2 × 16 MiB ceiling,
//!   the house zero-copy scheme).
//! - **Timing is the tables, not arithmetic**: `stts` deltas + `ctts`
//!   composition offsets build the per-sample DTS/PTS ladders in
//!   media-timescale ticks (mdhd); `elst` entries shift them — a
//!   normal edit's `media_time` subtracts (encoder-delay/gapless
//!   trimming), an empty edit's `segment_duration` delays the
//!   timeline start. v0 applies the first `elst` entry per track
//!   (the overwhelmingly common shape; multi-edit splices are a
//!   damage-noted gap, not a silent first-entry-only lie — see
//!   [`Mp4Demuxer`]'s limits).
//! - **Interleave order is file order**: progressive samples across
//!   all tracks merge by file offset (the `mdat` interleave truth);
//!   fragment samples emit per-moof in DTS order.
//! - **`stss` absence means all-samples-sync** (ISO 14496-12 §8.6.2)
//!   — the same reading ffmpeg and mp4parse apply.
//! - **fMP4 base-offset**: `tfhd` `base-data-offset` when present,
//!   else the enclosing `moof`'s file offset (the
//!   `default-base-is-moof` DASH convention; a file that relies on
//!   the spec's carry-forward-from-previous-traf rule without either
//!   is damage-noted and still lands on the moof base).
//! - **Seek**: progressive = `stss` keyframe walk-back over the
//!   per-sample offset table (`Fast { indexed: true }`); fragmented =
//!   bounded moof-scan (`SlowLinear` — sidx is parsed-and-skipped in
//!   v0, an honest recorded gap). Both are compute-then-commit (D4):
//!   every landing is computed for all tracks before any cursor
//!   moves.
//!
//! # v0 scope (documented limits — honest, not smoothed)
//!
//! - `sidx` is located and skipped; segment-index seek is a gap
//!   (moof-scan covers the AC).
//! - Only the first `elst` entry per track is applied; multi-edit
//!   splices damage-note `elst-multi-entry`.
//! - `cmov` (compressed MOV) is refused typed (bad-box vocabulary).
//! - Chapter titles read QuickTime text samples (16-bit
//!   length-prefixed) and `chpl`; other chapter codecs damage-note.
//! - 3GP: brand family (`3gp4/3gp5/3gg6/3g2a/3g2b`) and the
//!   `samr/sawb/s263` sample entries are mapped; 3GP-specific text
//!   styling is not interpreted (doc 04's honest-subset note).
//! - The whole first `stsd` entry payload rides
//!   `DecoderConfig::extra_data` (the WAV `fmt ` precedent) — the
//!   avcC/hvcC/esds/alac parse belongs to the codec cards.

use std::collections::VecDeque;

use media_types::error::ErrorContext;
use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::riff::{PayloadSlots, SourceReader};

pub mod error;
pub mod fuzz;
pub mod test_support;

#[cfg(test)]
mod tests;

/// Maximum single-sample payload trusted into the packet pool
/// `[PROPOSAL: 16 MiB — above every sane video sample; larger
/// declared samples damage-note and skip rather than allocate]`.
pub const MAX_SAMPLE_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum `moov` payload trusted into memory for the structural walk
/// `[PROPOSAL: 16 MiB — the largest real movie headers are low-MiB]`.
pub const MAX_MOOV_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum single `moof` payload trusted into memory
/// `[PROPOSAL: 1 MiB — fragment headers are KB-scale]`.
pub const MAX_MOOF_BYTES: u64 = 1024 * 1024;

/// Maximum per-track samples materialized from the sample tables
/// `[PROPOSAL: 1 Mi samples ≈ 48 MiB of `Sample` records — the
/// 24 h corpus still lands under it]`.
pub const MAX_SAMPLES: usize = 1024 * 1024;

/// Box-descent depth guard (`moov→trak→mdia→minf→stbl` is five;
/// `moov→udta→meta→ilst→©nam→data` is six).
pub const MAX_BOX_DEPTH: u32 = 8;

/// Top-level boxes the fragment scan walks past (bounded budget).
pub const MAX_FRAGMENT_SCAN_BOXES: usize = 65_536;

/// Brands whose `ftyp` (major or compatible) this demuxer claims.
const KNOWN_BRANDS: [&[u8; 4]; 20] = [
    b"isom", b"iso2", b"iso3", b"iso4", b"iso5", b"iso6", b"mp41", b"mp42", b"mp71", b"avc1",
    b"qt  ", b"3gp4", b"3gp5", b"3gg6", b"3g2a", b"3g2b", b"mmp4", b"dash", b"isml", b"M4A ",
];

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "m4a", "mov", "qt", "3gp", "3g2", "3ga", "3gpp",
];

/// `sample_is_non_sync_sample` (ISO 14496-12 §8.8.3.1 SampleFlags,
/// bit 15). Also matches ffmpeg's `MOV_FRAG_SAMPLE_FLAG_IS_NON_SYNC`
/// bit 16 in practice: both encodings appear in the wild, and
/// degradation-priority high bits are effectively unused.
const SAMPLE_IS_NON_SYNC: u32 = 0x0000_8000;

/// The probe: do the leading bytes look like ISO-BMFF this demuxer
/// claims? Accepts an `ftyp` whose major **or** any compatible brand
/// is known (isom/mp42/qt/3gp families), or a leading `moov`/`wide`/
/// `free` (pre-ftyp QuickTime). A probe never claims a format it
/// cannot see: fewer than 8 readable bytes refuse.
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    if header.len() < 8 {
        return false;
    }
    // A box header is size(4) then kind(4).
    let kind = [header[4], header[5], header[6], header[7]];
    if &kind == b"ftyp" {
        // Brands: major(4) + minor(4) + compatible list.
        let mut i = 8;
        while i + 4 <= header.len() {
            let brand = [header[i], header[i + 1], header[i + 2], header[i + 3]];
            if KNOWN_BRANDS.iter().any(|b| **b == brand) {
                return true;
            }
            i += 4;
        }
        return false;
    }
    // QuickTime legacy: files may open with wide/free before moov.
    matches!(&kind, b"moov" | b"wide" | b"free" | b"skip")
}

/// One chapter marker (the demuxer-side record; the engine maps it
/// to its chapter vocabulary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    /// Chapter start on the movie timeline.
    pub start: Nanoseconds,
    /// Title (lossy UTF-8 — advisory display text, never an error).
    pub title: String,
}

/// One expanded sample (progressive tables or one trun run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sample {
    offset: u64,
    size: u64,
    /// DTS in media ticks (cumulative stts deltas; for fragments,
    /// tfdt + cumulative trun deltas).
    dts: u64,
    /// Composition offset (ctts / trun cts) in media ticks.
    cts: i64,
    /// Duration in media ticks.
    dur: u64,
    /// Whether the sample starts a sync point.
    sync: bool,
}

/// `mvex`/`trex` per-track fragment defaults (inherited when a tfhd
/// or trun field is absent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrexDefaults {
    duration: u32,
    size: u32,
    flags: u32,
}

/// Per-track demux state (the `trak` parse product).
struct TrackState {
    track_id: u32,
    kind: MediaKind,
    codec: String,
    /// The verbatim first `stsd` entry payload (≤
    /// [`MAX_MOOV_BYTES`], rides `DecoderConfig::extra_data`).
    extra: Option<SharedBytes>,
    /// Media timescale (mdhd).
    timescale: u32,
    base: TimeBase,
    /// Media duration in ticks (mdhd; 0 when fragmented-only).
    media_duration: u64,
    /// `elst` first entry: empty-edit delay (media ticks).
    empty_edit_delay: u64,
    /// `elst` first entry: media_time shift (media ticks, ≥ 0).
    media_time: u64,
    /// `tref`/`chap` says this track *is* a chapter track.
    is_chapter: bool,
    /// Progressive per-sample table (empty for fragmented tracks).
    samples: Vec<Sample>,
    /// Cursor: next sample index to emit.
    next: usize,
    /// trex defaults when the track is fragmented.
    trex: Option<TrexDefaults>,
    language: Option<String>,
    damage: Vec<(u64, &'static str)>,
}

/// A replayable error shape (the WAV `StickyShape` precedent).
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

/// One in-memory box: kind + payload range (within the structural
/// buffer it was parsed from).
#[derive(Debug, Clone)]
struct RawBox {
    kind: [u8; 4],
    body: std::ops::Range<usize>,
}

/// The parsed top-level box header (kind, payload start, declared
/// end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TopBox {
    kind: [u8; 4],
    payload_start: u64,
    end: u64,
}

/// The MP4/MOV demuxer.
///
/// Construct with [`Mp4Demuxer::new`] over a seekable source stream,
/// then drive it through the [`Demuxer`] trait: `start` walks
/// `ftyp`+`moov` (progressive sample tables or `mvex` fragment
/// defaults), enumerates tracks and chapters; `next_event` yields
/// samples in file order (progressive merge or moof-scan);
/// `seek` lands on the keyframe-safe position ≤ target.
pub struct Mp4Demuxer {
    reader: SourceReader,
    pool: PayloadSlots,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    major_brand: [u8; 4],
    compatible_brands: Vec<[u8; 4]>,
    movie_timescale: u32,
    fragmented: bool,
    tracks: Vec<TrackState>,
    chapters: Vec<Chapter>,
    pending: VecDeque<DemuxEvent>,
    /// Progressive emission order: (file offset, track idx, sample
    /// idx), sorted — the mdat interleave truth.
    order: Vec<(u64, usize, usize)>,
    order_cursor: usize,
    /// Fragment walk: next top-level position to inspect.
    frag_pos: Option<u64>,
    /// Damage notes (bytes, reason) surfaced as Recovered events.
    damage: Vec<(u64, &'static str)>,
}

/// u32 big-endian at `at`, `None` past the end.
fn u32be(data: &[u8], at: usize) -> Option<u32> {
    let b = data.get(at..at + 4)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// u16 big-endian at `at`, `None` past the end.
fn u16be(data: &[u8], at: usize) -> Option<u16> {
    let b = data.get(at..at + 2)?;
    Some(u16::from_be_bytes([b[0], b[1]]))
}

/// u64 big-endian at `at`, `None` past the end.
fn u64be(data: &[u8], at: usize) -> Option<u64> {
    let b = data.get(at..at + 8)?;
    let mut v = [0u8; 8];
    v.copy_from_slice(b);
    Some(u64::from_be_bytes(v))
}

/// i64 big-endian at `at`, `None` past the end.
fn i64be(data: &[u8], at: usize) -> Option<i64> {
    u64be(data, at).map(|v| v as i64)
}

/// Walk boxes in a structural buffer (`data[range]`), enforcing
/// header truncation, 64-bit sizes, size==0 (to end), non-advancing
/// extents and child>parent.
fn iter_boxes(data: &[u8], start: usize, end: usize) -> Result<Vec<RawBox>, EngineError> {
    let mut out = Vec::new();
    let mut pos = start;
    while pos < end {
        if pos + 8 > end {
            // Header cut by the parent's declared end.
            return Err(error::truncated(end as u64, data.len() as u64));
        }
        let size32 = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let kind = [data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]];
        let (header_len, declared_end) = if size32 == 1 {
            let large = u64be(data, pos + 8)
                .ok_or_else(|| error::truncated((pos + 16) as u64, data.len() as u64))?;
            if large < 16 {
                return Err(error::bad_box(pos as u64));
            }
            (16u64, (pos as u64).checked_add(large))
        } else if size32 == 0 {
            // Extends to the end of the parent.
            (8u64, Some(end as u64))
        } else {
            (8u64, (pos as u64).checked_add(u64::from(size32)))
        };
        let Some(declared_end) = declared_end else {
            return Err(error::bad_box(pos as u64));
        };
        if declared_end > end as u64 {
            return Err(error::box_overruns_parent(end as u64));
        }
        if declared_end < pos as u64 + header_len {
            return Err(error::bad_box(pos as u64));
        }
        out.push(RawBox {
            kind,
            body: (pos + header_len as usize)..declared_end as usize,
        });
        pos = declared_end as usize;
    }
    Ok(out)
}

/// Read one top-level box header at `pos` from the source. `Ok(None)`
/// when `pos >= len`. A header cut by the stream's end is the typed
/// truncation (callers that tolerate it — the fragment scan —
/// clamp first).
fn read_top(reader: &mut SourceReader, pos: u64) -> Result<Option<TopBox>, EngineError> {
    let len = reader.length();
    if pos >= len {
        return Ok(None);
    }
    let clamped = pos.min(len.saturating_sub(8));
    let head = reader.read_prefix_at(16, clamped)?;
    if head.len() < 8 || clamped != pos {
        return Err(error::truncated(pos + 8, len));
    }
    // A box header is size(4) then kind(4).
    let kind = [head[4], head[5], head[6], head[7]];
    let size32 = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let (header_len, end) = if size32 == 1 {
        if head.len() < 16 {
            return Err(error::truncated(pos + 16, len));
        }
        let mut v = [0u8; 8];
        v.copy_from_slice(&head[8..16]);
        let large = u64::from_be_bytes(v);
        if large < 16 {
            return Err(error::bad_box(pos));
        }
        let Some(end) = pos.checked_add(large) else {
            return Err(error::bad_box(pos));
        };
        (16u64, end)
    } else if size32 == 0 {
        (8u64, len)
    } else {
        let Some(end) = pos.checked_add(u64::from(size32)) else {
            return Err(error::bad_box(pos));
        };
        (8u64, end)
    };
    if end < pos + header_len {
        return Err(error::bad_box(pos));
    }
    Ok(Some(TopBox {
        kind,
        payload_start: pos + header_len,
        end,
    }))
}

/// Map an `stsd` entry fourcc to the registry codec descriptor.
/// Returns `None` for entries outside the v0 vocabulary (the trak
/// is skipped with a damage note — never a fake codec name).
fn codec_of(kind: &[u8; 4]) -> Option<&'static str> {
    match kind {
        b"avc1" | b"avc2" | b"avc3" | b"avc4" | b"avcp" => Some("h264"),
        b"hvc1" | b"hev1" => Some("hevc"),
        b"mp4v" => Some("mpeg4video"),
        b"mjpa" | b"mjpb" | b"jpeg" => Some("mjpeg"),
        b"vp08" => Some("vp8"),
        b"vp09" => Some("vp9"),
        b"av01" => Some("av1"),
        b"s263" | b"h263" => Some("h263"),
        // Audio.
        b"mp4a" => Some("aac"),
        b"alac" => Some("alac"),
        b"ac-3" => Some("ac3"),
        b"ec-3" => Some("eac3"),
        b"opus" => Some("opus"),
        b"fLaC" => Some("flac"),
        b"sowt" => Some("pcm_s16le"),
        b"twos" | b"lpcm" | b"raw " => Some("pcm_s16be"),
        b"in24" => Some("pcm_s24be"),
        b"in32" => Some("pcm_s32be"),
        b"fl32" => Some("pcm_f32be"),
        b"fl64" => Some("pcm_f64be"),
        b"ulaw" => Some("pcm_mulaw"),
        b"alaw" => Some("pcm_alaw"),
        b".mp3" => Some("mp3"),
        b"samr" => Some("amr_nb"),
        b"sawb" => Some("amr_wb"),
        // Text.
        b"tx3g" | b"text" => Some("mov_text"),
        _ => None,
    }
}

/// MediaKind from an `hdlr` handler fourcc.
fn kind_of(handler: &[u8; 4]) -> MediaKind {
    match handler {
        b"vide" => MediaKind::Video,
        b"soun" => MediaKind::Audio,
        b"sbtl" | b"text" | b"subt" | b"clcp" => MediaKind::Subtitle,
        _ => MediaKind::Data,
    }
}

/// Unpack the mdhd 3×5-bit language code; `None` for "und".
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}

/// Look for an `esds` child inside an stsd entry payload and read its
/// ObjectTypeIndication; `Some(0x6b)` means the mp4a entry actually
/// carries MPEG-1 Layer III (the mislabeled-mp3 quirk).
fn esds_oti(entry: &[u8]) -> Option<u8> {
    // The entry payload's fixed prefix before child config boxes
    // (esds et al.): 6 reserved + 2 data-ref + 12 fixed (channelcount
    // ..samplerate 16.16) = 20 (ISO 14496-12 §12.4.3).
    let start = 20;
    if entry.len() < start + 12 {
        return None;
    }
    let boxes = iter_boxes(entry, start, entry.len()).ok()?;
    for b in boxes {
        if b.kind == *b"esds" {
            let body = entry.get(b.body.start..b.body.end)?;
            // ES_Descriptor(0x03) expandable-len ... DecoderConfig(0x04) len OTI
            let mut at = 4; // version/flags
            if body.get(at) == Some(&0x03) {
                at += 1;
                // Expandable length (1..4 bytes).
                for _ in 0..4 {
                    let byte = *body.get(at)?;
                    at += 1;
                    if byte & 0x80 == 0 {
                        break;
                    }
                }
                at += 3; // ES_ID + flags (no-dependency flags in practice)
                if body.get(at) == Some(&0x04) {
                    at += 1;
                    for _ in 0..4 {
                        let byte = *body.get(at)?;
                        at += 1;
                        if byte & 0x80 == 0 {
                            break;
                        }
                    }
                    return body.get(at).copied();
                }
            }
        }
    }
    None
}

impl Mp4Demuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// No container I/O happens here (the trait's `start` does the
    /// parsing).
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unseekable/unknown
    /// length/empty sources (the `SourceReader` contract).
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<Mp4Demuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(Mp4Demuxer {
            reader,
            pool: PayloadSlots::new(2, MAX_SAMPLE_BYTES as usize),
            started: false,
            eos: false,
            sticky: None,
            major_brand: *b"\0\0\0\0",
            compatible_brands: Vec::new(),
            movie_timescale: 1000,
            fragmented: false,
            tracks: Vec::new(),
            chapters: Vec::new(),
            pending: VecDeque::new(),
            order: Vec::new(),
            order_cursor: 0,
            frag_pos: None,
            damage: Vec::new(),
        })
    }

    /// The `ftyp` major brand (observability).
    #[must_use]
    pub fn major_brand(&self) -> &[u8; 4] {
        &self.major_brand
    }

    /// The `ftyp` compatible brands (observability).
    #[must_use]
    pub fn compatible_brands(&self) -> &[[u8; 4]] {
        &self.compatible_brands
    }

    /// Whether the file is fragmented (mvex/moof path).
    #[must_use]
    pub fn is_fragmented(&self) -> bool {
        self.fragmented
    }

    /// The chapter markers (tref/chap text tracks + chpl), movie
    /// timeline order.
    #[must_use]
    pub fn chapters(&self) -> &[Chapter] {
        &self.chapters
    }

    /// Bytes of pool backing (the D-1 observable for 5.X2).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// The verbatim first `stsd` entry payload of a track — the
    /// bytes the engine hands the decoder as
    /// `DecoderConfig::extra_data` (the WAV `fmt ` precedent: the
    /// avcC/hvcC/esds/alac parse belongs to the codec cards).
    #[must_use]
    pub fn sample_entry_bytes(&self, track: TrackId) -> Option<&[u8]> {
        let t = self.tracks.get(track.handle() as usize)?;
        t.extra.as_deref()
    }

    /// Record damage and queue a Recovered event for it.
    fn note(&mut self, bytes: u64, reason: &'static str) {
        self.damage.push((bytes, reason));
    }

    fn surface_damage(&mut self) {
        let notes = std::mem::take(&mut self.damage);
        for (bytes, reason) in notes {
            self.pending.push_back(DemuxEvent::Recovered {
                skipped: bytes,
                reason,
            });
        }
    }

    /// Sticky-fail: remember the shape and return it forever after.
    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    // ---- start: the top-level walk + moov parse -------------------

    fn open(&mut self) -> Result<Vec<Track>, EngineError> {
        let len = self.reader.length();
        let head = self.reader.read_prefix_at(32, 0)?;
        if !probe(&head) {
            return Err(error::not_mp4(&head));
        }
        // Scan top-level for ftyp + moov (moov may trail mdat).
        let mut pos = 0u64;
        let mut moov: Option<(u64, u64)> = None; // (payload_start, end)
        let mut boxes_seen = 0usize;
        let mut unknown_top = 0u64;
        while let Some(header) = {
            // Clamp reads to the stream; a mid-header cut ends the scan.
            if pos + 8 > len {
                None
            } else {
                read_top(&mut self.reader, pos)?
            }
        } {
            boxes_seen += 1;
            if boxes_seen > MAX_FRAGMENT_SCAN_BOXES {
                return Err(error::scan_budget_exhausted());
            }
            let end = header.end.min(len.max(header.payload_start));
            match &header.kind {
                b"ftyp" => {
                    let body = self.reader.read_prefix_at(
                        (end - header.payload_start).min(64) as usize,
                        header.payload_start,
                    )?;
                    if body.len() >= 8 {
                        self.major_brand = [body[0], body[1], body[2], body[3]];
                        let mut i = 8;
                        while i + 4 <= body.len() {
                            self.compatible_brands.push([
                                body[i],
                                body[i + 1],
                                body[i + 2],
                                body[i + 3],
                            ]);
                            i += 4;
                        }
                    }
                }
                b"moov" => {
                    if header.end > len {
                        return Err(error::truncated(header.end, len));
                    }
                    moov = Some((header.payload_start, header.end));
                    // Keep scanning only for ftyp-after-moov oddities? No:
                    // moov is what we need; stop.
                    break;
                }
                b"free" | b"skip" | b"wide" | b"mdat" | b"pdin" | b"sidx" => {}
                _ => {
                    unknown_top += 1;
                }
            }
            if header.end <= pos {
                break; // defensive: never spin
            }
            pos = header.end;
        }
        let Some((moov_start, moov_end)) = moov else {
            return Err(error::no_moov());
        };
        if unknown_top > 0 {
            self.note(unknown_top, "unknown-top-box-skipped");
        }
        let moov_len = moov_end - moov_start;
        if moov_len > MAX_MOOV_BYTES {
            return Err(error::oversized_box(b"moov", MAX_MOOV_BYTES, moov_len));
        }
        let mut data = vec![0u8; moov_len as usize];
        let got = self.reader.read_prefix_into(&mut data, moov_start)?;
        if got as u64 != moov_len {
            return Err(error::truncated(moov_end, moov_start + got as u64));
        }
        self.parse_moov(&data)?;
        if !self.tracks.iter().any(|t| !t.is_chapter) {
            return Err(error::no_tracks());
        }
        // Decide the packet path.
        let any_samples = self.tracks.iter().any(|t| !t.samples.is_empty());
        self.fragmented = self.tracks.iter().any(|t| t.trex.is_some())
            || (!any_samples && self.any_top_box(b"moof"));
        if self.fragmented {
            // Fragment scan starts after the moov (moofs never precede
            // the moov they extend).
            self.frag_pos = Some(moov_end);
            if !any_samples {
                // Pure fMP4: fine. Mixed (some tracks progressive, some
                // not) is damage-noted.
                if self.tracks.iter().any(|t| !t.trex.is_some()) {
                    self.note(0, "mixed-progressive-fragment-tracks");
                }
            }
        } else if !any_samples {
            return Err(error::no_sample_table(moov_start));
        }
        // Build the progressive emission order (file-offset merge).
        if !self.fragmented {
            let mut order: Vec<(u64, usize, usize)> = Vec::new();
            for (ti, t) in self.tracks.iter().enumerate() {
                for (si, s) in t.samples.iter().enumerate() {
                    order.push((s.offset, ti, si));
                }
            }
            order.sort_unstable();
            self.order = order;
        }
        // Chapter titles from the referenced text tracks.
        self.read_chapter_titles()?;
        self.started = true;
        let out: Vec<Track> = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.is_chapter)
            .map(|(ordinal, t)| self.track_record(ordinal, t))
            .collect::<Result<Vec<_>, EngineError>>()?;
        self.surface_damage();
        Ok(out)
    }

    /// Whether a top-level box of this kind exists (bounded scan).
    fn any_top_box(&mut self, kind: &[u8; 4]) -> bool {
        let len = self.reader.length();
        let mut pos = 0u64;
        let mut seen = 0usize;
        while pos + 8 <= len && seen < MAX_FRAGMENT_SCAN_BOXES {
            seen += 1;
            match read_top(&mut self.reader, pos) {
                Ok(Some(header)) => {
                    if &header.kind == kind {
                        return true;
                    }
                    if header.end <= pos {
                        return false;
                    }
                    pos = header.end;
                }
                Ok(None) => return false,
                Err(_) => return false,
            }
        }
        false
    }

    /// The public `Track` record for a track state.
    fn track_record(&self, ordinal: usize, t: &TrackState) -> Result<Track, EngineError> {
        let start = t.empty_edit_delay as i64 - t.media_time as i64;
        let end = if t.media_duration == 0 {
            Stamp::Missing
        } else {
            Stamp::Present(Timestamp::new(
                start.saturating_add(t.media_duration as i64),
                t.base,
            ))
        };
        let codec = CodecDescriptor::new(&t.codec).map_err(|_| error::bad_box(0))?;
        let language = t
            .language
            .as_ref()
            .and_then(|tag| media_types::ids::LanguageTag::parse(tag).ok());
        Ok(Track {
            id: TrackId::new(ordinal as u32),
            stream: StreamId::new(ordinal as u32),
            kind: t.kind,
            language,
            codec,
            title: None,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Present(Timestamp::new(start, t.base)),
                    end,
                },
                base: t.base,
                gaps: Vec::new(),
            },
        })
    }

    // ---- moov parse (in-memory over the whole box) ----------------

    fn parse_moov(&mut self, data: &[u8]) -> Result<(), EngineError> {
        // `data` is the moov *payload* (children start immediately).
        let inner = iter_boxes(data, 0, data.len())?;
        // mvhd.
        if let Some(mvhd) = inner.iter().find(|b| b.kind == *b"mvhd") {
            let body = &data[mvhd.body.clone()];
            let version = body.first().copied().unwrap_or(1);
            let at = if version == 1 { 20 } else { 12 };
            if let Some(scale) = u32be(body, at).filter(|&s| s > 0) {
                self.movie_timescale = scale;
            }
        }
        // trex defaults (per track_id) + chapter refs.
        let mut trex: Vec<(u32, TrexDefaults)> = Vec::new();
        let mut chapter_ids: Vec<u32> = Vec::new();
        for b in &inner {
            match &b.kind {
                b"mvex" => {
                    let mvex = iter_boxes(data, b.body.start, b.body.end)?;
                    for t in mvex {
                        if t.kind == *b"trex" {
                            let body = &data[t.body.clone()];
                            let id = u32be(body, 4).unwrap_or(0);
                            trex.push((
                                id,
                                TrexDefaults {
                                    duration: u32be(body, 16).unwrap_or(0),
                                    size: u32be(body, 20).unwrap_or(0),
                                    flags: u32be(body, 24).unwrap_or(0),
                                },
                            ));
                        }
                    }
                }
                b"udta" => {
                    if let Err(err) = self.parse_udta_chpl(data, b.body.clone()) {
                        self.note(0, "chpl-unparseable");
                        let _ = err;
                    }
                }
                _ => {}
            }
        }
        for b in inner.iter().filter(|b| b.kind == *b"trak") {
            // tref/chap collection pass.
            if let Some(ids) = trak_chap_refs(data, b.body.clone())? {
                chapter_ids.extend(ids);
            }
        }
        for b in inner.iter().filter(|b| b.kind == *b"trak") {
            if let Err(err) = self.parse_trak(data, b.body.clone(), &trex) {
                // A track that fails to parse is damage, not a dead
                // file — unless it is the only one (no_tracks later).
                self.note(0, "trak-unparseable");
                let _ = err;
            }
        }
        // Mark chapter tracks.
        for t in &mut self.tracks {
            if chapter_ids.contains(&t.track_id) {
                t.is_chapter = true;
            }
        }
        // Surface per-track damage collected during the parse.
        let mut track_damage: Vec<(u64, &'static str)> = Vec::new();
        for t in &mut self.tracks {
            track_damage.append(&mut t.damage);
        }
        self.damage.append(&mut track_damage);
        Ok(())
    }

    /// QuickTime `chpl` chapters (moov/udta/chpl) — fallback when no
    /// chap-referenced text track exists.
    fn parse_udta_chpl(
        &mut self,
        data: &[u8],
        range: std::ops::Range<usize>,
    ) -> Result<(), EngineError> {
        let boxes = iter_boxes(data, range.start, range.end)?;
        let Some(chpl) = boxes.iter().find(|b| b.kind == *b"chpl") else {
            return Ok(());
        };
        let body = &data[chpl.body.clone()];
        // version/flags(4) + reserved(4)? Nero layout: 1+3+4 then
        // count(u8?) — ffmpeg reads: 4 skip, 4 skip, u8 count... The
        // widely-interop shape: v/f(4) reserved(4) count(u8) then
        // entries { u64 start(100ns), u8 len, bytes }.
        let mut at = 9usize;
        let Some(count) = body.get(8).copied() else {
            return Err(error::bad_box(0));
        };
        for _ in 0..count.min(64) {
            let Some(start) = u64be(body, at) else {
                break;
            };
            at += 8;
            let Some(tlen) = body.get(at).copied() else {
                break;
            };
            at += 1;
            let Some(bytes) = body.get(at..(at + tlen as usize)) else {
                break;
            };
            at += tlen as usize;
            let nanos = start / 100;
            self.chapters.push(Chapter {
                start: Nanoseconds(nanos),
                title: String::from_utf8_lossy(bytes).to_string(),
            });
        }
        Ok(())
    }

    /// One `trak` → TrackState (progressive tables; trex attach for
    /// fragmented).
    fn parse_trak(
        &mut self,
        data: &[u8],
        range: std::ops::Range<usize>,
        trex: &[(u32, TrexDefaults)],
    ) -> Result<(), EngineError> {
        let boxes = iter_boxes(data, range.start, range.end)?;
        let mut track_id = 0u32;
        let mut language = None;
        let mut timescale = 0u32;
        let mut media_duration = 0u64;
        let mut handler = *b"\0\0\0\0";
        let mut empty_edit_delay = 0u64;
        let mut media_time = 0u64;
        let mut multi_edit = false;
        let mut stbl: Option<std::ops::Range<usize>> = None;
        let mut tkhd_flags = 0u32;
        for b in &boxes {
            match &b.kind {
                b"tkhd" => {
                    let body = &data[b.body.clone()];
                    let version = body.first().copied().unwrap_or(0);
                    tkhd_flags = u32be(body, 0).unwrap_or(0) & 0x00FF_FFFF;
                    let at = if version == 1 { 20 } else { 12 };
                    track_id = u32be(body, at).unwrap_or(0);
                }
                b"edts" => {
                    let edts = iter_boxes(data, b.body.start, b.body.end)?;
                    if let Some(elst) = edts.iter().find(|x| x.kind == *b"elst") {
                        let body = &data[elst.body.clone()];
                        let version = body.first().copied().unwrap_or(0);
                        let count = u32be(body, 4).unwrap_or(0);
                        if count > 1 {
                            multi_edit = true;
                        }
                        if count >= 1 {
                            let (seg_dur, media) = if version == 1 {
                                (u64be(body, 8).unwrap_or(0), i64be(body, 16).unwrap_or(0))
                            } else {
                                (
                                    u64::from(u32be(body, 8).unwrap_or(0)),
                                    // v0 media_time is a signed i32.
                                    i64::from(u32be(body, 12).unwrap_or(0) as i32),
                                )
                            };
                            if media == -1 {
                                empty_edit_delay = seg_dur;
                            } else if media > 0 {
                                media_time = media as u64;
                            }
                        }
                    }
                }
                b"mdia" => {
                    let mdia = iter_boxes(data, b.body.start, b.body.end)?;
                    for m in &mdia {
                        match &m.kind {
                            b"mdhd" => {
                                let body = &data[m.body.clone()];
                                let version = body.first().copied().unwrap_or(0);
                                let (ts_at, dur_at, lang_at) = if version == 1 {
                                    (20, 24, 28)
                                } else {
                                    (12, 16, 20)
                                };
                                timescale = u32be(body, ts_at).unwrap_or(0);
                                media_duration = if version == 1 {
                                    u64be(body, dur_at).unwrap_or(0)
                                } else {
                                    u64::from(u32be(body, dur_at).unwrap_or(0))
                                };
                                if let Some(packed) = u16be(body, lang_at) {
                                    language = unpack_language(packed);
                                }
                            }
                            b"hdlr" => {
                                let body = &data[m.body.clone()];
                                if body.len() >= 12 {
                                    handler = [body[8], body[9], body[10], body[11]];
                                }
                            }
                            b"minf" => {
                                let minf = iter_boxes(data, m.body.start, m.body.end)?;
                                if let Some(s) = minf.iter().find(|x| x.kind == *b"stbl") {
                                    stbl = Some(s.body.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if track_id == 0 || timescale == 0 {
            // No timing: not a demuxable trak.
            return Err(error::bad_box(range.start as u64));
        }
        let base = TimeBase::new(1, timescale).map_err(|_| error::bad_box(range.start as u64))?;
        // stsd codec.
        let Some(stbl_range) = stbl else {
            return Err(error::no_sample_table(range.start as u64));
        };
        let stbl_boxes = iter_boxes(data, stbl_range.start, stbl_range.end)?;
        let stsd = stbl_boxes
            .iter()
            .find(|b| b.kind == *b"stsd")
            .ok_or_else(|| error::no_sample_table(range.start as u64))?;
        let body = &data[stsd.body.clone()];
        // version/flags(4) entry_count(4) entries{ size(4) fourcc(4) ... }
        if body.len() < 16 {
            return Err(error::bad_box(stsd.body.start as u64));
        }
        let entry_size = u32be(body, 8).unwrap_or(0) as usize;
        let entry_kind = [body[12], body[13], body[14], body[15]];
        // The entry's declared size covers its own 8-byte header; the
        // payload handed downstream starts after it.
        if entry_size < 8 || 8 + entry_size > body.len() {
            return Err(error::bad_box(stsd.body.start as u64));
        }
        let entry = body.get(16..8 + entry_size).unwrap_or(&[]);
        let mut codec = codec_of(&entry_kind);
        // The mp4a-mislabeled-mp3 quirk (esds OTI 0x6b).
        if entry_kind == *b"mp4a" && esds_oti(entry) == Some(0x6b) {
            codec = Some("mp3");
        }
        let Some(codec) = codec else {
            self.note(entry_size as u64, "unknown-stsd-entry");
            return Err(error::bad_box(stsd.body.start as u64));
        };
        let extra = SharedBytes::from_vec(entry.to_vec());
        let trex_entry = trex.iter().find(|(id, _)| *id == track_id).map(|(_, d)| *d);
        let mut track = TrackState {
            track_id,
            kind: kind_of(&handler),
            codec: codec.to_string(),
            extra: Some(extra),
            timescale,
            base,
            media_duration,
            empty_edit_delay,
            media_time,
            is_chapter: false,
            samples: Vec::new(),
            next: 0,
            trex: trex_entry,
            language,
            damage: Vec::new(),
        };
        if tkhd_flags & 0x1 == 0 {
            track.damage.push((0, "track-disabled-in-tkhd"));
        }
        if multi_edit {
            track.damage.push((0, "elst-multi-entry"));
        }
        // Progressive tables.
        let mut sizes: Vec<u64> = Vec::new();
        let mut deltas: Vec<u64> = Vec::new();
        let mut offsets: Vec<u64> = Vec::new();
        let mut stsc: Vec<(u64, u64)> = Vec::new();
        let mut sync_set: Option<Vec<u32>> = None;
        let mut cts: Vec<i64> = Vec::new();
        for b in &stbl_boxes {
            let body = &data[b.body.clone()];
            match &b.kind {
                b"stsz" => {
                    let fixed = u32be(body, 4).unwrap_or(0);
                    let count = u32be(body, 8).unwrap_or(0) as usize;
                    if fixed != 0 {
                        sizes = vec![u64::from(fixed); count.min(MAX_SAMPLES)];
                        if count > MAX_SAMPLES {
                            self.note((count - MAX_SAMPLES) as u64, "stsz-count-over-ceiling");
                        }
                    } else {
                        for i in 0..count.min(MAX_SAMPLES) {
                            sizes.push(u64::from(u32be(body, 12 + i * 4).unwrap_or(0)));
                        }
                        if count > MAX_SAMPLES {
                            self.note((count - MAX_SAMPLES) as u64, "stsz-count-over-ceiling");
                        }
                    }
                }
                b"stz2" => {
                    let count = u32be(body, 8).unwrap_or(0) as usize;
                    let field = u32be(body, 4).unwrap_or(0) & 0xFF;
                    for i in 0..count.min(MAX_SAMPLES) {
                        let v = if field == 16 {
                            u64::from(u16be(body, 12 + i * 2).unwrap_or(0))
                        } else {
                            u64::from(body.get(12 + i).copied().unwrap_or(0))
                        };
                        sizes.push(v);
                    }
                    if count > MAX_SAMPLES {
                        self.note((count - MAX_SAMPLES) as u64, "stz2-count-over-ceiling");
                    }
                }
                b"stts" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        let run = u32be(body, 8 + i * 8).unwrap_or(0) as u64;
                        let delta = u64::from(u32be(body, 12 + i * 8).unwrap_or(0));
                        let take =
                            run.min((MAX_SAMPLES as u64).saturating_sub(deltas.len() as u64));
                        for _ in 0..take {
                            deltas.push(delta);
                        }
                        if run > take {
                            self.note(run - take, "stts-count-over-ceiling");
                        }
                    }
                }
                b"stco" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        offsets.push(u64::from(u32be(body, 8 + i * 4).unwrap_or(0)));
                    }
                }
                b"co64" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        offsets.push(u64be(body, 8 + i * 8).unwrap_or(0));
                    }
                }
                b"stsc" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        let first = u64::from(u32be(body, 8 + i * 12).unwrap_or(1));
                        let per = u64::from(u32be(body, 12 + i * 12).unwrap_or(0));
                        stsc.push((first, per));
                    }
                }
                b"stss" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    let mut set = Vec::with_capacity(count.min(MAX_SAMPLES));
                    for i in 0..count.min(MAX_SAMPLES) {
                        set.push(u32be(body, 8 + i * 4).unwrap_or(0));
                    }
                    sync_set = Some(set);
                }
                b"ctts" => {
                    let version = body.first().copied().unwrap_or(0);
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        let run = u32be(body, 8 + i * 8).unwrap_or(0) as u64;
                        let off = if version == 1 {
                            i64be(body, 12 + i * 8).unwrap_or(0)
                        } else {
                            // v0 composition offsets are signed i32.
                            i64::from(u32be(body, 12 + i * 8).unwrap_or(0) as i32)
                        };
                        let take = run.min((MAX_SAMPLES as u64).saturating_sub(cts.len() as u64));
                        for _ in 0..take {
                            cts.push(off);
                        }
                        if run > take {
                            self.note(run - take, "ctts-count-over-ceiling");
                        }
                    }
                }
                _ => {}
            }
        }
        // Expand to per-sample records (progressive only; fragmented
        // tracks carry empty tables).
        let count = sizes.len().min(deltas.len());
        if !offsets.is_empty() && count > 0 {
            let mut samples: Vec<Sample> = Vec::with_capacity(count);
            let mut si = 0usize;
            let mut dts = 0u64;
            let mut sample_no = 1u32;
            for (ci, chunk_offset) in offsets.iter().copied().enumerate() {
                // stsc run for this chunk (1-based).
                let chunk_1 = ci as u64 + 1;
                let mut per = stsc.last().map_or(0, |(_, p)| *p);
                for (first, p) in &stsc {
                    if *first <= chunk_1 {
                        per = *p;
                    }
                }
                let mut offset = chunk_offset;
                for _ in 0..per {
                    if si >= count {
                        break;
                    }
                    let size = sizes[si];
                    let dur = if si < deltas.len() { deltas[si] } else { 0 };
                    let sync = match &sync_set {
                        Some(set) => set.contains(&sample_no) || set.is_empty(),
                        // ISO 14496-12 §8.6.2: absent stss ⇒ all sync.
                        None => true,
                    };
                    samples.push(Sample {
                        offset,
                        size,
                        dts,
                        cts: cts.get(si).copied().unwrap_or(0),
                        dur,
                        sync,
                    });
                    dts = dts.saturating_add(dur);
                    offset = offset.saturating_add(size);
                    si += 1;
                    sample_no += 1;
                }
            }
            if si < count {
                self.note((count - si) as u64, "stsc-stsz-mismatch-short");
            }
            track.samples = samples;
        }
        self.tracks.push(track);
        Ok(())
    }

    /// Read chapter titles from the chap-referenced text tracks'
    /// sample tables (QuickTime text: 16-bit BE length prefix).
    fn read_chapter_titles(&mut self) -> Result<(), EngineError> {
        // (dts, timescale, offset, size) for every chapter sample.
        let mut specs: Vec<(u64, u32, u64, u64)> = Vec::new();
        for t in &self.tracks {
            if t.is_chapter {
                for s in &t.samples {
                    specs.push((s.dts, t.timescale, s.offset, s.size));
                }
            }
        }
        let mut chapters: Vec<(u64, Chapter)> = Vec::new();
        for (dts, timescale, offset, size) in specs {
            if size == 0 || size > 4096 {
                self.note(size, "chapter-sample-over-ceiling");
                continue;
            }
            let bytes = self.reader.read_prefix_at(size as usize, offset)?;
            if bytes.len() < size as usize {
                self.note(size, "chapter-sample-unreadable");
                continue;
            }
            let Some(tlen) = u16be(&bytes, 0) else {
                self.note(size, "chapter-sample-unparseable");
                continue;
            };
            let text = bytes
                .get(2..2 + tlen as usize)
                .map(|b| String::from_utf8_lossy(b).to_string())
                .unwrap_or_default();
            let nanos = ticks_to_nanos(dts, timescale);
            chapters.push((
                nanos,
                Chapter {
                    start: Nanoseconds(nanos),
                    title: text,
                },
            ));
        }
        chapters.sort_unstable_by_key(|(nanos, _)| *nanos);
        if !chapters.is_empty() {
            self.chapters = chapters.into_iter().map(|(_, ch)| ch).collect();
        } else if !self.chapters.is_empty() {
            // chpl chapters — sorted by construction order anyway.
            self.chapters.sort_unstable_by_key(|ch| ch.start.0);
        }
        Ok(())
    }

    // ---- next_event: progressive + fragment paths ------------------

    fn step_progressive(&mut self) -> Result<DemuxEvent, EngineError> {
        let len = self.reader.length();
        while self.order_cursor < self.order.len() {
            let (_, ti, si) = self.order[self.order_cursor];
            self.order_cursor += 1;
            let track = &self.tracks[ti];
            if si < track.next {
                continue; // pre-seek sample — skipped by the cursor
            }
            let sample = track.samples[si];
            self.tracks[ti].next = si + 1;
            if sample.size == 0 {
                self.note(0, "zero-size-sample");
                continue;
            }
            if sample.size > MAX_SAMPLE_BYTES {
                self.note(sample.size, "sample-over-ceiling");
                continue;
            }
            if sample.offset >= len || sample.offset + sample.size > len {
                // Declared sample past the stream: deliver nothing
                // fake — typed truncation (the WAV precedent: never a
                // silent EOS with lost tail).
                self.surface_damage();
                return Err(self.fail(error::truncated(sample.offset + sample.size, len)));
            }
            let want = sample.size as usize;
            let at = sample.offset;
            let payload = self
                .pool
                .fill(want, |out| self.reader.read_prefix_into(out, at));
            let payload = match payload {
                Ok(p) => p,
                Err(err) => return Err(self.fail(err)),
            };
            if payload.len() != want {
                return Err(self.fail(error::truncated(sample.offset + sample.size, len)));
            }
            let packet = self.mint_packet(ti, &sample, payload, false);
            self.surface_damage();
            return Ok(DemuxEvent::Packet(packet));
        }
        self.eos = true;
        self.surface_damage();
        Ok(DemuxEvent::EndOfStream)
    }

    /// Parse one moof's trafs into per-track fragment samples.
    /// `moof` is the moof *payload* (children start immediately);
    /// `moof_start` is the moof box's file offset (the
    /// default-base-is-moof anchor). Returns samples with absolute
    /// offsets.
    fn parse_moof(
        &mut self,
        moof: &[u8],
        moof_start: u64,
    ) -> Result<Vec<(usize, Sample)>, EngineError> {
        let inner = iter_boxes(moof, 0, moof.len())?;
        let mut out: Vec<(usize, Sample)> = Vec::new();
        for traf in inner.iter().filter(|b| b.kind == *b"traf") {
            let tf = iter_boxes(moof, traf.body.start, traf.body.end)?;
            let mut track_id = 0u32;
            let mut base_offset = moof_start; // default-base-is-moof (DASH)
            let mut default_dur = 0u32;
            let mut default_size = 0u32;
            let mut default_flags = 0u32;
            let mut tfdt: Option<u64> = None;
            for b in &tf {
                match &b.kind {
                    b"tfhd" => {
                        let body = &moof[b.body.clone()];
                        let flags = (u32be(body, 0).unwrap_or(0)) & 0x00FF_FFFF;
                        track_id = u32be(body, 4).unwrap_or(0);
                        let mut at = 8usize;
                        if flags & 0x1 != 0 {
                            base_offset = u64be(body, at).unwrap_or(base_offset);
                            at += 8;
                        }
                        if flags & 0x2 != 0 {
                            at += 4; // sample-description-index (v0 gap)
                        }
                        if flags & 0x8 != 0 {
                            default_dur = u32be(body, at).unwrap_or(0);
                            at += 4;
                        }
                        if flags & 0x10 != 0 {
                            default_size = u32be(body, at).unwrap_or(0);
                            at += 4;
                        }
                        if flags & 0x20 != 0 {
                            default_flags = u32be(body, at).unwrap_or(0);
                        }
                        if flags & 0x020_000 == 0 && flags & 0x1 == 0 {
                            // Carry-forward base without
                            // default-base-is-moof: moof base still
                            // applied — damage-noted honesty.
                            self.note(0, "tfhd-base-carry-forward");
                        }
                    }
                    b"tfdt" => {
                        let body = &moof[b.body.clone()];
                        let version = body.first().copied().unwrap_or(0);
                        tfdt = if version == 1 {
                            u64be(body, 4)
                        } else {
                            u32be(body, 4).map(u64::from)
                        };
                    }
                    _ => {}
                }
            }
            let Some(ti) = self.tracks.iter().position(|t| t.track_id == track_id) else {
                self.note(0, "traf-unknown-track");
                continue;
            };
            let trex = self.tracks[ti].trex;
            let dur_default = if default_dur != 0 {
                default_dur
            } else {
                trex.map_or(0, |t| t.duration)
            };
            let size_default = if default_size != 0 {
                default_size
            } else {
                trex.map_or(0, |t| t.size)
            };
            let flags_default = if default_flags != 0 {
                default_flags
            } else {
                trex.map_or(0, |t| t.flags)
            };
            let mut dts = tfdt.unwrap_or(0);
            for b in tf.iter().filter(|x| x.kind == *b"trun") {
                let body = &moof[b.body.clone()];
                // trun is a FullBox: the flag bits live in the box
                // header's flags field; the body starts with
                // sample_count.
                let flags = (u32be(body, 0).unwrap_or(0)) & 0x00FF_FFFF;
                let count = u32be(body, 4).unwrap_or(0) as usize;
                let mut at = 8usize;
                let mut data_offset: i64 = 0;
                if flags & 0x1 != 0 {
                    data_offset = i64::from(u32be(body, at).unwrap_or(0)) as i32 as i64;
                    at += 4;
                }
                let mut first_flags: Option<u32> = None;
                if flags & 0x4 != 0 {
                    first_flags = Some(u32be(body, at).unwrap_or(0));
                    at += 4;
                }
                let mut sample_offset = base_offset as i64 + data_offset;
                for i in 0..count.min(MAX_SAMPLES) {
                    let mut dur = dur_default as u64;
                    let mut size = size_default as u64;
                    let mut sflags = flags_default;
                    if flags & 0x100 != 0 {
                        dur = u64::from(u32be(body, at).unwrap_or(0));
                        at += 4;
                    }
                    if flags & 0x200 != 0 {
                        size = u64::from(u32be(body, at).unwrap_or(0));
                        at += 4;
                    }
                    if flags & 0x400 != 0 {
                        sflags = u32be(body, at).unwrap_or(0);
                        at += 4;
                    }
                    if i == 0 {
                        sflags = first_flags.unwrap_or(sflags);
                    }
                    let mut cts = 0i64;
                    if flags & 0x800 != 0 {
                        cts = i64::from(u32be(body, at).unwrap_or(0)) as i32 as i64;
                        at += 4;
                    }
                    if size > 0 {
                        out.push((
                            ti,
                            Sample {
                                offset: sample_offset.unsigned_abs().min(u64::MAX / 2),
                                size,
                                dts,
                                cts,
                                dur,
                                sync: sflags & SAMPLE_IS_NON_SYNC == 0 && sflags & 0x1_0000 == 0,
                            },
                        ));
                    }
                    sample_offset = sample_offset.saturating_add(size as i64);
                    dts = dts.saturating_add(dur);
                }
                if count > MAX_SAMPLES {
                    self.note((count - MAX_SAMPLES) as u64, "trun-count-over-ceiling");
                }
            }
        }
        Ok(out)
    }

    fn step_fragment(&mut self) -> Result<DemuxEvent, EngineError> {
        let len = self.reader.length();
        let mut pos = self.frag_pos.unwrap_or(len);
        let mut budget = MAX_FRAGMENT_SCAN_BOXES;
        while pos < len && budget > 0 {
            budget -= 1;
            let Some(header) = read_top(&mut self.reader, pos)? else {
                break;
            };
            match &header.kind {
                b"moof" => {
                    if header.end > len {
                        return Err(self.fail(error::truncated(header.end, len)));
                    }
                    let moof_len = header.end - header.payload_start;
                    if moof_len > MAX_MOOF_BYTES {
                        self.note(moof_len, "moof-over-ceiling");
                        pos = header.end;
                        continue;
                    }
                    let mut moof = vec![0u8; moof_len as usize];
                    let got = self
                        .reader
                        .read_prefix_into(&mut moof, header.payload_start)?;
                    if got as u64 != moof_len {
                        return Err(self.fail(error::truncated(header.end, len)));
                    }
                    let mut frag_samples = self.parse_moof(&moof, pos)?;
                    // Emit in DTS order (stable by track ordinal).
                    frag_samples.sort_by(|a, b| a.1.dts.cmp(&b.1.dts).then(a.0.cmp(&b.0)));
                    // Queue everything; return the first.
                    let mut first = true;
                    let mut first_packet: Option<Packet> = None;
                    for (ti, sample) in frag_samples {
                        if sample.size == 0 || sample.size > MAX_SAMPLE_BYTES {
                            self.note(sample.size, "frag-sample-size");
                            continue;
                        }
                        if sample.offset >= len || sample.offset + sample.size > len {
                            self.note(sample.size, "frag-sample-past-stream");
                            continue;
                        }
                        let want = sample.size as usize;
                        let at = sample.offset;
                        let payload = self
                            .pool
                            .fill(want, |out| self.reader.read_prefix_into(out, at));
                        let payload = match payload {
                            Ok(p) => p,
                            Err(err) => return Err(self.fail(err)),
                        };
                        if payload.len() != want {
                            return Err(
                                self.fail(error::truncated(sample.offset + sample.size, len))
                            );
                        }
                        let packet = self.mint_packet(ti, &sample, payload, false);
                        if first {
                            first = false;
                            first_packet = Some(packet);
                        } else {
                            self.pending.push_back(DemuxEvent::Packet(packet));
                        }
                    }
                    if let Some(packet) = first_packet {
                        self.frag_pos = Some(header.end);
                        self.surface_damage();
                        return Ok(DemuxEvent::Packet(packet));
                    }
                    pos = header.end;
                }
                b"mdat" | b"free" | b"skip" | b"wide" | b"sidx" | b"uuid" | b"prft" | b"emsg" => {
                    pos = header.end;
                }
                _ => {
                    self.note(header.end - pos, "unknown-top-box-skipped");
                    pos = header.end;
                }
            }
        }
        if budget == 0 {
            return Err(self.fail(error::scan_budget_exhausted()));
        }
        self.eos = true;
        self.surface_damage();
        Ok(DemuxEvent::EndOfStream)
    }

    /// Mint one packet from a sample (the elst timeline application).
    fn mint_packet(&self, ti: usize, s: &Sample, data: SharedBytes, discontinuity: bool) -> Packet {
        let t = &self.tracks[ti];
        let dts_ticks = (s.dts as i64).saturating_sub(t.media_time as i64);
        let pts_ticks = dts_ticks
            .saturating_add(s.cts)
            .saturating_add(t.empty_edit_delay as i64);
        let duration = if s.dur > 0 {
            Timestamp::new(s.dur as i64, t.base)
                .as_nanos()
                .ok()
                .map(Duration::Known)
        } else {
            None
        };
        Packet {
            track: TrackId::new(ti as u32),
            pts: Stamp::Present(Timestamp::new(pts_ticks, t.base)),
            dts: Stamp::Present(Timestamp::new(dts_ticks, t.base)),
            duration,
            data,
            keyframe: s.sync,
            discontinuity,
            generation: GenerationId::FIRST,
        }
    }

    /// The reference track for seeks (first video, else first).
    fn seek_reference(&self) -> Option<usize> {
        self.tracks
            .iter()
            .position(|t| !t.is_chapter && t.kind == MediaKind::Video)
            .or_else(|| self.tracks.iter().position(|t| !t.is_chapter))
    }

    /// The stream extent in nanoseconds (max track end; the seek
    /// range check).
    fn extent_nanos(&self) -> Option<u64> {
        self.tracks
            .iter()
            .filter(|t| !t.is_chapter)
            .filter_map(|t| {
                let end = t
                    .empty_edit_delay
                    .saturating_sub(t.media_time)
                    .saturating_add(t.media_duration);
                (end > 0).then(|| ticks_to_nanos(end, t.timescale))
            })
            .max()
    }
}

/// tref/chap referenced track ids of one trak body.
fn trak_chap_refs(
    data: &[u8],
    range: std::ops::Range<usize>,
) -> Result<Option<Vec<u32>>, EngineError> {
    let boxes = iter_boxes(data, range.start, range.end)?;
    let Some(tref) = boxes.iter().find(|b| b.kind == *b"tref") else {
        return Ok(None);
    };
    let inner = iter_boxes(data, tref.body.start, tref.body.end)?;
    let Some(chap) = inner.iter().find(|b| b.kind == *b"chap") else {
        return Ok(None);
    };
    let body = &data[chap.body.clone()];
    let mut ids = Vec::with_capacity(body.len() / 4);
    let mut at = 0usize;
    while at + 4 <= body.len() && ids.len() < 64 {
        ids.push(u32be(body, at).unwrap_or(0));
        at += 4;
    }
    Ok(Some(ids))
}

/// Media ticks → nanoseconds (u128 intermediate, saturating).
fn ticks_to_nanos(ticks: u64, timescale: u32) -> u64 {
    if timescale == 0 {
        return 0;
    }
    let v = (ticks as u128 * 1_000_000_000) / u128::from(timescale);
    v.min(u64::MAX as u128) as u64
}

/// Nanoseconds → media ticks (floor).
fn nanos_to_ticks(nanos: u64, timescale: u32) -> u64 {
    (nanos as u128 * u128::from(timescale) / 1_000_000_000).min(u64::MAX as u128) as u64
}

impl Demuxer for Mp4Demuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Err(error::not_started());
        }
        match self.open() {
            Ok(tracks) => Ok(tracks),
            Err(err) => Err(self.fail(err)),
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        if let Some(sticky) = &self.sticky {
            return Err(sticky.mint());
        }
        if self.eos {
            return Ok(DemuxEvent::EndOfStream);
        }
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        if self.fragmented {
            self.step_fragment()
        } else {
            self.step_progressive()
        }
    }

    fn seekability(&self) -> SeekSupport {
        if !self.started {
            return SeekSupport::None;
        }
        if self.fragmented {
            SeekSupport::SlowLinear
        } else {
            SeekSupport::Fast { indexed: true }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        if let Some(sticky) = &self.sticky {
            return Err(sticky.mint());
        }
        let target_ts = target
            .start
            .timestamp()
            .map_err(|_| error::seek_out_of_range())?;
        if target_ts.ticks < 0 {
            return Err(error::seek_out_of_range());
        }
        let target_nanos = target_ts
            .as_nanos()
            .map_err(|_| error::seek_out_of_range())?
            .0;
        let Some(ref_idx) = self.seek_reference() else {
            return Err(error::seek_out_of_range());
        };
        if self
            .extent_nanos()
            .is_some_and(|e| e > 0 && target_nanos > e)
        {
            return Err(error::seek_out_of_range());
        }
        // ---- compute (D4: nothing moves until every landing is known)
        if self.fragmented {
            self.seek_fragment(ref_idx, target_nanos)
        } else {
            self.seek_progressive(ref_idx, target_nanos)
        }
    }
}

impl Mp4Demuxer {
    /// Progressive seek: reference-track keyframe walk-back, then
    /// every track's first sample ≥ the landing.
    fn seek_progressive(
        &mut self,
        ref_idx: usize,
        target_nanos: u64,
    ) -> Result<TimeSpan, EngineError> {
        let ref_track = &self.tracks[ref_idx];
        let target_ticks = nanos_to_ticks(target_nanos, ref_track.timescale) + ref_track.media_time;
        // Last sample with DTS ≤ target.
        let mut land = None;
        for (i, s) in ref_track.samples.iter().enumerate() {
            if s.dts + ref_track.media_time <= target_ticks {
                land = Some(i);
            } else {
                break;
            }
        }
        let Some(mut land_idx) = land else {
            return Err(error::seek_out_of_range());
        };
        // Walk back to the previous sync sample.
        while land_idx > 0 && !ref_track.samples[land_idx].sync {
            land_idx -= 1;
        }
        let land_nanos = ticks_to_nanos(
            self.tracks[ref_idx].samples[land_idx].dts + self.tracks[ref_idx].media_time,
            self.tracks[ref_idx].timescale,
        );
        // Commit: per-track cursors (compute-then-commit — every
        // landing above already succeeded).
        for (ti, t) in self.tracks.iter_mut().enumerate() {
            if ti == ref_idx {
                t.next = land_idx;
            } else {
                let land_ticks = nanos_to_ticks(land_nanos, t.timescale) + t.media_time;
                t.next = t
                    .samples
                    .iter()
                    .position(|s| s.dts + t.media_time >= land_ticks)
                    .unwrap_or(t.samples.len());
            }
        }
        self.pending.clear();
        self.eos = false;
        let base = self.tracks[ref_idx].base;
        let land_dts = self.tracks[ref_idx].samples[land_idx].dts + self.tracks[ref_idx].media_time;
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(land_dts as i64, base)),
            end: Stamp::Missing,
        })
    }

    /// Fragmented seek: bounded moof-scan to the moof whose reference
    /// track starts a sync sample ≤ target.
    fn seek_fragment(
        &mut self,
        ref_idx: usize,
        target_nanos: u64,
    ) -> Result<TimeSpan, EngineError> {
        let len = self.reader.length();
        let timescale = self.tracks[ref_idx].timescale;
        let target_ticks = nanos_to_ticks(target_nanos, timescale);
        let mut pos = self.frag_pos.unwrap_or(len);
        let mut best: Option<(u64, u64)> = None; // (moof offset, land dts)
        let mut budget = MAX_FRAGMENT_SCAN_BOXES;
        while pos < len && budget > 0 {
            budget -= 1;
            let Some(header) = read_top(&mut self.reader, pos)? else {
                break;
            };
            if header.kind == *b"moof" {
                if header.end > len {
                    break;
                }
                let moof_len = header.end - header.payload_start;
                if moof_len > MAX_MOOF_BYTES {
                    pos = header.end;
                    continue;
                }
                let mut moof = vec![0u8; moof_len as usize];
                let got = self
                    .reader
                    .read_prefix_into(&mut moof, header.payload_start)?;
                if got as u64 != moof_len {
                    break;
                }
                let frag_samples = self.parse_moof(&moof, pos)?;
                // The reference track's first sync sample in this moof.
                let land = frag_samples
                    .iter()
                    .filter(|(ti, _)| *ti == ref_idx)
                    .find(|(_, s)| s.sync && s.dts <= target_ticks)
                    .map(|(_, s)| s.dts);
                if let Some(land_dts) = land {
                    best = Some((pos, land_dts));
                } else if frag_samples
                    .iter()
                    .any(|(ti, s)| *ti == ref_idx && s.dts > target_ticks)
                {
                    // Passed the target without a sync landing.
                    break;
                }
                pos = header.end;
            } else {
                pos = header.end;
            }
        }
        if budget == 0 {
            return Err(error::scan_budget_exhausted());
        }
        let Some((moof_offset, land_dts)) = best else {
            return Err(error::seek_out_of_range());
        };
        // Commit: restart the fragment walk at the chosen moof.
        self.frag_pos = Some(moof_offset);
        self.pending.clear();
        self.eos = false;
        let base = self.tracks[ref_idx].base;
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(land_dts as i64, base)),
            end: Stamp::Missing,
        })
    }
}
