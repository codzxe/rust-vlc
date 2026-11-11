# Audio record — WavPack hybrid-lossless decode

Gap record (card 8.G1, issue #58 — the records sweep found this
blocked matrix row had no docs/formats record). Front-matter keys per
doc 04 §6:

- id = `codec-audio-wavpack` (matches `docs/feature-matrix.toml`;
  row status `blocked` ×4, `phase_target = 8`)
- status = **blocked (v1)** — license conflict candidate, no decode
- phase = 8 (parked pending the license question)
- fuzz_target = none (no decoder admitted)
- crates = [] — nothing admitted
- self_built = []
- last_audit = 2026-11-10 (this record; underlying survey:
  feasibility audio-codecs §1 row 16 + §5.5, 2026-10-19)

## Parse / Decode / Encode

Nothing implemented. Carriage side: `.wv` files are their own
container (no `.wv` demuxer in `crates/demux/`); WavPack also rides
the Matroska `A_WAVPACK4` codec id (see
`docs/formats/container-matroska.md` for the family).

## Evidence

- Feasibility `docs/feasibility/sections/audio-codecs.md` §1 row 16:
  upstream symphonia placeholder empty (as Opus); a third-party
  `symphonia-codec-wavpack` 0.1.1 exists but is **GPL-3.0-or-later**
  — a copyleft conflict candidate for this MIT repo (ADR-006).
- §5.5 ("WavPack — a new license wrinkle"): flagged, not decided —
  `wavpack = blocked` in v1 with the license question parked with
  doc 11.
- Matrix row `codec-audio-wavpack` carries the same blocked_reason.

## Blocked reason + next step (MP §17)

**Blocked:** the only published Rust decoder is GPL-3.0-or-later
while the repository is MIT (ADR-006) — adopting it would change the
license story of shipped binaries; the decision is parked with doc 11
(license/compliance lane), not with this record. **Next step:** a
Lead decision on the GPL question (ship-incompatible → stays
blocked; carve-out → admission audit + record, the
`h264-software-decoder-audits.md` pattern); alternatively an M-sized
clean-room self-build from the public WavPack bitstream spec if
demand justifies it.

## Limits / Platforms / Corpus / Results

Matrix: blocked ×4, unchanged. No corpus fixtures, no results.
