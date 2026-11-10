# Audio record — PCM (unpacked) decode

Records-consolidation entry (card 8.G1, issue #58 — the format-records
audit found the first-vertical audio codec was implemented with no
docs/formats record; engine evidence from card 2.B5). Front-matter
keys per doc 04 §6:

- id = `codec-audio-pcm` (matches `docs/feature-matrix.toml`; row
  status `implemented` on win/linux/macos, `not_started` on web, with
  evidence `crates/codecs/src/audio/pcm/**` — this record
  consolidates it)
- status = **implemented** (unpacked-PCM decode; in-house per D-018's
  zero-new-dependency logic — NOT symphonia-codec-pcm, the decision
  is recorded in the module header)
- phase = 2 (first vertical, the audio path of the AVI+MJPEG+PCM
  slice)
- fuzz_target = `fuzz_codec_pcm` (`fuzz/fuzz_targets/fuzz_codec_pcm.rs`;
  in-crate survival harness `crates/codecs/src/audio/pcm/fuzz.rs`)
- crates = [] — self-built, std-only, no `unsafe`
- self_built = [`codecs::audio::pcm` (`mod.rs`/`decoder.rs`/
  `error.rs`/`fuzz.rs`, 551 src lines in `mod.rs` alone)]
- last_audit = 2026-11-10 (this consolidation; engine evidence from
  card 2.B5 and its gates, re-run this session — below)

## Parse / Decode / Encode

- **Decode (landed)**: the full unpacked-PCM format matrix — bit
  widths, byte orders, unsigned/float shapes, planar/interleaved
  accounting — normalized to the media-types sample vocabulary;
  bit-exact acceptance against the in-repo fixture writer's
  NCO-generated s16le stream pinned by the committed
  `audio_stream_sha256` sidecar (the `[audio_probe]` closed loop).
- **Encode: not in v1** (the recording/conversion lanes use the WAV
  muxer for PCM carriage, not a PCM *encoder* — see
  `docs/formats/container-wav.md`).

## Evidence

- Module: `crates/codecs/src/audio/pcm/` — the module header records
  the 2.B5 dependency decision (in-house on C5 discipline,
  in-repo ground truth, output-format control).
- Fixture ground truth:
  `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (+ the
  `.oracle.sha256` sidecar pinning the decoded stream).
- First-vertical integration evidence:
  `tests/integration/first-vertical/evidence/ledger.json` (2.E1,
  mock-labeled).

## Gates (this consolidation, exact commands, run 2026-11-10)

Shared-target worktree `wt/w24e` @ `c339ee8`, 2-core Linux sandbox:

- `cargo test --locked -p codecs --lib audio::pcm` — **39 passed /
  0 failed** (the module's own unit suite; the broader `pcm::`
  filter also sweeps the ADPCM suites that share the substring).
- The release-binary headless decode of the parent AVI (whose audio
  track is PCM s16le) is exercised by the RC smoke row 3 —
  `docs/audit/rc-smoke-2026-11-10.md`.

## Known gaps / honest limits

- Linux-only evidence, mock-labeled audio output (null device —
  ADR-005's pending real-backend admission; `docs/KNOWN-LIMITS.md`
  §4).
- Web (`wasm32`): the wasm host ships the engine core decoder-less
  (the `codecs` crate's external engines do not build for the
  target; the in-house PCM decoder is native-side per the 7.H1
  record `apps/web/README.md`).
