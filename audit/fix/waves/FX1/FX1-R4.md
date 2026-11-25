<!--FIXV1
agent: FX1-R4
title: FX1-A09 PGS retention-cap test — the crafted ODS declared a 0-byte RLE (usize to_be_bytes slice); re-craft with a valid 24-bit total
verdict: DONE
changed:
- crates/subtitles/tests/fx1_a09_guards.rs
tests:
- fx1_a09_pgs_retention_cap_trips_typed_not_unbounded
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX1/FX1-A09.md
- audit/files/crates/subtitles/src/pgs/mod.rs.audit.md
-->

# FX1-R4 — remedial: `fx1_a09_pgs_retention_cap_trips_typed_not_unbounded` fails on its own "no spurious RLE damage" invariant

Mission: fix the TEST, not src. Test name kept; no src/ or sibling-test
change (verified by the diff scope below — one helper in one file).

## Root cause

The failing test's ODS byte-crafting helper `max_canvas_ods()`
(`crates/subtitles/tests/fx1_a09_guards.rs:146`) encoded the ODS's
24-bit big-endian declared total as:

```rust
d.extend_from_slice(&total.to_be_bytes()[1..4]);   // total: usize
```

On the 64-bit host `usize::to_be_bytes()` is **8 bytes**, so
`[1..4]` is `[0x00, 0x00, 0x00]` — the ODS declares a **0-byte**
data length. `ods_segment` (`pgs/mod.rs:502-514`) then computes
`declared = 0.saturating_sub(4) = 0`, keeps zero of the 771 crafted
RLE bytes, and records one typed `FragmentOverrun { object_id: 1 }`;
the stored `ObjectState.rle` is empty, so every per-reference
`decode_object` in `compose()` is truncated → `RleOverrun { 1 }`
per reference. The cap check runs *after* the decode, so the fifth
reference emits its `RleOverrun` before `retained > cap` produces
`CompositionRetained { bytes: 83_886_080 }` and breaks. Net ledger
exactly as the mission reported:
`[FragmentOverrun{1}, RleOverrun ×5, CompositionRetained{83886080}]`
— i.e. the "genuine RLE damage" was an artifact of the *test's own*
malformed declared length, not of the run segments and not of src.

The run segments were already valid and exactly canvas-covering:
256 × `00 7F FF` (16383-px index-0 long runs) + one `00 41 00`
(256-px run) = 4,194,304 px = 2048×2048 — a clean decode once the
fragment is actually retained.

## Fix choice: (a) — valid-RLE re-craft

Per the mission's preference order, (a) was proportionate to a
one-line change because the RLE crafting was already correct; only
the 3-byte total field was wrong. Replace the sliced expression with
an explicit 24-bit BE encoding — exactly what the crate's own
in-src helper `ods_first_declared` (`pgs/mod.rs:766-768`) does:

```rust
d.extend_from_slice(&[(total >> 16) as u8, (total >> 8) as u8, total as u8]);
```

With it, `declared = 771 == fragment.len()` → no `FragmentOverrun`;
the full 771-byte RLE decodes clean (0 clipped runs, 0 short lines,
not truncated, write cursor lands exactly on 4,194,304) → no
`RleOverrun`; the display set is a genuinely **valid** ~2 KiB PGS
stream whose only damage event is the retention cap: one
`CompositionRetained { bytes: 5 × 16 MiB }`, exactly four objects
retained (sum == `MAX_COMPOSED_RGBA_BYTES`), each a full
2048×2048×4 canvas. Choice (b) — deleting the "no spurious RLE
damage" invariant — was rejected: it would weaken the guard to
tolerate a broken fixture, while (a) keeps the stronger claim: the
cap trips on a perfectly well-formed display set, which is the
honest shape of the audit's W09-A04 amplification attack (a hostile
but *valid* PCS re-referencing a valid canvas).

## Changes per file

1. `crates/subtitles/tests/fx1_a09_guards.rs` — only inside
   `max_canvas_ods()`: the declared-total line replaced with the
   shift-based 24-bit encoding + a 6-line comment explaining the
   usize-slice trap. Test name, test body, all six assertions, and
   every other test/helper in the file are untouched. No src/ change.

## Test design

`fx1_a09_pgs_retention_cap_trips_typed_not_unbounded` (name kept,
assertions unchanged): PCS (EpochStart, 1920×1080) listing object 1
five times + PDS id 0 (empty) + the now-valid 2048×2048 clean-fill
ODS + END. Asserts: exactly one `CompositionRetained` event; the
tripping total is 5 × 16,777,216; exactly the first four references
retained (`objects.len() == 4`, retained bytes ==
`pgs::MAX_COMPOSED_RGBA_BYTES`, each `rgba.len() == 2048*2048*4`);
and the RLE decodes clean — only the retention event fires (the
previously-failing invariant, now true).

## Verification reasoning (no cargo — read + python3 simulation)

1. **Positive**: a faithful python3 transcription of
   `walk_segments` / `parse_pcs` / `parse_pds` / `parse_ods` /
   `ods_segment` / `decode_object` / `compose` (incl. the cap
   accounting), driven by the exact bytes the fixed helper emits
   (847-byte payload), yields: walk 847/847 Stripped (detect prefers
   Stripped on ties), ODS total 775 → declared 771 = fragment 771 →
   no FragmentOverrun; decode covers 4,194,304/4,194,304 px,
   clipped=0, short=0, truncated=false → no RleOverrun; compose
   trips at ref 5 → damage == `[CompositionRetained{83886080}]`,
   4 objects, each rgba 16,777,216 B — **all six assertions pass**.
2. **Negative control**: the same simulation on the pre-fix bytes
   reproduces the mission's exact observed ledger
   `[FragmentOverrun{1}, RleOverrun ×5, CompositionRetained{83886080}]`
   — the root-cause attribution (the usize slice, not src) is
   anchored, not speculative.
3. Read-verified every transcribed src site: `pgs/mod.rs`
   `ods_segment` (:465-534) and `compose` (:538-598), `rle.rs`
   `decode_object` (:60-128), `segment.rs` `parse_ods` (:402-431),
   and `palette.rs`'s fixed `[Rgba; 256]` bank (the all-index-0
   lookup with an empty PDS is in-bounds).
4. Not compiled (wave gating — orchestrator re-gates centrally).
   The edit is a one-line replacement + comment; no type/API churn,
   host-width-independent. rustfmt is not installed on this box
   (`~/.cargo/bin` has no rustfmt binary) — manual formatting matched
   to the file's existing ≤100-col style.

## Acceptance vs FIX-PLAN

FX1-R4 is remedial and has no FIX-PLAN row (rg `FX1-R4` /
`fx1_a09_guards` / `retention_cap` in `audit/fix/FIX-PLAN.md`: no
matches); the governing contract is FX1-A09's own report semantics —
"clean-fill ODS … exactly one `CompositionRetained { bytes: 5×16 MiB }`
… no spurious RLE damage" — which the fixed test now asserts
faithfully. Mission constraints: name unchanged ✓, src/ untouched ✓,
no other test touched ✓, workspace lints irrelevant (test-only edit,
no unwrap/expect added) ✓.

## Risks & followups

- **Not compiled/run centrally yet** — the pass claim is
  simulation-derived; the simulation did reproduce the exact
  observed failure on the pre-fix bytes, so the model is anchored.
- The declared total is still hand-crafted; a future "simplification"
  back to a sliced `to_be_bytes` would silently regress on 64-bit
  hosts (on 32-bit hosts the old expression was accidentally
  correct). The new comment warns; an optional hardening would
  assert the declared/actual agreement inside the test (not added —
  minimal-diff assignment).
- Memory shape unchanged from FX1-A09's note: ~64 MiB retained + a
  4 MiB index buffer per reference decode — fine on the 3 GB box.
