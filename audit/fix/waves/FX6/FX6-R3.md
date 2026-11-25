<!--FIXV1
agent: FX6-R3
title: remedial — fx6_a04 18/60 frames: the drive already reached end-of-media and the decode oracle is green; the shortfall is engine-side, not pacer calibration — assertion kept byte-strong, failure message now carries the engine accounting
verdict: PARTIAL
changed:
- apps/desktop/tests/fx6_frame_lane.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- audit/fix/waves/FX6/FX6-A04.md
- audit/fix/waves/FX6/FX6-R2.md
- audit/fix/waves/FX6/FX6-R1.md
- audit/fix/waves/FX4/FX4-A03.md
- audit/files/apps/desktop/src/shell.rs.audit.md
- apps/desktop/tests/fx6_frame_lane.rs
- apps/desktop/tests/fx4_a03_audio_lane.rs
- apps/desktop/src/shell.rs
- apps/cli/src/host.rs
- apps/cli/src/pipeline_host.rs
- apps/cli/tests/pipeline_accounting.rs
- crates/playback-engine/src/session.rs
- crates/playback-engine/src/state.rs
- crates/playback-engine/src/pipeline.rs
- crates/playback-engine/tests/eom_playout.rs
- crates/video-engine/src/scheduler.rs
- crates/video-engine/src/policy.rs
- crates/video-engine/src/queue.rs
- crates/renderer-wgpu/src/renderer.rs
- crates/codecs/src/video/mjpeg/mod.rs
- tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
- tests/integration/first-vertical/evidence/transcripts/open.json
- tests/integration/first-vertical/evidence/transcripts/run-to-end.json
- audit/runs/baseline/cargo-test-workspace.txt
-->

# FX6-R3 — the fx6_a04 acceptance-test 18/60 calibration failure

## Diagnosis — why 18 of 60, and why it is NOT drive calibration

What the central run actually proves (every assert before the failing line
PASSED — the run's own green asserts disprove the situation brief's premise):

1. **The drive DID decode the whole fixture.** The decode-count oracle
   re-derived exactly through the GUI funnel: `scheduler_pushed == 60`,
   `scheduler_dropped_late == 0`, `frames_accepted == 120`
   (fx6_frame_lane.rs:600-605, all green). "The drive loop does not decode
   the whole fixture" is contradicted by the run itself.
2. **The drive DID reach end-of-media.** `ended` within the bound (:575-578),
   state `Ended` (:585), position exactly `2_000_000` µs (:587-590). The
   audio drain gate (apps/cli/src/pipeline_host.rs:698-729) releases EOS only
   below one render quantum of ring residue, and the end-tail play-out
   (apps/cli/src/host.rs:806-841) renders any residue with per-quantum
   scheduler ticks; 88 200 == 60 × 1 470 consumed frames is exactly the
   2.000 s fixture — the ring emptied to zero. "The pacer needed an
   end-of-media drain" is disproven: the drain handshake completed.
3. **The lane is not the defect.** `stats.offered == accounting.video_presented`
   (:613-617, green) — the tap mirrors the renderer exactly; the mailbox
   conservation identity (:618-623), `conserves()` and `dropped_stale == 0`
   all green; the four sibling fx6_a04 tests are green. The failing number is
   the ENGINE's own: `video_presented == 18` (apps/cli/src/host.rs:423) — the
   renderer presented 18 of the 60 decoded frames under the whole-ring pump
   class (`pump_once` → `CliHost::pump`, apps/desktop/src/shell.rs:258-261 →
   apps/cli/src/host.rs:468-472, whose `advance_time` renders the whole ring
   per pump, host.rs:726-747).
4. **More pump iterations are provably inert.** After the EOS pump the
   pump's halves no-op: `StopClock` pauses the clock at the `Drained → Ended`
   transition (crates/playback-engine/src/session.rs:1648-1653, row at
   state.rs:512-517), `advance_time` early-returns on the paused clock
   (host.rs:714-717), and `drain_tail_at_end` early-returns on the empty ring
   (host.rs:822-824). The 1024-pump bound was never binding — the loop ended
   on the EOS event, not the bound (`assert!(ended)` at :575-578 passed;
   the pacer proofs measure this fixture class at ~170 pumps). So the
   assignment's lever — "enough pump iterations / proper end-of-media drain"
   — cannot move 18 → 60; the count is already final at the break.

Where the other 42 went — engine-side, in counters `HeadlessAccounting`
never surfaces (host.rs:110-145 has no `dropped_superseded` field, no
window/hold census):

- **The scheduler's structural SUPERSEDED drop** —
  crates/video-engine/src/scheduler.rs:792-812: per tick only the newest due
  frame presents; "everything due before it is superseded — dropped and
  counted … the structural drop applies under every policy,
  `DropPolicy::Never` included". Under a bursty whole-ring clock phasing this
  decimates the present set by design.
- **The registered W1-06 KNOWN DEFECT** — apps/cli/src/pipeline_host.rs:
  1686-1714: the window-full hold stashes the UNCONVERTED original
  (`frame.clone()`, :1712) while the accept path converts I420→RGBA at
  :1669-1682; hold-retried frames reach the renderer as I420 and are refused
  (`render.upload.format-unsupported`, crates/renderer-wgpu/src/renderer.rs:
  239-241). THIS fixture decodes as I420: parsing the fixture's JPEG SOF0
  shows Y 2x2 / Cb,Cr 1x1 (4:2:0), and codecs/src/video/mjpeg/mod.rs:18 maps
  4:2:0 → `PixelFormat::I420`. The tree's own note says the complete fix
  needs the Lead's pacing/backpressure decision — a src change outside this
  tests-only assignment.
- **The class has a recorded prior**: the committed first-vertical
  run-to-end transcript for this same fixture pinned `video_presented: 16`
  of 60 (tests/integration/first-vertical/evidence/transcripts/run-to-end.json:
  175-179), and the baseline's transcript-currency failure
  (audit/runs/baseline/cargo-test-workspace.txt:2764-2776) shows the engine's
  present-path accounting drifted since (committed open.json: 94 accepted /
  16 pushed; fresh run: 30 / 15) — this drive class has never presented a
  full set anywhere in the repo's evidence.
- **The engine's own pinned oracle agrees with the drive, not the test**:
  apps/cli/tests/pipeline_accounting.rs:101-104 asserts "at least every
  frame once" ONLY at the scheduler level (`scheduler_presented +
  scheduler_repeats >= 60` — repeats count toward it), never at the
  renderer/tap level this acceptance test asserts. The cadence that WOULD
  present every frame (the production `pump_session`, wall-bounded) has no
  injectable clock seam (shell.rs audit F-004 — a src change the tests-only
  assignment may not make; FX6-A04's own risk #4 records the same).

Per the assignment's branch 3, this is a product-side present-path
shortfall, not drive calibration: STOP — no fix attempt, no relaxation.

## The calibration change (what actually changed)

The drive loop is UNTOUCHED (PUMP_BOUND 1024, the break-on-`ended`, the
per-pump tap drain — all as FX6-A04 wrote them): extra pumps are proven
inert (diagnosis §4) and the end-of-media drain already completed (§2).

The one edit is diagnostic-only: the failing assert's FAILURE MESSAGE now
appends the engine's own present-path accounting (`{accounting:?}`), so the
next central run prints `scheduler_presented` / `scheduler_repeats` /
`video_presented` / `video_refused` / `staging_frames` alongside the
shortfall. That single dump disambiguates the engine-side mechanism for the
owner: `scheduler_presented ≈ 18` → the superseded drop decimated the
selections (scheduler.rs:792-812); `scheduler_presented ≥ 60` with
`video_presented == 18` → the loss is between the scheduler and the renderer
(the W1-06 hold refusals, pipeline_host.rs:1686-1714). rustfmt
`--edition 2021` exit 0 (no let-chain rejections; binary at
/home/z/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustfmt —
RULES' PATH hint carries cargo but no rustfmt, matching FX6-R2).

## The assertion preserved

`stats.offered >= 60` — predicate, threshold (60), measured variable (the
tap mailbox's `offered`, presents AND repeats, exactly the original
message's "repeats ride the same lane, counted as presents") — byte-identical.
The message's original text is preserved verbatim and only appended to.
Nothing was relaxed, re-scoped or re-derived; the test still fails loudly on
the same shortfall, now with the evidence that routes the real fix.

## Risks & followups

1. **The wave's acceptance test will still fail centrally** — that is the
   finding, not a regression. The failing line now self-reports the engine
   accounting; route the number to the engine-side owner
   (apps/cli/src/pipeline_host.rs + crates/video-engine): the W1-06 note's
   own "obvious fix" (stash the CONVERTED frame in the hold) is gated on the
   Lead's pacing/backpressure decision, and surfacing `dropped_superseded`
   in `HeadlessAccounting` would make this drive class's decimation visible
   to every future test.
2. **The exact mechanism split (superseded drops vs hold refusals vs an
   underrun/re-anchor phasing not statically derivable)** is settled by the
   enriched message on the next run, not by this report; what IS proven here
   is the negative: engine-side, upstream of the lane, unreachable by pump
   count.
3. The machine-block title deviates from the assignment's suggested one
   ("the pacer needed end-of-media drain") because that mechanism is
   disproven by the run's own green asserts (:585-590) — kept truthful per
   the honesty contract.
4. Scratch: none outside this report and the one test-file edit (the fixture
   parse was read-only python over the committed fixture bytes).
