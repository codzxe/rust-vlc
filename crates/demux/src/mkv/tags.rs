//! Matroska `Tags` parsing (card 5.D2b — the metadata half of the
//! 5.D2 EXTRAS).
//!
//! The `Tags` master carries `Tag` records, each scoping a set of
//! `SimpleTag` name/value pairs at a target: the segment as a whole
//! (the **global** form — an absent `Targets` master or all-zero
//! UIDs, what ffmpeg surfaces as *format* metadata), a track
//! (`TagTrackUID` — ffmpeg's *stream* metadata), or an edition /
//! chapter / attachment by UID.
//!
//! # Structure (Matroska spec §Elements)
//!
//! `Tags` → `Tag` → { `Targets` → (`TargetTypeValue`, `TargetType`,
//! `TagTrackUID`, `TagEditionUID`, `TagChapterUID`,
//! `TagAttachmentUID`), `SimpleTag` → (`TagName`, `TagLanguage`,
//! `TagLanguageIETF`, `TagDefault`, `TagString` | `TagBinary`,
//! nested `SimpleTag`…) }.
//!
//! # Robustness model (the mkv zone's rules, card 5.D1)
//!
//! - Declared sizes are hints: every child read is clamped to its
//!   parent's effective end and the stream's physical end; a value
//!   cut by the *stream* is the typed truncation (the truncated
//!   SimpleTag path), a child crossing its *parent's* declared end is
//!   the parent's contained lie — tolerated, the walk stops at the
//!   parent bound.
//! - Unknown children are skipped by declared size (never fatal).
//! - `SimpleTag` nests (the spec's sub-tag form); the descent carries
//!   an explicit depth counter refused past [`MAX_TAG_NESTING`] with
//!   the typed structural error — no recursion without a guard.
//! - Unknown `TargetTypeValue` numbers are **tolerantly recorded**,
//!   never refused: the numeric hierarchy is open (spec values are
//!   10..=70 today; future/private levels keep their tags parseable).
//! - Memory is bounded (MP §12): tag count, simple-tag count, per-
//!   value size and the cumulative value budget all carry typed
//!   `ResourceLimit` refusals — the tag-bomb twin of the cue-table
//!   ceiling. The corpus's own meta-bomb (0017) carries 5 simple tags
//!   totalling ~47 KiB (a 32 KiB `LYRICS`); the budget is set with
//!   an order of headroom above that.
//! - `TagString` payloads may be NUL-padded (ffmpeg writes
//!   `DURATION` values with a trailing `\0`); the value ends at the
//!   first NUL, matching ffmpeg's reader exactly.
//!
//! # Seam (recorded honestly)
//!
//! The demuxer's `open` walk currently *skips* `Tags` (5.D1's
//! parse-and-skip tolerance). This module is the parse layer; wiring
//! it into `MkvDemuxer` (a `tags()` accessor) is a `mod.rs` change
//! the Lead merge performs together with D2a's chapters/attachments
//! — the parallel agent owns the same file's walk. Until then the
//! public `parse_tags` is the library seam (the integration tests
//! drive it over the real corpus).

use media_types::error::{ErrorContext, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

use crate::mkv::ebml::{self, EbmlReader, ElementHeader};
use crate::mkv::error;

// ---- Element IDs (the on-disk vint values, zero-padded like the
// ---- 5.D1 constants) ------------------------------------------------

/// `Tags` — the top-level metadata master.
pub const ID_TAGS: u32 = 0x1254_C367;
/// `Tag` — one scoped tag record.
pub const ID_TAG: u32 = 0x7373;
/// `Targets` — the scoping master.
pub const ID_TARGETS: u32 = 0x63C0;
/// `TargetTypeValue` — the numeric target level (default 50 = TRACK).
pub const ID_TARGET_TYPE_VALUE: u32 = 0x68CA;
/// `TargetType` — the ASCII target-level name (`TRACK`, `MOVIE`, …).
pub const ID_TARGET_TYPE: u32 = 0x63CA;
/// `TagTrackUID` — the targeted track(s) (0 = all / global).
pub const ID_TAG_TRACK_UID: u32 = 0x63C5;
/// `TagEditionUID` — the targeted edition(s).
pub const ID_TAG_EDITION_UID: u32 = 0x63C9;
/// `TagChapterUID` — the targeted chapter(s).
pub const ID_TAG_CHAPTER_UID: u32 = 0x63C4;
/// `TagAttachmentUID` — the targeted attachment(s).
pub const ID_TAG_ATTACHMENT_UID: u32 = 0x63C6;
/// `SimpleTag` — one name/value pair (may nest).
pub const ID_SIMPLE_TAG: u32 = 0x67C8;
/// `TagName` — the pair's key (UTF-8).
pub const ID_TAG_NAME: u32 = 0x45A3;
/// `TagLanguage` — the legacy language code (default `und`).
pub const ID_TAG_LANGUAGE: u32 = 0x447A;
/// `TagLanguageIETF` — the BCP 47 language tag.
pub const ID_TAG_LANGUAGE_IETF: u32 = 0x447B;
/// `TagDefault` — 1 when the pair is the default for its target
/// (spec default 1).
pub const ID_TAG_DEFAULT: u32 = 0x4484;
/// `TagString` — the UTF-8 value form.
pub const ID_TAG_STRING: u32 = 0x4487;
/// `TagBinary` — the opaque value form.
pub const ID_TAG_BINARY: u32 = 0x4485;

// ---- Ceilings (MP §12, the tag-bomb guards) -------------------------

/// Maximum `Tag` records per `Tags` element
/// `[PROPOSAL: 4096 — real files carry < 10; the corpus meta-bomb
/// (0017) carries 3]`.
pub const MAX_TAGS: usize = 4096;

/// Maximum `SimpleTag` records (nested included) per `Tags` element
/// `[PROPOSAL: 16384 — the chapter-bomb analog for tags; a list past
/// this is the typed refusal, never an unbounded Vec]`.
pub const MAX_SIMPLE_TAGS: usize = 16_384;

/// Maximum `SimpleTag` nesting depth
/// `[PROPOSAL: 8 — the spec's shape is ≤ 3 (Tags→Tag→SimpleTag→
/// nested SimpleTag); 8 is generous headroom, the guard is what
/// matters]`.
pub const MAX_TAG_NESTING: usize = 8;

/// Maximum bytes of one text value (`TagName`, `TagString`)
/// `[PROPOSAL: 1 MiB — the corpus meta-bomb's largest value is a
/// 32 KiB LYRICS; 32× headroom]`.
pub const MAX_TAG_TEXT_BYTES: u64 = 1024 * 1024;

/// Maximum bytes of one `TagBinary` value
/// `[PROPOSAL: 1 MiB — binary tag values are cover-art thumbnails and
/// signatures, KBs in practice]`.
pub const MAX_TAG_BINARY_BYTES: u64 = 1024 * 1024;

/// Cumulative value-byte budget per `Tags` element
/// `[PROPOSAL: 8 MiB — bounds a many-medium-values bomb the per-value
/// ceiling alone cannot]`.
pub const MAX_TAG_LIST_BYTES: u64 = 8 * 1024 * 1024;

/// The default `TargetTypeValue` when `Targets` declares none
/// (Matroska spec: 50 = TRACK level).
pub const DEFAULT_TARGET_TYPE_VALUE: u64 = 50;

// ---- Local typed errors (file-disjoint seam) --------------------------
//
// `mkv/error.rs` is 5.D1's file (the parallel 5.D2a agent owns its
// own additions); this zone mints its one tags-specific constructor
// locally with the same taxonomy — stage `Demux`, stable dotted code.

/// The tag structure exceeded a memory ceiling (`ResourceLimit`,
/// stage `Demux`) — the tag-bomb refusal.
fn tag_budget(budget: u64, attempted: u64) -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        ErrorContext::new("demux.mkv.tags.budget"),
    )
}

// ---- Model ------------------------------------------------------------

/// One `SimpleTag` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagValue {
    /// `TagString` — UTF-8 text, NUL padding stripped (the value ends
    /// at the first NUL, matching ffmpeg's reader).
    Text(String),
    /// `TagBinary` — opaque bytes, passed through raw.
    Binary(Vec<u8>),
    /// No value element present (a name-only pair, or a parent whose
    /// children are all nested pairs).
    None,
}

impl TagValue {
    /// The text form when this is a [`TagValue::Text`] value.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            TagValue::Text(s) => Some(s),
            TagValue::Binary(_) | TagValue::None => None,
        }
    }
}

/// One `SimpleTag`: a named value, possibly with nested sub-tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleTag {
    /// `TagName` — the pair's key (empty when absent/invalid — kept,
    /// not dropped: the pair still parses).
    pub name: String,
    /// `TagLanguage` — the legacy code (spec default `und`).
    pub language: String,
    /// `TagLanguageIETF` — the BCP 47 form, when declared.
    pub language_ietf: Option<String>,
    /// `TagDefault` — the pair is the default for its target.
    pub default: bool,
    /// `TagString` / `TagBinary`.
    pub value: TagValue,
    /// Nested `SimpleTag` children (depth-guarded).
    pub children: Vec<SimpleTag>,
}

impl SimpleTag {
    /// The pair as `(name, text)` when it carries a text value —
    /// the differential shape (ffprobe's `NAME=value` rendering).
    #[must_use]
    pub fn text_pair(&self) -> Option<(&str, &str)> {
        Some((self.name.as_str(), self.value.as_text()?))
    }
}

/// A `Tag` record's `Targets` scoping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagTargets {
    /// `TargetTypeValue` — the numeric target level (spec default 50
    /// = TRACK). Unknown/private values are recorded, never refused.
    pub target_type_value: u64,
    /// `TargetType` — the ASCII level name, when declared.
    pub target_type: Option<String>,
    /// `TagTrackUID` values (the element may repeat; 0 = all tracks).
    pub track_uids: Vec<u64>,
    /// `TagEditionUID` values.
    pub edition_uids: Vec<u64>,
    /// `TagChapterUID` values.
    pub chapter_uids: Vec<u64>,
    /// `TagAttachmentUID` values.
    pub attachment_uids: Vec<u64>,
}

impl Default for TagTargets {
    fn default() -> TagTargets {
        TagTargets {
            target_type_value: DEFAULT_TARGET_TYPE_VALUE,
            target_type: None,
            track_uids: Vec::new(),
            edition_uids: Vec::new(),
            chapter_uids: Vec::new(),
            attachment_uids: Vec::new(),
        }
    }
}

impl TagTargets {
    /// The **global** form: no nonzero UID in any list — an absent
    /// `Targets` master or the all-zero (UID 0) form. What ffmpeg
    /// surfaces as *format* metadata (the global-tag UID 0 rule).
    #[must_use]
    pub fn is_global(&self) -> bool {
        self.track_uids.iter().all(|&u| u == 0)
            && self.edition_uids.iter().all(|&u| u == 0)
            && self.chapter_uids.iter().all(|&u| u == 0)
            && self.attachment_uids.iter().all(|&u| u == 0)
    }

    /// Whether `uid` is among the targeted tracks (the UID 0 wildcard
    /// included).
    #[must_use]
    pub fn targets_track(&self, uid: u64) -> bool {
        self.track_uids.iter().any(|&u| u == 0 || u == uid)
    }
}

/// One `Tag`: a scoping plus its simple tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The `Targets` scoping (default = TRACK level when absent).
    pub targets: TagTargets,
    /// The `SimpleTag` pairs.
    pub simple_tags: Vec<SimpleTag>,
}

/// The parsed `Tags` element (one or more `Tag` records; the spec
/// allows the master to repeat at segment level — collect the lists).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagList {
    /// The `Tag` records in file order.
    pub tags: Vec<Tag>,
}

impl TagList {
    /// The global (segment-wide) pairs as `(name, value)` — top-level
    /// simple tags of global `Tag` records, text values only. The
    /// ffprobe `format_tags` shape.
    #[must_use]
    pub fn global_text_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for tag in &self.tags {
            if !tag.targets.is_global() {
                continue;
            }
            for simple in &tag.simple_tags {
                if let Some((name, value)) = simple.text_pair() {
                    pairs.push((name.to_string(), value.to_string()));
                }
            }
        }
        pairs
    }

    /// The pairs targeted at one track (by `TrackUID`), text values
    /// only — the ffprobe `stream_tags` shape.
    #[must_use]
    pub fn track_text_pairs(&self, track_uid: u64) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for tag in &self.tags {
            if !tag.targets.targets_track(track_uid) {
                continue;
            }
            for simple in &tag.simple_tags {
                if let Some((name, value)) = simple.text_pair() {
                    pairs.push((name.to_string(), value.to_string()));
                }
            }
        }
        pairs
    }

    /// The `Tag` records targeting one track by `TrackUID`.
    #[must_use]
    pub fn tags_for_track(&self, track_uid: u64) -> Vec<&Tag> {
        self.tags
            .iter()
            .filter(|tag| tag.targets.targets_track(track_uid))
            .collect()
    }
}

// ---- Parse ------------------------------------------------------------

/// The budget counters one `Tags` walk carries (the bomb guards).
#[derive(Default)]
struct TagBudget {
    /// Simple tags consumed (nested included).
    simple_tags: usize,
    /// Value bytes consumed (text + binary).
    value_bytes: u64,
}

/// Parse one top-level `Tags` element into a [`TagList`].
///
/// # Errors
/// [`error::truncated`] (via the reader) when the stream ends inside
/// a value; [`error::bad_structure`] for unknown-size children and
/// over-deep `SimpleTag` nesting; the typed `ResourceLimit`
/// (`demux.mkv.tags.budget`) when a count/value ceiling is exceeded.
pub fn parse_tags(
    reader: &mut EbmlReader,
    tags: &ElementHeader,
    bound: u64,
) -> Result<TagList, EngineError> {
    let end = tags.declared_end().unwrap_or(bound).min(bound);
    let mut list = TagList::default();
    let mut budget = TagBudget::default();
    // Declared Tag elements seen (degenerate records included — an
    // empty-record bomb must not slip past the count guard).
    let mut declared = 0usize;
    let mut pos = tags.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            // Unknown-size inside Tags: not the streaming form's scope.
            return Err(error::bad_structure(child.data_pos));
        };
        if child.id == ID_TAG {
            declared += 1;
            if declared > MAX_TAGS {
                return Err(tag_budget(MAX_TAGS as u64, declared as u64));
            }
            if let Some(tag) = parse_tag(reader, &child, child_end, &mut budget)? {
                list.tags.push(tag);
            }
        }
        // Unknown children (CRC-32, Void, future elements): skipped
        // by declared size.
        pos = child_end;
    }
    Ok(list)
}

/// Parse one `Tag` (a `Targets` master + `SimpleTag` children).
/// `Ok(None)` = a degenerate record (no usable content) — skipped,
/// the others survive.
fn parse_tag(
    reader: &mut EbmlReader,
    tag: &ElementHeader,
    end: u64,
    budget: &mut TagBudget,
) -> Result<Option<Tag>, EngineError> {
    let mut targets = TagTargets::default();
    let mut simple_tags = Vec::new();
    let mut pos = tag.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        match child.id {
            ID_TARGETS => targets = parse_targets(reader, &child, child_end)?,
            ID_SIMPLE_TAG => {
                if let Some(simple) = parse_simple_tag(reader, &child, child_end, 0, budget)? {
                    simple_tags.push(simple);
                }
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok((!simple_tags.is_empty()).then_some(Tag {
        targets,
        simple_tags,
    }))
}

/// Parse one `Targets` master. Unknown `TargetTypeValue` numbers are
/// recorded verbatim (tolerant skip of the unknown-level *semantics*,
/// never of the structure).
fn parse_targets(
    reader: &mut EbmlReader,
    targets: &ElementHeader,
    end: u64,
) -> Result<TagTargets, EngineError> {
    let mut out = TagTargets::default();
    let mut pos = targets.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ID_TARGET_TYPE_VALUE => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                // Tolerant: any value (or a degenerate zero-length
                // field, which the EBML default rule reads as the
                // default) is recorded.
                out.target_type_value =
                    ebml::uint_from_be(&bytes).unwrap_or(DEFAULT_TARGET_TYPE_VALUE);
            }
            ID_TARGET_TYPE => {
                out.target_type = read_text(reader, &child, child_end, MAX_TAG_TEXT_BYTES)?;
            }
            ID_TAG_TRACK_UID | ID_TAG_EDITION_UID | ID_TAG_CHAPTER_UID | ID_TAG_ATTACHMENT_UID => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                let uid = ebml::uint_from_be(&bytes).unwrap_or(0);
                match child.id {
                    ID_TAG_TRACK_UID => out.track_uids.push(uid),
                    ID_TAG_EDITION_UID => out.edition_uids.push(uid),
                    ID_TAG_CHAPTER_UID => out.chapter_uids.push(uid),
                    _ => out.attachment_uids.push(uid),
                }
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok(out)
}

/// Parse one `SimpleTag` (depth-guarded; nested `SimpleTag` children
/// recurse). `Ok(None)` = a degenerate pair (no name and no value) —
/// skipped, its siblings survive.
fn parse_simple_tag(
    reader: &mut EbmlReader,
    simple: &ElementHeader,
    end: u64,
    depth: usize,
    budget: &mut TagBudget,
) -> Result<Option<SimpleTag>, EngineError> {
    if depth >= MAX_TAG_NESTING {
        // The nesting guard: the spec's shape is ≤ 3 levels; deeper
        // declared nesting is corruption (or a bomb), typed.
        return Err(error::bad_structure(simple.data_pos));
    }
    if budget.simple_tags >= MAX_SIMPLE_TAGS {
        return Err(tag_budget(
            MAX_SIMPLE_TAGS as u64,
            MAX_SIMPLE_TAGS as u64 + 1,
        ));
    }
    budget.simple_tags += 1;
    let mut out = SimpleTag {
        name: String::new(),
        language: "und".to_string(),
        language_ietf: None,
        default: true,
        value: TagValue::None,
        children: Vec::new(),
    };
    let mut pos = simple.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ID_TAG_NAME => {
                out.name =
                    read_text(reader, &child, child_end, MAX_TAG_TEXT_BYTES)?.unwrap_or_default();
            }
            ID_TAG_LANGUAGE => {
                out.language = read_text(reader, &child, child_end, MAX_TAG_TEXT_BYTES)?
                    .unwrap_or_else(|| "und".to_string());
            }
            ID_TAG_LANGUAGE_IETF => {
                out.language_ietf = read_text(reader, &child, child_end, MAX_TAG_TEXT_BYTES)?;
            }
            ID_TAG_DEFAULT => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                out.default = ebml::uint_from_be(&bytes).unwrap_or(1) != 0;
            }
            ID_TAG_STRING => {
                if child.size > MAX_TAG_TEXT_BYTES {
                    return Err(tag_budget(MAX_TAG_TEXT_BYTES, child.size));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                budget.value_bytes = budget
                    .value_bytes
                    .checked_add(bytes.len() as u64)
                    .ok_or_else(|| error::bad_structure(child.data_pos))?;
                if budget.value_bytes > MAX_TAG_LIST_BYTES {
                    return Err(tag_budget(MAX_TAG_LIST_BYTES, budget.value_bytes));
                }
                out.value = TagValue::Text(text_from_padded(&bytes));
            }
            ID_TAG_BINARY => {
                if child.size > MAX_TAG_BINARY_BYTES {
                    return Err(tag_budget(MAX_TAG_BINARY_BYTES, child.size));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                budget.value_bytes = budget
                    .value_bytes
                    .checked_add(bytes.len() as u64)
                    .ok_or_else(|| error::bad_structure(child.data_pos))?;
                if budget.value_bytes > MAX_TAG_LIST_BYTES {
                    return Err(tag_budget(MAX_TAG_LIST_BYTES, budget.value_bytes));
                }
                out.value = TagValue::Binary(bytes);
            }
            ID_SIMPLE_TAG => {
                if let Some(nested) =
                    parse_simple_tag(reader, &child, child_end, depth + 1, budget)?
                {
                    out.children.push(nested);
                }
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok(
        (!out.name.is_empty() || !out.children.is_empty() || !matches!(out.value, TagValue::None))
            .then_some(out),
    )
}

/// Read a capped text child. `Ok(None)` = the payload was not valid
/// UTF-8 (kept as absent, never fabricated — the field is skipped,
/// the tag survives).
fn read_text(
    reader: &mut EbmlReader,
    child: &ElementHeader,
    end: u64,
    cap: u64,
) -> Result<Option<String>, EngineError> {
    if child.size > cap {
        return Err(tag_budget(cap, child.size));
    }
    let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
    let bytes = reader.read_exact_at(len, child.data_pos, end)?;
    Ok(ebml::utf8_from(&bytes))
}

/// The UTF-8 value with NUL padding stripped (the value ends at the
/// first NUL — ffmpeg's own reader rule; the corpus's `DURATION`
/// pairs carry one trailing `\0`).
fn text_from_padded(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    ebml::utf8_from(&bytes[..end]).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_globally_scope_the_uid_zero_form() {
        // Absent Targets (the default) and every all-zero list are
        // global; any nonzero UID opts out.
        let mut targets = TagTargets::default();
        assert!(targets.is_global());
        targets.track_uids.push(0);
        targets.chapter_uids.push(0);
        assert!(targets.is_global());
        assert!(targets.targets_track(7), "UID 0 is the all-tracks wildcard");
        // A list with the wildcard plus a specific UID still matches
        // everything (0 = all tracks), but is no longer global.
        targets.track_uids.push(7);
        assert!(!targets.is_global());
        assert!(targets.targets_track(7));
        assert!(targets.targets_track(8), "the 0 wildcard still matches");
        // Without the wildcard, only the listed UID matches.
        let mut scoped = TagTargets::default();
        scoped.track_uids.push(7);
        assert!(scoped.targets_track(7));
        assert!(!scoped.targets_track(8));
    }

    #[test]
    fn text_values_strip_nul_padding_like_ffmpeg() {
        assert_eq!(
            text_from_padded(b"00:00:02.104000000\0"),
            "00:00:02.104000000"
        );
        assert_eq!(text_from_padded(b"Lavf61.7.103"), "Lavf61.7.103");
        assert_eq!(text_from_padded(b"\0\0"), "");
        // Invalid UTF-8 never fabricates a value.
        assert_eq!(text_from_padded(&[0xFF, 0xFE]), "");
    }

    #[test]
    fn default_targets_are_track_level() {
        assert_eq!(TagTargets::default().target_type_value, 50);
    }

    #[test]
    fn budget_error_is_typed_resource_limit() {
        let err = tag_budget(1024, 2048);
        assert_eq!(err.stage, PipelineStage::Demux);
        assert_eq!(err.context.code, "demux.mkv.tags.budget");
        assert_eq!(
            err.kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::Memory,
                budget: 1024,
                attempted: 2048
            }
        );
        let text = err.to_string();
        assert!(text.starts_with("[demux] ") && text.contains("demux.mkv.tags.budget"));
    }

    #[test]
    fn global_and_track_pairs_shape_the_differential() {
        let mut list = TagList::default();
        list.tags.push(Tag {
            targets: TagTargets::default(),
            simple_tags: vec![
                SimpleTag {
                    name: "ENCODER".to_string(),
                    language: "und".to_string(),
                    language_ietf: None,
                    default: true,
                    value: TagValue::Text("Lavf61.7.103".to_string()),
                    children: Vec::new(),
                },
                SimpleTag {
                    name: "COVER_ART".to_string(),
                    language: "und".to_string(),
                    language_ietf: None,
                    default: true,
                    value: TagValue::Binary(vec![1, 2, 3]),
                    children: Vec::new(),
                },
            ],
        });
        let mut scoped = TagTargets::default();
        scoped.track_uids.push(42);
        list.tags.push(Tag {
            targets: scoped,
            simple_tags: vec![SimpleTag {
                name: "DURATION".to_string(),
                language: "und".to_string(),
                language_ietf: None,
                default: true,
                value: TagValue::Text("00:00:02.104000000".to_string()),
                children: Vec::new(),
            }],
        });
        // Binary values never surface as text pairs.
        assert_eq!(
            list.global_text_pairs(),
            vec![("ENCODER".to_string(), "Lavf61.7.103".to_string())]
        );
        assert_eq!(
            list.track_text_pairs(42),
            vec![("DURATION".to_string(), "00:00:02.104000000".to_string())]
        );
        assert!(list.track_text_pairs(43).is_empty());
        assert_eq!(list.tags_for_track(42).len(), 1);
    }
}
