# AUDIT: crates/video-engine/src/test_support.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 193 / 12 items |
| File verdict | REAL — cfg(test)-gated unit-test fixtures: virtual video clock (monotonic-safe), synthetic frames, recording presenter with scripted failure; never leaks into the public API |
| Reachable from shipped entry point? | NO by construction — `#[cfg(test)] mod test_support;` (lib.rs:181-182), so the release binary carries none of it (verified) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

Correct test fixtures. `VirtualVideoClock` wraps the real
`MediaClock::system_monotonic` over a settable atomic source — the
audio-less playback configuration, exactly what the video scheduler
runs under — and its `set_ns` uses `fetch_max` so a test cannot
accidentally inject a backwards jump (a source discontinuity) when it
meant to position the clock (:62-67, the doc comment names the hazard).
`frame`/`frame_stream_base`/`frame_missing` cover the three stamp
shapes the scheduler must handle (present in the clock's base, present
in an arbitrary stream base, missing). `RecordingPresenter` records
slots and generations in delivery order and can refuse a scripted
index with a structured typed error. The module is cfg(test)-gated
(verified at lib.rs:181-182), honoring the ARCH-7 no-dev-types-in-API
claim.

## 2. What actually works (verified)

— Gate verified: `#[cfg(test)] mod test_support;` — the release
binary carries none of this.
— The monotonic-safety choice (`fetch_max`) prevents a whole class of
accidental test invalidity.
— `presenter_failure` is a structured typed error (stage Present,
Device/Display/Busy, code `video.presenter.refused`) — failure tests
fail through the real error path.
— `RecordingPresenter::present` fails BEFORE logging (:184-188), so a
refused request never appears in the log — the recording is exactly
what was presented.

## 3. Findings

### F-001 [S4] Duplicated by design, but the duplication is between cfg(test) unit fixtures and the integration copy with no shared pin
Where: test_support.rs:1-3 vs crates/video-engine/tests/common/mod.rs

Code:
```
//! Shared fixtures for the crate's unit tests (cfg(test)-gated; the
//! integration suites under `tests/` carry their own copy — dev-only
//! types can never leak into the public API, ARCH-7).
```
What is wrong: the duplication is forced by Rust's cfg(test)/integration
split and is honestly documented — but nothing pins the two copies
against drift (e.g., a fixture-behavior change here that silently
invalidates the integration suite's assumptions). This is the
standard cost of the pattern; a one-line cross-reference in both
files ("change both") is absent.
Confidence: HIGH — Verification: read + the tests/common copy exists
(next audit).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FakeMonotonic` | 26-32 | REAL | — | settable atomic source |
| `VirtualVideoClock::{new,clock,set_ns,advance_ns,now_ns,set_generation}` | 39-84 | REAL | — | fetch_max monotonic safety; real MediaClock under it |
| `frame`/`frame_stream_base`/`frame_missing` | 92-119 | REAL | — | the three stamp shapes |
| `request` | 122-128 | REAL | — | |
| `presenter_failure` | 136-145 | REAL | — | structured typed error |
| `RecordingPresenter::{new,failing_at,slots_seen,gens_seen,present}` | 150-193 | REAL | — | fail-before-log; delivery-order records |

## 5. Test honesty

Fixture module (no tests of its own); consumed by the crate's unit
tests (sink/policy — other agents' files where not in my list).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cfg(test)-gated … dev-only types can never leak into the public API" | :1-3 | REAL | lib.rs:181-182 |
| "it is the audio-less playback configuration (doc 03 §4.7 d)" | :36-38 | REAL | matches media-clock's SystemMonotonic fallback (lib.rs audit) |

## 7. Dead / duplicated / unreachable code

`#![allow(dead_code)]` covers subset-use; the integration copy
duplication (F-001).

## 8. Dependencies & flags

`media_clock`, `media_types`, `testkit`, `crate::sink`. No feature
gates; cfg(test) only.

## 9. Open questions for the next agent

- Wave 6 (video-engine owners): do the unit suites and the
  integration suite (tests/common copy) ever disagree on fixture
  semantics (e.g., missing-stamp behavior)? A drift here means the
  two suites test different things.
