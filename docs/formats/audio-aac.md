# Audio record — AAC (LC decode; ADTS/ADIF carriage)

Records-consolidation entry (card 5.REC-2; engine evidence from cards
5.C2a + 5.C2b). Front-matter keys per doc 04 §6:

- id = `codec-audio-aac-lc` (primary) + sibling `codec-audio-he-aac`
  (blocked row — unchanged by this record)
- status = **implemented** for AAC-LC (ADTS + ASC modes); HE-AAC
  honestly partial/blocked (below)
- phase = 5 (landed)
- fuzz_target = `fuzz_codec_aac` (harness landed; libFuzzer CI owed)
- crates = [`symphonia-codec-aac =0.6.1 (MPL-2.0)`] — lock diff is
  exactly 4 nodes (codec-aac, symphonia-common, symphonia-metadata,
  regex-lite); NOTE: the card brief's `symphonia-bundle-aac` does not
  exist on crates.io — the real crate is `-codec-aac` (doc 04 §3.1's
  name); 0 `unsafe` sites in codec-aac+metadata (`forbid(unsafe)` in
  both), common has no forbid but 0 sites (recorded as the honest
  weakening)
- self_built = [`codecs::audio::aac` (adts byte-level parse + resync
  walk, codec-api adapter, typed taxonomy, fuzz) +
  `demux::adts` (ADTS/ADIF raw demuxer, card 5.C2b)]
- last_audit = 2026-10-24 (this consolidation; evidence 2026-10-2x,
  cards 5.C2a/5.C2b)

## Parse / Decode / Encode

- **Decode (5.C2a):** `AacDecoder` — ADTS mode (one packet = one
  complete ADTS frame; 7/9-byte fixed-header parse, lazy engine from
  the first frame, per-packet param-switch gate) and ASC mode
  (`extra_data` = ASC, one packet = one raw block — the MP4/esds
  path). 1024-samples/frame pts bookkeeping; pooled 4-slot output.
- **Carriage (5.C2b):** `AdtsDemuxer` — syncword/layer/protection +
  CRC-16(0x1021) over header scope, 13-bit variable frame length,
  ID3v2 head+interframe skip (16 MiB ceiling), bounded 4 MiB resync
  with Recovered events; ADIF header+PCE parse with CBR block
  arithmetic; VBR/multi-program ADIF = typed refusals. Timestamps =
  exact `n·1024/rate` frame arithmetic.
- **HE-AAC honesty (doc 04 §3.2):** explicit-ASC HE (AOT 5/29) →
  typed `aac.he_sbr` Unsupported (unit-tested, never reaches the
  engine); non-LC profiles → `aac.profile`; channel-config 0 (PCE) →
  `aac.channel_config_pce`; >2 channels → `aac.channel_count`.
  Implicit-ADTS HE is UNDETECTABLE from the header — such streams
  decode as half-rate LC and the degradation is recorded, never
  silently passed.

## Evidence (Tasks 5.C2a + 5.C2b)

- **Codec differential (measured live, ffmpeg 7.x; window = all
  complete ADTS frames, both sides frames×1024):**

  | file | rate | ch | differing | maxΔ | verdict |
  |---|---|---|---|---|---|
  | corpus 0059-audio-only.aac | 88200 | 1 | 476 (0.13%) | 1 LSB | PASS |
  | corpus 0110-audio-only.aac | 96000 | 1 | 487 (0.13%) | 1 LSB | PASS |
  | corpus 0029-cover-audio.m4a (bridge) | 12000 | 2 | 26 (0.63%) | 1 LSB | PASS |
  | corpus 0094-audio-only.m4a | 12000 | 6 | — | — | TYPED `aac.channel_count` (ffmpeg decodes 6ch) |
  | sine441_lc_stereo.aac (gen.) | 44100 | 2 | 84 (0.03%) | 1 LSB | PASS |
  | sine48_lc_mono.aac (gen.) | 48000 | 1 | 53 (0.04%) | 1 LSB | PASS |

  Lumen+ffmpeg PCM MD5s recorded per row (`tests/data/
  aac_bitexact_reference.txt`; corpus re-imaging FAILS loudly).
- **Demux differential (5.C2b):** 0059 = 346 pkts payload-MD5-equal +
  ADTS roundtrip == source; 0110 = 376 pkts identical; dirty lane
  (tail-garbage) = 346 real frames byte-exact + 4 recovered garbage
  frames + sticky typed truncation — the ONE recorded divergence vs
  ffmpeg (351st short packet vs typed refusal); ADIF verified by
  cross-format agreement (no ffmpeg ADIF demuxer exists — measured).
- **Throughput (measured, release):** 0059 → 66.8 M samples/s = 758×
  realtime; 0110 → 109.2 M/s = 1137×; stereo fixture → 99.8 M/s.
- **Fuzz:** 80 seeded mutations of the committed base fixture through
  the full harness (32 decoded, 48 typed refusals, zero panics);
  demuxer fuzz-lite + committed mutation fixtures.
- **Tests at card gates:** 5.C2a 271/0/13; 5.C2b 331/0 (244 unit
  incl. 30 adts + 4 corpus incl. differential/dirty/fuzz).

## Honest gaps

- **HE-AAC/SBR/PS decode: absent** — typed refusal on explicit
  signaling, recorded half-rate-LC degradation on implicit (upstream
  engine has no SBR; doc 04 self-build M, unscheduled). This is the
  `codec-audio-he-aac` blocked row, unchanged.
- **>2ch + PCE:** typed partial (0094 row above); 6ch/8ch recorded
  partial.
- **VLC oracle: PENDING-INFRA** (the 5.A1 flac pattern).
- ISO 14496-4 conformance vectors are non-redistributable
  (license+size) — LC conformance = ffmpeg-authored fixtures + real
  corpus (the 5.A3 precedent, recorded in the reference file).
- m4a bridge quirk: ffmpeg's own probe refuses short extracted ADTS —
  forced-input on both sides, documented.
- libFuzzer CI unwired (family status).

## Next steps

1. R3 registry rows (snippets in both worklog entries; `adts` row:
   capabilities resync/id3-skip/adif-cbr/unsupported-typed).
2. VLC differential when the oracle exists (±1-LSB class).
3. HE-AAC stays a Phase-6+ decision (SBR self-build M vs engine
   swap); the detection/reporting duty is implemented today.
