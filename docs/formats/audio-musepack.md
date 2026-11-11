# Audio record — Musepack SV7/SV8 decode

Gap record (card 8.G1, issue #58 — the records sweep found this
blocked matrix row had no docs/formats record). Front-matter keys per
doc 04 §6:

- id = `codec-audio-musepack` (matches `docs/feature-matrix.toml`;
  row status `blocked` ×4, `phase_target = 8`)
- status = **blocked (v1)** — no pure-Rust decoder exists
- phase = 8 (parked; revisit on demand)
- fuzz_target = none (no decoder admitted; doc 04 §6: required only
  when status != not_started — none exists to fuzz)
- crates = [] — nothing admitted
- self_built = []
- last_audit = 2026-11-10 (this record; the underlying crate survey
  is feasibility audio-codecs §1 row 18, 2026-10-19, re-checked then)

## Parse / Decode / Encode

Nothing implemented; no MPC parser or decoder exists in-tree. Encode
likewise out of scope.

## Evidence

- Feasibility `docs/feasibility/sections/audio-codecs.md` §1 row 18:
  "None. crates.io `musepack` NOT_FOUND (re-checked) [RV] … P3; M
  self-build if ever demanded."
- Matrix row `codec-audio-musepack` (`docs/feature-matrix.toml`)
  carries the same blocked_reason citing that survey; priority P3.
- The reference implementation (libmpc / Musepack SV8 spec) is C /
  spec-side material — usable only as oracle, never vendored
  (clean-room rule, AGENTS §3.1).

## Blocked reason + next step (MP §17)

**Blocked:** no pure-Rust crate exists (crates.io survey
2026-10-19, feasibility audio-codecs §1 row 18). **Next step:**
recheck crates.io at the Phase-8/9 boundary or on user demand; the
M-sized self-build option (SV7/SV8 bitstream is publicly documented)
is the fallback if share data ever justifies it. Re-opening requires
a fresh record per the matrix seed-notes rule.

## Limits / Platforms / Corpus / Results

Matrix: blocked ×4 on all platforms, unchanged. No corpus fixtures,
no results — nothing ships.
