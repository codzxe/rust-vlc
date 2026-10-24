# Container record — AIFF / AIFF-C

Status: **implemented** (demux, card 5.A2 — the WAV sibling).
Feature-matrix row `container-aiff` still reads `not_started` with
empty evidence — this record is that evidence; amendment recorded
in-file for the Lead (docs-only card 5.REC-1). Front-matter keys per
doc 04 §6:

- id = `container-aiff` (`docs/feature-matrix.toml:245`)
- status = implemented (demux); platform-independent code, evidence
  Linux-only
- phase = 5 (landed)
- fuzz_target = `fuzz_demux_aiff` (in-crate `aiff/fuzz.rs` + +FB
  mutation families; doc 04 §8.5 registry name)
- crates = [] — zero external deps
- self_built = [`crates/demux/src/aiff/**` (mod/chunks/fuzz/error/
  tests/test_support)]
- last_audit = 2026-10-24

## Scope landed (5.A2)

- Probe: `FORM <sz> AIFF|AIFC`; `EXTENSIONS = ["aiff","aif","aifc"]`
  (aiff/mod.rs:79, verified this audit).
- Codecs carried: PCM s8/16/24/32 **big-endian**; AIFF-C `NONE` and
  `sowt` (u8 / LE pass-through).
- COMM sample rate = 80-bit IEEE extended float; non-normalizable
  rates → typed structural refusal (never a guessed rate).
- MARK / INST chunks parsed skip-tolerantly (advisory only).
- Seek: `Fast { indexed: false }`. Zero-copy: same 2-slot
  `PayloadSlots` arithmetic-framing scheme as WAV (64 KiB ceiling).

## Corpus evidence (5.A2)

- **No corpus-v4 .aiff lane exists** (verified this audit: 143 media
  files, zero aiff) — the honest reference set is the committed
  ffmpeg-authored fixture family differential
  (`tests/wav_aiff_fixtures.rs`): valid lanes (e.g.
  `aiff_pcm_s16be_44k1_st`, `aiff_pcm_s24be_48k_st`) parse with
  frames/payload MD5 matching the ffmpeg truth table; every lane
  drains 0 damage.
- Malformed family pinned exactly: truncated head / truncated COMM;
  declared-SSND-past-actual; zero / offset-past-SSND;
  odd-COMM-pad; unsupported depths typed (`demux.aiff.*` codes:
  `truncated`, `no-ssnd`, `bad-comm`, …).

## Honest gaps (D-10, from 5.A2 + this audit)

- **No real-corpus differential** (fixture differential only — no
  .aiff in corpus-v4; acquisition item if AIFF share justifies it).
- AIFF-C uLaw corners (doc 04 §2.1's `[VERIFY-P0]`) NOT implemented
  — `NONE`/`sowt` only; other compression types refuse typed.
- Decode-throughput NOT measured (5.X1); no aiff row in
  `tests/zero_alloc_read_chain.rs` (5.X2).
- The AU sibling (`container-au`, "5.A2 sibling; trivial" per the
  matrix note) is NOT landed — separate row, still not_started.

## Gates (5.A2, exact)

Same card gates as WAV: fmt clean, clippy `-D warnings` clean,
demux suite PASS (163 unit + 8 wav_aiff_fixtures + 2 corpus_wav at
card time — the aiff tests ride the fixtures suite).

## Next steps

- Lead: matrix-row amendment (implemented + this file as evidence).
- Optional: corpus aiff lane acquisition; uLaw corners if demand
  appears; 5.X2 zero-alloc row.
