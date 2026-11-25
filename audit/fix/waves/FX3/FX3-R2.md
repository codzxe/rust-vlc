<!--FIXV1
agent: FX3-R2
title: Remedial batch — FX3 test-file lint fallout: 5 cited compile errors fixed (4 unused imports relocated/removed, 1 format placeholder) + 2 same-class sweep finds (unused PlaybackClock in pcm_pipeline.rs and fx3_a06); zero logic changes
verdict: DONE
changed:
- crates/audio-output/tests/fx3_a01_admission.rs
- crates/audio-output/tests/pcm_pipeline.rs
- crates/audio-engine/tests/fx3_a03_master_chain.rs
- crates/media-clock/tests/fx3_a06_counter_source.rs
tests:
- fx3_a01_admission_flag_is_flipped_true
- fx3_a01_default_build_reports_no_link_and_falls_back_to_null
- fx3_a01_default_builtin_table_is_byte_compatible_with_pre_admission
- fx3_a01_null_fallback_session_roundtrips_with_typed_backpressure
- fx3_a01_cpal_build_reports_the_link
- fx3_a01_cpal_backend_enumerates_honestly_and_refuses_unknown_ids
- fx3_a01_cpal_opens_plays_pauses_and_resumes_a_real_device
- clock_synced_pump_delivers_the_exact_sine_stream
- drop_policy_bounds_memory_and_keeps_the_prefix
- backpressure_is_typed_and_clears_as_the_clock_advances
- the_cpal_seam_is_admitted_and_constructs_a_backend
- fx3_a03_volume_changes_output_amplitude
- fx3_a03_mute_zeroes_output
- fx3_a03_eq3_attenuates_sine_at_band_center
- fx3_a03_balance_shifts_l_r_energy
- fx3_a03_eq_updates_apply_between_quanta_without_rebuild
- fx3_a03_uncontrolled_chain_is_bit_exact_unity
- fx3_a03_control_validation_is_typed
- fx3_a03_getters_report_the_live_config
- fx3_a06_counter_advances_media_time_exactly_with_samples
- fx3_a06_stalled_counter_stalls_the_clock_without_drift
- fx3_a06_runtime_source_switch_is_clean
- fx3_a06_adopt_refreshes_the_wired_audio_slot
- fx3_a06_unreadable_adopted_source_parks_until_first_reading
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md (§0-§1 incl. §1.4 definition-of-done; no FX3-R2 row — assignment came from the orchestrator's compile-error list)
- audit/fix/waves/FX3/FX3-R1.md (the remedial-batch precedent: no new tests for compile-time classes, central build is the oracle)
- audit/fix/waves/FX3/FX3-A01.md (author of fx3_a01_admission.rs + pcm_pipeline.rs edits; Risks 2 — the feature-on lane runs cargo check, not test)
- crates/audio-output/tests/fx3_a01_admission.rs
- crates/audio-output/tests/pcm_pipeline.rs
- crates/audio-output/tests/fx3_a05_null_stats.rs
- crates/audio-output/tests/clock_truth.rs (the compiled convention for trait imports on concrete backends)
- crates/audio-engine/tests/fx3_a03_master_chain.rs
- crates/audio-engine/tests/fx3_a04_rate_resampler.rs
- crates/media-clock/tests/fx3_a06_counter_source.rs
- crates/audio-output/src/cpal.rs (cpal_backend signature; CpalBackend inherent vs trait methods)
- crates/audio-output/src/device.rs (AudioBackend trait surface)
- crates/audio-engine/src/buffer.rs (AudioSink trait surface)
- crates/audio-output/src/null.rs (inherent NullBackend/NullSink/NullDevice methods)
- crates/audio-output/src/lib.rs (re-exports)
- crates/audio-output/src/counter.rs (SinkStats methods, for the a05 sweep)
- crates/audio-output/src/render.rs (RenderStats derives/fields, for the a05 sweep)
- crates/audio-engine/src/resample.rs (RateResampler API, for the a04 sweep)
- crates/media-clock/src/clock.rs (audio_master signature; PlaybackClock trait)
- crates/media-clock/src/backends.rs (adopt_sample_source/ClockBackends/MasterSwitch/MasterTarget/AdoptSourceError)
- crates/media-clock/src/lib.rs (re-exports)
- crates/media-types/src/time.rs (Rate::ONE, for the a06 sweep)
- Cargo.toml ([workspace.lints.rust] unused_imports = deny — applies to test targets)
-->

# FX3-R2 — remedial batch: FX3 wave test-file lint fallout

## Approach

Five cited compile errors (four `deny(unused_imports)` hits across
fx3_a01_admission.rs / pcm_pipeline.rs, one positional-`{}`-without-
argument in fx3_a03_master_chain.rs:221), then the commanded sweep of
the sibling files (fx3_a05, fx3_a04, fx3_a06) for the same classes.
Every remove-or-keep decision was grounded in the crate source, not the
lint text alone, because the decisive fact is *how each call resolves*:

- The default-build tests in fx3_a01 operate on
  `audio_output::cpal::cpal_backend()`'s `Box<dyn AudioBackend>`
  (cpal.rs:597/623) and on the `Arc<dyn SampleCounterSource>` its
  `clock_source()` hands back (device.rs:128). Method calls on a
  trait object resolve via **trait-object candidates** (the trait's
  own plus supertrait methods) — no trait import needed. That is
  exactly why the compiler flagged `AudioBackend` (fx3_a01:30),
  `AudioSink` (:28) and `SampleCounterSource` (:32) unused in the
  default build, and why `pcm_pipeline.rs:33` (`AudioBackend`) was
  flagged while `SampleCounterSource` in the same file's :38 group
  was not (it is used there as a type annotation, :88/:201).
- The feature-on lane (`backend_cpal_lane`, `#[cfg(all(test, feature
  = "backend-cpal"))]`) instead calls trait methods on the CONCRETE
  `CpalBackend` — and there the defining trait must be in scope. The
  repo's own compiled precedent is `tests/clock_truth.rs`: top-level
  `use audio_output::{AudioBackend, …}` for
  open/pause/resume/clock_source on the concrete `NullBackend`, plus
  a *local* `use audio_engine::AudioSink;` scoped exactly where
  `backend.write(...)` is called (`:67-71`). `CpalBackend`'s
  inherent surface (cpal.rs:281-355: new/is_open/stream_errors/
  underruns/sample_count/buffered_frames/written_frames) does not
  contain `enumerate`/`open`/`write`/`pause`/`resume`/`clock_source`
  — all of those are trait methods (device.rs:89-133, buffer.rs:49-63).

So the correct fix is not blind deletion: `AudioSink` and
`AudioBackend` were **relocated into the cfg'd module** (where they
are genuinely needed), while `SampleCounterSource` is unused in BOTH
configurations and was deleted outright. pcm_pipeline's
`AudioBackend` (whole statement unused, cited) is deleted; the same
file's `PlaybackClock` inside the :38 import group is a partial-group
unused import of the identical class — verified by rg (no occurrence
outside the import line) — and would redden the central build just
the same, so it was removed from the group.

For fx3_a03:221, the assertion's intent (its own message and its
siblings' style at :176-186/:240-245) is to report the measured L/R
amplitude ratio against the pan-law expectation: the expression is
`l / r`, the message now carries it as `{ratio}` via a new `let ratio
= l / r;` binding — the file's established inline-capture idiom.

rustfmt is absent in this sandbox (exit 127, same as FX3-R1 Risks 3);
all edits are mechanically conformant (single-line use statements,
4-space module indent, no line over 78 cols).

## Changes per file

- `crates/audio-output/tests/fx3_a01_admission.rs`
  1. Top-level import block: removed `use audio_engine::AudioSink;`
     (was :28), `use audio_output::AudioBackend;` (was :30) and
     `use media_clock::SampleCounterSource;` (was :32) — the three
     cited errors. All remaining imports are used in the default
     build (AudioBuffer, AUDIO_ADR005_ADMITTED, frame/packet/time
     types, Must — rg-verified).
  2. Inside `mod backend_cpal_lane` (after `use super::*;`): added
     `use audio_engine::AudioSink;` and `use audio_output::AudioBackend;`
     with a 6-line comment recording why they live there (clock_truth.rs
     convention; trait-object candidates need no import). No lane body
     change; no behavior change anywhere.
- `crates/audio-output/tests/pcm_pipeline.rs`
  1. Removed `use audio_output::AudioBackend;` (was :33 — cited
     error 4; the only `dyn` receiver uses are
     `cpal_backend().must().enumerate()`, trait-object candidates).
  2. `use media_clock::{ClockMaster, MediaClock, NANO_BASE,
     PlaybackClock, SampleCounterSource};` → the same group without
     `PlaybackClock` (unused: rg shows the name nowhere else in the
     file; `MediaClock::audio_master` takes
     `Arc<dyn SampleCounterSource>` directly, clock.rs:479-483, and
     `PcmPump::attach_clock` likewise, pump.rs:135).
- `crates/audio-engine/tests/fx3_a03_master_chain.rs` — in
  `fx3_a03_balance_shifts_l_r_energy`, inserted `let ratio = l / r;`
  after the `want` binding and changed the assertion to
  `(ratio - want).abs() < 1e-4` with message
  `"L/R split {ratio} vs the pan law's {want}"` (was
  `"L/R split {} vs the pan law's {want}"` — the cited :221
  positional-placeholder error). Identical predicate, identical
  message intent, now with the argument actually supplied.
- `crates/media-clock/tests/fx3_a06_counter_source.rs` — sweep find:
  dropped `PlaybackClock` from the `use media_clock::{…}` group
  (was :30-33). It is imported and never referenced (rg over the
  file: the only hit was the import line); every remaining name in
  the group is used (AdoptSourceError :232-237, ClockBackends :251,
  ClockMaster :202/…, MasterSwitch :268-281, MasterTarget :269/:276,
  MediaClock throughout, MonotonicSource :86, NANO_BASE :93,
  SampleCounterSource :72-79).

Sweep verdicts for the untouched files: `fx3_a04_rate_resampler.rs`
CLEAN (3 imports, each used; every format string is an inline
`{name}` capture with in-scope bindings; every API —
`RateResampler::{new, rates, is_passthrough, reset, planes, process}`
— verified in resample.rs:398-475); `fx3_a05_null_stats.rs` CLEAN
(`AudioSink` needed for `backend.write` on the concrete NullBackend
and `AudioBackend` for open/pause/resume — the same clock_truth.rs
convention; SinkStats/RenderStats/null.rs APIs all verified; no
placeholder/Debug-format defects).

## Test design

No new test files or fns: the defect class is compile-time (unused
imports, a format-string arity error) — the regression oracle is the
orchestrator's central wave build itself, exactly the FX3-R1 precedent
("the defect class is compile-time… cargo is forbidden to me per
RULES"). The 24 test fns listed above are the affected surface: they
could not compile before this batch and now stand to run. Claims are
re-derivable by rg:

- `rg -n 'PlaybackClock' crates/audio-output/tests/pcm_pipeline.rs
  crates/media-clock/tests/fx3_a06_counter_source.rs` → no hits.
- `rg -n 'use audio_(engine|output)::(AudioSink|AudioBackend)'
  crates/audio-output/tests/fx3_a01_admission.rs` → exactly the two
  lines inside `backend_cpal_lane`.
- `rg -n '\{\}' crates/audio-engine/tests/fx3_a03_master_chain.rs` →
  no hits; `:219` is `let ratio = l / r;`, `:222` reads
  `"L/R split {ratio} vs the pan law's {want}"`.
- `rg -n 'SampleCounterSource' crates/audio-output/tests/
  fx3_a01_admission.rs` → the ONLY hit is the explanatory comment
  inside the lane (import and call sites gone; no call site needed
  the trait — trait-object candidates).
- fx3_a01's lane calls on the concrete CpalBackend
  (enumerate/open/write/pause/resume/clock_source) are the compile
  pin that the relocated imports exist to serve.

## Acceptance vs FIX-PLAN

- Cited item 1 (`fx3_a01:30 AudioBackend`), 2 (`:28 AudioSink`),
  3 (`:32 SampleCounterSource`): DONE — removed from the top level
  where the compiler proved them unused, with items 1-2 relocated
  into the only module whose calls need them (the drift note: the
  cited lines were 28/30/32; the file now carries the imports at
  :139-:140 inside the cfg'd module).
- Cited item 4 (`pcm_pipeline:33 AudioBackend`): DONE — deleted.
- Cited item 5 (`fx3_a03:221` positional `{}`): DONE — argument
  supplied per the assertion's intent (measured L/R ratio), the
  file's own message idiom.
- Commanded sweep (a05/a04/a06, four classes): a06 one unused-import
  fix (DONE); a05 and a04 CLEAN, untouched. Two additional same-class
  instances were found and fixed inside files already in the
  assignment (pcm_pipeline's `PlaybackClock` — beyond the 5-item
  list, disclosed here; a06's `PlaybackClock` — the sweep's own
  mandate). The orchestrator's cited list was demonstrably partial:
  a06's `PlaybackClock` was not cited yet is the same denied lint.
- RULES hygiene: no git, no cargo (rustfmt attempted, binary absent —
  exit 127, as FX3-R1 recorded); no src/ file touched; no new
  TODO/FIXME/XXX markers; no `.unwrap()`/`.expect()` introduced; the
  single behavioral edit (a03) adds one local binding and a format
  argument.

## Risks & followups

1. **Pre-FX3 file with the same latent lint, NOT mine to fix**:
   `crates/audio-output/tests/clock_truth.rs:20` imports
   `PlaybackClock` from the media_clock group and never uses it
   (rg: the file's only hit is the import line; it predates this
   wave, 2.C3). The central build will redden on it under the same
   `deny(unused_imports)` unless the wave lead scopes it to the next
   remedial line. callback_budget.rs was checked and is clean
   (its `SampleCounterSource` is used as a type annotation, :175/:371).
2. The relocated lane imports are verified by convention and source
   reading only: nothing currently compiles `backend_cpal_lane`
   (FX3-A07's lane runs `cargo check --features backend-cpal` — lib
   target only, per FX3-A01 Risks 2), so the first
   `cargo test -p audio-output --features backend-cpal` remains the
   lane's real compile oracle.
3. rustfmt absent (exit 127, /home/z/my-project/.cargo/bin): the
   edited files could not be machine-formatted locally; the edits are
   single-line use statements / one 6-line comment / one two-line
   assert rewrite, mechanically conformant — the central
   `cargo fmt --all -- --check` re-derives it.
4. a03's fixed message changes failure-path output only; the green
   path is bit-identical (predicate unchanged, `ratio == l / r`).
