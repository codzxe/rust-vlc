<!--FIXV1
agent: FX4-R1
title: one-line remedial — PlaybackClock trait import in fx4_audio_path.rs (compile-fix of sibling FX4-A05's test file; no new test — the central wave build is the oracle)
verdict: DONE
changed:
- apps/cli/tests/fx4_audio_path.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX3/FX3-R2.md
- apps/cli/tests/fx4_audio_path.rs
- crates/media-clock/src/clock.rs
- crates/media-clock/src/lib.rs
- apps/cli/tests/sync_scenarios.rs
- apps/cli/src/pipeline_host.rs
- crates/audio-output/src/null.rs
- crates/audio-output/src/counter.rs
- apps/cli/src/host.rs
-->

# FX4-R1 — remedial: PlaybackClock trait import in fx4_audio_path.rs

## Approach

The wave's central compile produced exactly one error (E0599 at
`apps/cli/tests/fx4_audio_path.rs:292`): `session.clock.snapshot()`
calls the `PlaybackClock` trait method on a **concrete**
`Arc<MediaClock>` receiver, and the trait was not in scope — the same
class FX3-R2 handled (trait method used without the trait imported).
The fix is the compiler's own suggested one-liner,
`use media_clock::clock::PlaybackClock;`, placed in the file's
alphabetically-ordered import block (after the `lumen_cli::pipeline_host`
group, before `media_types::frame` — rustfmt-verified placement).

Path verification against source and siblings:
- `crates/media-clock/src/lib.rs:113` `pub mod clock;` and
  `:124` re-export `PlaybackClock` at the crate root — the compiler's
  `media_clock::clock::PlaybackClock` path is public and valid; the
  sibling convention (`apps/cli/tests/sync_scenarios.rs:33`,
  `use media_clock::{NANO_BASE, PlaybackClock};`) uses the root
  re-export of the same trait. Per the assignment's mandate I used the
  compiler's exact module path; both resolve to the identical trait
  (`crates/media-clock/src/clock.rs:192`).
- `snapshot` is indeed a trait method (`clock.rs:241`), impl for
  `MediaClock` at `:771/:825`; `session.clock` is `Arc<MediaClock>`
  (`apps/cli/src/pipeline_host.rs:1905`), so an import is required
  (no trait-object candidate applies).

Then the commanded sanity-scan of the same file for further
trait-in-scope hazards — every other method call in the file resolves
inherently or through an already-imported trait:
- `.must()` everywhere — `testkit::Must`, imported (`:47`).
- limb: `stage` (`pipeline_host.rs:884`), `written_frames` (`:806`),
  `audio_controls` (`:919`) — inherent on `AudioLimb`/`AudioSession`.
- device: `render` (`null.rs:311`), `sample_count` (`:360`),
  `buffered_frames` (`:365`), `stats` (`:372`) — inherent on
  `NullDevice` (the `AudioSink` impl at `null.rs:263` is the dyn path,
  not used here).
- `stats.frames/buffers/samples/bytes/silence_frames/data_frames` —
  inherent on `SinkStats` (`counter.rs:157-185`).
- host: `settle/state/position_us/advance` — inherent on `CliHost`
  (`host.rs:299/313/357/508`).
No other trait-on-concrete-receiver call sites exist; the central
compile's single-error fact corroborates. No further fixes needed —
nothing else of this class in the file.

## Changes per file

- `apps/cli/tests/fx4_audio_path.rs` — exactly one added line in the
  import block (now `:42`):
  `use media_clock::clock::PlaybackClock;`
  No other edit; no logic change; the error site `:293`
  (`session.clock.snapshot().position.ticks`) is untouched.

rustfmt: `rustfmt` is absent from `/home/z/my-project/.cargo/bin`
(exit 127, same sandbox state FX3-R2 recorded), but the toolchain
binary ran directly —
`/home/z/my-project/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustfmt
--edition 2021 apps/cli/tests/fx4_audio_path.rs` → exit 0, and
`--check` re-run is idempotent (exit 0). File is machine-formatted.

## Test design

None added, by assignment: this is a compile-time defect class (missing
trait import) in a sibling agent's (FX4-A05's) test file — the
regression oracle is the orchestrator's central wave build, the exact
FX3-R1/FX3-R2 precedent. The five `fx4_a05_*` test fns in the file are
the affected surface: they could not compile before and now stand to
run. Claim re-derivable: `rg -n 'PlaybackClock' apps/cli/tests/
fx4_audio_path.rs` → exactly two hits, the import (`:42`) and the
`snapshot()` call site (`:293`).

## Acceptance vs FIX-PLAN

- The cited error (E0599, trait not in scope) is fixed exactly as the
  compiler suggested and as scoped — DONE, minimal one-line diff,
  existing import style/order preserved.
- RULES hygiene: no git, no cargo/rustc builds (rustfmt only, on the
  one file I edited); no dependency edits; scratch none (no /tmp
  artifacts needed); no TODO/FIXME/XXX markers; no unwrap/expect added;
  `tests: []` is honest — no new test was authored.

## Risks & followups

1. Risk: none expected. The import is used (the `snapshot()` call at
   `:293`), so `deny(unused_imports)` is satisfied; the path is public
   and stable; zero behavior change (a scope fix only).
2. Cosmetic-only note: the sibling test convention imports
   `PlaybackClock` via the crate root re-export; this file now uses the
   compiler's module path. Both are first-class public paths of the same
   trait — no followup required.
