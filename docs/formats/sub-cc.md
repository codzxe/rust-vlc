# Subtitle record — ATSC closed captions (CEA-608 full + CEA-708 staged)

Records-consolidation entry (card 8.G1, issue #58 — the format-records
audit found the implemented `subtitle-cc-608-708` matrix row had no
docs/formats record; engine evidence from card 5.N3, ADR-012 self-build
option 1). Front-matter keys per doc 04 §6:

- id = `subtitle-cc-608-708` (matches `docs/feature-matrix.toml`;
  row status `implemented` on win/linux/macos, `not_started` on web —
  this record consolidates it)
- status = **implemented** (608 full; 708 staged — the module's own
  honesty table below is the authority on the split)
- phase = 5 (card 5.N3; in tree by the 5.R2-V Phase-5 exit `3116aec`)
- fuzz_target = in-crate `fuzz_n` (`crates/subtitles/src/cc/fuzz.rs`,
  fed directly by `tests/cc_acceptance.rs`) — the cargo-fuzz shim is
  Lead-owned wiring, the `fuzz_sub_srt` precedent (`docs/formats/
  sub-srt.md`); no `fuzz_sub_cc` target exists in `fuzz/` yet
- crates = [] — self-built, std-only, no `unsafe`
- self_built = [`subtitles::cc` (`cc_data`/`eia608`/`cea708`/`render`
  /`fuzz` + the `CcDecoder`/`CcCueBuilder` front doors in `mod.rs`)]
- last_audit = 2026-11-10 (this consolidation; engine evidence from
  5.N3, gates re-run this session — below)

## Parse / Decode / Render (the module honesty table, quoted)

IN (implemented and pinned by tests — `crates/subtitles/src/cc/mod.rs`
header table):

- Carriage grammars: GA94 SEI type-4 (the H.264 consume seam),
  MPEG-2 picture-`user_data` GA94, SCTE-20 bit-packed.
- 608: parity, field/channel latch (CC1–CC4), PAC (row + attributes +
  indent), mid-row, tab, misc codes; the 15×32 double-buffered grids;
  pop-on / roll-up / paint-on; basic + extended charsets.
- 708: DTVCC fragment reassembly, service-block layer, service-1 →
  608 compatibility route.
- Render: grid snapshot → `CompositedCue` over the 704×480 NTSC
  reference with per-cell colors (exact monospace cell metrics).

OUT (recorded honestly in the same table, not interpreted):

- XDS packets: skipped, counted.
- 708 native window model (DEFINE_WINDOW geometry, pens, per-window
  addressing, borders, wrap): walked + op-recorded, never applied.
- 708 G0–G3 native glyph planes: byte-walked only.
- 608 text mode (T1–T4): paint-on approximation (documented).
- VBI / line-21 analog extraction: out of v1 scope (ADR-012).

Timing: streaming protocol with no inherent end times —
`CcCueBuilder` gives each update's grids `[update_pts,
next_update_pts)` and leaves the last update open-ended (the player's
clock closes it; no 608/708 page timeout exists to bound it with).

## Encode

Not in scope (captions are decode-only in v1; no 608/708 writer
exists in-tree).

## Evidence

- Module: `crates/subtitles/src/cc/` — the header carries the honesty
  table and the ownership map reproduced above.
- Matrix row evidence quotes the 5.R2-V exit: "26 unit + 11
  cc_acceptance tests, all green in this tree 2026-10-26" (the unit
  count has since grown with the module — today's numbers below).
- Open seam (matrix `notes`): the codecs-side SEI/user-data FEED
  wiring (H.264 5.F1 consume seam) — carriage grammars live in the
  subtitles crate, the feed from a live H.264 decode is not wired.

## Gates (this consolidation, exact commands, run 2026-11-10)

Shared-target worktree `wt/w24e` @ `492db70`, 2-core Linux sandbox:

- `cargo test --locked -p subtitles --lib cc` — **28 passed / 0
  failed** (eia608 8, cc_data 8, cea708 6, render 4, + mod).
- `cargo test --locked -p subtitles --test cc_acceptance` — **11
  passed / 0 failed** (includes the fuzz_n mutation feeds).

## Known gaps / honest limits

- 708 ADVANCED window model not applied (staged partial, above) —
  recorded, never silently degraded.
- No end-to-end proof from a real H.264 SEI stream yet (the 5.F1
  consume seam is open — same class as the PGS R-27 overlay gap).
- Linux-only evidence (program-wide limit, `docs/KNOWN-LIMITS.md`
  §1); web column `not_started`.
