//! Bookmarks — per-media bookmark list: create / jump / delete (issue #32,
//! `media.get-bookmarks` / `media.add-bookmark` / `media.remove-bookmark`).
//!
//! # What this module is
//!
//! The **pure state half** of the bookmark feature, mirroring how the v0
//! engine owns chapters ([`crate::chapters`]): the wire commands above are
//! carried by the [`commands`] catalog, and this module is the testable
//! list semantics a host (or a future in-engine session) drives them with.
//! No pipeline invalidation is involved — bookmarks are session-scoped
//! annotations, hence the class-S, non-gen-bumping classification.
//!
//! # Semantics
//!
//! - **ids**: creation-ordered from 1, unique, never reused after a
//!   delete (a delete of id 3 does not renumber 4 → 3). 1-based to match
//!   the keymap's bookmark slots (keymaps/default.toml `set-bookmark-N`,
//!   N = 1..10).
//! - **create**: any position ≥ 0, any label (untrusted text, V7 — never
//!   interpreted). A negative position is refused typed
//!   ([`BookmarkList::add`] returns `None`) — a bookmark is a position,
//!   not a delta.
//! - **jump**: [`BookmarkList::seek_target`] resolves a bookmark to the
//!   existing absolute [`SeekTarget`] (µs wire → ns engine) — the same
//!   flow chapter jumps use; there is no separate jump command.
//! - **per-media lifetime**: [`BookmarkList::clear`] resets list AND id
//!   counter (a new media restarts numbering at 1, VLC parity). The
//!   session owner calls it on `session.open`; this module only owns the
//!   data shape.
//! - **order**: insertion order, not time order — the list shows what
//!   the user created, in the order they created it (VLC's dialog does
//!   the same); time-sorting is a view concern.

use crate::seek::SeekTarget;
use commands::catalog::types::Bookmark;

/// Wire `Timestamp` is µs; the engine seeks in ns.
const US_TO_NS: i64 = 1_000;

/// The per-media bookmark list (issue #32). Pure state — no clocks, no
/// pipeline; see the module docs for the semantics contract.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BookmarkList {
    entries: Vec<Bookmark>,
    next_id: u32,
}

impl BookmarkList {
    /// An empty list; ids start at 1 on the first add.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// Every bookmark, insertion order.
    #[must_use]
    pub fn entries(&self) -> &[Bookmark] {
        &self.entries
    }

    /// Create a bookmark at `time_us` (µs from media start). Refuses a
    /// negative position typed (`None`); the assigned id is
    /// creation-ordered and never reused.
    pub fn add(&mut self, time_us: i64, label: Option<String>) -> Option<Bookmark> {
        if time_us < 0 {
            return None;
        }
        let bookmark = Bookmark {
            id: self.next_id,
            time: commands::catalog::types::Timestamp(time_us),
            label,
        };
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("bookmark id space exhausted (u32)");
        self.entries.push(bookmark.clone());
        Some(bookmark)
    }

    /// Delete the bookmark with `id`. `true` when it existed; ids of
    /// later bookmarks are untouched (no renumbering).
    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.entries.len();
        self.entries.retain(|bookmark| bookmark.id != id);
        before != self.entries.len()
    }

    /// The jump target for a bookmark id — the existing absolute seek
    /// flow (µs wire → ns engine). `None` when no bookmark carries that
    /// id: the typed refusal, never a guessed neighbor.
    #[must_use]
    pub fn seek_target(&self, id: u32) -> Option<SeekTarget> {
        self.entries
            .iter()
            .find(|bookmark| bookmark.id == id)
            .map(|bookmark| SeekTarget::Absolute {
                position_ns: (bookmark.time.0) * US_TO_NS,
            })
    }

    /// Reset for a new media: empties the list and restarts id numbering
    /// at 1 (the session owner calls this on `session.open`).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn us(secs: i64) -> i64 {
        secs * 1_000_000
    }

    #[test]
    fn add_assigns_creation_ordered_ids_from_one() {
        let mut list = BookmarkList::new();
        let a = list.add(us(10), Some("Intro".to_string())).unwrap();
        let b = list.add(us(65), None).unwrap();
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
        assert_eq!(
            list.entries(),
            [
                Bookmark {
                    id: 1,
                    time: commands::catalog::types::Timestamp(us(10)),
                    label: Some("Intro".to_string()),
                },
                Bookmark {
                    id: 2,
                    time: commands::catalog::types::Timestamp(us(65)),
                    label: None,
                },
            ]
        );
    }

    #[test]
    fn negative_position_is_refused_typed() {
        let mut list = BookmarkList::new();
        assert!(list.add(-1, None).is_none());
        assert!(list.entries().is_empty());
        // A zero position is the media start — valid.
        assert!(list.add(0, None).is_some());
    }

    #[test]
    fn remove_deletes_without_renumbering() {
        let mut list = BookmarkList::new();
        list.add(us(10), None).unwrap();
        list.add(us(20), None).unwrap();
        list.add(us(30), None).unwrap();
        assert!(list.remove(2));
        assert!(!list.remove(2), "second remove of the same id is false");
        assert_eq!(list.entries().iter().map(|b| b.id).collect::<Vec<_>>(), [1, 3]);
        // The next add does NOT reuse the freed id.
        let next = list.add(us(40), None).unwrap();
        assert_eq!(next.id, 4);
    }

    #[test]
    fn seek_target_resolves_us_to_ns_absolute() {
        let mut list = BookmarkList::new();
        list.add(us(90), Some("Chorus".to_string())).unwrap();
        assert_eq!(
            list.seek_target(1),
            Some(SeekTarget::Absolute {
                position_ns: us(90) * 1_000
            })
        );
        assert_eq!(list.seek_target(7), None);
    }

    #[test]
    fn clear_resets_list_and_numbering() {
        let mut list = BookmarkList::new();
        list.add(us(10), None).unwrap();
        list.add(us(20), None).unwrap();
        list.clear();
        assert!(list.entries().is_empty());
        let next = list.add(us(5), None).unwrap();
        assert_eq!(next.id, 1, "numbering restarts for the new media");
    }

    #[test]
    fn duplicate_positions_are_distinct_bookmarks() {
        let mut list = BookmarkList::new();
        let a = list.add(us(10), Some("A".to_string())).unwrap();
        let b = list.add(us(10), Some("B".to_string())).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(list.entries().len(), 2);
    }
}
