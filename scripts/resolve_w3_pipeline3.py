#!/usr/bin/env python3
"""Resolve remaining w2-12 pipeline.rs regions (Lead, part 3)."""

from pathlib import Path
import re

P = Path("/home/z/my-project/crates/playback-engine/src/pipeline.rs")
t = P.read_text()

def rep(old, new, tag):
    global t
    assert t.count(old) == 1, f"{tag}: found {t.count(old)}"
    t = t.replace(old, new)

# R8: Limb struct fields — union (HEAD's 3.B3 fields + theirs' 3.A3 fields)
rep("""<<<<<<< HEAD
    /// Consecutive low-watermark ticks (card 3.B3, design B2): the
    /// starve-debounce streak. Reset by any non-low observation;
    /// per-pipeline-generation state (cleared at seek mints).
    pub low_streak: u32,
    /// A starvation episode has fired since this limb's queue last
    /// reached its high watermark (card 3.B3, design B2/F1.5): the
    /// first stall of a fresh episode fires at 1 tick; a re-entry
    /// dip while the episode is open must persist
    /// [`QueuePlan::starve_debounce_ticks`] consecutive low ticks.
    pub episode_open: bool,
=======
    /// The queue's low watermark in packets — the **recovery target**
    /// that ends a switch-installed limb's re-prime exemption (see
    /// [`Limb::first_frame_pending`]).
    pub low_packets: usize,
    /// **The limb's generation floor** (card 3.A3): the generation
    /// this limb was last flushed/configured at. The sink checks
    /// frames against this, not the pipeline-global scalar — after a
    /// switch mint the unchanged kind's G-stamped residents stay
    /// presentable while the switched kind's stale stragglers (stamps
    /// < the new limb's floor) still die. Soundness: every packet a
    /// limb decodes passed its queue's boundary (`stamp <
    /// queue.published` refused), and flushes only ratchet
    /// (`published ≥ floor`), so emitted stamps are never spuriously
    /// stale; genuinely stale work carries stamps below the floor of
    /// whichever limb would present it. Doc 03 §5 point 5's
    /// checkpoint semantics are thereby read **per the generation
    /// the consumer path was last invalidated at** (noted for
    /// 3.B4's contract suite).
    pub generation: GenerationId,
    /// A limb installed by a switch whose queue has not yet
    /// recovered from the swap (card 3.A3 step 5, the S2.5
    /// carve-out): while armed, the limb is exempt from the underrun
    /// check and from priming — the new queue starts empty through
    /// no fault of the input, and a selection change is not a
    /// starvation. Cleared when the queue first reaches its **low
    /// watermark** (not at the first frame: the queue is still
    /// climbing then, and the completing tick would otherwise flash
    /// playing→buffering). A sparse or span-exhausted track simply
    /// never re-paces (VLC parity — honest silence, no rebuffer).
    pub first_frame_pending: bool,
>>>>>>> wip/w2-12-track-switching
""", """    /// Consecutive low-watermark ticks (card 3.B3, design B2): the
    /// starve-debounce streak. Reset by any non-low observation;
    /// per-pipeline-generation state (cleared at seek mints).
    pub low_streak: u32,
    /// A starvation episode has fired since this limb's queue last
    /// reached its high watermark (card 3.B3, design B2/F1.5): the
    /// first stall of a fresh episode fires at 1 tick; a re-entry
    /// dip while the episode is open must persist
    /// [`QueuePlan::starve_debounce_ticks`] consecutive low ticks.
    pub episode_open: bool,
    /// The queue's low watermark in packets — the **recovery target**
    /// that ends a switch-installed limb's re-prime exemption (see
    /// [`Limb::first_frame_pending`]).
    pub low_packets: usize,
    /// **The limb's generation floor** (card 3.A3): the generation
    /// this limb was last flushed/configured at. The sink checks
    /// frames against this, not the pipeline-global scalar — after a
    /// switch mint the unchanged kind's G-stamped residents stay
    /// presentable while the switched kind's stale stragglers (stamps
    /// < the new limb's floor) still die. Soundness: every packet a
    /// limb decodes passed its queue's boundary (`stamp <
    /// queue.published` refused), and flushes only ratchet
    /// (`published ≥ floor`), so emitted stamps are never spuriously
    /// stale; genuinely stale work carries stamps below the floor of
    /// whichever limb would present it. Doc 03 §5 point 5's
    /// checkpoint semantics are thereby read **per the generation
    /// the consumer path was last invalidated at** (noted for
    /// 3.B4's contract suite).
    pub generation: GenerationId,
    /// A limb installed by a switch whose queue has not yet
    /// recovered from the swap (card 3.A3 step 5, the S2.5
    /// carve-out): while armed, the limb is exempt from the underrun
    /// check and from priming — the new queue starts empty through
    /// no fault of the input, and a selection change is not a
    /// starvation. Cleared when the queue first reaches its **low
    /// watermark** (not at the first frame: the queue is still
    /// climbing then, and the completing tick would otherwise flash
    /// playing→buffering). A sparse or span-exhausted track simply
    /// never re-paces (VLC parity — honest silence, no rebuffer).
    pub first_frame_pending: bool,
""", "R8")

P.write_text(t)
print(f"pipeline.rs markers now: {t.count('<<<<<<<')}")
