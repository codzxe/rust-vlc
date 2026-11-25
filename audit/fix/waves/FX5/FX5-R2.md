<!--FIXV1
agent: FX5-R2
title: remedial — 2 compile fixes: unused Demuxer import + CommandId deref
verdict: DONE
changed:
- apps/cli/tests/fx5_a07_seek_lane.rs
- crates/commands/tests/fx5_a02_catalog_audio_verbs.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- apps/cli/tests/fx5_a07_seek_lane.rs
- crates/commands/tests/fx5_a02_catalog_audio_verbs.rs
- crates/commands/src/catalog/ids.rs
-->

## Approach

Remedial batch: the wave's central compile left exactly two errors, both in
test files added by this wave. One-line, semantics-preserving fixes applied
verbatim as the compiler suggested; then a same-class scan of each file,
then rustfmt.

## Changes per file

- `apps/cli/tests/fx5_a07_seek_lane.rs` — removed the unused `Demuxer`
  identifier from the `use plugin_api::{...}` import (line 42). Verified
  genuinely unused first: `rg '\bDemuxer\b'` hits only the import line
  itself (the `SelectDemuxer`/`Mp4Demuxer` occurrences are substrings in
  doc comments and distinct identifiers). Import is now
  `use plugin_api::{DemuxEvent, SeekSupport};` — both still used.
- `crates/commands/tests/fx5_a02_catalog_audio_verbs.rs` — line 232:
  `assert_eq!(row.id, id, ...)` → `assert_eq!(row.id, *id, ...)`, message
  string unchanged. Loop context checked: `for (row, id) in
  data.iter().zip(CommandId::ALL)` — `CommandId::ALL` is
  `&'static [CommandId]` (ids.rs:57), so `zip` yields `&CommandId` and the
  deref is required; `row.id` is `CommandId`.

## Test design

None added — this batch is pure compile remediation of the wave's own new
test files; the existing test functions in both files already cover the
behavior and now compile cleanly.

## Acceptance vs FIX-PLAN

FIX-PLAN has no FX5-R2 row (grep: no FX5-R2/A07/A02 entries) — scope came
from the orchestrator's complete two-error compile list, and both cited
file:line pairs matched the actual code with no drift. Sanity scan of each
file for the same two error classes: every other imported identifier in
fx5_a07_seek_lane.rs is used in code, and the a02 file has no other
`row.id`/`zip(CommandId::ALL)` comparisons. rustfmt (edition 2021) ran
clean on both files and made no further changes.

## Risks & followups

None expected — both fixes are exactly the compiler's own suggestions,
no semantics or coverage change. Note for the orchestrator: the rustfmt
shim is absent from both `.cargo/bin` paths cited in RULES.md; the binary
lives at `/home/z/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustfmt`
(used directly, successfully).
