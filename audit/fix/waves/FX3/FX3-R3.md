<!--FIXV1
agent: FX3-R3
title: Restore the two PlaybackClock trait imports FX3-R2 wrongly removed — the trait is used via its methods (now/try_now/master/snapshot) on MediaClock values; both files were failing E0599
verdict: DONE
changed:
- crates/audio-output/tests/pcm_pipeline.rs
- crates/media-clock/tests/fx3_a06_counter_source.rs
tests:
- clock_synced_pump_delivers_the_exact_sine_stream
- fx3_a06_counter_advances_media_time_exactly_with_samples
- fx3_a06_stalled_counter_stalls_the_clock_without_drift
- fx3_a06_runtime_source_switch_is_clean
- fx3_a06_adopt_refreshes_the_wired_audio_slot
- fx3_a06_unreadable_adopted_source_parks_until_first_reading
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md (§1.4; no FX3-R3 row — assignment from the orchestrator's E0599 list)
- audit/fix/waves/FX3/FX3-R2.md (the removals being reverted, in its own words)
- audit/fix/waves/FX3/FX3-A06.md (author of fx3_a06_counter_source.rs)
- audit/files/crates/audio-output/tests/pcm_pipeline.rs.audit.md
- crates/audio-output/tests/pcm_pipeline.rs
- crates/media-clock/tests/fx3_a06_counter_source.rs
- crates/media-clock/src/clock.rs (trait def :192-:242; impl PlaybackClock for MediaClock :771; inherent impl :468-:613 carries no now/try_now/master/snapshot)
- crates/media-clock/src/lib.rs (pub use clock::{... PlaybackClock} at :124)
-->

# FX3-R3 — restore the PlaybackClock trait imports FX3-R2 deleted

## Approach

FX3-R2 removed `PlaybackClock` from two test files' `use media_clock::{…}`
groups, reasoning "rg shows the name nowhere else in the file". That check is
**insufficient for trait imports**: a trait is *used* through its methods.
Both files call `PlaybackClock` methods on `MediaClock` values, and those
methods exist ONLY on the trait — hence E0599 at pcm_pipeline.rs:94/:128 and
across fx3_a06_counter_source.rs:109-:316.

Grounding in the crate source (not the lint text):

- Trait `PlaybackClock` at clock.rs:192 defines exactly `now` (:198),
  `try_now` (:210), `master` (:213), `reanchor` (:224), `set_rate` (:237),
  `snapshot` (:241); re-exported `pub use clock::{… PlaybackClock}` in
  lib.rs:124.
- The **inherent** `impl MediaClock` (clock.rs:468-613: `audio_master`,
  `system_monotonic`, `pause`, `resume`, `is_paused`, `set_generation`,
  `anchor`, …) contains NONE of now/try_now/master/snapshot — so without the
  trait in scope every one of those calls is E0599 "no method named `now`
  found for struct `MediaClock`" (with rustc's note pointing at
  `PlaybackClock` as implemented-but-not-in-scope).
- clock.rs:190-191 documents the rule verbatim: "Object-safe; hold it as
  `Arc<dyn PlaybackClock>`. **Bring the trait into scope to call the methods
  on [`MediaClock`]**."
- Compiled precedent, rg over the tree: 6 sibling test files import
  `PlaybackClock` inside their media_clock groups and call the methods —
  e.g. `crates/audio-output/tests/clock_truth.rs:20`
  `use media_clock::{ClockMaster, ClockState, MediaClock, NANO_BASE,
  PlaybackClock};`, the exact shape restored at pcm_pipeline.rs:37. That file
  is baseline-green, proving the import is genuinely used via methods.

So the fix is a pure revert of R2's two deletions — one name re-inserted into
each existing import group, in the group's existing alphabetical order, at
the group position the file had before R2 (per R2's own record of the
pre-removal shapes). Nothing else touched.

## Lesson (binding for future lint sweeps)

A `use` of a TRAIT name is REQUIRED whenever trait methods are called on
values in the file, **even if the trait name itself never appears
textually** after the import line. A text-only `rg <TraitName>` therefore
cannot prove a trait import unused: `now`/`try_now`/`master`/`snapshot` on a
`MediaClock` value are uses of `PlaybackClock`. Lint sweeps must never delete
a trait import without checking **method usage** (are calls resolved by this
trait? are the receivers trait objects — where no import is needed — or
concrete/inherent types — where it is?). The safe order is: identify the
imported item's kind (trait/enum/fn/macro) BEFORE deciding "unused".

## Changes per file

- `crates/audio-output/tests/pcm_pipeline.rs` — :37 group restored to
  `use media_clock::{ClockMaster, MediaClock, NANO_BASE, PlaybackClock,
  SampleCounterSource};` (PlaybackClock re-inserted alphabetically between
  NANO_BASE and SampleCounterSource — the exact pre-R2 shape). Fixes
  `media.master()` (:94) and `media.now()` (:128).
- `crates/media-clock/tests/fx3_a06_counter_source.rs` — :30-:33 group
  restored to `… MonotonicSource, NANO_BASE, PlaybackClock,
  SampleCounterSource,` (same alphabetical re-insertion, the pre-R2 shape).
  Fixes the `clock.now()` / `clock.try_now()` / `clock.snapshot()` calls at
  :109-:316.

Both edits are single-name re-insertions into existing import statements;
no other line in either file was touched, and no src/ file was touched.

## Test design

No new test files or fns: the defect class is compile-time (missing trait
import → E0599), so the regression oracle is the orchestrator's central wave
build — the FX3-R1/R2 precedent ("no new tests for compile-time classes;
cargo is forbidden to me per RULES"). The six test fns above are the affected
surface: each calls at least one `PlaybackClock` method on a `MediaClock`
value and could not compile before this restore. All claims re-derivable:

- `rg -n 'PlaybackClock' crates/audio-output/tests/pcm_pipeline.rs
  crates/media-clock/tests/fx3_a06_counter_source.rs` → exactly the two
  restored import lines.
- `rg -n 'media\.(now|master)\(' crates/audio-output/tests/pcm_pipeline.rs`
  → :94 (`media.master()`), :128 (`media.now()`).
- `rg -n 'clock\.(now|try_now|snapshot)\('
  crates/media-clock/tests/fx3_a06_counter_source.rs` → 30+ hits, :109
  through :316.
- The restored imports are used (methods called), so the workspace
  `deny(unused_imports)` does NOT fire on them — unlike R2's deletions,
  which traded an unused-import lint for an E0599 hard error.

## Acceptance vs FIX-PLAN

- Assignment item 1 (pcm_pipeline.rs restore): DONE.
- Assignment item 2 (fx3_a06_counter_source.rs restore): DONE.
- "Do not touch anything else": obeyed — 2 files, 1 line each, no other
  edits; verified by rg that no other file under audit-workspace was written.
- RULES hygiene: no git, no cargo/rustc (rustfmt attempted per the allowed
  exception: exit 127, binary absent in this sandbox — same as FX3-R2
  Risks 3; the two edits are single-line import-group changes, mechanically
  conformant). No new TODO/FIXME/XXX; no `.unwrap()`/`.expect()` introduced;
  scratch work: none outside the report.

## Risks & followups

1. **R2's sibling verdict on clock_truth.rs:20 is the same misreading**
   (its Risks 1 claims that file's `PlaybackClock` is "imported and never
   used" and will redden the central build): clock_truth.rs calls the trait's
   methods on `MediaClock` values and is baseline-green — the import is
   used. The wave lead should discount that prediction; if clock_truth.rs
   reds, it will not be an unused-`PlaybackClock` lint.
2. rustfmt absent (exit 127): the two edited lines could not be
   machine-formatted locally; both are under 100 cols and follow the files'
   existing single-line / wrapped group styles — the central
   `cargo fmt --all -- --check` re-derives it.
3. Residual class risk: any *future* "unused import" sweep that decides by
   text-only name search will recommit this bug on any trait used via
   methods (the Lesson above is the guard). R2's other removals
   (fx3_a01's `AudioBackend`/`AudioSink`/`SampleCounterSource`) were
   grounded in trait-object-candidate resolution — different, correct
   reasoning — and were NOT touched by this batch.
