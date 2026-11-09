//! Chapter navigation — next/previous/current lookup over the wire
//! chapter list (issue #32, `media.get-chapters` / `media.set-chapter`;
//! doc 05 §2.10).
//!
//! # What this module is
//!
//! The **pure resolution half** of chapter navigation. The v0 engine
//! has no in-session chapter source ([`crate::seek`]'s
//! `SeekTarget::Chapter` refuses typed and hosts resolve chapters to
//! `Absolute` — the module docs there say exactly this), so this
//! module owns that resolution as testable data functions over the
//! [`commands`] wire [`Chapter`] values a host already holds from
//! `media.get-chapters` (MKV editions/entries and MP4 chapter traks
//! both surface through it; crates/demux's `ChapterList` is the
//! upstream source).
//!
//! # Semantics
//!
//! - **`chapter_at`**: the 0-based index of the chapter containing a
//!   position (a position before the first chapter start is in no
//!   chapter; positions at/after a start belong to that chapter).
//! - **`next_chapter`**: the first chapter starting strictly after
//!   the position — `None` at/after the last start.
//! - **`previous_chapter`**: **restart-then-rewind** — when more than
//!   [`CHAPTER_RESTART_THRESHOLD_NS`] has elapsed inside the current
//!   chapter, the step restarts the *current* chapter; only a
//!   near-start position rewinds into the previous one.
//!   **[PROPOSAL: VLC's bare prev-chapter always rewinds; the
//!   restart-then-rewind shape follows the jump-prev convention
//!   (keymaps' `jump-*` notes) and the 10 s threshold is a picked
//!   default — Lead call documented here.]**
//! - **`chapter_target`**: the absolute nanosecond [`SeekTarget`] for
//!   an index (µs wire → ns engine), for the existing seek flow.
//!
//! # Untrusted inputs
//!
//! Chapter titles are untrusted text (V7) and are never touched
//! here; only the numeric starts/lengths are read. Chapter lists
//! are not assumed sorted — lookups scan for containment, and
//! `chapter_target` clamps nothing (an out-of-range index is a typed
//! `None`, matching the demo transport's not-found refusal).

use crate::seek::SeekTarget;
use commands::catalog::types::Chapter;

/// How far into a chapter a prev-chapter step still restarts the
/// current chapter instead of rewinding into the previous one.
/// **[PROPOSAL]** — see module docs.
pub const CHAPTER_RESTART_THRESHOLD_NS: i64 = 10 * 1_000_000_000;

/// Wire `Timestamp` is µs; the engine seeks in ns.
const US_TO_NS: i64 = 1_000;

/// The 0-based index of the chapter containing `position_ns`, if any
/// (chapters are matched by start ≤ position < start + length when
/// the length is known, else start ≤ position).
#[must_use]
pub fn chapter_at(chapters: &[Chapter], position_ns: i64) -> Option<usize> {
    let position_us = position_ns / US_TO_NS;
    chapters
        .iter()
        .position(|chapter| chapter_contains(chapter, position_us))
}

/// The index of the first chapter starting strictly after
/// `position_ns` — the next-chapter step target. `None` when the
/// position is at/after the last chapter start.
#[must_use]
pub fn next_chapter(chapters: &[Chapter], position_ns: i64) -> Option<u32> {
    let position_us = position_ns / US_TO_NS;
    chapters
        .iter()
        .find(|chapter| (chapter.time_start.0) > position_us)
        .map(|chapter| chapter.index)
}

/// The prev-chapter step target: the current chapter's index when
/// enough of it has elapsed (restart-then-rewind), else the previous
/// chapter's, else `None` (before the first chapter start there is
/// nothing to rewind to — the honest typed answer, clamping to 0 is
/// the seek flow's job, not this lookup's).
#[must_use]
pub fn previous_chapter(chapters: &[Chapter], position_ns: i64) -> Option<u32> {
    let position_us = position_ns / US_TO_NS;
    // The chapter containing the position, or the last chapter that
    // started before it (position between chapters/skip gaps).
    let current = chapters
        .iter()
        .filter(|chapter| (chapter.time_start.0) <= position_us)
        .next_back()?;
    let elapsed_ns = (position_us - current.time_start.0) * US_TO_NS;
    if elapsed_ns > CHAPTER_RESTART_THRESHOLD_NS {
        return Some(current.index);
    }
    // Near the start (or before every chapter): rewind one.
    let index = current.index;
    if index == 0 {
        return None;
    }
    chapters
        .iter()
        .find(|chapter| chapter.index + 1 == index)
        .map(|chapter| chapter.index)
}

/// The absolute seek target for a chapter index (µs wire → ns).
/// `None` when no chapter carries that index — the typed refusal.
#[must_use]
pub fn chapter_target(chapters: &[Chapter], index: u32) -> Option<SeekTarget> {
    chapters
        .iter()
        .find(|chapter| chapter.index == index)
        .map(|chapter| SeekTarget::Absolute {
            position_ns: chapter.time_start.0 * US_TO_NS,
        })
}

/// Containment test on wire µs values (a zero/absent length means
/// open-ended — start ≤ position).
fn chapter_contains(chapter: &Chapter, position_us: i64) -> bool {
    let start = chapter.time_start.0;
    if position_us < start {
        return false;
    }
    match chapter.duration {
        commands::catalog::types::Duration::Known { time } if (time.0) > 0 => {
            position_us < start + (time.0)
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::catalog::types::{Duration, Timestamp};

    /// Three chapters: 0–10 s, 10–20 s, 20–30 s (wire µs).
    fn fixtures() -> Vec<Chapter> {
        [
            (0u32, 0i64, 10_000_000i64, "Intro"),
            (1, 10_000_000, 10_000_000, "Middle"),
            (2, 20_000_000, 10_000_000, "Outro"),
        ]
        .into_iter()
        .map(|(index, start, len, title)| Chapter {
            index,
            time_start: Timestamp(start),
            duration: Duration::Known {
                time: Timestamp(len),
            },
            title: Some(title.to_string()),
        })
        .collect()
    }

    fn secs(n: i64) -> i64 {
        n * 1_000_000_000
    }

    #[test]
    fn chapter_at_maps_containment() {
        let chapters = fixtures();
        assert_eq!(chapter_at(&chapters, secs(5)), Some(0));
        assert_eq!(chapter_at(&chapters, secs(15)), Some(1));
        assert_eq!(chapter_at(&chapters, secs(25)), Some(2));
        // Before the first start: in no chapter.
        assert_eq!(chapter_at(&chapters, secs(-1)), None);
    }

    #[test]
    fn next_chapter_finds_strictly_later_start() {
        let chapters = fixtures();
        assert_eq!(next_chapter(&chapters, secs(5)), Some(1));
        assert_eq!(next_chapter(&chapters, secs(10)), Some(2));
        // At/after the last start: nothing next.
        assert_eq!(next_chapter(&chapters, secs(20)), None);
        assert_eq!(next_chapter(&chapters, secs(29)), None);
    }

    #[test]
    fn previous_chapter_restarts_when_deep_in_chapter() {
        let chapters = fixtures();
        // 11 s into chapter 2 (past the 10 s threshold): restart it.
        assert_eq!(previous_chapter(&chapters, secs(31)), Some(2));
        // 2 s elapsed in chapter 1: rewind into chapter 0.
        assert_eq!(previous_chapter(&chapters, secs(12)), Some(1 - 1));
    }

    #[test]
    fn previous_chapter_before_first_start_is_none() {
        let chapters = fixtures();
        assert_eq!(previous_chapter(&chapters, secs(1)), None);
        assert_eq!(previous_chapter(&chapters, secs(-5)), None);
    }

    #[test]
    fn previous_chapter_early_in_first_chapter_is_none() {
        let chapters = fixtures();
        // 1 s into chapter 0: would rewind past the list.
        assert_eq!(previous_chapter(&chapters, secs(1)), None);
        // Deep into chapter 0: restart chapter 0 itself.
        assert_eq!(previous_chapter(&chapters, secs(11)), Some(0));
    }

    #[test]
    fn chapter_target_resolves_to_absolute_ns() {
        let chapters = fixtures();
        assert_eq!(
            chapter_target(&chapters, 1),
            Some(SeekTarget::Absolute {
                position_ns: secs(10)
            })
        );
        assert_eq!(chapter_target(&chapters, 7), None);
    }

    #[test]
    fn empty_list_is_all_none() {
        assert_eq!(chapter_at(&[], secs(1)), None);
        assert_eq!(next_chapter(&[], secs(1)), None);
        assert_eq!(previous_chapter(&[], secs(1)), None);
        assert_eq!(chapter_target(&[], 0), None);
    }

    #[test]
    fn open_ended_chapter_contains_from_its_start() {
        let open = Chapter {
            index: 0,
            time_start: Timestamp(5_000_000),
            duration: Duration::Unknown,
            title: None,
        };
        assert_eq!(chapter_at(&[open.clone()], secs(6)), Some(0));
        assert_eq!(chapter_at(&[open], secs(4)), None);
    }
}
