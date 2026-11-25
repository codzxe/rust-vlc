<!--FIXV1
agent: FX1-R3
title: remedial runtime fix — fx1_a03_stts_minimal_box_honors_no_entries expects the honest typed no-sample-table refusal (test-only, no src)
verdict: DONE
changed:
- crates/demux/tests/fx1_a03_mp4_box_bounds.rs
tests:
- fx1_a03_stts_minimal_box_honors_no_entries
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- audit/fix/waves/FX1/FX1-A03.md
- audit/fix/waves/FX1/FX1-R1.md
- audit/fix/waves/FX1/FX1-R2.md
- audit/files/crates/demux/src/mp4/mod.rs.audit.md
-->

# FX1-R3 — fx1_a03_stts_minimal_box_honors_no_entries failed at runtime (remedial batch)

## Root cause (with the assignment-hypothesis drift noted)

The assignment hypothesized the fixture was "TOO minimal — the spliced
MP4 lacks a complete sample table (stsz/stco/stsc)". That hypothesis is
**disproven by byte-level replication**: the fixture was already built
exactly like the two passing siblings — `test_support::progressive`
base file + length-neutral splice. A byte-exact python replication of
the builder + splice + parse (scratch `/tmp/fx1r3/check.py`) shows:

- ftyp is 36 bytes, moov box at 36, moov *payload* start = 44 — the
  exact "byte 44" of the runtime error;
- the stbl is COMPLETE: stsd(52) + stts(24) + stsc(28) + stsz(36) +
  stco(32), and the splice is length-neutral (crafted 24 == box 24),
  stco's first offset still the valid mdat payload start (545).

The real chain (all in src, which I must not change): the crafted stts
declares `LYING_COUNT = 0xFFFF_FFFF` with one **zero-run** entry
(run 0, delta 0) → the clamped walk (mp4/mod.rs:1253-1276) takes the 1
entry the body carries, run 0 pushes **zero deltas** → per-sample
expansion `count = sizes.len().min(deltas.len())` (mod.rs:1360) =
min(4, 0) = 0 → `if !offsets.is_empty() && count > 0` false → the trak
builds **zero samples** → `start()`'s `any_samples` check (mod.rs:780)
is false and `fragmented` is false (no trex, no moof) → the refusal at
mod.rs:794-795 `no_sample_table(moov_start)` = byte 44 → the observed
`[demux] corrupt structure at byte 44 (demux.mp4.no-sample-table)`,
which hit the test's own `unreachable!("start: {e}")` at :151.

So the demuxer is behaving by design: a progressive trak whose stts
yields zero deltas has nothing to demux, and a moov with no samples at
all is the module's typed `no-sample-table` contract (the in-crate
suite `zero_sample_tables_in_progressive_is_typed`, mp4/tests.rs:622,
pins the same refusal for empty stsz/stco). Per the assignment's own
fallback (point 3): a zero-entry stts genuinely cannot complete a
successful progressive parse as the sole trak — no fixture repair can
avoid the refusal while the zero-run entry is still honored — so the
honest fix is the typed-error expectation. FX1-A03's report predicted
"zero deltas → zero packets, one note, EOS reached" for this shape;
that prediction was wrong at runtime and this card corrects the test to
the demuxer's actual contract. The cited line :151 was accurate (no
drift) — it was the failing test's own `unreachable!`.

## Fix (Changes per file)

`crates/demux/tests/fx1_a03_mp4_box_bounds.rs` — test-only, one test +
comments; the two passing siblings and every helper untouched:

- `fx1_a03_stts_minimal_box_honors_no_entries` (name kept): the body
  now asserts `mp4.start()` returns `Err` whose `err.context.code ==
  "demux.mp4.no-sample-table"`, using the house idiom of this very
  tests dir (`match start() { Err(err) => assert_eq!(err.context.code,
  …), Ok(_) => unreachable!(…) }` — fx1_a02_short_ts.rs:57-60, the
  pattern FX1-R1 verified: `context.code` is `pub String`,
  media-types/src/error.rs:224, and `String == &str` compiles). The
  fixture construction is byte-for-byte unchanged (same
  `progressive` base, same `crafted_box(b"stts", LYING_COUNT,
  &entry(0, 0))`, same length-neutral splice) — only the expectation
  changed: success → honest typed refusal.
- Doc comment of that test rewritten to state the honest intent
  (bounded parse, typed error, never a panic, never the pre-fix
  ~4.3e9-iteration spin before the same refusal).
- Module header: +4 comment lines noting the minimal zero-run `stts`
  as the one exception whose honest outcome is the typed refusal.
- No src/ file, no Cargo.toml, no imports, no helper changed; no
  `.unwrap()`/`.expect()`/TODO markers added (deny lints respected;
  `match` + `unreachable!` follows the file's 12 pre-existing sites).

## Test design

The test stays meaningful exactly as the assignment requires: the
crafted box still parses **bounded** (the clamp takes the 1 entry the
body carries; pre-fix this same error only arrived after the
unbounded 4.3e9-iteration walk, i.e. an effective timeout) and the
demuxer's answer is the **typed** `no-sample-table` refusal — asserted
on the stable code, never a panic and never a hang. The two passing
siblings keep the clamp's positive oracle (damage note + fewer
entries), so the card's scope (one failing test, no collateral) holds.

## Verification reasoning (self-check, cargo-free per RULES)

- Byte-exact replication (`/tmp/fx1r3/check.py`): moov payload start
  44; stbl complete (stsd/stts/stsc/stsz/stco); splice 24==24
  length-neutral, stco offsets valid; count clamped to 1 entry;
  zero-run → `deltas == []` → 0 samples → `Err(no_sample_table(44))` —
  exactly the runtime failure; the SAME simulation over the sibling's
  `entry(2, 3000)` on the identical fixture construction yields
  `deltas == [3000, 3000]`, 2 samples, note 4294967294 — the passing
  test's exact assertions, proving the defect was the expectation,
  not the fixture.
- Post-edit re-read of the whole file: helpers `drain`/`Stamp`/
  `DemuxEvent`/`Demuxer`/`LYING_COUNT` all still used by the two
  untouched siblings (no unused-import lint); the failing test's new
  `match` needs `let mut mp4` (start takes `&mut self` — siblings
  call it the same way); no line exceeds 100 cols.
- rustfmt unavailable in this box (no binary under
  `/home/z/my-project/.cargo/bin`; same as FX1-A03/R2 reports) — the
  added block is hand-formatted to the file's own style and would not
  reflow.

## Acceptance vs FIX-PLAN

F0.4b's stts acceptance ("a truncated stts/ctts box must parse bounded
and either complete with fewer entries or return the module's existing
typed parse error; never panic/OOB") remains satisfied by the wave's
src fix; this card makes the third test assert the *typed parse error*
half of that very acceptance — the honest arm for a box whose entries
add up to zero. No src change was needed or made (assignment
constraint honored).

## Risks & followups

- No local cargo run is permitted (RULES), so this is
  source-grounded repair; residual compile risk is near-nil — every
  API used (`start()`, `err.context.code`, assert_eq on String vs
  &str) is already exercised verbatim by neighboring tests in this
  same crate.
- The minimal test no longer asserts the `stts-count-over-ceiling`
  note for the lying count: the note IS recorded in the demuxer's
  damage list but cannot surface because `start()` returns Err before
  `surface_damage()` (mod.rs:840) — the note's surfacing oracle stays
  with the two passing siblings, which assert it exactly.
- This test alone cannot discriminate pre-fix vs post-fix src behavior
  (same typed error in both eras; pre-fix it only arrives after the
  minutes-scale spin) — the clamp's behavioral oracle remains the two
  clamped siblings, which FX1's central gate already ran green.
