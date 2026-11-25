<!--FIXV1
agent: FX4-R2
title: remedial — 11 cited compile errors in two FX4 test files (9 format strings + 1 moved-borrow + 1 missing trait import ×2 sites)
verdict: DONE
changed:
- apps/cli/tests/fx4_a02_host_audio_verbs.rs
- apps/cli/tests/fx4_a01_backend_route.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX4/FX4-R1.md
- apps/cli/tests/fx4_a02_host_audio_verbs.rs
- apps/cli/tests/fx4_a01_backend_route.rs
- crates/commands/src/catalog/command.rs
- crates/media-clock/src/clock.rs
- crates/media-clock/src/lib.rs
- apps/cli/src/pipeline_host.rs
- apps/cli/tests/fx4_audio_path.rs
- apps/cli/tests/sync_scenarios.rs
-->

# FX4-R2 — remedial: 11 cited compile errors in the two FX4 sibling test files

## The errors

The wave's central compile (all known errors, per the orchestrator) hit
two sibling agents' test files:

1. `apps/cli/tests/fx4_a02_host_audio_verbs.rs`
   - invalid format string `expected }, found [` — inline captures
     written as `{lines[0]}` … `{lines[3]}` (indexing inside braces is
     not valid Rust format syntax). The cite said "9 occurrences" over
     the line range 248–259; the actual class had **10 sites** in that
     range (lines 248, 249, 252–259 as shipped) — all 10 fixed. Minor
     cite drift, noted here per the honesty contract.
   - E0382 borrow of moved value `verb` at :159 — `dispatch(&mut host,
     verb)` moved the loop-owned `Command` by value, then the failure
     messages `"{verb:?}: {error}"` (:159) and the `unreachable!`
     (:163) borrowed it after the move.
2. `apps/cli/tests/fx4_a01_backend_route.rs`
   - E0599 no method `now` on `Arc<MediaClock>` at :164:34 and :202:30
     — `PlaybackClock` (which provides `now`) not in scope; the exact
     class FX4-R1 fixed in fx4_audio_path.rs.

## The fixes

**fx4_a02_host_audio_verbs.rs**
- Format strings: each index group is bound before the asserts with a
  shadowed `let response = lines[N];` (the file's existing shadowing
  style, e.g. the repeated `let volume = …`), and the messages use the
  plain inline capture `"{response}"`. Message content and assertion
  semantics are byte-identical to what `{lines[N]}` intended — the same
  `&str` via Display, no `{:?}` quoting drift.
- E0382: `dispatch(&mut host, verb.clone())` — one-token change;
  `Command` derives `Clone` (crates/commands/src/catalog/command.rs:67),
  so `verb` stays owned and both messages keep the live `{verb:?}`
  capture unchanged. Same behavior (an equal-by-value clone crosses the
  seam; `PartialEq` is derived).

**fx4_a01_backend_route.rs**
- Added `use media_clock::clock::PlaybackClock;` (now :23) in the
  import block. Path verified: `pub mod clock` at
  crates/media-clock/src/lib.rs:113, trait at clock.rs:192 with
  `fn now` at :198, impl for `MediaClock` at :772; the receiver
  `AudioSession.clock` is `pub clock: Arc<MediaClock>`
  (apps/cli/src/pipeline_host.rs:1905) — import required, and it is the
  identical convention the sibling `apps/cli/tests/fx4_audio_path.rs:42`
  already uses. Both cited sites (`session.clock.now().ticks`, now
  :165/:203) are covered by the single import.

Sanity scans of both files for the same classes: `rg '\{[a-zA-Z_][a-zA-Z0-9_]*\['`
over apps/cli/tests finds no remaining indexing placeholders; no other
borrow-after-move sites (the `(volume, code)` loop uses `Copy` types; the
machine-mode stdin borrow is block-scoped before `wait_with_output`);
no other trait-method-on-concrete-receiver calls (`.must()` covered by
the imported `testkit::Must`; all limb/device/host/controls calls resolve
inherently) — corroborated by the central compile's exhaustive two-file
error list.

rustfmt: `rustfmt` is absent from `/home/z/my-project/.cargo/bin`
(exit 127, the sandbox state FX4-R1 recorded); the toolchain binary
`/home/z/my-project/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustfmt
--edition 2021` was used on both files — exit 0, and `--check` re-runs
are idempotent (exit 0). Note: canonical formatting also re-sorted the
two files' import blocks (e.g. `commands::catalog::*` before
`commands::Command*`; `media_types::GenerationId` to the
`media_types` group's tail) — machine-formatted, zero code change.

## Risks

1. Main risk: the machine-mode test spawns the real `lumen` binary and
   its assertions now compile — if the sibling FX4-A02 implementation
   has runtime gaps, the central run may fail on the test's own merits
   (not my scope; the central run is the oracle).
2. The clone in the not-ready loop costs one `Command` clone per verb
   (three) — negligible; chosen to keep the failure messages verbatim.
3. rustfmt's import re-sorting makes my diff a few lines wider than the
   semantic change; if the wave's gate compares against
   another rustfmt version's ordering it could flag churn (the
   `--check` idempotence above is the counter-evidence).
