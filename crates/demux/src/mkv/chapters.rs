//! Matroska `Chapters` — editions, chapter atoms and their display
//! records (card 5.D2a).
//!
//! Card 5.D1's top-level walk parse-and-skipped `Chapters`; this
//! module parses it into the typed [`ChapterList`] a UI needs,
//! following the landed core's rules:
//!
//! - **Chapter times are nanoseconds, full stop**: the Matroska
//!   spec declares `ChapterTimeStart`/`ChapterTimeEnd` in
//!   nanoseconds — they do **not** scale with `Info`'s
//!   `TimestampScale` (the format's own quirk: block timestamps are
//!   scale ticks, chapter timestamps are ns). Verified against
//!   ffprobe on the corpus (0007: raw `0x0F4240` = 1e6 ns with
//!   TimestampScale 1e6) and on the ffmpeg-generated fixture. The
//!   parse keeps the raw ns; the demuxer getter compares them
//!   against the segment duration.
//! - **Bounded memory (MP §12)**: editions, atoms, displays and
//!   per-atom track lists each carry a ceiling
//!   ([`MAX_EDITIONS`], [`MAX_CHAPTER_ATOMS`],
//!   [`MAX_CHAPTER_DISPLAYS`], [`MAX_CHAPTER_TRACKS`]). Over-ceiling
//!   editions/displays/tracks are skipped with damage; a total atom
//!   count past [`MAX_CHAPTER_ATOMS`] is the typed
//!   `demux.mkv.chapter-table-too-large` refusal (the cue-table
//!   precedent — never an unbounded allocation).
//! - **Recursive nesting is guarded**: a `ChapterAtom` may contain
//!   further `ChapterAtom`s (ordered editions nest). The recursion
//!   carries an explicit depth counter refused past the landed
//!   [`ebml::MAX_PARSE_DEPTH`] — a nesting bomb is the typed
//!   `bad_structure`, never a stack overflow.
//! - **Chapter time beyond the segment duration is a tolerant note**:
//!   such atoms are kept and flagged (`past_duration` /
//!   [`ChapterList::beyond_duration`]), never dropped and never an
//!   error — ffprobe shows them too.
//! - A `ChapterAtom` without `ChapterTimeStart` (mandatory per spec)
//!   is degenerate: skipped with damage, its siblings survive.
//!
//! # Exposure seam (the card's "chapter list for the UI layer")
//!
//! [`MkvDemuxer::chapters`] is the typed getter over the list the
//! demuxer **retains at `start`** (issue #34 wired the `open()` seam:
//! the on-demand positional re-scan of the 5.D2a card is gone — the
//! list is parsed once during the walk and held on the struct).

use media_types::error::{ErrorContext, LimitKind};
use media_types::time::TimeSpan;
use media_types::{EngineError, ErrorKind, PipelineStage};

use crate::mkv::ebml::{self, EbmlReader, ElementHeader};
use crate::mkv::error;
use crate::mkv::header::MAX_TEXT_FIELD;

// ---- Element IDs (on-disk form, length-marker bits intact) ----------

/// `Chapters` — the top-level chapters master.
pub const ID_CHAPTERS: u32 = 0x1043_A770;
/// `EditionEntry` — one edition (a chapter set).
pub const ID_EDITION_ENTRY: u32 = 0x45B9;
/// `EditionUID` (default 0 when absent).
pub const ID_EDITION_UID: u32 = 0x45BC;
/// `EditionFlagHidden` — the edition is not a user selection.
pub const ID_EDITION_FLAG_HIDDEN: u32 = 0x45BD;
/// `EditionFlagDefault` — the edition a player should pick.
pub const ID_EDITION_FLAG_DEFAULT: u32 = 0x45DB;
/// `EditionFlagOrdered` — the atoms are an ordered sequence.
pub const ID_EDITION_FLAG_ORDERED: u32 = 0x45DD;
/// `ChapterAtom` — one chapter (recursive).
pub const ID_CHAPTER_ATOM: u32 = 0xB6;
/// `ChapterUID` (default 0 when absent).
pub const ID_CHAPTER_UID: u32 = 0x73C4;
/// `ChapterTimeStart` — nanoseconds (spec: NOT scale ticks),
/// mandatory.
pub const ID_CHAPTER_TIME_START: u32 = 0x91;
/// `ChapterTimeEnd` — nanoseconds; absent = open-ended chapter.
pub const ID_CHAPTER_TIME_END: u32 = 0x92;
/// `ChapterFlagHidden`.
pub const ID_CHAPTER_FLAG_HIDDEN: u32 = 0x98;
/// `ChapterFlagEnabled` (default on).
pub const ID_CHAPTER_FLAG_ENABLED: u32 = 0x4598;
/// `ChapterTrack` — the tracks an ordered chapter applies to.
pub const ID_CHAPTER_TRACK: u32 = 0x8F;
/// `ChapterTrackNumber` — one track number inside `ChapterTrack`.
pub const ID_CHAPTER_TRACK_NUMBER: u32 = 0x89;
/// `ChapterDisplay` — the user-facing strings of one atom.
pub const ID_CHAPTER_DISPLAY: u32 = 0x80;
/// `ChapString` — the title.
pub const ID_CHAP_STRING: u32 = 0x85;
/// `ChapLanguage` — the legacy language code (`eng`, …).
pub const ID_CHAP_LANGUAGE: u32 = 0x437C;
/// `ChapLanguageIETF` — the BCP 47 form.
pub const ID_CHAP_LANGUAGE_IETF: u32 = 0x437D;
/// `ChapCountry` — the country code.
pub const ID_CHAP_COUNTRY: u32 = 0x437E;

/// Maximum editions trusted into the list
/// `[PROPOSAL: 64 — real files carry 1; over-ceiling editions are
/// skipped with damage, the rest survive]`.
pub const MAX_EDITIONS: usize = 64;

/// Maximum chapter atoms trusted into the whole list (all editions,
/// nested atoms included)
/// `[PROPOSAL: 65_536 — the corpus chapter bomb carries 2_580; a
/// table past this is the typed ResourceLimit refusal, the cue-table
/// precedent]`.
pub const MAX_CHAPTER_ATOMS: usize = 65_536;

/// Maximum display records per atom
/// `[PROPOSAL: 16 — one per language; more is a metadata bomb]`.
pub const MAX_CHAPTER_DISPLAYS: usize = 16;

/// Maximum track numbers per atom (ordered editions)
/// `[PROPOSAL: 64 — the track-table ceiling's value]`.
pub const MAX_CHAPTER_TRACKS: usize = 64;

// ---- The data model --------------------------------------------------

/// One `ChapterDisplay`: the localized strings of a chapter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChapterDisplay {
    /// `ChapString` — the title, when declared and valid UTF-8.
    pub string: Option<String>,
    /// `ChapLanguage` — the legacy Matroska code (`eng`, …).
    pub language: Option<String>,
    /// `ChapLanguageIETF` — the BCP 47 form.
    pub language_ietf: Option<String>,
    /// `ChapCountry` — the country code.
    pub country: Option<String>,
}

/// One `ChapterAtom` (a chapter). Nested atoms (ordered editions)
/// land in [`ChapterAtom::children`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChapterAtom {
    /// `ChapterUID` (0 when absent).
    pub uid: u64,
    /// `ChapterTimeStart` in **nanoseconds** (the spec's own unit —
    /// chapter times ignore `TimestampScale`, unlike block times).
    pub start_ns: u64,
    /// `ChapterTimeEnd` in nanoseconds; `None` = open-ended.
    pub end_ns: Option<u64>,
    /// `ChapterFlagHidden`.
    pub hidden: bool,
    /// `ChapterFlagEnabled` (default on).
    pub enabled: bool,
    /// The track numbers an ordered chapter applies to.
    pub track_numbers: Vec<u64>,
    /// The display records (title/languages), declared order.
    pub displays: Vec<ChapterDisplay>,
    /// Nested `ChapterAtom`s (ordered editions).
    pub children: Vec<ChapterAtom>,
    /// Tolerant note (set by the demuxer getter): this atom's times
    /// extend past the segment's declared `Duration` — kept, flagged,
    /// never dropped.
    pub past_duration: bool,
}

impl ChapterAtom {
    /// The first display's title (the UI default; `None` when the
    /// atom has no titled display).
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.displays.iter().find_map(|d| d.string.as_deref())
    }

    /// This atom and its descendants, depth-first pre-order.
    pub fn flatten_into<'a>(&'a self, out: &mut Vec<&'a ChapterAtom>) {
        out.push(self);
        for child in &self.children {
            child.flatten_into(out);
        }
    }
}

/// One `EditionEntry` (a chapter set).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditionEntry {
    /// `EditionUID` (0 when absent).
    pub uid: u64,
    /// `EditionFlagHidden`.
    pub hidden: bool,
    /// `EditionFlagDefault` — the edition a player opens with (issue
    /// #34: parsed for the seek-target selection below; the twin
    /// flag lives in `editions::Edition` — that module's link-list
    /// parse is the tags-zone seam, this one carries the structure).
    pub default: bool,
    /// `EditionFlagOrdered`.
    pub ordered: bool,
    /// The top-level atoms, declared order.
    pub atoms: Vec<ChapterAtom>,
}

/// The parsed `Chapters` element (+ every `Chapters` element found by
/// the demuxer getter, merged).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChapterList {
    /// The editions, declared order.
    pub editions: Vec<EditionEntry>,
    /// Tolerant note: how many atoms (nested included) carry times
    /// past the segment's declared duration — kept, never dropped.
    pub beyond_duration: usize,
    /// Damage rows (bytes, reason) — the landed core's shape.
    pub damage: Vec<(u64, &'static str)>,
}

impl ChapterList {
    /// Every atom, depth-first pre-order across editions — the
    /// flattening ffprobe's chapter list uses (the differential's
    /// shape).
    #[must_use]
    pub fn atoms_flattened(&self) -> Vec<&ChapterAtom> {
        let mut out = Vec::new();
        for edition in &self.editions {
            for atom in &edition.atoms {
                atom.flatten_into(&mut out);
            }
        }
        out
    }

    /// The edition a player opens with: the `FlagDefault` one, else
    /// the first non-hidden, else the first (the spec's own selection
    /// guidance — the same rule `editions::EditionList` applies to its
    /// link-list twin; recorded honestly: a file with no default flag
    /// still chapters, the choice falls back).
    #[must_use]
    pub fn default_edition(&self) -> Option<&EditionEntry> {
        self.editions
            .iter()
            .find(|e| e.default)
            .or_else(|| self.editions.iter().find(|e| !e.hidden))
            .or_else(|| self.editions.first())
    }

    /// Flag every atom whose nanosecond times extend past
    /// `duration_ns` — the tolerant note (kept, never dropped),
    /// applied by the demuxer's `open()` to the retained list.
    /// Returns the flagged count.
    pub fn flag_beyond_duration(&mut self, duration_ns: u64) -> usize {
        self.beyond_duration = flag_beyond_duration(&mut self.editions, duration_ns);
        self.beyond_duration
    }
}

/// One chapter expressed as a **seek target** on the segment's clock
/// (issue #34: the surface a chapter-navigation command consumes —
/// `MkvDemuxer::seek(target.span)` plus `title`/`uid` for the UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterTarget {
    /// The atom's `ChapterUID` (0 when the file declares none).
    pub uid: u64,
    /// The first display's title (the UI default).
    pub title: Option<String>,
    /// The chapter's span: `start` present (scale ticks on the
    /// segment's clock), `end` when the file declares one that fits
    /// the clock (else `Stamp::Missing` — the honest open end).
    pub span: TimeSpan,
}

/// Chapter damage rows: (bytes, reason).
pub type ChapterDamage = Vec<(u64, &'static str)>;

/// The `Chapters` parse. An unknown-size `Chapters` clamps to `bound`
/// (the segment end — masters may legally carry the unknown size).
///
/// # Errors
/// [`error::truncated`] when an inner element's declared extent
/// overruns its parent (the contained lie) or the stream physically
/// ends inside it; [`error::bad_structure`] for unknown-size inner
/// elements, out-of-grammar fields and the nesting depth guard;
/// `demux.mkv.chapter-table-too-large` over the atom ceiling.
pub fn parse_chapters(
    reader: &mut EbmlReader,
    chapters: &ElementHeader,
    bound: u64,
) -> Result<ChapterList, EngineError> {
    let end = chapters.declared_end().unwrap_or(bound).min(bound);
    let mut list = ChapterList::default();
    let mut atoms_total = 0usize;
    let mut pos = chapters.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child_end > end {
            // The edition claims bytes past the master: the contained
            // lie, typed at the master's end.
            return Err(error::truncated(child_end, end));
        }
        if child.id == ID_EDITION_ENTRY {
            if list.editions.len() >= MAX_EDITIONS {
                list.damage.push((child_end - pos, "edition-over-ceiling"));
            } else {
                let (edition, damage) = parse_edition(reader, &child, child_end, &mut atoms_total)?;
                list.damage.extend(damage);
                list.editions.push(edition);
            }
        }
        // Unknown `Chapters` children: skipped by declared size.
        pos = child_end;
    }
    Ok(list)
}

/// One `EditionEntry` parse (its damage rows bubble up; the atom
/// budget is shared across the whole list).
fn parse_edition(
    reader: &mut EbmlReader,
    edition: &ElementHeader,
    end: u64,
    atoms_total: &mut usize,
) -> Result<(EditionEntry, ChapterDamage), EngineError> {
    let mut parsed = EditionEntry::default();
    let mut damage: ChapterDamage = Vec::new();
    let mut pos = edition.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child_end > end {
            return Err(error::truncated(child_end, end));
        }
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ID_EDITION_UID => {
                if let Some(value) = read_uint(reader, &child, len, child_end)? {
                    parsed.uid = value;
                }
            }
            ID_EDITION_FLAG_HIDDEN => {
                parsed.hidden = read_flag(reader, &child, len, child_end)?;
            }
            ID_EDITION_FLAG_DEFAULT => {
                parsed.default = read_flag(reader, &child, len, child_end)?;
            }
            ID_EDITION_FLAG_ORDERED => {
                parsed.ordered = read_flag(reader, &child, len, child_end)?;
            }
            ID_CHAPTER_ATOM => {
                *atoms_total += 1;
                if *atoms_total > MAX_CHAPTER_ATOMS {
                    return Err(chapter_table_too_large(
                        MAX_CHAPTER_ATOMS as u64,
                        *atoms_total as u64,
                    ));
                }
                match parse_atom(reader, &child, child_end, 1, atoms_total, &mut damage)? {
                    Some(atom) => parsed.atoms.push(atom),
                    None => damage.push((child_end - pos, "atom-without-time-start")),
                }
            }
            _ => {
                // Unknown edition children: skipped by declared size.
            }
        }
        pos = child_end;
    }
    Ok((parsed, damage))
}

/// One `ChapterAtom` parse at `depth` (1 = edition level). `Ok(None)`
/// = degenerate (no `ChapterTimeStart` — skip with damage).
fn parse_atom(
    reader: &mut EbmlReader,
    atom: &ElementHeader,
    end: u64,
    depth: usize,
    atoms_total: &mut usize,
    damage: &mut ChapterDamage,
) -> Result<Option<ChapterAtom>, EngineError> {
    let mut parsed = ChapterAtom {
        enabled: true,
        ..ChapterAtom::default()
    };
    let mut start: Option<u64> = None;
    let mut pos = atom.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child_end > end {
            return Err(error::truncated(child_end, end));
        }
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ID_CHAPTER_UID => {
                if let Some(value) = read_uint(reader, &child, len, child_end)? {
                    parsed.uid = value;
                }
            }
            ID_CHAPTER_TIME_START => {
                start = read_uint(reader, &child, len, child_end)?;
            }
            ID_CHAPTER_TIME_END => {
                parsed.end_ns = read_uint(reader, &child, len, child_end)?;
            }
            ID_CHAPTER_FLAG_HIDDEN => {
                parsed.hidden = read_flag(reader, &child, len, child_end)?;
            }
            ID_CHAPTER_FLAG_ENABLED => {
                parsed.enabled = read_flag(reader, &child, len, child_end)?;
            }
            ID_CHAPTER_TRACK => {
                parse_track_numbers(reader, &child, child_end, &mut parsed)?;
            }
            ID_CHAPTER_DISPLAY => {
                if parsed.displays.len() >= MAX_CHAPTER_DISPLAYS {
                    // Over-ceiling displays: skipped with damage (a
                    // display bomb cannot become an unbounded list).
                    damage.push((child_end - pos, "display-over-ceiling"));
                } else {
                    parsed
                        .displays
                        .push(parse_display(reader, &child, child_end)?);
                }
            }
            ID_CHAPTER_ATOM => {
                // Nested atom: the recursion's depth guard (the
                // card's recursive-nesting error path).
                if depth + 1 > ebml::MAX_PARSE_DEPTH {
                    return Err(error::bad_structure(child.data_pos));
                }
                *atoms_total += 1;
                if *atoms_total > MAX_CHAPTER_ATOMS {
                    return Err(chapter_table_too_large(
                        MAX_CHAPTER_ATOMS as u64,
                        *atoms_total as u64,
                    ));
                }
                match parse_atom(reader, &child, child_end, depth + 1, atoms_total, damage)? {
                    Some(nested) => parsed.children.push(nested),
                    None => damage.push((child_end - pos, "atom-without-time-start")),
                }
            }
            _ => {
                // ChapterSegmentUID / ChapterPhysicalEquiv / …:
                // skipped by declared size.
            }
        }
        pos = child_end;
    }
    // `ChapterTimeStart` is mandatory: an atom without it is
    // degenerate (its subtree goes with it — the sibling atoms
    // survive).
    let Some(start) = start else {
        return Ok(None);
    };
    parsed.start_ns = start;
    Ok(Some(parsed))
}

/// The `ChapterTrack` master's numbers (ordered-edition scoping).
fn parse_track_numbers(
    reader: &mut EbmlReader,
    track: &ElementHeader,
    end: u64,
    parsed: &mut ChapterAtom,
) -> Result<(), EngineError> {
    let mut taken = 0usize;
    let mut pos = track.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child_end > end {
            return Err(error::truncated(child_end, end));
        }
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        if child.id == ID_CHAPTER_TRACK_NUMBER && taken < MAX_CHAPTER_TRACKS && len <= 8 {
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            if let Some(value) = ebml::uint_from_be(&bytes) {
                parsed.track_numbers.push(value);
                taken += 1;
            }
        }
        pos = child_end;
    }
    Ok(())
}

/// One `ChapterDisplay` parse.
fn parse_display(
    reader: &mut EbmlReader,
    display: &ElementHeader,
    end: u64,
) -> Result<ChapterDisplay, EngineError> {
    let mut parsed = ChapterDisplay::default();
    let mut pos = display.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child_end > end {
            return Err(error::truncated(child_end, end));
        }
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        // Every field here is advisory text: tolerant both ways
        // (over-ceiling or invalid UTF-8 → absent, never fatal).
        if len <= MAX_TEXT_FIELD as usize {
            let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
            let text = ebml::utf8_from(&bytes);
            match child.id {
                ID_CHAP_STRING => parsed.string = text,
                ID_CHAP_LANGUAGE => parsed.language = text,
                ID_CHAP_LANGUAGE_IETF => parsed.language_ietf = text,
                ID_CHAP_COUNTRY => parsed.country = text,
                _ => {}
            }
        }
        pos = child_end;
    }
    Ok(parsed)
}

/// A fixed-width uint field (0..=8 bytes by grammar; wider is
/// `bad_structure`; a zero-length element reads as the spec's 0).
fn read_uint(
    reader: &mut EbmlReader,
    child: &ElementHeader,
    len: usize,
    child_end: u64,
) -> Result<Option<u64>, EngineError> {
    if len > 8 {
        return Err(error::bad_structure(child.data_pos));
    }
    let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
    Ok(ebml::uint_from_be(&bytes))
}

/// A flag field (nonzero = set; a zero-length element reads 0).
fn read_flag(
    reader: &mut EbmlReader,
    child: &ElementHeader,
    len: usize,
    child_end: u64,
) -> Result<bool, EngineError> {
    Ok(read_uint(reader, child, len, child_end)?.unwrap_or(0) != 0)
}

/// Over-ceiling chapter table — the typed ResourceLimit refusal
/// (the cue-table precedent's shape; the constructor is local
/// because `error.rs` is card 5.D1's file — this card's grant is the
/// two new modules).
fn chapter_table_too_large(budget: u64, attempted: u64) -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        ErrorContext::new("demux.mkv.chapter-table-too-large"),
    )
}

/// Flag every atom whose nanosecond times extend past
/// `duration_ns` — the tolerant note. Returns the count.
fn flag_beyond_duration(editions: &mut [EditionEntry], duration_ns: u64) -> usize {
    let mut flagged = 0usize;
    for edition in editions {
        for atom in &mut edition.atoms {
            flagged += flag_atom(atom, duration_ns);
        }
    }
    flagged
}

/// One atom's flag walk (recursive over the nested atoms).
fn flag_atom(atom: &mut ChapterAtom, duration_ns: u64) -> usize {
    let mut flagged = 0usize;
    // Chapter times are nanoseconds by spec: a plain comparison, no
    // scale math to overflow (kept, flagged — never an error here).
    let beyond = |ns: u64| ns > duration_ns;
    if beyond(atom.start_ns) || atom.end_ns.is_some_and(beyond) {
        atom.past_duration = true;
        flagged += 1;
    }
    for child in &mut atom.children {
        flagged += flag_atom(child, duration_ns);
    }
    flagged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avi::test_support::mem_stream;
    use testkit::Must;

    /// The on-disk id vint (marker bits intact, minimal width).
    fn id_bytes(id: u32) -> Vec<u8> {
        // The marker bits of the id's first byte fix its width:
        // 0x10..0x1F → 4 bytes, 0x20..0x3F → 3, 0x40..0x7F → 2,
        // 0x80..0xFF → 1 (the u32's leading zeros count them).
        let width = 4 - id.leading_zeros() / 8;
        id.to_be_bytes()[4 - width as usize..].to_vec()
    }

    /// The minimal size vint for `len`.
    fn size_bytes(len: usize) -> Vec<u8> {
        if len < 0x80 {
            vec![0x80 | len as u8]
        } else if len < 0x4000 {
            vec![0x40 | (len >> 8) as u8, (len & 0xFF) as u8]
        } else {
            let bytes = (len as u64).to_be_bytes();
            let mut out = vec![0x01];
            out.extend_from_slice(&bytes[1..]);
            out
        }
    }

    /// One element: id vint + size vint + payload.
    fn el(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = id_bytes(id);
        out.extend(size_bytes(payload.len()));
        out.extend_from_slice(payload);
        out
    }

    /// `Chapters` master bytes over `payload` (the element at 0).
    fn chapters_element(payload: &[u8]) -> Vec<u8> {
        el(ID_CHAPTERS, payload)
    }

    /// A minimal atom: `ChapterTimeStart = start` (+ end when given).
    fn atom(start: u64, end: Option<u64>) -> Vec<u8> {
        let mut payload = el(ID_CHAPTER_TIME_START, &start.to_be_bytes());
        if let Some(end) = end {
            payload.extend(el(ID_CHAPTER_TIME_END, &end.to_be_bytes()));
        }
        el(ID_CHAPTER_ATOM, &payload)
    }

    /// Parse a `Chapters` element at 0 of a fresh in-memory stream.
    fn parse(bytes: &[u8]) -> Result<ChapterList, EngineError> {
        let bound = bytes.len() as u64;
        let mut reader = EbmlReader::new(mem_stream(bytes.to_vec()).must()).must();
        let header = ebml::parse_header_at(&mut reader, 0, bound).must().must();
        parse_chapters(&mut reader, &header, bound)
    }

    #[test]
    fn editions_atoms_and_displays_parse_the_full_record() {
        let mut display = el(ID_CHAP_STRING, b"Intro");
        display.extend(el(ID_CHAP_LANGUAGE, b"eng"));
        display.extend(el(ID_CHAP_LANGUAGE_IETF, b"en-US"));
        display.extend(el(ID_CHAP_COUNTRY, b"US"));
        let mut first_payload = el(ID_CHAPTER_TIME_START, &0u64.to_be_bytes());
        first_payload.extend(el(ID_CHAPTER_TIME_END, &1000u64.to_be_bytes()));
        first_payload.extend(el(ID_CHAPTER_DISPLAY, &display));
        first_payload.extend(el(ID_CHAPTER_TRACK, &el(ID_CHAPTER_TRACK_NUMBER, &[1])));
        let first = el(ID_CHAPTER_ATOM, &first_payload);
        let second = el(
            ID_CHAPTER_ATOM,
            &el(ID_CHAPTER_TIME_START, &1000u64.to_be_bytes()),
        );
        let mut edition = el(ID_EDITION_UID, &[5]);
        edition.extend(el(ID_EDITION_FLAG_ORDERED, &[1]));
        edition.extend(el(ID_EDITION_FLAG_DEFAULT, &[1]));
        edition.extend(first);
        edition.extend(second);
        let bytes = chapters_element(&el(ID_EDITION_ENTRY, &edition));
        let list = parse(&bytes).must();
        assert_eq!(list.editions.len(), 1);
        let got = &list.editions[0];
        assert_eq!(got.uid, 5);
        assert!(got.ordered);
        assert!(got.default, "the EditionFlagDefault parse (issue #34)");
        assert!(!got.hidden);
        assert_eq!(got.atoms.len(), 2);
        assert_eq!(got.atoms[0].start_ns, 0);
        assert_eq!(got.atoms[0].end_ns, Some(1000));
        assert_eq!(got.atoms[0].track_numbers, vec![1]);
        assert_eq!(got.atoms[0].displays.len(), 1);
        let d = &got.atoms[0].displays[0];
        assert_eq!(d.string.as_deref(), Some("Intro"));
        assert_eq!(d.language.as_deref(), Some("eng"));
        assert_eq!(d.language_ietf.as_deref(), Some("en-US"));
        assert_eq!(d.country.as_deref(), Some("US"));
        assert_eq!(got.atoms[0].title(), Some("Intro"));
        // Open-ended second atom + the enabled default.
        assert_eq!(got.atoms[1].end_ns, None);
        assert!(got.atoms[1].enabled);
        assert_eq!(got.atoms[1].title(), None);
        assert!(list.damage.is_empty());
        // The flatten shape (ffprobe's chapter-list order).
        assert_eq!(list.atoms_flattened().len(), 2);
    }

    #[test]
    fn default_edition_selects_flag_then_first_visible_then_first() {
        // The selection rule the seek-target surface uses (issue #34):
        // FlagDefault → first non-hidden → first — the spec's own
        // guidance, the `editions::EditionList` twin's rule.
        let mut list = ChapterList::default();
        assert_eq!(list.default_edition(), None);
        list.editions.push(EditionEntry {
            uid: 9,
            hidden: true,
            ..EditionEntry::default()
        });
        list.editions.push(EditionEntry {
            uid: 1,
            default: true,
            ..EditionEntry::default()
        });
        list.editions.push(EditionEntry {
            uid: 2,
            ..EditionEntry::default()
        });
        assert_eq!(list.default_edition().must().uid, 1);
        list.editions.remove(1);
        assert_eq!(list.default_edition().must().uid, 2, "first visible");
        list.editions.retain(|e| e.hidden);
        assert_eq!(list.default_edition().must().uid, 9, "first, all hidden");
    }

    #[test]
    fn nested_atoms_parse_to_the_depth_guard() {
        // 16 nesting levels parse (MAX_PARSE_DEPTH); the 17th is the
        // typed bad_structure (the recursion's explicit counter).
        let build = |depth: usize| {
            // Every level carries its own TimeStart (an atom without
            // one is degenerate); the chain nests one atom per level.
            let mut payload = el(
                ID_CHAPTER_ATOM,
                &el(ID_CHAPTER_TIME_START, &0u64.to_be_bytes()),
            );
            for _ in 1..depth {
                let mut inner = el(ID_CHAPTER_TIME_START, &0u64.to_be_bytes());
                inner.extend(payload);
                payload = el(ID_CHAPTER_ATOM, &inner);
            }
            chapters_element(&el(ID_EDITION_ENTRY, &payload))
        };
        let ok = parse(&build(ebml::MAX_PARSE_DEPTH)).must();
        assert_eq!(ok.atoms_flattened().len(), ebml::MAX_PARSE_DEPTH);
        let err = match parse(&build(ebml::MAX_PARSE_DEPTH + 1)) {
            Ok(_) => unreachable!("the over-depth nesting level must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.mkv.bad-structure");
    }

    #[test]
    fn atom_table_ceiling_is_the_typed_resource_limit() {
        // MAX_CHAPTER_ATOMS + 1 minimal atoms: the 65_537th is the
        // typed refusal (never an unbounded table).
        let mut payload = Vec::new();
        for i in 0..=MAX_CHAPTER_ATOMS {
            payload.extend(atom(i as u64, None));
        }
        let bytes = chapters_element(&el(ID_EDITION_ENTRY, &payload));
        let err = match parse(&bytes) {
            Ok(_) => unreachable!("this fixture must type-refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.mkv.chapter-table-too-large");
        assert!(matches!(
            err.kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::Memory,
                ..
            }
        ));
    }

    #[test]
    fn atom_without_time_start_is_damage_skipped() {
        let edition = [
            el(ID_CHAPTER_ATOM, &el(ID_CHAPTER_UID, &[9])),
            atom(50, None),
        ]
        .concat();
        let bytes = chapters_element(&el(ID_EDITION_ENTRY, &edition));
        let list = parse(&bytes).must();
        let got = &list.editions[0];
        assert_eq!(got.atoms.len(), 1);
        assert_eq!(got.atoms[0].start_ns, 50);
        assert_eq!(list.damage.len(), 1);
        assert_eq!(list.damage[0].1, "atom-without-time-start");
    }

    #[test]
    fn edition_ceiling_skips_with_damage() {
        let mut payload = Vec::new();
        for _ in 0..=MAX_EDITIONS {
            payload.extend(el(ID_EDITION_ENTRY, &atom(0, None)));
        }
        let bytes = chapters_element(&payload);
        let list = parse(&bytes).must();
        assert_eq!(list.editions.len(), MAX_EDITIONS);
        assert_eq!(list.damage.len(), 1);
        assert_eq!(list.damage[0].1, "edition-over-ceiling");
    }

    #[test]
    fn truncated_atom_field_is_the_typed_truncation() {
        // ChapterTimeStart declares 8 bytes; the stream carries 4.
        let mut edition = id_bytes(ID_CHAPTER_TIME_START);
        edition.extend(size_bytes(8));
        edition.extend_from_slice(&[0, 0, 0, 4]);
        let bytes = chapters_element(&el(ID_EDITION_ENTRY, &el(ID_CHAPTER_ATOM, &edition)));
        let err = match parse(&bytes) {
            Ok(_) => unreachable!("this fixture must type-refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.mkv.truncated");
    }

    #[test]
    fn times_past_the_duration_are_the_tolerant_note() {
        let edition = [atom(0, Some(500)), atom(2_000, Some(9_000))].concat();
        let mut list = parse(&chapters_element(&el(ID_EDITION_ENTRY, &edition))).must();
        // Chapter times are raw nanoseconds (no TimestampScale
        // math): against a 5 µs duration the 0..500 ns atom sits
        // inside; the 2..9 µs atom extends past it. The method is
        // the demuxer's `open()` seam (issue #34) — it records the
        // count on the list itself.
        let flagged = list.flag_beyond_duration(5_000);
        assert_eq!(flagged, 1);
        assert_eq!(list.beyond_duration, 1);
        assert!(!list.editions[0].atoms[0].past_duration);
        assert!(list.editions[0].atoms[1].past_duration);
    }

    #[test]
    fn extreme_times_land_in_the_tolerant_bucket() {
        let edition = [atom(u64::MAX, None)].concat();
        let mut list = parse(&chapters_element(&el(ID_EDITION_ENTRY, &edition))).must();
        // The raw ns value parses as-is (no scale multiplication to
        // overflow): kept, flagged against any finite duration —
        // never an error.
        let flagged = list.flag_beyond_duration(u64::MAX - 1);
        assert_eq!(flagged, 1);
        assert_eq!(list.beyond_duration, 1);
        assert!(list.editions[0].atoms[0].past_duration);
        assert_eq!(list.editions[0].atoms[0].start_ns, u64::MAX);
    }

    #[test]
    fn unknown_size_inside_chapters_is_bad_structure() {
        let mut payload = id_bytes(ID_EDITION_ENTRY);
        payload.push(0xFF);
        payload.extend(atom(0, None));
        let bytes = chapters_element(&payload);
        let err = match parse(&bytes) {
            Ok(_) => unreachable!("this fixture must type-refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.mkv.bad-structure");
    }

    #[test]
    fn empty_chapters_element_is_an_empty_list() {
        let bytes = chapters_element(&[]);
        let list = parse(&bytes).must();
        assert!(list.editions.is_empty());
        assert!(list.damage.is_empty());
    }
}
