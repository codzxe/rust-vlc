<!--FIXV1
agent: FX4-R3
title: report hygiene — FX4-R1/R2 machine blocks: rg-research targets now bare evidence_read: paths (parentheticals stripped; changed: lists already exact)
verdict: DONE
changed:
- audit/fix/waves/FX4/FX4-R1.md
- audit/fix/waves/FX4/FX4-R2.md
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX4/FX4-R1.md
- audit/fix/waves/FX4/FX4-R2.md
-->

# FX4-R3 — report hygiene: bare machine-block list paths in FX4-R1/R2

## Approach

Paperwork-only assignment: the wave gate parses every `changed:` /
`evidence_read:` machine-block line as a literal repo-relative path, so
entries carrying parenthetical rg-research descriptions (e.g.
`- apps/cli/tests/sync_scenarios.rs (rg — the sibling PlaybackClock import
convention)`) fail its existence check.

Drift note (honesty contract): the assignment described the annotated
rg-target lines as sitting inside `changed:`. In both files they actually
sat inside `evidence_read:` — both `changed:` lists were already exact
(R1: only `apps/cli/tests/fx4_audio_path.rs`; R2: only
`apps/cli/tests/fx4_a02_host_audio_verbs.rs` +
`apps/cli/tests/fx4_a01_backend_route.rs`). The real defect was therefore
"list entries are not bare paths", and the fix is to strip the
parenthetical annotations so every list line is a bare repo path — the end
state is exactly the one the assignment specified (changed: = only the
really-changed files; rg-research targets present as bare paths in
evidence_read:, existing evidence_read entries kept).

Every path remaining in both blocks was existence-verified by listing the
containing directories: apps/cli/tests, apps/cli/src,
crates/media-clock/src, crates/audio-output/src,
crates/commands/src/catalog, audit/fix/waves/FX3, audit/fix/waves/FX4.

## Changes per file

- `audit/fix/waves/FX4/FX4-R1.md` — machine block only: five
  `evidence_read:` lines lost their `(rg — …)` annotations
  (apps/cli/tests/sync_scenarios.rs, apps/cli/src/pipeline_host.rs,
  crates/audio-output/src/null.rs, crates/audio-output/src/counter.rs,
  apps/cli/src/host.rs — now bare paths). `changed:` (one entry),
  agent/title/verdict/tests untouched; human prose untouched.
- `audit/fix/waves/FX4/FX4-R2.md` — machine block only: six
  `evidence_read:` lines lost their `(rg — …)` annotations
  (crates/commands/src/catalog/command.rs, crates/media-clock/src/clock.rs,
  crates/media-clock/src/lib.rs, apps/cli/src/pipeline_host.rs,
  apps/cli/tests/fx4_audio_path.rs, apps/cli/tests/sync_scenarios.rs —
  now bare paths). `changed:` (two entries), agent/title/verdict/tests
  untouched; human prose untouched.

Re-derivable: `rg -n 'rg — ' audit/fix/waves/FX4/FX4-R1.md
audit/fix/waves/FX4/FX4-R2.md` → no hits inside the machine blocks
(remaining hits, if any, are in the untouched human prose only).

## Test design

None — no executable surface. This is a report-metadata text edit; the
gate's own path-existence parse over the two machine blocks is the
oracle. `tests: []` is honest.

## Acceptance vs FIX-PLAN

- Scope held exactly: only the two named report files touched (plus this
  report); zero code files, zero dependency files, no git, no cargo.
- All eleven distinct paths now cited across the two blocks verified
  present in the repo; every machine-block list line is a bare
  repo-relative path; no field other than the annotated `evidence_read:`
  lines was altered.

## Risks & followups

1. Risk: none expected — pure comment-metadata edit, no code semantics
   involved. The parenthetical research context remains recorded in each
   report's human prose, so no information was lost to the wave reader.
2. Followup (optional, cosmetic): nothing required.
