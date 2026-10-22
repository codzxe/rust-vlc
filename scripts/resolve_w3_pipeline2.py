#!/usr/bin/env python3
"""Resolve remaining w2-12 pipeline.rs regions (Lead, part 2)."""

from pathlib import Path

P = Path("/home/z/my-project/crates/playback-engine/src/pipeline.rs")
t = P.read_text()

def rep(old, new, tag):
    global t
    assert t.count(old) == 1, f"{tag}: found {t.count(old)}"
    t = t.replace(old, new)

# R4 (356): for_kind — HEAD's Option shape (3.B3 design; Data => None)
rep("""<<<<<<< HEAD
            MediaKind::Audio => Some(self.audio),
            MediaKind::Video => Some(self.video),
            MediaKind::Subtitle => Some(self.subtitle),
            MediaKind::Data => None,
=======
            MediaKind::Video => self.video,
            MediaKind::Audio => self.audio,
            MediaKind::Subtitle => self.subtitle,
            MediaKind::Data => self.audio,
>>>>>>> wip/w2-12-track-switching
""", """            MediaKind::Audio => Some(self.audio),
            MediaKind::Video => Some(self.video),
            MediaKind::Subtitle => Some(self.subtitle),
            MediaKind::Data => None,
""", "R4")

# R5 (411): default doc — HEAD values (128/16MiB/10s, matches default_subtitle_limits)
rep("""<<<<<<< HEAD
    /// 4 MiB, subtitle 128 packets / 16 MiB / 10 s `[PROPOSAL: the
    /// byte caps; the packet/duration figures are doc 03 §4.4's]`.
=======
    /// 4 MiB, subtitle 64 packets / 30 s / 64 KiB `[PROPOSAL: the
    /// byte caps and the subtitle axis; the packet/duration figures
    /// are doc 03 §4.4's]`.
>>>>>>> wip/w2-12-track-switching
""", """    /// 4 MiB, subtitle 128 packets / 16 MiB / 10 s `[PROPOSAL: the
    /// byte caps; the packet/duration figures are doc 03 §4.4's]`.
""", "R5")

# R6 (425 in Default): keep HEAD (debounce field + renamed fn)
rep("""<<<<<<< HEAD
            subtitle: default_subtitle_limits(),
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
=======
            subtitle: subtitle_default_limits(),
>>>>>>> wip/w2-12-track-switching
""", """            subtitle: default_subtitle_limits(),
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
""", "R6")

# R7: delete the dead theirs-side duplicate fn (subtitle_default_limits at ~436)
rep("""/// The subtitle axis's doc-default caps (see [`QueuePlan::default`]).
fn subtitle_default_limits() -> QueueLimits {
    QueueLimits {
        max_packets: nonzero(64),
        max_bytes: nonzero(64 << 10),
        max_duration: media_types::Nanoseconds(30 * 1_000_000_000),
    }
}

""", "", "R7-dead-fn")

P.write_text(t)
print(f"pipeline.rs markers now: {t.count('<<<<<<<')}")
