<!--FIXV1
agent: FX3-R1
title: Remedial batch — controls.rs AudioEffect import (4× E0599), boundaries.toml cpal allowlist row, ci.yml gate-job alsa-sys apt guard
verdict: DONE
changed:
- crates/audio-engine/src/controls.rs
- boundaries.toml
- .github/workflows/ci.yml
tests:
- python3 tomllib: boundaries.toml parses; [crates.audio-output].external_allowlist == ["cpal"], external_dev_allowlist == [], 39 crate entries
- python3 yaml.safe_load: ci.yml parses; 8 jobs; gate job 12 steps; ALSA apt step sits between the ffmpeg prereq step and "2. clippy" (the first cargo step); run block contains libasound2-dev + pkg-config + fail-closed pkg-config --exists alsa
- rg controls.rs:70 "use crate::effects::{AudioEffect, Balance, Eq3Band, Gain, Mute};" present
- rg controls.rs:229/:232/:235/:238 — the four process call sites byte-identical (self.gain/mute/balance/eq.process(planes)?;)
- rg crates/audio-output/Cargo.toml: cpal = { version = "=0.18.2", optional = true } in [dependencies] + backend-cpal = ["dep:cpal"] (the dep the new allowlist row permits)
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md (§3 F1.1 Route A; §8.3 contradiction rule)
- audit/fix/wave-FX3.tsv
- audit/fix/waves/FX3/FX3-A01.md (Risks 1 — the boundaries.toml allowlist gap)
- audit/fix/waves/FX3/FX3-A03.md (the controls.rs author's report)
- audit/fix/waves/FX3/FX3-A07.md (Risks 1 — the gate-job --all-features alsa-sys flag)
- audit/fix/waves/FX3/FX3-A02.md (paperwork landed; boundaries.toml explicitly left uncovered, Risks 4)
- crates/audio-engine/src/controls.rs
- crates/audio-engine/src/effects.rs (trait :63; impls Gain :166 / Mute :189 / Balance :248; eq.rs Eq3Band :325)
- crates/audio-engine/src/effects/eq.rs
- tools/xtask/src/cmd/check_architecture.rs (the EXTERNAL check that reads [crates.<n>].external_allowlist, :1394)
- crates/audio-output/Cargo.toml
- .github/workflows/ci.yml (in full: gate, tauri-check, audio-cpal-check lanes)
-->

# FX3-R1 — remedial batch for the FX3 wave (three orchestrator-flagged items)

## Approach

Three follow-ups from the wave's own risk registers, each a one-lane fix:
the central compile feedback on FX3-A03's new file (E0599 ×4 — the
`AudioEffect` trait implemented for `Gain`/`Mute`/`Balance`/`Eq3Band`
but not imported in `controls.rs`), FX3-A01's flagged arch-gate blocker
(`boundaries.toml` allowlist still `[]` for the now-admitted optional
cpal), and FX3-A07's flagged CI red-risk (the ubuntu `gate` job's
`--all-features` builds would die at alsa-sys's pkg-config probe for
`alsa.pc` unless the runner carries libasound2-dev). Grounding: read
RULES.md, FIX-PLAN §3 F1.1 Route A + §8.3, the wave TSV, and the four
sibling reports (A01/A02/A03/A07) before editing; verified the trait's
impl sites (`effects.rs:166/189/248`, `effects/eq.rs:325`), the xtask
EXTERNAL check that consumes the allowlist row
(`check_architecture.rs:1394`), and the optional manifest entry the row
permits (`cpal = { version = "=0.18.2", optional = true }`,
`backend-cpal = ["dep:cpal"]`). No git, no cargo (rustfmt binary absent
in this sandbox — noted in Risks 4; the one-line edit is trivially
conformant: single line, <100 cols, braces sorted).

## Changes per file

- `crates/audio-engine/src/controls.rs` — **one line, line 70**:
  `use crate::effects::{Balance, Eq3Band, Gain, Mute};` →
  `use crate::effects::{AudioEffect, Balance, Eq3Band, Gain, Mute};`
  (trait added to the existing brace list, alphabetical, the file's own
  import idiom; lib.rs:128 already re-exports it, but the module-path
  form matches the neighboring use-lines). **Zero logic change**: the
  four call sites (`:229 self.gain.process`, `:232 mute.process`,
  `:235 balance.process`, `:238 eq.process`) are byte-identical — this
  import alone resolves all four E0599s (method resolution requires the
  trait in scope; the compiler's own hint).
- `boundaries.toml` — `[crates.audio-output]`: `external_allowlist = []`
  → `external_allowlist = ["cpal"]` (single-line string array, the
  schema of every populated neighbor row, e.g. codecs' line 330). **Plus
  a 4-line amendment of the comment clause directly above that row** —
  "Std-only by C5: zero external dependencies" would contradict the row
  I am adding in the same entry (FIX-PLAN §8.3's rejected
  doc-vs-code-contradiction class): it now reads "Std-only by C5 in the
  default graph: the ONE external seam is the optional cpal behind
  `backend-cpal` (ADR-005 amended 2026-11-25, FX3-A01 — cpal =0.18.2,
  docs/dependencies/allowlist.md §3); default builds resolve nothing
  external. No unsafe." Disclosed as a judgment call beyond the literal
  row edit; trivially revertable if the orchestrator prefers the bare
  row.
- `.github/workflows/ci.yml` — one new step in the ubuntu `gate` job,
  inserted after the ffmpeg-prerequisite step and before "2. clippy"
  (the first cargo step): "Install the ALSA dev packages (alsa-sys
  guard for --all-features)" — `sudo apt-get update` +
  `apt-get install -y --no-install-recommends libasound2-dev
  pkg-config` + the fail-closed `pkg-config --exists alsa || { ::error::
  …; exit 1; }` assertion. Style mirrors the file's two prerequisite
  precedents: the gate job's own ffmpeg apt step (update+install,
  `--no-install-recommends`, no `if:` on prereq steps) and tauri-check's
  webkit2gtk install (multi-line continuation + the pkg-config
  post-assert). No `if: always()` (matching the ffmpeg prereq), no
  `continue-on-error`, nothing else in the job touched; the numbered
  full_gate.sh steps keep their numbers.

## Test design

The mission defines the tests as the parse checks; all executed and
green (the consolidated python3 script in the session transcript
re-derives every claim below):

1. **tomllib (item 2)**: `boundaries.toml` parses; audio-output entry =
   `external_allowlist ['cpal']`, `external_dev_allowlist []`, internal
   and dev dep lists and layer unchanged; 39 crate entries total. Also
   xtask-subset-compatible: the row is a single-line string array and
   only `#` comments changed around it (the checker's own accepted
   shapes, check_architecture.rs header).
2. **yaml.safe_load (item 3)**: ci.yml parses; 8 jobs (unchanged set);
   gate job now 12 steps; the ALSA step's index sits strictly between
   the ffmpeg prereq step and "2. clippy"; its run block contains
   `libasound2-dev`, `pkg-config`, `--no-install-recommends`, and the
   `pkg-config --exists alsa` guard; step count of every other job
   unchanged (lint 9 / schema 10 / config 9 / deny 6 / wasm 9 / tauri
   12 / cpal 8).
3. **rg (item 1)**: `controls.rs:70` carries the extended import;
   `:229/:232/:235/:238` are the exact original call-site lines. A
   whole-crate `rg '\.process\('` sweep confirms no other file calls the
   effect stages' trait methods without the trait in scope (pipeline.rs
   calls are the resampler's and AudioPipeline's inherent methods;
   effects.rs/eq.rs define/implement the trait in-scope; the
   fx3_a03/fx3_a04 integration tests use inherent methods) — controls.rs
   was the only E0599 site, matching the central compiler report 4×.
4. **Manifest cross-check**: audio-output's optional `cpal` entry +
   `backend-cpal` feature verified by rg — the allowlist row permits
   exactly the dep the arch gate's EXTERNAL check
   (check_architecture.rs:1394, table-key scan incl. optional entries)
   will now read.

For item 1, the regression catch for a missing trait import IS
compilation of `AudioControls::process`'s callers — the crate's existing
`fx3_a03_*` unit tests (controls.rs `mod tests`) and
`tests/fx3_a03_master_chain.rs` all invoke `process` and cannot compile
without the trait in scope; the orchestrator's central wave build is the
oracle (cargo is forbidden to me per RULES). No new test file added: a
new test would not exercise anything the existing four+eight do not
already pin, and the defect class is compile-time.

## Acceptance vs FIX-PLAN

- Mission item 1 (compile fix, import-only, call sites byte-identical):
  DONE — one use-line extended, four E0599 sites untouched.
- Mission item 2 (arch-gate allowlist row): DONE — `["cpal"]` in
  `[crates.audio-output].external_allowlist`, TOML-valid; this closes
  FX3-A01 Risks 1's "required companion edit, same wave commit" and
  FX3-A02 Risks 4's boundaries.toml gap (the doc 01 registry row noted
  there remains open — see Risks 3).
- Mission item 3 (CI alsa-sys guard): DONE — apt step (libasound2-dev +
  pkg-config) in the ubuntu gate job before the cargo steps, in-file
  style, fail-closed assert; this closes FX3-A07 Risks 1's ubuntu-leg
  half (the alternative it offered — cfg(windows)-scoping the dep in
  Cargo.toml — would have contradicted FX3-A01's landed cross-platform
  optional-dep design and is NOT taken).
- RULES definition of done: code/truth fixes + parse-check evidence +
  this report; no cargo/git/lockfile touched; no new TODO/FIXME/XXX
  markers; no `.unwrap()` introduced (no Rust logic touched at all).

## Risks & followups

1. **Gate-leg red until the runner proves it**: the apt guard is
   statically verified only (yaml parse + step order); the first
   ubuntu-latest run is the proof event — if `pkg-config --exists alsa`
   still fails after install (e.g. alsa.pc ships under a non-default
   pkg-config dir on the image), the fail-closed assert reddens loudly
   instead of pushing the failure downstream into cargo, which is the
   intended posture. FX3-A07 Risks 3's `--locked` sequencing note (lock
   refresh owed by the orchestrator for FX3-A01's manifest edit) still
   applies to gate steps 2-3.
2. `boundaries.toml`'s amended comment covers only the clause the new
   row directly contradicts; the same entry's older prose ("real
   backends land in platform-* per doc 01 §2.7 behind ADR-005") is now
   stale history after FX3-A01 landed the adapter in-crate — left as-is
   (scope discipline; the ADR-005 amendment FX3-A02 landed is the truth
   record). doc 01 §2.6's registry row still describes the
   L4/std-only/no-OS-lib posture — FX3-A02 Risks 4's open item, next
   docs wave's lane.
3. rustfmt binary is absent in this sandbox (exit 127 from
   /home/z/my-project/.cargo/bin), so the edited Rust file could not be
   rustfmt-checked locally; the single edited line is mechanically
   conformant (single-line use, alphabetized braces, <100 cols), and
   the orchestrator's central `cargo fmt --all -- --check` (lint job
   step 1) will re-derive it.
4. The arch gate (`cargo run -p xtask --locked -- check-architecture`,
   gate step 4) also needs the central lock refresh before it can run
   `--locked` at all; the allowlist row is the only manifest-shape
   change it needed from this batch.
