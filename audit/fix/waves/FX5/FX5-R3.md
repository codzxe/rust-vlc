<!--FIXV1
agent: FX5-R3
title: remedial — the last 65→69 pin, control-server e2e.rs:318 + its derived assertions
verdict: DONE
changed:
- apps/control-server/tests/e2e.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- audit/fix/waves/FX5/FX5-R1.md
- audit/fix/waves/FX5/FX5-A02.md
- audit/fix/waves/FX5/FX5-R2.md
- apps/control-server/tests/e2e.rs
- apps/control-server/src/dispatch.rs
- apps/cli/src/run.rs
- apps/cli/tests/command_surface.rs
- crates/commands/tests/fx5_a02_catalog_audio_verbs.rs
-->

# FX5-R3 — remedial: the last 65→69 catalog pin (control-server e2e.rs:318)

## Approach

The orchestrator's central run left exactly one failing assertion, the last
65-pin in the repo sweep: `e2e_full_session_through_the_app_stack` at
`apps/control-server/tests/e2e.rs:318` (left 69, right 65) — the same
out-of-domain count miss FX5-R1 fixed at its four sibling sites (A02 minted
the audio quartet: audio.set/get-balance, audio.set/get-eq). The failing
test's own left-hand value is the oracle: the live table answers 69 rows,
matching FX5-A02's canonical pin
(`fx5_a02_catalog_counts_carry_the_quartet`: 69 command rows, 10 audio
rows). I read the whole e2e.rs before editing, moved the pin, and swept the
file for any other catalog-derived assertion the 4 new ids could shift.

## Changes per file

- `apps/control-server/tests/e2e.rs` — exactly one hunk:
  - :311 comment header `all 65 commands` → `all 69 commands`;
  - :314-315 appended FX5-R1's narration sentence verbatim (same style as
    the run.rs:964-965 and command_surface.rs:100-101 siblings — count
    history kept, quartet sentence appended):
    `// 69 since FX5-A02 minted the audio quartet (set/get-balance and
    // set/get-eq, the sound-experience knobs).`
  - :318→:320 the pin `assert_eq!(commands.len(), 65);` → `69`.
  - Derived-assertion sweep of the whole file, honestly reported: NONE
    other. The only other catalog-dependent assertion is :321
    `commands.iter().any(|row| row.id == "config.set-setting")` — a
    membership check, count-independent (config.set-setting is in the 69).
    `requests_served "3"` / `ops_denied "1"` / `sessions 1` /
    `session_records.len() 1` count the test's own four sends, not catalog
    rows; welcome/engine-info/version assertions are not catalog-derived;
    the file has no checksum, id-order table, or second listing count.
- rustfmt (edition 2021) ran clean and changed nothing beyond my hunk
  (file verified line-by-line before/after: 433→435 lines, the +2 being
  the narration). The RULES.md cargo-bin PATH is absent on this box (exit
  127), so the shim FX5-R2 recorded was used:
  `/home/z/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustfmt`.

## Test design

None added (`tests: []`) — this is a remedial pin fix on a pre-existing
e2e; the regression is the moved pin itself, and the test that would have
caught the drift at mint time is exactly this `system.list-commands`
introspection, now pinned to the live table's true 69.

## Acceptance vs FIX-PLAN

No FIX-PLAN row exists for FX5-R3 (grep for FX5-R3: no entries — same
remedial posture as FX5-R1/R2, the assignment brief is the spec). Both
deliverables complete: (1) the pin moved 65→69 at the exact cited site
with the sibling-matched narration; (2) the file swept for derived counts
with none found beyond the pin. A repo-wide rg for the count-pin shapes
confirms this was the last catalog 65 (remaining 65 hits are unrelated:
exit code 65, PCM channel count, signature score).

## Risks & followups

1. Not compile/test-verified by me (cargo banned for agents); the pin now
   matches the orchestrator's own observed left-hand 69, so the central
   rerun is the oracle and should go green.
2. Process disclosure (honesty contract): while checking for stray
   scratch files I accidentally invoked `git diff --stat` (read-only,
   stderr-suppressed, mutated nothing — no add/commit/checkout was ever
   run; no scratch files existed and none were created). Noting it so the
   gate sees it; the file-state verification above was done by direct
   Read, not by git.
3. The pre-edit working tree already carried whole-wave drift from the
   sibling agents (visible in that accidental stat output); nothing from
   it was used — every edit here was decided before that command ran.
