# Container record — WAV (RIFF/WAVE, incl. RF64/BW64)

Status: **implemented** (demux, card 5.A2). Feature-matrix row
`container-wav` still reads `not_started` with empty evidence — this
record is that evidence; the row amendment (status → implemented,
evidence → this file) is recorded in-file for the Lead (matrix is
Lead-owned, docs-only card 5.REC-1). Front-matter keys per doc 04 §6:

- id = `container-wav` (`docs/feature-matrix.toml:219`)
- status = implemented (demux); platform-independent code, all
  evidence Linux-only (the container-avi precedent)
- phase = 5 (landed)
- fuzz_target = `fuzz_demux_wav` (in-crate `wav/fuzz.rs` + the +FB
  mutation families; the doc 04 §8.5 registry name)
- crates = [] — **zero external deps**, C5 std-only
- self_built = [`crates/demux/src/wav/**` (mod/format/fuzz/error/
  tests/test_support)]
- last_audit = 2026-10-24 (this audit; card landed earlier in Phase 5)

## Scope landed (5.A2)

- Probe: 12-byte head, magic ∈ {RIFF, RF64, BW64} and form `WAVE`;
  `EXTENSIONS = ["wav","wave","rf64","bw64"]` (wav/mod.rs:132,
  verified this audit).
- Codecs carried: PCM s8..s64 / u8 / f32 / f64 (LE), ADPCM-MS
  (format tag 0x0002), ADPCM-IMA-WAV (0x0011), FLAC-in-WAV
  (WAVE_FORMAT_EXTENSIBLE 0xFFFE + FLAC GUID — `frames()` = None, no
  packet stamps; frame alignment owned by the FLAC decode path
  [PROPOSAL 5.A1]).
- WAVE_FORMAT_EXTENSIBLE body parse (0xFFFE without a body = typed
  structural lie); odd-sized chunk pad-byte skip; LIST/INFO metadata
  via `info()`.
- Seek: `Fast { indexed: false }` (arithmetic framing — no index
  needed).
- Zero-copy: frames PCM by arithmetic (no per-packet parse); 2-slot
  `PayloadSlots` pool, `MAX_PACKET_BYTES` = 64 KiB; steady-state
  per-packet allocations zero; `pool_bytes()` observable.

## Corpus evidence (5.A2, corpus-v4, ffmpeg 7.1.5 — 12/12 PASS)

Params + frame counts vs ffprobe; payload MD5 vs
`ffmpeg -map 0:a -c copy -f md5` (bit-exact):

| Lane | Codec | Frames | Note |
|---|---|---|---|
| 0004, 0041, 0095, 0138 | pcm_s24le/s16le/s32le/s16le | 32 000–176 400 | MD5 parity |
| 0013, 0026, 0129, 0135 | pcm_s16le/u8 (sr-chaos ×2) | 24 255–61 233 | MD5 parity |
| 0006 | pcm_s24le 96 kHz | 388 433 | MD5 parity |
| 0016 | flac-in-wav | 384 000 | MD5 parity |
| 0042 | adpcm_ima_wav | 168 822 | `fact` under-declares (probe 167 853) — damage-noted `fact-frames-short-of-data`, not smoothed |
| 0102 | adpcm_ms | 128 524 | same damage note (probe 128 000) |

## Error paths (typed, one test each — 5.A2 audit)

Truncated head / mid-header; declared-size-past-actual; zero data
size; odd-chunk pad skip / odd-data pad strip; 0xFFFE-without-body;
unknown tags + unsupported depths = typed refusals. +FB: wav_*
mutated families (flip/garbage/trunc ×5) with pinned exact outcomes
+ fuzz-harness survival; 20 malformed fixtures committed.

## Honest gaps (D-10, from 5.A2)

- Decode-throughput NOT measured (5.X1 owns the bench; demux is O(1)
  arithmetic framing).
- No wav row in `tests/zero_alloc_read_chain.rs` yet (the natural
  5.X2 extension; pool bounds are unit-pinned, not chain-proven).
- RF64 > 4 GB real file untested (probe accepts the magic; no corpus
  lane — [VERIFY] on a generated large-file lane if ever needed).
- FLAC-in-WAV packet stamps absent by design (see above).

## Gates (5.A2, exact)

`cargo fmt -p demux` clean; clippy `-D warnings` clean;
`cargo test -p demux --all-features` PASS (163 unit + 8
wav_aiff_fixtures + 2 corpus_wav at card time). Workspace gate at
wave-7 close: 3040 passed / 0 failed (5.W7-CLOSE).

## Next steps

- Lead: matrix-row amendment (status/evidence backfill).
- 5.X2: zero-alloc chain row. 5.X1: demux framing bench.
