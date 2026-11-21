# WAVE 7 — SUMMARY (audio codecs)

Date: 2026-11-21 (session 2) · Agents W07-A01…A10 · Domain: audio codecs — PCM,
Symphonia wrappers, AAC/MP3/MP2/FLAC/Vorbis/Opus/ALAC/ADPCM, audio-engine
(mix, resample, pipeline, effects), TrueHD/AC-3/MIDI claims.
Gate: **PASS — 75/75 assignments audited, 0 missing, 0 thin, citation
spot-checks clean.** (One agent named two chunk files `.partN` instead of
`#partN`; renamed by the orchestrator, content untouched. Two manifest header
lines inflated the verdict histogram; ignored in counts below.)

## Verdicts (file/chunk level)
REAL 54 · REAL-UNPROVEN 5 · SHALLOW 2 · UNREACHABLE 14

## Findings
**S0:0 S1:3 S2:15 S3:62 S4:117 — total 197**

## The wave's hostile question: AAC-in-MP4 end to end?
Convergent verdict of 6+ agents, independently traced:
- **AAC-in-MP4 decodes end to end in the shipped CLI**: baseline
  `sample-aac.mp4` → Playing, exit 0 (audio goes to the null device — no real
  output backend exists, Wave 8's question).
- **`lumen engine-info`'s "decoders: mjpeg, pcm_s16le" is a hard-coded lie**
  (established W05 correction v2, re-confirmed here): the shipped binary
  actually reaches 9+ live audio decoders (PCM, AAC, MP3, MP2, FLAC, Vorbis,
  Opus, ALAC, ADPCM family). The tooling under-reports by design.
- **No sample-rate conversion exists anywhere.** The only resampler is a
  ±1000 ppm drift micro-corrector, and it is dormant in the shipped build;
  the pipeline refuses rate mismatches with a typed error.
- **TrueHD, AC-3, MIDI: zero decoder code** — any doc claiming them is
  PAPER-ONLY (matrix rows land in Wave 18).

## Worst findings (citations in the per-file audits; sampled at gate)
1. [S1] `mp3/id3.rs:387,410` `parse_picture` slice-index panic on 0–4-byte
   APIC/PIC frames — violates the crate's "no panics on any input" contract;
   reachable via pub `decode_stream` + registered fuzz target (W07-A09).
2. [S1×2, W07-A10] static-only panic/break proofs in convert.rs family (no
   rustc for agents; REQUEST-ORCH-RUN filed to re-prove at gate).
3. [S2] Every audio seek on counter-pts decoders (ALAC/MP3/AAC) re-anchors
   the clock at 0 and reports `Seeked.actual=0` — every audio seek lands at
   position 0; FLAC/Vorbis anchor correctly (W07-A08, family-wide).
4. [S2] MP3 Layer-III 529-sample latency trim applied to every descriptor
   incl. MP2/MP1 and tagless MP3 (ffmpeg measurement: no head-skip without a
   LAME tag) — every shipped MP3/MP2 playback misaligned ~12 ms (W07-A04).
5. [S2] Ogg-Opus seeks re-apply stream-head pre-skip: 312 samples / 6.5 ms
   dropped per seek; the test pins pts only, not the window (W07-A01).
6. [S2] FLAC-in-Ogg cannot decode in the shipped build — containers/ogg.rs
   seeds the 38-byte block-header-prefixed STREAMINFO where configure needs
   the bare 34-byte body → typed refusal at an untested seam (W07-A10).
7. [S2] MP2/MP3 geometry change (channels/rate) mid-stream: first-frame latch
   silently emits stale-mixed samples at the wrong rate on the shipped PS lane
   (W07-A05).

## Blocked / UNVERIFIED (agents' items)
- External-crate internals (symphonia, ruopus, claxon panic-freedom) — no
  vendored sources in sandbox; record-cited only.
- Corpus-gated opus evidence vacuous in baseline (REQUEST-ORCH-RUN filed).
- `cargo test --doc` never run in baseline (standing REQUEST-ORCH-RUN).
