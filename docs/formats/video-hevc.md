# Codec record — H.265/HEVC (decode)

Status: **hw decode-path seam delivered (card 5.G2); playback
`hw-only` pending Phase-7 adapters**. The 5.G1 clean-room parser
(2-byte NAL header, Annex-B/length-prefixed/`hvcC` framing, VPS/SPS/
PPS, slice-segment header prefix, conformance-window crop) is LANDED
and corpus-verified (3/3 lanes, live-ffprobe differential). 5.G2 adds
the **hardware consumption point**: `codecs::video::hevc::hw` (the
adapter seam) + `codecs::video::hevc::decode` (the codec-api
`Decoder`) — the mirror of the H.264 5.F1/5.F2 shape, one spec
generation later. **There is no software HEVC decode, and none is
scheduled before ADR-001's conformance-gated software track produces
a validated decoder** (doc 04 §4.3: "ADR-001 hw-only, parser
self-build M, software decode `blocked` with next-step recorded").
Doc 04 §4.3 also carries the heavier-patent note: licensing
assessment for *distributing an HEVC decoder at all* is a §14 item
(doc 11 / ADR-010 territory) — the hw-first posture keeps that
decision with the OS adapters, not this crate.

- Date of evidence: **2026-10-25** (card 5.G2; parser evidence per
  5.G1, same date window).
- Governing decision: **ADR-001** (hw-first for H.264/HEVC, E2
  OS-ABI exception, truthful active-decoder reporting).
- Feature-matrix proposal (for the Lead's matrix pass):
  `codec-video-hevc` = `hw-only`: `probe`/`parse` done, `decode`
  behind the hw seam, `software_decode = blocked` on every platform,
  wasm32 = `blocked` (no OS decoder; WebCodecs is ADR-008 territory).

## Parse / Decode / Encode

- **Parse (done, 5.G1):** Annex-B + length-prefixed splitting,
  `hvcC` records (ISO/IEC 14496-15 §8.3.3.1), VPS/SPS/PPS
  (field-exact vs ffmpeg `trace_headers`/ffprobe on corpus lanes
  0044/0091/0100), slice-segment header prefix (the honest recorded
  subset: everything from short-term RPS on is deliberately
  unparsed — slice *data* is opaque bytes to an adapter anyway).
  Fuzz contract: total, typed, no panics.
- **Decode (5.G2, the seam):** `HevcDecoder` configures from `hvcC`
  or Annex-B extradata or in-band parameter sets (VPS **and** SPS/PPS
  — HEVC's outermost parameter set is part of the session offer),
  assembles access units per §7.4.2.4.4 (picture keys:
  `slice_pic_parameter_set_id`, `slice_pic_order_cnt_lsb` (absent for
  IDR — §7.4.8 pins POC 0), NAL type, layer/temporal ids,
  `pic_output_flag`, `no_output_of_prior_pics_flag`; AUD/EOS/EOB
  delimiters; end-of-packet closes an AU under length-prefixed
  framing per ISO/IEC 14496-15 §5.3.4.2), and feeds every picture —
  raw NAL bytes **plus** the parsed slice-segment facts — to a
  hardware session through `HevcHwDecoder::decode`. The spec's
  "previous NAL unit was not a VCL NAL unit" boundary condition is
  what separates an all-IDR stream's identical-header pictures
  (HEVC has no `idr_pic_id`; corpus 0044 is exactly that shape —
  56/56 AUs assembled). Output is `FrameStorage::OpaqueHandle`
  surfaces; the frame's `PixelFormat` is the SPS-derived presentation
  view (4:2:0/monochrome 8-bit → Nv12 with neutral chroma, 4:2:2 →
  Yuyv422; 4:4:4 and >8-bit — **Main 10, a real HEVC population** —
  are a typed `hevc.hw.surface_format` refusal: an honest vocabulary
  gap, `[PROPOSAL]`: media-types opaque-surface format extension).
- **Encode:** not planned (out of scope through Phase 7).

## The hardware seam (what 7.B1–7.B3 implement)

`crates/codecs/src/video/hevc/hw.rs` defines, and this card commits
as the stable adapter surface:

- `HevcHwDecoder` — one session: `configure(HevcHwStreamInfo)`
  (VPS + SPS + PPS lists, framing, conformance-window surface size),
  `decode(&HevcHwAccessUnit)` (one picture per call: NALs in stream
  order, VCL units carrying the parsed `SliceSegmentHeader` — which
  itself retains the NAL-header projection, unit type/layer/temporal
  ids), `flush` (drain), `backend_id` (ADR-001's active-decoder
  label). Synchronous, bounded-work, typed `HevcHwError`
  (Configure/Decode/DeviceLost).
- `HevcHwBridge` — the plug: adapter crates ship one bridge type
  that downcasts the negotiated `Arc<dyn HwAccelerator>` (the
  codec-api `as_any` seam) to their own concrete type and mints the
  session. `HevcHwPath::open` walks the installed bridges.
- `HevcHwFrame` carries no `interlaced` flag — HEVC has no field
  pictures (§7.4.2.4: interlaced sources are coded as frame pictures
  with VUI signalling); every emitted picture is a frame.
- The in-tree proof is the integration suite's **fake adapter**
  (accelerator + session + bridge) feeding real corpus bytes
  end-to-end — Annex-B in-band, `hvcC` + length-prefixed samples,
  pipeline-lag + flush, failure injection, and the full 0044 stream
  (every VCL unit reaches exactly one AU). No VA-API/VideoToolbox/
  Media Foundation code exists yet — that is exactly cards
  7.B1/7.B2/7.B3, which plug in without touching this crate (the
  bridge list is the only wiring).

## The honest fallback (typed, never silent)

ADR-001: machines without OS-decoder support get an honest typed
error, never a fake success. Implemented as **typed configure-time
refusals** (the engine learns before the first packet):

| Situation | Error kind | Context code |
|---|---|---|
| No accelerator negotiated | `Unsupported` | `hevc.hw.required` |
| Accelerator negotiated, no bridge claims it (the v0 in-tree state) | `Unsupported` | `hevc.hw.no_bridge` |
| Active SPS sampling has no PixelFormat vocabulary (4:4:4, >8-bit) | `Unsupported` | `hevc.hw.surface_format` |
| Session configure refused (device profile, surfaces) | `Unsupported` | `hevc.hw.configure` |
| Session decode failure / device lost | `Decoder{Bitstream}` / `Decoder{HardwareLost}` | `hevc.hw.decode` |

In-stream damage (truncated NALs, forbidden bits, emulation
violations, unparsable or context-ambiguous slices, orphan
continuation segments, truncated length prefixes) is skipped,
**counted**, and reported as `DecodeOutcome::Recovered{CorruptInput,
skipped}` — the decoder stays usable (bounded `MAX_PENDING_UNITS`
accumulator; MP §12). A NAL that parses under two PPS contexts
round-trip-matching both is refused as ambiguous rather than guessed.

## Known limitations (recorded, not hidden)

- The slice-segment header is the 5.G1 honest prefix: short-term
  RPS, SAO, deblocking and entry-point syntax are unparsed — the
  session receives raw NAL bytes for those, which is the form every
  OS decoder wants anyway; a full-header parser is a follow-up card
  if an adapter ever needs it.
- Main 10 / Rext >8-bit and 4:4:4 streams configure-refuse
  (`hevc.hw.surface_format`) — an honest media-types vocabulary gap
  (corpus has no such lane; C-V6 acquisition note per 5.G1).
- The "active SPS" for surface negotiation is the lowest-id
  PPS-referenced one; multi-parameter-set streams re-sync on their
  in-band activations (single-PS is the whole-corpus reality).
- The pts attached to an AU is its first VCL's packet pts; hw
  pipeline reorder is the session's `flush` contract (pinned by the
  fake-adapter lag test).
- Identical parameter-set re-activations are no-ops (streams that
  repeat VPS/SPS/PPS before every picture negotiate the session
  exactly once — pinned on the full 0044 run); a *changed* parameter
  set re-offers the stream view.

## VLC parity (the honest ceiling)

VLC decodes HEVC via `avcodec` (hardware priority; VLC's own build
notes call HEVC software decode "very slow"). Our mirror of that
posture is the seam + truthful matrix rows above; actual decode
arrives with the Phase-7 per-OS adapters. Until then an HEVC track
on any machine surfaces the typed `hevc.hw.*` refusal — doc 04
§4.3's requirement verbatim: the format is listed with its real
status, not smuggled in as "sometimes works".
