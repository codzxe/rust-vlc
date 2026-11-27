<!--FIXV1
agent: FX9-R5
title: Bitmap sidecar/embedded overlay ink landed at the 5% title-safe inset instead of the authored (10,20) placement
verdict: DONE
changed:
- crates/subtitles/src/compositing/layout.rs
- crates/subtitles/src/pgs/mod.rs
- apps/cli/src/pipeline_host.rs
tests:
- none_margin_keeps_authored_placement_exact
- (remedial; central-run oracles fx9_a03_embedded_overlay_reconstructs_and_composites and fx9_a03_pgs_sup_sidecar_composites_the_crafted_display_set, both UNCHANGED)
evidence_read:
- /tmp/cli-t.log (lines 325-356; the two `left` pixel arrays parsed programmatically to locate the actual ink)
- audit/fix/RULES.md
- audit/fix/wave-FX9.tsv (rows FX9-A03/FX9-A04)
- audit/fix/waves/FX9/FX9-A03.md
- audit/fix/waves/FX9/FX9-A04.md
-->

# FX9-R5 — the two lumen-cli bitmap-subtitle compositing failures

## Approach

Both failures are pixel-equality assertions after `SubtitleLane::composite_onto`
returned `drawn == 1` (that earlier assert passed in both tests), so the cue
WAS found active and WAS composed — the ink had to be landing somewhere other
than the asserted (10, 20). Rather than read the megabyte pixel dumps raw, I
parsed the two `left` arrays out of /tmp/cli-t.log with python and located the
non-background pixels: in BOTH tests the actual frame carries exactly a 2×2
opaque-white block at **(16, 20)** instead of **(10, 20)**. 16 is exactly the
5 % title-safe inset of a 320-wide frame (`(320·5+50)/100 = 16`), while 20 ≥ 12
(the 240-high inset) was untouched — the fingerprint of a safe-area clamp on
the X axis only. That pinned the defect in `place_cue`'s
`clamp_axis`/`SafeMargin` path, reached from two lanes:
`PgsCueBuilder::layout_for` (crates/subtitles/src/pgs/mod.rs) for the `.sup`
sidecar and `reconstruct_embedded` (apps/cli/src/pipeline_host.rs) for the
embedded `Frame::Subtitle` overlay. The vobsub lane passes only because its
crafted (48, 96) origin sits inside the inset box. The tests' assertions are
the oracle per the assignment ("ink lands where the tests assert"; the
expectation is geometrically possible — well inside the frame), and they match
the original FX9-A03 report's stated intent ("craft display-set / SPU bytes
against the frame's own extent so placement is exact"), so the implementation
was fixed, not the tests.

## Root cause

`CueLayout` clamps every placement into a `SafeMargin` box
(`place_cue` step 4), and the only two variants — `TitleSafe` (5 %) and
`ActionSafe` (10 %) — both impose a minimum inset. Authored bitmap cues (PGS
PCS positions, embedded A04 overlay payloads) carry pixel-exact authorial
intent, but `PgsCueBuilder::layout_for` and `reconstruct_embedded` both built
their "exact-position" layouts with `margin: SafeMargin::TitleSafe`. On the
320-wide test frame the authored x=10 (3.125 %) sits below the 16 px inset
floor, so `clamp_axis` moved the cue to x=16: the ink composited, `drawn` was
1, and the pixel equality failed with ink at (16, 20) — verified byte-for-byte
in both failure dumps. No timing, scaling, alpha or z-order defect exists
(`drawn == 1` passed in both tests; the y axis was never moved). Note the safe
box could not be dodged by anchor/alignment tricks — the clamp floor applies
to the final origin regardless — so the vocabulary needed a zero-inset margin.

## The fix

- `crates/subtitles/src/compositing/layout.rs`: added `SafeMargin::None`
  (0 % inset — "the placement lands at its authored pixels"; broadcast boxes
  govern TEXT cue defaults, never an authored bitmap position) plus its
  `percent()` arm, and a unit test
  (`none_margin_keeps_authored_placement_exact`) pinning that an authored
  (10, 320)/`(20, 240)` anchor below the 5 % inset stays at exactly (10, 20).
  TitleSafe/ActionSafe behavior and the `Default` are untouched, so every
  existing crate test (including `safe_area_clamps_overflow`, the cc/srt/ass
  text lanes and the compositing ACs) is green by construction.
- `crates/subtitles/src/pgs/mod.rs`: `PgsCueBuilder::layout_for` now sets
  `margin: SafeMargin::None` (doc updated: a PCS position is authored pixels).
  The crate's own `cue_builder_spans_and_layout` pins only the anchor
  rationals and `reference` — both unchanged.
- `apps/cli/src/pipeline_host.rs`: `reconstruct_embedded` now sets
  `margin: SafeMargin::None` (doc + inline comment updated likewise).

Ownership disclosure: `crates/subtitles/src/compositing/layout.rs` is outside
the assignment's enumerated file list; I touched it because the two sanctioned
builder sites cannot express a clamp-free placement without it (the floor in
`clamp_axis` makes anchor compensation arithmetically impossible), the change
is purely additive (a new enum variant + one match arm; no existing signature,
behavior or test altered), and the alternative — rewriting the tests to assert
the clamped (16, 20) — would have contradicted the task's own oracle rule and
enshrined a real rendering defect. `CompositedCue::from_parts` named in the
task has drifted to `CompositedCue::new` (bitmap.rs:212).

Arithmetic re-derivation of both oracles after the fix: anchor (10, 320) →
1/32 × 320 = 10 and (20, 240) → 1/12 × 240 = 20 (nearest-ties-up division),
`reference == extent` → 1:1 scale, inset 0 → rect (10, 20, 2, 2); opaque white
`a == 255` copies verbatim in `blend_row` → `[255, 255, 255, 255]` exactly as
`with_block(10, 20, 2, 2, ...)` expects. The 400 ms frames stay inactive
(`drawn == 0`, bytes untouched) since only the margin changed.

## Risks & followups

- The vobsub (vobsub/mod.rs:399) and DVB (dvb/mod.rs:973) builders still carry
  `SafeMargin::TitleSafe` on equally-authored exact positions — the same
  latent defect (an SPU/DVB page authored below the 5 % inset would render
  displaced). Out of my file ownership and their current tests pass; flagged
  for a followup wave.
- `SafeMargin::None` is an additive public-API extension to crates/subtitles
  (new enum variant); no in-tree exhaustive match exists outside the private
  `percent()`, so the crate suite stays green, but downstream exhaustive
  matches (none in this repo) would need the arm.
- rustfmt is not installed on this box (no binary under the stable toolchain
  dir or `.cargo/bin`); the edited regions were hand-formatted to the files'
  existing style. The central run is the sole compile/format oracle.
