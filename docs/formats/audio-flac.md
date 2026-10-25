# Audio record — FLAC (decode)

Records-consolidation entry (card 5.REC-2; engine evidence from card 5.A1,
finisher P5-A1-R2). Front-matter keys per doc 04 §6:

- id = `codec-audio-flac` (matches `docs/feature-matrix.toml`; matrix
  `status = not_started` rows + empty `evidence` are STALE vs the landed
  decoder — evidence backfill proposed for the Lead's matrix pass)
- status = **implemented** (native-FLAC decode + seek; roadmap vocabulary
  §1.1: `pure-rust-exists` verified by admission)
- phase = 5 (landed)
- fuzz_target = `fuzz_codec_flac` (harness logic in-crate; libFuzzer CI
  wiring is the Lead-owned family gap)
- crates = [`claxon =0.4.3 (Apache-2.0)`]
- self_built = [`codecs::audio::flac` (2 382 lines: decoder/metadata/
  seek/error/fuzz — header chain, SEEKTABLE walk, error taxonomy, glue)]
- last_audit = 2026-10-24 (this consolidation; engine evidence 2026-10-2x,
  card 5.A1; purity record `docs/dependencies/records/claxon-0.4.3.md`)

## Parse / Decode / Encode

- Decode: `codecs::audio::flac::FlacDecoder` behind the codec-api
  `Decoder` contract, engine `claxon =0.4.3`. STREAMINFO + frame decode,
  4–32-bit integer depths, 1–8 channels, Vorbis-order channel
  conventions; metadata blocks (Vorbis comments, PADDING, pictures)
  surfaced through our own `metadata.rs`.
- Seek: native SEEKTABLE when present; verified binary-search fallback
  over frames (flac_seek.rs). Gapless by construction (sample-count
  domain).
- Encode: not in v1 (flacenc 0.5.1 audited for the Phase-7 path per
  doc 04 §3.1).
- Typed rejections, never panics: channel-mismatch class pinned as
  `flac.frame.channels`; mislabeled files refuse at probe
  (`flac.probe.magic`, corpus 0071 ID3/MP3-mislabeled).

## Evidence (Task 5.A1; re-verified at merge)

- **Bit-exactness (measured):** 4 ffmpeg 7.1.5-authored fixtures
  byte-exact vs ffmpeg PCM dumps **and** the STREAMINFO MD5
  (`tests/data/flac_bitexact_reference.txt` records commands + MD5s);
  corpus 0046 (88.2 kHz/24-bit, 44 frames) byte-exact both directions;
  0106 metadata-only (0 frames — matches ffmpeg's own 0 packets).
- **Throughput (measured, release, median of 7):** 47 552 483
  inter-channel samples/s ≈ **539× realtime** at 88.2 kHz stereo 24-bit
  (corpus 0046, full contract path; `tests/flac_bench.rs`).
- **Fuzz:** survival suite over bit-flip sweeps, 512-seed classes,
  corpus dirty/mutant lanes — typed errors only, zero panics/hangs
  (13.1 s exhaustive flip sweep is an opt-in `--ignored` row).
- **Tests at card gate:** `cargo test -p codecs --all-features` =
  208 passed / 0 failed / 11 ignored (documented opt-ins). Workspace
  at wave-7 close: 3040/0.
- **Purity:** tarball sha256 4bfbf567…0688, no build.rs, no links=, no
  extern/asm (F1–F7 NO), Apache-2.0, zero normal deps — Cargo.lock diff
  is the single claxon node.

## Honest gaps

- **VLC oracle: PENDING-INFRA** — `tests/flac_vlc_differential.rs`
  exists with pinned dump command + version gate, but no vlc/cvlc
  binary exists in the sandbox; ffmpeg is the always-on oracle.
- **Ogg-FLAC: not exercised end-to-end** — the Ogg demuxer (5.B1)
  carries the flac-in-ogg mapping, but no corpus/fixture lane was
  decoded through the pairing (out of 5.A1's card scope, recorded
  there).
- **32-bit depth:** claxon supports it; no fixture covers it.
- **wasm32 build of claxon:** `[VERIFY-P1]` flagged for the first wasm
  gate (5.A1 stage summary).
- libFuzzer CI campaign unwired (harness logic proven by in-tree
  survival tests; same status as mp3/vorbis/aac/mjpeg/pcm).

## Next steps

1. R3 registry row (snippet drafted in the 5.A1 worklog entry: id=flac,
   capabilities probe/decode/seek).
2. VLC differential the moment an oracle binary exists (digests must
   agree bit-exact — lossless codec, no tolerance class).
3. Ogg-FLAC + FLAC-in-WAV pairing lanes (WAV demuxer 5.A2 already
   surfaces 0xFFFE+FLAC GUID; frame alignment recorded as the codec
   card's ownership).
4. wasm32 gate + fuzz CI (family items).
