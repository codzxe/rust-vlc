#!/usr/bin/env python3
"""Graft the merged wire() (Lead, part 4): theirs' one-per-kind structure
+ HEAD's 3.B3 starve_debounce init; drop HEAD's old all-tracks loop."""

from pathlib import Path

P = Path("/home/z/my-project/crates/playback-engine/src/pipeline.rs")
t = P.read_text()

old = """<<<<<<< HEAD
        self.selected = Vec::with_capacity(self.tracks.len());
        self.starve_debounce = plan.starve_debounce_ticks.max(1);
        let mut wired = 0;
        for track in &self.tracks {
            let wireable = matches!(track.kind, MediaKind::Audio | MediaKind::Video);
            if !wireable {
                self.selected.push(false);
=======
        self.selected = vec![false; self.tracks.len()];
        let mut outcome = WireOutcome::default();
        for kind in [MediaKind::Video, MediaKind::Audio, MediaKind::Subtitle] {
            let Some(track) = crate::tracks::default_track(&self.tracks, kind) else {
>>>>>>> wip/w2-12-track-switching
"""
new = """        self.selected = vec![false; self.tracks.len()];
        self.starve_debounce = plan.starve_debounce_ticks.max(1);
        let mut outcome = WireOutcome::default();
        for kind in [MediaKind::Video, MediaKind::Audio, MediaKind::Subtitle] {
            let Some(track) = crate::tracks::default_track(&self.tracks, kind) else {
"""
assert t.count(old) == 1
t = t.replace(old, new)

old2 = """<<<<<<< HEAD
            let Some(limits) = plan.for_kind(track.kind) else {
                // Unreachable for wireable kinds (audio/video always
                // have lanes); the exhaustive `Option` keeps the type
                // honest for 3.A3's track switching.
                self.selected.push(false);
                continue;
            };
            let queue = Arc::new(BoundedQueue::new(limits, plan.thresholds(&limits)));
            let high_packets = plan.thresholds(&limits).high.packets.max(1);
            // Publish the live generation before any producer starts
            // (doc 03 §5 checking point 4): a fresh queue publishes
            // FIRST, so this flush is a no-op at session start and
            // the standing boundary check for every later mint.
            let _ = queue.flush(generation);
            let config = DecoderConfig::new(
                track.codec.clone(),
                generation,
                ThreadBudget::default(),
                None,
                None,
            );
            let decoder = provider.decoder_for(track, &config)?;
            self.limbs.push(Limb {
                track: track.clone(),
                queue,
                decoder,
                drained: false,
                high_packets,
                low_streak: 0,
                episode_open: false,
                last_frame: None,
                conceal_failures: 0,
            });
            self.selected.push(true);
            wired += 1;
=======
>>>>>>> wip/w2-12-track-switching
"""
new2 = ""
assert t.count(old2) == 1
t = t.replace(old2, new2)

P.write_text(t)
print(f"markers now: {t.count('<<<<<<<')}")
