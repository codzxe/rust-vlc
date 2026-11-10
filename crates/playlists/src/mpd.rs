//! Typed DASH **MPD** manifest model (card 6.C1a) — the MPEG-DASH
//! parse core, the C-series head.
//!
//! * [`parse_mpd`]/[`parse_mpd_bytes`] — parse an MPD (ISO/IEC
//!   23009-1 XML manifest) into the typed model below: MPD attrs
//!   (`type` static/dynamic, `mediaPresentationDuration`,
//!   `minimumUpdatePeriod`, `profiles`, `availabilityStartTime`, …),
//!   [`Period`] → [`AdaptationSet`] → [`Representation`] with the
//!   media-fact attrs (mimeType/codecs/lang/segmentAlignment,
//!   bandwidth, width/height, audioSamplingRate), per-level
//!   `BaseURL`s, and the three segment-description **shapes**
//!   ([`SegmentTemplate`], [`SegmentList`], [`SegmentBase`]) parsed
//!   and modeled with their attributes **honestly** —
//!   `initialization`/`media` stay **unresolved template strings**
//!   (`$RepresentationID$`, `$Number$` …), SegmentTimeline `S`
//!   entries stay raw `{t, d, r}`, SegmentURL lists stay name lists.
//!   Resolution to concrete URLs/segment indices is card **6.C1b**;
//!   ABR policy is 6.C2.
//! * [`ContentProtection`] is **recorded** (schemeIdUri/value/
//!   default_KID), never interpreted — the typed "encrypted, not
//!   supported yet" refusal belongs to the future session layer,
//!   exactly like `#EXT-X-KEY` in the HLS model.
//! * Descriptor-class elements (`AudioChannelConfiguration`, `Role`,
//!   `Accessibility`, `EssentialProperty`, `SupplementalProperty`,
//!   `FramePacking`) are recorded as [`Descriptor`] pairs
//!   (schemeIdUri + value) for C1b/C2.
//!
//! # Home decision (recorded for the Lead)
//!
//! Doc 13 §9's 6.C1 row owns `crates/streaming/src/dash/mpd.rs` —
//! that is the **session** home for the whole of 6.C1 (and C2's
//! `sched.rs` sibling). This unit is the Lead-decomposed **parse
//! core** (6.C1a) and lands in `playlists` per the 6.B1a precedent:
//! the HLS m3u8 parse lives at `playlists/src/hls.rs` while the
//! streaming session (`streaming/src/hls/media.rs`) consumes it over
//! the registered production edge `streaming → playlists`
//! (boundaries.toml, wave-4 close). The DASH session cards (C1b/C2)
//! consume this module the same way. No new dependency: this crate
//! stays std + thiserror.
//!
//! # The XML reader (std-only, the zero-xml-deps discipline)
//!
//! Hand-rolled pull scanner, the `xspf` module's proven pattern
//! (card 5.O1b): comments, processing instructions, CDATA, naive
//! `<!…>` declaration skip, both quote styles, entity decoding (the
//! five named entities plus decimal/hex numeric references, decoded
//! **once** — never re-scanned, so nested `&amp;amp;…` shapes cannot
//! expand), namespace/prefix-tolerant **local-name** matching
//! (`<dash:MPD>` is an MPD). The scanner is deliberately a sibling
//! copy rather than a shared module: `xspf`'s scanner is coupled to
//! `XspfWarning` types; unifying them is a recorded follow-up for
//! the Lead (a `playlists::xml` core would touch every format
//! module — too wide for this unit).
//!
//! # Bounds (D-1) — size, depth, lines, entity bombs
//!
//! * Total input over [`MAX_MPD_BYTES`] (8 MiB, matching the fetch
//!   port's default response budget) → [`MpdError::TooLarge`].
//! * Any line over the crate's 64 KiB cap (`m3u::MAX_LINE_BYTES`)
//!   → [`MpdError::LineTooLong`].
//! * Nesting deeper than [`MAX_MPD_DEPTH`] (64 — real MPDs are
//!   ~7 deep) → [`MpdError::DepthExceeded`], element stack capped.
//! * **Entity-expansion bombs are structurally impossible**: custom
//!   entities are never defined or expanded (DOCTYPE is skipped,
//!   unknown references stay verbatim with a warning), and numeric
//!   references decode once, one char per reference. A
//!   billion-laughs-shaped input therefore costs at most its own
//!   byte size in memory — pinned by test.
//!
//! # Dirty-world tolerance (D-2) vs typed refusals
//!
//! The card AC is *schema-validity tolerance for broken servers*:
//! unknown elements and attributes are warnings, never failures;
//! a mismatched end tag salvages what closed cleanly. The line
//! between warn and refuse is drawn at **what a wrong number would
//! lie about**:
//!
//! * **Fatal** [`MpdError::BadValue`]: junk values on attributes
//!   that describe the *document's timeline or identity* — MPD
//!   `@type`, MPD/Period durations, SegmentTemplate/SegmentList
//!   `timescale`/`duration`/`startNumber`/`presentationTimeOffset`,
//!   timeline `S@t/d/r` (a wrong `S@d` mis-places every segment
//!   after it). The m3u8 precedent (promoted-tag `BadTagValue`).
//! * **Warn + drop**: junk or missing values on *one
//!   representation's* facts (bandwidth, width/height,
//!   audioSamplingRate, adaptation-set summaries) — the other
//!   representations stay usable, which is the tolerance the AC
//!   asks for; the field records `None` honestly.
//! * **Fatal** [`MpdError::Truncated`]: unlike XSPF (an entry list,
//!   where a salvaged track is still useful), a manifest cut short
//!   may be missing whole periods/representations — EOF with any
//!   element still open, or EOF inside a tag/comment/CDATA/PI, is a
//!   typed refusal, not a salvage.
//!
//! I/O-free like the rest of the crate: bytes arrive as arguments;
//! nothing is fetched (fetching is the streaming crate's C1b/C2
//! session).
//!
//! # Fixtures (provenance)
//!
//! * `tests/fixtures/dash/ffmpeg_av_template.mpd` — real ffmpeg
//!   7.1.5 `-f dash -use_template 1 -use_timeline 1` output (3
//!   representations: 2 video + 1 audio, SegmentTemplate +
//!   SegmentTimeline). Committed byte-identical; the exact command
//!   is recorded in the worklog and the fixture test's comments,
//!   together with the ffprobe differential numbers.
//! * `tests/fixtures/dash/ffmpeg_av_list.mpd` — real ffmpeg
//!   `-use_template 0` output (SegmentList + SegmentURL shape).
//! * `hand_static_cenc.mpd`, `hand_dynamic.mpd` — hand-authored
//!   minimal manifests per the ISO/IEC 23009-1 shapes (duration-
//!   based templates, ContentProtection, BaseURL levels, dynamic
//!   type with update period), for surface the ffmpeg muxer does
//!   not emit.
//!
//! Hostile inputs (truncations, deep nesting, entity-bomb shapes,
//! oversize) are generated in-test from those bytes.

use crate::m3u::MAX_LINE_BYTES;
use std::fmt;

/// Maximum accepted MPD nesting depth (D-1). Real manifests are
/// ~7 levels deep (MPD → Period → AdaptationSet → Representation →
/// SegmentTemplate → SegmentTimeline → S); 64 leaves generous head
/// room while capping hostile nesting. Same value as the XSPF
/// reader's cap.
pub const MAX_MPD_DEPTH: usize = 64;

/// Maximum accepted MPD input size (D-1). Matches the streaming
/// port's default `HttpRequest::response_limit` (8 MiB) so a
/// manifest fetched under the house budget always fits the parser's
/// own budget.
pub const MAX_MPD_BYTES: usize = 8 * 1024 * 1024;

// ----------------------------------------------------------------- model

/// A parsed MPD manifest (card 6.C1a): the typed tree, plus the
/// dirty-world warnings gathered while parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct MpdParsed {
    /// The manifest model.
    pub mpd: Mpd,
    /// Non-fatal findings (unknown elements/attrs, degraded
    /// representation facts, salvage notes) in document order.
    pub warnings: Vec<MpdWarning>,
}

/// The MPD root element's typed model.
#[derive(Debug, Clone, PartialEq)]
pub struct Mpd {
    /// `@type` — `static` (default per the spec) or `dynamic`.
    pub mpd_type: MpdType,
    /// `@profiles` — verbatim URN list (profile admission is the
    /// session's policy, not the parser's).
    pub profiles: Option<String>,
    /// `@mediaPresentationDuration` (seconds). `None` when absent
    /// (legal: dynamic manifests may omit it).
    pub media_presentation_duration: Option<f64>,
    /// `@minimumUpdatePeriod` (seconds) — the dynamic reload hint.
    pub minimum_update_period: Option<f64>,
    /// `@maxSegmentDuration` (seconds), when present.
    pub max_segment_duration: Option<f64>,
    /// `@minBufferTime` (seconds), when present.
    pub min_buffer_time: Option<f64>,
    /// `@timeShiftBufferDepth` (seconds; dynamic manifests).
    pub time_shift_buffer_depth: Option<f64>,
    /// `@availabilityStartTime` — **recorded verbatim** (ISO 8601
    /// date-time). Wall-clock parsing/UTC sync is C2's
    /// availability-time concern; the parser does not guess it.
    pub availability_start_time: Option<String>,
    /// `@publishTime` — recorded verbatim, same rule.
    pub publish_time: Option<String>,
    /// `@id`, when present.
    pub id: Option<String>,
    /// MPD-level `<BaseURL>` elements in order (verbatim text).
    pub base_urls: Vec<String>,
    /// `<Period>` elements in document order.
    pub periods: Vec<Period>,
}

/// `MPD@type` (ISO/IEC 23009-1 §5.3.1.1): `static` or `dynamic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpdType {
    /// `type="static"` — a fixed manifest (VOD); also the spec's
    /// default when the attribute is absent.
    Static,
    /// `type="dynamic"` — a live sliding-window manifest; reload
    /// cadence is `minimumUpdatePeriod` (the session layer, B1b's
    /// analogue, owns the reload loop).
    Dynamic,
}

/// One `<Period>`: a run of content made of adaptation sets.
#[derive(Debug, Clone, PartialEq)]
pub struct Period {
    /// `@id`, when present.
    pub id: Option<String>,
    /// `@start` (seconds, `xs:duration` relative to the MPD's
    /// start), when present.
    pub start: Option<f64>,
    /// `@duration` (seconds), when present.
    pub duration: Option<f64>,
    /// `@bitstreamSwitching`, when present.
    pub bitstream_switching: Option<bool>,
    /// Period-level `<BaseURL>` elements (verbatim).
    pub base_urls: Vec<String>,
    /// Period-level `<SegmentTemplate>` (inherited by contained
    /// sets/representations; inheritance resolution is C1b).
    pub segment_template: Option<SegmentTemplate>,
    /// `<AdaptationSet>` elements in document order.
    pub adaptation_sets: Vec<AdaptationSet>,
}

/// One `<AdaptationSet>`: alternative encodings of the same media
/// component — the ABR ladder (6.C2's input).
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptationSet {
    /// `@id`, when present.
    pub id: Option<String>,
    /// `@contentType` when present, else `@mimeType` — the
    /// component kind hint ("video"/"audio"/"text"/…), verbatim.
    pub content_type: Option<String>,
    /// `@mimeType`, when present (the set-level override).
    pub mime_type: Option<String>,
    /// `@codecs` — the comma-separated codec list, verbatim.
    pub codecs: Option<String>,
    /// `@xml:lang` (matched by local name, namespace-tolerant).
    pub lang: Option<String>,
    /// `@segmentAlignment` ("true"/"false"/"1"/"0").
    pub segment_alignment: Option<bool>,
    /// `@bitstreamSwitching`, when present.
    pub bitstream_switching: Option<bool>,
    /// `@startWithSAP` (enumerated 0/1/2), when present.
    pub start_with_sap: Option<u64>,
    /// `@maxWidth`, when present.
    pub max_width: Option<u64>,
    /// `@maxHeight`, when present.
    pub max_height: Option<u64>,
    /// `@par` (pixel aspect ratio, e.g. "4:3"), verbatim.
    pub par: Option<String>,
    /// `@frameRate` (e.g. "10/1"), verbatim — rational parsing is
    /// C1b's (the container's fps is the truth).
    pub frame_rate: Option<String>,
    /// Set-level `<BaseURL>` elements (verbatim).
    pub base_urls: Vec<String>,
    /// Set-level `<SegmentTemplate>` (inherited by representations).
    pub segment_template: Option<SegmentTemplate>,
    /// Set-level `<SegmentList>`, when present.
    pub segment_list: Option<SegmentList>,
    /// `<ContentProtection>` records on this set.
    pub content_protection: Vec<ContentProtection>,
    /// Descriptor-class children (`AudioChannelConfiguration`, …).
    pub descriptors: Vec<Descriptor>,
    /// `<Representation>` elements in document order.
    pub representations: Vec<Representation>,
}

/// One `<Representation>`: one rung of the ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct Representation {
    /// `@id` (a string per the spec, even where numeric in
    /// practice), when present.
    pub id: Option<String>,
    /// `@bandwidth` (bits/s). REQUIRED by the spec; absence or a
    /// junk value is recorded as `None` plus a warning (the other
    /// rungs stay usable — the broken-server tolerance).
    pub bandwidth: Option<u64>,
    /// `@width`, when present and numeric.
    pub width: Option<u64>,
    /// `@height`, when present and numeric.
    pub height: Option<u64>,
    /// `@frameRate`, verbatim.
    pub frame_rate: Option<String>,
    /// `@sar` (sample aspect ratio), verbatim.
    pub sar: Option<String>,
    /// `@mimeType`, when present.
    pub mime_type: Option<String>,
    /// `@codecs`, when present.
    pub codecs: Option<String>,
    /// `@audioSamplingRate` (Hz), when present and numeric.
    pub audio_sampling_rate: Option<u64>,
    /// Representation-level `<BaseURL>` elements (verbatim).
    pub base_urls: Vec<String>,
    /// Representation-level `<SegmentTemplate>`, when present.
    pub segment_template: Option<SegmentTemplate>,
    /// Representation-level `<SegmentList>`, when present.
    pub segment_list: Option<SegmentList>,
    /// Representation-level `<SegmentBase>` (single-URL + index
    /// range shape), when present.
    pub segment_base: Option<SegmentBase>,
    /// `<ContentProtection>` records on this representation.
    pub content_protection: Vec<ContentProtection>,
    /// Descriptor-class children.
    pub descriptors: Vec<Descriptor>,
}

/// `<SegmentTemplate>` — the addressing shape (attributes parsed,
/// templates **unresolved**; resolution is card 6.C1b).
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentTemplate {
    /// `@timescale` (ticks per second; the spec default is 1).
    pub timescale: Option<u64>,
    /// `@duration` (ticks) — the fixed-segment form; mutually
    /// exclusive with a SegmentTimeline in practice.
    pub duration: Option<u64>,
    /// `@startNumber` (the first segment's number, default 1).
    pub start_number: Option<u64>,
    /// `@presentationTimeOffset` (ticks).
    pub presentation_time_offset: Option<u64>,
    /// `@initialization` — the init-segment **template string,
    /// verbatim** (`$RepresentationID$` et al. stay unresolved).
    pub initialization: Option<String>,
    /// `@media` — the media-segment **template string, verbatim**.
    pub media: Option<String>,
    /// `<SegmentTimeline>` child, when present.
    pub segment_timeline: Option<SegmentTimeline>,
}

/// `<SegmentTimeline>` — the explicit segment-duration list.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentTimeline {
    /// `<S>` entries in document order.
    pub entries: Vec<SegmentTimelineEntry>,
}

/// One `<S>` timeline entry: `d` is required; `t` (start tick)
/// defaults to the previous entry's end; `r` (repeat) may be
/// negative (spec §5.3.9.6) — all raw, C1b resolves.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentTimelineEntry {
    /// `@t` (start tick), when present.
    pub t: Option<u64>,
    /// `@d` (duration in ticks) — required by the spec; a missing
    /// `d` is a fatal [`MpdError::BadValue`].
    pub d: u64,
    /// `@r` (repeat count, may be negative), when present.
    pub r: Option<i64>,
}

/// `<SegmentList>` — the explicit URL-list shape.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentList {
    /// `@timescale` (ticks per second).
    pub timescale: Option<u64>,
    /// `@duration` (ticks), when present.
    pub duration: Option<u64>,
    /// `@startNumber`, when present.
    pub start_number: Option<u64>,
    /// `@presentationTimeOffset` (ticks), when present.
    pub presentation_time_offset: Option<u64>,
    /// `<Initialization sourceURL="…">`, verbatim.
    pub initialization: Option<String>,
    /// `<SegmentURL>` entries in document order.
    pub segment_urls: Vec<SegmentUrl>,
}

/// One `<SegmentURL>`: `media` (verbatim URL) and `mediaRange`
/// (verbatim byte-range string — range parsing is C1b's).
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentUrl {
    /// `@media`, when present.
    pub media: Option<String>,
    /// `@mediaRange`, verbatim.
    pub media_range: Option<String>,
}

/// `<SegmentBase>` — the single-URL + index shape (parsed only;
/// index interpretation is C1b).
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentBase {
    /// `@timescale` (ticks per second).
    pub timescale: Option<u64>,
    /// `@presentationTimeOffset` (ticks).
    pub presentation_time_offset: Option<u64>,
    /// `@indexRange` — verbatim byte-range string.
    pub index_range: Option<String>,
    /// `@indexRangeExact`, when present.
    pub index_range_exact: Option<bool>,
    /// `<Initialization range="…">`, verbatim.
    pub initialization_range: Option<String>,
}

/// `<ContentProtection>` — **recorded, never interpreted**: the
/// presence of DRM is a fact of the manifest; the typed
/// "encrypted → not supported yet" refusal belongs to the session
/// layer when it actually tries to play (the `#EXT-X-KEY` split).
#[derive(Debug, Clone, PartialEq)]
pub struct ContentProtection {
    /// `@schemeIdUri`, when present.
    pub scheme_id_uri: Option<String>,
    /// `@value`, when present.
    pub value: Option<String>,
    /// `@cenc:default_KID` (matched by local name), when present.
    pub default_kid: Option<String>,
}

/// A descriptor-class element (`AudioChannelConfiguration`, `Role`,
/// `Accessibility`, `EssentialProperty`, `SupplementalProperty`,
/// `FramePacking`): the scheme/value pair recorded for C1b/C2.
#[derive(Debug, Clone, PartialEq)]
pub struct Descriptor {
    /// The element's local name (which descriptor class this is).
    pub element: String,
    /// `@schemeIdUri`, when present.
    pub scheme_id_uri: Option<String>,
    /// `@value`, when present.
    pub value: Option<String>,
}

// ---------------------------------------------------------------- errors

/// Typed refusal of the MPD parse (card 6.C1a). House style
/// (matching [`HlsPlaylistError`](crate::HlsPlaylistError)): a
/// thiserror-free enum with stable dotted codes, structured
/// `Display`, no URIs/paths/line text (MP §12). The four codes the
/// card names — `not-mpd`, `bad-value`, `truncated`,
/// `depth-exceeded` — plus the crate's standard bounded-input
/// refusals (`line-too-long`, `too-large`, `invalid-utf8`).
#[derive(Debug)]
pub enum MpdError {
    /// The root element is not `MPD` (local-name matched), or the
    /// input contains no element at all (`playlists.mpd.not-mpd`).
    NotMpd,
    /// An attribute that describes the document's timeline or
    /// identity carried a value of the wrong type
    /// (`playlists.mpd.bad-value`) — a wrong number here would be a
    /// lie, so the parse refuses rather than guessing.
    BadValue {
        /// The owning element + attribute, e.g. `"MPD@mediaPresentationDuration"`.
        attr: &'static str,
    },
    /// The document ended with an element still open, or EOF
    /// arrived inside a tag/comment/CDATA/PI (`playlists.mpd.
    /// truncated`) — a manifest cut short may be missing whole
    /// periods, so this is a refusal, not a salvage (unlike XSPF's
    /// entry-list salvage).
    Truncated {
        /// 1-based line where EOF was reached.
        line: usize,
    },
    /// Element nesting exceeded [`MAX_MPD_DEPTH`]
    /// (`playlists.mpd.depth-exceeded`) — bounded input.
    DepthExceeded {
        /// The depth at which the parse refused.
        depth: usize,
    },
    /// A single line exceeds the crate's 64 KiB cap
    /// (`playlists.mpd.line-too-long`) — bounded input (D-1).
    LineTooLong {
        /// 1-based line number.
        line: usize,
        /// The line's byte length (terminator excluded).
        length: usize,
    },
    /// The input exceeds [`MAX_MPD_BYTES`] (`playlists.mpd.too-
    /// large`) — bounded input (D-1).
    TooLarge {
        /// The input's byte length.
        size: usize,
        /// The cap ([`MAX_MPD_BYTES`]).
        limit: usize,
    },
    /// The byte input is not valid UTF-8 (`playlists.mpd.
    /// invalid-utf8`) — MPD is a text format; no encoding guessed.
    InvalidUtf8,
}

impl MpdError {
    /// The stable dotted context code for this refusal.
    pub fn code(&self) -> &'static str {
        match self {
            MpdError::NotMpd => "playlists.mpd.not-mpd",
            MpdError::BadValue { .. } => "playlists.mpd.bad-value",
            MpdError::Truncated { .. } => "playlists.mpd.truncated",
            MpdError::DepthExceeded { .. } => "playlists.mpd.depth-exceeded",
            MpdError::LineTooLong { .. } => "playlists.mpd.line-too-long",
            MpdError::TooLarge { .. } => "playlists.mpd.too-large",
            MpdError::InvalidUtf8 => "playlists.mpd.invalid-utf8",
        }
    }
}

impl fmt::Display for MpdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MpdError::NotMpd => write!(
                f,
                "playlists: not an MPD manifest ({}): no MPD root element",
                self.code()
            ),
            MpdError::BadValue { attr } => write!(
                f,
                "playlists: MPD attribute {} needs a valid typed value ({}); refusing to guess",
                attr,
                self.code()
            ),
            MpdError::Truncated { line } => write!(
                f,
                "playlists: MPD truncated — end of input with markup still open at line {} ({})",
                line,
                self.code()
            ),
            MpdError::DepthExceeded { depth } => write!(
                f,
                "playlists: MPD nesting depth {} over the {} cap ({})",
                depth,
                MAX_MPD_DEPTH,
                self.code()
            ),
            MpdError::LineTooLong { line, length } => write!(
                f,
                "playlists: line {} is {} bytes, over the {}-byte line cap ({})",
                line,
                length,
                MAX_LINE_BYTES,
                self.code()
            ),
            MpdError::TooLarge { size, limit } => write!(
                f,
                "playlists: MPD is {} bytes, over the {}-byte budget ({})",
                size,
                limit,
                self.code()
            ),
            MpdError::InvalidUtf8 => write!(
                f,
                "playlists: input is not valid UTF-8 (MPD is a text format; encoding is the caller's policy) ({})",
                self.code()
            ),
        }
    }
}

impl std::error::Error for MpdError {}

// -------------------------------------------------------------- warnings

/// Non-fatal dirty-world findings (D-2), in document order.
/// Diagnostic names (element/attribute local names) are structural,
/// never content: no line text, URIs, paths, or attribute values
/// are embedded (MP §12).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MpdWarning {
    /// An element outside the modeled subset appeared (tolerated
    /// and skipped, with its subtree). Real manifests carry plenty
    /// (`ProgramInformation`, `ServiceDescription`, `UTCTiming`, …)
    /// — the warning is the honest record that the model does not
    /// carry that information.
    #[error("playlists: line {line}: unmodeled <{name}> element (skipped with its subtree)")]
    UnknownElement {
        /// 1-based line of the start tag.
        line: usize,
        /// The element's local name (structural, not content).
        name: String,
    },
    /// A known element carried an attribute outside the modeled
    /// subset (tolerated, value dropped).
    #[error("playlists: line {line}: unmodeled {name} attribute (value dropped)")]
    UnknownAttribute {
        /// 1-based line of the owning start tag.
        line: usize,
        /// The attribute's local name.
        name: String,
    },
    /// A modeled element appeared at a position the model does not
    /// attach (e.g. `SegmentTemplate` directly under `MPD`).
    #[error("playlists: line {line}: <{name}> not supported at this position (skipped)")]
    UnsupportedHere {
        /// 1-based line of the start tag.
        line: usize,
        /// The element's local name.
        name: String,
    },
    /// A `<Representation>` had no `@bandwidth` (required by the
    /// spec); the rung survives with `bandwidth: None` so its
    /// siblings stay usable.
    #[error("playlists: line {line}: Representation without @bandwidth (recorded as absent)")]
    MissingBandwidth {
        /// 1-based line of the start tag.
        line: usize,
    },
    /// A representation/adaptation-set fact attribute carried a
    /// junk value (bandwidth, width, height, …) — degraded to
    /// `None` for that rung only (the broken-server tolerance; the
    /// timeline-level equivalents are fatal [`MpdError::BadValue`]).
    #[error("playlists: line {line}: malformed {attr} value (field recorded as absent)")]
    BadAttrValue {
        /// 1-based line of the start tag.
        line: usize,
        /// The attribute's name.
        attr: &'static str,
    },
    /// An entity reference was unknown, malformed, or out of range;
    /// it is kept **verbatim** (no invented characters).
    #[error("playlists: line {line}: unknown or malformed entity (kept verbatim)")]
    BadEntity {
        /// 1-based line of the reference.
        line: usize,
    },
    /// An end tag with no matching open element (noted, ignored).
    #[error("playlists: line {line}: end tag with no matching open element (ignored)")]
    MismatchedEndTag {
        /// 1-based line of the end tag.
        line: usize,
    },
    /// Character data appeared where the model carries none
    /// (directly inside a structural element, or outside the root).
    #[error("playlists: line {line}: stray character data (dropped)")]
    StrayText {
        /// 1-based line of the text run.
        line: usize,
    },
    /// An element appeared outside the root element (before or
    /// after it).
    #[error("playlists: line {line}: <{name}> outside the MPD root (skipped)")]
    StrayElement {
        /// 1-based line of the start tag.
        line: usize,
        /// The element's local name.
        name: String,
    },
    /// `<` followed by a non-name character: kept as literal text.
    #[error("playlists: line {line}: malformed tag markup (kept as text / resynchronized)")]
    BadTag {
        /// 1-based line of the stray `<`.
        line: usize,
    },
    /// A second segment description of the same kind on one element
    /// (first one wins).
    #[error("playlists: line {line}: duplicate segment description (first one wins)")]
    DuplicateSegmentDescription {
        /// 1-based line of the duplicate start tag.
        line: usize,
    },
    /// The manifest parsed with zero `<Period>` elements (legal
    /// XML, unplayable manifest — the session refuses; the model
    /// records).
    #[error("playlists: MPD has no Period elements (model recorded; session will refuse)")]
    NoPeriods {
        /// 1-based line of the MPD start tag.
        line: usize,
    },
}

// ----------------------------------------------------------------- parse

/// Parse `text` as an MPD manifest. See the [module docs](self)
/// for the tolerance/refusal policy and bounds.
///
/// # Errors
/// [`MpdError`] — typed refusals only; everything dirty-but-tolerable
/// degrades to [`MpdWarning`]s in the returned [`MpdParsed`].
pub fn parse_mpd(text: &str) -> Result<MpdParsed, MpdError> {
    if text.len() > MAX_MPD_BYTES {
        return Err(MpdError::TooLarge {
            size: text.len(),
            limit: MAX_MPD_BYTES,
        });
    }
    if let Some((line, length)) = line_over_cap(text) {
        return Err(MpdError::LineTooLong { line, length });
    }
    let mut scanner = Scanner::new(text);
    let mut driver = Driver::new();
    while let Some(token) = scanner.next_token() {
        driver.feed(token)?;
    }
    if let Some(line) = scanner.truncated {
        return Err(MpdError::Truncated { line });
    }
    driver.finish()?;
    // Scanner-level findings (bad entities, tag markup damage)
    // join the driver's — two passes, one list (order: driver's
    // structural findings first, scanner's lexical ones after).
    driver.warnings.extend(scanner.warnings);
    Ok(MpdParsed {
        mpd: driver.mpd,
        warnings: driver.warnings,
    })
}

/// Parse raw bytes as an MPD manifest (non-UTF-8 is a typed error).
///
/// # Errors
/// [`MpdError`] — same set as [`parse_mpd`] plus `InvalidUtf8`.
pub fn parse_mpd_bytes(bytes: &[u8]) -> Result<MpdParsed, MpdError> {
    let text = std::str::from_utf8(bytes).map_err(|_| MpdError::InvalidUtf8)?;
    parse_mpd(text)
}

// ------------------------------------------------------------- scanners

/// One lexical event. Names are prefix-stripped local names; text
/// runs are entity-decoded (CDATA passes through raw).
enum Token {
    Start {
        line: usize,
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    End {
        line: usize,
        name: String,
    },
    Text {
        line: usize,
        content: String,
    },
}

/// Byte-level pull scanner (the `xspf` module's pattern, sibling
/// copy — see the module docs for the recorded reason). Tracks the
/// 1-based line for warnings; never panics; marks truncation
/// instead of salvaging when EOF arrives inside a construct (the
/// MPD posture: a cut manifest is refused, not salvaged).
struct Scanner<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    truncated: Option<usize>,
    warnings: Vec<MpdWarning>,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
            truncated: None,
            warnings: Vec::new(),
        }
    }

    /// Consumes `src[pos..end]`, counting newlines for line
    /// tracking. Callers pass char-boundary ends only.
    fn take(&mut self, end: usize) -> &'a str {
        let start = self.pos;
        let slice = &self.src[start..end];
        self.line += slice.bytes().filter(|&b| b == b'\n').count();
        self.pos = end;
        slice
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn advance_char(&mut self) {
        let step = self.peek_char().map_or(0, char::len_utf8);
        self.take(self.pos + step);
    }

    fn skip_ws(&mut self) {
        while self.peek_char().is_some_and(char::is_whitespace) {
            self.advance_char();
        }
    }

    fn rest_starts_with(&self, prefix: &str) -> bool {
        self.src[self.pos..].starts_with(prefix)
    }

    /// EOF inside an unterminated construct: mark truncation (fatal
    /// for MPD) and consume the rest.
    fn truncated_at_eof(&mut self) {
        if self.truncated.is_none() {
            self.truncated = Some(self.line);
        }
        self.take(self.src.len());
    }

    fn skip_after(&mut self, marker: &str) {
        match self.src[self.pos..].find(marker) {
            Some(i) => {
                self.take(self.pos + i + marker.len());
            }
            None => self.truncated_at_eof(),
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        loop {
            if self.pos >= self.src.len() {
                return None;
            }
            if !self.rest_starts_with("<") {
                // Character data run up to the next '<'.
                let end = self.src[self.pos..]
                    .find('<')
                    .map_or(self.src.len(), |i| self.pos + i);
                let line = self.line;
                let raw = self.take(end);
                return Some(Token::Text {
                    line,
                    content: decode_entities(raw, &mut self.warnings, line),
                });
            }
            if self.rest_starts_with("<!--") {
                self.skip_after("-->");
                continue;
            }
            if self.rest_starts_with("<![CDATA[") {
                let line = self.line;
                self.take(self.pos + "<![CDATA[".len()); // the opener
                return match self.src[self.pos..].find("]]>") {
                    Some(i) => {
                        let end = self.pos + i;
                        let raw = self.take(end);
                        self.take(end + 3); // the "]]>" terminator
                        Some(Token::Text {
                            line,
                            content: raw.to_string(),
                        })
                    }
                    None => {
                        self.truncated_at_eof();
                        None
                    }
                };
            }
            if self.rest_starts_with("<?") {
                self.skip_after("?>");
                continue;
            }
            if self.rest_starts_with("<!") {
                // DOCTYPE and friends: naive skip to the next '>'
                // (custom entities inside are never registered —
                // the entity-bomb bound, see module docs).
                self.skip_after(">");
                continue;
            }
            if self.rest_starts_with("</") {
                return Some(self.scan_end_tag());
            }
            match self.src[self.pos + 1..].chars().next() {
                Some(c) if is_name_start(c) => return Some(self.scan_start_tag()),
                _ => {
                    // '<' with no name: keep it as literal text.
                    let line = self.line;
                    self.warnings.push(MpdWarning::BadTag { line });
                    let raw = self.take(self.pos + 1);
                    return Some(Token::Text {
                        line,
                        content: raw.to_string(),
                    });
                }
            }
        }
    }

    fn scan_end_tag(&mut self) -> Token {
        let start_line = self.line;
        self.take(self.pos + 2); // "</"
        self.skip_ws();
        let name_start = self.pos;
        while self.peek_char().is_some_and(is_name_char) {
            self.advance_char();
        }
        let raw_name = self.src[name_start..self.pos].to_string();
        self.skip_ws();
        if self.rest_starts_with(">") {
            self.take(self.pos + 1);
        } else {
            match self.src[self.pos..].find('>') {
                Some(i) => {
                    self.take(self.pos + i + 1);
                    self.warnings
                        .push(MpdWarning::MismatchedEndTag { line: start_line });
                }
                None => self.truncated_at_eof(),
            }
        }
        Token::End {
            line: start_line,
            name: local_name(&raw_name),
        }
    }

    fn scan_start_tag(&mut self) -> Token {
        let start_line = self.line;
        self.take(self.pos + 1); // "<"
        let name_start = self.pos;
        while self.peek_char().is_some_and(is_name_char) {
            self.advance_char();
        }
        let raw_name = self.src[name_start..self.pos].to_string();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        let mut terminated = false;
        loop {
            self.skip_ws();
            if self.pos >= self.src.len() {
                break;
            }
            if self.rest_starts_with("/>") {
                self.take(self.pos + 2);
                self_closing = true;
                terminated = true;
                break;
            }
            if self.rest_starts_with(">") {
                self.take(self.pos + 1);
                terminated = true;
                break;
            }
            // One attribute: name '=' quoted-value, damage resync.
            let attr_start = self.pos;
            while self.peek_char().is_some_and(is_name_char) {
                self.advance_char();
            }
            let attr_raw = self.src[attr_start..self.pos].to_string();
            self.skip_ws();
            if !self.rest_starts_with("=") {
                self.warnings.push(MpdWarning::BadTag { line: self.line });
                self.resync_attr();
                continue;
            }
            self.take(self.pos + 1); // "="
            self.skip_ws();
            let Some(quote) = self.peek_char().filter(|&c| c == '"' || c == '\'') else {
                self.warnings.push(MpdWarning::BadTag { line: self.line });
                self.resync_attr();
                continue;
            };
            let q = quote.to_string();
            self.advance_char();
            let v_start = self.pos;
            let v_end = self.src[v_start..]
                .find(&q)
                .map_or(self.src.len(), |i| v_start + i);
            let raw_value = self.take(v_end);
            let decoded = decode_entities(raw_value, &mut self.warnings, self.line);
            if self.rest_starts_with(&q) {
                self.take(self.pos + q.len()); // closing quote (absent at EOF)
            }
            if attr_raw.is_empty() {
                self.warnings.push(MpdWarning::BadTag { line: self.line });
            } else if !is_namespace_decl(&attr_raw) {
                attrs.push((local_name(&attr_raw), decoded));
            }
        }
        if !terminated {
            self.truncated_at_eof();
        }
        Token::Start {
            line: start_line,
            name: local_name(&raw_name),
            attrs,
            self_closing,
        }
    }

    /// Attribute-damage resync: skip to the next whitespace or `>`
    /// (a `/` is consumed — only `/` immediately before `>` is a
    /// terminator; stopping at a lone `/` would spin).
    fn resync_attr(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '>' {
                break;
            }
            self.advance_char();
        }
    }
}

/// `xmlns`, `xmlns:*` declarations are infrastructure, not content:
/// recorded nowhere, warned about never (documented rule).
fn is_namespace_decl(raw: &str) -> bool {
    raw == "xmlns" || raw.starts_with("xmlns:")
}

// ----------------------------------------------------------- tree build

/// The parse driver's element stack. Each frame remembers its local
/// name (end-tag matching + salvage) and its builder state.
enum Frame {
    Root,
    Period(Period),
    AdaptationSet(AdaptationSet),
    Representation(Representation),
    SegmentTemplate(SegmentTemplate),
    SegmentList(SegmentList),
    SegmentBase(SegmentBase),
    SegmentTimeline(SegmentTimeline),
    BaseUrl(String),
    Unknown,
}

struct Driver {
    mpd: Mpd,
    stack: Vec<(String, usize, Frame)>,
    warnings: Vec<MpdWarning>,
    root_seen: bool,
    root_line: usize,
}

impl Driver {
    fn new() -> Self {
        Self {
            mpd: Mpd {
                mpd_type: MpdType::Static,
                profiles: None,
                media_presentation_duration: None,
                minimum_update_period: None,
                max_segment_duration: None,
                min_buffer_time: None,
                time_shift_buffer_depth: None,
                availability_start_time: None,
                publish_time: None,
                id: None,
                base_urls: Vec::new(),
                periods: Vec::new(),
            },
            stack: Vec::new(),
            warnings: Vec::new(),
            root_seen: false,
            root_line: 1,
        }
    }

    /// Depth guard applies to every push, known or unknown frame.
    fn push(&mut self, name: String, line: usize, frame: Frame) -> Result<(), MpdError> {
        if self.stack.len() >= MAX_MPD_DEPTH {
            return Err(MpdError::DepthExceeded {
                depth: self.stack.len() + 1,
            });
        }
        self.stack.push((name, line, frame));
        Ok(())
    }

    fn feed(&mut self, token: Token) -> Result<(), MpdError> {
        match token {
            Token::Start {
                line,
                name,
                attrs,
                self_closing,
            } => self.on_start(line, name, attrs, self_closing),
            Token::End { line, name } => self.on_end(line, name),
            Token::Text { line, content } => self.on_text(line, content),
        }
    }

    fn on_start(
        &mut self,
        line: usize,
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    ) -> Result<(), MpdError> {
        if !self.root_seen {
            if name != "MPD" {
                return Err(MpdError::NotMpd);
            }
            self.apply_mpd_attrs(line, &attrs)?;
            self.root_seen = true;
            self.root_line = line;
            return if self_closing {
                Ok(())
            } else {
                self.push(name, line, Frame::Root)
            };
        }
        if self.stack.is_empty() {
            // After the root closed: structural garbage, skipped.
            self.warnings.push(MpdWarning::StrayElement {
                line,
                name: name.clone(),
            });
            return if self_closing {
                Ok(())
            } else {
                self.push(name, line, Frame::Unknown)
            };
        }
        match name.as_str() {
            "Period" => {
                let mut period = Period {
                    id: None,
                    start: None,
                    duration: None,
                    bitstream_switching: None,
                    base_urls: Vec::new(),
                    segment_template: None,
                    adaptation_sets: Vec::new(),
                };
                apply_period_attrs(line, &attrs, &mut period, &mut self.warnings)?;
                self.push_or_close(name, line, Frame::Period(period), self_closing)
            }
            "AdaptationSet" => {
                let mut set = AdaptationSet {
                    id: None,
                    content_type: None,
                    mime_type: None,
                    codecs: None,
                    lang: None,
                    segment_alignment: None,
                    bitstream_switching: None,
                    start_with_sap: None,
                    max_width: None,
                    max_height: None,
                    par: None,
                    frame_rate: None,
                    base_urls: Vec::new(),
                    segment_template: None,
                    segment_list: None,
                    content_protection: Vec::new(),
                    descriptors: Vec::new(),
                    representations: Vec::new(),
                };
                apply_adaptation_set_attrs(line, &attrs, &mut set, &mut self.warnings);
                self.push_or_close(name, line, Frame::AdaptationSet(set), self_closing)
            }
            "Representation" => {
                let mut rep = Representation {
                    id: None,
                    bandwidth: None,
                    width: None,
                    height: None,
                    frame_rate: None,
                    sar: None,
                    mime_type: None,
                    codecs: None,
                    audio_sampling_rate: None,
                    base_urls: Vec::new(),
                    segment_template: None,
                    segment_list: None,
                    segment_base: None,
                    content_protection: Vec::new(),
                    descriptors: Vec::new(),
                };
                apply_representation_attrs(line, &attrs, &mut rep, &mut self.warnings);
                self.push_or_close(name, line, Frame::Representation(rep), self_closing)
            }
            "SegmentTemplate" => {
                let mut template = SegmentTemplate {
                    timescale: None,
                    duration: None,
                    start_number: None,
                    presentation_time_offset: None,
                    initialization: None,
                    media: None,
                    segment_timeline: None,
                };
                apply_segment_template_attrs(line, &attrs, &mut template, &mut self.warnings)?;
                self.push_or_close(name, line, Frame::SegmentTemplate(template), self_closing)
            }
            "SegmentList" => {
                let mut list = SegmentList {
                    timescale: None,
                    duration: None,
                    start_number: None,
                    presentation_time_offset: None,
                    initialization: None,
                    segment_urls: Vec::new(),
                };
                apply_segment_list_attrs(line, &attrs, &mut list, &mut self.warnings)?;
                self.push_or_close(name, line, Frame::SegmentList(list), self_closing)
            }
            "SegmentBase" => {
                let mut base = SegmentBase {
                    timescale: None,
                    presentation_time_offset: None,
                    index_range: None,
                    index_range_exact: None,
                    initialization_range: None,
                };
                apply_segment_base_attrs(line, &attrs, &mut base, &mut self.warnings);
                self.push_or_close(name, line, Frame::SegmentBase(base), self_closing)
            }
            "SegmentTimeline" => {
                self.note_unknown_attrs(line, &attrs, &[]);
                self.push_or_close(
                    name,
                    line,
                    Frame::SegmentTimeline(SegmentTimeline {
                        entries: Vec::new(),
                    }),
                    self_closing,
                )
            }
            "S" => {
                // Timeline entry: only inside a SegmentTimeline.
                if matches!(self.stack.last(), Some((_, _, Frame::SegmentTimeline(_)))) {
                    let entry = timeline_entry_from_attrs(&attrs)?;
                    if let Some((_, _, Frame::SegmentTimeline(timeline))) = self.stack.last_mut() {
                        timeline.entries.push(entry);
                    }
                    self.note_unknown_attrs(line, &attrs, &["t", "d", "r"]);
                    if self_closing {
                        Ok(())
                    } else {
                        self.push(name, line, Frame::Unknown)
                    }
                } else {
                    self.warnings.push(MpdWarning::UnsupportedHere {
                        line,
                        name: name.clone(),
                    });
                    self.skip_start(line, name, attrs, self_closing)
                }
            }
            "Initialization" => {
                // SegmentList → sourceURL; SegmentBase → range.
                let source_url = attr(&attrs, "sourceURL").map(str::to_string);
                let range = attr(&attrs, "range").map(str::to_string);
                self.note_unknown_attrs(line, &attrs, &["sourceURL", "range"]);
                let mut unsupported = false;
                match self.stack.last_mut() {
                    Some((_, _, Frame::SegmentList(list))) => {
                        if list.initialization.is_none() {
                            list.initialization = source_url;
                        } else {
                            self.warnings
                                .push(MpdWarning::DuplicateSegmentDescription { line });
                        }
                    }
                    Some((_, _, Frame::SegmentBase(base))) => {
                        if base.initialization_range.is_none() {
                            base.initialization_range = range;
                        } else {
                            self.warnings
                                .push(MpdWarning::DuplicateSegmentDescription { line });
                        }
                    }
                    _ => unsupported = true,
                }
                if unsupported {
                    self.warnings.push(MpdWarning::UnsupportedHere {
                        line,
                        name: name.clone(),
                    });
                }
                if !self_closing {
                    self.push(name, line, Frame::Unknown)?;
                }
                Ok(())
            }
            "SegmentURL" => {
                let url = SegmentUrl {
                    media: attr(&attrs, "media").map(str::to_string),
                    media_range: attr(&attrs, "mediaRange").map(str::to_string),
                };
                self.note_unknown_attrs(line, &attrs, &["media", "mediaRange"]);
                if matches!(self.stack.last(), Some((_, _, Frame::SegmentList(_)))) {
                    if let Some((_, _, Frame::SegmentList(list))) = self.stack.last_mut() {
                        list.segment_urls.push(url);
                    }
                    if self_closing {
                        Ok(())
                    } else {
                        self.push(name, line, Frame::Unknown)
                    }
                } else {
                    self.warnings.push(MpdWarning::UnsupportedHere {
                        line,
                        name: name.clone(),
                    });
                    self.skip_start(line, name, attrs, self_closing)
                }
            }
            "ContentProtection" => {
                let record = ContentProtection {
                    scheme_id_uri: attr(&attrs, "schemeIdUri").map(str::to_string),
                    value: attr(&attrs, "value").map(str::to_string),
                    default_kid: attr(&attrs, "default_KID").map(str::to_string),
                };
                self.note_unknown_attrs(line, &attrs, &["schemeIdUri", "value", "default_KID"]);
                self.attach_protection(line, record, &name)?;
                if self_closing {
                    Ok(())
                } else {
                    // Children (e.g. cenc:pssh) are unmodeled; the
                    // frame keeps the end tag matched.
                    self.push(name, line, Frame::Unknown)
                }
            }
            "AudioChannelConfiguration"
            | "Role"
            | "Accessibility"
            | "EssentialProperty"
            | "SupplementalProperty"
            | "FramePacking" => {
                let descriptor = Descriptor {
                    element: name.clone(),
                    scheme_id_uri: attr(&attrs, "schemeIdUri").map(str::to_string),
                    value: attr(&attrs, "value").map(str::to_string),
                };
                self.note_unknown_attrs(line, &attrs, &["schemeIdUri", "value"]);
                self.attach_descriptor(line, descriptor);
                if self_closing {
                    Ok(())
                } else {
                    self.push(name, line, Frame::Unknown)
                }
            }
            "BaseURL" => {
                // Attributes on BaseURL (serviceLocation, range, …)
                // are unmodeled.
                self.note_unknown_attrs(line, &attrs, &[]);
                if self_closing {
                    // An empty (self-closing) BaseURL contributes
                    // an empty-string base — keep it verbatim.
                    self.attach_base_url(line, String::new());
                    Ok(())
                } else {
                    self.push(name, line, Frame::BaseUrl(String::new()))
                }
            }
            _ => {
                self.warnings.push(MpdWarning::UnknownElement {
                    line,
                    name: name.clone(),
                });
                self.skip_start(line, name, attrs, self_closing)
            }
        }
    }

    /// Shared tail for start tags of unmodeled/misplaced elements.
    fn skip_start(
        &mut self,
        line: usize,
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    ) -> Result<(), MpdError> {
        let _ = attrs;
        if self_closing {
            Ok(())
        } else {
            self.push(name, line, Frame::Unknown)
        }
    }

    /// Push a built frame, or (self-closing) attach it immediately.
    fn push_or_close(
        &mut self,
        name: String,
        line: usize,
        frame: Frame,
        self_closing: bool,
    ) -> Result<(), MpdError> {
        if self_closing {
            self.attach(line, frame);
            Ok(())
        } else {
            self.push(name, line, frame)
        }
    }

    fn apply_mpd_attrs(&mut self, line: usize, attrs: &[(String, String)]) -> Result<(), MpdError> {
        for (name, value) in attrs {
            match name.as_str() {
                "type" => {
                    self.mpd.mpd_type = match value.trim() {
                        "static" => MpdType::Static,
                        "dynamic" => MpdType::Dynamic,
                        _ => {
                            return Err(MpdError::BadValue { attr: "MPD@type" });
                        }
                    };
                }
                "profiles" => {
                    self.mpd.profiles = Some(value.clone());
                }
                "id" => {
                    self.mpd.id = Some(value.clone());
                }
                "availabilityStartTime" => {
                    self.mpd.availability_start_time = Some(value.clone());
                }
                "publishTime" => {
                    self.mpd.publish_time = Some(value.clone());
                }
                "mediaPresentationDuration" => {
                    self.mpd.media_presentation_duration =
                        Some(fatal_duration("MPD@mediaPresentationDuration", value)?);
                }
                "minimumUpdatePeriod" => {
                    self.mpd.minimum_update_period =
                        Some(fatal_duration("MPD@minimumUpdatePeriod", value)?);
                }
                "maxSegmentDuration" => {
                    self.mpd.max_segment_duration =
                        Some(fatal_duration("MPD@maxSegmentDuration", value)?);
                }
                "minBufferTime" => {
                    self.mpd.min_buffer_time = Some(fatal_duration("MPD@minBufferTime", value)?);
                }
                "timeShiftBufferDepth" => {
                    self.mpd.time_shift_buffer_depth =
                        Some(fatal_duration("MPD@timeShiftBufferDepth", value)?);
                }
                _ => {
                    self.warnings.push(MpdWarning::UnknownAttribute {
                        line,
                        name: name.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Warn about attributes outside `known` on an already-handled
    /// element (the modeled attrs were consumed by the caller).
    fn note_unknown_attrs(&mut self, line: usize, attrs: &[(String, String)], known: &[&str]) {
        for (name, _) in attrs {
            if !known.contains(&name.as_str()) {
                self.warnings.push(MpdWarning::UnknownAttribute {
                    line,
                    name: name.clone(),
                });
            }
        }
    }

    fn attach_protection(
        &mut self,
        line: usize,
        record: ContentProtection,
        name: &str,
    ) -> Result<(), MpdError> {
        match self.stack.last_mut() {
            Some((_, _, Frame::AdaptationSet(set))) => {
                set.content_protection.push(record);
                Ok(())
            }
            Some((_, _, Frame::Representation(rep))) => {
                rep.content_protection.push(record);
                Ok(())
            }
            _ => {
                self.warnings.push(MpdWarning::UnsupportedHere {
                    line,
                    name: name.to_string(),
                });
                Ok(())
            }
        }
    }

    fn attach_descriptor(&mut self, line: usize, descriptor: Descriptor) {
        match self.stack.last_mut() {
            Some((_, _, Frame::AdaptationSet(set))) => set.descriptors.push(descriptor),
            Some((_, _, Frame::Representation(rep))) => rep.descriptors.push(descriptor),
            _ => {
                self.warnings.push(MpdWarning::UnsupportedHere {
                    line,
                    name: descriptor.element,
                });
            }
        }
    }

    fn attach_base_url(&mut self, line: usize, url: String) {
        let trimmed = url.trim().to_string();
        match self.stack.last_mut() {
            Some((_, _, Frame::Root)) => self.mpd.base_urls.push(trimmed),
            Some((_, _, Frame::Period(period))) => period.base_urls.push(trimmed),
            Some((_, _, Frame::AdaptationSet(set))) => set.base_urls.push(trimmed),
            Some((_, _, Frame::Representation(rep))) => rep.base_urls.push(trimmed),
            _ => {
                self.warnings.push(MpdWarning::UnsupportedHere {
                    line,
                    name: "BaseURL".to_string(),
                });
            }
        }
    }

    /// Attach a completed frame to its parent (the new stack top).
    fn attach(&mut self, line: usize, frame: Frame) {
        match frame {
            Frame::Period(period) => self.mpd.periods.push(period),
            Frame::AdaptationSet(set) => {
                if let Some((_, _, Frame::Period(period))) = self.stack.last_mut() {
                    period.adaptation_sets.push(set);
                }
            }
            Frame::Representation(rep) => {
                if let Some((_, _, Frame::AdaptationSet(set))) = self.stack.last_mut() {
                    set.representations.push(rep);
                }
            }
            Frame::SegmentTemplate(template) => match self.stack.last_mut() {
                Some((_, _, Frame::Period(period))) => {
                    if period.segment_template.is_none() {
                        period.segment_template = Some(template);
                    } else {
                        self.warnings
                            .push(MpdWarning::DuplicateSegmentDescription { line });
                    }
                }
                Some((_, _, Frame::AdaptationSet(set))) => {
                    if set.segment_template.is_none() {
                        set.segment_template = Some(template);
                    } else {
                        self.warnings
                            .push(MpdWarning::DuplicateSegmentDescription { line });
                    }
                }
                Some((_, _, Frame::Representation(rep))) => {
                    if rep.segment_template.is_none() {
                        rep.segment_template = Some(template);
                    } else {
                        self.warnings
                            .push(MpdWarning::DuplicateSegmentDescription { line });
                    }
                }
                _ => {
                    self.warnings.push(MpdWarning::UnsupportedHere {
                        line,
                        name: "SegmentTemplate".to_string(),
                    });
                }
            },
            Frame::SegmentList(list) => match self.stack.last_mut() {
                Some((_, _, Frame::AdaptationSet(set))) => {
                    attach_segment_list(&mut set.segment_list, list, &mut self.warnings, line);
                }
                Some((_, _, Frame::Representation(rep))) => {
                    attach_segment_list(&mut rep.segment_list, list, &mut self.warnings, line);
                }
                _ => {
                    self.warnings.push(MpdWarning::UnsupportedHere {
                        line,
                        name: "SegmentList".to_string(),
                    });
                }
            },
            Frame::SegmentBase(base) => match self.stack.last_mut() {
                Some((_, _, Frame::Representation(rep))) => {
                    if rep.segment_base.is_none() {
                        rep.segment_base = Some(base);
                    } else {
                        self.warnings
                            .push(MpdWarning::DuplicateSegmentDescription { line });
                    }
                }
                _ => {
                    self.warnings.push(MpdWarning::UnsupportedHere {
                        line,
                        name: "SegmentBase".to_string(),
                    });
                }
            },
            Frame::SegmentTimeline(timeline) => match self.stack.last_mut() {
                Some((_, _, Frame::SegmentTemplate(template))) => {
                    if template.segment_timeline.is_none() {
                        template.segment_timeline = Some(timeline);
                    } else {
                        self.warnings
                            .push(MpdWarning::DuplicateSegmentDescription { line });
                    }
                }
                _ => {
                    self.warnings.push(MpdWarning::UnsupportedHere {
                        line,
                        name: "SegmentTimeline".to_string(),
                    });
                }
            },
            Frame::BaseUrl(text) => self.attach_base_url(line, text),
            Frame::Root | Frame::Unknown => {}
        }
    }

    fn on_end(&mut self, line: usize, name: String) -> Result<(), MpdError> {
        // Find the matching frame, innermost first.
        let mut match_idx = None;
        for (i, (frame_name, _, _)) in self.stack.iter().enumerate().rev() {
            if *frame_name == name {
                match_idx = Some(i);
                break;
            }
        }
        let Some(idx) = match_idx else {
            // Stray end tag: noted, ignored (no frame matches).
            self.warnings.push(MpdWarning::MismatchedEndTag { line });
            return Ok(());
        };
        // Salvage: close (and attach) every frame from the top down
        // to and including the match — a missing intermediate end
        // tag is dirty-world, not fatal (the document still closes).
        while self.stack.len() > idx {
            if let Some((_, frame_line, frame)) = self.stack.pop() {
                self.attach(frame_line, frame);
            }
        }
        Ok(())
    }

    fn on_text(&mut self, line: usize, content: String) -> Result<(), MpdError> {
        if content.trim().is_empty() {
            // Whitespace between elements (pretty-printed XML) is
            // formatting, not data.
            return Ok(());
        }
        match self.stack.last_mut() {
            Some((_, _, Frame::BaseUrl(text))) => text.push_str(&content),
            Some((_, _, Frame::Unknown)) => {
                // Text inside an unmodeled element: already covered
                // by that element's warning; dropped silently.
            }
            _ => {
                self.warnings.push(MpdWarning::StrayText { line });
            }
        }
        Ok(())
    }

    /// EOF: anything still open is a truncation (manifest posture);
    /// no root at all is not-mpd.
    fn finish(&mut self) -> Result<(), MpdError> {
        if !self.root_seen {
            return Err(MpdError::NotMpd);
        }
        if let Some((_, line, _)) = self.stack.last() {
            return Err(MpdError::Truncated { line: *line });
        }
        if self.mpd.periods.is_empty() {
            self.warnings.push(MpdWarning::NoPeriods {
                line: self.root_line,
            });
        }
        Ok(())
    }
}

/// First-wins attachment for a second same-kind segment description.
fn attach_segment_list(
    slot: &mut Option<SegmentList>,
    list: SegmentList,
    warnings: &mut Vec<MpdWarning>,
    line: usize,
) {
    if slot.is_none() {
        *slot = Some(list);
    } else {
        warnings.push(MpdWarning::DuplicateSegmentDescription { line });
    }
}

// Segment-template attachment inlines the first-wins rule (see the
// `attach` arms); `attach_segment_list` below is the list-shape
// helper of the same rule.

// -------------------------------------------------------- attr plumbing

/// First-match attribute lookup (duplicate attribute names are not
/// legal XML; the first occurrence wins, documented).
fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// Fatal-capable numeric (timeline-level attr).
fn fatal_u64(what: &'static str, value: &str) -> Result<u64, MpdError> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| MpdError::BadValue { attr: what })
}

/// Fatal-capable duration (timeline-level attr), seconds.
fn fatal_duration(what: &'static str, value: &str) -> Result<f64, MpdError> {
    parse_xs_duration(value).ok_or(MpdError::BadValue { attr: what })
}

/// Warn-capable numeric (representation-fact attr): junk → `None`
/// plus a warning.
fn warn_u64(
    line: usize,
    attr_name: &'static str,
    value: &str,
    warnings: &mut Vec<MpdWarning>,
) -> Option<u64> {
    match value.trim().parse::<u64>() {
        Ok(n) => Some(n),
        Err(_) => {
            warnings.push(MpdWarning::BadAttrValue {
                line,
                attr: attr_name,
            });
            None
        }
    }
}

/// Warn-capable boolean: "true"/"1", "false"/"0", junk → `None`
/// plus a warning.
fn warn_bool(
    line: usize,
    attr_name: &'static str,
    value: &str,
    warnings: &mut Vec<MpdWarning>,
) -> Option<bool> {
    match value.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => {
            warnings.push(MpdWarning::BadAttrValue {
                line,
                attr: attr_name,
            });
            None
        }
    }
}

fn apply_period_attrs(
    line: usize,
    attrs: &[(String, String)],
    period: &mut Period,
    warnings: &mut Vec<MpdWarning>,
) -> Result<(), MpdError> {
    for (name, value) in attrs {
        match name.as_str() {
            "id" => period.id = Some(value.clone()),
            "start" => {
                period.start = Some(fatal_duration("Period@start", value)?);
            }
            "duration" => {
                period.duration = Some(fatal_duration("Period@duration", value)?);
            }
            "bitstreamSwitching" => {
                period.bitstream_switching = warn_bool(line, "bitstreamSwitching", value, warnings);
            }
            _ => warnings.push(MpdWarning::UnknownAttribute {
                line,
                name: name.clone(),
            }),
        }
    }
    Ok(())
}

fn apply_adaptation_set_attrs(
    line: usize,
    attrs: &[(String, String)],
    set: &mut AdaptationSet,
    warnings: &mut Vec<MpdWarning>,
) {
    for (name, value) in attrs {
        match name.as_str() {
            "id" => set.id = Some(value.clone()),
            "contentType" => set.content_type = Some(value.clone()),
            "mimeType" => set.mime_type = Some(value.clone()),
            "codecs" => set.codecs = Some(value.clone()),
            "lang" => set.lang = Some(value.clone()),
            "segmentAlignment" => {
                set.segment_alignment = warn_bool(line, "segmentAlignment", value, warnings);
            }
            "bitstreamSwitching" => {
                set.bitstream_switching = warn_bool(line, "bitstreamSwitching", value, warnings);
            }
            "startWithSAP" => {
                set.start_with_sap = warn_u64(line, "startWithSAP", value, warnings);
            }
            "maxWidth" => set.max_width = warn_u64(line, "maxWidth", value, warnings),
            "maxHeight" => set.max_height = warn_u64(line, "maxHeight", value, warnings),
            "par" => set.par = Some(value.clone()),
            "frameRate" => set.frame_rate = Some(value.clone()),
            _ => warnings.push(MpdWarning::UnknownAttribute {
                line,
                name: name.clone(),
            }),
        }
    }
}

fn apply_representation_attrs(
    line: usize,
    attrs: &[(String, String)],
    rep: &mut Representation,
    warnings: &mut Vec<MpdWarning>,
) {
    for (name, value) in attrs {
        match name.as_str() {
            "id" => rep.id = Some(value.clone()),
            "bandwidth" => rep.bandwidth = warn_u64(line, "bandwidth", value, warnings),
            "width" => rep.width = warn_u64(line, "width", value, warnings),
            "height" => rep.height = warn_u64(line, "height", value, warnings),
            "frameRate" => rep.frame_rate = Some(value.clone()),
            "sar" => rep.sar = Some(value.clone()),
            "mimeType" => rep.mime_type = Some(value.clone()),
            "codecs" => rep.codecs = Some(value.clone()),
            "audioSamplingRate" => {
                rep.audio_sampling_rate = warn_u64(line, "audioSamplingRate", value, warnings);
            }
            _ => warnings.push(MpdWarning::UnknownAttribute {
                line,
                name: name.clone(),
            }),
        }
    }
    if !attrs.iter().any(|(k, _)| k == "bandwidth") {
        warnings.push(MpdWarning::MissingBandwidth { line });
    }
}

fn apply_segment_template_attrs(
    line: usize,
    attrs: &[(String, String)],
    template: &mut SegmentTemplate,
    warnings: &mut Vec<MpdWarning>,
) -> Result<(), MpdError> {
    for (name, value) in attrs {
        match name.as_str() {
            "timescale" => {
                template.timescale = Some(fatal_u64("SegmentTemplate@timescale", value)?)
            }
            "duration" => template.duration = Some(fatal_u64("SegmentTemplate@duration", value)?),
            "startNumber" => {
                template.start_number = Some(fatal_u64("SegmentTemplate@startNumber", value)?)
            }
            "presentationTimeOffset" => {
                template.presentation_time_offset =
                    Some(fatal_u64("SegmentTemplate@presentationTimeOffset", value)?)
            }
            "initialization" => template.initialization = Some(value.clone()),
            "media" => template.media = Some(value.clone()),
            _ => warnings.push(MpdWarning::UnknownAttribute {
                line,
                name: name.clone(),
            }),
        }
    }
    Ok(())
}

fn apply_segment_list_attrs(
    line: usize,
    attrs: &[(String, String)],
    list: &mut SegmentList,
    warnings: &mut Vec<MpdWarning>,
) -> Result<(), MpdError> {
    for (name, value) in attrs {
        match name.as_str() {
            "timescale" => list.timescale = Some(fatal_u64("SegmentList@timescale", value)?),
            "duration" => list.duration = Some(fatal_u64("SegmentList@duration", value)?),
            "startNumber" => list.start_number = Some(fatal_u64("SegmentList@startNumber", value)?),
            "presentationTimeOffset" => {
                list.presentation_time_offset =
                    Some(fatal_u64("SegmentList@presentationTimeOffset", value)?)
            }
            _ => warnings.push(MpdWarning::UnknownAttribute {
                line,
                name: name.clone(),
            }),
        }
    }
    Ok(())
}

fn apply_segment_base_attrs(
    line: usize,
    attrs: &[(String, String)],
    base: &mut SegmentBase,
    warnings: &mut Vec<MpdWarning>,
) {
    for (name, value) in attrs {
        match name.as_str() {
            "timescale" => base.timescale = warn_u64(line, "timescale", value, warnings),
            "presentationTimeOffset" => {
                base.presentation_time_offset =
                    warn_u64(line, "presentationTimeOffset", value, warnings)
            }
            "indexRange" => base.index_range = Some(value.clone()),
            "indexRangeExact" => {
                base.index_range_exact = warn_bool(line, "indexRangeExact", value, warnings);
            }
            _ => warnings.push(MpdWarning::UnknownAttribute {
                line,
                name: name.clone(),
            }),
        }
    }
}

fn timeline_entry_from_attrs(attrs: &[(String, String)]) -> Result<SegmentTimelineEntry, MpdError> {
    let t = match attr(attrs, "t") {
        Some(v) => Some(
            v.trim()
                .parse::<u64>()
                .map_err(|_| MpdError::BadValue { attr: "S@t" })?,
        ),
        None => None,
    };
    let d = attr(attrs, "d")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .ok_or(MpdError::BadValue { attr: "S@d" })?;
    let r = match attr(attrs, "r") {
        Some(v) => Some(
            v.trim()
                .parse::<i64>()
                .map_err(|_| MpdError::BadValue { attr: "S@r" })?,
        ),
        None => None,
    };
    Ok(SegmentTimelineEntry { t, d, r })
}

// ------------------------------------------------------------ utilities

/// `xs:duration` subset (ISO 8601): `[-]P[nD][T[nH][nM][n.nS]]`,
/// returning seconds. Years/months are **refused** (no exact
/// second value exists — honest numbers, no 365-day guessing);
/// non-finite results are refused likewise.
fn parse_xs_duration(raw: &str) -> Option<f64> {
    let s = raw.trim();
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let rest = rest.strip_prefix('P')?;
    let bytes = rest.as_bytes();
    let mut total = 0.0_f64;
    let mut saw = false;
    let mut i = 0_usize;
    // Date part: whole days only (fractions are seconds-only per
    // xs:duration).
    while i < bytes.len() {
        if bytes[i] == b'T' {
            i += 1;
            break;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let num = &rest[start..i];
        if num.is_empty() || i >= bytes.len() {
            return None;
        }
        match bytes[i] {
            b'D' => {
                total += num.parse::<f64>().ok()? * 86_400.0;
                saw = true;
            }
            // Years/months: no exact seconds; refused, not guessed.
            b'Y' | b'M' | b'W' => return None,
            _ => return None,
        }
        i += 1;
    }
    // Time part.
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        let num = &rest[start..i];
        if num.is_empty() || i >= bytes.len() {
            return None;
        }
        let value: f64 = num.parse().ok()?;
        match bytes[i] {
            b'H' => total += value * 3_600.0,
            b'M' => total += value * 60.0,
            b'S' => total += value,
            _ => return None,
        }
        saw = true;
        i += 1;
    }
    if !saw || !total.is_finite() {
        return None;
    }
    Some(if neg { -total } else { total })
}

/// Prefix-stripped local name (`dash:Representation` →
/// `Representation`; unprefixed names pass through).
fn local_name(raw: &str) -> String {
    match raw.rfind(':') {
        Some(i) => raw[i + 1..].to_string(),
        None => raw.to_string(),
    }
}

fn is_name_start(c: char) -> bool {
    c == '_' || c == ':' || c.is_alphabetic()
}

fn is_name_char(c: char) -> bool {
    is_name_start(c) || c == '-' || c == '.' || c.is_numeric()
}

/// The crate's D-1 line cap, applied to raw bytes between newlines
/// (the `xspf` rule; a copy of its helper — see module docs).
fn line_over_cap(input: &str) -> Option<(usize, usize)> {
    let mut line = 1_usize;
    let mut start = 0_usize;
    let bytes = input.as_bytes();
    let mut i = 0_usize;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let len = i - start;
            if len > MAX_LINE_BYTES {
                return Some((line, len));
            }
            line += 1;
            start = i + 1;
        }
        i += 1;
    }
    let last = input.len() - start;
    if last > MAX_LINE_BYTES {
        return Some((line, last));
    }
    None
}

/// Entity decode, **single pass** (no re-scanning — nested
/// `&amp;amp;` cannot expand; the entity-bomb bound). The five
/// named entities plus decimal/hex numeric references; unknown or
/// out-of-range references stay verbatim with a warning.
fn decode_entities(raw: &str, warnings: &mut Vec<MpdWarning>, line: usize) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        match after.find(';') {
            Some(semi) if semi > 0 => {
                let name = &after[..semi];
                match decode_one(name) {
                    Some(c) => out.push(c),
                    None => {
                        warnings.push(MpdWarning::BadEntity { line });
                        out.push('&');
                        out.push_str(name);
                        out.push(';');
                    }
                }
                rest = &after[semi + 1..];
            }
            _ => {
                // No terminator: literal '&'.
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_one(name: &str) -> Option<char> {
    match name {
        "amp" => return Some('&'),
        "lt" => return Some('<'),
        "gt" => return Some('>'),
        "quot" => return Some('"'),
        "apos" => return Some('\''),
        _ => {}
    }
    let num = name.strip_prefix('#')?;
    let value = if let Some(hex) = strip_hex(num) {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        num.parse::<u32>().ok()?
    };
    char::from_u32(value)
}

fn strip_hex(num: &str) -> Option<&str> {
    let lower = num.to_ascii_lowercase();
    if lower.starts_with("0x") {
        Some(&num[2..])
    } else {
        None
    }
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    /// Real ffmpeg 7.1.5 output (`-f dash -use_template 1
    /// -use_timeline 1`): 2 video representations (500k 320x240,
    /// 200k 160x120) + 1 audio (64k), SegmentTemplate +
    /// SegmentTimeline per representation. Command (worklog 6.C1a):
    /// ffmpeg -filter_complex "testsrc2=duration=6:size=320x240:
    /// rate=10[s0];testsrc2=duration=6:size=160x120:rate=10[s1];
    /// sine=frequency=440:duration=6[a0]" -map "[s0]" -map "[s1]"
    /// -map "[a0]" -c:v:0 libx264 -preset ultrafast -b:v:0 500k
    /// -c:v:1 libx264 -preset ultrafast -b:v:1 200k -pix_fmt
    /// yuv420p -c:a aac -b:a 64k -f dash -use_template 1
    /// -use_timeline 1 -adaptation_sets "id=0,streams=0,1
    /// id=1,streams=2" ffmpeg_av_template.mpd
    ///
    /// ffprobe differential (run this session, worklog 6.C1a):
    /// `ffprobe -v error -show_entries
    /// stream=index,codec_name,codec_type,width,height,sample_rate,
    /// channels -of default=noprint_wrappers=1` reports **3
    /// streams** — h264 320x240, h264 160x120, aac 44100 Hz mono
    /// — matching the model's representation ladder exactly (order,
    /// kinds, dimensions, sampling rate). ffprobe's `bit_rate`
    /// (550744 / 227661 / 64453) is its *measured* segment rate,
    /// deliberately different from the MPD's *declared* `bandwidth`
    /// (500000 / 200000 / 64000) — both recorded, neither guessed.
    #[test]
    fn ffmpeg_template_fixture_types_and_matches_ffprobe() {
        let text = include_str!("../tests/fixtures/dash/ffmpeg_av_template.mpd");
        let parsed = match parse_mpd(text) {
            Ok(p) => p,
            Err(e) => unreachable!("ffmpeg fixture must parse: {e}"),
        };
        let mpd = &parsed.mpd;
        assert_eq!(mpd.mpd_type, MpdType::Static);
        let duration = match mpd.media_presentation_duration {
            Some(d) => d,
            None => unreachable!("fixture carries mediaPresentationDuration"),
        };
        assert!((duration - 6.0).abs() < 1e-9);
        let min_buffer = match mpd.min_buffer_time {
            Some(d) => d,
            None => unreachable!("fixture carries minBufferTime"),
        };
        assert!((min_buffer - 12.0).abs() < 1e-9);
        let max_segment = match mpd.max_segment_duration {
            Some(d) => d,
            None => unreachable!("fixture carries maxSegmentDuration"),
        };
        assert!((max_segment - 5.0).abs() < 1e-9);
        assert!(
            mpd.profiles
                .as_deref()
                .is_some_and(|p| p.contains("isoff-live"))
        );
        assert_eq!(mpd.periods.len(), 1);
        let period = &mpd.periods[0];
        assert_eq!(period.id.as_deref(), Some("0"));
        assert_eq!(period.start, Some(0.0));
        assert_eq!(period.adaptation_sets.len(), 2);

        let video = &period.adaptation_sets[0];
        assert_eq!(video.content_type.as_deref(), Some("video"));
        assert_eq!(video.segment_alignment, Some(true));
        assert_eq!(video.bitstream_switching, Some(true));
        assert_eq!(video.start_with_sap, Some(1));
        assert_eq!(video.max_width, Some(320));
        assert_eq!(video.max_height, Some(240));
        assert_eq!(video.frame_rate.as_deref(), Some("10/1"));
        assert_eq!(video.par.as_deref(), Some("4:3"));
        assert_eq!(video.representations.len(), 2);

        let v0 = &video.representations[0];
        assert_eq!(v0.id.as_deref(), Some("0"));
        assert_eq!(v0.bandwidth, Some(500_000));
        assert_eq!(v0.width, Some(320));
        assert_eq!(v0.height, Some(240));
        assert_eq!(v0.mime_type.as_deref(), Some("video/mp4"));
        assert_eq!(v0.codecs.as_deref(), Some("avc1.42c014"));
        assert_eq!(v0.sar.as_deref(), Some("1:1"));
        let Some(t0) = v0.segment_template.as_ref() else {
            unreachable!("video rep 0 carries a SegmentTemplate");
        };
        assert_eq!(t0.timescale, Some(10_240));
        assert_eq!(t0.start_number, Some(1));
        assert_eq!(
            t0.initialization.as_deref(),
            Some("init-stream$RepresentationID$.m4s")
        );
        assert_eq!(
            t0.media.as_deref(),
            Some("chunk-stream$RepresentationID$-$Number%05d$.m4s")
        );
        let Some(timeline) = t0.segment_timeline.as_ref() else {
            unreachable!("video rep 0 carries a SegmentTimeline");
        };
        assert_eq!(timeline.entries.len(), 1);
        assert_eq!(timeline.entries[0].t, Some(0));
        assert_eq!(timeline.entries[0].d, 61_440);
        assert_eq!(timeline.entries[0].r, None);

        let v1 = &video.representations[1];
        assert_eq!(v1.id.as_deref(), Some("1"));
        assert_eq!(v1.bandwidth, Some(200_000));
        assert_eq!(v1.width, Some(160));
        assert_eq!(v1.height, Some(120));

        let audio = &period.adaptation_sets[1];
        assert_eq!(audio.content_type.as_deref(), Some("audio"));
        assert_eq!(audio.representations.len(), 1);
        let a0 = &audio.representations[0];
        assert_eq!(a0.id.as_deref(), Some("2"));
        assert_eq!(a0.bandwidth, Some(64_000));
        assert_eq!(a0.audio_sampling_rate, Some(44_100));
        assert_eq!(a0.codecs.as_deref(), Some("mp4a.40.2"));
        assert_eq!(a0.descriptors.len(), 1);
        assert_eq!(a0.descriptors[0].element, "AudioChannelConfiguration");
        assert!(
            a0.descriptors[0]
                .scheme_id_uri
                .as_deref()
                .is_some_and(|u| u.contains("audio_channel_configuration"))
        );
        assert_eq!(a0.descriptors[0].value.as_deref(), Some("1"));

        // ffprobe cross-check surface: 3 streams total, in order.
        let total_reps: usize = period
            .adaptation_sets
            .iter()
            .map(|s| s.representations.len())
            .sum();
        assert_eq!(total_reps, 3);
        assert_eq!(video.representations[0].width, Some(320));
        assert_eq!(video.representations[1].width, Some(160));
        assert_eq!(a0.audio_sampling_rate, Some(44_100));
    }

    /// Real ffmpeg `-use_template 0` output: the SegmentList +
    /// SegmentURL + Initialization@sourceURL shape. ffprobe 7.1.5's
    /// own DASH demuxer REFUSES this file ("Cannot get fragment,
    /// missing template URL" — it demands templates in this
    /// configuration); recorded as the oracle's gap, not ours: the
    /// differential for this fixture is the structural model
    /// against the file's own text (pinned below).
    #[test]
    fn ffmpeg_list_fixture_types() {
        let text = include_str!("../tests/fixtures/dash/ffmpeg_av_list.mpd");
        let parsed = match parse_mpd(text) {
            Ok(p) => p,
            Err(e) => unreachable!("ffmpeg list fixture must parse: {e}"),
        };
        let mpd = &parsed.mpd;
        assert_eq!(mpd.mpd_type, MpdType::Static);
        assert_eq!(mpd.periods.len(), 1);
        let period = &mpd.periods[0];
        assert_eq!(period.adaptation_sets.len(), 2);
        let video = &period.adaptation_sets[0];
        assert_eq!(video.representations.len(), 2);
        let v0 = &video.representations[0];
        let Some(list) = v0.segment_list.as_ref() else {
            unreachable!("video rep 0 carries a SegmentList");
        };
        assert_eq!(list.timescale, Some(1_000_000));
        assert_eq!(list.duration, Some(5_000_000));
        assert_eq!(list.start_number, Some(1));
        assert_eq!(list.initialization.as_deref(), Some("init-stream0.m4s"));
        assert_eq!(list.segment_urls.len(), 1);
        assert_eq!(
            list.segment_urls[0].media.as_deref(),
            Some("chunk-stream0-00001.m4s")
        );
        let audio = &period.adaptation_sets[1];
        let a0 = &audio.representations[0];
        let Some(alist) = a0.segment_list.as_ref() else {
            unreachable!("audio rep carries a SegmentList");
        };
        assert_eq!(alist.initialization.as_deref(), Some("init-stream2.m4s"));
        assert!(!alist.segment_urls.is_empty());
        assert!(alist.segment_urls.iter().all(|u| {
            u.media
                .as_deref()
                .is_some_and(|m| m.starts_with("chunk-stream2-"))
        }));
    }

    /// Hand-authored static manifest (ISO/IEC 23009-1 shapes the
    /// ffmpeg muxer never emits): duration-based SegmentTemplate
    /// (no timeline), ContentProtection recorded, per-level
    /// BaseURLs, xml:lang, Role descriptor, SegmentBase shape.
    #[test]
    fn hand_static_fixture_full_surface() {
        let text = include_str!("../tests/fixtures/dash/hand_static_cenc.mpd");
        let parsed = match parse_mpd(text) {
            Ok(p) => p,
            Err(e) => unreachable!("hand static fixture must parse: {e}"),
        };
        let mpd = &parsed.mpd;
        assert_eq!(mpd.mpd_type, MpdType::Static);
        let duration = match mpd.media_presentation_duration {
            Some(d) => d,
            None => unreachable!("fixture carries mediaPresentationDuration"),
        };
        assert!((duration - 120.0).abs() < 1e-9);
        assert_eq!(mpd.base_urls, ["https://cdn.example/vod/"]); // lumen-allow-literal: URL parse/redaction test vector — data under test
        assert_eq!(mpd.periods.len(), 1);
        let period = &mpd.periods[0];
        assert_eq!(period.base_urls, ["p0/"]);
        let set = &period.adaptation_sets[0];
        assert_eq!(set.lang.as_deref(), Some("en"));
        assert_eq!(set.mime_type.as_deref(), Some("video/mp4"));
        assert_eq!(set.codecs.as_deref(), Some("avc1.640028"));
        assert_eq!(set.content_protection.len(), 1);
        let cp = &set.content_protection[0];
        assert_eq!(
            cp.scheme_id_uri.as_deref(),
            Some("urn:mpeg:dash:mp4protection:2011")
        );
        assert_eq!(cp.value.as_deref(), Some("cenc"));
        assert_eq!(
            cp.default_kid.as_deref(),
            Some("11223344-5566-7788-99aa-bbccddeeff00")
        );
        // DRM is recorded, never refused here (the session owns
        // that refusal) — no error, no warning about it.
        assert!(
            !parsed
                .warnings
                .iter()
                .any(|w| matches!(w, MpdWarning::StrayText { .. }))
        );

        assert_eq!(set.representations.len(), 3);
        let r0 = &set.representations[0];
        assert_eq!(r0.id.as_deref(), Some("v-1080"));
        assert_eq!(r0.bandwidth, Some(4_800_000));
        assert_eq!(r0.width, Some(1920));
        assert_eq!(r0.height, Some(1080));
        let Some(template) = set.segment_template.as_ref() else {
            unreachable!("set carries the shared SegmentTemplate");
        };
        assert_eq!(template.timescale, Some(900_000));
        assert_eq!(template.duration, Some(4_500_000));
        assert_eq!(template.start_number, Some(1));
        assert_eq!(
            template.initialization.as_deref(),
            Some("$RepresentationID$/init.mp4")
        );
        assert_eq!(
            template.media.as_deref(),
            Some("$RepresentationID$/seg-$Number$.m4s")
        );
        assert!(template.segment_timeline.is_none());
        assert!(r0.segment_template.is_none()); // inherited, not copied
        assert_eq!(r0.base_urls.len(), 1);
        assert_eq!(r0.base_urls[0], "hi/");

        // The subtitle set exercises SegmentBase + Role descriptor.
        let sub = &period.adaptation_sets[1];
        assert_eq!(sub.content_type.as_deref(), Some("text"));
        let sr = &sub.representations[0];
        let Some(base) = sr.segment_base.as_ref() else {
            unreachable!("subtitle rep carries a SegmentBase");
        };
        assert_eq!(base.index_range.as_deref(), Some("500-1200"));
        assert_eq!(base.index_range_exact, Some(true));
        assert_eq!(base.initialization_range.as_deref(), Some("0-499"));
        assert_eq!(sub.descriptors.len(), 1);
        assert_eq!(sub.descriptors[0].element, "Role");
        assert_eq!(sub.descriptors[0].value.as_deref(), Some("subtitle"));
    }

    /// Hand-authored dynamic manifest: type, minimumUpdatePeriod,
    /// availabilityStartTime (recorded verbatim), timeShiftBufferDepth,
    /// SegmentTimeline with t/d/r (negative r included),
    /// presentationTimeOffset.
    #[test]
    fn hand_dynamic_fixture_types() {
        let text = include_str!("../tests/fixtures/dash/hand_dynamic.mpd");
        let parsed = match parse_mpd(text) {
            Ok(p) => p,
            Err(e) => unreachable!("hand dynamic fixture must parse: {e}"),
        };
        let mpd = &parsed.mpd;
        assert_eq!(mpd.mpd_type, MpdType::Dynamic);
        let update_period = match mpd.minimum_update_period {
            Some(d) => d,
            None => unreachable!("fixture carries minimumUpdatePeriod"),
        };
        assert!((update_period - 2.0).abs() < 1e-9);
        let shift_depth = match mpd.time_shift_buffer_depth {
            Some(d) => d,
            None => unreachable!("fixture carries timeShiftBufferDepth"),
        };
        assert!((shift_depth - 30.0).abs() < 1e-9);
        assert_eq!(
            mpd.availability_start_time.as_deref(),
            Some("2026-10-31T12:00:00Z")
        );
        assert_eq!(mpd.publish_time.as_deref(), Some("2026-10-31T12:00:10Z"));
        let period = &mpd.periods[0];
        let set = &period.adaptation_sets[0];
        let rep = &set.representations[0];
        let Some(template) = rep.segment_template.as_ref() else {
            unreachable!("rep carries its own SegmentTemplate");
        };
        assert_eq!(template.timescale, Some(90_000));
        assert_eq!(template.presentation_time_offset, Some(4_500_000));
        assert_eq!(template.duration, None);
        let Some(timeline) = template.segment_timeline.as_ref() else {
            unreachable!("rep carries a SegmentTimeline");
        };
        assert_eq!(timeline.entries.len(), 3);
        assert_eq!(timeline.entries[0].t, Some(4_500_000));
        assert_eq!(timeline.entries[0].d, 1_800_000);
        assert_eq!(timeline.entries[0].r, Some(4));
        assert_eq!(timeline.entries[1].t, None);
        assert_eq!(timeline.entries[1].d, 900_000);
        assert_eq!(timeline.entries[2].d, 1_800_000);
        assert_eq!(timeline.entries[2].r, Some(-1));
    }

    /// The xs:duration table: the ffmpeg spellings, hand shapes,
    /// and the refusals (years/months/weeks, missing P, empty).
    #[test]
    fn duration_table() {
        let cases: &[(&str, f64)] = &[
            ("PT6.0S", 6.0),
            ("PT0S", 0.0),
            ("PT6S", 6.0),
            ("PT1H2M3.5S", 3723.5),
            ("PT59.9S", 59.9),
            ("P1D", 86_400.0),
            ("P1DT2H", 93_600.0),
            ("-PT30S", -30.0),
            ("PT1M30S", 90.0),
            (" P2DT0.5S ", 172_800.5),
        ];
        for (raw, want) in cases {
            let got = match parse_xs_duration(raw) {
                Some(v) => v,
                None => unreachable!("{raw:?} must parse"),
            };
            assert!((got - want).abs() < 1e-9, "{raw:?} -> {got}, want {want}");
        }
        for bad in [
            "P1Y", "P1M", "P1W", "6S", "PTS", "PT", "P", "", "P1.5D", "PT1H30", "PT-.5S", "PT1S3",
            "xPT1S",
        ] {
            assert!(parse_xs_duration(bad).is_none(), "{bad:?} must refuse");
        }
    }

    /// not-mpd: wrong root, no root at all, comment-only input.
    #[test]
    fn not_mpd_refuses() {
        for bad in [
            "<html><body>no</body></html>",
            "<Playlist/>",
            "",
            "<!-- only a comment -->",
            "<?xml version=\"1.0\"?>",
        ] {
            let Err(err) = parse_mpd(bad) else {
                unreachable!("{bad:?} must refuse NotMpd");
            };
            assert!(matches!(err, MpdError::NotMpd), "{bad:?} -> {err:?}");
            assert_eq!(err.code(), "playlists.mpd.not-mpd");
        }
    }

    /// bad-value: junk on the timeline/identity attrs is fatal.
    #[test]
    fn bad_value_refuses() {
        let cases: &[(&str, &str)] = &[
            ("<MPD type=\"vod\"></MPD>", "MPD@type"),
            (
                "<MPD mediaPresentationDuration=\"6s\"></MPD>",
                "MPD@mediaPresentationDuration",
            ),
            (
                "<MPD minimumUpdatePeriod=\"often\"></MPD>",
                "MPD@minimumUpdatePeriod",
            ),
            (
                "<MPD><Period start=\"late\"></Period></MPD>",
                "Period@start",
            ),
            (
                "<MPD><Period><SegmentTemplate timescale=\"fast\"/>\
                  </Period></MPD>",
                "SegmentTemplate@timescale",
            ),
            (
                "<MPD><Period><SegmentTemplate duration=\"-4\"/>\
                  </Period></MPD>",
                "SegmentTemplate@duration",
            ),
            (
                "<MPD><Period><SegmentTemplate><SegmentTimeline>\
                  <S t=\"zero\" d=\"100\"/></SegmentTimeline>\
                  </SegmentTemplate></Period></MPD>",
                "S@t",
            ),
            (
                "<MPD><Period><SegmentTemplate><SegmentTimeline>\
                  <S/></SegmentTimeline></SegmentTemplate></Period></MPD>",
                "S@d",
            ),
        ];
        for (input, want_attr) in cases {
            let Err(err) = parse_mpd(input) else {
                unreachable!("{input:?} must refuse BadValue");
            };
            match err {
                MpdError::BadValue { attr } => assert_eq!(attr, *want_attr),
                other => unreachable!("{input:?} -> {other:?}"),
            }
            assert_eq!(err.code(), "playlists.mpd.bad-value");
        }
    }

    /// truncated: the real ffmpeg fixture cut at several byte
    /// offsets — mid-XML-decl, mid-tag, mid-attr-value, after a
    /// Period open, one byte short of `</MPD>`.
    #[test]
    fn truncated_refuses() {
        let text = include_str!("../tests/fixtures/dash/ffmpeg_av_template.mpd");
        for cut in [7_usize, 120, 480, 900, text.len() - 3] {
            let sliced = &text[..cut];
            let Err(err) = parse_mpd(sliced) else {
                unreachable!("cut at {cut} must refuse Truncated");
            };
            assert!(
                matches!(err, MpdError::Truncated { .. }),
                "cut at {cut} -> {err:?}"
            );
            assert_eq!(err.code(), "playlists.mpd.truncated");
        }
    }

    /// depth-exceeded: 65-deep nesting inside a Representation.
    #[test]
    fn depth_exceeded_refuses() {
        let mut input = String::from("<MPD><Period><AdaptationSet><Representation>");
        for _ in 0..70 {
            input.push_str("<nested>");
        }
        input.push_str("</Representation></AdaptationSet></Period></MPD>");
        let Err(err) = parse_mpd(&input) else {
            unreachable!("deep nesting must refuse DepthExceeded");
        };
        assert!(matches!(err, MpdError::DepthExceeded { depth } if depth > MAX_MPD_DEPTH));
        assert_eq!(err.code(), "playlists.mpd.depth-exceeded");
    }

    /// The bounded-input battery: entity bomb (no expansion, no
    /// hang), oversize input, over-cap line, invalid UTF-8.
    #[test]
    fn bounded_input_battery() {
        // Billion-laughs shape: the classic nested-entity payload.
        // Custom entities are never registered here (DOCTYPE is
        // skipped), references stay verbatim with warnings, and the
        // run costs at most its own byte size — asserted by simply
        // completing, plus the verbatim-keep check.
        let bomb = "<?xml version=\"1.0\"?>\n<!DOCTYPE lolz [\n\
            <!ENTITY lol \"lol\">\n\
            <!ENTITY lol2 \"&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;\">\n\
            <!ENTITY lol3 \"&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;\">\n\
            ]>\n<MPD type=\"static\">\
            <Period>&lol3;&lol3;&lol3;</Period></MPD>";
        let parsed = match parse_mpd(bomb) {
            Ok(p) => p,
            Err(e) => unreachable!("entity bomb must parse bounded: {e}"),
        };
        assert_eq!(parsed.mpd.periods.len(), 1);
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| matches!(w, MpdWarning::BadEntity { .. }))
        );

        // Oversize: just over the 8 MiB budget.
        let huge = " ".repeat(MAX_MPD_BYTES + 1);
        let Err(err) = parse_mpd(&huge) else {
            unreachable!("oversize must refuse TooLarge");
        };
        assert!(matches!(err, MpdError::TooLarge { .. }));
        assert_eq!(err.code(), "playlists.mpd.too-large");

        // Over-cap single line (64 KiB).
        let long_line = format!("<MPD note=\"{}\"></MPD>", "x".repeat(MAX_LINE_BYTES + 8));
        let Err(err) = parse_mpd(&long_line) else {
            unreachable!("over-cap line must refuse LineTooLong");
        };
        assert!(matches!(err, MpdError::LineTooLong { .. }));

        // Non-UTF-8 bytes.
        let Err(err) = parse_mpd_bytes(&[0x3c, 0x4d, 0x50, 0x44, 0x20, 0xff]) else {
            unreachable!("non-UTF-8 must refuse InvalidUtf8");
        };
        assert!(matches!(err, MpdError::InvalidUtf8));
        assert_eq!(err.code(), "playlists.mpd.invalid-utf8");
    }

    /// Dirty-world tolerance: unknown elements/attrs warn (never
    /// fail); a missing intermediate end tag salvages what closed;
    /// junk representation facts degrade per-field; a duplicate
    /// segment description keeps the first.
    #[test]
    fn dirty_world_degrades_to_warnings() {
        // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        let input = "<MPD type=\"static\" vendor=\"Acme (unmodeled)\">\
            <UTCTiming schemeIdUri=\"urn:mpeg:dash:utc:http-iso:2014\" \
            value=\"https://time.example/t\"/>\
            <Period id=\"p1\">\
            <AdaptationSet mimeType=\"video/mp4\">\
            <Representation id=\"r\" bandwidth=\"4,000\" width=\"1920\" height=\"junk\"/>\
            <Representation id=\"nobw\"/>\
            <SegmentTemplate timescale=\"1000\" duration=\"2000\" media=\"$Number$\"/>\
            <SegmentTemplate timescale=\"1000\" duration=\"9999\" media=\"$Other$\"/>\
            </AdaptationSet>\
            </MPD>";
        let parsed = match parse_mpd(input) {
            Ok(p) => p,
            Err(e) => unreachable!("dirty manifest must parse: {e}"),
        };
        let mpd = &parsed.mpd;
        assert_eq!(mpd.periods.len(), 1);
        let set = &mpd.periods[0].adaptation_sets[0];
        assert_eq!(set.representations.len(), 2);
        // Junk facts: warned + None, rung survives.
        assert_eq!(set.representations[0].bandwidth, None);
        assert_eq!(set.representations[0].width, Some(1920));
        assert_eq!(set.representations[0].height, None);
        assert!(parsed.warnings.iter().any(|w| matches!(
            w,
            MpdWarning::BadAttrValue {
                attr: "bandwidth",
                ..
            }
        )));
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| matches!(w, MpdWarning::BadAttrValue { attr: "height", .. }))
        );
        // Missing bandwidth (spec-required): warned + None.
        assert_eq!(set.representations[1].bandwidth, None);
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| matches!(w, MpdWarning::MissingBandwidth { .. }))
        );
        // Duplicate segment description: first wins.
        let Some(template) = set.segment_template.as_ref() else {
            unreachable!("set carries a SegmentTemplate");
        };
        assert_eq!(template.duration, Some(2000));
        assert_eq!(template.media.as_deref(), Some("$Number$"));
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| matches!(w, MpdWarning::DuplicateSegmentDescription { .. }))
        );
        // Unknown attr + unknown element: warned, tolerated.
        assert!(
            parsed.warnings.iter().any(
                |w| matches!(w, MpdWarning::UnknownAttribute { name, .. } if name == "vendor")
            )
        );
        assert!(
            parsed.warnings.iter().any(
                |w| matches!(w, MpdWarning::UnknownElement { name, .. } if name == "UTCTiming")
            )
        );

        // Salvage: AdaptationSet closes via Period's end tag
        // (missing </AdaptationSet>) — everything attaches.
        let salvage = "<MPD><Period><AdaptationSet mimeType=\"video/mp4\">\
            <Representation id=\"r\" bandwidth=\"1000\"/>\
            </Period></MPD>";
        let parsed = match parse_mpd(salvage) {
            Ok(p) => p,
            Err(e) => unreachable!("salvage shape must parse: {e}"),
        };
        assert_eq!(parsed.mpd.periods[0].adaptation_sets.len(), 1);
        assert_eq!(
            parsed.mpd.periods[0].adaptation_sets[0]
                .representations
                .len(),
            1
        );
        // Stray end tag: noted, ignored.
        let stray = "<MPD type=\"static\"></Banana></MPD>";
        let parsed = match parse_mpd(stray) {
            Ok(p) => p,
            Err(e) => unreachable!("stray end tag must parse: {e}"),
        };
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| matches!(w, MpdWarning::MismatchedEndTag { .. }))
        );
    }

    /// Empty/self-closing root parses; zero periods is a recorded
    /// warning (the session refuses, the model records).
    #[test]
    fn empty_root_records_no_periods() {
        let parsed = match parse_mpd("<MPD type=\"static\"/>") {
            Ok(p) => p,
            Err(e) => unreachable!("empty root must parse: {e}"),
        };
        assert_eq!(parsed.mpd.mpd_type, MpdType::Static);
        assert!(parsed.mpd.periods.is_empty());
        assert!(matches!(
            parsed.warnings.first(),
            Some(MpdWarning::NoPeriods { .. })
        ));
    }

    /// Namespaced/prefixed input: `<dash:MPD>` is an MPD;
    /// `xml:lang` matches by local name; prefixed children match.
    #[test]
    fn namespace_tolerant_local_names() {
        let input = "<dash:MPD xmlns:dash=\"urn:mpeg:dash:schema:mpd:2011\" \
            type=\"static\">\
            <dash:Period><dash:AdaptationSet xml:lang=\"de\" mimeType=\"video/mp4\">\
            <dash:Representation id=\"r\" bandwidth=\"1000\"/>\
            </dash:AdaptationSet></dash:Period></dash:MPD>";
        let parsed = match parse_mpd(input) {
            Ok(p) => p,
            Err(e) => unreachable!("prefixed manifest must parse: {e}"),
        };
        let set = &parsed.mpd.periods[0].adaptation_sets[0];
        assert_eq!(set.lang.as_deref(), Some("de"));
        assert_eq!(set.representations.len(), 1);
    }

    /// BaseURL text is entity-decoded; template strings ride
    /// attributes verbatim (entity-decoded once, never re-scanned).
    #[test]
    fn base_url_and_entity_decoding() {
        // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        let input = "<MPD type=\"static\">\
            <BaseURL>https://cdn.example/a&amp;b/</BaseURL>\
            <Period><AdaptationSet mimeType=\"video/mp4\">\
            <Representation id=\"r\" bandwidth=\"1000\">\
            <BaseURL>../alt/</BaseURL>\
            </Representation></AdaptationSet></Period></MPD>";
        let parsed = match parse_mpd(input) {
            Ok(p) => p,
            Err(e) => unreachable!("base url manifest must parse: {e}"),
        };
        assert_eq!(parsed.mpd.base_urls, ["https://cdn.example/a&b/"]); // lumen-allow-literal: URL parse/redaction test vector — data under test
        assert_eq!(
            parsed.mpd.periods[0].adaptation_sets[0].representations[0].base_urls,
            ["../alt/"]
        );
        // Nested entity: decoded exactly once, no re-expansion.
        let nested = "<MPD type=\"static\"><BaseURL>x&amp;amp;y</BaseURL></MPD>";
        let parsed = match parse_mpd(nested) {
            Ok(p) => p,
            Err(e) => unreachable!("nested entity manifest must parse: {e}"),
        };
        assert_eq!(parsed.mpd.base_urls, ["x&amp;y"]);
    }

    /// Every refusal code is pinned (the card's four + the
    /// bounded-input set).
    #[test]
    fn error_code_table() {
        let table: &[(MpdError, &str)] = &[
            (MpdError::NotMpd, "playlists.mpd.not-mpd"),
            (
                MpdError::BadValue { attr: "MPD@type" },
                "playlists.mpd.bad-value",
            ),
            (MpdError::Truncated { line: 12 }, "playlists.mpd.truncated"),
            (
                MpdError::DepthExceeded { depth: 65 },
                "playlists.mpd.depth-exceeded",
            ),
            (
                MpdError::LineTooLong {
                    line: 3,
                    length: 70_000,
                },
                "playlists.mpd.line-too-long",
            ),
            (
                MpdError::TooLarge {
                    size: 9,
                    limit: MAX_MPD_BYTES,
                },
                "playlists.mpd.too-large",
            ),
            (MpdError::InvalidUtf8, "playlists.mpd.invalid-utf8"),
        ];
        for (err, want) in table {
            assert_eq!(err.code(), *want);
            assert!(err.to_string().contains(*want), "{err}");
        }
    }

    /// fuzz-lite: random truncations and byte mutations of the
    /// real fixtures never panic (the crate's deterministic
    /// harness discipline — splitmix64, bounded passes).
    #[test]
    fn fuzz_lite_mutations_never_panic() {
        let seed_text = include_str!("../tests/fixtures/dash/ffmpeg_av_template.mpd");
        let bytes = seed_text.as_bytes();
        let mut state: u64 = 0x6d70_6421_6d70_6421; // "mpd!mpd!"
        let mut next = move || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let mut buf = Vec::with_capacity(bytes.len());
        for _ in 0..600 {
            buf.clear();
            buf.extend_from_slice(bytes);
            match next() % 4 {
                0 => {
                    // truncate at a random point
                    let cut = (next() as usize) % (buf.len() + 1);
                    buf.truncate(cut);
                }
                1 => {
                    // flip a random byte
                    if !buf.is_empty() {
                        let idx = (next() as usize) % buf.len();
                        let mask = (next() as u8) | 1;
                        buf[idx] ^= mask;
                    }
                }
                2 => {
                    // splice junk into the middle
                    let at = (next() as usize) % (buf.len() + 1);
                    let junk = [(next() as u8), b'<', b'&', b'"', 0xff];
                    for (k, b) in junk.iter().enumerate() {
                        let pos = (at + k).min(buf.len());
                        buf.insert(pos, *b);
                    }
                }
                _ => {
                    // cut a random window out
                    let at = (next() as usize) % buf.len().max(1);
                    let len = (next() as usize) % 64;
                    let end = (at + len).min(buf.len());
                    buf.drain(at..end);
                }
            }
            // Every mutation either returns or errors — never
            // panics (unwrap/expect are denied; this is the proof).
            let _ = parse_mpd_bytes(&buf);
        }
    }
}
