//! Matroska `Chapters` → **editions** parsing (card 5.D2b — the
//! structure half of the 5.D2 EXTRAS metadata).
//!
//! An *edition* is one alternate chaptering of a segment: an
//! `EditionEntry` inside the `Chapters` master carrying the UID the
//! `Tags` zone targets ([`crate::mkv::tags`]) plus the three flags —
//! `EditionFlagDefault` (the edition a player opens with),
//! `EditionFlagHidden` (not user-selectable) and
//! `EditionFlagOrdered` (the atoms are an *ordered* set: time-ordered
//! parts to play as a sequence, not just markers — the edition is a
//! playlist whose ordering is significant and whose atoms may nest).
//!
//! # The D2a seam (recorded honestly)
//!
//! The **full `ChapterAtom` parse — `ChapterTimeStart`/`ChapterTimeEnd`,
//! `ChapterDisplay` titles, languages, tracks — is the parallel card
//! 5.D2a's owned surface** (`chapters.rs`, same wave, file-disjoint).
//! This module parses the *edition structure*: the flags, the UID,
//! and the **edition→chapter-UID links** (every `ChapterUID` inside
//! the edition's atoms, nested included, in file order) — the join
//! key the tags zone and any edition-aware UI need. When D2a's
//! `ChapterAtom` parse lands, the merge point is
//! [`parse_editions`]' atom walk: replace [`collect_atom_uids`] with
//! the structured parse and keep `Edition`'s UID list as the link
//! seam. Until then the atoms' non-UID content is skipped by
//! declared size (the 5.D1 tolerance) — nothing fabricated.
//!
//! # Robustness model (the mkv zone's rules)
//!
//! - Unknown children are skipped by declared size, never fatal.
//! - The atom descent carries an explicit depth counter refused past
//!   [`MAX_ATOM_NESTING`] (the spec's shape is 2 — atom → nested
//!   atom for ordered sub-parts).
//! - Memory is bounded (MP §12): edition count and atom count carry
//!   the typed `ResourceLimit` refusal — the corpus's own
//!   chapter-bomb (0007) carries **2580** atoms in one edition; the
//!   ceiling sits an order above that.

use media_types::error::{ErrorContext, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

use crate::mkv::ebml::{self, EbmlReader, ElementHeader};
use crate::mkv::error;

// ---- Element IDs ------------------------------------------------------

/// `Chapters` — the top-level chapters/editions master.
pub const ID_CHAPTERS: u32 = 0x1043_A770;
/// `EditionEntry` — one alternate chaptering.
pub const ID_EDITION_ENTRY: u32 = 0x45B9;
/// `EditionUID` — the identifier `Tags` targets (`TagEditionUID`).
pub const ID_EDITION_UID: u32 = 0x45BC;
/// `EditionFlagHidden` — 1 = not user-selectable (spec default 0).
pub const ID_EDITION_FLAG_HIDDEN: u32 = 0x45BD;
/// `EditionFlagDefault` — 1 = the edition to open with (default 0).
pub const ID_EDITION_FLAG_DEFAULT: u32 = 0x45DB;
/// `EditionFlagOrdered` — 1 = the atoms are an ordered set (default 0).
pub const ID_EDITION_FLAG_ORDERED: u32 = 0x45DD;
/// `ChapterAtom` — one chapter (5.D2a's owned parse; the UID walk
/// here is the link seam).
pub const ID_CHAPTER_ATOM: u32 = 0xB6;
/// `ChapterUID` — the chapter identifier `Tags` targets
/// (`TagChapterUID`).
pub const ID_CHAPTER_UID: u32 = 0x73C4;

// ---- Ceilings (MP §12, the chapter-bomb guards) -----------------------

/// Maximum editions per `Chapters` element
/// `[PROPOSAL: 1024 — real files carry 1–2; alternate-cuts releases
/// stay under a dozen]`.
pub const MAX_EDITIONS: usize = 1024;

/// Maximum chapter atoms per `Chapters` element, nested included
/// `[PROPOSAL: 65_536 — the corpus chapter-bomb (0007) carries 2580
/// in one edition; 25× headroom, matching the cues ceiling's shape]`.
pub const MAX_CHAPTER_ATOMS: usize = 65_536;

/// Maximum `ChapterAtom` nesting depth
/// `[PROPOSAL: 8 — the spec's shape is 2 (atom → nested atom); the
/// guard, not the number, is the point]`.
pub const MAX_ATOM_NESTING: usize = 8;

// ---- Local typed error (file-disjoint seam) ----------------------------
//
// `mkv/error.rs` is 5.D1's file (D2a owns its own additions); this
// zone mints its one editions-specific constructor locally — same
// taxonomy, stable dotted code.

/// The edition structure exceeded a memory ceiling (`ResourceLimit`,
/// stage `Demux`) — the chapter-bomb refusal.
fn edition_budget(budget: u64, attempted: u64) -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        ErrorContext::new("demux.mkv.editions.budget"),
    )
}

// ---- Model ------------------------------------------------------------

/// One `EditionEntry`: the chaptering's identity and selection
/// semantics, plus its chapter-UID links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edition {
    /// `EditionUID` — the join key for `TagEditionUID` targeting (0
    /// when the file declares none — the spec default; still a valid
    /// edition, just untargetable).
    pub uid: u64,
    /// `EditionFlagHidden` — present but not user-selectable.
    pub hidden: bool,
    /// `EditionFlagDefault` — the edition a player opens with.
    pub default: bool,
    /// `EditionFlagOrdered` — the atoms are an ordered set (a
    /// playlist of parts, not a marker list); nested atoms carry the
    /// sub-part semantics in this form.
    pub ordered: bool,
    /// How many `ChapterAtom` records the edition carries (top-level
    /// and nested).
    pub atom_count: usize,
    /// Every `ChapterUID` inside the edition's atoms, in file order
    /// — the edition→chapter link seam (the full atom parse is
    /// 5.D2a's; this records the join keys only).
    pub chapter_uids: Vec<u64>,
}

/// The parsed `Chapters` element (the segment's editions).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditionList {
    /// The editions in file order.
    pub editions: Vec<Edition>,
}

impl EditionList {
    /// The edition a player opens with: the `FlagDefault` one, else
    /// the first non-hidden, else the first (recorded honestly — a
    /// file with no default flag still chapters, the choice falls
    /// back by the spec's own selection guidance).
    #[must_use]
    pub fn default_edition(&self) -> Option<&Edition> {
        self.editions
            .iter()
            .find(|e| e.default)
            .or_else(|| self.editions.iter().find(|e| !e.hidden))
            .or_else(|| self.editions.first())
    }

    /// The editions carrying `EditionFlagOrdered`.
    #[must_use]
    pub fn ordered_editions(&self) -> Vec<&Edition> {
        self.editions.iter().filter(|e| e.ordered).collect()
    }

    /// The edition carrying `uid`, when targeted.
    #[must_use]
    pub fn by_uid(&self, uid: u64) -> Option<&Edition> {
        self.editions.iter().find(|e| e.uid == uid)
    }
}

// ---- Parse ------------------------------------------------------------

/// Parse one top-level `Chapters` element into an [`EditionList`].
///
/// # Errors
/// [`error::bad_structure`] for unknown-size children or over-deep
/// atom nesting; the typed `ResourceLimit`
/// (`demux.mkv.editions.budget`) when an edition/atom ceiling is
/// exceeded; reader truncation errors propagate typed.
pub fn parse_editions(
    reader: &mut EbmlReader,
    chapters: &ElementHeader,
    bound: u64,
) -> Result<EditionList, EngineError> {
    let end = chapters.declared_end().unwrap_or(bound).min(bound);
    let mut list = EditionList::default();
    let mut atoms_total = 0usize;
    let mut pos = chapters.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            // Unknown-size inside Chapters: not the streaming form's
            // scope.
            return Err(error::bad_structure(child.data_pos));
        };
        if child.id == ID_EDITION_ENTRY {
            if list.editions.len() >= MAX_EDITIONS {
                return Err(edition_budget(MAX_EDITIONS as u64, MAX_EDITIONS as u64 + 1));
            }
            if let Some(edition) = parse_edition(reader, &child, child_end, &mut atoms_total)? {
                list.editions.push(edition);
            }
        }
        // Unknown children (CRC-32, Void, future elements): skipped
        // by declared size.
        pos = child_end;
    }
    Ok(list)
}

/// Parse one `EditionEntry` (flags + UID + the atom-UID links).
/// `Ok(None)` = a degenerate entry (no atoms at all) — skipped, the
/// other editions survive.
fn parse_edition(
    reader: &mut EbmlReader,
    edition: &ElementHeader,
    end: u64,
    atoms_total: &mut usize,
) -> Result<Option<Edition>, EngineError> {
    let mut out = Edition {
        uid: 0,
        hidden: false,
        default: false,
        ordered: false,
        atom_count: 0,
        chapter_uids: Vec::new(),
    };
    let mut pos = edition.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ID_EDITION_UID
            | ID_EDITION_FLAG_HIDDEN
            | ID_EDITION_FLAG_DEFAULT
            | ID_EDITION_FLAG_ORDERED => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                let value = ebml::uint_from_be(&bytes).unwrap_or(0);
                match child.id {
                    ID_EDITION_UID => out.uid = value,
                    ID_EDITION_FLAG_HIDDEN => out.hidden = value != 0,
                    ID_EDITION_FLAG_DEFAULT => out.default = value != 0,
                    _ => out.ordered = value != 0,
                }
            }
            ID_CHAPTER_ATOM => {
                if *atoms_total >= MAX_CHAPTER_ATOMS {
                    return Err(edition_budget(
                        MAX_CHAPTER_ATOMS as u64,
                        MAX_CHAPTER_ATOMS as u64 + 1,
                    ));
                }
                *atoms_total += 1;
                out.atom_count += 1;
                // The D2a seam: the atom's non-UID content is D2a's
                // owned parse — this walk records only the link keys
                // (and counts the nested atoms it descends into).
                collect_atom_uids(reader, &child, child_end, 0, &mut out, atoms_total)?;
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok((out.atom_count > 0).then_some(out))
}

/// Walk one `ChapterAtom` recording every `ChapterUID` (its own plus
/// its nested atoms'), depth-guarded. This is the **link seam**: the
/// full atom parse (times, displays, tracks) is card 5.D2a's
/// file-disjoint surface; replacing this walk with the structured
/// parse at merge is the recorded integration point.
fn collect_atom_uids(
    reader: &mut EbmlReader,
    atom: &ElementHeader,
    end: u64,
    depth: usize,
    edition: &mut Edition,
    atoms_total: &mut usize,
) -> Result<(), EngineError> {
    if depth >= MAX_ATOM_NESTING {
        // The nesting guard: the spec's shape is atom → nested atom;
        // deeper declared nesting is corruption, typed.
        return Err(error::bad_structure(atom.data_pos));
    }
    let mut pos = atom.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ID_CHAPTER_UID => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                let uid = ebml::uint_from_be(&bytes).unwrap_or(0);
                // A zero UID is degenerate (the spec mints nonzero
                // values); it is still recorded — the link is the
                // file's own claim, never fabricated, never dropped.
                edition.chapter_uids.push(uid);
            }
            ID_CHAPTER_ATOM => {
                if edition.chapter_uids.len() >= MAX_CHAPTER_ATOMS {
                    return Err(edition_budget(
                        MAX_CHAPTER_ATOMS as u64,
                        MAX_CHAPTER_ATOMS as u64 + 1,
                    ));
                }
                *atoms_total += 1;
                edition.atom_count += 1;
                collect_atom_uids(reader, &child, child_end, depth + 1, edition, atoms_total)?;
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn default_edition_selects_flag_then_first_visible() {
        let mut list = EditionList::default();
        assert_eq!(list.default_edition(), None);
        let hidden = Edition {
            uid: 9,
            hidden: true,
            default: false,
            ordered: false,
            atom_count: 1,
            chapter_uids: vec![9],
        };
        let flagged = Edition {
            uid: 1,
            hidden: false,
            default: true,
            ordered: true,
            atom_count: 2,
            chapter_uids: vec![1, 2],
        };
        let plain = Edition {
            uid: 2,
            hidden: false,
            default: false,
            ordered: false,
            atom_count: 1,
            chapter_uids: vec![3],
        };
        list.editions.push(hidden.clone());
        list.editions.push(flagged.clone());
        list.editions.push(plain.clone());
        assert_eq!(list.default_edition().must().uid, 1);
        assert_eq!(list.ordered_editions().len(), 1);
        assert_eq!(list.by_uid(2).must().uid, 2);
        assert_eq!(list.by_uid(99), None);
        // Without the flag, the first visible edition wins.
        list.editions.remove(1);
        assert_eq!(list.default_edition().must().uid, 2);
        // Without any visible edition, the first wins.
        list.editions.retain(|e| e.hidden);
        assert_eq!(list.default_edition().must().uid, 9);
    }

    #[test]
    fn budget_error_is_typed_resource_limit() {
        let err = edition_budget(100, 200);
        assert_eq!(err.stage, PipelineStage::Demux);
        assert_eq!(err.context.code, "demux.mkv.editions.budget");
        assert_eq!(
            err.kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::Memory,
                budget: 100,
                attempted: 200
            }
        );
    }
}
