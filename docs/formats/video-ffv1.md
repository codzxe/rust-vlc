# Codec record — FFV1 (lossless intra video, RFC 9043)

Status: **deferred (later) — decision record (card 5.P2b-I part B)**,
per feasibility subsystems §7.3 μADR-B recommendation: **`later`
(Phase 6+ decision card)**. FFV1 matters to film-archive users, not to
the parity share; if a concrete use-case appears, schedule the M-sized
work (admit a candidate below, or self-build per the public RFC).
A v1 attempt was explicitly rejected: no design surface, no corpus in
hand (μADR-B option c).

- Date of evidence: **2026-10-24** (crates.io API, this card).
- Supersedes-cites: doc 04 §4.1 row "FFV1 (lossless) P3 later, none on
  crates.io verified [VERIFY-P0] … self-build M", feasibility
  video-codecs §1 row 15 + §5 (2026-10-19 — already closed doc 04's
  [VERIFY-P0] by finding `oxideav-ffv1`/`codec_ffv1`), feature-matrix
  row `codec-video-ffv1` (not_started ×4, notes already carry the
  candidates).

## Parse / Decode / Encode

Nothing implemented. When scheduled: decode versions 0/1/3 (range
coder + context model; slices → later threading per μADR-B), RGB and
YCbCr 8/10/12-bit, progressive (interlaced is v1 spec-only edge).
Encode: not planned v1 (archival ingest is out of scope; the crates
below ship encoders we would not expose).

## Evidence (what was checked, 2026-10-24)

- **crates.io:** `ffv1` — 0.0.0 **RESERVED** placeholder since
  2017-09-19 (2,183 downloads; name-squat, unchanged). Real candidates,
  re-confirmed today:
  - `oxideav-ffv1` **0.0.9** — "Pure-Rust FFV1 (RFC 9043) lossless
    intra-only video codec for oxideav", **MIT**, newest of 7 releases
    since 0.0.3 (2026-04-17), 1,566 downloads. OxideAV family —
    provenance unverified vs our clean-room bar `[VERIFY: ffmpeg's
    FFV1 is LGPL and cannot be a source]`, pre-1.0 churn (7 releases
    in ~11 weeks).
  - `codec_ffv1` **0.1.0** — "Container-agnostic implementation of
    FFV1 lossless video codec", **MIT**, 2026-08-10, single release,
    179 downloads. Same youth caveat.
  - `cu-ffv1-codec` 1.3.0 (Apache-2.0, 2026-10-15) — "FFV1 log codec
    for Copper image payloads" — robotics-framing image log codec,
    **not** a general FFV1 video decoder; name-collision disambiguation
    recorded so future searches are not fooled.
  - `gst-plugin-ffv1` 0.0.0 — GStreamer plugin placeholder, not
    relevant.
  - Doc 04 §4.1's flat "none" is **stale since 2026-04-17** (feasibility
    already recorded this; matrix notes already amended).
- **Spec:** RFC 9043 is public and informational — the strongest
  self-build input of this record's whole family (M effort per doc 04).
- **VLC handling (oracle):** `avcodec` (formats.json
  `video-census-tail`, census-only, not probe-verified — no probe
  fixture; ffmpeg `-c:v ffv1` fixtures would be oracle-side C-V6
  work).

## Bit depths / layouts

8/10/12-bit RGB(YCbCr); context-model per version (0: fixed, 1/3:
adaptive). No code — recorded from the RFC for the future card's
scope table.

## Tracks / Seeking / Timestamps

Codec-level; payloads arrive in MKV/MOV — existing container records
own mechanics. FFV1-in-MKV tracks already surface as typed-unsupported
streams through the MKV demuxer (codec id pass-through + decode
refusal), which is the honest v1 behavior.

## Corrupt-input behavior

N/A (no parser ships). Range-coder desync tolerance is a design input
for the future card (lossless archival files are often truncated).

## Limits / Platforms

Matrix: not_started ×4 (phase target 8). Patents: RFC 9043
informational, believed unencumbered — **ADR-010 checklist item**
(ADR-010 not yet written; μADR-B pinned this) `[VERIFY at ADR-010]`.

## Corpus

None — no FFV1 lane in corpus-v4; generation via ffmpeg `-c:v ffv1`
is trivial oracle-side once the card exists.

## Results

None (no code). Next step: **Phase-6+ decision card if archival
use-case lands; admission path = validate-then-admit `oxideav-ffv1`
0.0.9 / `codec_ffv1` 0.1.0 (conformance vs ffmpeg-generated corpus +
purity audit), else M self-build per RFC 9043; ADR-010 patent check
before any claim; recheck crates.io at Phase 7 R-10 either way.**
Fuzz target per doc 04 §4.1 pattern: `fuzz_codec_ffv1`.
