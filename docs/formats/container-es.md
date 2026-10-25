# Container record — Raw elementary-stream wrappers (ES: .264/.265/.mp3/.aac)

Status: **implemented** (demux, card 5.I3a; `.aac` = the landed ADTS
row `container-adts-aac`, consumed as-is). Matrix rows
`container-h264-annexb`, `container-hevc-annexb`,
`container-mp3-raw` still read `not_started` with empty evidence —
this record is that evidence; amendments recorded in-file for the
Lead (docs-only card 5.REC-1). Front-matter keys per doc 04 §6:

- id = `container-h264-annexb` + `container-hevc-annexb` +
  `container-mp3-raw` (`docs/feature-matrix.toml:310,323,284`)
- status = implemented (demux, the three rows above);
  `container-mpeg-es-video` (.mpgv/.m1v/.m2v) = **not_started**
  (see gaps)
- phase = 5 (landed)
- fuzz_target = `fuzz_demux_es` (in-crate fuzz-lite ×512 rounds;
  doc 04 §8.5 registry names per family)
- crates = [] — zero external deps
- self_built = [`crates/demux/src/es/**` (annexb/h264/hevc/mp3/aac/
  error/mod + tests)]
- last_audit = 2026-10-24

## Scope landed (5.I3a)

- **Shared Annex-B engine** (`annexb.rs`): windowed start-code scan
  (256 KiB window + 32 B carry; multi-MB NAL bodies skipped unread —
  only NAL headers classified); NAL-type-aware AU boundaries:
  non-VCL prefix NAL after VCL opens an AU (measured: ffmpeg packet
  19 of 0019 starts at the mid-stream SPS at 4338, not the slice);
  VCL-after-VCL opens one on H.264 `first_mb_in_slice==0` / HEVC
  `first_slice_segment_in_pic_flag` (single escape-free bit
  probe); zero-walk-back clamp; `MAX_AU_BYTES` 16 MiB typed
  ceiling; 2-slot PayloadSlots pool.
- `es::h264` — `EXTENSIONS ["h264","264","avc"]`;
  `es::hevc` — `["h265","265","hevc"]` (VPS/SPS/PPS 32/33/34 in
  the prefix class; filler/suffix-SEI deliberately excluded).
- `es::mp3` — ID3v2.2/2.3/2.4 skip at head AND between frames;
  strict first-frame validation; bounded resync with Recovered
  events; exact sample-clock pts on 1/rate base; **Xing/Info/VBRI
  first frame packet-skipped** (the measured ffmpeg behavior:
  0139's VBR header frame at 44 is not a packet, ffmpeg packet 1
  starts at 1088). `EXTENSIONS ["mp3","mp2"]`.
- `es::aac` — the landed `adts::AdtsDemuxer` consumed as-is
  (registry surface only).
- Timing honesty: raw Annex-B carries no stamps — video packets
  `Stamp::Missing`/no duration (ffmpeg's own pts for 0019 are N/A,
  measured); mp3/aac use exact sample arithmetic.

## Corpus evidence (5.I3a, corpus-v4, ffprobe 7.1.5 — 6/6)

Packet counts AND exact AU boundary geometry (sizes byte-equal ⇒
contiguous coverage):

| Lane | Demuxer | Ours = ffprobe | Boundary geometry |
|---|---|---|---|
| 0019-raw-video.h264 | es::h264 | 26 = 26 | ALL 26 AU sizes byte-equal; first packet [0,1135) incl. the stray leading byte (ffmpeg's own shape) |
| 0044-raw-video.265 | es::hevc | 56 = 56 | first 8 AU sizes exact (19 745, 13 800, …); total = file length |
| 0139-audio-only.mp3 | es::mp3 | 79 = 79 | MPEG-2.5 L3 11 025 Hz stereo, 576-sample frames; pts step 576 verified |
| 0145-cover-audio.mp3 | es::mp3 | 6 = 6 audio | ID3 APIC cover skipped as metadata (divergence below) |
| 0059-audio-only.aac | es::aac | 346 = 346 | landed adts walk |
| 0110-audio-only.aac | es::aac | 376 = 376 | landed adts walk |

## Honest divergences (measured, recorded — 5.I3a)

- Keyframe flags: ours IDR-only; ffmpeg additionally marks packet
  19 of 0019 K via recovery-point SEI (decode-domain).
- 0145: ffmpeg synthesizes an attached-pic mjpeg packet; we skip
  the ID3 APIC as metadata (player behavior, not framing).

## Error paths / fuzz

Typed `demux.es.*`: no-sync (Unsupported-after-scan, sniffed bytes,
sticky), truncated (sticky — ffmpeg emits a short packet instead:
the adts-recorded divergence), AU ceiling (ResourceLimit),
bad-header, not-started. +FB fuzz-lite: seeded-LCG ×512 rounds over
a real-shaped h264+hevc+mp3 hybrid — no panic/hang, Ok-or-typed
only.

## Codec rides (later cards, cited)

5.F1a/F1b: h264 parser corpus 19/19 (0019 High@1.3 64×48).
5.G1: HEVC parser 0044 Rext lane field-verified. 5.A3: MP3/MP2
decode + gapless (metadata twin noted — layer ordering forbids
sharing the crate, 5.I3a).

## Honest gaps (D-10, from 5.I3a)

- **`container-mpeg-es-video` (.mpgv/.m1v/.m2v) NOT implemented** —
  `es/` = aac/annexb/h264/hevc/mp3 only (re-verified by 5.K4-I);
  the raw MPEG-1/2 video ES row stays not_started (the 5.H1
  sequence layer is codec-side; its raw .m2v fixtures were
  ffprobe-differenced directly, not through an es wrapper).
- >16 MiB AUs refuse typed (not measured — the ceiling is the
  policy).
- Big-endian hosts untested [VERIFY].
- Decode-throughput (5.X1); zero-alloc chain row (5.X2).

## Gates (5.I3a, exact)

fmt CLEAN; clippy EXIT 0; `cargo test -p demux --all-features` ALL
GREEN — 310 lib (12 new es) + every integration suite incl. the new
es_corpus (4). Merged wave-5 tree: workspace 2752/0 (5.W5-CLOSE).

## Next steps

- Lead: three matrix-row amendments + the mpeg-es-video row stays
  honest (not_started).
- Optional S card: `.m1v/.m2v` wrapper reusing the 5.H1 start-code
  scanner if raw MPEG video demand appears.
