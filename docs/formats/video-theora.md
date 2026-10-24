# Video record — Theora (decode)

Decision record (card 5.P2a-I part A). No code changed. Front-matter keys
per doc 04 §6:

- id = `codec-video-theora` (matches `docs/feature-matrix.toml`, which
  carries `status = blocked` on all four platforms with empty `evidence` —
  this record is that evidence)
- status = **blocked** (decode; Ogg carriage is landed, below)
- phase = 8 per matrix `phase_target`; a Phase-6 decision point is proposed
  below and may pull it earlier
- fuzz_target = `fuzz_codec_theora` (planned name, doc 04 §4.1 row — does
  not exist until a decoder is admitted)
- crates = [] — **none admitted** (`crates/codecs` video modules are the
  H.264 parser and Motion-JPEG; no theora engine)
- self_built = []
- last_audit = 2026-10-24 (this card's evidence pass; prior: feasibility
  `video-codecs.md` §1 row 4 + §11, 2026-10-19)

## Verdict (the card's question: blocked vs defer vs self-build-L)

**DEFER — Phase-6 decision point.** The matrix row stays `blocked`
(truthful: no decoder today, typed unsupported, never silent), but the
*category* is not H.264-software-style blocked and not a scheduled
self-build:

1. **Hard-blocked: REJECTED.** Nothing structural stands in the way — the
   Theora spec is public (Xiph, VP3-derived), the demux half is landed and
   test-pinned, and the effort is bounded at L (below). A "no path exists"
   framing would be false.
2. **Self-build L now: REJECTED for scheduling.** The share case cannot
   outrank the queued L-class codecs (MPEG-1/2, doc 04 §8.4 Phase 6;
   MPEG-4 ASP L–XL) that serve much larger real-world populations. Doc 04
   §8.4's own gate — "Theora (L) **if prioritized by Phase 0 share data**"
   — resolves to *not prioritized* on the numbers below.
3. **Recorded for cheap promotion:** the split-card self-build proposal
   (below) plus two monitor triggers, so a Phase-6 Lead decision is a
   promote, not an investigation.

## Evidence (all fetched this session, 2026-10-24, unless dated)

**crates.io — no admissible engine (live queries, this session).**
- `theora` → 0.0.0, "RESERVED. Open for adoption…", created 2017-09-19,
  2 009 downloads, 1 recent — confirms doc 04 §4.1 row's parenthetical.
- **NEW finding, missed by the 2026-10-19 feasibility survey:**
  `theora-rs` 0.1.0 (published 2026-10-08, 16 days old, single release,
  87 downloads): "Rust library-focused rewrite of the libtheora 1.2.0",
  MPL-2.0, **zero dependencies**, edition 2024. But its declared repository
  `github.com/xmoezzz/siglus_rs` is a visual-novel engine reimplementation
  whose README (master and main, fetched this session) makes **no Theora
  claim at all** — provenance unresolvable from the repo link; and a
  "rewrite of libtheora" is a derivation from the C reference, not a
  clean-room implementation (BSD licensing is fine; derivation-vs-clean-
  room is a doc 11 policy call). Below the adoption bar (feasibility §1.3
  young-crate precedent) → **recheck trigger, not candidate**.
- `oxideav-theora` 0.0.12 (2026-04-17→2026-09-01, 1 750 downloads):
  self-describes "orphan-rebuild scaffold pending clean-room
  re-implementation" — not a decoder (feasibility §1 row 4; same oxideav
  family as the `oxideav-wma` 5.K3-I rejection).
- `theorafile-rs` 0.1.2 (2024): bindings to the C `theorafile` —
  **forbidden** (doc 04 §1.2), listed to close the loop.
- NOT_FOUND: `libtheora`, `theora-sys`, `theora-decoder`,
  `symphonia-codec-theora`. **libtheora bindings status on crates.io:
  effectively nonexistent** beyond theorafile-rs; VLC's own theora decoder
  module is C and not portable under §1.2.

**Usage share (Phase 0 inventory).**
- `docs/inventory/formats.json` family `theora`: `usage_share = "P1"`,
  rationale "doc04 §4 P1 (Wikipedia-era free video; Commons
  application/ogg video 157,811 files)"; VLC modules `theora (video
  decoder)` + `theora (packetizer)`, probe green (ogg_theora.log, exit 0).
- The honest datum (usage_share_sources, Commons Special:MediaStatistics,
  fetched 2026-10-19): even in the friendliest possible population — a
  repository with a mandatory free-format policy — application/ogg video
  is **157 811 files vs 2.62 M ogg-audio and 1.72 M wav** in the same
  population. General-player share is far below that.
- **Internal inconsistency flagged for the Lead:** formats.json + the
  matrix row say P1; doc 04 §1.3 lists Theora under P2 "real but niche";
  doc 04 §4.1 table says P1. The share data above supports the P2 read.
- corpus-v4: **1 Theora lane of 150 media lanes (0.67 %)**.

**Corpus lane (ffprobe 7.1.5, this session).**
`0090-ogv-video.ogv` (47 900 B): theora, 360×640, yuv420p, 0.5 fps
(time_base 2/1), 4 s, **video-only — single stream, no Vorbis track**.
Consequence the matrix `blocked_reason` should absorb: typical theora+vorbis
.ogv degrades to audio-only playback, but this lane degrades to **zero
playable tracks** (typed unsupported at open).

**Landed state in this tree (no code changed by this card).**
- Carriage works: `crates/demux/src/ogg` maps the `\x80theora` BOS magic →
  `OggCodec::Theora`, parses the info header (PICW BE24@14, PICH BE24@17,
  FRN BE32@22, FRD BE32@25, KFGSHIFT), and computes `Clock::Theora`
  granule→frame arithmetic — frame `n = (g >> shift) + (g & mask)`, keyframe
  inference from the granule high bits — test-pinned against
  libtheora-encoded fixtures. The demux half of Theora support is **sunk
  cost**; the L below is decode-only.
- Decode: none — `crates/codecs` registers no theora engine; the `theora`
  descriptor surfaces the engine's typed unsupported path.

**Self-build L estimate (why L, not XL).**
Theora is VP3-derived (On2 VP3 open-sourced 2001 → Xiph Theora): 8×8
integer DCT, half-pel motion compensation, **all-Huffman entropy coding
(no CABAC)**, no B-frames, single reference frame — materially simpler
than VP8/VP9 (doc 04 §4.1 sizes those XL vs Theora's L) and comparable to
the MPEG-1/2-class L items. Oracles exist: ffmpeg 7.1.5 (libtheora-backed
encode/decode for differential + vector generation) and the VLC 3.0.23
baseline probe.

## Split-card self-build proposal (recorded, NOT scheduled)

- **6.?a (M) — unpack half:** identification/comment/setup header chain
  (0x80/0x81/0x82 BOS pages), the ~80 static Huffman codebooks, frame
  header + per-macroblock mode/token decoding → coefficient arrays.
- **6.?b (M) — reconstruct half:** dequantization, inverse integer
  transform, half-pel motion compensation + prediction, loop filter,
  YUV420P frame assembly (planar output per codec-api).
- **6.?c (S–M) — admission:** codec-api `VideoDecoder` glue (zero new
  dependencies, C5 pattern), `fuzz_codec_theora` + seed corpus
  (ffmpeg-generated vectors + corpus 0090), differential vs ffmpeg 7.1.5
  and VLC 3.0.23, this record → `implemented` with evidence.
Sum: **L (8–26 person-weeks)**, consistent with doc 04 §4.1 row.

## Next actions

1. **Phase-6 decision point (Lead):** promote the split only if (a)
   free-culture/.ogv demand materializes (Commons-class content is the
   realistic customer), or (b) the video self-build pipeline wants a
   lowest-risk first decoder — Theora's no-B-frame, all-Huffman shape is a
   gentler pipeline-prover than MPEG-2.
2. **Monitor each wave:** re-query `theora-rs` (ruopus-style
   validate-then-admit **if** it reaches a stable 0.1+ with conformance
   evidence and resolvable provenance — an S spike would then beat L
   self-build); recheck `oxideav-theora` at the Phase-7 R-10-class pass.
3. **Until any of the above lands:** matrix stays `blocked` ×4, typed
   refusal, honest reporting — typical .ogv = audio-only playback; corpus
   lane 0090 = zero playable tracks.

## Matrix-row proposal (for the Lead; feature-matrix.toml untouched — investigation scope)

- Keep `id`/`status = blocked ×4`/`phase_target = 8`; **backfill
  `evidence`** = "docs/formats/video-theora.md (5.P2a-I, 2026-10-24)".
- Amend `blocked_reason`: append the new facts — `theora-rs` 0.1.0
  (2026-10-08) exists but provenance-unresolvable and below the adoption
  bar; Phase-6 decision point with the split proposal on file.
- `notes`: point "Revisit trigger: usage demand vs L effort" at this
  record's split proposal + monitor triggers.
- Doc drift for the Lead (out of scope here): resolve the P1 (§4.1 row,
  formats.json, matrix row) vs P2 (§1.3 list) priority inconsistency —
  the share data supports P2; feasibility §11's "Theora (P1, missing)"
  framing likewise.
