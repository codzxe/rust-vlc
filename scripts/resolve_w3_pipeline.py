#!/usr/bin/env python3
"""Resolve the w2-12 (3.A3) merge conflicts in playback-engine (Lead)."""

from pathlib import Path

P = Path("/home/z/my-project/crates/playback-engine/src/pipeline.rs")
t = P.read_text()
n0 = t.count("<<<<<<<")

def rep(old, new, tag):
    global t
    assert t.count(old) == 1, f"{tag}: found {t.count(old)}"
    t = t.replace(old, new)

# R1: struct fields — merge subtitle docs, keep debounce field
rep("""<<<<<<< HEAD
    /// Caps for the subtitle lane (doc 03 §4.4(b): "small and
    /// duration-only") — the caps vocabulary only; wiring a subtitle
    /// limb is card 3.A3's.
    pub subtitle: QueueLimits,
    /// How many consecutive low ticks a re-entry dip must persist
    /// before it fires `starved` (design B2/F1.5).
    pub starve_debounce_ticks: u32,
=======
    /// Caps for subtitle limbs (card 3.A3): subtitle packets are tiny,
    /// sparse and latency-tolerant, so the caps sit far below the A/V
    /// defaults — `[PROPOSAL: doc-default constants; the product.toml
    /// wiring is owed to card 3.B3, which owns per-kind limits]`.
    pub subtitle: QueueLimits,
>>>>>>> wip/w2-12-track-switching
""", """    /// Caps for subtitle limbs (doc 03 §4.4(b): "small and
    /// duration-only"; card 3.A3): subtitle packets are tiny, sparse
    /// and latency-tolerant, so the caps sit far below the A/V
    /// defaults — `[PROPOSAL: doc-default constants; the product.toml
    /// wiring is owed to card 3.B3, which owns per-kind limits]`.
    pub subtitle: QueueLimits,
    /// How many consecutive low ticks a re-entry dip must persist
    /// before it fires `starved` (design B2/F1.5).
    pub starve_debounce_ticks: u32,
""", "R1")

# R2: ctor doc
rep("""<<<<<<< HEAD
    /// packet-queue constructor's own outputs); the subtitle lane and
    /// the debounce take their documented defaults.
=======
    /// packet-queue constructor's own outputs). The subtitle axis
    /// takes the doc default (see [`QueuePlan::default`]).
>>>>>>> wip/w2-12-track-switching
""", """    /// packet-queue constructor's own outputs); the subtitle lane and
    /// the debounce take their documented defaults (see
    /// [`QueuePlan::default`]).
""", "R2")

# R3: ctor body + next fn docs — keep HEAD body, then BOTH fns (with_subtitle doc then for_kind doc)
rep("""<<<<<<< HEAD
            subtitle: default_subtitle_limits(),
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
        }
    }

    /// A plan with every lane explicit (card 3.A3's track-switching
    /// shape).
=======
            subtitle: subtitle_default_limits(),
        }
    }

    /// The caps for a track kind. Audio/video/subtitle map to their
    /// limbs; `Data` keeps no limb (dropped at push, the existing
    /// policy) — the answer is only consulted for wired kinds.
>>>>>>> wip/w2-12-track-switching
""", """            subtitle: default_subtitle_limits(),
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
        }
    }

    /// A plan with every lane explicit (card 3.A3's track-switching
    /// shape).
""", "R3")

P.write_text(t)
print(f"pipeline.rs: {n0} -> {t.count('<<<<<<<')} markers")
