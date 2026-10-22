#!/usr/bin/env python3
"""Resolve final pipeline.rs regions (Lead, part 5)."""

from pathlib import Path

P = Path("/home/z/my-project/crates/playback-engine/src/pipeline.rs")
t = P.read_text()

def rep(old, new, tag):
    global t
    assert t.count(old) == 1, f"{tag}: {t.count(old)}"
    t = t.replace(old, new)

# R-carveout: union — theirs' switch-aware carve-out + HEAD's 3.C1 resolution
# (supersedes the stale [PROPOSAL]); theirs' window_live() covers
# seek_progress via the window model (verify with compiler).
rep("""<<<<<<< HEAD
        // one carve-out: while a seek is in flight, decode runs even
        // in Paused so the landing frame exists to re-anchor on (the
        // wire-visible state stays paused; the clock stays frozen
        // until the completion report). While the seek is in flight
        // the decode's frames enter the M10 presentation hold, not
        // the consumer — the carve-out produces evidence and held
        // frames, never a pre-terminal presentation.
        // [3.C1 resolution of Lead decision 3.HARD-AUDIT D-a: the
        // buffered pause (Buffering+Pause → Paused) now rides its own
        // episode bookend rows — `Paused + Primed` closes the episode
        // the pause held open — so the pre-roll of a pause-departed
        // SEEK keeps this carve-out (there is still no
        // paused→buffering wire edge, doc 05 §3.3; the seeking
        // pre-roll presents through the M10 hold, not an episode).]
        if matches!(state, SessionState::Buffering | SessionState::Playing)
            || self.seek_progress.is_some()
        {
=======
        // one carve-out: while a seek **or a track switch** is in
        // flight, decode runs even in Paused so the landing frame
        // exists to re-anchor/complete on (the wire-visible state
        // stays paused; the clock stays frozen until the completion
        // report). While the seek is in flight the decode's frames
        // enter the M10 presentation hold, not the consumer — the
        // carve-out produces evidence and held frames, never a
        // pre-terminal presentation. A switch's re-prime holds
        // nothing (card 3.A3 step 7: the frame's delivery IS the
        // completion fact).
        // [PROPOSAL: Lead decision 3.HARD-AUDIT D-a — there is no
        // paused→buffering wire edge (doc 05 §3.3), so the pre-roll
        // cannot ride the buffering episode while paused; revisit at
        // 3.C1 state-machine completeness. 3.A3 extends the same
        // carve-out to its switch windows.]
        if matches!(state, SessionState::Buffering | SessionState::Playing) || self.window_live() {
>>>>>>> wip/w2-12-track-switching
""", """        // one carve-out: while a seek **or a track switch** is in
        // flight, decode runs even in Paused so the landing frame
        // exists to re-anchor/complete on (the wire-visible state
        // stays paused; the clock stays frozen until the completion
        // report). While the seek is in flight the decode's frames
        // enter the M10 presentation hold, not the consumer — the
        // carve-out produces evidence and held frames, never a
        // pre-terminal presentation. A switch's re-prime holds
        // nothing (card 3.A3 step 7: the frame's delivery IS the
        // completion fact).
        // [3.C1 resolution of Lead decision 3.HARD-AUDIT D-a: the
        // buffered pause (Buffering+Pause → Paused) now rides its own
        // episode bookend rows — `Paused + Primed` closes the episode
        // the pause held open — so the pre-roll of a pause-departed
        // SEEK keeps this carve-out (there is still no
        // paused→buffering wire edge, doc 05 §3.3; the seeking
        // pre-roll presents through the M10 hold, not an episode).
        // 3.A3 extends the same carve-out to its switch windows.]
        if matches!(state, SessionState::Buffering | SessionState::Playing)
            || self.seek_progress.is_some()
            || self.window_live()
        {
""", "carveout")

P.write_text(t)
print(f"markers now: {t.count('<<<<<<<')}")
