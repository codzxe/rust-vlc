# Container record — Ogg (chained physical streams)

Status: **implemented** (demux, card 5.B1). Feature-matrix row
`container-ogg` still reads `not_started` with empty evidence — this
record is that evidence; amendment recorded in-file for the Lead
(docs-only card 5.REC-1). Front-matter keys per doc 04 §6:

- id = `container-ogg` (`docs/feature-matrix.toml:154`)
- status = implemented (demux); platform-independent code, evidence
  Linux-only
- phase = 5 (landed)
- fuzz_target = `fuzz_demux_ogg` (in-crate `ogg/fuzz.rs` + fuzz-lite
  sweeps; doc 04 §8.5 registry name)
- crates = [] — zero external deps
- self_built = [`crates/demux/src/ogg/**` (mod/page/error/fuzz/
  test_support/tests — 2 651 lines at 5.B1)]
- last_audit = 2026-10-24

## Scope landed (5.B1)

- Probe: `OggS` magic (4 B); `EXTENSIONS = ["ogg","oga","ogv","opus",
  "spx"]` (ogg/mod.rs:115, verified this audit).
- Pages: CRC-32 verification (bad CRC → packet drop, damage-noted,
  walk continues), 255-run lacing overrun bound, sync-scan recovery
  after mid-stream capture corruption, sequence-discontinuity
  detection, truncated page typed.
- Logical streams: N serials per link, interleaved; granule clocks
  per codec (Vorbis/Opus/Theora); granule regressions/holes/seq-gaps
  surface as Discontinuity events — never smoothed.
- **Chained streams** (`cat` of links): each link opens fresh track
  ids behind a Discontinuity event; per-serial granule clocks
  restart (no fabricated cross-link timeline).
- Seek: `SlowLinear` — granule-mapped linear scan bounded by
  MAX_SEEK_SCAN_PAGES.
- Codec mappings carried: vorbis, opus, theora, flac-in-ogg, speex,
  ogg-opaque (unknown → opaque payload, never a guess).

## Corpus evidence (5.B1, corpus-v4, ffmpeg 7.1.5 — 4/4 PASS)

Packet counts + EOS granules vs ffprobe; payload MD5 vs
`ffmpeg -c copy -f md5` (bit-exact):

| Lane | Codec | Packets (ours = ffprobe) | Granule | MD5 |
|---|---|---|---|---|
| 0047-audio-only.ogg | opus 48 k mono | 201 = 201 | 192 312 | f2d26449… ✓ |
| 0077-audio-only.ogg | opus 48 k 6 ch | 201 = 201 | 192 312 | fb151c11… ✓ |
| 0085-audio-only.ogg | vorbis 8 k stereo | 126 = 126 | 32 000 | d0e0f86e… ✓ |
| 0090-ogv-video.ogv | theora 360×640 | 2 = 2 (frames) | 2 (4.0 s) | c0cd6bbf… ✓ (video map) |

Plus generated fixtures: 3 parents (vorbis 8 k stereo / vorbis 16 k
mono / opus 48 k mono) bit-exact (14/14/21 packets, md5s 335fd5…/
73b12e…/3e6f08…) + a real two-link `cat` chain: boundary =
Discontinuity, link 2 fresh track id, per-serial clocks restart,
payload byte-equal to parent A ‖ parent B. 9 derivatives (5 exact
prefix truncations, 3 single-bit flips, +4096 B garbage) with
outcomes pinned to the typed vocabulary.

## Honest divergences (measured, recorded — 5.B1)

- Two-link chain: ffmpeg reports 31 packets (it strips only the
  FIRST link's Vorbis headers; link 2's 3 header packets leak as
  data); we stash both links' headers → 28 packets. Payload =
  two ffmpeg-verified parents concatenated. Recorded in the fixture
  suite module docs.
- ffprobe-side channels/dims are decode-domain (header-stashed by
  us — noted, not asserted).

## Error paths / fuzz

Typed `demux.ogg.*` vocabulary; fuzz-lite green over all fixtures +
derivatives + 4 KiB-stride truncation/flip sweeps + ALL dirty/
(27) + garbage/ (22) real-world files.

## Honest gaps (D-10, from 5.B1 + later cards)

- **Multi-link cross-chain seek NOT tested** — single-link seek is
  tested; cross-link seek targets documented as link-local clocks.
- Decode-throughput NOT measured (5.X1); no ogg row in
  `tests/zero_alloc_read_chain.rs` (5.X2; pool measured 80 B..4 KiB
  live, 2-slot × 256 KiB ceiling).
- **Kate is NOT an OggCodec variant** — falls to Unknown/ogg-opaque.
  DOC-DRIFT: the matrix Ogg note claims "carries … Kate … mappings"
  (flagged by 5.P2b-I; still unamended — Lead).
- Speex mapping carried but no corpus lane and no decoder (5.B1's
  opaque class; Speex decode = unscheduled).
- Skeleton / Opus-preskip edge cases (doc 04 §2.1 row): preskip is
  decode-domain; skeleton untested [VERIFY].

## Codec rides (later cards, cited)

Vorbis decode landed (5.B2, corpus bit-exact); Opus decode BLOCKED
pending ruopus validation (5.K2-I record `audio-opus.md`); Theora
demuxed, decode deferred Phase 6 (5.P2a-I record).

## Gates (5.B1, exact)

fmt clean; clippy `-D warnings` clean after 7 fixes;
`cargo test -p demux --all-features` 265/0/4 (6 new ogg tests).
Workspace at wave-7 close: 3040/0 (5.W7-CLOSE).

## Next steps

- Lead: matrix-row amendment (incl. the Kate note correction).
- 5.X2 zero-alloc row; cross-chain seek if chained files appear.
