# Codec record — WMV1/2/3 + VC-1 (WMV9 advanced) video decode

Status: **deferred (later) — decision record (card 5.P2b-I part B)**. Split
verdict per doc 04 §4.1 row "VC-1 / WMV9" + feasibility video-codecs §1
row 11: software decode = `missing`, **hw-first** where the OS provides it
(ADR-001): Windows in-box "WMV family" MFTs, Linux Intel-class VC-1 +
NVIDIA shim; **macOS = blocked** (no VideoToolbox exposure, no pure
software decoder); **web = unsupported** (not in the WebCodecs registry).
`vc1.software = missing` stays forever-visible in the matrix; no software
card is scheduled in v1. The container half (ASF demux) is a separate
Phase-6 proposal (5.K3-I → `container-asf.md`).

- Date of evidence: **2026-10-24** (crates.io API, this card).
- Supersedes-cites: doc 04 §4.1 row + §4.5, feasibility video-codecs §1
  row 11 + §5 (2026-10-19), feature-matrix row `codec-video-wmv-vc1`
  (win/linux not_started, macos blocked, web unsupported).

## Parse / Decode / Encode

Nothing implemented. When the Phase-6 ASF container lands, WMV/VC-1 video
streams must ride the **typed-unsupported** semantics decided in
`container-asf.md` / `audio-wma.md` (5.K3-I): demux + track listing +
play other streams + typed `unsupported-decoder` error naming the missing
decoder. A **pure-Rust H.264-class parser analog (NAL/SPS subset) does not
exist for VC-1 either** — profile/level probing of `.wmv`/`.vc1` streams
is part of the same deferred work. Encode: never planned.

## Evidence (what was checked, 2026-10-24)

- **crates.io direct names:** `vc1`, `vc1-rs`, `vc-1`, `wmv9` — all
  **NOT_FOUND**. `wmv` — 0.0.0 **RESERVED** placeholder since 2017-09-19
  (2,111 downloads; matches feasibility §1 row 11 [RV] 2026-10-19).
- **NEW since the 2026-10-19 feasibility pass:** search surfaces
  **`wmv-decoder` 0.2.0** — "Pure-Rust WMV/WMA video and audio decoder",
  **MPL-2.0**, published 2026-10-08, single release, 75 downloads, repo
  `xmoezzz/siglus_rs` (a SiglusEngine game-engine monorepo — the codec is
  a component there, not a standalone audited library). Assessment:
  16 days old, zero conformance evidence, no published test corpus,
  provenance unverified `[VERIFY: purity + conformance before any
  adoption]`. **Not a candidate today**; recorded as the **Phase-7
  recheck trigger** (doc 14 R-10 window).
- **Reference decoder:** SMPTE VC-1 reference decoder is not freely
  distributable (doc 04 §4.1: "reference decoder unavailable") — the
  classic self-build input is absent, which is why the effort estimate
  is XL (B-frames, overlap smoothing, loop filter, interlaced field
  pictures).
- **VLC handling (oracle, not a path):** WMV2 probed via `avcodec`
  (formats.json `wmv2` + `asf-wmv` rows: fixture WMV2 64×48 + WMAv2,
  `asf_wmv2_wmav2.log`, exit 0); VC-1 = census-only (`vc1` demux +
  packetizer + `avcodec`, `vc1-wmv9` row: `not-probe-verified`). All
  C-side; ffmpeg is forbidden in our tree.
- **Landed state:** `codecs` crate admits exactly 4 symphonia engines
  (mp3/vorbis/aac/alac); no WMV/VC-1 edge exists; ASF container itself
  is not landed (see `container-asf.md`).

## Bit depths / layouts

N/A (blocked/deferred). For the record: WMV9/VC-1 is 8-bit 4:2:0-class;
VC-1 advanced profile adds interlaced field pictures (doc 04 §4.1
complexity note).

## Tracks / Seeking / Timestamps

Codec-level record; container-side is `container-asf.md`'s scope. The
Phase-6 container card must carry WMV/VC-1 tracks as typed-unsupported
streams (stream-granularity policy, MKV-with-blocked-TrueHD precedent).

## Corrupt-input behavior

N/A (no parser ships). The typed refusal path is registry-level and
panic-free by construction.

## Limits / Platforms

Matrix today: windows/linux `not_started` (hw-first path per ADR-001 +
feasibility §1 row 11 `vc1.hw = partial` when landed), macos `blocked`,
web `unsupported`. Patent note: VC-1 pool / MPEG-LA exposure is an
ADR-010 checklist item (same list as HEVC, doc 14; ADR-010 not yet
written) `[VERIFY at ADR-010]`.

## Corpus

None. No asf/wmv/vc1 lane exists in corpus-v4 (checked in 5.K3-I,
2026-10-24); the Phase-0 probe fixture (ffmpeg WMV2+WMAv2) is
oracle-side only. C-V6 acquisition is a prerequisite for any future
differential.

## Results

None (no code). Next step: **recheck crates.io at the Phase 7 R-10
decision window — named triggers: `wmv-decoder` (conformance + MPL-2.0
provenance audit) and any `oxideav`/`veloster`-family WMV entrance**;
schedule only with the Phase-6 ASF container work; ADR-010 must clear
the patent question before any hw-path claim ships.
