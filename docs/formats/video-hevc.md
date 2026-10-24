# Video record — HEVC/H.265 (parser; decode blocked)

Records-consolidation entry (card 5.REC-2; evidence from card 5.G1,
finisher P5-G1F — the clean-room parser. The 5.G2 decode-path
integration card is NOT landed: wave-7 close lists it among the
remaining cards, so no decode-path seam exists for HEVC yet, unlike
H.264's 5.F2). Front-matter keys per doc 04 §6:

- id = `codec-video-hevc` (hw row `not_started`, software row
  `blocked` — both consistent with this record; evidence backfill
  proposed)
- status = **parser implemented; decode blocked** (roadmap §1.1: hw
  track `not_started` pending 7.B adapters; software `blocked` on the
  rusty_h265-class young-crate gate + ADR-010)
- phase = 5 (parser landed)
- fuzz_target = `fuzz_codec_hevc` (planned name; fuzz-lite in-tree
  today)
- crates = [] — none admitted
- self_built = [`codecs::video::hevc` (parser: nal/vps/sps/pps/hvcc/
  slice — ITU-T H.265 v4 clean-room per the 5.F1 pattern)]
- last_audit = 2026-10-24 (this consolidation; parser evidence
  2026-10-2x per 5.G1)

## Parse / Decode / Encode

- **Parser (landed 5.G1):** 2-byte NAL header (fzb/layer/tid +
  Table 7-1 classification incl. is_vcl/is_irap); Annex-B 3/4-byte
  start codes + length-prefixed AV walk; emulation-prevention removal
  (chained overlapping escapes, pinned on the real 0100 VPS); hvcC
  record (ISO 14496-15 §8.3.3.1); VPS (shared profile_tier_level +
  HRD walker); SPS (chroma/bit-depths/geometry/conformance-window/
  sub-layer ordering/CTB+transform sizes/VUI); PPS (ids/tile grid/
  loop-filter/QP); **slice-segment header prefix** — first-slice/
  IRAP no-output/pps-id/dependent+CTB address/type/output-flag/
  colour-plane/POC-lsb. Everything from short-term RPS on is
  deliberately unparsed (the honest recorded subset).
- **Decode: none.** No `Decoder` impl, no hw seam (that was 5.G2,
  un-landed); HEVC tracks refuse typed at the codec registry today.

## Evidence (Task 5.G1)

- **Corpus differential (live ffprobe 7.1.5):** 3/3 lanes PASS —
  profile/tier/level/resolution/chroma/depth all agree:

  | lane | entry form | verdict |
  |---|---|---|
  | 0044-raw-video.265 | raw Annex-B | Rext/Main/63, 360×640 (coded 368×640 + conf-win), 8-bit, poc_bits 8 — PASS |
  | 0091-bframe-storm.ts | ffmpeg `-c copy -f hevc` extraction | Main/Main/63, 480×360 — PASS |
  | 0100-mp4-video.mp4 | real hvcC via ffprobe extradata hex | Main/Main/93, 1024×768; 4 arrays (VPS/SPS/PPS/prefix-SEI) byte-verified vs the live record — PASS |

- **Independent cross-check:** the parser was verified against a
  from-scratch Python bit-level SPS walk (spec order) — every field
  agrees; the core was right where the inherited test expectations
  were wrong (8 failures fixed by correcting the tests, incl. a
  builder that omitted per-array `numNalus`).
- **Fuzz-lite:** 10k random buffers framed as VPS/SPS/PPS +
  annexb/hvcC walks; 5k mutations of the real 0100 parameter sets +
  hvcC; 2 000 whole-stream mutations of 0044 — no panics, no hangs,
  all failures typed `HevcError`.
- **Tests at card gate:** 479 passed / 0 failed (19 new hevc lib
  tests + 3 corpus integration tests); fmt + clippy clean.

## Honest gaps

- **No decode** (above) — 5.G2 un-landed; hw adapters are the Phase-7
  7.B cards, software blocked (rusty_h265 0.6.2 / rust_h265 0.1.0
  young/unproven + ADR-010 distribution legality — matrix
  `codec-video-hevc-software` blocked_reason).
- **Full slice-segment header unparsed** (RPS/SAO/deblocking/
  entry-points — out of the honest subset by design).
- **Main-10/Rext >8-bit and 4:4:4 depth lanes:** corpus has none
  (C-V6 acquisition note).
- **Sub-layer streams** (max_sub_layers>1): walked, no corpus lane.
- **TS/MP4 entry through our own demuxers:** the corpus lanes used
  ffmpeg extraction + ffprobe-hex seams (sibling cards own the real
  paths); 0038/0099 hevc-in-AVI SKIP — ffprobe itself reports codec
  unknown (AVI fourcc hack), no oracle to differ against.

## Next steps

1. 5.G2 decode-path integration (the H.264 5.F2 analog: hw seam +
   typed fallback) when scheduled; then 7.B adapters.
2. Phase-6 software decision point: rusty_h265-class candidates stay
   behind the conformance gate (the rusty_vp9/ruopus
   validate-then-admit pattern would apply).
3. C-V6 acquisition for Main-10/4:4:4 lanes; own-demuxer entry forms.
