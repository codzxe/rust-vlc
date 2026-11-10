# Video record — H.264 software decode (the fallback/headless path)

Gap record (card 8.G1, issue #58 — the records sweep found the
`codec-video-h264-software` blocked matrix row had no docs/formats
record; the admission evidence landed 2026-11-09 with issue #2 and
outdates `docs/formats/video-h264.md`'s "none is scheduled" framing).
Front-matter keys per doc 04 §6:

- id = `codec-video-h264-software` (matches
  `docs/feature-matrix.toml`; row status `blocked` ×4,
  `phase_target = 5` recorded the track decision — admission is still
  pending)
- status = **blocked — admission audits complete, Lead decision
  pending** (the recommended candidate is a `[PROPOSAL]`, not an
  admission)
- phase = 5 (5.F2 conditional track; the audits are wave-15 issue #2)
- fuzz_target = none (no decoder admitted)
- crates = [] — none admitted; **no decoder crate is a workspace
  dependency** (audited in throwaway projects, never this workspace)
- self_built = []
- last_audit = 2026-11-10 (this record; the audits themselves are
  dated 2026-11-09, `docs/dependencies/records/
  h264-software-decoder-audits.md`)

## Parse / Decode / Encode

Nothing implemented for software decode: the H.264 path today is the
5.F1 parser + the hardware seam (see `docs/formats/video-h264.md` —
parse landed, decode behind `H264HwDecoder`). A software decoder
would slot behind the same codec-api `Decoder` as the fallback/
headless path.

## Evidence (the admission audits, 2026-11-09)

`docs/dependencies/records/h264-software-decoder-audits.md`
(issue #2, wave 15) — candidate table, abridged:

| Candidate | Verdict |
|---|---|
| `rust_h264` 0.6.1 (MIT OR Apache-2.0, zero deps) | **RECOMMEND `[PROPOSAL — Lead sign-off]`** |
| `rusty_h264` 0.17.2 (BSD-2, forbid(unsafe) core) | viable fallback, narrower coverage |
| `openh264` 0.9.9 | **REJECT** — vendored Cisco C++/NASM via build script (MP §9 forbidden bindings) |
| `va` 0.0.1 | false friend (a smart-contract language stub) |

Admission is Lead-gated (AGENTS §3.2/§3.3): the record ends in a
`[PROPOSAL]`, and ADR-001's conformance-gated track (corpus + fuzz +
differential kit) applies before any flip.

## Blocked reason + next step (MP §17)

**Blocked:** admission pending the Lead's sign-off on the audits'
`[PROPOSAL]` (R-01 detail row, doc 14 §2.2); the matrix row keeps
`blocked` ×4 until then — status flips are evidence-driven, never
self-served (MP §17). **Next step:** Lead decision on `rust_h264`
0.6.1; if accepted, the admission lands with a corpus + fuzz +
differential gate pass and this record gains the landed scope (the
`audio-pcm.md` consolidation shape).

## Limits / Platforms / Corpus / Results

Matrix: blocked ×4, unchanged by this record. No corpus results —
the audits' numbers (JVT conformance claims etc.) are upstream
claims quoted in the audits record, not measurements in this tree.
