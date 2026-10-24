# Format record — MIDI (Standard MIDI File demux + SoundFont synthesis) [PROPOSAL]

Status: **blocked-v1 / deferred (later) — decision record (card 5.P2b-I
part B)**. Two matrix rows, two verdicts: `container-midi-smf` (SMF
demux) = **later** (S, no v1 card — no `smf` module exists in
`crates/demux/src/`, honest not_started); `codec-audio-midi-synth`
(synthesis) = **blocked v1 with a named later path [PROPOSAL] =
rustysynth** — doc 04 §3.2's own framing: "`later`, not `blocked`",
amended here to blocked-v1-with-later-path to match the matrix's
blocked ×4 row and MP §17 (blocked items carry reasons + next steps).

- Date of evidence: **2026-10-24** (crates.io API + GitHub raw README,
  this card).
- Supersedes-cites: doc 04 §3.1 row "MIDI / synthesized" + §3.2 MIDI
  [PROPOSAL] paragraph + §8.6 item 6 ("oxisynth LGPL review;
  rustysynth SMF needs"), feasibility subtitles §5 (2026-10-19), doc 11
  (LGPL allowlist posture), ADR-006 (MIT posture), feature-matrix rows
  `container-midi-smf` + `codec-audio-midi-synth`.

## Parse / Decode / Encode

Nothing implemented. **Synthesis is decode for MIDI**: SMF bytes →
event stream → SoundFont-rendered PCM. The proposed later path is
S-sized *because* rustysynth closes the two hard halves in-crate (SMF
parsing + synthesis); our glue is demux registration + audio-engine
plumbing + the user-supplied `.sf2` policy. Encode: never planned.
v1 honest behavior: a `.mid` file is not openable (no demux module) —
acceptable per P3; the typed-refusal question only arises once an SMF
module lands.

## Evidence (what was checked, 2026-10-24)

**oxisynth (LGPL review — doc 04 §8.6 item 6 first half):**
- `oxisynth` **0.1.0**, published 2025-05-25, **LGPL-2.1** (all six
  releases since 2022-07-03 carry LGPL-2.1), 33,851 downloads,
  repo PolyMeilex/oxisynth, "Rust soundfont synthesizer… inspired by
  the widely known FluidSynth". Runtime deps present (bitflags ^2.4 +);
  consumes MIDI events, **does not parse SMF** (feasibility §5).
- License analysis per doc 11 + ADR-006: ADR-006 fixes the repo at
  MIT with a **permissive-only merge policy** (GPL strictly forbidden;
  the cargo-deny allowlist keeps **GPL/LGPL deliberately absent "until
  the Lead decides (oxisynth LGPL-2.1 case, doc 04 §3.1)"** — doc 11
  §dependency-allowlist). Static linking (Rust's model) triggers LGPL
  §2's relink-ability duty (ship source/object files — a burden our
  simple MIT distribution does not carry); the dynamic cdylib escape
  (libloading) costs per-platform dylib shipping, unstable ABI
  `[VERIFY]`, unsafe adapter code (AGENTS §3.3), and single-binary
  simplicity. Additionally, "inspired by FluidSynth" (LGPL C) means
  its copyleft is **lineage-inherent, not incidental** — relicensing
  is not an ask-the-author fix. **Verdict: stays out-of-tree; doc 11
  owns any future change; revisit only if rustysynth proves
  insufficient after real use.**

**rustysynth (purity + needs — doc 04 §8.6 item 6 second half):**
- `rustysynth` **1.3.8** (2026-10-20 — newer than feasibility's 1.3.7
  of 2026-10-01; actively maintained), **MIT**, 143,021 downloads,
  repo sinshu/rustysynth, "A SoundFont MIDI synthesizer written in
  pure Rust, ported from MeltySynth" (MeltySynth is C#/MIT —
  **permissive lineage, not FluidSynth-derived**).
- **Purity verified this card:** crates.io dependencies endpoint for
  1.3.8 returns `[]` — README corroborates: "No dependencies other
  than the standard library". Transitive cargo-deny/vet graph = the
  crate itself.
- **"rustysynth SMF needs" answered:** README: "Supports standard
  MIDI files with additional features including dynamic tempo
  changing" — `MidiFile` (parser) + `MidiFileSequencer` ship in-crate.
  **This closes doc 04 §3.1's "needs SMF parser (S)" estimate.** The
  remaining open need is **SoundFont policy only**: user-supplied
  `.sf2` (exactly VLC's own fluidsynth posture — VLC ships no font
  either); **no bundled default font in v1** (bundling would add a
  font-license question for zero parity value). `.sf3` = open
  `[VERIFY: rustysynth sf3 status — see rustysynth-ext below]`.
- Ecosystem notes (recorded, not adopted): `rustysynth-ext` 1.5.0
  (MIT, 2026-09-06→2026-10-24, fork adding **sf2+sf3**) — the follow-on
  if sf3 is demanded; `ferrosintesis` 0.21.59 (MIT OR Apache-2.0,
  2026-09-27) — "pure-Rust General MIDI synthesizer and MIDI-to-WAV
  renderer" with **built-in GM voices (no SoundFont needed)** —
  interesting no-font alternative, young (165 dl), provenance
  unverified `[VERIFY]`; `midly` 0.5.3 (518 k dl) exists as a
  standalone SMF parser if a demux-first shape is ever preferred.

**VLC handling (oracle):** `smf` demux probed (formats.json
`midi-smf`: minimal type-0/480-ppqn fixture, chosen_demux `smf`,
**decode negative** — "Codec `MIDI' (MIDI Audio) is not supported");
the `fluidsynth` plugin was one of the 10 excluded headless debs, so
the pinned oracle **cannot show MIDI decode at all** — any future
differential needs a fluidsynth-equipped build `[VERIFY — recorded in
formats.json midi-synth notes]`.

## Bit depths / layouts

Synth output = 16-bit or float stereo at the engine's rate
(SynthesizerSettings); input `.mid` formats 0/1/2 per SMF spec.
Recorded for the future card's scope table.

## Tracks / Seeking / Timestamps

SMF tracks → event streams; seeking = sequencer-position math (SMPTE
vs PPQN tempo maps) — all rustysynth-side when scheduled; our demux
module owns file-level chunking (MThd/MTrk).

## Corrupt-input behavior

N/A (no parser ships). Truncated .mid tolerance is a design input for
the future card; fuzz target per doc 04: `fuzz_codec_midi`.

## Limits / Platforms

Matrix: smf demux not_started ×4; synth blocked ×4 (blocked_reason
already names the rustysynth-later + oxisynth-out-of-tree posture —
consistent with this record). Web: synthesis is CPU-bound pure Rust —
no wasm blocker exists beyond priority.

## Corpus

None — no .mid lane in corpus-v4; Wikimedia stats show 7,118 MIDI
files there (formats.json usage-share note) — niche (P3) confirmed.

## Results

None (no code). **Recommendation [PROPOSAL]:** when a MIDI card is
ever scheduled (Phase 6+): **rustysynth admission path** — direct dep
`rustysynth = "1.3"` (MIT, zero-dep), SMF parse via its in-crate
`MidiFile`/`MidiFileSequencer` (no `midly`, no SMF self-build), glue
ours (demux registration + audio-engine node), **user-supplied .sf2,
no bundled font**, honest `midi = partial (synth; user soundfont)`
matrix semantics. **oxisynth stays out-of-tree** (LGPL-2.1 vs
ADR-006/doc 11 posture — doc 11 owns the allowlist decision, not this
card). Interim next step: none required — recheck only at Phase 7
R-10 if the Lead wants the licensing posture re-examined.
