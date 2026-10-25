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
# Video record — HEVC/H.265 (parser; decode blocked)

Records-consolidation entry (card 5.REC-2; evidence from card 5.G1,
finisher P5-G1F — the clean-room parser. The 5.G2 decode-path
integration card is NOT landed: wave-7 close lists it among the
remaining cards, so no decode-path seam exists for HEVC yet, unlike
H.264's 5.F2). Front-matter keys per doc 04 §6:

- id = `codec-video-hevc` (hw row `not_started`, software row
  `blocked` — both consistent with this record; evidence backfill
  proposed)
- status = **parser implemented; decode blocked** (roadmap §1.1: hw
  track `not_started` pending 7.B adapters; software `blocked` on the
  rusty_h265-class young-crate gate + ADR-010)
- phase = 5 (parser landed)
- fuzz_target = `fuzz_codec_hevc` (planned name; fuzz-lite in-tree
  today)
- crates = [] — none admitted
- self_built = [`codecs::video::hevc` (parser: nal/vps/sps/pps/hvcc/
  slice — ITU-T H.265 v4 clean-room per the 5.F1 pattern)]
- last_audit = 2026-10-24 (this consolidation; parser evidence
  2026-10-2x per 5.G1)

## Parse / Decode / Encode

- **Parser (landed 5.G1):** 2-byte NAL header (fzb/layer/tid +
  Table 7-1 classification incl. is_vcl/is_irap); Annex-B 3/4-byte
  start codes + length-prefixed AV walk; emulation-prevention removal
  (chained overlapping escapes, pinned on the real 0100 VPS); hvcC
  record (ISO 14496-15 §8.3.3.1); VPS (shared profile_tier_level +
  HRD walker); SPS (chroma/bit-depths/geometry/conformance-window/
  sub-layer ordering/CTB+transform sizes/VUI); PPS (ids/tile grid/
  loop-filter/QP); **slice-segment header prefix** — first-slice/
  IRAP no-output/pps-id/dependent+CTB address/type/output-flag/
  colour-plane/POC-lsb. Everything from short-term RPS on is
  deliberately unparsed (the honest recorded subset).
- **Decode: none.** No `Decoder` impl, no hw seam (that was 5.G2,
  un-landed); HEVC tracks refuse typed at the codec registry today.

## Evidence (Task 5.G1)

- **Corpus differential (live ffprobe 7.1.5):** 3/3 lanes PASS —
  profile/tier/level/resolution/chroma/depth all agree:

  | lane | entry form | verdict |
  |---|---|---|
  | 0044-raw-video.265 | raw Annex-B | Rext/Main/63, 360×640 (coded 368×640 + conf-win), 8-bit, poc_bits 8 — PASS |
  | 0091-bframe-storm.ts | ffmpeg `-c copy -f hevc` extraction | Main/Main/63, 480×360 — PASS |
  | 0100-mp4-video.mp4 | real hvcC via ffprobe extradata hex | Main/Main/93, 1024×768; 4 arrays (VPS/SPS/PPS/prefix-SEI) byte-verified vs the live record — PASS |

- **Independent cross-check:** the parser was verified against a
  from-scratch Python bit-level SPS walk (spec order) — every field
  agrees; the core was right where the inherited test expectations
  were wrong (8 failures fixed by correcting the tests, incl. a
  builder that omitted per-array `numNalus`).
- **Fuzz-lite:** 10k random buffers framed as VPS/SPS/PPS +
  annexb/hvcC walks; 5k mutations of the real 0100 parameter sets +
  hvcC; 2 000 whole-stream mutations of 0044 — no panics, no hangs,
  all failures typed `HevcError`.
- **Tests at card gate:** 479 passed / 0 failed (19 new hevc lib
  tests + 3 corpus integration tests); fmt + clippy clean.

## Honest gaps

- **No decode** (above) — 5.G2 un-landed; hw adapters are the Phase-7
  7.B cards, software blocked (rusty_h265 0.6.2 / rust_h265 0.1.0
  young/unproven + ADR-010 distribution legality — matrix
  `codec-video-hevc-software` blocked_reason).
- **Full slice-segment header unparsed** (RPS/SAO/deblocking/
  entry-points — out of the honest subset by design).
- **Main-10/Rext >8-bit and 4:4:4 depth lanes:** corpus has none
  (C-V6 acquisition note).
- **Sub-layer streams** (max_sub_layers>1): walked, no corpus lane.
- **TS/MP4 entry through our own demuxers:** the corpus lanes used
  ffmpeg extraction + ffprobe-hex seams (sibling cards own the real
  paths); 0038/0099 hevc-in-AVI SKIP — ffprobe itself reports codec
  unknown (AVI fourcc hack), no oracle to differ against.

## Next steps

1. 5.G2 decode-path integration (the H.264 5.F2 analog: hw seam +
   typed fallback) when scheduled; then 7.B adapters.
2. Phase-6 software decision point: rusty_h265-class candidates stay
   behind the conformance gate (the rusty_vp9/ruopus
   validate-then-admit pattern would apply).
3. C-V6 acquisition for Main-10/4:4:4 lanes; own-demuxer entry forms.
