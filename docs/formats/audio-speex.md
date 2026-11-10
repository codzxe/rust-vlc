# Audio record — Speex decode (legacy VoIP, Ogg-mapped)

Gap record (card 8.G1, issue #58 — the records sweep found this
blocked matrix row had no docs/formats record). Front-matter keys per
doc 04 §6:

- id = `codec-audio-speex` (matches `docs/feature-matrix.toml`; row
  status `blocked` ×4, `phase_target = 8`)
- status = **blocked (v1)** — no pure-Rust decoder exists
- phase = 8 (parked)
- fuzz_target = none (no decoder admitted)
- crates = [] — nothing admitted
- self_built = []
- last_audit = 2026-11-10 (this record; the crate survey is
  feasibility audio-codecs §1 row 17, 2026-10-19)

## Parse / Decode / Encode

Nothing implemented. Carriage side: Speex rides Ogg logical streams
(the Ogg demuxer's `OggCodec` — see `docs/formats/container-ogg.md`
for the mapping family); the codec itself has no decoder in-tree.

## Evidence

- Feasibility `docs/feasibility/sections/audio-codecs.md` §1 row 17:
  "None. crates.io `speex` NOT_FOUND (re-checked) [RV] … Legacy VoIP
  streams stay `missing`; P3."
- Matrix row `codec-audio-speex` carries the same blocked_reason;
  priority P3 (legacy VoIP niche).

## Blocked reason + next step (MP §17)

**Blocked:** no pure crate (survey 2026-10-19); legacy VoIP codec
with a shrinking user base (P3). **Next step:** recheck crates.io at
the Phase-8/9 boundary; libspeex (C, BSD) is oracle-only material
(clean-room rule) — if demand appears, the CELT-derived bitstream is
publicly documented for a self-build estimate (M) with a fresh
record.

## Limits / Platforms / Corpus / Results

Matrix: blocked ×4, unchanged. No corpus fixtures, no results.
