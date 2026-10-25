# Container record — MP4 / MOV / 3GP (ISO-BMFF family)

Status: **implemented** (demux, card 5.C1 core + finisher).
Feature-matrix row `container-mp4-mov-3gp` still reads `not_started`
with empty evidence — this record is that evidence; amendment
recorded in-file for the Lead (docs-only card 5.REC-1). Front-matter
keys per doc 04 §6:

- id = `container-mp4-mov-3gp` (`docs/feature-matrix.toml:141`)
- status = implemented (demux); platform-independent code, evidence
  Linux-only
- phase = 5 (landed; doc 04 §2.1's "M self-build / mp4parse" path
  was superseded by the native self-build per D-001/D-018)
- fuzz_target = `fuzz_demux_mp4` (in-crate fuzz + 14 committed
  malformed derivatives; doc 04 §8.5 registry name)
- crates = [] — zero external deps (mp4parse NOT used)
- self_built = [`crates/demux/src/mp4/**`]
- last_audit = 2026-10-24

## Scope landed (5.C1)

- Probe: ftyp brand gate (isom/qt/3gp/M4A …) or pre-ftyp
  moov/wide/free head; `EXTENSIONS = ["mp4","m4v","m4a","mov","qt",
  "3gp","3g2","3ga","3gpp"]` (mp4/mod.rs:138, verified this audit).
- **Progressive**: stbl sample tables (stsz/stts/stsc/stco/stss),
  file-offset interleave truth.
- **Fragmented**: moof/traf/trun with mvex/trex defaults, tfdt
  ladder, default-base-is-moof.
- **Edit lists**: first elst entry (empty-edit delay + media_time
  trim).
- **Chapters**: tref/chap QuickTime text traks — hidden from BOTH
  tracks and packets (consumed as metadata; the packet-leak bug was
  found and fixed by the regenerated 3-chapter fixture) + `chpl`
  fallback.
- **Attached pictures**: udta/meta/ilst/covr → synthetic
  still_image mjpeg/png track (track_id u32::MAX), ONE keyframe
  packet queued ahead of the media walk (ffmpeg attached-pic
  precedent; a seek past it does not re-deliver). Unknown cover
  types damage-note `cover-type-unsupported`.
- Handle alignment: packets carry the EXPOSED ordinal via an
  explicit `exposed` map (hidden chapter traks leave gaps).
- Seek: stss walk-back `Fast{indexed}` / bounded moof-scan
  `SlowLinear`; lands at-or-before target.
- Codec surface: h264, hevc, mpeg4video, mjpeg, png, vp8, vp9, av1,
  h263, aac, alac, ac3, eac3, opus, flac, mp3, amr_nb/wb, PCM
  family, mov_text.

## Corpus evidence (5.C1, corpus-v4, ffmpeg 7.1.5 — 30/30)

Track count + codec identity + per-stream packet counts +
first-video keyframe counts vs live ffprobe, over all 30 ISO-BMFF
lanes in the differential table (24 .mp4 + 5 .mov + 2 .m4a in this
corpus instance). Notable lanes:

| Lane | Truth | Verdict |
|---|---|---|
| 0029-cover-audio.m4a | aac (2 pkts) + attached-pic mjpeg (1 pkt, 1 keyframe, 14 566 B JFIF payload) | PASS (cover synthesis) |
| mp4_chapters.mp4 (regenerated parent) | 3 titled markers 0/600/1500 ms, monotone | PASS (chapter-trak leak fixed) |
| 0128-param-midchange.mp4 | mpeg2video (mp2v stsd) | **refuses typed `no-tracks`** — the MPEG-2 codec row's recorded vocabulary gap, unchanged |

Fixtures: 7 valid + 14 malformed derivatives through fuzz + typed
passes; edts first-PTS shift pinned.

## Error paths

Unknown stsd entries refuse typed downstream (never a guessed
codec); `demux.mp4.*` vocabulary; `traf-chapter-track` damage note;
no panics (fuzz-lite + in-crate fuzz_mutations green).

## Honest gaps (D-10, from 5.C1)

- `sidx` parsed-and-skipped (moof-scan seek covers the fragmented
  AC) — unchanged, recorded.
- 0128 (mp2v) typed `no-tracks` — MPEG-2 stsd mapping is the codec
  row's follow-up.
- Decoders themselves are out of scope (demux-only; h264 parser 5.F1
  rides `avcC` extra_data, HEVC hvcC cross-checked 5.G1 via the
  0100 lane, AAC 5.C2a rides esds).
- Decode-throughput (5.X1); 3GP-specific quirks beyond the brand
  gate are exercised only by whatever the corpus carries (no .3gp
  lane in corpus-v4 — [VERIFY] on acquisition).

## Gates (5.C1 finisher, exact)

fmt clean; clippy `-D warnings` EXIT 0;
`cargo test -p demux --all-features` 298/0/4 (unit 224/0;
corpus_mp4 2/2; mp4_fixtures 5/5). Merged wave-4 tree: codecs+demux
768/0 (5.W4-CLOSE). Workspace at wave-7 close: 3040/0.

## Next steps

- Lead: matrix-row amendment; MPEG-2 stsd entry when the MPEG-1/2
  decode decision lands (5.H1 sequence layer exists).
- sidx-based seek only if a corpus lane needs it.
