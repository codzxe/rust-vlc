# Audio record — ADPCM family (7 flavors landed, 1 typed gap)

Records-consolidation entry (card 5.REC-2; engine evidence from cards
5.J2a + 5.J2b + the 5.J2-MERGE fusion). Front-matter keys per doc 04
§6:

- id = `codec-audio-adpcm` (matrix row `not_started` + empty
  `evidence` is stale vs the landed decoder — backfill proposed)
- status = **implemented** for 7 descriptors; **typed-Unsupported**
  for the lettered `adpcm_oki` gap (below)
- phase = 5 (landed; WAV pair wave 4, breadth wave 5, fusion
  5.J2-MERGE)
- fuzz_target = `fuzz_codec_adpcm` (planned name; fuzz-lite survival
  suites in-tree today)
- crates = [] — **in-house, zero new dependencies** (D-018 logic; NOT
  symphonia-codec-adpcm)
- self_built = [`codecs::audio::adpcm` (mod/ms/ima/yamaha/oki/swf/
  misc — one `AdpcmDecoder`, 8 gate strings, 49 unit tests post-
  fusion)]
- last_audit = 2026-10-24 (this consolidation; evidence 2026-10-2x)

## Parse / Decode / Encode

One `AdpcmDecoder` behind the codec-api contract; `accepts()` = union
of 8 descriptor strings. Configure dispatch: WAV flavors
(`adpcm_ms` 0x0002, `adpcm_ima_wav` 0x0011) take the facts path (the
WAV demuxer's verbatim `fmt` payload via `DecoderConfig::extra_data`);
raw flavors (`adpcm_yamaha`, `adpcm_ima_oki`, `adpcm_oki`,
`adpcm_swf`, `adpcm_ima_qt`, `adpcm_ima_ws`) take the 16-byte tag-0
WAVEFORMATEX skeleton. The non-obvious bit laws (MS field-interleaved
block headers, multiplicative delta update; IMA low-nibble-first,
multiply-before-shift `(step·m)>>3`; SWF 2..5-bit code widths; QT
34-byte BE chunks) were all pinned empirically against ffmpeg output
before a line of Rust was written. Encode: out of scope.

## Evidence (Tasks 5.J2a + 5.J2b + 5.J2-MERGE)

- **WAV pair (corpus + fixtures, bit-exact, MD5 vs
  `ffmpeg -map 0:a -f md5`):**

  | file | descriptor | frames | verdict |
  |---|---|---|---|
  | corpus 0042-audio-only.wav | adpcm_ima_wav 2/44056 | 168 822 | bit-exact `72528c03…` |
  | corpus 0102-audio-only.wav | adpcm_ms 2/32000 | 128 524 | bit-exact `a1de073a…` |
  | fixture adpcm_ima_mono.wav | mono | 12 246 | bit-exact |
  | fixture adpcm_ms_mono.wav | mono | 12 216 | bit-exact |

  (Window = the DATA-derived extent both sides; the corpus files'
  under-declared `fact` chunks are damage-noted, not smoothed — 5.A2.)
- **Breadth five (8/8 bit-exact, ffmpeg 7.1.5):** adpcm_yamaha mono
  `aeddc262…` + stereo `fcf6aeca…` (NUT enc→dec); adpcm_swf stereo
  `9aac9305…` (FLV); adpcm_ima_qt stereo `a2cb46ce…` (MOV, 173 chunks
  = 346 real continuation seams); adpcm_ima_oki mono/stereo
  (forced-decoder oracle); adpcm_ima_ws mono/stereo (forced decoder).
  Pinned in `tests/adpcm_breadth_differential.rs` with an in-tree
  RFC-1321 MD5.
- **Fusion gate (5.J2-MERGE):** one decoder, 457 passed / 0 failed
  across 42 binaries; both differentials re-run green through the
  FUSED decoder; error codes from both parents preserved verbatim.
- **Throughput (measured, release, full pipeline):** 105–253 MB/s of
  decoded PCM (~1330× realtime at 44.056 kHz stereo on 0042).
- **Fuzz:** 512+ rounds WAV pair (flips/truncations/wild headers) +
  1 920 deterministic runs across the 5 breadth flavors — zero
  panics, decode never `Err`s (damage = typed outcomes/
  concealment), outputs within the pooled bound.

## Honest gaps

- **Lettered `adpcm_oki` (32k/24k): typed `adpcm.oki.no_reference` —
  NO ffmpeg 7.1.5 encoder or decoder exists** (checked
  `-encoders`/`-decoders` at card time). Accepted-but-refused at the
  gate; honest gap until real vectors are pinned.
- **Westwood vqa3 planar variant:** documented gap owed to the future
  VQA/AUD demuxer card (extradata seam).
- **QT continuation boundary** (adopt iff |Δ| > 0x7F): pinned by the
  corpus but the exact ffmpeg comparison operator was NOT
  source-verified — `[VERIFY]`.
- **Raw containers not landed:** .vox/.adp/SWF/MOV/AUD demuxers are
  the recorded future seam; facts ride extra_data until then.
- Big-endian hosts (native-endian s16 output; LE pinned by MD5s);
  >2-channel MS packing is a typed refusal (undefined in the format).

## Next steps

1. R3 registration (snippet in the 5.J2a worklog entry + the merge
   entry's 8-descriptor note).
2. `adpcm_oki` recheck when a reference appears (ffmpeg bump or
   committed vectors); vqa3 with the VQA/AUD card.
3. VLC differential when the oracle exists (bit-exact class).
