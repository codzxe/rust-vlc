# AUDIT: crates/media-clock/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 143 / 1 production fn (module root + re-exports + doctest) |
| File verdict | REAL — module wiring, one defensive helper, one executable doctest that exercises the clock contract |
| Reachable from shipped entry point? | YES: `MediaClock` is constructed by the CLI host for every session (`open_audio_session` → `MediaClock::audio_master`, apps/cli/src/pipeline_host.rs:1609-1612; `wall_clock` → `MediaClock::system_monotonic`, pipeline_host.rs:1632-1638) — the crate is the engine's timing spine |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

A module root: 75 lines of honest design docs, six child modules
re-exported, one `pub(crate)` helper. The docs state the wave-3
critical fact plainly — audio is master, `now()` derives from the audio
output's sample counter, "never from a wall clock", no thread of its
own, and the clock "observes; it never drives the audio device" —
which is exactly why the W00 baseline freezes at 0.000000 s: the
sample counter only moves when a host renders, and no shipped host
renders between commands (see the pipeline.rs audit, F-001). The one
helper (`validate_rate`, :141-143) re-validates `Rate` at mutation
boundaries because `Rate`'s public fields allow bypassing `Rate::new`
— a real hole plugged defensively. The doctest (:66-102) is a genuine
executable contract check.

## 2. What actually works (verified)

— Re-exports match child modules (clock.rs, source.rs, backends.rs,
drift.rs, mapping.rs, sync.rs all exist, 4 991 loc total — assigned to
other agents; names re-exported here all appear in those files).
— `validate_rate` has two real callers at the clock's rate-mutation
boundaries (clock.rs:522, clock.rs:808) — not dead.
— The doctest constructs `MediaClock::audio_master`, advances a
counter, asserts `now() == 1 s`, pauses, advances the counter again,
asserts the position frozen, resumes, asserts continuity — it would
fail if the pause/resume epoch logic broke. Not run by me (no cargo);
W00's workspace run (doctests included, 1 751 pass) is the evidence.
UNVERIFIED individually.

## 3. Findings

### F-001 [S4] The "Phase-3 hardening" deferrals are load-bearing for the shipped product but read as footnotes
Where: lib.rs:36-38 (and :50-52)

Code:
```
//! non-blocking ([`source`] contract). The lock-free / seqlock read path
//! is Phase-3 hardening; v0's short lock satisfies the "never held
//! across I/O" clause and keeps all arithmetic checked and panic-free.
```
What is wrong: not a code defect — a documentation-priority defect.
The same paragraph pattern ("Phase 3", "[PROPOSAL]", "later card")
appears at the clock's concurrency model, the drift limiter's number,
and (per the audio-pipeline audit) the arming of the drift governor.
A reader skimming the crate docs would take a finished timing stack;
the deferral list is scattered rather than consolidated, and several
of the deferred items are precisely the ones whose absence W00
measured (frozen clock without pumping).
What correct looks like: one consolidated KNOWN-GAPS block at the
crate root listing every deferred seam and its card.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `validate_rate` | 141-143 | REAL | — | mirrors `Rate::new` (rejects zero components, reduces); called clock.rs:522/:808 |
| module docs doctest | 66-102 | REAL | — | executable pause/resume/now contract |
| re-exports | 109-129 | REAL | — | all names exist in child modules |

## 5. Test honesty

The doctest asserts exact nanosecond positions after counter
advances and across pause/resume — a real behavioral check, not a
smoke. The child modules carry their own suites (not this file's
scope; other agents).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "audio is master by default … `now()` from the audio output's sample counter" | :7-9 | REAL | pipeline_host.rs:1609-1612 constructs `audio_master(sink.clock_source(), …)`; clock.rs implements (other agent) |
| "When a medium has no audio … falls back explicitly to SystemMonotonic" | :9-12 | REAL | pipeline_host.rs:1632-1638 `wall_clock()` → `system_monotonic` |
| "No dedicated thread" | :33 | REAL | no `thread::` in the crate (Grep); consistent with the engine's host-driven model |
| "The clock observes; it never drives the audio device" | :106-107 | REAL | and it is exactly why nothing moves without a host pump (W00 baseline) |
| "monotonic — never backwards, equal at worst" | :42-43 | UNVERIFIED here | implemented in clock.rs (not my file); the doctest covers only the frozen-pause case |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

`media_types::time::Rate`. No feature gates. media-clock is a
workspace member and is linked into the shipped CLI (the host imports
`MediaClock, MonotonicSource, PlaybackClock`, apps/cli/src/host.rs:87).

## 9. Open questions for the next agent

- Wave 3 A02 (clock.rs, backends.rs — 3 295 loc): verify the
  monotonicity claim (`now()` never backwards within an epoch) against
  the implementation, and whether `ClockBackends`' video-master /
  master-loss chain has any production caller or is another
  built-but-unwired surface (the audio-engine drift-governor precedent
  suggests checking).
