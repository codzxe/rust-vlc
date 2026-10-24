# Codec record — Apple ProRes 422/4444 + Avid DNxHD/HR (decode)

Status: **deferred (later) — decision record (card 5.P2b-I part B)**. P3
professional camera formats (doc 04 §4.1 tail rows: "Apple ProRes P3
later, none, L" / "DNxHD/HR P3 later, none, L"). No v1/v-next card.
Materially better crate availability than doc 04's 2026-10-19 "none"
column says — feasibility video-codecs §1 rows 13–14 already amended the
verdict to "later with named candidates"; this record is the future
admission card's evidence base. macOS has VideoToolbox ProRes decode
built in (hw path per ADR-001 if ever prioritized).

- Date of evidence: **2026-10-24** (crates.io API, this card).
- Supersedes-cites: doc 04 §4.1 rows, feasibility video-codecs §1 rows
  13–14 + §5 (2026-10-19), feature-matrix row `codec-video-prores-dnxhd`
  (not_started ×4).

## Parse / Decode / Encode

Nothing implemented. When prioritized: decode-only, via a
validate-then-admit candidate (below); ProRes 422 Proxy/LT/Standard/HQ +
4444/4444 XQ and DNxHD/DNxHR (SMPTE ST 2019-1 VC-3). Encode: not planned
(both crates below ship encoders; we would simply not expose them in v1).
Typed-unsupported stream semantics apply meanwhile wherever the payloads
appear (MOV `ap4x`/MKV/ASF-adjacent containers expose tracks).

## Evidence (what was checked, 2026-10-24)

**ProRes candidates:**
- `oxideav-prores` **0.1.1** — "Pure-Rust Apple ProRes codec — decoder
  + encoder for 422 Proxy/LT/Standard/HQ and 4444/4…", **MIT**,
  2026-07-20, 3,262 downloads (OxideAV family — same project as
  `oxideav-ffv1`/`oxideav-wma`, the family the WMA record already flags
  for provenance `[VERIFY: clean-room lineage vs ffmpeg's reverse-
  engineered ProRes — ffmpeg cannot be a source, feasibility §5]`).
- `veloster-prores` **0.1.10** — "conforming to Apple RDD 36", **MIT OR
  Apache-2.0**, 2026-10-12, 27 downloads; **0.1.0→0.1.10 all published
  the same day** (rapid iteration, no soak time).
- `veloce-prores` **0.1.7** — same RDD 36 claim, same author family
  (`magosam/veloce`), MIT OR Apache-2.0, 2026-10-11, 19 downloads.
- `oxideav-videotoolbox` 0.0.4 (MIT, 2026-10-17) — Apple-platform
  VideoToolbox bridge, "runtime-loaded" — an OS-ABI-exception-shaped
  crate (ADR-001 territory), not a software path; noted for the hw
  column only.
- Doc 04's flat "none" for ProRes is **stale since at least 2026-07-20**
  (feasibility already recorded this drift).

**DNxHD/HR candidates:**
- `soundkit-dnx` **0.13.3** — "Memory-safe Rust VC-3, DNxHD, and DNxHR
  decoder", **LGPL-2.1-or-later**, 2026-09-13, 40 downloads.
  **MATERIAL FINDING missed by the feasibility pass: the license was
  never recorded there — LGPL-2.1+ puts it in the same out-of-tree
  posture as `oxisynth` (ADR-006 permissive-only; doc 11's cargo-deny
  allowlist keeps GPL/LGPL absent pending the Lead's decision).** Not a
  candidate under current policy.
- **NEW (2026-10-23, post-feasibility): `filmcraft-dnx` 0.9.0** —
  "Clean-room Avid DNxHD / DNxHR (SMPTE ST 2019-1 VC-3) decoder and
  encoder", **MIT OR Apache-2.0**, 9 downloads, one release, no repo
  link on crates.io. Clean-room claim is the author's, unverified
  `[VERIFY: provenance + conformance]`. The permissive-license
  candidate the format was waiting for — too young to admit, exactly
  the right recheck trigger.
- `veloster-dnx` 0.1.10 / `veloce-dnx` 0.1.7 — **frame header parsers +
  compression-ID analyzers only** (MIT OR Apache-2.0, 2026-10-12/-11) —
  probe/identification grade, not decode.
- Direct names `dnxhd`, `dnxhr`, `dnxhd-decoder`, `prores`, `prores422`,
  `apple-prores`, `prores-rs` — NOT_FOUND.

**VLC handling (oracle):** both families decode via `avcodec`
(formats.json `video-census-tail`, census-only, not probe-verified; no
dedicated VLC modules — modules-linux.json's 14 video-decoder
registrations contain none for ProRes/DNxHD). ProRes also rides macOS
VideoToolbox in VLC's hw paths.

## Bit depths / layouts

N/A (deferred). For the record: ProRes 422 = 10-bit 4:2:2, 4444 adds
12-bit + alpha; DNxHD/HR 8–12-bit 4:2:2 (per RDD 36 / ST 2019-1 — public
specs, a genuine self-build option at L per doc 04).

## Tracks / Seeking / Timestamps

Codec-level record; payloads arrive in MOV/MP4/MKV — existing container
records own track/seek mechanics. Typed-unsupported carrying until any
admission.

## Corrupt-input behavior

N/A (no parser ships).

## Limits / Platforms

Matrix today: not_started ×4 (phase target 8). If macOS hw is ever
claimed: ADR-001 audit path, same as other VT codecs.

## Corpus

None — no ProRes/DNxHD lane in corpus-v4 `[VERIFY: acquire at C-V6 or
ffmpeg-generate for the admission card]`.

## Results

None (no code). Next step: **recheck crates.io at Phase 7 R-10 window —
named triggers: `filmcraft-dnx` (permissive clean-room claim — the lead
candidate), `veloster-prores`/`veloce-prores` soak + conformance,
`oxideav-prores` provenance audit**; `soundkit-dnx` stays out-of-tree
unless doc 11 changes the LGPL posture. Any admission is a Phase-6+
card with differential vs VLC + fuzz (`fuzz_codec_prores` /
`fuzz_codec_dnxhd` per doc 04 §4.1 pattern).
