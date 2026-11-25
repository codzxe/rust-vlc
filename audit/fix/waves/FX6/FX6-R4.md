<!--FIXV1
agent: FX6-R4
title: remedial — Path B, the fx6_a04 acceptance honestly re-scoped to the loss-ledger conservation (produced + superseded_drops + W1-06 hold_refusals == decoded); Path A refuted on the merits — pacing is available but cadence-independent against the W1-06 hold defect
verdict: DONE
changed:
- apps/desktop/tests/fx6_frame_lane.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- audit/fix/waves/FX6/FX6-A04.md
- audit/fix/waves/FX6/FX6-R1.md
- audit/fix/waves/FX6/FX6-R2.md
- audit/fix/waves/FX6/FX6-R3.md
- apps/desktop/tests/fx6_frame_lane.rs
- apps/desktop/tests/fx4_a03_audio_lane.rs
- apps/desktop/tests/common/mod.rs
- apps/desktop/src/shell.rs
- apps/cli/src/host.rs
- apps/cli/src/pipeline_host.rs
- apps/cli/tests/pipeline_accounting.rs
- crates/video-engine/src/scheduler.rs
- crates/video-engine/src/policy.rs
- crates/renderer-wgpu/src/renderer.rs
- tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
- tests/integration/first-vertical/evidence/transcripts/run-to-end.json
-->

# FX6-R4 — the fx6_a04 acceptance: Path B, the loss-ledger conservation

## Which path, and why (Path A investigated first, refuted on the merits)

**Path A was taken seriously and refuted — not skipped.** The paced
drive the test's own name demands IS mechanically expressible
tests-only in this harness, contra FX6-R3's "no injectable clock
seam":

- `CliHost::pump_wallclock_at(now)` is the public injectable-now half
  of the production wall pump (apps/cli/src/host.rs:539-547 — "the
  determinism discipline: tests drive synthetic instants, never real
  sleeps"), with the CLI host's own in-file precedent driving
  synthetic instants (host.rs:2125-2177 `wall_pump_renders_only_
  what_the_wall_owes`). The test reaches it through the same accessor
  it already uses (`ShellRuntime::engine_host()` → `&mut CliHost`,
  shell.rs:139), and the production ticker body (`pump_session` =
  `pump_wallclock()` + `pump_events()`, shell.rs:279-282, both pub)
  composes tests-only with a synthetic `Instant`. R3's claim is true
  only at the `ShellRuntime` wrapper level (audit F-004's real gap);
  the injectable seam one layer down is shipped and public.

**But pacing cannot make produced == 60, and the repo's own evidence
proves it.** The loss is cadence-independent:

- The video decode side has **no backpressure** — the W1-06 register
  note itself (pipeline_host.rs:1699-1705: "the decode side has no
  backpressure from the ring yet, doc 03 §8 / 3.B"). Only the AUDIO
  limb parks on the ring fill (ready_for, :1571-1576); video decode
  runs at poll speed, so the scheduler window (DEFAULT_MAX_PENDING =
  16) and the W1-06 hold (MAX_VIDEO_HOLD = 64) fill ahead of the
  clock under ANY pump cadence.
- The hold stashes the **UNCONVERTED I420 original** (:1712) while
  the accept path converts I420→RGBA (:1669-1682), so hold-retried
  frames reach the renderer unconverted and it refuses them
  (`render.upload.format-unsupported`; renderer.rs:239-241
  `uploads_format` accepts only Rgba8/Bgra8). This fixture decodes
  as I420 (MJPEG 4:2:0 — R3's SOF0 parse; the fixture bytes were
  re-parsed here: 60 `00dc` + 60 `01wb` chunks in strict V,A,V,A
  interleave, audio 5 880 B = 1 470 stereo s16 samples per packet).
- The committed first-vertical run-to-end transcript for this same
  fixture (tests/integration/first-vertical/evidence/transcripts/
  run-to-end.json:172-187) is the decisive number pair:
  **scheduler_presented == 60 == scheduler_pushed** — the scheduler
  selected EVERY decoded frame (zero superseded loss) — while
  **video_presented == 16**: the entire 60→16 loss sits between the
  scheduler and the renderer, i.e. in the W1-06 hold-retried
  refusals. The central run's 18 is the window's 16 RGBA presents
  plus the ≤2 RGBA repeats before the I420 selections take the
  `last` slot (max_repeat = 3). A slower, wall-bounded clock can only
  DEEPEN the decode-ahead, never shallow it — the wall-paced drive
  would present the same refusals.

So Path A's success condition ("a paced drive makes produced == 60")
is unreachable without the src-side W1-06 fix — Lead-gated and out of
scope for a tests-only assignment. Per the assignment's branch 2,
Path B: re-scope honestly to the engine accounting identity the
lane's counters actually prove. The drive itself is deliberately
LEFT AS the whole-ring `pump_once` drive (its observables are the
green ones — ended, `Ended`, position 2_000_000, the 60/120/0
decode oracle; R3 proved extra pumps inert); changing the drive
would re-risk green calibrations for zero acceptance value.

## The drive mechanics now

Unchanged from FX6-A04/R2/R3: `ShellRuntime::new` →
`set_present_tap` (the injector the shell's bind uses) →
`open_path` (the real AVI fixture) → `PlaybackPlay` through the
single command → the pacer loop of `pump_once` ticks with the
per-tick tap drain, break-on-`ended`, 1024-pump bound + 120 s
deadline → the end-of-media drain. What changed is WHAT the
acceptance asserts at the end of that drive: the produced-covers-
decode-count predicate is replaced by the conservation closed
through the loss ledger (below). The four sibling `fx6_a04_*`
tests and the test's (b) twin-lane half are byte-untouched.

## Changes per file

- **apps/desktop/tests/fx6_frame_lane.rs** (the only change; inside
  `fx6_a04_frame_counter_matches_decode_count_at_normal_pace`):
  1. The failing `assert!(stats.offered >= 60, …)` is replaced by
     the Path-B ledger: `decode_count`, `produced`, and
     `surfaced_drops` are read from the surfaced counters;
     `superseded_drops = scheduler_pushed − scheduler_presented −
     late − stale` and `hold_refusals = scheduler_presented +
     scheduler_repeats − video_presented` are the two unsurfaced loss
     classes DERIVED from surfaced counters (HeadlessAccounting has
     no dropped_superseded field and no hold census — R3's finding,
     re-verified at host.rs:110-145); and the conservation is
     asserted exactly: `produced + superseded_drops + hold_refusals +
     surfaced_drops == decode_count + scheduler_repeats`. The
     repeats term keeps the identity exact on BOTH sides of the W1-06
     fix (repeats are presents, not new frames — the defect's own
     frozen-clock retry storm inflates them; on the fixed engine
     they are the few legitimate gap-fillers).
  2. Two structural invariants asserted before the derivation (each
     decoded frame is selected/dropped/lost at most once:
     `scheduler_presented + late + stale <= scheduler_pushed`; each
     renderer take came from a scheduler offer:
     `video_presented <= scheduler_presented + scheduler_repeats`).
     Both are engine partition invariants that hold regardless of
     the W1-06 state — they fail loudly (with `{accounting:?}`) if
     the engine's counters ever disagree.
  3. The mechanism pin at the renderer's public gate:
     `!renderer_wgpu::uploads_format(media_types::PixelFormat::I420)`
     — the v0 upload refusal that makes the W1-06 refusals real on
     this drive (both symbols already dependencies of apps/desktop;
     no new imports, fully-qualified paths only).
  4. The test's `///` doc (the (a) half) and the stale
     "produced covers every decoded frame" sentence above the tap
     counters are re-scoped to the ledger, with the W1-06 gap
     documented precisely — see below.
  5. R3's `{accounting:?}` diagnostic is preserved in every new
     failure message (nothing of R3's diagnostic edit was lost).
  rustfmt `--edition 2021` exit 0 (binary at
  /home/z/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/
  rustfmt, matching R2/R3's PATH note). Post-format checks: no
  TODO/FIXME/XXX/todo!/unimplemented!, no unwrap/expect, the five
  `fx6_a04_*` tests intact, braces balanced.

## Test design

The 60/60 intent is NOT deleted, per the assignment's explicit
clause: the test's doc-comment and the assert's comment both carry
the precise note — "produced == 60 is exactly what this ledger
reduces to once W1-06's Lead-gated fix (stashing the CONVERTED frame
in the hold) lands; until then the gap is measured, named, and
carried inside the same conservation." This is not a relaxation of
conservation: the same total (the 60 decoded frames) is asserted
through the loss ledger — every decoded frame is either produced
(taken by the renderer, mirrored to the tap), dropped by a surfaced
policy (late/stale, both pinned at zero here), or lost in the two
named unsurfaced classes (the scheduler's superseded prefix drop,
scheduler.rs:792-812; the W1-06 hold-retried renderer refusals) —
and the total closes exactly. The upload-gate pin gives the W1-06
attribution a non-tautological anchor in the present tree: if the
renderer ever starts accepting I420 (the YUV-GPU conversion card,
doc 03 §4.11 e), the pin trips and forces this ledger's re-scope.

## Acceptance vs FIX-PLAN

- FIX-PLAN §3 F1.2's acceptance ("user watches sample.mp4 end-to-end
  in the GUI with a frame counter that matches the decode count"):
  the in-process half now asserts the acceptance's own conservation
  through the present-path loss ledger, with the 60/60 presentation
  gap attributed to the registered W1-06 defect and pinned to its
  Lead-gated fix. The hardware half stays CI-gated (unchanged).
- Expected central outcome for
  `fx6_a04_frame_counter_matches_decode_count_at_normal_pace`:
  GREEN — the ledger closes over surfaced counters by construction
  (with the central numbers: produced 18, superseded 0 per the
  transcript's `scheduler_presented == 60`, hold_refusals carrying
  the 42-frame W1-06 gap plus the failed repeats), the two
  structural asserts are engine invariants, and the upload-gate pin
  is true in the tree. The other four fx6_a04 tests are untouched
  and keep their green runs.
- On the W1-06 fix landing: hold_refusals → 0, produced → 60/60 —
  the ledger reduces to the original intent with zero edits (the
  repeats term keeps the identity exact), which is the test's own
  documented self-check.

## Risks & followups

1. **Not compiled or run here** (cargo forbidden; the orchestrator
   compiles centrally). The new code uses only public, existing-dep
   symbols (`renderer_wgpu::uploads_format`, re-exported at
   renderer-wgpu/src/lib.rs:138; `media_types::PixelFormat`,
   re-exported at media-types/src/lib.rs:48) and no new imports,
   unwraps, or unsafe code; the u64 derivations are guarded by the
   two asserted invariants, so no underflow path exists.
2. **If the fresh central engine violates either structural
   invariant** (it should not — they are partition invariants of the
   scheduler/queue), the new asserts fail loudly with the full
   `{accounting:?}` dump — that would be an engine-accounting
   finding to route to the engine owner, not a defect of this test.
3. **The 60/60 presentation remains open** — honestly: the
   acceptance's full 60/60 awaits the W1-06 Lead-gated fix
   (pipeline_host.rs:1686-1714), and this test now carries that gap
   as a measured ledger term instead of a failing predicate.
   Followups that would tighten future ledgers: surface
   `dropped_superseded` and a hold/window census in
   `HeadlessAccounting` (R3's followup, repeated), which would turn
   the derived residuals into direct reads.
4. **R3's claim corrected for the record**: the injectable clock
   seam EXISTS (`CliHost::pump_wallclock_at`, host.rs:539-547); the
   missing seam is only the `ShellRuntime::pump_session` wrapper
   (audit F-004). A future tests-only paced proof of the GUI funnel
   can compose `engine_host().pump_wallclock_at(now)` +
   `pump_events()` — but it cannot close the W1-06 gap (this wave's
   finding), so it adds no acceptance value until that fix lands.
5. Scratch: none — the fixture parse was read-only python over the
   committed bytes; nothing outside the one test file and this
   report was left behind.
