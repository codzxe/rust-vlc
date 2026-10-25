# Audio record — Vorbis (decode)

Records-consolidation entry (card 5.REC-2; engine evidence from card
5.B2, finisher P5-B2F). Front-matter keys per doc 04 §6:

- id = `codec-audio-vorbis` (matrix row `not_started` + empty
  `evidence` is stale vs the landed decoder — backfill proposed)
- status = **implemented** (decode + comments + granule timeline)
- phase = 5 (landed)
- fuzz_target = `fuzz_codec_vorbis` (3 surfaces: header chain, free
  block, contract path ×2 incl. discontinuity reset; libFuzzer CI
  Lead-owned)
- crates = [`symphonia-codec-vorbis =0.6.1 (MPL-2.0)`] — lock diff is
  exactly 4 nodes (codec-vorbis, symphonia-common,
  symphonia-metadata, regex-lite); NOTE: doc 04 §3.1's
  `symphonia-bundle-vorbis` spelling does not exist on crates.io —
  the 0.6.1 train's crate is `-codec-vorbis` (recorded at admission)
- self_built = [`codecs::audio::vorbis` (2 045 lines: headers/
  comments/seek/decoder/error/fuzz — the Ogg header chain, comment
  surface and granule arithmetic are ours, the engine is symphonia's)]
- last_audit = 2026-10-24 (this consolidation; engine evidence
  2026-10-2x, card 5.B2)

## Parse / Decode / Encode

- Decode: `VorbisDecoder` behind the codec-api contract; Ogg carriage
  via the native Ogg demuxer (5.B1). Output = f32 interleaved
  native-endian, no rate/channel conversion.
- Comments: vendor + user tags (TITLE/ARTIST/ALBUM/DATE…),
  case-insensitive lookup, repeatable tags.
- Timeline: pts = raw − measured shift on the delay/padding-trimmed
  timeline; single-data-page streams honestly expose `Missing` pts
  (never invented). Encode: out of scope.

## Evidence (Task 5.B2)

- **Differential (measured live, ffmpeg 7.1.5, f32le, aligned window,
  −0.0→+0.0 canonicalized):**

  | file | ch/rate | live maxΔ | verdict |
  |---|---|---|---|
  | corpus 0085-audio-only.ogg | 2/8000 | 5.96e-8 | PASS |
  | vorbis_sine_44100_stereo.ogg | 2/44100 | 3.73e-8 | PASS |
  | vorbis_sine_8000_mono.ogg | 1/8000 | 4.47e-8 | PASS |
  | vorbis_silence_48000_stereo.ogg | 2/48000 | 0 | **BIT-EXACT** |
  | vorbis_tagged_sine.ogg | 2/44100 | 2.98e-8 | PASS |

  Silence is the true byte-exact corridor (every conforming decoder
  emits exact zeros). MD5 pins per row in
  `tests/data/vorbis_reference.txt`.
- **Honest reference-data correction (recorded in-file):** the dying
  prior session had recorded raw−final-granule distances as head
  trims with unmeasured 1e-7 bounds (live delta ~9e-2 — every sample
  off). Corrected by exhaustive alignment scan: true head delays
  128 (44.1k) / 256 (8k mono) = the decoder's first-stamp shift
  measurement. The correction is documented inside the reference
  file — the honesty precedent this record cites.
- **Throughput (measured, release, median of 7):** 23 373 371
  inter-channel samples/s ≈ **2922× realtime** at 8 kHz stereo
  (corpus 0085, full ogg→decoder composition).
- **Corruption:** all-0xFF data packet post-CRC → exactly one typed
  `vorbis.packet.decode` refusal, decode resumes next packet (66/67
  blocks still emitted — the overlap-scoped one-block scar, doc 03
  §4.5(d)). Single flipped bytes inside a payload can still decode
  (entropy-coded local robustness) — the guaranteed-refusal lane uses
  a garbage block; recorded, not smoothed.
- **Fuzz:** 280 seeded mutants + 80 truncations + pristine lanes over
  the 3 harness surfaces — no panic, no hang, bounded allocation.
- **Tests at card gate:** 278 passed / 0 failed (27 binaries).

## Honest gaps

- **VLC oracle: PENDING-VLC** (no binary; digests asserted against
  ffmpeg; pinned dump command + version gate ready —
  `tests/vorbis_vlc_differential.rs`).
- **Vorbis-in-Matroska composition unexercised:** the MKV extra_data
  seam is ready and the header chain is container-agnostic, but no
  MKV-vorbis lane has pinned it yet (first such file pins it).
- libFuzzer CI campaign unwired (family status).
- Mid-payload single-bit robustness above (inherent to the format,
  surfaced as a known non-guaranteed-refusal case).

## Next steps

1. R3 registry row (snippet in the 5.B2 worklog entry).
2. VLC differential when the oracle exists (same f32 tolerance class).
3. First MKV-vorbis corpus/fixture lane through the extra_data seam.
