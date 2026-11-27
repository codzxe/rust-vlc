<!--FIXV1
agent: FX8-R1
title: remedial — the four first-vertical diagnostic reds are all legitimate-consequence recalibrations (pause/stop warm-up bound after FX7-A01; seek accounting after the pre-FX settle-priming drift the audit baseline recorded; mp4 trunc4's fatal arriving within the bounded advance, never missing) — no engine regression found, no engine change made
verdict: DONE
changed:
- tests/integration/first-vertical/src/scenarios.rs
- tests/integration/first-vertical/tests/recovery_matrix.rs
tests:
- journey_pause
- journey_stop
- journey_seek
- matrix_mp4_trunc4
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX8/FX8-R1.ASSIGNMENT.md
- audit/fix/waves/FX3/FX3-A06.md
- audit/fix/waves/FX4/FX4-A01.md
- audit/fix/waves/FX6/FX6-R3.md
- audit/fix/waves/FX6/FX6-R4.md
- audit/fix/waves/FX7/FX7-A01.md
- audit/fix/waves/FX7/FX7-A02.md
- audit/fix/waves/FX8/FX8-A08.md
- audit/runs/baseline/cargo-test-workspace.txt (:2764-2782, the open drift diff)
- tests/integration/first-vertical/evidence/STALE-TRANSCRIPTS.md
- tests/integration/first-vertical/evidence/transcripts/open.json
- tests/integration/first-vertical/evidence/transcripts/pause.json
- tests/integration/first-vertical/evidence/transcripts/seek.json
- tests/integration/first-vertical/src/harness.rs
- tests/integration/first-vertical/src/scenarios.rs
- tests/integration/first-vertical/tests/journey.rs
- tests/integration/first-vertical/tests/recovery_matrix.rs
- tests/integration/first-vertical/tests/evidence.rs
- tests/integration/first-vertical/tests/baselines_consistency.rs
- tests/integration/first-vertical/tests/common/mod.rs
- apps/cli/src/host.rs
- apps/cli/src/pipeline_host.rs
- crates/playback-engine/src/pipeline.rs
- crates/playback-engine/src/state.rs
- crates/demux/src/mp4/error.rs
- tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__trunc4.mp4.toml
-->

# FX8-R1 — the four diagnostic reds: all four recalibrated, no engine regression

## Dispatch context (honesty contract)

This is the remedial dispatch. When I started, the worktree already
carried the scenario/matrix recalibration edits (a prior interrupted
FX8-R1 run left them in the tree with no report; this report is the
first). Per the contract I did NOT trust them: every edit was re-derived
below against the evidence chain — the baseline drift diff's actual
direction (re-parsed from
`audit/runs/baseline/cargo-test-workspace.txt:2775-2776`: **fresh 30
accepted / 15 pushed, committed 94/16** — `assert_eq!(fresh, committed)`
at `tests/evidence.rs:47-53`, so left=fresh), the committed
transcripts' accountings, the engine constants (`RENDER_QUANTUM` 1470,
`AUDIO_PARK_FILL` 32 768, `TickBudget` 64 demux events / 16
packets-per-limb / starve-debounce 2), and the FX7-A01 pre-registered
consequence. One imprecision the prior edits carried was corrected (the
WARMUP_PUMPS bound comment: "< 0.77 s"/"< 1.54 s" was arithmetically
wrong; now the re-derivable 34 238 frames < 0.78 s / < 1.56 s).

## Approach

Four diagnostics, one decision rule each (regression vs legitimate
consequence), decided from code + recorded runs — never by scrubbing
evidence. No engine file was touched: every failure turned out to be a
test expectation pinned to pre-drift engine behavior (the seek count
and the trunc4 settle state — the drift the audit baseline itself
recorded on 2026-11-23, before any FX wave) or to W1-06-defect-era
pacing (pause/stop — FX7-A01's pre-registered Risks #1). The engine's
new behavior was verified CORRECT in each case (the deep questions the
assignment posed: the media legitimately ENDS inside a 64-pump advance
window — correct-by-design under the registered whole-ring `advance`
drain, with the production pacing decision kept on the register as doc
03 §8 / 3.B; the clock is NOT advancing too fast — it advances exactly
the ring's bounded fill; 154 is the committed `seek.json` transcript's
own `frames_accepted`, mirrored by the scenario constant, both stale
against the pre-FX settle-priming drift; trunc4's error lands LATER
(pacing), never missing — proven by the sibling rows' greens).

## Per-failure root cause

### 1. `journey_pause` — "the state is Paused": observed `Ended`, expected `Paused` → RECALIBRATED

**Diagnosis.** `scenario_pause` warmed up with `advance(64)` before
dispatching `playback.pause` (scenarios.rs, the pre-FX8-R1 code). The
engine change: FX7-A01 (commit ef57c31d) closed W1-06 — the window-full
hold now stashes the CONVERTED frame, so the render loop no longer
breaks early on the I420 upload refusals. Consequence (FX7-A01's own
report, Risks #1, verbatim: "the first-vertical `pause`/`stop`
scenarios' 64-pump warm-ups (scenarios.rs :407/:583) overshoot into
`Ended`"): the whole-ring-per-pump `advance_time` drain
(host.rs:714-748) now plays the 2.000 s fixture out to `Ended` in ~4-5
pumps (the old W1-06 defect braked it at ~16 ms/pump — the committed
`pause.json` shows 64 pumps landing at only 1 033 333 µs). 64 pumps
overshoot; the pause command on an `Ended` session cannot land, so the
state check observed `Ended`.

**Regression vs consequence:** legitimate consequence of FX7-A01,
pre-registered by its report. The clock is NOT advancing too fast —
per pump it advances at most the audio ring's park-fill bound
(`AUDIO_PARK_FILL` 32 768 frames + one packet's 1 470-frame overshoot =
34 238 frames < 0.78 s; the audio limb's decode park at
pipeline_host.rs:787-800 bounds the ring by construction), so the
advance verb's behavior is the registered design (doc 03 §8 / 3.B's
pacing decision, kept on the register — FX7-A01's rewritten comment).

**What changed.** A new `WARMUP_PUMPS = 2` bound (scenarios.rs:51-68)
replaces the raw `advance(64)` in `scenario_pause` (and `scenario_stop`
— same mechanism, see #2). Two pumps land the clock solidly mid-media
(~1.3 s of the 2.000 s played, ~0.7 s to go — derived: settle ring
22 050 frames + park-bound refills of 33 810 / 23 520 frames across the
two pumps), and two pumps (< 1.56 s) can never reach the 2.000 s end,
so the pause still freezes WHERE the media was, never past its end. The
scenario's five checks are unchanged (state `Paused`, played > 0, 256
paused ticks move the clock by zero, landing == played, trail
Playing→Paused).

**Expected new observed values (central re-run).** `journey_pause`
green: state `Paused`; `played > 0` (≈ 1 300 000 µs, the sample-counter
clock over 57 330 rendered frames at 44.1 kHz); `frozen - landing == 0`
after the 256 paused ticks (FX3-A06's counter clock is frozen by
`Effect::FreezeClock` — position_us reads the frozen counter, host.rs:
357-370); trail carries Playing→Paused.

### 2. `journey_stop` — "stop lands Stopped": observed `Ended`, expected `Stopped` → RECALIBRATED

**Diagnosis.** Identical mechanism to #1: `scenario_stop`'s 64-pump
warm-up played the fixture out to `Ended` before `playback.stop`
dispatched. FX7-A01 Risks #1 names this exact check ("'stop lands
Stopped'").

**Regression vs consequence:** legitimate consequence of FX7-A01 (same
pre-registration). `Stopped` is reachable from `Playing`/`Buffering`
(state.rs:476), and mid-media after two pumps the session is `Playing`
(no mid-run starvation: the starve signal needs
`DEFAULT_STARVE_DEBOUNCE_TICKS` = 2 consecutive low-watermark ticks
while the demuxer cannot refill — pipeline.rs:257, 2066 — and every
pump's poll refills the queues before the render drains the ring).

**What changed.** The same `WARMUP_PUMPS = 2` warm-up
(scenarios.rs:620-631), same bound derivation. All seven checks
unchanged.

**Expected new observed values.** `journey_stop` green: state
`Stopped` at ≈ the same ~1.3 s landing (frozen, nonzero);
`media.get-info` answers from the stopped session; `session.close`
completes → `Idle`; the post-close snapshot refuses `session.not-ready`.

### 3. `journey_seek` — "the decode accounting keeps the pre-seek deliveries": observed 90, expected 154 → RECALIBRATED

**Diagnosis.** Where 154 comes from: the committed `seek.json`
transcript's own `frames_accepted: 154` (evidence/transcripts/seek.json:
229) and the scenario constant mirroring it — 94 pre-seek decode-ahead
+ 60 post-seek. The 94 (and the sibling constants `scheduler_pushed` 46,
`scheduler_dropped_stale` 16) were calibrated to the 2026-11-15 engine:
the committed `open.json` records the same era (`frames_accepted` 94 /
`scheduler_pushed` 16). The engine's settle priming then drifted —
**before any FX wave**: the audit baseline's fresh run (2026-11-23,
pre-FX; `cargo-test-workspace.txt:2775-2776`, left side = fresh) shows
the open/settle window at **30 accepted / 15 pushed** — the audio
queue's 1 s duration cap parks the demuxer at 30 queued packets and
the producer-parked priming escape closes the buffering episode after
~1 poll (TickBudget demux_events 64 covers the 30-packet window in one
poll). That drift is the audit's own F2.2(a) finding ("the committed
evidence is stale... the committed transcripts understate current
behavior by ~3×" — STALE-TRANSCRIPTS.md); the journey binary never ran
against it because cargo's fail-fast aborted every earlier run at the
evidence binary (FX8-A08's finding). FX7-A01 did NOT move the settle
window (its change is downstream of the scheduler admission counters)
— the 2026-11-26 diagnostic confirms the pre-seek window is still 30
post-FX7 (90 − 60).

**Regression vs consequence:** legitimate consequence — not of an FX
wave, but of the pre-FX settle-priming drift the audit baseline already
recorded (cited above; the causal record is the baseline diff, which is
why no FX wave report is cited for this one). No engine regression: the
pipeline was NOT rebuilt (the session id survives, the check that
passed), the landing is still exactly 1 000 000 µs and the remainder
still plays to exactly 2 000 000 µs (both checks green in the diagnostic
run), and the post-seek decode is the full second half (30 video + 30
audio): 30 pre-seek + 60 post-seek = the observed 90.

**What changed.** The four accounting checks recalibrated
(scenarios.rs:547-591) with the full derivation in the comment:
`frames_accepted` 154 → **90** (15v+15a pre-seek + the landing's own
half); `scheduler_pushed` 46 → **45** (15 pre-seek pushes + the
landing's 30 video); `scheduler_dropped_stale` 16 → **15** (the
settle's 15 resident frames, presented never — the settle never ticks
the scheduler — purged wholesale at the seek's generation change);
`queue_published` stays **1** (the seek's mint).

**Expected new observed values.** `journey_seek` green: 90 / 45 / 15 /
1, landing 1 000 000 µs, end 2 000 000 µs, session id preserved, trail
in-pipeline. (90 is the diagnostic run's own observation; 45 and 15
are derived from the baseline's fresh open window (15 pushed) plus the
landing half (30) and its resident purge — [VERIFY-central].)

### 4. `matrix_mp4_trunc4` — "the settle lands in Error": observed `Playing`, expected `Error` → RECALIBRATED

**Diagnosis.** The row pinned the mp4 box walker's last-byte-cut FATAL
(`demux.mp4.truncated`, minted at crates/demux/src/mp4/error.rs:41-48)
to arrive during the settle. But trunc4's cut sits at offset 424 451 —
mdat's very tail (expectation TOML: "cut the final byte") — beyond the
queue caps' reach during the settle: the demuxer parks at the audio
queue's 1 s cap after ~30 packets (~the first half of the timeline),
the producer-parked escape closes the episode first, and the settle
lands `Playing` — the same pre-FX priming drift as #3 (the sibling rows
prove the split: trunc2's cut at 2 208 B and trunc3's at 213 878 B sit
within the settle's walk and both still land `Error` at the settle —
both GREEN in the diagnostic run). The row was authored against the
pre-drift engine (when the settle's priming walked deeper) and never
ran against the drifted engine until the diagnostic — the recovery
binary was aborted before executing in every prior run (fail-fast,
FX8-A08's finding).

**Regression vs consequence: the fatal lands LATER (pacing), never
missing.** Error handling is intact: within the bounded
`advance(run_pumps())` the decode drains the queues (16
packets/limb/poll), the demuxer resumes, and the walk reaches the cut —
the same lazy-walk mechanism that drives every `CleanEnded` row to
`Ended` and trunc2/trunc3 to their settle-time fatals — landing `Error`
via the same FATAL, well inside the 512-pump bound (the walk completes
in ~4-5 pumps) and the row's 5 s wall cap. Nothing recovers before the
fatal (the box walker's refusal precedes resync), so zero non-fatal
damage still holds.

**What changed.** `tests/recovery_matrix.rs` gains one outcome
vocabulary member, `Outcome::ErrorAfterAdvanceNoFlush`
(:94-103, :387-429): the open accepted; **the settle lands `Playing`**
(the producer-parked priming); **the bounded advance then lands
`Error`**; exactly one FATAL `demux.mp4.truncated`, demux-stage; zero
non-fatal damage. `matrix_mp4_trunc4` (:801-819) moves to it, its
comment records the walk, and the module doc table's trunc4 row (:25)
is updated to match. The trunc2/trunc3 rows and every other row are
untouched.

**Expected new observed values.** `matrix_mp4_trunc4` green: settled
state `Playing`, end state `Error`, one FATAL (demux-stage,
`demux.mp4.truncated`), 0 non-fatal, wall time within the 5 s cap.
([VERIFY-central] on the end-state split.)

## Test design

No new test files, deliberately: the assignment's decision rule
reserves `fx8_r1_<slug>` regression tests for ENGINE regressions, and
all four failures are legitimate-consequence recalibrations — the
existing tests (`journey_pause`, `journey_stop`, `journey_seek`,
`matrix_mp4_trunc4`) are the pins, now carrying the observable truth.
Each recalibrated check would catch its own drift by construction
(`Check::int`/`Check::same` are exact equality). The recalibrations
double as the regression tests for the classes they pin: a future
engine change that re-deepens the settle priming (90/45/15), re-brakes
the advance pump (pause/stop landing past `Ended`... i.e. failing to
reach it, or overshooting the 2-pump mid-media landing), or swallows
trunc4's FATAL (settle or advance never reaching `Error`) fails these
four tests loudly with the observed number in the message.

## Acceptance vs FIX-PLAN / the wave

- All four diagnostic reds root-caused, each decided on recorded
  evidence, each recalibrated to the observable truth with the causal
  record cited (FX7-A01 Risks #1 for pause/stop; the audit baseline's
  own drift diff for seek and trunc4's settle state; the sibling rows'
  greens for trunc4's still-arriving fatal).
- Integrity: no evidence/ artifact was hand-edited; the committed
  transcripts stay stale-but-declared until the orchestrator's central
  refresh (`scripts/quality/refresh_transcripts.sh` — FX8-A08's writer,
  whose refusal-on-red was working as designed and will now find the
  pre-flight green).
- Files stayed inside the assignment's editable set
  (first-vertical src/tests only); no engine change, no dependency
  change, no public API change.

## Risks & followups

1. **Not compiled or run here** (cargo forbidden; the central run is
   the oracle). The derived sub-values — seek's 45 and 15, the pause
   landing ≈ 1.3 s, trunc4's advance-then-`Error` — are code-derived
   (90 itself is the diagnostic run's observation). If any lands
   differently, the central run fails loudly with the observed number;
   the recalibration will then be corrected against the observed truth,
   never relaxed around it.
2. The pause/stop mid-media landing depends on the whole-ring `advance`
   drain staying registered (doc 03 §8 / 3.B's pacing decision). If the
   Lead lands steady-state pacing, `WARMUP_PUMPS`'s bound arithmetic
   must be re-derived (the comment records the derivation inputs:
   `AUDIO_PARK_FILL`, the 1 470-frame packet, the 44.1 kHz rate).
3. The transcript refresh (FX8-A08's central step 3) is now
  unblocked: after this wave's central green, the writer regenerates
  the 12 transcripts + final-frame.png + ledger and retires
  STALE-TRANSCRIPTS.md; the refreshed `pause.json`/`seek.json` will
  carry the new accountings (90 etc.), making the recalibration the
  committed evidence too.
4. Scratch: none — nothing outside the two test files and this report
   was written; the ASSIGNMENT.md scratch file is deleted per the
   dispatch instructions.
