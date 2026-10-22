#!/usr/bin/env python3
"""Final pipeline.rs regions (Lead, part 6b — corrected old-strings)."""

from pathlib import Path

P = Path("/home/z/my-project/crates/playback-engine/src/pipeline.rs")
t = P.read_text()

def rep(old, new, tag):
    global t
    assert t.count(old) == 1, f"{tag}: {t.count(old)}"
    t = t.replace(old, new)

# @1894: HEAD empty; theirs = pacing primed + buffer_fill comment.
# The primed assignment is upgraded separately; keep only the comment.
rep("""<<<<<<< HEAD
=======
            // Priming waits for every pacing limb's high watermark
            // (subtitles and switch-installed limbs still recovering
            // are exempt — see [`Pipeline::paces`]).
            report.primed = !self.limbs.is_empty()
                && self
                    .limbs
                    .iter()
                    .filter(|limb| self.paces(limb))
                    .all(|limb| limb.queue.watermark().high);
            // The in-episode progress: the bottleneck pacing limb's
            // fill toward its high watermark (card 3.B4 — doc 05
            // §3.2's `0.0..0.1` while buffering). The exit bookend
            // `1.0` is the table effect; this is the honest
            // mid-episode view.
>>>>>>> wip/w2-12-track-switching
""", """            // The in-episode progress: the bottleneck pacing limb's
            // fill toward its high watermark (card 3.B4 — doc 05
            // §3.2's `0.0..1.0` while buffering). The exit bookend
            // `1.0` is the table effect; this is the honest
            // mid-episode view.
""", "R-1894")

# Upgrade the priming block to pacing-filtered
rep("""        if matches!(state, SessionState::Buffering | SessionState::Paused) {
            let mut primed = !self.limbs.is_empty();
            for limb in self.limbs.iter_mut() {
                if limb.queue.watermark().high {
                    // Priming is a full recovery: any starvation
                    // episode closes here, so the next dip is a fresh
                    // stall (1-tick latency, 3.B3 design B2/F1.5).
                    limb.episode_open = false;
                } else {
                    primed = false;
                }
            }
            report.primed = primed;
        }""", """        if matches!(state, SessionState::Buffering | SessionState::Paused) {
            // Priming waits for every PACING limb's high watermark
            // (subtitles and switch-installed limbs still recovering
            // are exempt — see [`Pipeline::paces`]).
            let mut primed = !self.limbs.is_empty();
            for limb in self.limbs.iter_mut() {
                if !self.paces(limb) {
                    continue;
                }
                if limb.queue.watermark().high {
                    // Priming is a full recovery: any starvation
                    // episode closes here, so the next dip is a fresh
                    // stall (1-tick latency, 3.B3 design B2/F1.5).
                    limb.episode_open = false;
                } else {
                    primed = false;
                }
            }
            report.primed = primed;
        }""", "priming-upgrade")

# @1923: keep HEAD's full episode machinery + add the paces() filter
rep("""<<<<<<< HEAD
            // (rebuffering at the end would be a lie). The re-entry
            // debounce (module docs) filters low-line flapping
            // without delaying a fresh episode's first stall.
            let mut starved = false;
            let mut any_low = false;
            for limb in self.limbs.iter_mut() {
                let mark = limb.queue.watermark();
                if mark.high {
                    // A full recovery closes the episode even without
                    // a rebuffer cycle in between.
                    limb.episode_open = false;
                }
                if mark.low {
                    any_low = true;
                    limb.low_streak = limb.low_streak.saturating_add(1);
                    if !limb.episode_open || limb.low_streak >= self.starve_debounce {
                        limb.episode_open = true;
                        starved = true;
                    }
                } else {
                    limb.low_streak = 0;
                }
            }
            if any_low {
                self.starvation.starved_ticks = self.starvation.starved_ticks.saturating_add(1);
            }
            report.starved = !self.eof && starved;
            if report.starved {
                self.starvation.starve_signals = self.starvation.starve_signals.saturating_add(1);
            }
=======
            // (rebuffering at the end would be a lie), and a
            // mid-switch re-prime is a selection change, not a
            // starvation (the carve-out).
            report.starved = !self.eof
                && self
                    .limbs
                    .iter()
                    .filter(|limb| self.paces(limb))
                    .any(|limb| limb.queue.watermark().low);
>>>>>>> wip/w2-12-track-switching
""", """            // (rebuffering at the end would be a lie), and a
            // mid-switch re-prime is a selection change, not a
            // starvation (the paces() carve-out). The re-entry
            // debounce (module docs) filters low-line flapping
            // without delaying a fresh episode's first stall.
            let mut starved = false;
            let mut any_low = false;
            for limb in self.limbs.iter_mut() {
                if !self.paces(limb) {
                    continue;
                }
                let mark = limb.queue.watermark();
                if mark.high {
                    // A full recovery closes the episode even without
                    // a rebuffer cycle in between.
                    limb.episode_open = false;
                }
                if mark.low {
                    any_low = true;
                    limb.low_streak = limb.low_streak.saturating_add(1);
                    if !limb.episode_open || limb.low_streak >= self.starve_debounce {
                        limb.episode_open = true;
                        starved = true;
                    }
                } else {
                    limb.low_streak = 0;
                }
            }
            if any_low {
                self.starvation.starved_ticks = self.starvation.starved_ticks.saturating_add(1);
            }
            report.starved = !self.eof && starved;
            if report.starved {
                self.starvation.starve_signals = self.starvation.starve_signals.saturating_add(1);
            }
""", "R-1923")

P.write_text(t)
print(f"markers now: {t.count('<<<<<<<')}")
