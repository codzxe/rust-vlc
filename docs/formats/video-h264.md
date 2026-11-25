# Codec record — H.264/AVC (decode)

Status (audit-corrected 2026-11-24, Devil Audit C-001): **software
H.264 decode is SHIPPED and executed** — `rust_h264` 0.6.1 behind the
codec-api contract, Lead-admitted 2026-11-15 (admission record
`docs/dependencies/records/rust_h264-0.6.1.md`; differential evidence
`crates/codecs/tests/h264_sw_differential.rs`: bit-exact vs ffmpeg 7.1.5
on six lanes × both entry forms). The shipped dispatch arm is
`"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)`
(`apps/cli/src/pipeline_host.rs:346`), seeded from the container's
`avcC` harvest (`apps/cli/src/containers/mp4.rs:237`). Audit ground
truth: the release binary decodes h264+aac to **100.0% end-of-media**
(3.041814 s, exit 0), moov-at-end and faststart demux open
identically, I420→RGBA executes (`pipeline_host.rs:1340-1342`), and
`H264SwDecoder::emit_frame` symbols are present in the shipped binary
(581 `rust_h264` strings) — `audit/final/MP4-REALITY-TRACE.md` §1/§3
(W20-A05) and DEVIL-AUDIT-REPORT §2's corrected interpretation
(W20-A02 F-3). The 5.F1 clean-room parser (NAL/Annex-B/AVC/avcC,
SPS/PPS, slice headers, SEI) is LANDED and corpus-verified; 5.F2's
**hardware consumption point** (`codecs::video::h264::hw` +
`codecs::video::h264::decode`) remains the adapter seam — **no
hardware lane is wired in the shipped build** (the
`H264Decoder::from_setting` / `playback.hw-decode-policy` opt-in
Phase-7 path, `pipeline_host.rs:341-345`; the audit's honest
remaining gaps: null audio sink, headless renderer, MP4 reanchor —
MP4-REALITY-TRACE F-002/F-003/F-001).

- Date of evidence: **2026-10-25** (card 5.F2; parser evidence
  2026-10-2x per 5.F1) — superseded 2026-11-15 (rust_h264 admission)
  and audit-corrected 2026-11-24 (Devil Audit C-001/W20-A05).
- Governing decision: **ADR-001** (hw-first for H.264/HEVC, E2
  OS-ABI exception, truthful active-decoder reporting).
- Feature-matrix status: the 2026-10 proposal (h264 as the hw-only
  seam with software decode blocked on every platform) is
  **superseded** — software decode landed 2026-11-15 and ships; the
  audit holds row 023's sw-decode lane REAL (W19-S2 ledger).
  wasm32 stays `blocked` (no OS decoder; WebCodecs is ADR-008
  territory).

## Parse / Decode / Encode

- **Parse (done, 5.F1):** Annex-B + AVC length-prefixed splitting,
  `avcC` records, SPS/PPS (field-exact vs ffmpeg `trace_headers`),
  slice headers (§7.3.3 with SPS/PPS context seams), SEI (buffering
  period, pic timing, user-data unregistered, recovery point,
  filler). Fuzz contract: total, typed, no panics.
- **Decode (5.F2, the seam):** `H264Decoder` configures from
  `avcC`/Annex-B extradata or in-band parameter sets, assembles
  access units per §7.4.1.2.4 (picture keys: frame_num/pps-id/field
  flags/nal_ref_idc/IDR id/POC-type-0 lsb+delta; AUD + EOS delimiters;
  end-of-packet closes an AU under AVC framing per ISO/IEC
  14496-15), and feeds every picture — raw NAL bytes **plus** the
  parsed slice facts — to a hardware session through
  `H264HwDecoder::decode`. Output is `FrameStorage::OpaqueHandle`
  surfaces; the frame's `PixelFormat` is the SPS-derived presentation
  view (4:2:0/gray 8-bit → Nv12 with neutral chroma — the corpus
  0019/ffmpeg convention; 4:2:2 → Yuyv422; 4:4:4 and >8-bit are a
  typed `h264.hw.surface_format` refusal — an honest vocabulary gap,
  `[PROPOSAL]`: media-types opaque-surface format extension).
- **Encode:** not planned (out of scope through Phase 7).

## The hardware seam (what 7.B1–7.B3 implement)

`crates/codecs/src/video/h264/hw.rs` defines, and this card commits
as the stable adapter surface:

- `H264HwDecoder` — one session: `configure(H264HwStreamInfo)`
  (parameter sets + framing + surface size), `decode(&H264HwAccessUnit)`
  (one picture per call: NALs in stream order, VCL units carrying
  the parsed `SliceHeader` + `SliceNalInfo`), `flush` (drain),
  `backend_id` (ADR-001's active-decoder label). Synchronous,
  bounded-work, typed `H264HwError` (Configure/Decode/DeviceLost).
- `H264HwBridge` — the plug: adapter crates ship one bridge type
  that downcasts the negotiated `Arc<dyn HwAccelerator>` (the
  codec-api `as_any` seam — "the decoder wrapper that knows its
  concrete backend retrieves it") to their own concrete type and
  mints the session. `H264HwPath::open` walks the installed bridges.
- The in-tree proof is the integration suite's **fake adapter**
  (accelerator + session + bridge) feeding real corpus bytes
  end-to-end, including pipeline-lag + flush and failure injection.
  No VA-API/VideoToolbox/Media Foundation code exists yet — that is
  exactly cards 7.B1/7.B2/7.B3, which plug in without touching this
  crate (the bridge list is the only wiring).

## The honest fallback (typed, never silent)

ADR-001: "Machines without OS-decoder support get an honest
`unsupported-software-decode-pending` error, never a fake success."
Implemented as **typed configure-time refusals** (the engine learns
before the first packet):

| Situation | Error kind | Context code |
|---|---|---|
| No accelerator negotiated | `Unsupported` | `h264.hw.required` |
| Accelerator negotiated, no bridge claims it (the v0 in-tree state) | `Unsupported` | `h264.hw.no_bridge` |
| Active SPS sampling has no PixelFormat vocabulary | `Unsupported` | `h264.hw.surface_format` |
| Session configure refused (device profile, surfaces) | `Unsupported` | `h264.hw.configure` |
| Session decode failure / device lost | `Decoder{Bitstream}` / `Decoder{HardwareLost}` | `h264.hw.decode` |

In-stream damage (truncated NALs, forbidden bits, unparsable slices,
missing SPS/PPS context, legacy data partitions) is skipped,
**counted**, and reported as `DecodeOutcome::Recovered{CorruptInput,
skipped}` — the decoder stays usable (bounded `MAX_PENDING_UNITS`
accumulator; MP §12).

## Evidence (audit amendment, card 5.REC-2 — numbers from the worklog)

- **Parser (5.F1a/5.F1b):** corpus differential 19/19 lanes PASS vs
  live ffprobe (0019 raw Annex-B High@L1.3 64×48 + 18 mkv h264 tracks
  via real avcC CodecPrivate: profiles Baseline→High 4:4:4 Predictive,
  levels 10–32, 420/422/444/mono); slice/SEI differential vs
  `trace_headers`: 0019 = 26 slices (P5×8/B6×16/I7×2, == ffprobe
  nb_read_frames), 0014-muxed-av.mkv = 185/185 slices, I4/P46/B135.
  Real ffmpeg-writer bug found+fixed at 5.F1a: avcc PPS count byte
  is the bare count (reserved bits clear) — masked 5-bit read,
  documented deviation vs ISO 14496-15.
- **Decode-path (5.F2):** fake-adapter end-to-end on real corpus
  bytes — 0019: 26/26 VCL→AU accounting, session config
  (1,1,(64,48)) exact, pts mapping pinned; 16-test suite incl.
  typed-fallback codes asserted by string, damage = counted
  recovery, hw failure mapping ×3, +FB fuzz-lite ×2 (5k packets +
  2k extradata blobs, absence-of-panic contract).
- **Tests at gates:** 5.F1a 273/0; 5.F1b 266/0; 5.F2 473/0 (16 new
  h264_hw_path tests; h264_parser_corpus untouched & green).
- Parser fuzz: 3 000-round xorshift mutations (nal.rs) + 2 000-round
  real-stream mutations + every-prefix truncation sweep of a real
  0019 slice — Ok-or-typed only, zero panics.

## Known limitations (recorded, not hidden)

- `delta_pic_order_always_zero_flag` is not retained by the LANDED
  SPS parser (read-and-discard at sps.rs §7.3.2.1.1 walk); the
  slice-context projection therefore pins it `false`, and a
  set-flag POC-type-1 stream surfaces as counted slice-header
  damage. Parser follow-up card, not a 5.F2 fix (parser files are
  read-only).
- POC type-1 pictures have no §7.4.1.2.4 boundary rule computable
  from headers alone (needs decoder-side frame history); AU assembly
  relies on the other key fields for those streams.
- The "active SPS" for surface negotiation is the lowest-id
  PPS-referenced one; multi-parameter-set streams re-sync on
  in-band activations (single-PS is the whole-corpus reality).
- The pts attached to an AU is its first VCL's packet pts; hw
  pipeline reorder is the session's `flush` contract (pinned by the
  fake-adapter lag test).

## VLC parity (the honest ceiling)

VLC decodes H.264 via `avcodec` (ffmpeghw priority 10001 — the
pinned baseline itself prefers hardware). Our mirror of that
posture was the seam + truthful matrix rows; **software decode has
since landed and ships** (rust_h264 0.6.1, above — the binary decodes
h264 tracks on every machine, MP4-REALITY-TRACE §1: 100.0%
end-of-media, exit 0). The hardware adapters remain the absent half:
today's honest status is "software decode shipped, hw lane not
wired" — the typed `h264.hw.*` refusals above are the hardware
session's surface for the machines/policies that demand it. Doc 04
§4.2's requirement stands verbatim: the format is listed with its
real status, not smuggled in as "sometimes works".
