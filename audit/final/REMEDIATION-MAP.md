# REMEDIATION MAP — what stands between HEAD and "an MP4 plays with picture and sound on Windows"

Ranked by (is the user experience broken without it) × (how provable the fix is). Not a fix;
each item names the blocker, the evidence, the estimated size, and its dependencies.

## B1. Audio output backend (BLOCKER — sound is impossible on every OS today)
- **Evidence:** `crates/audio-output/src/lib.rs:152-157` ships exactly `[NullBackend, UnsupportedBackend]`;
  `pipeline_host.rs:1555-1556` opens the NullBackend in **discard** mode; `cpal.rs:43`
  `AUDIO_ADR005_ADMITTED = false` (compile-time closed); `worklog.md:2294-2296` admits
  "audio output remains the null device engine-wide"; a partial attempt exists stashed
  (`git stash list`: "w31-audio-cpal-partial").
- **Size:** medium-large — one real backend (cpal on the three OSes) + the ADR-005 admission
  reversal + device enumeration UI + volume/mute plumbing (the command layer already exists).
- **Depends on:** nothing below. Everything above the null seam already decodes AAC-LC end to
  end (executed by this audit).

## B2. Runtime GUI display proof (BLOCKER for the claim "you can see it", not proven broken)
- **Evidence:** the SEE chain is statically complete (engine I420→RGBA → FrameTap →
  newest-frame-per-tick → `putImageData` on the canvas, all cited in W13-A02 /
  `audit/final/MP4-REALITY-TRACE.md` §B.10), but (a) no runtime pixel evidence exists in the
  tree (the GUI frame fixtures are AVI/MJPEG era, never H.264+AAC), (b) the tauri transport's
  event-listen shape carries an unresolved `[VERIFY]` marker (`tauri-transport.ts:92-98`),
  (c) the canvas seam is mock-tested only (jsdom has no rasterizer).
- **Size:** small-medium — one Playwright/real-OS smoke that opens the H.264+AAC MP4 in the
  shell and asserts rendered frames (even a pixel-diff hash), plus resolving the `[VERIFY]`
  marker. The w31.4 lane claims to have done this manually; automate it.
- **Depends on:** B4 (the self-descriptions that currently mislead every tester).

## B3. MP4 seek (MAJOR defect in an otherwise working path)
- **Evidence:** typed refusal `plugin.demux.reanchor.unsupported` on the executed sample;
  `help.rs:57` describes a reopen semantics that does not exist; W05-A01's backward-seek
  cursor bug (`order_cursor` never reset, `mod.rs:1571-1577`/`:2129-2141`).
- **Size:** medium — implement re-anchor + fix the cursor reset + update the help line +
  the three demux S1s below ride along naturally.

## B4. Self-description refresh discipline (the paperwork class defect)
- **Evidence:** engine-info static 2-decoder vec pinned by a test (`host.rs:1452-1482` +
  `command_surface.rs:125-133`); decode-bench's stale allowlist (`bench.rs:300-334`) which
  cannot even seed `extra_data` (`bench.rs:427-446`); PARITY-REPORT 64/252 symmetric vs live
  66/61/66; KNOWN-LIMITS canvas row; README URL-refusal; ROADMAP command counts; 17 stale
  claims total (W19).
- **Size:** small each, systemic in aggregate — derive every status surface from the live
  registry (the registry data exists; `engine_info()` just doesn't read it), and add
  "doc-touched-in-the-same-commit-as-the-code" to the change-control rule the matrix header
  already declares.

## B5. Demux hardening before any real user corpus (S1s found this run)
- stts/ctts unclamped loops (`mod.rs:1253-1266`/`:1314-1332`) — crafted-moov CPU DoS;
- track-id space split on chapter traks (`mod.rs:833-839` vs `:1882`);
- elst empty-edit units bug — and DELETE the test that pins it (`tests.rs:196-197` asserts
  the wrong value; fix both together);
- dead depth guard (`error.rs:81` never called while the doc claims it exists).
- **Size:** small each; ship them as one demux-hardening pass with crafted fixtures.

## B6. Evidence standard upgrade (the audit-ability defect)
- **Evidence:** the empty-file experiment — `feature-audit` PASSes a fabricated
  4-platform-implemented row whose evidence is a 0-byte file; 52 SHALLOW matrix rows are
  certified to that standard today.
- **Size:** medium — evidence strings should name runnable checks (a test path + a filter),
  and `feature-audit` check C should run that test (or at least verify the referenced test
  exists AND the referenced feature is registered in the shipped build).
- **Depends on:** B4 culturally; the tool is honest about the gap (lines 17-22) — the
  ecosystem chose not to close it.

## Dependency order
B1 and B5 are independent; B4 first among the paperwork; B2 depends on B4 (testers must be
able to trust self-descriptions); B3 independent; B6 continuous. With B1+B2+B4 done, the
headline claim "an MP4 plays with picture and sound" becomes provable in one automated run.
