# WAVE 03 SUMMARY — playback engine, clocks, queues, engine tests

| Field | Value |
|---|---|
| Agents | A01–A10 complete + 1 orchestrator-caused completion run (A06) |
| Files audited | 109 files (152 chunk rows), 152/152 covered |
| Findings | S0:0 S1:0 S2:17 S3:62 S4:174 (253 total) |
| Citation gate | 198 checked, 174 verbatim, 24 range-offset/rewrap defects (all real text, 0 fabrications) |
| Tree purity | clean |

## MAJOR CORRECTION TO WAVE-0 INTERPRETATION
The MP4 litmus "zero decode" was an instrument artifact, not engine truth:
- bench.rs's decoder registry is hardcoded to "first vertical: mjpeg + pcm"
  and engine-info prints a matching stale list — but the shipped binary
  LINKS rust_h264 (CGU symbols verified by orchestrator) and 12+ decoders
  are wired (W03-A02).
- The 19ms Playing->Stopped was closed-stdin teardown (M17). With stdin held
  open the session plays the full 3s MP4 to end-of-media (W03-A01 executed;
  orchestrator re-verified).
- seek is NOT reopen-at-target (CLI help text is stale); the 7-step
  in-pipeline seek is real (W03-A06/A07).
Verdict so far: the ENGINE is real; the self-description strings and bench
instrument lie (in the understating direction). MP4-with-picture-and-sound
is still blocked downstream: null audio device, software compositor only,
GUI canvas question deferred to Waves 14/15.

## Worst findings
1. [S2] pipeline.rs zero-limb seek wedge: deselect both tracks + seek =>
   seek never completes, no terminal, clock frozen (W03-A01, traced).
2. [S2] session.rs subtitle selection lies on provider error (Completed
   receipt + TrackSelectionChanged emitted while mirror untouched) (W03-A02).
3. [S2] seek.rs `ns.clamp(0, duration)` PANICS on negative demux duration;
   queue-flush cost uncounted (W03-A07).
4. [S2] media-clock sync.rs: audio-side drift governor NEVER ARMED in
   production — A/V sync is video-ladder only, audio open-loop; shift>126
   EMA math broken (W03-A09).
5. [S2 cluster] recovery.rs + SourceRetryGuard/SurfaceGuard + poll_hotplug:
   compiled but ZERO production callers — recovery policy unreachable in
   shipped build (W03-A04/A09); hotplug 4 hooks zero shipped callers,
   doc 05 lists them as wire rows => matrix rows citing 7.C1 FALSE
   (W03-A02); frame_step UNREACHABLE — CLI refuses playback.step-frame
   while the UI ships 3 hotkeys for it (W03-A10); bookmarks dead code
   (live feature is a TS reimplementation) (W03-A03); adjust.rs zero
   callers with wrong hue rotation labeled "byte-exact GPU reference"
   (W03-A08).
6. [S2] scenario_12_device_loss: doc-10 row-12 criteria proven OPPOSITE by
   the test (W03-A09); scenario_04 tolerance claim 3x loose vs recorded
   value (W03-A07).
7. [S2] scenarios_b host serves tracks.set-*/set-position that NO shipped
   host serves; production budget breaks mid-media switching per scenario
   8's own comment (W03-A04).
8. [KEY STAT] playback-engine root: 67/88 re-exports (76%) have zero
   production refs; chapters/epoch/frame_step/hotplug/recovery/tracks
   re-export groups 100% dead (W03-A06 completion run).
9. [S2] scenario_01_open asserts "mp4 must refuse in this build" via a
   harness-local AVI-only sniff while the shipped CLI opens MP4 (executed)
   (W03-A02).
10. [KEY] Clock reality: no internal engine loop — host-stepped; the null
    sink's step counter drives the audio-master clock; backends are step
    counters except InstantSource; Ended is a near-trap (seek_back exists,
    unwired) (W03-A05).

## Carried forward
- Negative-duration -> seek panic reachability from mp4/mkv demuxers
  (W03-A07 -> Wave 5, S2->S1 candidate).
- decode-bench registry staleness -> Wave 13 (apps/cli bench.rs).
- events "WiredLater" 11/26 never-emitted set matches engine reality
  (W03-A03) -> Wave 19 PARITY cross-check.
- Disk: two agents hit ENOSPC mid-run (handled via prebuilt binaries);
  orchestrator reclaimed 2.6G before gate.
