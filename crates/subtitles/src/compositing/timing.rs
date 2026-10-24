//! Cue timing: the half-open `[t0, t1)` active-interval rule,
//! evaluated on the shared clock's nanosecond positions.
//!
//! # Why half-open
//!
//! A cue ending at `t` and the next starting at `t` must never both
//! display (double-draw at the seam) nor neither (one-frame gap):
//! `start <= now < end` gives exactly that. This is the subtitle
//! convention (WebVTT/SRT intervals are end-exclusive at the swap
//! instant) and matches the video scheduler's frame-presentation
//! comparisons.
//!
//! # Why integer nanoseconds
//!
//! The shared clock (`media-clock`, doc 03 §4.7) positions are
//! nanosecond-`Timestamp`s derived from the audio sample counter —
//! sample-exact by construction. Cue spans convert to the same base
//! ONCE at [`crate::compositing::CompositedCue`] build; the per-frame
//! check below is two integer compares — no float, no conversion, no
//! allocation.
//!
//! # Missing ends
//!
//! `None` start = live from the beginning; `None` end = open-ended
//! (live cues). Both missing = active: the decode handed us the cue
//! with no extent data — hiding it would fabricate a duration (MP §8
//! is the reverse direction too: never invent an end).

/// Is a cue with span `[start, end)` (nanoseconds, either end open)
/// active at `now_ns`?
#[must_use = "the compositor draws exactly the active set — skipping the check draws everything"]
pub fn active_at(start: Option<i64>, end: Option<i64>, now_ns: i64) -> bool {
    if let Some(start) = start
        && now_ns < start
    {
        return false;
    }
    if let Some(end) = end
        && now_ns >= end
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_000_000_000;
    const T1: i64 = 2_000_000_000;

    #[test]
    fn interval_is_half_open_to_the_nanosecond() {
        assert!(!active_at(Some(T0), Some(T1), T0 - 1));
        assert!(active_at(Some(T0), Some(T1), T0));
        assert!(active_at(Some(T0), Some(T1), T0 + 1));
        assert!(active_at(Some(T0), Some(T1), T1 - 1));
        assert!(!active_at(Some(T0), Some(T1), T1));
        assert!(!active_at(Some(T0), Some(T1), T1 + 1));
    }

    #[test]
    fn adjacent_cues_never_double_draw_nor_gap() {
        // [T0, T1) and [T1, T1+1s): at exactly T1 the first is gone
        // and the second is on — the seam property.
        assert!(!active_at(Some(T0), Some(T1), T1));
        assert!(active_at(Some(T1), Some(T1 + 1_000_000_000), T1));
    }

    #[test]
    fn zero_duration_cue_is_never_active() {
        assert!(!active_at(Some(T0), Some(T0), T0));
        assert!(!active_at(Some(T0), Some(T0), T0 - 1));
    }

    #[test]
    fn open_ended_cues_stay_active() {
        assert!(active_at(None, None, i64::MIN));
        assert!(active_at(None, None, 0));
        assert!(active_at(None, None, i64::MAX));
        assert!(active_at(Some(T0), None, T0));
        assert!(!active_at(Some(T0), None, T0 - 1));
        assert!(active_at(None, Some(T1), T1 - 1));
        assert!(!active_at(None, Some(T1), T1));
    }
}
